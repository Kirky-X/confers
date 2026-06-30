// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

use crate::error::ConfigError;
use crate::key::{now_timestamp, KeyMetadata, KeyRing, KeyStatus, RotationPlan, SECONDS_PER_DAY};
use serde::{Deserialize, Serialize};

const CRITICAL_EXPIRY_DAYS: u64 = 7;
const WARNING_EXPIRY_DAYS: u64 = 30;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyRotationPolicy {
    pub max_versions: u32,
    pub rotation_interval_days: u32,
    pub grace_period_days: u32,
    pub auto_rotate: bool,
    pub notify_before_expiry_days: u32,
}

impl Default for KeyRotationPolicy {
    fn default() -> Self {
        Self {
            max_versions: 5,
            rotation_interval_days: 90,
            grace_period_days: 14,
            auto_rotate: false,
            notify_before_expiry_days: 30,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RotationTask {
    pub task_id: String,
    pub key_id: String,
    pub plan: RotationPlan,
    pub status: RotationStatus,
    pub created_at: u64,
    pub started_at: Option<u64>,
    pub completed_at: Option<u64>,
    pub progress: u32,
    pub total_items: u32,
    pub errors: Vec<String>,
}

#[allow(dead_code)] // used via KeyRotationService in integrated scenarios
impl RotationTask {
    pub fn new(key_id: String, plan: RotationPlan) -> Self {
        let plan_cloned = plan.clone();
        Self {
            task_id: format!("task_{}_{}", key_id, now_timestamp()),
            key_id,
            plan: plan_cloned,
            status: RotationStatus::Pending,
            created_at: now_timestamp(),
            started_at: None,
            completed_at: None,
            progress: 0,
            total_items: plan.keys_to_rotate.len() as u32,
            errors: Vec::new(),
        }
    }

    pub fn start(&mut self) {
        self.status = RotationStatus::InProgress;
        self.started_at = Some(now_timestamp());
    }

    pub fn complete(&mut self) {
        self.status = RotationStatus::Completed;
        self.completed_at = Some(now_timestamp());
        self.progress = self.total_items;
    }

    pub fn fail(&mut self, error: String) {
        self.status = RotationStatus::Failed;
        self.completed_at = Some(now_timestamp());
        self.errors.push(error);
    }

    pub fn add_error(&mut self, error: String) {
        self.errors.push(error);
    }

    pub fn is_complete(&self) -> bool {
        self.status == RotationStatus::Completed
            || self.status == RotationStatus::Failed
            || self.status == RotationStatus::Cancelled
    }

    pub fn progress_percent(&self) -> f64 {
        if self.total_items == 0 {
            100.0
        } else {
            (self.progress as f64 / self.total_items as f64) * 100.0
        }
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)] // used via KeyRotationPolicy in integrated scenarios
pub struct KeyRotationService {
    policy: KeyRotationPolicy,
    history: Vec<RotationHistory>,
    active_tasks: Vec<RotationTask>,
}

impl KeyRotationService {
    pub fn new(policy: KeyRotationPolicy) -> Self {
        Self {
            policy,
            history: Vec::new(),
            active_tasks: Vec::new(),
        }
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

    pub fn validate_rotation(
        key_ring: &KeyRing,
        plan: &RotationPlan,
        policy: &KeyRotationPolicy,
    ) -> Result<(), ConfigError> {
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

        for version in &plan.keys_to_rotate {
            if key_ring.get_key_by_version(*version).is_none() {
                return Err(ConfigError::ParseError {
                    format: "key".to_string(),
                    message: format!("Key version {} not found", version),
                    location: None,
                    source: None,
                });
            }
        }

        Ok(())
    }

    #[cfg(feature = "encryption")]
    pub fn execute_rotation(
        key_ring: &mut KeyRing,
        master_key: &[u8; 32],
        rotated_by: String,
        reason: Option<String>,
    ) -> Result<RotationResult, ConfigError> {
        let old_version = key_ring.current_version;
        let new_key = key_ring.rotate(master_key, rotated_by.clone(), reason.clone())?;

        let _history = RotationHistory {
            rotation_id: format!("rot_{}_{}", key_ring.key_id, now_timestamp()),
            key_id: key_ring.key_id.clone(),
            from_version: old_version,
            to_version: new_key.metadata.version,
            rotated_at: now_timestamp(),
            rotated_by,
            reason,
            reencryption_count: 0,
            status: RotationStatus::Completed,
        };

        Ok(RotationResult {
            key_id: key_ring.key_id.clone(),
            previous_version: old_version,
            new_version: new_key.metadata.version,
            rotated_at: now_timestamp(),
            reencryption_required: true,
        })
    }

    pub fn check_key_expiration(metadata: &KeyMetadata) -> KeyExpirationStatus {
        if metadata.is_expired() {
            KeyExpirationStatus::Expired
        } else if let Some(expires_at) = metadata.expires_at {
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

    pub fn can_rotate(key_ring: &KeyRing, policy: &KeyRotationPolicy) -> Result<(), ConfigError> {
        let inactive_versions: Vec<u32> = key_ring
            .secondary_keys
            .iter()
            .filter(|k| k.metadata.status != KeyStatus::Active)
            .map(|k| k.metadata.version)
            .collect();

        if inactive_versions.len() as u32 >= policy.max_versions.saturating_sub(1) {
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
            .saturating_sub(key_ring.primary_key.metadata.created_at))
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
    fn test_key_rotation_policy_default() {
        let p = KeyRotationPolicy::default();
        assert_eq!(p.max_versions, 5);
        assert_eq!(p.rotation_interval_days, 90);
        assert_eq!(p.grace_period_days, 14);
        assert!(!p.auto_rotate);
        assert_eq!(p.notify_before_expiry_days, 30);
    }

    #[test]
    fn test_key_rotation_policy_new() {
        let p = KeyRotationPolicy::new(10, 30, 7, true);
        assert_eq!(p.max_versions, 10);
        assert_eq!(p.rotation_interval_days, 30);
        assert_eq!(p.grace_period_days, 7);
        assert!(p.auto_rotate);
        assert_eq!(p.notify_before_expiry_days, 30);
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
    fn test_rotation_task_new_initial_state() {
        let plan = RotationPlan::new("k1".to_string(), 1, 3);
        let task = RotationTask::new("k1".to_string(), plan);
        assert_eq!(task.key_id, "k1");
        assert_eq!(task.status, RotationStatus::Pending);
        assert_eq!(task.progress, 0);
        assert_eq!(task.total_items, 2); // versions 2, 3
        assert!(task.errors.is_empty());
        assert!(task.started_at.is_none());
        assert!(task.completed_at.is_none());
        assert!(!task.is_complete());
        assert!(task.task_id.starts_with("task_k1_"));
    }

    #[test]
    fn test_rotation_task_start_sets_in_progress() {
        let plan = RotationPlan::new("k1".to_string(), 1, 2);
        let mut task = RotationTask::new("k1".to_string(), plan);
        task.start();
        assert_eq!(task.status, RotationStatus::InProgress);
        assert!(task.started_at.is_some());
        assert!(task.completed_at.is_none());
        // Started but not complete
        assert!(!task.is_complete());
    }

    #[test]
    fn test_rotation_task_complete_sets_completed_and_progress() {
        let plan = RotationPlan::new("k1".to_string(), 1, 3);
        let mut task = RotationTask::new("k1".to_string(), plan);
        task.start();
        task.complete();
        assert_eq!(task.status, RotationStatus::Completed);
        assert!(task.completed_at.is_some());
        assert_eq!(task.progress, task.total_items);
        assert!(task.is_complete());
    }

    #[test]
    fn test_rotation_task_fail_sets_failed_and_records_error() {
        let plan = RotationPlan::new("k1".to_string(), 1, 2);
        let mut task = RotationTask::new("k1".to_string(), plan);
        task.start();
        task.fail("encryption error".to_string());
        assert_eq!(task.status, RotationStatus::Failed);
        assert!(task.completed_at.is_some());
        assert_eq!(task.errors.len(), 1);
        assert_eq!(task.errors[0], "encryption error");
        assert!(task.is_complete());
    }

    #[test]
    fn test_rotation_task_add_error_accumulates() {
        let plan = RotationPlan::new("k1".to_string(), 1, 2);
        let mut task = RotationTask::new("k1".to_string(), plan);
        assert!(task.errors.is_empty());
        task.add_error("err1".to_string());
        task.add_error("err2".to_string());
        assert_eq!(task.errors, vec!["err1".to_string(), "err2".to_string()]);
    }

    #[test]
    fn test_rotation_task_is_complete_returns_true_for_terminal_states() {
        let plan = RotationPlan::new("k".to_string(), 1, 2);
        let mut task = RotationTask::new("k".to_string(), plan);

        task.status = RotationStatus::Completed;
        assert!(task.is_complete());

        task.status = RotationStatus::Failed;
        assert!(task.is_complete());

        task.status = RotationStatus::Cancelled;
        assert!(task.is_complete());

        task.status = RotationStatus::Pending;
        assert!(!task.is_complete());

        task.status = RotationStatus::InProgress;
        assert!(!task.is_complete());
    }

    #[test]
    fn test_rotation_task_progress_percent_zero_items_returns_100() {
        // Same version → empty keys_to_rotate → total_items = 0
        let plan = RotationPlan::new("k".to_string(), 5, 5);
        let task = RotationTask::new("k".to_string(), plan);
        assert_eq!(task.total_items, 0);
        assert_eq!(task.progress_percent(), 100.0);
    }

    #[test]
    fn test_rotation_task_progress_percent_partial() {
        let plan = RotationPlan::new("k".to_string(), 1, 5); // 4 items
        let mut task = RotationTask::new("k".to_string(), plan);
        task.progress = 2;
        assert_eq!(task.total_items, 4);
        assert_eq!(task.progress_percent(), 50.0);
    }

    #[test]
    fn test_key_rotation_service_new_initializes_empty() {
        let policy = KeyRotationPolicy::default();
        let _service = KeyRotationService::new(policy);
        // No public accessors for history/active_tasks; just ensure construction works.
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
    fn test_key_rotation_service_validate_rotation_missing_version_errors() {
        // plan.keys_to_rotate = [4, 5], but ring only has version 3 (primary).
        let ring = make_key_ring(3, vec![], None, None);
        let plan = RotationPlan::new("k".to_string(), 3, 5);
        let policy = KeyRotationPolicy::new(10, 90, 14, false);

        let err = KeyRotationService::validate_rotation(&ring, &plan, &policy).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("Key version 4 not found"), "got: {}", msg);
    }

    #[test]
    fn test_key_rotation_service_validate_rotation_passes_when_all_versions_exist() {
        // validate_rotation requires that every version in plan.keys_to_rotate
        // already exists in the key_ring (i.e., pre-staged as secondaries).
        let ring = make_key_ring(
            3,
            vec![(4, KeyStatus::Active), (5, KeyStatus::Active)],
            None,
            None,
        );
        let plan = RotationPlan::new("k".to_string(), 3, 5); // keys_to_rotate = [4, 5]
        let policy = KeyRotationPolicy::new(10, 90, 14, false); // max_versions = 10

        KeyRotationService::validate_rotation(&ring, &plan, &policy)
            .expect("validate_rotation should pass when all target versions exist");
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
        meta.expires_at = Some(now_timestamp().saturating_sub(1));
        let status = KeyRotationService::check_key_expiration(&meta);
        assert_eq!(status, KeyExpirationStatus::Expired);
    }

    #[test]
    fn test_key_rotation_service_check_key_expiration_critical_threshold() {
        let mut meta = KeyMetadata::new(1, "u".to_string(), None);
        // 5 days until expiry → Critical (≤ 7 days)
        meta.expires_at = Some(now_timestamp() + 5 * SECONDS_PER_DAY);
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
        meta.expires_at = Some(now_timestamp() + 20 * SECONDS_PER_DAY);
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
        meta.expires_at = Some(now_timestamp() + 100 * SECONDS_PER_DAY);
        let status = KeyRotationService::check_key_expiration(&meta);
        assert_eq!(status, KeyExpirationStatus::Valid);
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
                                                   // inactive_versions = [v1] (len 1) < 5-1=4 → Ok
        KeyRotationService::can_rotate(&ring, &policy).expect("can_rotate should pass");
    }

    #[test]
    fn test_key_rotation_service_can_rotate_fails_with_too_many_inactive() {
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
        let policy = KeyRotationPolicy::new(5, 90, 14, false); // max_versions = 5
                                                               // inactive = 4, threshold = max_versions - 1 = 4 → 4 >= 4 → error
        let err = KeyRotationService::can_rotate(&ring, &policy).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("inactive key versions"), "got: {}", msg);
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

        let result = KeyRotationService::execute_rotation(
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
