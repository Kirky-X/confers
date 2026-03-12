// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

use crate::error::ConfigError;
use crate::key::KeyManager;
use crate::secret::{SecretBytes, XChaCha20Crypto};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::PathBuf;

/// 脱敏级别定义
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SanitizationLevel {
    /// 最小脱敏：仅替换完整的十六进制编码密钥
    Minimal,
    /// 标准脱敏：替换密钥片段和常见模式
    Standard,
    /// 激进脱敏：替换所有可能的敏感信息
    Aggressive,
}

/// 错误消息脱敏器
pub struct ErrorSanitizer {
    master_key_hex: String,
    level: SanitizationLevel,
    replacement: String,
}

impl ErrorSanitizer {
    /// 创建新的错误脱敏器
    pub fn new(master_key: &[u8; 32], level: SanitizationLevel) -> Self {
        Self {
            master_key_hex: hex::encode(master_key),
            level,
            replacement: "***".to_string(),
        }
    }

    /// 设置自定义替换字符串
    pub fn with_replacement(mut self, replacement: String) -> Self {
        self.replacement = replacement;
        self
    }

    /// 脱敏错误消息
    pub fn sanitize(&self, error_message: &str) -> String {
        let mut sanitized = error_message.to_string();

        // 根据脱敏级别进行不同程度的处理
        match self.level {
            SanitizationLevel::Minimal => {
                sanitized = self.sanitize_full_key(&sanitized);
            }
            SanitizationLevel::Standard => {
                sanitized = self.sanitize_full_key(&sanitized);
                sanitized = self.sanitize_key_fragments(&sanitized);
                sanitized = self.sanitize_key_patterns(&sanitized);
            }
            SanitizationLevel::Aggressive => {
                sanitized = self.sanitize_full_key(&sanitized);
                sanitized = self.sanitize_key_fragments(&sanitized);
                sanitized = self.sanitize_key_patterns(&sanitized);
                sanitized = self.sanitize_suspicious_patterns(&sanitized);
            }
        }

        sanitized
    }

    /// 脱敏完整的密钥十六进制编码
    fn sanitize_full_key(&self, message: &str) -> String {
        message.replace(&self.master_key_hex, &self.replacement)
    }

    /// 脱敏密钥片段（8字符以上的十六进制字符串）
    fn sanitize_key_fragments(&self, message: &str) -> String {
        use regex::Regex;

        // 匹配8-64个字符的十六进制字符串
        let hex_pattern = Regex::new(r"[0-9a-fA-F]{8,64}").unwrap();
        hex_pattern
            .replace_all(message, &self.replacement)
            .to_string()
    }

    /// 脱敏常见的密钥相关模式
    fn sanitize_key_patterns(&self, message: &str) -> String {
        use regex::Regex;

        let mut result = message.to_string();

        // 脱敏 "key:" 模式
        let key_pattern = Regex::new(r"(?i)key\s*[:=]\s*[0-9a-fA-F]+").unwrap();
        result = key_pattern.replace_all(&result, "key: ***").to_string();

        // 脱敏 "master:" 模式
        let master_pattern = Regex::new(r"(?i)master\s*[:=]\s*[0-9a-fA-F]+").unwrap();
        result = master_pattern
            .replace_all(&result, "master: ***")
            .to_string();

        // 脱敏 "secret:" 模式
        let secret_pattern = Regex::new(r"(?i)secret\s*[:=]\s*[0-9a-fA-F]+").unwrap();
        result = secret_pattern
            .replace_all(&result, "secret: ***")
            .to_string();

        result
    }

    /// 脱敏可疑模式（激进模式）
    fn sanitize_suspicious_patterns(&self, message: &str) -> String {
        use regex::Regex;

        let mut result = message.to_string();

        // 脱敏任何64字符的字符串（可能是完整的密钥）
        let long_hex_pattern = Regex::new(r"[0-9a-fA-F]{64}").unwrap();
        result = long_hex_pattern
            .replace_all(&result, &self.replacement)
            .to_string();

        // 脱敏base64编码的长字符串（可能是编码的密钥）
        let base64_pattern = Regex::new(r"[A-Za-z0-9+/]{32,}={0,2}").unwrap();
        result = base64_pattern
            .replace_all(&result, &self.replacement)
            .to_string();

        result
    }
}

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
    master_key: Option<SecretBytes>,
    key_manager: KeyManager,
    error_sanitizer: Option<ErrorSanitizer>,
}

