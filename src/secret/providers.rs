// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Additional key providers for secret management.

use std::path::PathBuf;

use crate::error::{ConfigError, ConfigResult};
use crate::interface::KeyProvider;
use crate::types::{KeyCachePolicy, ZeroizingBytes};

#[cfg(feature = "remote")]
use crate::interface::AsyncKeyProvider;

pub struct FileKeyProvider {
    path: PathBuf,
    cache_policy: KeyCachePolicy,
}

impl FileKeyProvider {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            cache_policy: KeyCachePolicy::default(),
        }
    }

    pub fn builder() -> FileKeyProviderBuilder {
        FileKeyProviderBuilder::new()
    }

    pub fn with_cache_policy(mut self, policy: KeyCachePolicy) -> Self {
        self.cache_policy = policy;
        self
    }

    fn read_key_from_file(&self) -> ConfigResult<Vec<u8>> {
        let content = std::fs::read(&self.path).map_err(|e| ConfigError::FileNotFound {
            filename: self.path.clone(),
            source: Some(e),
        })?;

        let key_str = String::from_utf8(content).map_err(|_| ConfigError::InvalidValue {
            key: self.path.to_string_lossy().to_string(),
            expected_type: "utf8 string".to_string(),
            message: "Key file contains non-UTF8 content".to_string(),
        })?;

        let key_str = key_str.trim();

        if key_str.len() < 32 {
            return Err(ConfigError::KeyError {
                message: "Key file must contain at least 32 characters".to_string(),
            });
        }

        Ok(key_str.as_bytes()[..32].to_vec())
    }
}

impl KeyProvider for FileKeyProvider {
    fn get_key(&self) -> ConfigResult<ZeroizingBytes> {
        let key = self.read_key_from_file()?;
        Ok(ZeroizingBytes::new(key))
    }

    fn provider_type(&self) -> &'static str {
        "file"
    }

    fn cache_policy(&self) -> KeyCachePolicy {
        self.cache_policy
    }
}

pub struct FileKeyProviderBuilder {
    path: Option<PathBuf>,
    cache_policy: KeyCachePolicy,
}

impl FileKeyProviderBuilder {
    pub fn new() -> Self {
        Self {
            path: None,
            cache_policy: KeyCachePolicy::default(),
        }
    }

    pub fn path(mut self, path: impl Into<PathBuf>) -> Self {
        self.path = Some(path.into());
        self
    }

    pub fn cache_policy(mut self, policy: KeyCachePolicy) -> Self {
        self.cache_policy = policy;
        self
    }

    pub fn build(self) -> ConfigResult<FileKeyProvider> {
        let path = self.path.ok_or(ConfigError::InvalidValue {
            key: "file_key_provider_path".to_string(),
            expected_type: "PathBuf".to_string(),
            message: "Path is required for FileKeyProvider".to_string(),
        })?;

        Ok(FileKeyProvider {
            path,
            cache_policy: self.cache_policy,
        })
    }
}

impl Default for FileKeyProviderBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "remote")]
pub struct VaultKeyProvider {
    vault_addr: String,
    secret_path: String,
    secret_key: String,
    token: Option<String>,
    cache_policy: KeyCachePolicy,
}

#[cfg(feature = "remote")]
impl VaultKeyProvider {
    pub fn new(
        vault_addr: impl Into<String>,
        secret_path: impl Into<String>,
        secret_key: impl Into<String>,
    ) -> ConfigResult<Self> {
        let addr = vault_addr.into();

        if !addr.starts_with("https://") {
            return Err(ConfigError::KeyError {
                message: "Vault address must use HTTPS for security".to_string(),
            });
        }

        Ok(Self {
            vault_addr: addr,
            secret_path: secret_path.into(),
            secret_key: secret_key.into(),
            token: None,
            cache_policy: KeyCachePolicy::default(),
        })
    }

    pub fn builder() -> VaultKeyProviderBuilder {
        VaultKeyProviderBuilder::new()
    }

    pub fn with_token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(token.into());
        self
    }

    pub fn with_cache_policy(mut self, policy: KeyCachePolicy) -> Self {
        self.cache_policy = policy;
        self
    }

    fn get_token(&self) -> ConfigResult<String> {
        if let Some(ref token) = self.token {
            return Ok(token.clone());
        }
        std::env::var("VAULT_TOKEN").map_err(|_| ConfigError::KeyError {
            message: "Vault token not provided".to_string(),
        })
    }
}

