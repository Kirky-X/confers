// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

use crate::error::ConfigError;
use crate::key::{KeyMetadata, KeyRing, KeyStatus, RotationPlan, SECONDS_PER_DAY, now_timestamp};
use serde::{Deserialize, Serialize};

const CRITICAL_EXPIRY_DAYS: u64 = 7;
const WARNING_EXPIRY_DAYS: u64 = 30;

/// Maximum number of rotation history entries retained in memory by
/// [`KeyRotationService`]. When the cap is exceeded, the oldest entries are
/// dropped first so long-lived services cannot grow the history without
/// bound.
#[cfg(feature = "encryption")]
const MAX_ROTATION_HISTORY: usize = 100;

/// 惰性重加密策略：旧版本密文何时被改写为新版本。
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReencryptPolicy {
    /// 旧版本密文保持原样（默认，行为兼容）。
    #[default]
    Never,
    /// 解密方主动调用 `KeyManager::reencrypt_envelope` 时改写。
    Lazy,
    /// rotate_key 成功后立刻重加密 KeyRing 记录的字段清单。
    /// 字段账本尚未就绪，`validate_rotation`/`can_rotate` 会拒绝该策略。
    OnRotate,
}

/// `dual_key_window_secs` 的 serde 缺省值。serde 默认函数没有 self 上下文，
/// 无法引用 `grace_period_days`，取保守的固定 1 天（1 小时时窗不足以完成
/// 轮换）；`KeyRotationPolicy::new`/`Default` 显式按 `grace_period_days * 86400`
/// 计算，可被 builder 覆盖。
fn default_dual_key_window() -> u64 {
    86_400
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyRotationPolicy {
    pub max_versions: u32,
    pub rotation_interval_days: u32,
    pub grace_period_days: u32,
    pub auto_rotate: bool,
    pub notify_before_expiry_days: u32,
    /// 新旧版本并存时窗（秒）。超出后旧版本密文不再被惰性重加密。
    #[serde(default = "default_dual_key_window")]
    pub dual_key_window_secs: u64,
    /// 新写入是否使用 Active 版本加密。
    #[serde(default = "default_true")]
    pub reencrypt_on_write: bool,
    /// 读路径惰性重加密策略（默认 `Never`，行为兼容）。
    #[serde(default)]
    pub reencrypt_read: ReencryptPolicy,
}

impl Default for KeyRotationPolicy {
    fn default() -> Self {
        Self {
            max_versions: 5,
            rotation_interval_days: 90,
            grace_period_days: 14,
            auto_rotate: false,
            notify_before_expiry_days: 30,
            dual_key_window_secs: 14 * 86_400,
            reencrypt_on_write: true,
            reencrypt_read: ReencryptPolicy::Never,
        }
    }
}

impl KeyRotationPolicy {
    pub fn new(
        max_versions: u32,
        rotation_interval_days: u32,
        grace_period_days: u32,
        auto_rotate: bool,
    ) -> Self {
        Self {
            max_versions,
            rotation_interval_days,
            grace_period_days,
            auto_rotate,
            notify_before_expiry_days: 30,
            dual_key_window_secs: grace_period_days as u64 * 86_400,
            reencrypt_on_write: true,
            reencrypt_read: ReencryptPolicy::Never,
        }
    }

    pub fn with_auto_rotate(mut self, auto_rotate: bool) -> Self {
        self.auto_rotate = auto_rotate;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RotationResult {
    pub key_id: String,
    pub previous_version: u32,
    pub new_version: u32,
    pub rotated_at: u64,
    pub reencryption_required: bool,
}

impl RotationResult {
    pub fn new(
        key_id: String,
        previous_version: u32,
        new_version: u32,
        rotated_at: u64,
        reencryption_required: bool,
    ) -> Self {
        Self {
            key_id,
            previous_version,
            new_version,
            rotated_at,
            reencryption_required,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RotationHistory {
    pub rotation_id: String,
    pub key_id: String,
    pub from_version: u32,
    pub to_version: u32,
    pub rotated_at: u64,
    pub rotated_by: String,
    pub reason: Option<String>,
    pub reencryption_count: u32,
    pub status: RotationStatus,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RotationStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Cancelled,
}

impl std::fmt::Display for RotationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RotationStatus::Pending => write!(f, "pending"),
            RotationStatus::InProgress => write!(f, "in_progress"),
            RotationStatus::Completed => write!(f, "completed"),
            RotationStatus::Failed => write!(f, "failed"),
            RotationStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

#[derive(Clone)]
pub struct KeyRotationService {
    policy: KeyRotationPolicy,
    /// In-memory audit trail of executed rotations. Capped at
    /// `MAX_ROTATION_HISTORY` entries; the oldest entries are dropped first.
    history: Vec<RotationHistory>,
    /// 轮换审计出口端口；未注入时为 Noop（零行为变化）。
    #[cfg(feature = "audit")]
    audit_port: std::sync::Arc<dyn crate::key::audit_port::RotationAuditPort>,
}

impl std::fmt::Debug for KeyRotationService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // 手写 Debug：端口是 trait 对象（非 Debug），且 history 只暴露条数。
        f.debug_struct("KeyRotationService")
            .field("policy", &self.policy)
            .field("history_len", &self.history.len())
            .finish_non_exhaustive()
    }
}

/// Number of secondary key versions that are no longer Active.
///
/// Shared by `execute_rotation` and `can_rotate` so both use identical
/// accounting for how many retired versions a key ring retains.
fn inactive_secondary_count(key_ring: &KeyRing) -> u32 {
    key_ring
        .secondary_keys
        .iter()
        .filter(|k| k.metadata.status() != KeyStatus::Active)
        .count() as u32
}

impl KeyRotationService {
    pub fn new(policy: KeyRotationPolicy) -> Self {
        Self {
            policy,
            history: Vec::new(),
            #[cfg(feature = "audit")]
            audit_port: std::sync::Arc::new(crate::key::audit_port::NoopRotationAudit),
        }
    }

    /// 注入轮换审计出口端口（builder 风格）。
    ///
    /// 端口在 `execute_rotation` 成功（`RotationStatus::Completed`）后收到
    /// 一条 `AuditEvent::KeyRotation`；未注入时保持 Noop。
    #[cfg(feature = "audit")]
    pub fn with_audit_port(
        mut self,
        port: std::sync::Arc<dyn crate::key::audit_port::RotationAuditPort>,
    ) -> Self {
        self.audit_port = port;
        self
    }

    pub fn create_rotation_plan(
        key_ring: &KeyRing,
        target_version: u32,
    ) -> Result<RotationPlan, ConfigError> {
        if target_version <= key_ring.current_version {
            return Err(ConfigError::ParseError {
                format: "key".to_string(),
                message: "Target version must be greater than current version".to_string(),
                location: None,
                source: None,
            });
        }

        let plan = RotationPlan::new(
            key_ring.key_id.clone(),
            key_ring.current_version,
            target_version,
        );

        Ok(plan)
    }

    /// Validate a rotation plan against a key ring and policy.
    ///
    /// # Semantics
    ///
    /// A [`RotationPlan`] (see [`RotationPlan::new`] and
    /// [`KeyRotationService::create_rotation_plan`]) lists the *future*
    /// versions `current_version + 1 ..= target_version` that the rotation
    /// will create. Validation therefore checks that:
    ///
    /// - `plan.current_version` exists in the key ring;
    /// - every version in `plan.keys_to_rotate` does **not** exist yet (it
    ///   will be created by the rotation);
    /// - `plan.keys_to_rotate` is contiguous and strictly ascending, starting
    ///   at `plan.current_version + 1`;
    /// - the target stays within `policy.max_versions` of the ring's current
    ///   version.
    ///
    /// Note: this function currently has no production callers; it is part of
    /// the rotation-planning API.
    pub fn validate_rotation(
        key_ring: &KeyRing,
        plan: &RotationPlan,
        policy: &KeyRotationPolicy,
    ) -> Result<(), ConfigError> {
        // OnRotate 需要 KeyRing 携带"哪些字段用此密钥加密"的字段账本；账本
        // 未就绪前显式拒绝，避免策略被静默忽略。
        if policy.reencrypt_read == ReencryptPolicy::OnRotate {
            return Err(ConfigError::InvalidValue {
                key: "key.rotation".to_string(),
                expected_type: "never | lazy".to_string(),
                message: "reencrypt_read=on_rotate requires the per-field ledger, \
                          which is not implemented yet"
                    .to_string(),
            });
        }

        if plan.target_version > key_ring.current_version + policy.max_versions {
            return Err(ConfigError::ParseError {
                format: "key".to_string(),
                message: format!(
                    "Rotation would exceed max versions ({}). Current: {}, Target: {}",
                    policy.max_versions, key_ring.current_version, plan.target_version
                ),
                location: None,
                source: None,
            });
        }

        if key_ring.get_key_by_version(plan.current_version).is_none() {
            return Err(ConfigError::ParseError {
                format: "key".to_string(),
                message: format!(
                    "Plan current version {} not found in key ring '{}'",
                    plan.current_version, key_ring.key_id
                ),
                location: None,
                source: None,
            });
        }

        let mut expected = plan.current_version;
        for version in &plan.keys_to_rotate {
            if *version != expected + 1 {
                return Err(ConfigError::ParseError {
                    format: "key".to_string(),
                    message: format!(
                        "Plan versions must be contiguous and ascending from current_version + 1 ({}), got {}",
                        plan.current_version + 1,
                        version
                    ),
                    location: None,
                    source: None,
                });
            }
            expected = *version;
            if key_ring.get_key_by_version(*version).is_some() {
                return Err(ConfigError::ParseError {
                    format: "key".to_string(),
                    message: format!(
                        "Key version {} already exists in key ring '{}'; keys_to_rotate must list future versions that the rotation will create",
                        version, key_ring.key_id
                    ),
                    location: None,
                    source: None,
                });
            }
        }

        Ok(())
    }

    #[cfg(feature = "encryption")]
    pub fn execute_rotation(
        &mut self,
        key_ring: &mut KeyRing,
        master_key: &[u8; 32],
        rotated_by: String,
        reason: Option<String>,
    ) -> Result<RotationResult, ConfigError> {
        // Enforce max_versions bound before rotating to prevent unbounded
        // version growth. The accounting matches `can_rotate` exactly: only
        // non-Active secondary versions count (Active secondaries may still
        // be needed to decrypt data written under them).
        let inactive_count = inactive_secondary_count(key_ring);
        if inactive_count >= self.policy.max_versions {
            return Err(ConfigError::ParseError {
                format: "key".to_string(),
                message: format!(
                    "Cannot rotate: {} inactive versions already exist (max_versions={}). Clean up old keys first.",
                    inactive_count, self.policy.max_versions
                ),
                location: None,
                source: None,
            });
        }

        let old_version = key_ring.current_version;
        let new_key = key_ring.rotate(master_key, rotated_by.clone(), reason.clone())?;

        let history = RotationHistory {
            rotation_id: format!("rot_{}_{}", key_ring.key_id, now_timestamp()),
            key_id: key_ring.key_id.clone(),
            from_version: old_version,
            to_version: new_key.metadata.version(),
            rotated_at: now_timestamp(),
            rotated_by,
            reason,
            reencryption_count: 0,
            status: RotationStatus::Completed,
        };
        self.history.push(history);

        // Cap the in-memory history so long-lived services don't grow it
        // without bound; the oldest entries are dropped first.
        if self.history.len() > MAX_ROTATION_HISTORY {
            self.history
                .drain(..self.history.len() - MAX_ROTATION_HISTORY);
        }

        // 轮换成功（Completed）后经端口发射审计事件；`RotationHistory`
        // 内存账本继续保留，作为链外快速查询。
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

    pub fn check_key_expiration(metadata: &KeyMetadata) -> KeyExpirationStatus {
        if metadata.is_expired() {
            KeyExpirationStatus::Expired
        } else if let Some(expires_at) = metadata.expires_at() {
            let now = now_timestamp();
            let days_until_expiry = (expires_at.saturating_sub(now)) / SECONDS_PER_DAY;

            if days_until_expiry <= CRITICAL_EXPIRY_DAYS {
                KeyExpirationStatus::Critical(days_until_expiry as u32)
            } else if days_until_expiry <= WARNING_EXPIRY_DAYS {
                KeyExpirationStatus::Warning(days_until_expiry as u32)
            } else {
                KeyExpirationStatus::Valid
            }
        } else {
            KeyExpirationStatus::Valid
        }
    }

    /// Check whether a rotation is allowed under the policy.
    ///
    /// The accounting and threshold are identical to `execute_rotation`: only
    /// non-Active secondary versions count, and rotation is rejected once
    /// `max_versions` (or more) retired versions already exist. In other
    /// words, `max_versions` bounds the number of retired secondary versions
    /// retained alongside the single Active primary — a ring holding
    /// `max_versions` non-Active secondaries must clean up before rotating.
    pub fn can_rotate(key_ring: &KeyRing, policy: &KeyRotationPolicy) -> Result<(), ConfigError> {
        // 与 validate_rotation 相同的 OnRotate 拒绝：字段账本未就绪。
        if policy.reencrypt_read == ReencryptPolicy::OnRotate {
            return Err(ConfigError::InvalidValue {
                key: "key.rotation".to_string(),
                expected_type: "never | lazy".to_string(),
                message: "reencrypt_read=on_rotate requires the per-field ledger, \
                          which is not implemented yet"
                    .to_string(),
            });
        }

        let inactive_count = inactive_secondary_count(key_ring);

        if inactive_count >= policy.max_versions {
            return Err(ConfigError::ParseError {
                format: "key".to_string(),
                message: "Too many inactive key versions. Consider cleaning up old keys."
                    .to_string(),
                location: None,
                source: None,
            });
        }

        Ok(())
    }

    pub fn get_rotation_recommendation(
        key_ring: &KeyRing,
        policy: &KeyRotationPolicy,
    ) -> RotationRecommendation {
        let days_since_rotation = key_ring
            .last_rotated_at
            .map(|last| (now_timestamp().saturating_sub(last)) / SECONDS_PER_DAY)
            .unwrap_or(0);

        let version_age_days = (now_timestamp()
            .saturating_sub(key_ring.primary_key.metadata.created_at()))
            / SECONDS_PER_DAY;

        let should_rotate = days_since_rotation >= policy.rotation_interval_days as u64
            || version_age_days >= policy.rotation_interval_days as u64 * 2;

        let priority = if days_since_rotation
            >= policy.rotation_interval_days as u64 + policy.grace_period_days as u64
        {
            RecommendationPriority::Critical
        } else if should_rotate {
            RecommendationPriority::High
        } else if days_since_rotation
            >= policy.rotation_interval_days as u64 - policy.notify_before_expiry_days as u64
        {
            RecommendationPriority::Medium
        } else {
            RecommendationPriority::Low
        };

        RotationRecommendation {
            key_id: key_ring.key_id.clone(),
            current_version: key_ring.current_version,
            days_since_rotation,
            recommended_interval: policy.rotation_interval_days,
            should_rotate,
            priority,
            estimated_downtime_minutes: if should_rotate { Some(5) } else { None },
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum KeyExpirationStatus {
    Valid,
    Warning(u32),
    Critical(u32),
    Expired,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RecommendationPriority {
    Low,
    Medium,
    High,
    Critical,
}

impl std::fmt::Display for RecommendationPriority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RecommendationPriority::Low => write!(f, "low"),
            RecommendationPriority::Medium => write!(f, "medium"),
            RecommendationPriority::High => write!(f, "high"),
            RecommendationPriority::Critical => write!(f, "critical"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RotationRecommendation {
    pub key_id: String,
    pub current_version: u32,
    pub days_since_rotation: u64,
    pub recommended_interval: u32,
    pub should_rotate: bool,
    pub priority: RecommendationPriority,
    pub estimated_downtime_minutes: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::key::{KeyBundle, KeyMetadata, KeyRing, KeyStatus};

    /// Build a KeyRing via struct literal so tests don't require the `encryption` feature.
    fn make_key_ring(
        current_version: u32,
        secondary_versions: Vec<(u32, KeyStatus)>,
        last_rotated_at: Option<u64>,
        primary_created_at: Option<u64>,
    ) -> KeyRing {
        let primary = KeyBundle::new(
            current_version,
            format!("k_{}", current_version),
            "encrypted".to_string(),
            "creator".to_string(),
            None,
        );
        // Override created_at if requested
        if let Some(ts) = primary_created_at {
            let mut primary = primary;
            primary.metadata.created_at = ts;
            return KeyRing {
                key_id: "k".to_string(),
                current_version,
                primary_key: primary,
                secondary_keys: secondary_versions
                    .into_iter()
                    .map(|(v, status)| {
                        let mut b = KeyBundle::new(
                            v,
                            format!("k_{}", v),
                            "encrypted".to_string(),
                            "creator".to_string(),
                            None,
                        );
                        b.metadata.status = status;
                        b
                    })
                    .collect(),
                created_at: 0,
                last_rotated_at,
            };
        }
        KeyRing {
            key_id: "k".to_string(),
            current_version,
            primary_key: primary,
            secondary_keys: secondary_versions
                .into_iter()
                .map(|(v, status)| {
                    let mut b = KeyBundle::new(
                        v,
                        format!("k_{}", v),
                        "encrypted".to_string(),
                        "creator".to_string(),
                        None,
                    );
                    b.metadata.status = status;
                    b
                })
                .collect(),
            created_at: 0,
            last_rotated_at,
        }
    }

    #[test]
    fn test_reencrypt_policy_serde_strings() {
        assert_eq!(
            serde_json::to_string(&ReencryptPolicy::Never).unwrap(),
            "\"never\""
        );
        assert_eq!(
            serde_json::to_string(&ReencryptPolicy::Lazy).unwrap(),
            "\"lazy\""
        );
        assert_eq!(
            serde_json::to_string(&ReencryptPolicy::OnRotate).unwrap(),
            "\"on_rotate\""
        );
    }

    #[test]
    fn test_reencrypt_policy_deserializes_snake_case_and_defaults_to_never() {
        assert_eq!(
            serde_json::from_str::<ReencryptPolicy>("\"never\"").unwrap(),
            ReencryptPolicy::Never
        );
        assert_eq!(
            serde_json::from_str::<ReencryptPolicy>("\"lazy\"").unwrap(),
            ReencryptPolicy::Lazy
        );
        assert_eq!(
            serde_json::from_str::<ReencryptPolicy>("\"on_rotate\"").unwrap(),
            ReencryptPolicy::OnRotate
        );
        // 缺省反序列化得到 Never（serde(default) 字段的取值来源）。
        assert_eq!(ReencryptPolicy::default(), ReencryptPolicy::Never);
    }

    #[test]
    fn test_key_rotation_policy_default() {
        let p = KeyRotationPolicy::default();
        assert_eq!(p.max_versions, 5);
        assert_eq!(p.rotation_interval_days, 90);
        assert_eq!(p.grace_period_days, 14);
        assert!(!p.auto_rotate);
        assert_eq!(p.notify_before_expiry_days, 30);
        assert_eq!(p.dual_key_window_secs, 14 * 86_400);
        assert!(p.reencrypt_on_write);
        assert_eq!(p.reencrypt_read, ReencryptPolicy::Never);
    }

    #[test]
    fn test_key_rotation_policy_new() {
        let p = KeyRotationPolicy::new(10, 30, 7, true);
        assert_eq!(p.max_versions, 10);
        assert_eq!(p.rotation_interval_days, 30);
        assert_eq!(p.grace_period_days, 7);
        assert!(p.auto_rotate);
        assert_eq!(p.notify_before_expiry_days, 30);
        assert_eq!(p.dual_key_window_secs, 7 * 86_400);
        assert!(p.reencrypt_on_write);
        assert_eq!(p.reencrypt_read, ReencryptPolicy::Never);
    }

    #[test]
    fn test_key_rotation_policy_serde_backward_compatible() {
        // 旧配置文件只含原五个字段：反序列化成功且新字段取默认值。
        let legacy = serde_json::json!({
            "max_versions": 5,
            "rotation_interval_days": 90,
            "grace_period_days": 14,
            "auto_rotate": false,
            "notify_before_expiry_days": 30
        });
        let p: KeyRotationPolicy =
            serde_json::from_value(legacy).expect("legacy config must parse");
        assert_eq!(p.dual_key_window_secs, 86_400);
        assert!(p.reencrypt_on_write);
        assert_eq!(p.reencrypt_read, ReencryptPolicy::Never);
    }

    #[test]
    fn test_key_rotation_policy_new_14_day_grace_window() {
        // new(5, 90, 14, false) → dual_key_window_secs == 14 * 86400。
        let p = KeyRotationPolicy::new(5, 90, 14, false);
        assert_eq!(p.dual_key_window_secs, 1_209_600);
    }

    #[test]
    fn test_key_rotation_policy_with_auto_rotate_overrides() {
        let p = KeyRotationPolicy::default().with_auto_rotate(true);
        assert!(p.auto_rotate);
        let p2 = p.with_auto_rotate(false);
        assert!(!p2.auto_rotate);
    }

    #[test]
    fn test_rotation_result_new() {
        let r = RotationResult::new("k1".to_string(), 1, 2, 12345, true);
        assert_eq!(r.key_id, "k1");
        assert_eq!(r.previous_version, 1);
        assert_eq!(r.new_version, 2);
        assert_eq!(r.rotated_at, 12345);
        assert!(r.reencryption_required);
    }

    #[test]
    fn test_rotation_status_variants_and_equality() {
        assert_ne!(RotationStatus::Pending, RotationStatus::InProgress);
        assert_ne!(RotationStatus::Completed, RotationStatus::Failed);
        assert_ne!(RotationStatus::Cancelled, RotationStatus::Pending);
    }

    #[test]
    fn test_rotation_status_display() {
        assert_eq!(RotationStatus::Pending.to_string(), "pending");
        assert_eq!(RotationStatus::InProgress.to_string(), "in_progress");
        assert_eq!(RotationStatus::Completed.to_string(), "completed");
        assert_eq!(RotationStatus::Failed.to_string(), "failed");
        assert_eq!(RotationStatus::Cancelled.to_string(), "cancelled");
    }

    #[test]
    fn test_rotation_status_serialize_deserialize() {
        let s = RotationStatus::InProgress;
        let json = serde_json::to_string(&s).expect("serialize");
        let de: RotationStatus = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(de, s);
    }

    #[test]
    fn test_key_rotation_service_new_initializes_empty() {
        let policy = KeyRotationPolicy::default();
        let _service = KeyRotationService::new(policy);
        // No public accessors for history; just ensure construction works.
    }

    #[test]
    fn test_key_rotation_service_create_rotation_plan_succeeds() {
        let ring = make_key_ring(3, vec![], None, None);
        let plan = KeyRotationService::create_rotation_plan(&ring, 5).expect("create plan");
        assert_eq!(plan.key_id, "k");
        assert_eq!(plan.current_version, 3);
        assert_eq!(plan.target_version, 5);
        assert_eq!(plan.keys_to_rotate, vec![4, 5]);
    }

    #[test]
    fn test_key_rotation_service_create_rotation_plan_target_too_low_errors() {
        let ring = make_key_ring(3, vec![], None, None);
        let err = KeyRotationService::create_rotation_plan(&ring, 3).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("must be greater than current version"),
            "got: {}",
            msg
        );

        let err2 = KeyRotationService::create_rotation_plan(&ring, 2).unwrap_err();
        let msg2 = err2.to_string();
        assert!(
            msg2.contains("must be greater than current version"),
            "got: {}",
            msg2
        );
    }

    #[test]
    fn test_key_rotation_service_validate_rotation_exceeds_max_versions_errors() {
        let ring = make_key_ring(3, vec![], None, None);
        let plan = RotationPlan::new("k".to_string(), 3, 100);
        let policy = KeyRotationPolicy::new(5, 90, 14, false); // max_versions = 5

        let err = KeyRotationService::validate_rotation(&ring, &plan, &policy).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("max versions"), "got: {}", msg);
        assert!(msg.contains("100"), "got: {}", msg);
    }

    #[test]
    fn test_key_rotation_service_validate_rotation_current_version_missing_errors() {
        // plan claims the ring is at version 1, but the ring is at version 3:
        // the plan's current version must exist in the key ring.
        let ring = make_key_ring(3, vec![], None, None);
        let plan = RotationPlan::new("k".to_string(), 1, 3);
        let policy = KeyRotationPolicy::new(10, 90, 14, false);

        let err = KeyRotationService::validate_rotation(&ring, &plan, &policy).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("Plan current version 1 not found"),
            "got: {}",
            msg
        );
    }

    #[test]
    fn test_key_rotation_service_validate_rotation_existing_version_errors() {
        // keys_to_rotate lists FUTURE versions that the rotation will create;
        // a version that already exists in the ring must be rejected.
        let ring = make_key_ring(3, vec![(4, KeyStatus::Active)], None, None);
        let plan = RotationPlan::new("k".to_string(), 3, 5); // keys_to_rotate = [4, 5]
        let policy = KeyRotationPolicy::new(10, 90, 14, false);

        let err = KeyRotationService::validate_rotation(&ring, &plan, &policy).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("already exists"), "got: {}", msg);
        assert!(msg.contains("4"), "got: {}", msg);
    }

    #[test]
    fn test_key_rotation_service_validate_rotation_non_contiguous_errors() {
        let ring = make_key_ring(3, vec![], None, None);
        let mut plan = RotationPlan::new("k".to_string(), 3, 6);
        plan.keys_to_rotate = vec![4, 6]; // gap at 5
        let policy = KeyRotationPolicy::new(10, 90, 14, false);

        let err = KeyRotationService::validate_rotation(&ring, &plan, &policy).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("contiguous"), "got: {}", msg);
    }

    #[test]
    fn test_key_rotation_service_validate_rotation_passes_for_future_versions() {
        // keys_to_rotate lists the versions the rotation WILL create; a fresh
        // ring whose future versions do not exist yet must validate cleanly.
        let ring = make_key_ring(3, vec![], None, None);
        let plan = RotationPlan::new("k".to_string(), 3, 5); // keys_to_rotate = [4, 5]
        let policy = KeyRotationPolicy::new(10, 90, 14, false); // max_versions = 10

        KeyRotationService::validate_rotation(&ring, &plan, &policy)
            .expect("validate_rotation should accept a plan that creates future versions");
    }

    #[test]
    fn test_key_rotation_service_check_key_expiration_no_expiry_returns_valid() {
        let meta = KeyMetadata::new(1, "u".to_string(), None);
        let status = KeyRotationService::check_key_expiration(&meta);
        assert_eq!(status, KeyExpirationStatus::Valid);
    }

    #[test]
    fn test_key_rotation_service_check_key_expiration_expired_returns_expired() {
        let mut meta = KeyMetadata::new(1, "u".to_string(), None);
        meta.set_expires_at(Some(now_timestamp().saturating_sub(1)));
        let status = KeyRotationService::check_key_expiration(&meta);
        assert_eq!(status, KeyExpirationStatus::Expired);
    }

    #[test]
    fn test_key_rotation_service_check_key_expiration_critical_threshold() {
        let mut meta = KeyMetadata::new(1, "u".to_string(), None);
        // 5 days until expiry → Critical (≤ 7 days)
        meta.set_expires_at(Some(now_timestamp() + 5 * SECONDS_PER_DAY));
        let status = KeyRotationService::check_key_expiration(&meta);
        match status {
            KeyExpirationStatus::Critical(days) => {
                assert!(days <= 5, "expected ~5 days, got {}", days);
            }
            other => panic!("expected Critical, got {:?}", other),
        }
    }

    #[test]
    fn test_key_rotation_service_check_key_expiration_warning_threshold() {
        let mut meta = KeyMetadata::new(1, "u".to_string(), None);
        // 20 days until expiry → Warning (≤ 30 but > 7)
        meta.set_expires_at(Some(now_timestamp() + 20 * SECONDS_PER_DAY));
        let status = KeyRotationService::check_key_expiration(&meta);
        match status {
            KeyExpirationStatus::Warning(days) => {
                assert!(days <= 20, "expected ~20 days, got {}", days);
            }
            other => panic!("expected Warning, got {:?}", other),
        }
    }

    #[test]
    fn test_key_rotation_service_check_key_expiration_far_future_returns_valid() {
        let mut meta = KeyMetadata::new(1, "u".to_string(), None);
        // 100 days until expiry → Valid (> 30)
        meta.set_expires_at(Some(now_timestamp() + 100 * SECONDS_PER_DAY));
        let status = KeyRotationService::check_key_expiration(&meta);
        assert_eq!(status, KeyExpirationStatus::Valid);
    }

    #[test]
    fn test_validate_rotation_rejects_on_rotate_policy() {
        let ring = make_key_ring(3, vec![], None, None);
        let plan = RotationPlan::new("k".to_string(), 3, 5);
        let mut policy = KeyRotationPolicy::new(10, 90, 14, false);
        policy.reencrypt_read = ReencryptPolicy::OnRotate;

        let err = KeyRotationService::validate_rotation(&ring, &plan, &policy).unwrap_err();
        match err {
            ConfigError::InvalidValue { key, message, .. } => {
                assert_eq!(key, "key.rotation");
                assert!(message.contains("ledger"), "got: {}", message);
            }
            other => panic!("expected InvalidValue, got: {}", other),
        }
    }

    #[test]
    fn test_validate_rotation_allows_never_and_lazy_policies() {
        let ring = make_key_ring(3, vec![], None, None);
        let plan = RotationPlan::new("k".to_string(), 3, 5);
        let mut policy = KeyRotationPolicy::new(10, 90, 14, false);

        policy.reencrypt_read = ReencryptPolicy::Never;
        KeyRotationService::validate_rotation(&ring, &plan, &policy)
            .expect("Never must behave as before");

        policy.reencrypt_read = ReencryptPolicy::Lazy;
        KeyRotationService::validate_rotation(&ring, &plan, &policy)
            .expect("Lazy must behave as before");
    }

    #[test]
    fn test_can_rotate_rejects_on_rotate_policy() {
        let ring = make_key_ring(3, vec![], None, None);
        let policy = KeyRotationPolicy {
            reencrypt_read: ReencryptPolicy::OnRotate,
            ..KeyRotationPolicy::default()
        };

        let err = KeyRotationService::can_rotate(&ring, &policy).unwrap_err();
        match err {
            ConfigError::InvalidValue { key, .. } => assert_eq!(key, "key.rotation"),
            other => panic!("expected InvalidValue, got: {}", other),
        }
    }

    #[test]
    fn test_can_rotate_allows_never_and_lazy_policies() {
        let ring = make_key_ring(3, vec![(1, KeyStatus::Deprecated)], None, None);

        let never = KeyRotationPolicy {
            reencrypt_read: ReencryptPolicy::Never,
            ..KeyRotationPolicy::default()
        };
        KeyRotationService::can_rotate(&ring, &never).expect("Never must pass");

        let lazy = KeyRotationPolicy {
            reencrypt_read: ReencryptPolicy::Lazy,
            ..KeyRotationPolicy::default()
        };
        KeyRotationService::can_rotate(&ring, &lazy).expect("Lazy must pass");
    }

    #[test]
    fn test_key_rotation_service_can_rotate_passes_with_few_inactive() {
        let ring = make_key_ring(
            3,
            vec![(1, KeyStatus::Deprecated), (2, KeyStatus::Active)],
            None,
            None,
        );
        let policy = KeyRotationPolicy::default(); // max_versions = 5
        // inactive_versions = [v1] (count 1) < 5 → Ok
        KeyRotationService::can_rotate(&ring, &policy).expect("can_rotate should pass");
    }

    #[test]
    fn test_key_rotation_service_can_rotate_passes_at_threshold_minus_one() {
        // Threshold is unified with execute_rotation: 4 inactive < max_versions
        // of 5 → rotation stays allowed (the old `max_versions - 1` threshold
        // incorrectly rejected this state).
        let ring = make_key_ring(
            5,
            vec![
                (1, KeyStatus::Deprecated),
                (2, KeyStatus::Deprecated),
                (3, KeyStatus::Expired),
                (4, KeyStatus::Compromised),
            ],
            None,
            None,
        );
        let policy = KeyRotationPolicy::new(5, 90, 14, false);
        KeyRotationService::can_rotate(&ring, &policy)
            .expect("4 inactive versions < max_versions 5 must be allowed");
    }

    #[test]
    fn test_key_rotation_service_can_rotate_fails_with_too_many_inactive() {
        let ring = make_key_ring(
            6,
            vec![
                (1, KeyStatus::Deprecated),
                (2, KeyStatus::Deprecated),
                (3, KeyStatus::Expired),
                (4, KeyStatus::Compromised),
                (5, KeyStatus::Deprecated),
            ],
            None,
            None,
        );
        let policy = KeyRotationPolicy::new(5, 90, 14, false);
        // inactive = 5 >= max_versions = 5 → error (same accounting as
        // execute_rotation; Active secondaries are not counted).
        let err = KeyRotationService::can_rotate(&ring, &policy).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("inactive key versions"), "got: {}", msg);
    }

    #[test]
    fn test_key_rotation_service_can_rotate_ignores_active_secondaries() {
        // Unified accounting: Active secondary versions are never counted.
        let ring = make_key_ring(
            6,
            vec![
                (1, KeyStatus::Deprecated),
                (2, KeyStatus::Active),
                (3, KeyStatus::Active),
                (4, KeyStatus::Active),
                (5, KeyStatus::Active),
            ],
            None,
            None,
        );
        let policy = KeyRotationPolicy::new(5, 90, 14, false);
        // Only v1 is inactive (1 < 5) despite 5 total secondaries.
        KeyRotationService::can_rotate(&ring, &policy).expect("Active secondaries must not count");
    }

    #[cfg(feature = "encryption")]
    #[test]
    fn test_key_rotation_service_history_is_bounded() {
        let master_key = [0xcd; 32];
        let mut ring = KeyRing::new(&master_key, "k".to_string(), "u".to_string()).unwrap();
        let mut service = KeyRotationService::new(KeyRotationPolicy::default());

        for _ in 0..MAX_ROTATION_HISTORY + 10 {
            service
                .execute_rotation(&mut ring, &master_key, "u".to_string(), None)
                .expect("rotate");
        }

        assert_eq!(
            service.history.len(),
            MAX_ROTATION_HISTORY,
            "history must be capped at MAX_ROTATION_HISTORY"
        );
        // Oldest entries are dropped first: rotations 1..10 were evicted, so
        // the first retained entry is rotation #11 (from version 11).
        assert_eq!(service.history[0].from_version, 11);
    }

    #[test]
    fn test_key_rotation_service_get_rotation_recommendation_low_priority_never_rotated() {
        // last_rotated_at = None → days_since_rotation = 0
        // created_at = now → version_age_days = 0
        let ring = make_key_ring(1, vec![], None, Some(now_timestamp()));
        let policy = KeyRotationPolicy::new(90, 90, 14, false);

        let rec = KeyRotationService::get_rotation_recommendation(&ring, &policy);
        assert_eq!(rec.key_id, "k");
        assert_eq!(rec.current_version, 1);
        assert_eq!(rec.days_since_rotation, 0);
        assert!(!rec.should_rotate);
        assert_eq!(rec.priority, RecommendationPriority::Low);
        assert_eq!(rec.estimated_downtime_minutes, None);
    }

    #[test]
    fn test_key_rotation_service_get_rotation_recommendation_high_priority_overdue() {
        // last_rotated_at = 100 days ago, rotation_interval_days = 90
        // → days_since_rotation = 100 >= 90 → should_rotate = true, High priority
        let past = now_timestamp().saturating_sub(100 * SECONDS_PER_DAY);
        let ring = make_key_ring(1, vec![], Some(past), Some(now_timestamp()));
        let policy = KeyRotationPolicy::new(90, 90, 14, false);

        let rec = KeyRotationService::get_rotation_recommendation(&ring, &policy);
        assert!(rec.should_rotate);
        assert_eq!(rec.priority, RecommendationPriority::High);
        assert_eq!(rec.estimated_downtime_minutes, Some(5));
    }

    #[test]
    fn test_key_rotation_service_get_rotation_recommendation_critical_past_grace() {
        // last_rotated_at = 110 days ago, rotation_interval_days = 90, grace = 14
        // → 110 >= 90 + 14 = 104 → Critical
        let past = now_timestamp().saturating_sub(110 * SECONDS_PER_DAY);
        let ring = make_key_ring(1, vec![], Some(past), Some(now_timestamp()));
        let policy = KeyRotationPolicy::new(5, 90, 14, false);

        let rec = KeyRotationService::get_rotation_recommendation(&ring, &policy);
        assert!(rec.should_rotate);
        assert_eq!(rec.priority, RecommendationPriority::Critical);
    }

    #[test]
    fn test_key_rotation_service_get_rotation_recommendation_medium_in_notify_window() {
        // last_rotated_at = 65 days ago, rotation_interval_days = 90, notify = 30
        // → 65 >= 90 - 30 = 60 AND 65 < 90 → Medium
        let past = now_timestamp().saturating_sub(65 * SECONDS_PER_DAY);
        let ring = make_key_ring(1, vec![], Some(past), Some(now_timestamp()));
        let policy = KeyRotationPolicy::new(5, 90, 14, false);

        let rec = KeyRotationService::get_rotation_recommendation(&ring, &policy);
        assert!(!rec.should_rotate);
        assert_eq!(rec.priority, RecommendationPriority::Medium);
    }

    #[test]
    fn test_key_rotation_service_get_rotation_recommendation_old_version_triggers_rotate() {
        // last_rotated_at = recent (10 days ago), but version_age_days = 200
        // → 200 >= 90 * 2 = 180 → should_rotate = true, High priority
        let recent = now_timestamp().saturating_sub(10 * SECONDS_PER_DAY);
        let old_version = now_timestamp().saturating_sub(200 * SECONDS_PER_DAY);
        let ring = make_key_ring(1, vec![], Some(recent), Some(old_version));
        let policy = KeyRotationPolicy::new(5, 90, 14, false);

        let rec = KeyRotationService::get_rotation_recommendation(&ring, &policy);
        assert!(rec.should_rotate);
        // days_since_rotation = 10, which is < 90, so not Critical; but should_rotate=true → High
        assert_eq!(rec.priority, RecommendationPriority::High);
    }

    /// 内存捕获端口：记录全部到达的审计事件供断言。
    #[cfg(all(test, feature = "audit"))]
    struct CapturingPort(std::sync::Arc<std::sync::RwLock<Vec<crate::impl_::audit::AuditEvent>>>);

    #[cfg(all(test, feature = "audit"))]
    impl crate::key::audit_port::RotationAuditPort for CapturingPort {
        fn record(&self, event: crate::impl_::audit::AuditEvent) {
            self.0.write().expect("capture lock").push(event);
        }
    }

    #[cfg(all(test, feature = "audit", feature = "encryption"))]
    #[test]
    fn test_execute_rotation_emits_exactly_one_event_on_success() {
        let master_key = [0x1b; 32];
        let mut ring = KeyRing::new(&master_key, "k".to_string(), "u".to_string()).unwrap();
        let events = std::sync::Arc::new(std::sync::RwLock::new(Vec::new()));
        let mut service = KeyRotationService::new(KeyRotationPolicy::default())
            .with_audit_port(std::sync::Arc::new(CapturingPort(events.clone())));

        let result = service
            .execute_rotation(&mut ring, &master_key, "u".to_string(), None)
            .expect("execute_rotation");

        let captured = events.read().expect("capture lock").clone();
        assert_eq!(captured.len(), 1, "success must emit exactly one event");
        match &captured[0] {
            crate::impl_::audit::AuditEvent::KeyRotation {
                key_id,
                old_version,
                new_version,
                ..
            } => {
                assert_eq!(key_id, "k");
                assert_eq!(*old_version, format!("v{}", result.previous_version));
                assert_eq!(*new_version, format!("v{}", result.new_version));
            }
            other => panic!("expected KeyRotation event, got {:?}", other),
        }
    }

    #[cfg(all(test, feature = "audit", feature = "encryption"))]
    #[test]
    fn test_execute_rotation_emits_no_event_on_failure() {
        let master_key = [0x1c; 32];
        let mut ring = KeyRing::new(&master_key, "k".to_string(), "u".to_string()).unwrap();
        // 先退役 max_versions 个版本，令后续 execute_rotation 失败。
        let mut policy = KeyRotationPolicy::new(1, 90, 14, false);
        policy.max_versions = 1;
        let events = std::sync::Arc::new(std::sync::RwLock::new(Vec::new()));
        let mut service = KeyRotationService::new(policy)
            .with_audit_port(std::sync::Arc::new(CapturingPort(events.clone())));

        service
            .execute_rotation(&mut ring, &master_key, "u".to_string(), None)
            .expect("first rotation");
        assert_eq!(events.read().expect("capture lock").len(), 1);

        // v1 归档后仍是 Active secondary，需显式退役才计入 inactive 配额。
        ring.deactivate_version(1);
        let err = service
            .execute_rotation(&mut ring, &master_key, "u".to_string(), None)
            .unwrap_err();
        assert!(err.to_string().contains("inactive"), "got: {}", err);
        assert_eq!(
            events.read().expect("capture lock").len(),
            1,
            "failure must not emit additional events"
        );
    }

    #[test]
    fn test_key_expiration_status_variants_equality() {
        assert_eq!(KeyExpirationStatus::Valid, KeyExpirationStatus::Valid);
        assert_eq!(
            KeyExpirationStatus::Warning(5),
            KeyExpirationStatus::Warning(5)
        );
        assert_ne!(
            KeyExpirationStatus::Warning(5),
            KeyExpirationStatus::Warning(6)
        );
        assert_eq!(
            KeyExpirationStatus::Critical(3),
            KeyExpirationStatus::Critical(3)
        );
        assert_ne!(
            KeyExpirationStatus::Critical(3),
            KeyExpirationStatus::Warning(3)
        );
        assert_ne!(KeyExpirationStatus::Valid, KeyExpirationStatus::Expired);
    }

    #[test]
    fn test_recommendation_priority_variants_and_equality() {
        assert_ne!(RecommendationPriority::Low, RecommendationPriority::Medium);
        assert_ne!(
            RecommendationPriority::High,
            RecommendationPriority::Critical
        );
        assert_ne!(
            RecommendationPriority::Low,
            RecommendationPriority::Critical
        );
    }

    #[test]
    fn test_recommendation_priority_display() {
        assert_eq!(RecommendationPriority::Low.to_string(), "low");
        assert_eq!(RecommendationPriority::Medium.to_string(), "medium");
        assert_eq!(RecommendationPriority::High.to_string(), "high");
        assert_eq!(RecommendationPriority::Critical.to_string(), "critical");
    }

    #[test]
    fn test_rotation_recommendation_struct_construction() {
        let rec = RotationRecommendation {
            key_id: "k1".to_string(),
            current_version: 3,
            days_since_rotation: 100,
            recommended_interval: 90,
            should_rotate: true,
            priority: RecommendationPriority::High,
            estimated_downtime_minutes: Some(5),
        };
        assert_eq!(rec.key_id, "k1");
        assert_eq!(rec.current_version, 3);
        assert_eq!(rec.days_since_rotation, 100);
        assert_eq!(rec.recommended_interval, 90);
        assert!(rec.should_rotate);
        assert_eq!(rec.priority, RecommendationPriority::High);
        assert_eq!(rec.estimated_downtime_minutes, Some(5));
    }

    #[test]
    fn test_rotation_history_struct_construction() {
        let h = RotationHistory {
            rotation_id: "rot_1".to_string(),
            key_id: "k".to_string(),
            from_version: 1,
            to_version: 2,
            rotated_at: 1234,
            rotated_by: "team".to_string(),
            reason: Some("scheduled".to_string()),
            reencryption_count: 5,
            status: RotationStatus::Completed,
        };
        assert_eq!(h.rotation_id, "rot_1");
        assert_eq!(h.from_version, 1);
        assert_eq!(h.to_version, 2);
        assert_eq!(h.reencryption_count, 5);
        assert_eq!(h.status, RotationStatus::Completed);
    }

    #[test]
    fn test_rotation_history_serialize_deserialize() {
        let h = RotationHistory {
            rotation_id: "rot_1".to_string(),
            key_id: "k".to_string(),
            from_version: 1,
            to_version: 2,
            rotated_at: 0,
            rotated_by: "u".to_string(),
            reason: None,
            reencryption_count: 0,
            status: RotationStatus::Pending,
        };
        let json = serde_json::to_string(&h).expect("serialize");
        let de: RotationHistory = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(de.rotation_id, h.rotation_id);
        assert_eq!(de.status, h.status);
    }

    #[cfg(feature = "encryption")]
    #[test]
    fn test_key_rotation_service_execute_rotation_advances_version() {
        use crate::key::KeyRing;

        let master_key = [0xab; 32];
        let mut ring = KeyRing::new(&master_key, "k".to_string(), "u".to_string()).unwrap();
        let original_version = ring.current_version;

        let mut service = KeyRotationService::new(KeyRotationPolicy::default());
        let result = service
            .execute_rotation(
                &mut ring,
                &master_key,
                "rotator".to_string(),
                Some("scheduled".to_string()),
            )
            .expect("execute_rotation");

        assert_eq!(result.previous_version, original_version);
        assert_eq!(result.new_version, original_version + 1);
        assert!(result.reencryption_required);
        assert_eq!(ring.current_version, original_version + 1);
        assert_eq!(ring.secondary_keys.len(), 1);
    }
}