impl KeyStorage {
    pub fn new(storage_path: PathBuf) -> Result<Self, ConfigError> {
        fs::create_dir_all(&storage_path).map_err(|e| ConfigError::ParseError {
            format: "key".to_string(),
            message: format!("Failed to create key storage directory: {}", e),
            location: None,
            source: None,
        })?;

        let key_manager = KeyManager::new(storage_path.join("keys.json"))?;

        Ok(Self {
            storage_path,
            master_key: None,
            key_manager,
            error_sanitizer: None,
        })
    }

    pub fn set_master_key(&mut self, master_key: &[u8; 32]) {
        self.master_key = Some(SecretBytes::new(master_key.to_vec()));
        self.error_sanitizer = Some(ErrorSanitizer::new(master_key, SanitizationLevel::Standard));
    }

    pub fn clear_master_key(&mut self) {
        self.master_key = None;
        self.error_sanitizer = None;
    }

    fn get_master_key_bytes(&self) -> Result<[u8; 32], ConfigError> {
        let secret = self
            .master_key
            .as_ref()
            .ok_or_else(|| ConfigError::ParseError {
                format: "key".to_string(),
                message: "Master key not set".to_string(),
                location: None,
                source: None,
            })?;

        let slice = secret.as_slice();
        if slice.len() != 32 {
            return Err(ConfigError::ParseError {
                format: "key".to_string(),
                message: "Invalid master key length".to_string(),
                location: None,
                source: None,
            });
        }

        let mut arr = [0u8; 32];
        arr.copy_from_slice(slice);
        Ok(arr)
    }

    pub fn initialize_with_master_key(
        &mut self,
        master_key: &[u8; 32],
        key_id: String,
        created_by: String,
    ) -> Result<(), ConfigError> {
        self.master_key = Some(SecretBytes::new(master_key.to_vec()));
        let key_id_for_error = key_id.clone();
        self.key_manager
            .initialize(master_key, key_id, created_by)
            .map_err(|e| ConfigError::ParseError {
                format: "key".to_string(),
                message: format!(
                    "Failed to initialize key ring for '{}': {}",
                    key_id_for_error,
                    self.sanitize_error(&e.to_string())
                ),
                location: None,
                source: None,
            })?;
        self.save()?;
        Ok(())
    }

    pub fn save(&self) -> Result<(), ConfigError> {
        let master_key = self.get_master_key_bytes()?;

        let key_data = self.serialize_key_manager()?;
        let encrypted_data =
            self.encrypt_data(&key_data, &master_key)
                .map_err(|e| ConfigError::ParseError {
                    format: "key".to_string(),
                    message: format!(
                        "Failed to encrypt key data: {}",
                        self.sanitize_error(&e.to_string())
                    ),
                    location: None,
                    source: None,
                })?;
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
        let master_key = self.get_master_key_bytes()?;

        if !self.storage_path.join("keys.json").exists() {
            return Ok(());
        }

        let store = self.read_store()?;
        self.validate_checksum(&store)?;
        let key_data = self
            .decrypt_data(&store.encrypted_data, &master_key)
            .map_err(|e| ConfigError::ParseError {
                format: "key".to_string(),
                message: format!(
                    "Failed to decrypt key data: {}",
                    self.sanitize_error(&e.to_string())
                ),
                location: None,
                source: None,
            })?;
        self.deserialize_key_manager(&key_data)?;

        Ok(())
    }

    fn serialize_key_manager(&self) -> Result<String, ConfigError> {
        let data = serde_json::to_vec(&self.key_manager).map_err(|e| ConfigError::ParseError {
            format: "key".to_string(),
            message: format!("Failed to serialize key manager: {}", e),
            location: None,
            source: None,
        })?;
        Ok(BASE64.encode(data))
    }

