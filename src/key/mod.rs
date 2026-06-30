// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

mod manager;
mod rotation;
#[cfg(feature = "encryption")]
mod storage;
mod version;

pub use manager::{KeyInfo, KeyManager, KeyVersion};
pub use rotation::{KeyRotationPolicy, KeyRotationService, RotationResult};
#[cfg(feature = "encryption")]
pub use storage::{ErrorSanitizer, KeyStorage, SanitizationLevel};
pub use version::KeyFormatVersion;

use crate::error::ConfigError;
#[cfg(feature = "encryption")]
use crate::secret::XChaCha20Crypto;
#[cfg(feature = "encryption")]
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[cfg(feature = "encryption")]
use rand::Rng;

pub const CONFERS_KEY_VERSION: &str = "v1";
pub const KEY_VERSION_PREFIX: &str = "v";
pub const CURRENT_KEY_VERSION: u32 = 1;
pub(crate) const SECONDS_PER_DAY: u64 = 86_400;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum KeyStatus {
    Active,
    Deprecated,
    Compromised,
    Expired,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyMetadata {
    pub version: u32,
    pub created_at: u64,
    pub created_by: String,
    pub status: KeyStatus,
    pub expires_at: Option<u64>,
    pub description: Option<String>,
}

impl KeyMetadata {
    pub fn new(version: u32, created_by: String, description: Option<String>) -> Self {
        Self {
            version,
            created_at: now_timestamp(),
            created_by,
            status: KeyStatus::Active,
            expires_at: None,
            description,
        }
    }

    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            now_timestamp() > expires_at
        } else {
            false
        }
    }

    pub fn is_active(&self) -> bool {
        self.status == KeyStatus::Active && !self.is_expired()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyBundle {
    pub metadata: KeyMetadata,
    pub key_id: String,
    pub encrypted_key: String,
}

impl KeyBundle {
    pub fn new(
        version: u32,
        key_id: String,
        encrypted_key: String,
        created_by: String,
        description: Option<String>,
    ) -> Self {
        Self {
            metadata: KeyMetadata::new(version, created_by, description),
            key_id,
            encrypted_key,
        }
    }

    #[cfg(feature = "encryption")]
    pub fn generate(
        master_key: &[u8; 32],
        version: u32,
        created_by: String,
        description: Option<String>,
    ) -> Result<Self, ConfigError> {
        let mut key_bytes = [0u8; 32];
        let mut rng = rand::thread_rng();
        rng.fill(&mut key_bytes);

        let encryptor = XChaCha20Crypto::new();
        let (nonce, ciphertext) = encryptor
            .encrypt(BASE64.encode(key_bytes).as_bytes(), master_key)
            .map_err(|e| ConfigError::ParseError {
                format: "key".to_string(),
                message: format!("Encryption failed: {}", e),
                location: None,
                source: None,
            })?;

        // 格式: nonce_base64:ciphertext_base64
        let encrypted_key = format!("{}:{}", BASE64.encode(&nonce), BASE64.encode(&ciphertext));

        let key_id = format!("{}_{}", KEY_VERSION_PREFIX, version);

        Ok(Self::new(
            version,
            key_id,
            encrypted_key,
            created_by,
            description,
        ))
    }

    #[cfg(feature = "encryption")]
    pub fn get_plaintext_key(&self, master_key: &[u8; 32]) -> Result<[u8; 32], ConfigError> {
        let parts: Vec<&str> = self.encrypted_key.split(':').collect();
        if parts.len() != 2 {
            return Err(ConfigError::ParseError {
                format: "key".to_string(),
                message: "Invalid encrypted key format".to_string(),
                location: None,
                source: None,
            });
        }

        let nonce = BASE64
            .decode(parts[0])
            .map_err(|e| ConfigError::ParseError {
                format: "key".to_string(),
                message: format!("Failed to decode nonce: {}", e),
                location: None,
                source: None,
            })?;
        let ciphertext = BASE64
            .decode(parts[1])
            .map_err(|e| ConfigError::ParseError {
                format: "key".to_string(),
                message: format!("Failed to decode ciphertext: {}", e),
                location: None,
                source: None,
            })?;

        let encryptor = XChaCha20Crypto::new();
        let plaintext = encryptor
            .decrypt(&nonce, &ciphertext, master_key)
            .map_err(|e| ConfigError::ParseError {
                format: "key".to_string(),
                message: format!("Decryption failed: {}", e),
                location: None,
                source: None,
            })?;

        let key_bytes = BASE64
            .decode(
                String::from_utf8(plaintext).map_err(|e| ConfigError::ParseError {
                    format: "key".to_string(),
                    message: format!("Invalid key bytes: {}", e),
                    location: None,
                    source: None,
                })?,
            )
            .map_err(|e| ConfigError::ParseError {
                format: "key".to_string(),
                message: format!("Invalid key bytes: {}", e),
                location: None,
                source: None,
            })?;

        if key_bytes.len() != 32 {
            return Err(ConfigError::ParseError {
                format: "key".to_string(),
                message: "Invalid key length".to_string(),
                location: None,
                source: None,
            });
        }

        let mut result = [0u8; 32];
        result.copy_from_slice(&key_bytes);
        Ok(result)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyRing {
    pub key_id: String,
    pub current_version: u32,
    pub primary_key: KeyBundle,
    pub secondary_keys: Vec<KeyBundle>,
    pub created_at: u64,
    pub last_rotated_at: Option<u64>,
}

impl KeyRing {
    #[cfg(feature = "encryption")]
    pub fn new(
        master_key: &[u8; 32],
        key_id: String,
        created_by: String,
    ) -> Result<Self, ConfigError> {
        let primary_key = KeyBundle::generate(master_key, CURRENT_KEY_VERSION, created_by, None)?;

        Ok(Self {
            key_id,
            current_version: CURRENT_KEY_VERSION,
            primary_key,
            secondary_keys: Vec::new(),
            created_at: now_timestamp(),
            last_rotated_at: None,
        })
    }

    #[cfg(feature = "encryption")]
    pub fn rotate(
        &mut self,
        master_key: &[u8; 32],
        created_by: String,
        description: Option<String>,
    ) -> Result<KeyBundle, ConfigError> {
        let new_version = self.current_version + 1;
        let new_key = KeyBundle::generate(master_key, new_version, created_by, description)?;

        self.secondary_keys.push(self.primary_key.clone());

        self.primary_key = new_key.clone();
        self.current_version = new_version;
        self.last_rotated_at = Some(now_timestamp());

        Ok(new_key)
    }

    pub fn get_key_by_version(&self, version: u32) -> Option<&KeyBundle> {
        if self.primary_key.metadata.version == version {
            Some(&self.primary_key)
        } else {
            self.secondary_keys
                .iter()
                .find(|k| k.metadata.version == version)
        }
    }

    pub fn add_secondary_key(&mut self, key: KeyBundle) {
        self.secondary_keys.push(key);
    }

    pub fn deactivate_version(&mut self, version: u32) {
        if let Some(key) = self.get_key_by_version_mut(version) {
            key.metadata.status = KeyStatus::Deprecated;
        }
    }

    fn get_key_by_version_mut(&mut self, version: u32) -> Option<&mut KeyBundle> {
        if self.primary_key.metadata.version == version {
            Some(&mut self.primary_key)
        } else {
            self.secondary_keys
                .iter_mut()
                .find(|k| k.metadata.version == version)
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyRotationSchedule {
    pub key_id: String,
    pub rotation_interval_days: u32,
    pub last_rotation: u64,
    pub next_rotation: u64,
    pub max_versions: u32,
    pub auto_rotate: bool,
}

impl KeyRotationSchedule {
    pub fn new(
        key_id: String,
        rotation_interval_days: u32,
        last_rotation: u64,
        max_versions: u32,
    ) -> Self {
        let next_rotation =
            last_rotation.saturating_add(rotation_interval_days as u64 * SECONDS_PER_DAY);

        Self {
            key_id,
            rotation_interval_days,
            last_rotation,
            next_rotation,
            max_versions,
            auto_rotate: true,
        }
    }

    pub fn is_rotation_due(&self) -> bool {
        now_timestamp() >= self.next_rotation
    }

    pub fn update_after_rotation(&mut self) {
        self.last_rotation = now_timestamp();
        self.next_rotation = self
            .last_rotation
            .saturating_add(self.rotation_interval_days as u64 * SECONDS_PER_DAY);
    }

    pub fn days_until_rotation(&self) -> i64 {
        let now = now_timestamp() as i64;
        let next = self.next_rotation as i64;
        (next - now) / SECONDS_PER_DAY as i64
    }
}

pub(crate) fn now_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_secs()
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RotationPlan {
    pub key_id: String,
    pub current_version: u32,
    pub target_version: u32,
    pub keys_to_rotate: Vec<u32>,
    pub reencryption_required: bool,
}

impl RotationPlan {
    pub fn new(key_id: String, current_version: u32, target_version: u32) -> Self {
        let keys_to_rotate: Vec<u32> = (current_version + 1..=target_version).collect();
        let reencryption_required = !keys_to_rotate.is_empty();

        Self {
            key_id,
            current_version,
            target_version,
            keys_to_rotate,
            reencryption_required,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_status_variants() {
        let active = KeyStatus::Active;
        let deprecated = KeyStatus::Deprecated;
        let compromised = KeyStatus::Compromised;
        let expired = KeyStatus::Expired;

        assert_ne!(active, deprecated);
        assert_ne!(active, compromised);
        assert_ne!(active, expired);
        assert_ne!(deprecated, expired);
    }

    #[test]
    fn test_key_metadata_new_defaults() {
        let meta = KeyMetadata::new(3, "alice".to_string(), Some("desc".to_string()));
        assert_eq!(meta.version, 3);
        assert_eq!(meta.created_by, "alice");
        assert_eq!(meta.status, KeyStatus::Active);
        assert_eq!(meta.expires_at, None);
        assert_eq!(meta.description.as_deref(), Some("desc"));
        assert!(meta.created_at > 0);
    }

    #[test]
    fn test_key_metadata_new_without_description() {
        let meta = KeyMetadata::new(1, "bob".to_string(), None);
        assert_eq!(meta.description, None);
    }

    #[test]
    fn test_key_metadata_is_expired_no_expiry() {
        let meta = KeyMetadata::new(1, "u".to_string(), None);
        assert!(!meta.is_expired());
    }

    #[test]
    fn test_key_metadata_is_expired_past() {
        let mut meta = KeyMetadata::new(1, "u".to_string(), None);
        meta.expires_at = Some(now_timestamp().saturating_sub(1));
        assert!(meta.is_expired());
    }

    #[test]
    fn test_key_metadata_is_expired_future() {
        let mut meta = KeyMetadata::new(1, "u".to_string(), None);
        meta.expires_at = Some(now_timestamp().saturating_add(86_400));
        assert!(!meta.is_expired());
    }

    #[test]
    fn test_key_metadata_is_active_when_active_no_expiry() {
        let meta = KeyMetadata::new(1, "u".to_string(), None);
        assert!(meta.is_active());
    }

    #[test]
    fn test_key_metadata_is_active_when_deprecated() {
        let mut meta = KeyMetadata::new(1, "u".to_string(), None);
        meta.status = KeyStatus::Deprecated;
        assert!(!meta.is_active());
    }

    #[test]
    fn test_key_metadata_is_active_when_expired() {
        let mut meta = KeyMetadata::new(1, "u".to_string(), None);
        meta.expires_at = Some(now_timestamp().saturating_sub(1));
        assert!(!meta.is_active());
    }

    #[test]
    fn test_key_bundle_new_sets_metadata() {
        let bundle = KeyBundle::new(
            2,
            "k_2".to_string(),
            "encrypted".to_string(),
            "creator".to_string(),
            Some("desc".to_string()),
        );
        assert_eq!(bundle.metadata.version, 2);
        assert_eq!(bundle.key_id, "k_2");
        assert_eq!(bundle.encrypted_key, "encrypted");
        assert_eq!(bundle.metadata.created_by, "creator");
        assert_eq!(bundle.metadata.status, KeyStatus::Active);
    }

    #[test]
    fn test_key_rotation_schedule_new_computes_next_rotation() {
        let last_rotation = 1_000_000_u64;
        let schedule = KeyRotationSchedule::new("k1".to_string(), 90, last_rotation, 5);
        assert_eq!(schedule.key_id, "k1");
        assert_eq!(schedule.rotation_interval_days, 90);
        assert_eq!(schedule.last_rotation, last_rotation);
        assert_eq!(schedule.max_versions, 5);
        assert!(schedule.auto_rotate);
        // next_rotation = last_rotation + 90 days in seconds
        assert_eq!(schedule.next_rotation, last_rotation + 90 * SECONDS_PER_DAY);
    }

    #[test]
    fn test_key_rotation_schedule_new_saturates_on_overflow() {
        let near_max = u64::MAX;
        let schedule = KeyRotationSchedule::new("k".to_string(), 1, near_max, 1);
        // saturating_add keeps it at u64::MAX instead of overflowing
        assert_eq!(schedule.next_rotation, u64::MAX);
    }

    #[test]
    fn test_key_rotation_schedule_is_rotation_due_past() {
        let past = now_timestamp().saturating_sub(86_400);
        let schedule = KeyRotationSchedule::new("k".to_string(), 1, past, 5);
        // next_rotation is past + 1 day, which is now-ish, so should be due
        assert!(schedule.is_rotation_due());
    }

    #[test]
    fn test_key_rotation_schedule_is_rotation_due_future() {
        let now = now_timestamp();
        // last_rotation = now, interval = 90 days → next_rotation far in future
        let schedule = KeyRotationSchedule::new("k".to_string(), 90, now, 5);
        assert!(!schedule.is_rotation_due());
    }

    #[test]
    fn test_key_rotation_schedule_update_after_rotation() {
        let original_last = 1_000_000_u64;
        let mut schedule = KeyRotationSchedule::new("k".to_string(), 30, original_last, 5);
        let original_next = schedule.next_rotation;

        schedule.update_after_rotation();

        assert!(schedule.last_rotation > original_last);
        assert!(schedule.next_rotation > original_next);
        assert_eq!(
            schedule.next_rotation,
            schedule.last_rotation + 30 * SECONDS_PER_DAY
        );
    }

    #[test]
    fn test_key_rotation_schedule_days_until_rotation_future() {
        let now = now_timestamp();
        let schedule = KeyRotationSchedule::new("k".to_string(), 10, now, 5);
        // ~10 days remaining (within a day of 10 due to elapsed seconds)
        let days = schedule.days_until_rotation();
        assert!((9..=10).contains(&days), "expected ~10 days, got {}", days);
    }

    #[test]
    fn test_key_rotation_schedule_days_until_rotation_past() {
        let past = now_timestamp().saturating_sub(20 * 86_400);
        let schedule = KeyRotationSchedule::new("k".to_string(), 10, past, 5);
        let days = schedule.days_until_rotation();
        assert!(days < 0, "expected negative days, got {}", days);
    }

    #[test]
    fn test_now_timestamp_nonzero() {
        let t = now_timestamp();
        assert!(t > 0);
        // Two consecutive calls should be >= the first (monotonic-ish)
        let t2 = now_timestamp();
        assert!(t2 >= t);
    }

    #[test]
    fn test_rotation_plan_new_collects_versions() {
        let plan = RotationPlan::new("k1".to_string(), 3, 6);
        assert_eq!(plan.key_id, "k1");
        assert_eq!(plan.current_version, 3);
        assert_eq!(plan.target_version, 6);
        assert_eq!(plan.keys_to_rotate, vec![4, 5, 6]);
        assert!(plan.reencryption_required);
    }

    #[test]
    fn test_rotation_plan_new_adjacent_version() {
        let plan = RotationPlan::new("k1".to_string(), 5, 6);
        assert_eq!(plan.keys_to_rotate, vec![6]);
        assert!(plan.reencryption_required);
    }

    #[test]
    fn test_rotation_plan_new_same_version_empty() {
        let plan = RotationPlan::new("k1".to_string(), 5, 5);
        assert!(plan.keys_to_rotate.is_empty());
        assert!(!plan.reencryption_required);
    }

    #[test]
    fn test_constants() {
        assert_eq!(CONFERS_KEY_VERSION, "v1");
        assert_eq!(KEY_VERSION_PREFIX, "v");
        assert_eq!(CURRENT_KEY_VERSION, 1);
        assert_eq!(SECONDS_PER_DAY, 86_400);
    }

    #[test]
    fn test_key_status_debug_clone_serialize() {
        let status = KeyStatus::Active;
        let cloned = status;
        assert_eq!(status, cloned);
        let debug_str = format!("{:?}", status);
        assert_eq!(debug_str, "Active");
        let json = serde_json::to_string(&status).expect("serialize");
        assert_eq!(json, "\"Active\"");
    }

    #[test]
    fn test_key_metadata_serialize_deserialize() {
        let meta = KeyMetadata::new(2, "alice".to_string(), Some("d".to_string()));
        let json = serde_json::to_string(&meta).expect("serialize");
        let de: KeyMetadata = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(de.version, meta.version);
        assert_eq!(de.created_by, meta.created_by);
    }

    #[test]
    fn test_key_bundle_serialize_deserialize() {
        let bundle = KeyBundle::new(
            1,
            "k_1".to_string(),
            "enc".to_string(),
            "u".to_string(),
            None,
        );
        let json = serde_json::to_string(&bundle).expect("serialize");
        let de: KeyBundle = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(de.key_id, bundle.key_id);
        assert_eq!(de.encrypted_key, bundle.encrypted_key);
    }

    #[cfg(feature = "encryption")]
    #[test]
    fn test_key_bundle_generate_and_get_plaintext_key_round_trip() {
        let master_key = [0x11; 32];
        let bundle = KeyBundle::generate(
            &master_key,
            1,
            "creator".to_string(),
            Some("test key".to_string()),
        )
        .expect("generate");

        assert_eq!(bundle.metadata.version, 1);
        assert_eq!(bundle.metadata.created_by, "creator");
        assert_eq!(bundle.metadata.status, KeyStatus::Active);
        assert!(bundle.encrypted_key.contains(':'));
        assert_eq!(bundle.key_id, format!("{}_1", KEY_VERSION_PREFIX));

        let plaintext = bundle
            .get_plaintext_key(&master_key)
            .expect("decrypt round-trip");
        assert_eq!(plaintext.len(), 32);
    }

    #[cfg(feature = "encryption")]
    #[test]
    fn test_key_bundle_get_plaintext_key_invalid_format() {
        let master_key = [0u8; 32];
        let mut bundle =
            KeyBundle::generate(&master_key, 1, "u".to_string(), None).expect("generate");
        bundle.encrypted_key = "no-colon-here".to_string();
        let err = bundle.get_plaintext_key(&master_key).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("Invalid encrypted key format"), "got: {}", msg);
    }

    #[cfg(feature = "encryption")]
    #[test]
    fn test_key_bundle_get_plaintext_key_wrong_master_key() {
        let master_key = [0x42; 32];
        let wrong_key = [0x99; 32];
        let bundle = KeyBundle::generate(&master_key, 1, "u".to_string(), None).expect("generate");
        let err = bundle.get_plaintext_key(&wrong_key).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("Decryption failed"), "got: {}", msg);
    }

    #[cfg(feature = "encryption")]
    #[test]
    fn test_key_ring_new_initializes_primary_key() {
        let master_key = [0x01; 32];
        let ring = KeyRing::new(&master_key, "prod".to_string(), "team".to_string())
            .expect("new key ring");
        assert_eq!(ring.key_id, "prod");
        assert_eq!(ring.current_version, CURRENT_KEY_VERSION);
        assert!(ring.secondary_keys.is_empty());
        assert!(ring.created_at > 0);
        assert_eq!(ring.last_rotated_at, None);
        assert_eq!(ring.primary_key.metadata.version, CURRENT_KEY_VERSION);
        assert_eq!(ring.primary_key.metadata.status, KeyStatus::Active);
    }

    #[cfg(feature = "encryption")]
    #[test]
    fn test_key_ring_rotate_increments_version_and_archives_old_primary() {
        let master_key = [0x02; 32];
        let mut ring = KeyRing::new(&master_key, "k".to_string(), "u".to_string()).unwrap();
        let old_primary_version = ring.primary_key.metadata.version;
        let old_primary = ring.primary_key.clone();

        let new_key = ring
            .rotate(
                &master_key,
                "rotator".to_string(),
                Some("scheduled".to_string()),
            )
            .expect("rotate");

        assert_eq!(new_key.metadata.version, old_primary_version + 1);
        assert_eq!(ring.current_version, old_primary_version + 1);
        assert_eq!(ring.primary_key.metadata.version, new_key.metadata.version);
        // Old primary moved to secondaries
        assert_eq!(ring.secondary_keys.len(), 1);
        assert_eq!(ring.secondary_keys[0].metadata.version, old_primary_version);
        assert_eq!(
            ring.secondary_keys[0].encrypted_key,
            old_primary.encrypted_key
        );
        assert!(ring.last_rotated_at.is_some());
    }

    #[test]
    fn test_key_ring_get_key_by_version_returns_primary() {
        let bundle = KeyBundle::new(
            5,
            "k_5".to_string(),
            "enc".to_string(),
            "u".to_string(),
            None,
        );
        let ring = KeyRing {
            key_id: "k".to_string(),
            current_version: 5,
            primary_key: bundle.clone(),
            secondary_keys: vec![],
            created_at: 0,
            last_rotated_at: None,
        };
        let found = ring.get_key_by_version(5);
        assert!(found.is_some());
        assert_eq!(found.unwrap().key_id, bundle.key_id);
    }

    #[test]
    fn test_key_ring_get_key_by_version_returns_secondary() {
        let primary = KeyBundle::new(
            2,
            "k_2".to_string(),
            "e2".to_string(),
            "u".to_string(),
            None,
        );
        let secondary = KeyBundle::new(
            1,
            "k_1".to_string(),
            "e1".to_string(),
            "u".to_string(),
            None,
        );
        let ring = KeyRing {
            key_id: "k".to_string(),
            current_version: 2,
            primary_key: primary,
            secondary_keys: vec![secondary.clone()],
            created_at: 0,
            last_rotated_at: None,
        };
        let found = ring.get_key_by_version(1);
        assert!(found.is_some());
        assert_eq!(found.unwrap().key_id, secondary.key_id);
        // Missing version returns None
        assert!(ring.get_key_by_version(99).is_none());
    }

    #[test]
    fn test_key_ring_add_secondary_key() {
        let primary = KeyBundle::new(
            1,
            "k_1".to_string(),
            "e1".to_string(),
            "u".to_string(),
            None,
        );
        let extra = KeyBundle::new(
            2,
            "k_2".to_string(),
            "e2".to_string(),
            "u".to_string(),
            None,
        );
        let mut ring = KeyRing {
            key_id: "k".to_string(),
            current_version: 1,
            primary_key: primary,
            secondary_keys: vec![],
            created_at: 0,
            last_rotated_at: None,
        };
        assert_eq!(ring.secondary_keys.len(), 0);
        ring.add_secondary_key(extra);
        assert_eq!(ring.secondary_keys.len(), 1);
    }

    #[test]
    fn test_key_ring_deactivate_version_marks_deprecated() {
        let primary = KeyBundle::new(
            2,
            "k_2".to_string(),
            "e2".to_string(),
            "u".to_string(),
            None,
        );
        let secondary = KeyBundle::new(
            1,
            "k_1".to_string(),
            "e1".to_string(),
            "u".to_string(),
            None,
        );
        let mut ring = KeyRing {
            key_id: "k".to_string(),
            current_version: 2,
            primary_key: primary,
            secondary_keys: vec![secondary],
            created_at: 0,
            last_rotated_at: None,
        };
        ring.deactivate_version(1);
        let v1 = ring.get_key_by_version(1).unwrap();
        assert_eq!(v1.metadata.status, KeyStatus::Deprecated);
    }

    #[test]
    fn test_key_ring_deactivate_version_no_op_for_missing() {
        let primary = KeyBundle::new(
            1,
            "k_1".to_string(),
            "e1".to_string(),
            "u".to_string(),
            None,
        );
        let mut ring = KeyRing {
            key_id: "k".to_string(),
            current_version: 1,
            primary_key: primary,
            secondary_keys: vec![],
            created_at: 0,
            last_rotated_at: None,
        };
        // Missing version: deactivate is a no-op (no panic, no change)
        ring.deactivate_version(99);
        assert_eq!(ring.primary_key.metadata.status, KeyStatus::Active);
    }
}
