// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

use crate::error::ConfigError;
use crate::key::{
    CURRENT_KEY_VERSION, KeyBundle, KeyRing, KeyRotationSchedule, KeyStatus, RotationPlan,
    RotationResult, SECONDS_PER_DAY, now_timestamp,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[cfg(feature = "encryption")]
use rand::Rng;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyVersion {
    pub id: String,
    pub version: u32,
    pub created_at: u64,
    pub status: KeyStatus,
    pub algorithm: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyInfo {
    pub key_id: String,
    pub current_version: u32,
    pub total_versions: usize,
    pub active_versions: usize,
    pub deprecated_versions: usize,
    pub created_at: u64,
    pub last_rotated_at: Option<u64>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct KeyManager {
    key_rings: HashMap<String, KeyRing>,
    schedules: HashMap<String, KeyRotationSchedule>,
    default_key_id: String,
    /// 轮换策略（`reencrypt_envelope` 的 Lazy/时窗判定来源）。
    #[serde(default)]
    rotation_policy: crate::key::KeyRotationPolicy,
    /// 轮换审计出口端口；未注入时为 Noop（零行为变化）。
    /// serde 跳过：反序列化得到的实例回退到 Noop 端口。
    #[cfg(feature = "audit")]
    #[serde(skip, default = "default_audit_port")]
    audit_port: std::sync::Arc<dyn crate::key::audit_port::RotationAuditPort>,
}

/// 反序列化被跳过的 `audit_port` 字段的取值：Noop 端口。
#[cfg(feature = "audit")]
fn default_audit_port() -> std::sync::Arc<dyn crate::key::audit_port::RotationAuditPort> {
    std::sync::Arc::new(crate::key::audit_port::NoopRotationAudit)
}

impl std::fmt::Debug for KeyManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Redacted Debug: key rings hold key material (albeit encrypted) and
        // must never leak into logs. Only non-sensitive metadata is emitted.
        f.debug_struct("KeyManager")
            .field("default_key_id", &self.default_key_id)
            .field(
                "key_rings",
                &self
                    .key_rings
                    .iter()
                    .map(|(id, ring)| {
                        format!(
                            "{} (current v{}, {} secondaries)",
                            id,
                            ring.current_version,
                            ring.secondary_keys.len()
                        )
                    })
                    .collect::<Vec<String>>(),
            )
            .field("schedule_ids", &self.schedules.keys().collect::<Vec<_>>())
            .finish()
    }
}

/// Trim `secondary_keys` down to `max_versions` entries.
///
/// Pruning prefers the oldest non-Active (retired) versions first; Active
/// secondaries are only dropped as a last resort when the policy bound would
/// otherwise be exceeded. This backs `KeyManager::rotate_key`'s enforcement
/// of the schedule's `max_versions` (unbounded secondary growth otherwise).
#[cfg(feature = "encryption")]
fn prune_secondary_keys_to_cap(key_ring: &mut KeyRing, max_versions: usize) {
    if key_ring.secondary_keys.len() <= max_versions {
        return;
    }
    let excess = key_ring.secondary_keys.len() - max_versions;

    // Order candidates for pruning: non-Active before Active, oldest first
    // within each group. Sorting indices keeps the ring's storage order
    // stable for the keys that survive.
    let mut prune_order: Vec<usize> = (0..key_ring.secondary_keys.len()).collect();
    prune_order.sort_by_key(|&i| {
        let key = &key_ring.secondary_keys[i];
        (
            key.metadata.status() == KeyStatus::Active,
            key.metadata.version(),
        )
    });
    let to_remove: Vec<usize> = prune_order.into_iter().take(excess).collect();

    let mut kept = Vec::with_capacity(key_ring.secondary_keys.len() - excess);
    for (i, key) in key_ring.secondary_keys.drain(..).enumerate() {
        if !to_remove.contains(&i) {
            kept.push(key);
        }
    }
    key_ring.secondary_keys = kept;
}

impl KeyManager {
    #[cfg(feature = "encryption")]
    pub fn new() -> Result<Self, ConfigError> {
        Ok(Self {
            key_rings: HashMap::new(),
            schedules: HashMap::new(),
            default_key_id: "default".to_string(),
            rotation_policy: crate::key::KeyRotationPolicy::default(),
            #[cfg(feature = "audit")]
            audit_port: std::sync::Arc::new(crate::key::audit_port::NoopRotationAudit),
        })
    }

    /// 设置轮换策略（builder 风格）。
    ///
    /// `reencrypt_envelope` 依据此策略判定 `reencrypt_read` 与
    /// `dual_key_window_secs`；缺省为 `KeyRotationPolicy::default()`。
    pub fn with_rotation_policy(mut self, policy: crate::key::KeyRotationPolicy) -> Self {
        self.rotation_policy = policy;
        self
    }

    /// 注入轮换审计出口端口（builder 风格）。
    ///
    /// 端口在 `rotate_key` 轮换成功后收到一条 `AuditEvent::KeyRotation`；
    /// 未注入时保持 Noop，轮换行为与现状完全一致。
    #[cfg(feature = "audit")]
    pub fn with_audit_port(
        mut self,
        port: std::sync::Arc<dyn crate::key::audit_port::RotationAuditPort>,
    ) -> Self {
        self.audit_port = port;
        self
    }

    /// Initialize a new key ring with the given master key
    ///
    /// # Security Notes
    ///
    /// - ⚠️ **Master Key**: The master key must be stored securely and never shared or committed to version control
    /// - ⚠️ **Key ID**: Use descriptive key IDs (e.g., "production", "staging", "development")
    /// - ⚠️ **Created By**: Include creator information for audit trail
    /// - ⚠️ **Key Backup**: Ensure you have a secure backup of the master key
    /// - ⚠️ **Key Rotation**: Set up automatic key rotation schedule after initialization
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use confers::key::KeyManager;
    /// # let master_key = [0u8; 32];
    /// let mut km = KeyManager::new()?;
    /// let version = km.initialize(
    ///     &master_key,
    ///     "production".to_string(),
    ///     "security-team".to_string()
    /// )?;
    /// # Ok::<(), confers::error::ConfigError>(())
    /// ```
    #[cfg(feature = "encryption")]
    pub fn initialize(
        &mut self,
        master_key: &[u8; 32],
        key_id: String,
        created_by: String,
    ) -> Result<KeyVersion, ConfigError> {
        let key_ring = KeyRing::new(master_key, key_id.clone(), created_by)?;
        self.key_rings.insert(key_id.clone(), key_ring);

        let schedule = KeyRotationSchedule::new(key_id.clone(), 90, now_timestamp(), 5);
        self.schedules.insert(key_id.clone(), schedule);

        self.default_key_id = key_id.clone();

        Ok(KeyVersion {
            id: format!(
                "{}_{}",
                key_id,
                crate::key::KeyFormatVersion::CURRENT.as_str()
            ),
            version: CURRENT_KEY_VERSION,
            created_at: now_timestamp(),
            status: KeyStatus::Active,
            algorithm: "XChaCha20-Poly1305".to_string(),
        })
    }

    /// Generate a new cryptographically secure random key
    ///
    /// # Security Notes
    ///
    /// - ⚠️ **Randomness**: Uses cryptographically secure random number generator (CSPRNG)
    /// - ⚠️ **Key Strength**: Generates 256-bit keys for AES-256-GCM encryption
    /// - ⚠️ **Key Usage**: Use the generated key immediately or store it securely
    /// - ⚠️ **Key Disposal**: Ensure the key is properly zeroized when no longer needed
    /// - ⚠️ **Key Reuse**: Never reuse keys for different purposes
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use confers::key::KeyManager;
    /// # use confers::secret::XChaCha20Crypto;
    /// # let mut km = KeyManager::new().unwrap();
    /// let key = km.generate_key()?;
    /// let encryption = XChaCha20Crypto::new();
    /// # Ok::<(), confers::error::ConfigError>(())
    /// ```
    #[cfg(feature = "encryption")]
    pub fn generate_key(&mut self) -> Result<[u8; 32], ConfigError> {
        let mut key_bytes = [0u8; 32];
        rand::rng().fill_bytes(&mut key_bytes);
        Ok(key_bytes)
    }

    #[cfg(feature = "encryption")]
    pub fn create_key_ring(
        &mut self,
        master_key: &[u8; 32],
        key_id: String,
        created_by: String,
        description: Option<String>,
    ) -> Result<KeyVersion, ConfigError> {
        if self.key_rings.contains_key(&key_id) {
            return Err(ConfigError::ParseError {
                format: "key".to_string(),
                message: format!("Key ring '{}' already exists", key_id),
                location: None,
                source: None,
            });
        }

        let key_ring = KeyRing::new(master_key, key_id.clone(), created_by)?;

        self.key_rings.insert(key_id.clone(), key_ring);

        if let Some(desc) = description
            && let Some(key) = self.key_rings.get_mut(&key_id)
        {
            key.primary_key.metadata.description = Some(desc);
        }

        let schedule = KeyRotationSchedule::new(key_id.clone(), 90, now_timestamp(), 5);
        self.schedules.insert(key_id.clone(), schedule);

        Ok(KeyVersion {
            id: format!(
                "{}_{}",
                key_id,
                crate::key::KeyFormatVersion::CURRENT.as_str()
            ),
            version: CURRENT_KEY_VERSION,
            created_at: now_timestamp(),
            status: KeyStatus::Active,
            algorithm: "XChaCha20-Poly1305".to_string(),
        })
    }

    /// Rotate the key to a new version
    ///
    /// # Security Notes
    ///
    /// - ⚠️ **Master Key**: Must use the same master key that was used to initialize the key ring
    /// - ⚠️ **Key Rotation**: Regular key rotation is recommended (every 90 days for production)
    /// - ⚠️ **Key Transition**: Old keys remain available for decryption during transition period
    /// - ⚠️ **Audit Trail**: Include creation information and description for audit purposes
    /// - ⚠️ **Re-encryption**: After rotation, re-encrypt all data that was encrypted with the old key
    /// - ⚠️ **Retention Cap**: After rotation, secondary keys are trimmed back to the ring
    ///   schedule's `max_versions` (retired/non-Active versions are pruned first, oldest first)
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use confers::key::KeyManager;
    /// # let mut km = KeyManager::new().unwrap();
    /// # let master_key = [0u8; 32];
    /// let result = km.rotate_key(
    ///     &master_key,
    ///     Some("production".to_string()),
    ///     "security-team".to_string(),
    ///     Some("Scheduled rotation".to_string())
    /// )?;
    /// println!("Rotated from version {} to {}", result.previous_version, result.new_version);
    /// # Ok::<(), confers::error::ConfigError>(())
    /// ```
    #[cfg(feature = "encryption")]
    pub fn rotate_key(
        &mut self,
        master_key: &[u8; 32],
        key_id: Option<String>,
        created_by: String,
        description: Option<String>,
    ) -> Result<RotationResult, ConfigError> {
        let key_id = key_id.unwrap_or_else(|| self.default_key_id.clone());

        let key_ring = self
            .key_rings
            .get_mut(&key_id)
            .ok_or_else(|| ConfigError::ParseError {
                format: "key".to_string(),
                message: format!(
                    "Key ring '{}' not found — initialize it with initialize()/create_key_ring() before rotating keys",
                    key_id
                ),
                location: None,
                source: None,
            })?;

        let old_version = key_ring.current_version;
        let new_key = key_ring.rotate(master_key, created_by, description)?;

        // Enforce the rotation policy's retention bound: `KeyRing::rotate`
        // archives every old primary without a cap, so trim `secondary_keys`
        // back to the schedule's `max_versions` after each rotation. Retired
        // (non-Active) versions are pruned first, oldest first; Active
        // secondaries are only removed as a last resort when the bound cannot
        // be met otherwise.
        if let Some(schedule) = self.schedules.get(&key_id) {
            let max_versions = schedule.max_versions as usize;
            if key_ring.secondary_keys.len() > max_versions {
                prune_secondary_keys_to_cap(key_ring, max_versions);
            }
        }

        if let Some(schedule) = self.schedules.get_mut(&key_id) {
            schedule.update_after_rotation();
        }

        // 轮换成功（Completed）后经端口发射审计事件；fields_reencrypted
        // 在惰性重加密账本就绪前恒为 0。
        #[cfg(feature = "audit")]
        self.audit_port
            .record(crate::impl_::audit::AuditEvent::KeyRotation {
                key_id: key_ring.key_id.clone(),
                old_version: format!("v{old_version}"),
                new_version: format!("v{}", new_key.metadata.version()),
                fields_reencrypted: 0,
                timestamp: chrono::Utc::now(),
            });

        Ok(RotationResult {
            key_id: key_ring.key_id.clone(),
            previous_version: old_version,
            new_version: new_key.metadata.version(),
            rotated_at: now_timestamp(),
            reencryption_required: true,
        })
    }

    pub fn get_key_info(&self, key_id: &str) -> Result<KeyInfo, ConfigError> {
        let key_ring = self
            .key_rings
            .get(key_id)
            .ok_or_else(|| ConfigError::ParseError {
                format: "key".to_string(),
                message: format!("Key ring '{}' not found", key_id),
                location: None,
                source: None,
            })?;

        Ok(KeyInfo {
            key_id: key_ring.key_id.clone(),
            current_version: key_ring.current_version,
            total_versions: key_ring.secondary_keys.len() + 1,
            active_versions: key_ring
                .secondary_keys
                .iter()
                .filter(|k| k.metadata.is_active())
                .count()
                + 1,
            deprecated_versions: key_ring
                .secondary_keys
                .iter()
                .filter(|k| k.metadata.status() == KeyStatus::Deprecated)
                .count(),
            created_at: key_ring.created_at,
            last_rotated_at: key_ring.last_rotated_at,
        })
    }

    pub fn list_keys(&self) -> Vec<KeyInfo> {
        self.key_rings
            .values()
            .map(|ring| KeyInfo {
                key_id: ring.key_id.clone(),
                current_version: ring.current_version,
                total_versions: ring.secondary_keys.len() + 1,
                active_versions: ring
                    .secondary_keys
                    .iter()
                    .filter(|k| k.metadata.is_active())
                    .count()
                    + 1,
                deprecated_versions: ring
                    .secondary_keys
                    .iter()
                    .filter(|k| k.metadata.status() == KeyStatus::Deprecated)
                    .count(),
                created_at: ring.created_at,
                last_rotated_at: ring.last_rotated_at,
            })
            .collect()
    }

    pub fn get_rotation_status(&self) -> Vec<RotationStatus> {
        self.schedules
            .values()
            .map(|schedule| {
                let key_ring = self.key_rings.get(&schedule.key_id);
                let next_rotation = schedule.next_rotation;
                let days_until = schedule.days_until_rotation();

                RotationStatus {
                    key_id: schedule.key_id.clone(),
                    current_version: key_ring.map(|r| r.current_version).unwrap_or(0),
                    rotation_interval_days: schedule.rotation_interval_days,
                    last_rotation: schedule.last_rotation,
                    next_rotation,
                    days_until_rotation: days_until,
                    is_overdue: schedule.is_rotation_due(),
                    auto_rotate: schedule.auto_rotate,
                }
            })
            .collect()
    }

    pub fn set_rotation_interval(
        &mut self,
        key_id: &str,
        interval_days: u32,
    ) -> Result<(), ConfigError> {
        let schedule = self
            .schedules
            .get_mut(key_id)
            .ok_or_else(|| ConfigError::ParseError {
                format: "key".to_string(),
                message: format!("Key ring '{}' not found", key_id),
                location: None,
                source: None,
            })?;

        schedule.rotation_interval_days = interval_days;
        schedule.next_rotation = schedule
            .last_rotation
            .saturating_add(interval_days as u64 * SECONDS_PER_DAY);

        Ok(())
    }

    pub fn plan_rotation(
        &self,
        target_version: u32,
        key_id: Option<String>,
    ) -> Result<RotationPlan, ConfigError> {
        let key_id = key_id.unwrap_or_else(|| self.default_key_id.clone());

        let key_ring = self
            .key_rings
            .get(&key_id)
            .ok_or_else(|| ConfigError::ParseError {
                format: "key".to_string(),
                message: format!("Key ring '{}' not found", key_id),
                location: None,
                source: None,
            })?;

        if target_version <= key_ring.current_version {
            return Err(ConfigError::ParseError {
                format: "key".to_string(),
                message: "Target version must be greater than current version".to_string(),
                location: None,
                source: None,
            });
        }

        Ok(RotationPlan::new(
            key_id,
            key_ring.current_version,
            target_version,
        ))
    }

    pub fn get_key_by_version(
        &self,
        key_id: &str,
        version: u32,
    ) -> Result<Option<&KeyBundle>, ConfigError> {
        let key_ring = self
            .key_rings
            .get(key_id)
            .ok_or_else(|| ConfigError::ParseError {
                format: "key".to_string(),
                message: format!("Key ring '{}' not found", key_id),
                location: None,
                source: None,
            })?;

        Ok(key_ring.get_key_by_version(version))
    }

    pub fn deprecate_version(&mut self, key_id: &str, version: u32) -> Result<(), ConfigError> {
        let key_ring = self
            .key_rings
            .get_mut(key_id)
            .ok_or_else(|| ConfigError::ParseError {
                format: "key".to_string(),
                message: format!("Key ring '{}' not found", key_id),
                location: None,
                source: None,
            })?;

        if version == key_ring.current_version {
            return Err(ConfigError::ParseError {
                format: "key".to_string(),
                message: "Cannot deprecate the current active version".to_string(),
                location: None,
                source: None,
            });
        }

        key_ring.deactivate_version(version);
        Ok(())
    }

    /// Prune retired secondary key versions, keeping at most `keep_versions`
    /// of the newest non-Active secondaries.
    ///
    /// # Security Notes
    ///
    /// - ⚠️ **Active secondary keys are NEVER removed by cleanup**: they may
    ///   still be required to decrypt data written under them (e.g. the
    ///   previous primary right after a rotation). Retire a version first
    ///   via `deprecate_version` to make it eligible for cleanup.
    ///
    /// Returns the number of removed key versions.
    pub fn cleanup_old_keys(
        &mut self,
        key_id: &str,
        keep_versions: u32,
    ) -> Result<u32, ConfigError> {
        let key_ring = self
            .key_rings
            .get_mut(key_id)
            .ok_or_else(|| ConfigError::ParseError {
                format: "key".to_string(),
                message: format!("Key ring '{}' not found", key_id),
                location: None,
                source: None,
            })?;

        // Split secondaries into Active (always preserved) and retired
        // (non-Active) versions eligible for pruning.
        let (active, mut retired): (Vec<KeyBundle>, Vec<KeyBundle>) =
            std::mem::take(&mut key_ring.secondary_keys)
                .into_iter()
                .partition(|k| k.metadata.status() == KeyStatus::Active);

        // Newest first so the head of `retired` is the set we keep.
        retired.sort_by_key(|k| std::cmp::Reverse(k.metadata.version()));

        let removed = retired.len().saturating_sub(keep_versions as usize);
        key_ring.secondary_keys = active;
        key_ring
            .secondary_keys
            .extend(retired.into_iter().take(keep_versions as usize));

        Ok(removed as u32)
    }

    pub fn get_default_key_id(&self) -> &str {
        &self.default_key_id
    }

    /// 惰性重加密助手：把旧版本信封改写为当前 Active 版本。
    ///
    /// # 语义（四分支）
    ///
    /// - 信封格式非法（非 `enc:v1:<key_version>:<payload>` 形态）→
    ///   `Err(ConfigError::InvalidValue)`；
    /// - `key_version` 仍为默认 KeyRing 的 Active 版本、策略为 `Never`、或
    ///   轮换时间超出 `dual_key_window_secs` → `Ok(None)`（原串不改写，
    ///   不产生审计事件）；
    /// - 版本已退役且在时窗内、策略为 `Lazy` → `Ok(Some(new_envelope))`，
    ///   新信封的 `key_version` 为 Active 版本，可用同一 master key + 同一
    ///   `field_path` 解密还原原文，并经端口发射一条 `AuditEvent::Decrypt`。
    ///
    /// master key 与加载管线同源：`CONFERS_MASTER_KEY`（hex 32 字节或 32
    /// 字节 ASCII），见 [`crate::field_crypto::MASTER_KEY_ENV`]。
    #[cfg(feature = "encryption")]
    pub fn reencrypt_envelope(
        &self,
        field_path: &str,
        envelope: &str,
    ) -> crate::error::ConfigResult<Option<String>> {
        use base64::Engine as _;
        use base64::engine::general_purpose::STANDARD as BASE64;

        let parsed = crate::envelope::EncryptedEnvelope::parse(envelope).ok_or_else(|| {
            ConfigError::InvalidValue {
                key: "envelope".to_string(),
                expected_type: "enc:v1:<key_version>:<payload>".to_string(),
                message: "value is not a parseable encrypted envelope".to_string(),
            }
        })?;

        let key_ring =
            self.key_rings
                .get(&self.default_key_id)
                .ok_or_else(|| ConfigError::ParseError {
                    format: "key".to_string(),
                    message: format!("Key ring '{}' not found", self.default_key_id),
                    location: None,
                    source: None,
                })?;

        let active_version = format!("v{}", key_ring.current_version);
        if parsed.key_version == active_version {
            return Ok(None);
        }
        if self.rotation_policy.reencrypt_read != crate::key::ReencryptPolicy::Lazy {
            return Ok(None);
        }
        // 退役起点取最近一次轮换时间；无从判定时（未轮换过）不改写。
        let Some(rotated_at) = key_ring.last_rotated_at else {
            return Ok(None);
        };
        if now_timestamp().saturating_sub(rotated_at) > self.rotation_policy.dual_key_window_secs {
            return Ok(None);
        }

        let Some(master_key) = crate::field_crypto::resolve_master_key() else {
            return Err(ConfigError::InvalidValue {
                key: "master_key".to_string(),
                expected_type: "32-byte key via CONFERS_MASTER_KEY".to_string(),
                message: "CONFERS_MASTER_KEY is not set to a usable 32-byte master key".to_string(),
            });
        };

        // 用旧版本派生密钥解开原文（与 field_crypto 解密路径同一套原语）。
        let old_key =
            crate::secret::crypto::derive_field_key(&master_key, field_path, &parsed.key_version)
                .map_err(|e| ConfigError::InvalidValue {
                key: "envelope.key_version".to_string(),
                expected_type: "derivable field key".to_string(),
                message: format!("field key derivation failed: {}", e),
            })?;
        let blob =
            BASE64
                .decode(parsed.payload.as_bytes())
                .map_err(|e| ConfigError::InvalidValue {
                    key: "envelope.payload".to_string(),
                    expected_type: "base64(nonce || ciphertext)".to_string(),
                    message: format!("payload is not valid base64: {}", e),
                })?;
        if blob.len() < 24 {
            return Err(ConfigError::InvalidValue {
                key: "envelope.payload".to_string(),
                expected_type: "base64(nonce || ciphertext)".to_string(),
                message: "payload shorter than a 24-byte nonce".to_string(),
            });
        }
        let (nonce, ciphertext) = blob.split_at(24);
        let plaintext = crate::secret::crypto::XChaCha20Crypto::new()
            .decrypt(nonce, ciphertext, old_key.as_slice())
            .map_err(|e| ConfigError::InvalidValue {
                key: "envelope.payload".to_string(),
                expected_type: "authentic ciphertext".to_string(),
                message: format!("reencrypt decrypt failed: {}", e),
            })?;

        // 用 Active 版本派生密钥重新加密（复用 field_crypto 的 encrypt 路径）。
        let new_key =
            crate::secret::crypto::derive_field_key(&master_key, field_path, &active_version)
                .map_err(|e| ConfigError::InvalidValue {
                    key: "envelope.key_version".to_string(),
                    expected_type: "derivable field key".to_string(),
                    message: format!("field key derivation failed: {}", e),
                })?;
        let (new_nonce, new_ciphertext) = crate::secret::crypto::XChaCha20Crypto::new()
            .encrypt(&plaintext, new_key.as_slice())
            .map_err(|e| ConfigError::InvalidValue {
                key: "envelope.payload".to_string(),
                expected_type: "encryptable plaintext".to_string(),
                message: format!("reencrypt encrypt failed: {}", e),
            })?;
        let mut new_blob = new_nonce;
        new_blob.extend_from_slice(&new_ciphertext);
        let new_envelope =
            crate::envelope::EncryptedEnvelope::new(&active_version, BASE64.encode(&new_blob))
                .to_envelope_string();

        #[cfg(feature = "audit")]
        self.audit_port
            .record(crate::impl_::audit::AuditEvent::Decrypt {
                field: field_path.to_string(),
                success: true,
                timestamp: chrono::Utc::now(),
            });

        Ok(Some(new_envelope))
    }

    pub fn set_default_key_id(&mut self, key_id: &str) -> Result<(), ConfigError> {
        if !self.key_rings.contains_key(key_id) {
            return Err(ConfigError::ParseError {
                format: "key".to_string(),
                message: format!("Key ring '{}' not found", key_id),
                location: None,
                source: None,
            });
        }
        self.default_key_id = key_id.to_string();
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct RotationStatus {
    pub key_id: String,
    pub current_version: u32,
    pub rotation_interval_days: u32,
    pub last_rotation: u64,
    pub next_rotation: u64,
    pub days_until_rotation: i64,
    pub is_overdue: bool,
    pub auto_rotate: bool,
}

#[cfg(all(test, feature = "encryption"))]
mod tests {
    use super::*;

    fn make_manager() -> KeyManager {
        KeyManager::new().expect("KeyManager::new")
    }

    fn make_manager_with_default_ring(master_key: &[u8; 32]) -> KeyManager {
        let mut km = make_manager();
        km.initialize(master_key, "prod".to_string(), "team".to_string())
            .expect("initialize");
        km
    }

    #[test]
    fn test_key_manager_new_returns_default_state() {
        let km = make_manager();
        assert_eq!(km.get_default_key_id(), "default");
        assert!(km.list_keys().is_empty());
        assert!(km.get_rotation_status().is_empty());
    }

    #[test]
    fn test_key_manager_initialize_creates_default_key_ring() {
        let master_key = [0x01; 32];
        let mut km = make_manager();
        let version = km
            .initialize(&master_key, "prod".to_string(), "team".to_string())
            .expect("initialize");

        assert_eq!(version.version, CURRENT_KEY_VERSION);
        assert_eq!(version.status, KeyStatus::Active);
        assert_eq!(version.algorithm, "XChaCha20-Poly1305");
        assert_eq!(km.get_default_key_id(), "prod");

        let info = km.get_key_info("prod").expect("get_key_info");
        assert_eq!(info.key_id, "prod");
        assert_eq!(info.current_version, CURRENT_KEY_VERSION);
        assert_eq!(info.total_versions, 1);
        assert_eq!(info.active_versions, 1);
        assert_eq!(info.deprecated_versions, 0);
    }

    #[test]
    fn test_key_manager_generate_key_returns_32_random_bytes() {
        let mut km = make_manager();
        let key1 = km.generate_key().expect("generate_key 1");
        let key2 = km.generate_key().expect("generate_key 2");

        assert_eq!(key1.len(), 32);
        assert_eq!(key2.len(), 32);
        // Two consecutive generations should differ (extremely high probability)
        assert_ne!(key1, key2, "CSPRNG produced identical keys");
    }

    #[test]
    fn test_key_manager_create_key_ring_duplicate_errors() {
        let master_key = [0x02; 32];
        let mut km = make_manager();
        km.create_key_ring(&master_key, "k1".to_string(), "u".to_string(), None)
            .expect("first create_key_ring");

        let err = km
            .create_key_ring(&master_key, "k1".to_string(), "u".to_string(), None)
            .unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("already exists"), "got: {}", msg);
    }

    #[test]
    fn test_key_manager_create_key_ring_with_description() {
        let master_key = [0x03; 32];
        let mut km = make_manager();
        km.create_key_ring(
            &master_key,
            "k1".to_string(),
            "u".to_string(),
            Some("primary key".to_string()),
        )
        .expect("create_key_ring");

        let bundle = km
            .get_key_by_version("k1", 1)
            .expect("get_key_by_version")
            .unwrap();
        assert_eq!(bundle.metadata.description(), Some("primary key"));
    }

    #[test]
    fn test_key_manager_rotate_key_default_uses_default_key_id() {
        let master_key = [0x04; 32];
        let mut km = make_manager_with_default_ring(&master_key);

        let result = km
            .rotate_key(
                &master_key,
                None,
                "rotator".to_string(),
                Some("scheduled".to_string()),
            )
            .expect("rotate_key");

        assert_eq!(result.key_id, "prod");
        assert_eq!(result.previous_version, CURRENT_KEY_VERSION);
        assert_eq!(result.new_version, CURRENT_KEY_VERSION + 1);
        assert!(result.reencryption_required);
        assert!(result.rotated_at > 0);
    }

    #[test]
    fn test_key_manager_rotate_key_explicit_key_id() {
        let master_key = [0x05; 32];
        let mut km = make_manager();
        km.create_key_ring(&master_key, "staging".to_string(), "u".to_string(), None)
            .unwrap();

        let result = km
            .rotate_key(
                &master_key,
                Some("staging".to_string()),
                "u".to_string(),
                None,
            )
            .expect("rotate_key");

        assert_eq!(result.key_id, "staging");
        assert_eq!(result.new_version, CURRENT_KEY_VERSION + 1);
    }

    #[test]
    fn test_key_manager_rotate_key_not_found_errors() {
        let master_key = [0x06; 32];
        let mut km = make_manager();
        let err = km
            .rotate_key(
                &master_key,
                Some("nonexistent".to_string()),
                "u".to_string(),
                None,
            )
            .unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("not found"), "got: {}", msg);
    }

    #[test]
    fn test_key_manager_get_key_info_not_found_errors() {
        let km = make_manager();
        let err = km.get_key_info("nonexistent").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("not found"), "got: {}", msg);
    }

    #[test]
    fn test_key_manager_list_keys_after_multiple_rings() {
        let master_key = [0x07; 32];
        let mut km = make_manager();
        km.create_key_ring(&master_key, "k1".to_string(), "u".to_string(), None)
            .unwrap();
        km.create_key_ring(&master_key, "k2".to_string(), "u".to_string(), None)
            .unwrap();

        let mut ids: Vec<String> = km.list_keys().into_iter().map(|i| i.key_id).collect();
        ids.sort();
        assert_eq!(ids, vec!["k1".to_string(), "k2".to_string()]);
    }

    #[test]
    fn test_key_manager_list_keys_counts_deprecated_correctly() {
        let master_key = [0x08; 32];
        let mut km = make_manager();
        km.create_key_ring(&master_key, "k".to_string(), "u".to_string(), None)
            .unwrap();
        // Rotate once → v1 becomes secondary
        km.rotate_key(&master_key, Some("k".to_string()), "u".to_string(), None)
            .unwrap();
        // Deprecate v1
        km.deprecate_version("k", 1).unwrap();

        let info = km.get_key_info("k").unwrap();
        assert_eq!(info.total_versions, 2);
        assert_eq!(info.current_version, 2);
        assert_eq!(info.deprecated_versions, 1);
    }

    #[test]
    fn test_key_manager_get_rotation_status_after_initialize() {
        let master_key = [0x09; 32];
        let mut km = make_manager();
        km.create_key_ring(&master_key, "k1".to_string(), "u".to_string(), None)
            .unwrap();

        let statuses = km.get_rotation_status();
        assert_eq!(statuses.len(), 1);
        assert_eq!(statuses[0].key_id, "k1");
        assert_eq!(statuses[0].current_version, CURRENT_KEY_VERSION);
        assert_eq!(statuses[0].rotation_interval_days, 90);
        assert!(statuses[0].auto_rotate);
        assert!(
            !statuses[0].is_overdue,
            "freshly created key should not be overdue"
        );
    }

    #[test]
    fn test_key_manager_set_rotation_interval_updates_next_rotation() {
        let master_key = [0x0a; 32];
        let mut km = make_manager();
        km.create_key_ring(&master_key, "k".to_string(), "u".to_string(), None)
            .unwrap();

        let original_status = km.get_rotation_status().into_iter().next().unwrap();
        km.set_rotation_interval("k", 30)
            .expect("set_rotation_interval");
        let new_status = km.get_rotation_status().into_iter().next().unwrap();

        assert_eq!(new_status.rotation_interval_days, 30);
        assert_ne!(new_status.next_rotation, original_status.next_rotation);
    }

    #[test]
    fn test_key_manager_set_rotation_interval_not_found_errors() {
        let mut km = make_manager();
        let err = km.set_rotation_interval("nonexistent", 30).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("not found"), "got: {}", msg);
    }

    #[test]
    fn test_key_manager_plan_rotation_for_default_key() {
        let master_key = [0x0b; 32];
        let km = make_manager_with_default_ring(&master_key);
        let plan = km.plan_rotation(3, None).expect("plan_rotation");

        assert_eq!(plan.key_id, "prod");
        assert_eq!(plan.current_version, CURRENT_KEY_VERSION);
        assert_eq!(plan.target_version, 3);
        assert_eq!(plan.keys_to_rotate, vec![2, 3]);
        assert!(plan.reencryption_required);
    }

    #[test]
    fn test_key_manager_plan_rotation_target_not_greater_errors() {
        let master_key = [0x0c; 32];
        let km = make_manager_with_default_ring(&master_key);
        let err = km.plan_rotation(1, None).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("must be greater than current version"),
            "got: {}",
            msg
        );
    }

    #[test]
    fn test_key_manager_plan_rotation_not_found_errors() {
        let km = make_manager();
        let err = km
            .plan_rotation(5, Some("nonexistent".to_string()))
            .unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("not found"), "got: {}", msg);
    }

    #[test]
    fn test_key_manager_get_key_by_version_returns_primary() {
        let master_key = [0x0d; 32];
        let km = make_manager_with_default_ring(&master_key);
        let found = km
            .get_key_by_version("prod", CURRENT_KEY_VERSION)
            .expect("get_key_by_version");
        assert!(found.is_some());
        assert_eq!(found.unwrap().metadata.version(), CURRENT_KEY_VERSION);
    }

    #[test]
    fn test_key_manager_get_key_by_version_returns_none_for_missing_version() {
        let master_key = [0x0e; 32];
        let km = make_manager_with_default_ring(&master_key);
        let found = km
            .get_key_by_version("prod", 999)
            .expect("get_key_by_version");
        assert!(found.is_none());
    }

    #[test]
    fn test_key_manager_get_key_by_version_not_found_errors() {
        let km = make_manager();
        let err = km.get_key_by_version("nonexistent", 1).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("not found"), "got: {}", msg);
    }

    #[test]
    fn test_key_manager_deprecate_version_succeeds_for_secondary() {
        let master_key = [0x0f; 32];
        let mut km = make_manager();
        km.create_key_ring(&master_key, "k".to_string(), "u".to_string(), None)
            .unwrap();
        km.rotate_key(&master_key, Some("k".to_string()), "u".to_string(), None)
            .unwrap();

        km.deprecate_version("k", 1).expect("deprecate_version");

        let v1 = km
            .get_key_by_version("k", 1)
            .expect("get_key_by_version")
            .unwrap();
        assert_eq!(v1.metadata.status(), KeyStatus::Deprecated);
    }

    #[test]
    fn test_key_manager_deprecate_current_version_errors() {
        let master_key = [0x10; 32];
        let mut km = make_manager_with_default_ring(&master_key);
        let err = km
            .deprecate_version("prod", CURRENT_KEY_VERSION)
            .unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("Cannot deprecate the current active version"),
            "got: {}",
            msg
        );
    }

    #[test]
    fn test_key_manager_deprecate_version_not_found_errors() {
        let mut km = make_manager();
        let err = km.deprecate_version("nonexistent", 1).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("not found"), "got: {}", msg);
    }

    #[test]
    fn test_key_manager_cleanup_old_keys_returns_zero_when_under_threshold() {
        let master_key = [0x11; 32];
        let mut km = make_manager();
        km.create_key_ring(&master_key, "k".to_string(), "u".to_string(), None)
            .unwrap();
        // No secondaries → keep_versions=5 keeps everything
        let removed = km.cleanup_old_keys("k", 5).expect("cleanup_old_keys");
        assert_eq!(removed, 0);
    }

    #[test]
    fn test_key_manager_cleanup_old_keys_removes_retired_keeping_newest() {
        let master_key = [0x12; 32];
        let mut km = make_manager();
        km.create_key_ring(&master_key, "k".to_string(), "u".to_string(), None)
            .unwrap();
        // Rotate 3 times → secondaries = [v1, v2, v3], primary = v4
        for _ in 0..3 {
            km.rotate_key(&master_key, Some("k".to_string()), "u".to_string(), None)
                .unwrap();
        }
        assert_eq!(km.get_key_info("k").unwrap().total_versions, 4);

        // Retire v1 and v2; v3 stays Active.
        km.deprecate_version("k", 1).unwrap();
        km.deprecate_version("k", 2).unwrap();

        // keep_versions=1 → keep the newest retired version (v2), remove v1.
        // The Active secondary v3 must never be touched.
        let removed = km.cleanup_old_keys("k", 1).expect("cleanup_old_keys");
        assert_eq!(removed, 1);
        assert!(
            km.get_key_by_version("k", 1).expect("get").is_none(),
            "oldest retired version must be removed"
        );
        assert!(
            km.get_key_by_version("k", 2).expect("get").is_some(),
            "newest retired version must be kept"
        );
        assert!(
            km.get_key_by_version("k", 3).expect("get").is_some(),
            "Active secondary must never be removed"
        );
        assert!(
            km.get_key_by_version("k", 4).expect("get").is_some(),
            "primary must survive cleanup"
        );
    }

    #[test]
    fn test_key_manager_cleanup_old_keys_never_removes_active_secondaries() {
        let master_key = [0x15; 32];
        let mut km = make_manager();
        km.create_key_ring(&master_key, "k".to_string(), "u".to_string(), None)
            .unwrap();
        // Rotate 3 times → secondaries = [v1, v2, v3] all Active, primary = v4.
        for _ in 0..3 {
            km.rotate_key(&master_key, Some("k".to_string()), "u".to_string(), None)
                .unwrap();
        }

        // Even with keep_versions=1, Active secondaries are preserved: they
        // may still be required to decrypt data written under them.
        let removed = km.cleanup_old_keys("k", 1).expect("cleanup_old_keys");
        assert_eq!(removed, 0);
        for version in 1..=3 {
            assert!(
                km.get_key_by_version("k", version).expect("get").is_some(),
                "Active secondary v{} must survive cleanup",
                version
            );
        }
    }

    #[test]
    fn test_key_manager_cleanup_old_keys_not_found_errors() {
        let mut km = make_manager();
        let err = km.cleanup_old_keys("nonexistent", 5).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("not found"), "got: {}", msg);
    }

    #[test]
    fn test_key_manager_get_default_key_id_after_initialize() {
        let master_key = [0x13; 32];
        let mut km = make_manager();
        km.create_key_ring(&master_key, "first".to_string(), "u".to_string(), None)
            .unwrap();
        // create_key_ring does NOT change default; default is still "default" until set
        assert_eq!(km.get_default_key_id(), "default");

        km.set_default_key_id("first").expect("set_default_key_id");
        assert_eq!(km.get_default_key_id(), "first");
    }

    #[test]
    fn test_key_manager_set_default_key_id_not_found_errors() {
        let mut km = make_manager();
        let err = km.set_default_key_id("nonexistent").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("not found"), "got: {}", msg);
    }

    #[test]
    fn test_key_manager_rotate_key_updates_rotation_schedule() {
        let master_key = [0x14; 32];
        let mut km = make_manager();
        km.create_key_ring(&master_key, "k".to_string(), "u".to_string(), None)
            .unwrap();
        let pre_status = km.get_rotation_status().into_iter().next().unwrap();

        km.rotate_key(&master_key, Some("k".to_string()), "u".to_string(), None)
            .unwrap();
        let post_status = km.get_rotation_status().into_iter().next().unwrap();

        // last_rotation advances after rotate_key (which calls schedule.update_after_rotation)
        assert!(post_status.last_rotation >= pre_status.last_rotation);
        assert!(post_status.next_rotation >= pre_status.next_rotation);
    }

    #[test]
    fn test_key_manager_rotate_key_enforces_schedule_max_versions() {
        let master_key = [0x16; 32];
        let mut km = make_manager();
        km.create_key_ring(&master_key, "k".to_string(), "u".to_string(), None)
            .unwrap();
        // Default schedule max_versions = 5; rotate 7 times. Without
        // enforcement `secondary_keys` would grow to 7.
        for _ in 0..7 {
            km.rotate_key(&master_key, Some("k".to_string()), "u".to_string(), None)
                .unwrap();
        }

        let info = km.get_key_info("k").unwrap();
        assert_eq!(
            info.total_versions, 6,
            "expected primary + 5 capped secondaries"
        );
        // With no retired versions available, the oldest Active secondaries
        // were pruned as a last resort to honor the policy bound.
        assert!(km.get_key_by_version("k", 1).expect("get").is_none());
        assert!(km.get_key_by_version("k", 2).expect("get").is_none());
        assert!(km.get_key_by_version("k", 3).expect("get").is_some());
        assert!(km.get_key_by_version("k", 7).expect("get").is_some());
    }

    #[test]
    fn test_key_manager_rotate_key_prunes_retired_versions_first() {
        let master_key = [0x17; 32];
        let mut km = make_manager();
        km.create_key_ring(&master_key, "k".to_string(), "u".to_string(), None)
            .unwrap();
        for _ in 0..5 {
            km.rotate_key(&master_key, Some("k".to_string()), "u".to_string(), None)
                .unwrap();
        }
        // Retire v1 so the cap has a preferred victim.
        km.deprecate_version("k", 1).unwrap();

        // This rotation pushes secondaries to 6 (> max_versions 5): the
        // retired v1 must be pruned instead of any Active version.
        km.rotate_key(&master_key, Some("k".to_string()), "u".to_string(), None)
            .unwrap();

        assert!(km.get_key_by_version("k", 1).expect("get").is_none());
        for version in 2..=6 {
            assert!(
                km.get_key_by_version("k", version).expect("get").is_some(),
                "Active secondary v{} must survive the cap",
                version
            );
        }
    }

    #[test]
    fn test_key_manager_debug_redacts_key_material() {
        let master_key = [0x18; 32];
        let mut km = make_manager();
        km.create_key_ring(&master_key, "k".to_string(), "u".to_string(), None)
            .unwrap();
        km.rotate_key(&master_key, Some("k".to_string()), "u".to_string(), None)
            .unwrap();

        let debug = format!("{:?}", km);
        assert!(debug.contains("KeyManager"), "got: {}", debug);
        // Encrypted key material of both the primary and secondary versions
        // must never appear in Debug output.
        for version in 1..=2 {
            let encrypted = km
                .get_key_by_version("k", version)
                .expect("get")
                .unwrap()
                .encrypted_key
                .clone();
            assert!(
                !debug.contains(&encrypted),
                "Debug leaked key material: {}",
                debug
            );
        }
    }

    #[test]
    fn test_key_version_struct_construction() {
        let v = KeyVersion {
            id: "k_v1".to_string(),
            version: 1,
            created_at: 1234,
            status: KeyStatus::Active,
            algorithm: "XChaCha20-Poly1305".to_string(),
        };
        assert_eq!(v.id, "k_v1");
        assert_eq!(v.version, 1);
        assert_eq!(v.created_at, 1234);
        assert_eq!(v.status, KeyStatus::Active);
        assert_eq!(v.algorithm, "XChaCha20-Poly1305");
    }

    #[test]
    fn test_key_version_serialize_deserialize() {
        let v = KeyVersion {
            id: "k_v1".to_string(),
            version: 1,
            created_at: 0,
            status: KeyStatus::Deprecated,
            algorithm: "XChaCha20-Poly1305".to_string(),
        };
        let json = serde_json::to_string(&v).expect("serialize");
        let de: KeyVersion = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(de.id, v.id);
        assert_eq!(de.version, v.version);
        assert_eq!(de.status, v.status);
    }

    #[test]
    fn test_key_info_struct_construction() {
        let info = KeyInfo {
            key_id: "k1".to_string(),
            current_version: 5,
            total_versions: 8,
            active_versions: 6,
            deprecated_versions: 2,
            created_at: 100,
            last_rotated_at: Some(200),
        };
        assert_eq!(info.key_id, "k1");
        assert_eq!(info.current_version, 5);
        assert_eq!(info.total_versions, 8);
        assert_eq!(info.active_versions, 6);
        assert_eq!(info.deprecated_versions, 2);
        assert_eq!(info.last_rotated_at, Some(200));
    }

    #[test]
    fn test_rotation_status_struct_construction() {
        let status = RotationStatus {
            key_id: "k1".to_string(),
            current_version: 3,
            rotation_interval_days: 30,
            last_rotation: 100,
            next_rotation: 200,
            days_until_rotation: 10,
            is_overdue: false,
            auto_rotate: true,
        };
        assert_eq!(status.key_id, "k1");
        assert_eq!(status.current_version, 3);
        assert_eq!(status.rotation_interval_days, 30);
        assert!(!status.is_overdue);
        assert!(status.auto_rotate);
    }

    /// 按加载管线同款原语构造信封（field 派生密钥 + 随机 nonce）。
    #[cfg(feature = "encryption")]
    fn make_field_envelope(
        master: &[u8],
        field: &str,
        key_version: &str,
        plaintext: &str,
    ) -> String {
        use base64::Engine as _;
        use base64::engine::general_purpose::STANDARD as BASE64;

        let field_key =
            crate::secret::crypto::derive_field_key(master, field, key_version).unwrap();
        let (nonce, ct) = crate::secret::crypto::XChaCha20Crypto::new()
            .encrypt(plaintext.as_bytes(), field_key.as_slice())
            .unwrap();
        let mut blob = nonce;
        blob.extend_from_slice(&ct);
        crate::envelope::EncryptedEnvelope::new(key_version, BASE64.encode(&blob))
            .to_envelope_string()
    }

    #[cfg(feature = "encryption")]
    #[test]
    #[serial_test::serial]
    fn test_reencrypt_envelope_four_branches() {
        let master: [u8; 32] = std::array::from_fn(|i| (i as u8) ^ 0x33);
        unsafe { std::env::set_var(crate::field_crypto::MASTER_KEY_ENV, hex::encode(master)) };

        let lazy_policy = crate::key::KeyRotationPolicy {
            reencrypt_read: crate::key::ReencryptPolicy::Lazy,
            ..crate::key::KeyRotationPolicy::default()
        };

        // 分支 1：key_version 仍为 Active → Ok(None)。
        let mut km = KeyManager::new()
            .unwrap()
            .with_rotation_policy(lazy_policy.clone());
        km.initialize(&master, "prod".to_string(), "team".to_string())
            .expect("initialize");
        let env_v1 = make_field_envelope(&master, "api_key", "v1", "tok-abc");
        assert_eq!(
            km.reencrypt_envelope("api_key", &env_v1).expect("branch 1"),
            None,
            "active version must not be rewritten"
        );

        // 分支 3：策略 Never（默认策略）→ Ok(None)。
        let mut km_never = KeyManager::new().unwrap();
        km_never
            .initialize(&master, "prod".to_string(), "team".to_string())
            .expect("initialize");
        km_never
            .rotate_key(&master, None, "u".to_string(), None)
            .expect("rotate");
        assert_eq!(
            km_never
                .reencrypt_envelope("api_key", &env_v1)
                .expect("branch 3"),
            None,
            "Never policy must not rewrite"
        );

        // 分支 2：版本退役 + Lazy + 时窗内 → Ok(Some)，roundtrip 还原原文。
        km.rotate_key(&master, None, "u".to_string(), None)
            .expect("rotate");
        let rewritten = km
            .reencrypt_envelope("api_key", &env_v1)
            .expect("branch 2")
            .expect("lazy rewrite must return a new envelope");
        let parsed = crate::envelope::EncryptedEnvelope::parse(&rewritten).unwrap();
        assert_eq!(
            parsed.key_version, "v2",
            "rewritten envelope uses active version"
        );

        // 新信封用 Active 版本密钥 + 同一 field_path 解密还原原文。
        let field_key = crate::secret::crypto::derive_field_key(&master, "api_key", "v2").unwrap();
        use base64::Engine as _;
        use base64::engine::general_purpose::STANDARD as BASE64;
        let blob = BASE64.decode(parsed.payload.as_bytes()).unwrap();
        let (nonce, ct) = blob.split_at(24);
        let plaintext = crate::secret::crypto::XChaCha20Crypto::new()
            .decrypt(nonce, ct, field_key.as_slice())
            .expect("active key must decrypt rewritten envelope");
        assert_eq!(plaintext, b"tok-abc".to_vec().into());

        // 时窗外：把最近轮换时间拨回 20 天前（默认时窗 14 天）→ Ok(None)。
        let ring = km.key_rings.get_mut("prod").unwrap();
        ring.last_rotated_at = Some(now_timestamp().saturating_sub(20 * SECONDS_PER_DAY));
        assert_eq!(
            km.reencrypt_envelope("api_key", &env_v1)
                .expect("window branch"),
            None,
            "out-of-window envelope must not be rewritten"
        );

        // 分支 4：非法格式 → Err(InvalidValue)。
        let err = km
            .reencrypt_envelope("api_key", "not-an-envelope")
            .unwrap_err();
        assert!(err.to_string().contains("envelope"), "got: {}", err);

        unsafe { std::env::remove_var(crate::field_crypto::MASTER_KEY_ENV) };
    }

    #[cfg(all(feature = "encryption", feature = "audit"))]
    #[test]
    #[serial_test::serial]
    fn test_reencrypt_envelope_none_paths_emit_no_audit_event() {
        let master: [u8; 32] = std::array::from_fn(|i| (i as u8) ^ 0x44);
        unsafe { std::env::set_var(crate::field_crypto::MASTER_KEY_ENV, hex::encode(master)) };

        let events = std::sync::Arc::new(std::sync::RwLock::new(Vec::new()));
        let lazy_policy = crate::key::KeyRotationPolicy {
            reencrypt_read: crate::key::ReencryptPolicy::Lazy,
            ..crate::key::KeyRotationPolicy::default()
        };
        let mut km = KeyManager::new()
            .unwrap()
            .with_rotation_policy(lazy_policy)
            .with_audit_port(std::sync::Arc::new(CapturingPort(events.clone())));
        km.initialize(&master, "prod".to_string(), "team".to_string())
            .expect("initialize");
        km.rotate_key(&master, None, "u".to_string(), None)
            .expect("rotate");

        // rotate_key 自身发射 1 条 KeyRotation 事件；此前的 Ok(None) 不发射。
        let after_rotate = events.read().expect("capture lock").len();
        assert_eq!(after_rotate, 1);

        let env_v3_unknown_active = make_field_envelope(&master, "api_key", "v1", "tok");
        assert!(
            km.reencrypt_envelope("api_key", &env_v3_unknown_active)
                .expect("lazy rewrite")
                .is_some()
        );
        let captured = events.read().expect("capture lock").clone();
        assert_eq!(captured.len(), 2, "rewrite adds exactly one Decrypt event");
        assert!(matches!(
            &captured[1],
            crate::impl_::audit::AuditEvent::Decrypt { field, success: true, .. } if field == "api_key"
        ));

        unsafe { std::env::remove_var(crate::field_crypto::MASTER_KEY_ENV) };
    }

    #[test]
    fn test_key_manager_debug_clone_serialize() {
        let km = make_manager();
        let cloned = km.clone();
        assert_eq!(km.get_default_key_id(), cloned.get_default_key_id());
        let _debug = format!("{:?}", km);
        // Serialize derives Serialize but not Deserialize for KeyManager
        let json = serde_json::to_string(&km).expect("serialize");
        assert!(json.contains("default"));
    }

    /// 内存捕获端口：记录全部到达的审计事件供断言。
    #[cfg(feature = "audit")]
    struct CapturingPort(std::sync::Arc<std::sync::RwLock<Vec<crate::impl_::audit::AuditEvent>>>);

    #[cfg(feature = "audit")]
    impl crate::key::audit_port::RotationAuditPort for CapturingPort {
        fn record(&self, event: crate::impl_::audit::AuditEvent) {
            self.0.write().expect("capture lock").push(event);
        }
    }

    #[cfg(feature = "audit")]
    #[test]
    fn test_rotate_key_emits_exactly_one_rotation_event() {
        let master_key = [0x19; 32];
        let events = std::sync::Arc::new(std::sync::RwLock::new(Vec::new()));
        let mut km =
            make_manager().with_audit_port(std::sync::Arc::new(CapturingPort(events.clone())));
        km.initialize(&master_key, "prod".to_string(), "team".to_string())
            .expect("initialize");

        let result = km
            .rotate_key(
                &master_key,
                Some("prod".to_string()),
                "rotator".to_string(),
                Some("scheduled".to_string()),
            )
            .expect("rotate_key");

        let captured = events.read().expect("capture lock").clone();
        assert_eq!(captured.len(), 1, "success must emit exactly one event");
        match &captured[0] {
            crate::impl_::audit::AuditEvent::KeyRotation {
                key_id,
                old_version,
                new_version,
                fields_reencrypted,
                ..
            } => {
                assert_eq!(key_id, "prod");
                assert_eq!(*old_version, format!("v{}", result.previous_version));
                assert_eq!(*new_version, format!("v{}", result.new_version));
                assert_eq!(*fields_reencrypted, 0);
            }
            other => panic!("expected KeyRotation event, got {:?}", other),
        }
    }

    #[cfg(feature = "audit")]
    #[test]
    fn test_rotate_key_failure_emits_no_event() {
        let master_key = [0x1a; 32];
        let events = std::sync::Arc::new(std::sync::RwLock::new(Vec::new()));
        let mut km =
            make_manager().with_audit_port(std::sync::Arc::new(CapturingPort(events.clone())));
        km.initialize(&master_key, "prod".to_string(), "team".to_string())
            .expect("initialize");

        // 轮换不存在的 key ring → 失败路径零事件。
        let err = km
            .rotate_key(
                &master_key,
                Some("nonexistent".to_string()),
                "u".to_string(),
                None,
            )
            .unwrap_err();
        assert!(err.to_string().contains("not found"));
        assert!(
            events.read().expect("capture lock").is_empty(),
            "failure must not emit events"
        );
    }
}