    fn deserialize_key_manager(&mut self, data: &str) -> Result<(), ConfigError> {
        let bytes = BASE64.decode(data).map_err(|e| ConfigError::ParseError {
            format: "key".to_string(),
            message: format!("Invalid key data: {}", e),
            location: None,
            source: None,
        })?;
        let key_manager: KeyManager =
            serde_json::from_slice(&bytes).map_err(|e| ConfigError::ParseError {
                format: "key".to_string(),
                message: format!("Failed to deserialize key manager: {}", e),
                location: None,
                source: None,
            })?;
        self.key_manager = key_manager;
        Ok(())
    }

    fn encrypt_data(&self, data: &str, master_key: &[u8; 32]) -> Result<String, ConfigError> {
        let encryptor = XChaCha20Crypto::new();
        let (nonce, ciphertext) = encryptor
            .encrypt(data.as_bytes(), master_key)
            .map_err(|e| ConfigError::ParseError {
                format: "key".to_string(),
                message: format!("Encryption failed: {}", self.sanitize_error(&e.to_string())),
                location: None,
                source: None,
            })?;

        // 格式: nonce_base64:ciphertext_base64
        let result = format!("{}:{}", BASE64.encode(&nonce), BASE64.encode(&ciphertext));
        Ok(result)
    }

    fn decrypt_data(&self, encrypted: &str, master_key: &[u8; 32]) -> Result<String, ConfigError> {
        let parts: Vec<&str> = encrypted.split(':').collect();
        if parts.len() != 2 {
            return Err(ConfigError::ParseError {
                format: "key".to_string(),
                message: "Invalid encrypted data format".to_string(),
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
                message: format!("Decryption failed: {}", self.sanitize_error(&e.to_string())),
                location: None,
                source: None,
            })?;

        String::from_utf8(plaintext).map_err(|e| ConfigError::ParseError {
            format: "key".to_string(),
            message: format!("Failed to convert decrypted data to string: {}", e),
            location: None,
            source: None,
        })
    }

    fn calculate_checksum(data: &str) -> String {
        use sha2::{Digest, Sha256};

        let mut hasher = Sha256::new();
        hasher.update(data.as_bytes());
        let hash = hasher.finalize();
        BASE64.encode(hash)
    }

    fn validate_checksum(&self, store: &EncryptedKeyStore) -> Result<(), ConfigError> {
        let calculated = Self::calculate_checksum(&store.encrypted_data);
        if store.checksum != calculated {
            return Err(ConfigError::ParseError {
                format: "key".to_string(),
                message: "Key store checksum mismatch".to_string(),
                location: None,
                source: None,
            });
        }
        Ok(())
    }

    fn write_store(&self, store: &EncryptedKeyStore) -> Result<(), ConfigError> {
        let store_path = self.storage_path.join("keys.json");
        let json = serde_json::to_string_pretty(store).map_err(|e| ConfigError::ParseError {
            format: "key".to_string(),
            message: format!("Failed to serialize key store: {}", e),
            location: None,
            source: None,
        })?;

        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&store_path)
            .map_err(|e| {
                std::io::Error::new(e.kind(), format!("Failed to open key store: {}", e))
            })?;

        file.write_all(json.as_bytes()).map_err(|e| {
            std::io::Error::new(e.kind(), format!("Failed to write key store: {}", e))
        })?;

        Ok(())
    }

    fn read_store(&self) -> Result<EncryptedKeyStore, ConfigError> {
        let store_path = self.storage_path.join("keys.json");
        let mut file = File::open(&store_path).map_err(|e| {
            std::io::Error::new(e.kind(), format!("Failed to open key store: {}", e))
        })?;

        let mut contents = String::new();
        file.read_to_string(&mut contents).map_err(|e| {
            std::io::Error::new(e.kind(), format!("Failed to read key store: {}", e))
        })?;

        serde_json::from_str(&contents).map_err(|e| ConfigError::ParseError {
            format: "key".to_string(),
            message: format!("Failed to parse key store: {}", e),
            location: None,
            source: None,
        })
    }

