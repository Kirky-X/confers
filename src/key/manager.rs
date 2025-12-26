use crate::error::ConfigError;
use crate::key::{
    KeyBundle, KeyRing, KeyRotationSchedule, KeyStatus, RotationPlan, RotationResult,
    CURRENT_KEY_VERSION,
};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

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
    pub fn new(storage_path: PathBuf) -> Result<Self, ConfigError> {
        Ok(Self {
            master_key_hash: String::new(),
            key_rings: HashMap::new(),
            schedules: HashMap::new(),
            default_key_id: "default".to_string(),
            storage_path,
        })
    }

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
            id: format!("{}_{}", key_id, crate::key::CONFERS_KEY_VERSION),
            version: CURRENT_KEY_VERSION,
            created_at: now_timestamp(),
            status: KeyStatus::Active,
            algorithm: "AES256-GCM".to_string(),
        })
    }

    pub fn generate_key(&mut self, _master_key: &[u8; 32]) -> Result<[u8; 32], ConfigError> {
        let mut key_bytes = [0u8; 32];
        let mut rng = rand::rng();
        rng.fill(&mut key_bytes);
        Ok(key_bytes)
    }

    pub fn create_key_ring(
        &mut self,
        master_key: &[u8; 32],
        key_id: String,
        created_by: String,
        description: Option<String>,
    ) -> Result<KeyVersion, ConfigError> {
        if self.key_rings.contains_key(&key_id) {
            return Err(ConfigError::FormatDetectionFailed(format!(
                "Key ring '{}' already exists",
                key_id
            )));
        }

        let key_ring = KeyRing::new(master_key, key_id.clone(), created_by)?;

        if let Some(desc) = description {
            if let Some(key) = self.key_rings.get_mut(&key_id) {
                key.primary_key.metadata.description = Some(desc);
            }
        }

        self.key_rings.insert(key_id.clone(), key_ring);

        let schedule = KeyRotationSchedule::new(key_id.clone(), 90, now_timestamp(), 5);
        self.schedules.insert(key_id.clone(), schedule);

        Ok(KeyVersion {
            id: format!("{}_{}", key_id, crate::key::CONFERS_KEY_VERSION),
            version: CURRENT_KEY_VERSION,
            created_at: now_timestamp(),
            status: KeyStatus::Active,
            algorithm: "AES256-GCM".to_string(),
        })
    }

    pub fn rotate_key(
        &mut self,
        master_key: &[u8; 32],
        key_id: Option<String>,
        created_by: String,
        description: Option<String>,
    ) -> Result<RotationResult, ConfigError> {
        let key_id = key_id.unwrap_or_else(|| self.default_key_id.clone());

        let key_ring = self.key_rings.get_mut(&key_id).ok_or_else(|| {
            ConfigError::FormatDetectionFailed(format!("Key ring '{}' not found", key_id))
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
        let key_ring = self.key_rings.get(key_id).ok_or_else(|| {
            ConfigError::FormatDetectionFailed(format!("Key ring '{}' not found", key_id))
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
        let schedule = self.schedules.get_mut(key_id).ok_or_else(|| {
            ConfigError::FormatDetectionFailed(format!("Key ring '{}' not found", key_id))
        })?;

        schedule.rotation_interval_days = interval_days;
        schedule.next_rotation = schedule
            .last_rotation
            .saturating_add(interval_days as u64 * 86400);

        Ok(())
    }

    pub fn plan_rotation(
        &self,
        target_version: u32,
        key_id: Option<String>,
    ) -> Result<RotationPlan, ConfigError> {
        let key_id = key_id.unwrap_or_else(|| self.default_key_id.clone());

        let key_ring = self.key_rings.get(&key_id).ok_or_else(|| {
            ConfigError::FormatDetectionFailed(format!("Key ring '{}' not found", key_id))
        })?;

        if target_version <= key_ring.current_version {
            return Err(ConfigError::FormatDetectionFailed(
                "Target version must be greater than current version".to_string(),
            ));
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
        let key_ring = self.key_rings.get(key_id).ok_or_else(|| {
            ConfigError::FormatDetectionFailed(format!("Key ring '{}' not found", key_id))
        })?;

        Ok(key_ring.get_key_by_version(version))
    }

    pub fn deprecate_version(&mut self, key_id: &str, version: u32) -> Result<(), ConfigError> {
        let key_ring = self.key_rings.get_mut(key_id).ok_or_else(|| {
            ConfigError::FormatDetectionFailed(format!("Key ring '{}' not found", key_id))
        })?;

        if version == key_ring.current_version {
            return Err(ConfigError::FormatDetectionFailed(
                "Cannot deprecate the current active version".to_string(),
            ));
        }

        key_ring.deactivate_version(version);
        Ok(())
    }

    pub fn cleanup_old_keys(
        &mut self,
        key_id: &str,
        keep_versions: u32,
    ) -> Result<u32, ConfigError> {
        let key_ring = self.key_rings.get_mut(key_id).ok_or_else(|| {
            ConfigError::FormatDetectionFailed(format!("Key ring '{}' not found", key_id))
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
            return Err(ConfigError::FormatDetectionFailed(format!(
                "Key ring '{}' not found",
                key_id
            )));
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

fn now_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_secs()
}
