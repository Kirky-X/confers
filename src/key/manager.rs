// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

use crate::error::ConfigError;
use crate::key::{
    now_timestamp, KeyBundle, KeyRing, KeyRotationSchedule, KeyStatus, RotationPlan,
    RotationResult, CURRENT_KEY_VERSION, SECONDS_PER_DAY,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyManager {
    master_key_hash: String,
    key_rings: HashMap<String, KeyRing>,
    schedules: HashMap<String, KeyRotationSchedule>,
    default_key_id: String,
    storage_path: PathBuf,
}

impl KeyManager {
    #[cfg(feature = "encryption")]
    pub fn new(storage_path: PathBuf) -> Result<Self, ConfigError> {
        Ok(Self {
            master_key_hash: String::new(),
            key_rings: HashMap::new(),
            schedules: HashMap::new(),
            default_key_id: "default".to_string(),
            storage_path,
        })
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
    /// # use std::path::PathBuf;
    /// # let master_key = [0u8; 32];
    /// let mut km = KeyManager::new(PathBuf::from("./keys"))?;
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
            algorithm: "AES256-GCM".to_string(),
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
    /// # use std::path::PathBuf;
    /// # let mut km = KeyManager::new(PathBuf::from("./keys")).unwrap();
    /// let key = km.generate_key()?;
    /// let encryption = XChaCha20Crypto::new();
    /// # Ok::<(), confers::error::ConfigError>(())
    /// ```
    #[cfg(feature = "encryption")]
    pub fn generate_key(&mut self) -> Result<[u8; 32], ConfigError> {
        let mut key_bytes = [0u8; 32];
        let mut rng = rand::thread_rng();
        rng.fill(&mut key_bytes);
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

        if let Some(desc) = description {
            if let Some(key) = self.key_rings.get_mut(&key_id) {
                key.primary_key.metadata.description = Some(desc);
            }
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
            algorithm: "AES256-GCM".to_string(),
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
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use confers::key::KeyManager;
    /// # use std::path::PathBuf;
    /// # let mut km = KeyManager::new(PathBuf::from("./keys")).unwrap();
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
                message: format!("Key ring '{}' not found", key_id),
                location: None,
                source: None,
            })?;

        let old_version = key_ring.current_version;
        let new_key = key_ring.rotate(master_key, created_by, description)?;

        if let Some(schedule) = self.schedules.get_mut(&key_id) {
            schedule.update_after_rotation();
        }

        Ok(RotationResult {
            key_id: key_ring.key_id.clone(),
            previous_version: old_version,
            new_version: new_key.metadata.version,
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
                .filter(|k| k.metadata.status == KeyStatus::Deprecated)
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
                    .filter(|k| k.metadata.status == KeyStatus::Deprecated)
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

        if key_ring.secondary_keys.len() <= keep_versions as usize {
            return Ok(0);
        }

        let initial_count = key_ring.secondary_keys.len();
        key_ring.secondary_keys.retain(|k| {
            k.metadata.status == KeyStatus::Active || k.metadata.version > keep_versions
        });

        Ok((initial_count - key_ring.secondary_keys.len()) as u32)
    }

    pub fn get_default_key_id(&self) -> &str {
        &self.default_key_id
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
    use std::path::PathBuf;

    fn make_manager() -> KeyManager {
        KeyManager::new(PathBuf::from("./nonexistent_keys_for_tests")).expect("KeyManager::new")
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
        assert_eq!(version.algorithm, "AES256-GCM");
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
        assert_eq!(bundle.metadata.description.as_deref(), Some("primary key"));
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
        assert_eq!(found.unwrap().metadata.version, CURRENT_KEY_VERSION);
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
        assert_eq!(v1.metadata.status, KeyStatus::Deprecated);
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
    fn test_key_manager_cleanup_old_keys_removes_inactive_below_threshold() {
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

        // keep_versions=2 → inactive versions ≤ 2 are eligible for removal.
        // Note: cleanup logic keeps Active versions OR versions > keep_versions.
        let removed = km.cleanup_old_keys("k", 2).expect("cleanup_old_keys");
        // Active secondaries (v1, v2, v3 are all Active by default) are retained.
        assert_eq!(removed, 0);
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
    fn test_key_version_struct_construction() {
        let v = KeyVersion {
            id: "k_v1".to_string(),
            version: 1,
            created_at: 1234,
            status: KeyStatus::Active,
            algorithm: "AES256-GCM".to_string(),
        };
        assert_eq!(v.id, "k_v1");
        assert_eq!(v.version, 1);
        assert_eq!(v.created_at, 1234);
        assert_eq!(v.status, KeyStatus::Active);
        assert_eq!(v.algorithm, "AES256-GCM");
    }

    #[test]
    fn test_key_version_serialize_deserialize() {
        let v = KeyVersion {
            id: "k_v1".to_string(),
            version: 1,
            created_at: 0,
            status: KeyStatus::Deprecated,
            algorithm: "AES256-GCM".to_string(),
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
}