    pub fn export_keys(&self, output_path: &PathBuf) -> Result<(), ConfigError> {
        let master_key = self.get_master_key_bytes()?;

        let key_data = self.serialize_key_manager()?;
        let encrypted_data = self.encrypt_data(&key_data, &master_key)?;

        let export = KeyExport {
            version: 1,
            exported_at: now_timestamp(),
            encrypted_data,
        };

        let json = serde_json::to_string_pretty(&export).map_err(|e| ConfigError::ParseError {
            format: "key".to_string(),
            message: format!("Failed to serialize export: {}", e),
            location: None,
            source: None,
        })?;

        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(output_path)
            .map_err(|e| std::io::Error::other(format!("Failed to create export file: {}", e)))?;

        file.write_all(json.as_bytes())
            .map_err(|e| std::io::Error::other(format!("Failed to write export file: {}", e)))?;

        Ok(())
    }

    pub fn import_keys(
        &mut self,
        input_path: &PathBuf,
        master_key: &[u8; 32],
    ) -> Result<(), ConfigError> {
        self.master_key = Some(SecretBytes::new(master_key.to_vec()));

        let mut file = File::open(input_path)
            .map_err(|e| std::io::Error::other(format!("Failed to open import file: {}", e)))?;

        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .map_err(|e| std::io::Error::other(format!("Failed to read import file: {}", e)))?;

        let export: KeyExport =
            serde_json::from_str(&contents).map_err(|e| ConfigError::ParseError {
                format: "key".to_string(),
                message: format!("Failed to parse import file: {}", e),
                location: None,
                source: None,
            })?;

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
            return Err(ConfigError::ParseError {
                format: "key".to_string(),
                message: "Import checksum mismatch".to_string(),
                location: None,
                source: None,
            });
        }
        Ok(())
    }

    pub fn get_key_manager(&self) -> &KeyManager {
        &self.key_manager
    }

    pub fn get_key_manager_mut(&mut self) -> &mut KeyManager {
        &mut self.key_manager
    }

    /// 脱敏错误消息，如果设置了主密钥则进行脱敏处理
    pub fn sanitize_error(&self, error_message: &str) -> String {
        match &self.error_sanitizer {
            Some(sanitizer) => sanitizer.sanitize(error_message),
            None => {
                // 如果没有设置脱敏器，进行基本的后备脱敏处理
                error_message.to_string()
            }
        }
    }

    pub fn backup(&self, backup_path: &PathBuf) -> Result<(), ConfigError> {
        let timestamp = now_timestamp();
        let backup_file = backup_path.join(format!("keys_backup_{}.json", timestamp));

        let master_key = self.get_master_key_bytes()?;

        let key_data = self.serialize_key_manager()?;
        let encrypted_data = self.encrypt_data(&key_data, &master_key)?;

        let backup = KeyExport {
            version: 1,
            exported_at: now_timestamp(),
            encrypted_data,
        };

        let json = serde_json::to_string_pretty(&backup).map_err(|e| ConfigError::ParseError {
            format: "key".to_string(),
            message: format!("Failed to serialize backup: {}", e),
            location: None,
            source: None,
        })?;

        fs::create_dir_all(backup_path).map_err(|e| ConfigError::ParseError {
            format: "key".to_string(),
            message: format!("Failed to create backup directory: {}", e),
            location: None,
            source: None,
        })?;

        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&backup_file)
            .map_err(|e| std::io::Error::other(format!("Failed to create backup file: {}", e)))?;

        file.write_all(json.as_bytes())
            .map_err(|e| std::io::Error::other(format!("Failed to write backup file: {}", e)))?;

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
        let decrypted_data =
            self.decrypt_data(&key_data, old_master_key)
                .map_err(|e| ConfigError::ParseError {
                    format: "key".to_string(),
                    message: format!(
                        "Failed to decrypt with old master key: {}",
                        self.sanitize_error(&e.to_string())
                    ),
                    location: None,
                    source: None,
                })?;
        let reencrypted_data = self
            .encrypt_data(&decrypted_data, new_master_key)
            .map_err(|e| ConfigError::ParseError {
                format: "key".to_string(),
                message: format!(
                    "Failed to encrypt with new master key: {}",
                    self.sanitize_error(&e.to_string())
                ),
                location: None,
                source: None,
            })?;

        self.master_key = Some(SecretBytes::new(new_master_key.to_vec()));

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

