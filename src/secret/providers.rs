//! Additional key providers for secret management.

use std::path::PathBuf;

use crate::error::{ConfigError, ConfigResult};
use crate::traits::{KeyCachePolicy, KeyProvider, ZeroizingBytes};

#[cfg(feature = "remote")]
use crate::traits::AsyncKeyProvider;

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

        let key_str = String::from_utf8(content)
            .map_err(|_| ConfigError::InvalidValue {
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
    pub fn new(vault_addr: impl Into<String>, secret_path: impl Into<String>, secret_key: impl Into<String>) -> Self {
        Self {
            vault_addr: vault_addr.into(),
            secret_path: secret_path.into(),
            secret_key: secret_key.into(),
            token: None,
            cache_policy: KeyCachePolicy::default(),
        }
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
        let url = format!("{}/v1/{}", self.vault_addr.trim_end_matches('/'), self.secret_path);
        
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

        let json: serde_json::Value = response.json().await.map_err(|e| ConfigError::ParseError {
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
        temp_file.write_all(b"this-is-a-test-key-with-32-chars-minimum").unwrap();
        
        let provider = FileKeyProvider::new(temp_file.path());
        let key = provider.get_key().unwrap();
        
        assert_eq!(key.len(), 32);
        assert_eq!(provider.provider_type(), "file");
    }

    #[test]
    fn test_file_key_provider_builder() {
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"this-is-a-test-key-with-32-chars-minimum").unwrap();
        
        let provider = FileKeyProvider::builder()
            .path(temp_file.path())
            .cache_policy(KeyCachePolicy::Never)
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
}
