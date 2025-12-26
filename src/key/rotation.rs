// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

use crate::error::ConfigError;
use crate::key::{KeyMetadata, KeyRing, KeyStatus, RotationPlan};
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

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

#[allow(dead_code)]
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
#[allow(dead_code)]
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
            return Err(ConfigError::FormatDetectionFailed(
                "Target version must be greater than current version".to_string(),
            ));
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
            return Err(ConfigError::FormatDetectionFailed(format!(
                "Rotation would exceed max versions ({}). Current: {}, Target: {}",
                policy.max_versions, key_ring.current_version, plan.target_version
            )));
        }

        for version in &plan.keys_to_rotate {
            if key_ring.get_key_by_version(*version).is_none() {
                return Err(ConfigError::FormatDetectionFailed(format!(
                    "Key version {} not found",
                    version
                )));
            }
        }

        Ok(())
    }

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
            let days_until_expiry = (expires_at.saturating_sub(now)) / 86400;

            if days_until_expiry <= 7 {
                KeyExpirationStatus::Critical(days_until_expiry as u32)
            } else if days_until_expiry <= 30 {
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
            return Err(ConfigError::FormatDetectionFailed(
                "Too many inactive key versions. Consider cleaning up old keys.".to_string(),
            ));
        }

        Ok(())
    }

    pub fn get_rotation_recommendation(
        key_ring: &KeyRing,
        policy: &KeyRotationPolicy,
    ) -> RotationRecommendation {
        let days_since_rotation = key_ring
            .last_rotated_at
            .map(|last| (now_timestamp().saturating_sub(last)) / 86400)
            .unwrap_or(0);

        let version_age_days =
            (now_timestamp().saturating_sub(key_ring.primary_key.metadata.created_at)) / 86400;

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

fn now_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_secs()
}
