// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

mod manager;
mod rotation;
#[cfg(feature = "encryption")]
mod storage;

pub use manager::{KeyInfo, KeyManager, KeyVersion};
pub use rotation::{KeyRotationPolicy, KeyRotationService, RotationResult};
#[cfg(feature = "encryption")]
pub use storage::KeyStorage;

#[cfg(feature = "encryption")]
use crate::encryption::ConfigEncryption;
use crate::error::ConfigError;
#[cfg(feature = "encryption")]
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[cfg(feature = "encryption")]
use rand::Rng;

pub const CONFERS_KEY_VERSION: &str = "v1";
pub const KEY_VERSION_PREFIX: &str = "v";
pub const CURRENT_KEY_VERSION: u32 = 1;

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
        let mut rng = rand::rng();
        rng.fill(&mut key_bytes);

        let encryptor = ConfigEncryption::new(*master_key);
        let encrypted_key = encryptor.encrypt(&BASE64.encode(key_bytes))?;

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
        let encryptor = ConfigEncryption::new(*master_key);
        let decrypted = encryptor.decrypt(&self.encrypted_key)?;
        let key_bytes = BASE64
            .decode(&decrypted)
            .map_err(|e| ConfigError::FormatDetectionFailed(format!("Invalid key bytes: {}", e)))?;

        if key_bytes.len() != 32 {
            return Err(ConfigError::FormatDetectionFailed(
                "Invalid key length".to_string(),
            ));
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
        let next_rotation = last_rotation.saturating_add(rotation_interval_days as u64 * 86400);

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
            .saturating_add(self.rotation_interval_days as u64 * 86400);
    }

    pub fn days_until_rotation(&self) -> i64 {
        let now = now_timestamp() as i64;
        let next = self.next_rotation as i64;
        (next - now) / 86400
    }
}

fn now_timestamp() -> u64 {
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