#[cfg(feature = "remote")]
#[async_trait::async_trait]
impl AsyncKeyProvider for VaultKeyProvider {
    async fn get_key(&self) -> ConfigResult<ZeroizingBytes> {
        let token = self.get_token()?;

        let client = reqwest::Client::new();
        let url = format!(
            "{}/v1/{}",
            self.vault_addr.trim_end_matches('/'),
            self.secret_path
        );

        let response = client
            .get(&url)
            .header("X-Vault-Token", token)
            .send()
            .await
            .map_err(|e| ConfigError::RemoteUnavailable {
                error_type: format!("vault_request: {}", e),
                retryable: true,
            })?;

        if !response.status().is_success() {
            return Err(ConfigError::RemoteUnavailable {
                error_type: format!("vault_response: {}", response.status()),
                retryable: false,
            });
        }

        let json: serde_json::Value =
            response.json().await.map_err(|e| ConfigError::ParseError {
                format: "json".to_string(),
                message: format!("Failed to parse Vault response: {}", e),
                location: None,
                source: None,
            })?;

        let key_value = json
            .get("data")
            .and_then(|d| d.get(&self.secret_key))
            .and_then(|v| v.as_str())
            .ok_or(ConfigError::KeyError {
                message: format!("Key '{}' not found in Vault secret", self.secret_key),
            })?;

        if key_value.len() < 32 {
            return Err(ConfigError::KeyError {
                message: "Vault key must be at least 32 characters".to_string(),
            });
        }

        Ok(ZeroizingBytes::new(key_value.as_bytes()[..32].to_vec()))
    }

    fn provider_type(&self) -> &'static str {
        "vault"
    }

    fn cache_policy(&self) -> KeyCachePolicy {
        self.cache_policy
    }
}

#[cfg(feature = "remote")]
pub struct VaultKeyProviderBuilder {
    vault_addr: Option<String>,
    secret_path: Option<String>,
    secret_key: Option<String>,
    token: Option<String>,
    cache_policy: KeyCachePolicy,
}

#[cfg(feature = "remote")]
impl VaultKeyProviderBuilder {
    pub fn new() -> Self {
        Self {
            vault_addr: None,
            secret_path: None,
            secret_key: None,
            token: None,
            cache_policy: KeyCachePolicy::default(),
        }
    }

    pub fn vault_addr(mut self, addr: impl Into<String>) -> Self {
        self.vault_addr = Some(addr.into());
        self
    }

    pub fn secret_path(mut self, path: impl Into<String>) -> Self {
        self.secret_path = Some(path.into());
        self
    }

    pub fn secret_key(mut self, key: impl Into<String>) -> Self {
        self.secret_key = Some(key.into());
        self
    }

    pub fn token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(token.into());
        self
    }

    pub fn cache_policy(mut self, policy: KeyCachePolicy) -> Self {
        self.cache_policy = policy;
        self
    }

    pub fn build(self) -> ConfigResult<VaultKeyProvider> {
        let vault_addr = self.vault_addr.ok_or(ConfigError::InvalidValue {
            key: "vault_addr".to_string(),
            expected_type: "string".to_string(),
            message: "Vault address is required".to_string(),
        })?;

        if !vault_addr.starts_with("https://") {
            return Err(ConfigError::KeyError {
                message: "Vault address must use HTTPS for security".to_string(),
            });
        }

        let secret_path = self.secret_path.ok_or(ConfigError::InvalidValue {
            key: "secret_path".to_string(),
            expected_type: "string".to_string(),
            message: "Secret path is required".to_string(),
        })?;

        let secret_key = self.secret_key.ok_or(ConfigError::InvalidValue {
            key: "secret_key".to_string(),
            expected_type: "string".to_string(),
            message: "Secret key is required".to_string(),
        })?;

        Ok(VaultKeyProvider {
            vault_addr,
            secret_path,
            secret_key,
            token: self.token,
            cache_policy: self.cache_policy,
        })
    }
}

#[cfg(feature = "remote")]
impl Default for VaultKeyProviderBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_file_key_provider() {
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file
            .write_all(b"this-is-a-test-key-with-32-chars-minimum")
            .unwrap();

        let provider = FileKeyProvider::new(temp_file.path());
        let key = provider.get_key().unwrap();