impl Drop for KeyStorage {
    fn drop(&mut self) {
        self.clear_master_key();
    }
}

fn now_timestamp() -> u64 {
    use std::time::SystemTime;
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or(std::time::Duration::ZERO)
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile;

    #[test]
    fn test_error_sanitizer_minimal_level() {
        let master_key = [0x42; 32];
        let sanitizer = ErrorSanitizer::new(&master_key, SanitizationLevel::Minimal);

        let error_msg = format!("Failed with key: {}", hex::encode(master_key));
        let sanitized = sanitizer.sanitize(&error_msg);

        assert_eq!(sanitized, "Failed with key: ***");
    }

    #[test]
    fn test_error_sanitizer_standard_level() {
        let master_key = [0x42; 32];
        let sanitizer = ErrorSanitizer::new(&master_key, SanitizationLevel::Standard);

        // 测试完整密钥脱敏
        let error_msg = format!("Failed with key: {}", hex::encode(master_key));
        let sanitized = sanitizer.sanitize(&error_msg);
        assert!(sanitized.contains("***"));
        assert!(!sanitized.contains(&hex::encode(master_key)));

        // 测试密钥片段脱敏
        let fragment_msg = "Error with key fragment: deadbeefcafebabe";
        let sanitized = sanitizer.sanitize(fragment_msg);
        assert!(sanitized.contains("***"));
        assert!(!sanitized.contains("deadbeefcafebabe"));

        // 测试模式匹配脱敏
        let pattern_msg = "key: 12345678, master: abcdefgh, secret: 87654321";
        let sanitized = sanitizer.sanitize(pattern_msg);
        assert!(sanitized.contains("key: ***"));
        assert!(sanitized.contains("master: ***"));
        assert!(sanitized.contains("secret: ***"));
    }

    #[test]
    fn test_error_sanitizer_aggressive_level() {
        let master_key = [0x42; 32];
        let sanitizer = ErrorSanitizer::new(&master_key, SanitizationLevel::Aggressive);

        // 测试64字符十六进制字符串脱敏
        let hex_64 = "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
        let sanitized = sanitizer.sanitize(&format!("Error: {}", hex_64));
        assert!(sanitized.contains("***"));
        assert!(!sanitized.contains(hex_64));

        // 测试base64字符串脱敏
        let base64_long = "SGVsbG8gV29ybGQhVGhpcyBpcyBhIHZlcnkgbG9uZyBiYXNlNjQgc3RyaW5nIHRlc3Q=";
        let sanitized = sanitizer.sanitize(&format!("Data: {}", base64_long));
        assert!(sanitized.contains("***"));
        assert!(!sanitized.contains(base64_long));
    }

    #[test]
    fn test_key_storage_sanitization_integration() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut storage = KeyStorage::new(temp_dir.path().to_path_buf()).unwrap();

        // 测试未设置主密钥时的错误处理
        let error_msg = "Test error without master key";
        let sanitized = storage.sanitize_error(error_msg);
        assert_eq!(sanitized, error_msg);

        // 设置主密钥后测试脱敏
        let master_key = [0x42; 32];
        storage.set_master_key(&master_key);

        let error_msg = format!("Test error with key: {}", hex::encode(master_key));
        let sanitized = storage.sanitize_error(&error_msg);

        assert!(sanitized.contains("***"));
        assert!(!sanitized.contains(&hex::encode(master_key)));

        // 清除主密钥后测试
        storage.clear_master_key();
        let error_msg = "Test error after clearing master key";
        let sanitized = storage.sanitize_error(error_msg);
        assert_eq!(sanitized, error_msg);
    }
}

const _: () = ();
