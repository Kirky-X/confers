// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

use crate::encryption::ConfigEncryption;
use crate::error::ConfigError;
use crate::key::KeyManager;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedKeyStore {
    pub version: u32,
    pub encrypted_data: String,
    pub checksum: String,
    pub created_at: u64,
    pub metadata: KeyStoreMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyStoreMetadata {
    pub key_id: String,
    pub key_count: u32,
    pub last_modified: u64,
    pub schema_version: u32,
}

pub struct KeyStorage {
    storage_path: PathBuf,
    master_key: Option<[u8; 32]>,
    key_manager: KeyManager,
}

impl KeyStorage {
    pub fn new(storage_path: PathBuf) -> Result<Self, ConfigError> {
        fs::create_dir_all(&storage_path).map_err(|e| {
            ConfigError::RuntimeError(format!("Failed to create key storage directory: {}", e))
        })?;

        let key_manager = KeyManager::new(storage_path.join("keys.json"))?;

        Ok(Self {
            storage_path,
            master_key: None,
            key_manager,
        })
    }

    pub fn set_master_key(&mut self, master_key: &[u8; 32]) {
        self.master_key = Some(*master_key);
    }

    pub fn initialize_with_master_key(
        &mut self,
        master_key: &[u8; 32],
        key_id: String,
        created_by: String,
    ) -> Result<(), ConfigError> {
        self.master_key = Some(*master_key);
        self.key_manager
            .initialize(master_key, key_id, created_by)?;
        self.save()?;
        Ok(())
    }

    pub fn save(&self) -> Result<(), ConfigError> {
        let master_key = self
            .master_key
            .ok_or_else(|| ConfigError::RuntimeError("Master key not set".to_string()))?;

        let key_data = self.serialize_key_manager()?;
        let encrypted_data = self.encrypt_data(&key_data, &master_key)?;
        let checksum = KeyStorage::calculate_checksum(&encrypted_data);

        let store = EncryptedKeyStore {
            version: 1,
            encrypted_data,
            checksum,
            created_at: now_timestamp(),
            metadata: KeyStoreMetadata {
                key_id: self.key_manager.get_default_key_id().to_string(),
                key_count: self.key_manager.list_keys().len() as u32,
                last_modified: now_timestamp(),
                schema_version: 1,
            },
        };

        self.write_store(&store)?;
        Ok(())
    }

    pub fn load(&mut self) -> Result<(), ConfigError> {
        let master_key = self
            .master_key
            .ok_or_else(|| ConfigError::RuntimeError("Master key not set".to_string()))?;

        if !self.storage_path.join("keys.json").exists() {
            return Ok(());
        }

        let store = self.read_store()?;
        self.validate_checksum(&store)?;
        let key_data = self.decrypt_data(&store.encrypted_data, &master_key)?;
        self.deserialize_key_manager(&key_data)?;

        Ok(())
    }

    fn serialize_key_manager(&self) -> Result<String, ConfigError> {
        let data = serde_json::to_vec(&self.key_manager).map_err(|e| {
            ConfigError::SerializationError(format!("Failed to serialize key manager: {}", e))
        })?;
        Ok(BASE64.encode(data))
    }

    fn deserialize_key_manager(&mut self, data: &str) -> Result<(), ConfigError> {
        let bytes = BASE64
            .decode(data)
            .map_err(|e| ConfigError::FormatDetectionFailed(format!("Invalid key data: {}", e)))?;
        let key_manager: KeyManager = serde_json::from_slice(&bytes).map_err(|e| {
            ConfigError::ParseError(format!("Failed to deserialize key manager: {}", e))
        })?;
        self.key_manager = key_manager;
        Ok(())
    }

    fn encrypt_data(&self, data: &str, master_key: &[u8; 32]) -> Result<String, ConfigError> {
        let encryptor = ConfigEncryption::new(*master_key);
        encryptor.encrypt(data)
    }

    fn decrypt_data(&self, encrypted: &str, master_key: &[u8; 32]) -> Result<String, ConfigError> {
        let encryptor = ConfigEncryption::new(*master_key);
        encryptor.decrypt(encrypted)
    }

    fn calculate_checksum(data: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        data.hash(&mut hasher);
        let hash = hasher.finish();
        BASE64.encode(hash.to_ne_bytes())
    }

    fn validate_checksum(&self, store: &EncryptedKeyStore) -> Result<(), ConfigError> {
        let calculated = Self::calculate_checksum(&store.encrypted_data);
        if store.checksum != calculated {
            return Err(ConfigError::FormatDetectionFailed(
                "Key store checksum mismatch".to_string(),
            ));
        }
        Ok(())
    }

    fn write_store(&self, store: &EncryptedKeyStore) -> Result<(), ConfigError> {
        let store_path = self.storage_path.join("keys.json");
        let json = serde_json::to_string_pretty(store).map_err(|e| {
            ConfigError::SerializationError(format!("Failed to serialize key store: {}", e))
        })?;

        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&store_path)
            .map_err(|e| ConfigError::IoError(format!("Failed to open key store: {}", e)))?;

        file.write_all(json.as_bytes())
            .map_err(|e| ConfigError::IoError(format!("Failed to write key store: {}", e)))?;

        Ok(())
    }

    fn read_store(&self) -> Result<EncryptedKeyStore, ConfigError> {
        let store_path = self.storage_path.join("keys.json");
        let mut file = File::open(&store_path)
            .map_err(|e| ConfigError::IoError(format!("Failed to open key store: {}", e)))?;

        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .map_err(|e| ConfigError::IoError(format!("Failed to read key store: {}", e)))?;

        serde_json::from_str(&contents)
            .map_err(|e| ConfigError::ParseError(format!("Failed to parse key store: {}", e)))
    }

    pub fn export_keys(&self, output_path: &PathBuf) -> Result<(), ConfigError> {
        let master_key = self
            .master_key
            .ok_or_else(|| ConfigError::RuntimeError("Master key not set".to_string()))?;

        let key_data = self.serialize_key_manager()?;
        let encrypted_data = self.encrypt_data(&key_data, &master_key)?;

        let export = KeyExport {
            version: 1,
            exported_at: now_timestamp(),
            encrypted_data,
        };

        let json = serde_json::to_string_pretty(&export).map_err(|e| {
            ConfigError::SerializationError(format!("Failed to serialize export: {}", e))
        })?;

        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(output_path)
            .map_err(|e| ConfigError::IoError(format!("Failed to create export file: {}", e)))?;

        file.write_all(json.as_bytes())
            .map_err(|e| ConfigError::IoError(format!("Failed to write export file: {}", e)))?;

        Ok(())
    }

    pub fn import_keys(
        &mut self,
        input_path: &PathBuf,
        master_key: &[u8; 32],
    ) -> Result<(), ConfigError> {
        self.master_key = Some(*master_key);

        let mut file = File::open(input_path)
            .map_err(|e| ConfigError::IoError(format!("Failed to open import file: {}", e)))?;

        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .map_err(|e| ConfigError::IoError(format!("Failed to read import file: {}", e)))?;

        let export: KeyExport = serde_json::from_str(&contents)
            .map_err(|e| ConfigError::ParseError(format!("Failed to parse import file: {}", e)))?;

        self.validate_checksum_by_data(&export.encrypted_data)?;
        let key_data = self.decrypt_data(&export.encrypted_data, master_key)?;
        self.deserialize_key_manager(&key_data)?;

        self.save()?;
        Ok(())
    }

    fn validate_checksum_by_data(&self, encrypted_data: &str) -> Result<(), ConfigError> {
        let checksum = Self::calculate_checksum(encrypted_data);
        let store = self.read_store()?;
        if store.checksum != checksum {
            return Err(ConfigError::FormatDetectionFailed(
                "Import checksum mismatch".to_string(),
            ));
        }
        Ok(())
    }

    pub fn get_key_manager(&self) -> &KeyManager {
        &self.key_manager
    }

    pub fn get_key_manager_mut(&mut self) -> &mut KeyManager {
        &mut self.key_manager
    }

    pub fn backup(&self, backup_path: &PathBuf) -> Result<(), ConfigError> {
        let timestamp = now_timestamp();
        let backup_file = backup_path.join(format!("keys_backup_{}.json", timestamp));

        let master_key = self
            .master_key
            .ok_or_else(|| ConfigError::RuntimeError("Master key not set".to_string()))?;

        let key_data = self.serialize_key_manager()?;
        let encrypted_data = self.encrypt_data(&key_data, &master_key)?;

        let backup = KeyExport {
            version: 1,
            exported_at: now_timestamp(),
            encrypted_data,
        };

        let json = serde_json::to_string_pretty(&backup).map_err(|e| {
            ConfigError::SerializationError(format!("Failed to serialize backup: {}", e))
        })?;

        fs::create_dir_all(backup_path).map_err(|e| {
            ConfigError::RuntimeError(format!("Failed to create backup directory: {}", e))
        })?;

        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&backup_file)
            .map_err(|e| ConfigError::IoError(format!("Failed to create backup file: {}", e)))?;

        file.write_all(json.as_bytes())
            .map_err(|e| ConfigError::IoError(format!("Failed to write backup file: {}", e)))?;

        Ok(())
    }

    pub fn list_backups(&self, backup_path: &PathBuf) -> Result<Vec<BackupInfo>, ConfigError> {
        let mut backups = Vec::new();

        if let Ok(entries) = fs::read_dir(backup_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                    if file_name.starts_with("keys_backup_") && file_name.ends_with(".json") {
                        let timestamp_str = file_name
                            .strip_prefix("keys_backup_")
                            .and_then(|s| s.strip_suffix(".json"))
                            .and_then(|s| s.parse::<u64>().ok());

                        if let Some(timestamp) = timestamp_str {
                            let backup_path = path.clone();
                            backups.push(BackupInfo {
                                path: backup_path,
                                timestamp,
                                file_name: file_name.to_string(),
                            });
                        }
                    }
                }
            }
        }

        backups.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        Ok(backups)
    }

    pub fn rotate_master_key(
        &mut self,
        old_master_key: &[u8; 32],
        new_master_key: &[u8; 32],
    ) -> Result<(), ConfigError> {
        let key_data = self.serialize_key_manager()?;
        let decrypted_data = self.decrypt_data(&key_data, old_master_key)?;
        let reencrypted_data = self.encrypt_data(&decrypted_data, new_master_key)?;

        self.master_key = Some(*new_master_key);

        let checksum = Self::calculate_checksum(&reencrypted_data);
        let store = EncryptedKeyStore {
            version: 1,
            encrypted_data: reencrypted_data,
            checksum,
            created_at: now_timestamp(),
            metadata: KeyStoreMetadata {
                key_id: self.key_manager.get_default_key_id().to_string(),
                key_count: self.key_manager.list_keys().len() as u32,
                last_modified: now_timestamp(),
                schema_version: 1,
            },
        };

        self.write_store(&store)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyExport {
    pub version: u32,
    pub exported_at: u64,
    pub encrypted_data: String,
}

#[derive(Debug, Clone)]
pub struct BackupInfo {
    pub path: PathBuf,
    pub timestamp: u64,
    pub file_name: String,
}

fn now_timestamp() -> u64 {
    use std::time::SystemTime;
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or(std::time::Duration::ZERO)
        .as_secs()
}

const _: () = ();