        assert_eq!(key.len(), 32);
        assert_eq!(provider.provider_type(), "file");
    }

    #[test]
    fn test_file_key_provider_builder() {
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file
            .write_all(b"this-is-a-test-key-with-32-chars-minimum")
            .unwrap();

        let provider = FileKeyProvider::builder()
            .path(temp_file.path())
            .cache_policy(KeyCachePolicy::NoCache)
            .build()
            .unwrap();

        let key = provider.get_key().unwrap();
        assert_eq!(key.len(), 32);
    }

    #[test]
    fn test_file_key_provider_short_key() {
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"short").unwrap();

        let provider = FileKeyProvider::new(temp_file.path());
        let result = provider.get_key();

        assert!(result.is_err());
    }

    #[test]
    fn test_file_key_provider_whitespace_trim() {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(b"  this-is-a-test-key-with-32-chars-min  \n")
            .unwrap();
        let p = FileKeyProvider::new(f.path());
        let key = p.get_key().unwrap();
        assert_eq!(key.len(), 32);
    }

    #[test]
    fn test_file_key_provider_builder_no_path() {
        let result = FileKeyProvider::builder().build();
        assert!(result.is_err());
    }

    #[test]
    fn test_file_key_provider_builder_cache_policy() {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(b"test-key-12345678901234567890").unwrap();
        let p = FileKeyProvider::builder()
            .path(f.path())
            .cache_policy(KeyCachePolicy::NoCache)
            .build()
            .unwrap();
        assert_eq!(p.provider_type(), "file");
    }

    #[test]
    fn test_file_key_provider_file_not_found() {
        let provider = FileKeyProvider::new("/nonexistent/path/does-not-exist-key.txt");
        let result = provider.get_key();
        assert!(result.is_err());
    }

    #[test]
    fn test_file_key_provider_non_utf8_content() {
        let mut f = NamedTempFile::new().unwrap();
        // Invalid UTF-8 bytes — from_utf8 conversion must fail before length check.
        f.write_all(&[0xFF, 0xFE, 0xFD, 0xFC, 0xFB, 0xFA]).unwrap();
        let provider = FileKeyProvider::new(f.path());
        let result = provider.get_key();
        assert!(result.is_err());
    }

    #[test]
    fn test_file_key_provider_empty_file() {
        let f = NamedTempFile::new().unwrap();
        let provider = FileKeyProvider::new(f.path());
        // Empty content → trimmed length 0 < 32 → KeyError
        let result = provider.get_key();
        assert!(result.is_err());
    }

    #[test]
    fn test_file_key_provider_with_cache_policy() {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(b"this-is-a-test-key-with-32-chars-minimum")
            .unwrap();
        let provider = FileKeyProvider::new(f.path()).with_cache_policy(KeyCachePolicy::NoCache);
        assert_eq!(provider.cache_policy(), KeyCachePolicy::NoCache);
    }

    #[test]
    fn test_file_key_provider_default_cache_policy() {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(b"this-is-a-test-key-with-32-chars-minimum")
            .unwrap();
        let provider = FileKeyProvider::new(f.path());
        assert_eq!(
            provider.cache_policy(),
            KeyCachePolicy::CacheWithTtl(std::time::Duration::from_secs(3600))
        );
    }

    #[test]
    fn test_file_key_provider_cache_indefinitely_policy() {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(b"this-is-a-test-key-with-32-chars-minimum")
            .unwrap();
        let provider =
            FileKeyProvider::new(f.path()).with_cache_policy(KeyCachePolicy::CacheIndefinitely);
        assert_eq!(provider.cache_policy(), KeyCachePolicy::CacheIndefinitely);
    }

    #[test]
    fn test_file_key_provider_extracts_exactly_32_bytes() {
        let mut f = NamedTempFile::new().unwrap();
        let key = b"0123456789abcdef0123456789abcdefEXTRA_TRAILING_DATA"; // pragma: allowlist secret
        f.write_all(key).unwrap();
        let provider = FileKeyProvider::new(f.path());
        let result = provider.get_key().unwrap();
        assert_eq!(result.len(), 32);
        assert_eq!(&*result, &key[..32]);
    }

    #[test]
    fn test_file_key_provider_exactly_32_chars_boundary() {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(b"0123456789abcdef0123456789abcdef").unwrap(); // pragma: allowlist secret
        let provider = FileKeyProvider::new(f.path());
        let result = provider.get_key();
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 32);
    }

    #[test]
    fn test_file_key_provider_builder_default_impl() {
        let builder = FileKeyProviderBuilder::default();
        // Default builder has no path set — build must fail.
        assert!(builder.build().is_err());
    }

    #[test]
    fn test_file_key_provider_as_trait_object() {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(b"this-is-a-test-key-with-32-chars-minimum")
            .unwrap();
        let provider = FileKeyProvider::new(f.path());
        let provider_ref: &dyn KeyProvider = &provider;
        assert_eq!(provider_ref.provider_type(), "file");
        let key = provider_ref.get_key().unwrap();
        assert_eq!(key.len(), 32);
    }

    #[test]
    fn test_file_key_provider_builder_with_cache_indefinitely() {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(b"this-is-a-test-key-with-32-chars-minimum")
            .unwrap();
        let provider = FileKeyProvider::builder()
            .path(f.path())
            .cache_policy(KeyCachePolicy::CacheIndefinitely)
            .build()
            .unwrap();
        assert_eq!(provider.cache_policy(), KeyCachePolicy::CacheIndefinitely);
    }

    // ===== VaultKeyProvider tests (only when `remote` feature is enabled) =====

    #[cfg(feature = "remote")]
    #[test]
    fn test_vault_key_provider_new_https() {
        let provider =
            VaultKeyProvider::new("https://vault.example.com", "secret/data/path", "my_key")
                .unwrap();
        assert_eq!(provider.provider_type(), "vault");
    }

    #[cfg(feature = "remote")]
    #[test]
    fn test_vault_key_provider_new_non_https() {
        let result = VaultKeyProvider::new("http://vault.example.com", "secret/path", "key");
        assert!(result.is_err());
    }

    #[cfg(feature = "remote")]
    #[test]
    fn test_vault_key_provider_with_token() {
        let provider = VaultKeyProvider::new("https://vault.example.com", "secret/path", "key")
            .unwrap()
            .with_token("my-token");
        assert_eq!(provider.provider_type(), "vault");
    }

    #[cfg(feature = "remote")]
    #[test]
    fn test_vault_key_provider_with_cache_policy() {
        let provider = VaultKeyProvider::new("https://vault.example.com", "secret/path", "key")
            .unwrap()
            .with_cache_policy(KeyCachePolicy::NoCache);
        assert_eq!(provider.cache_policy(), KeyCachePolicy::NoCache);
    }

    #[cfg(feature = "remote")]
    #[test]
    fn test_vault_key_provider_cache_policy_default() {
        let provider =
            VaultKeyProvider::new("https://vault.example.com", "secret/path", "key").unwrap();
        assert_eq!(
            provider.cache_policy(),
            KeyCachePolicy::CacheWithTtl(std::time::Duration::from_secs(3600))
        );
    }

    #[cfg(feature = "remote")]
    #[test]
    fn test_vault_key_provider_builder_default_impl() {
        let builder = VaultKeyProviderBuilder::default();
        assert!(builder.build().is_err());
    }

    #[cfg(feature = "remote")]
    #[test]
    fn test_vault_key_provider_builder_success() {
        let provider = VaultKeyProvider::builder()
            .vault_addr("https://vault.example.com")
            .secret_path("secret/data/path")
            .secret_key("my_key")
            .token("my-token")
            .cache_policy(KeyCachePolicy::NoCache)
            .build()
            .unwrap();
        assert_eq!(provider.provider_type(), "vault");
        assert_eq!(provider.cache_policy(), KeyCachePolicy::NoCache);
    }

    #[cfg(feature = "remote")]
    #[test]
    fn test_vault_key_provider_builder_no_vault_addr() {
        let result = VaultKeyProvider::builder()
            .secret_path("secret/path")
            .secret_key("key")
            .build();
        assert!(result.is_err());
    }

    #[cfg(feature = "remote")]
    #[test]
    fn test_vault_key_provider_builder_non_https() {
        let result = VaultKeyProvider::builder()
            .vault_addr("http://vault.example.com")
            .secret_path("secret/path")
            .secret_key("key")
            .build();
        assert!(result.is_err());
    }

    #[cfg(feature = "remote")]
    #[test]
    fn test_vault_key_provider_builder_no_secret_path() {
        let result = VaultKeyProvider::builder()
            .vault_addr("https://vault.example.com")
            .secret_key("key")
            .build();
        assert!(result.is_err());
    }

    #[cfg(feature = "remote")]
    #[test]
    fn test_vault_key_provider_builder_no_secret_key() {
        let result = VaultKeyProvider::builder()
            .vault_addr("https://vault.example.com")
            .secret_path("secret/path")
            .build();
        assert!(result.is_err());
    }
}
