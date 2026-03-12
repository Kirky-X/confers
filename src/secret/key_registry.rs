use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use std::sync::RwLock;

use crate::secret::{CryptoError, SecretBytes};
use crate::traits::{AsyncKeyProvider, KeyProvider};

#[derive(Debug)]
pub struct KeyVersion {
    pub version: String,
    pub key: SecretBytes,
    pub created_at: Instant,
    pub is_primary: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyCachePolicy {
    NoCache,
    CacheWithTtl(Duration),
    CacheIndefinitely,
}

impl Default for KeyCachePolicy {
    fn default() -> Self {
        KeyCachePolicy::CacheWithTtl(Duration::from_secs(3600))
    }
}

#[derive(Debug, Clone)]
pub struct KeyRotationConfig {
    pub max_key_versions: usize,
    pub cache_policy: KeyCachePolicy,
    pub rotation_grace_period: Duration,
}

impl Default for KeyRotationConfig {
    fn default() -> Self {
        Self {
            max_key_versions: 3,
            cache_policy: KeyCachePolicy::default(),
            rotation_grace_period: Duration::from_secs(300),
        }
    }
}

pub struct KeyRegistry {
    keys: RwLock<BTreeMap<String, KeyVersion>>,
    primary_version: RwLock<Option<String>>,
    config: KeyRotationConfig,
    providers: RwLock<Vec<Arc<dyn KeyProvider>>>,
    async_providers: RwLock<Vec<Arc<dyn AsyncKeyProvider>>>,
}

impl KeyRegistry {
    pub fn new(config: KeyRotationConfig) -> Self {
        Self {
            keys: RwLock::new(BTreeMap::new()),
            primary_version: RwLock::new(None),
            config,
            providers: RwLock::new(Vec::new()),
            async_providers: RwLock::new(Vec::new()),
        }
    }

    pub fn builder() -> KeyRegistryBuilder {
        KeyRegistryBuilder::new()
    }

    pub fn register_key(&self, version: String, key: SecretBytes, is_primary: bool) -> Result<(), CryptoError> {
        let mut keys = self.keys.write().unwrap();
        
        if is_primary {
            for (_, v) in keys.iter_mut() {
                v.is_primary = false;
            }
            *self.primary_version.write().unwrap() = Some(version.clone());
        }

        keys.insert(
            version.clone(),
            KeyVersion {
                version,
                key,
                created_at: Instant::now(),
                is_primary,
            },
        );

        if keys.len() > self.config.max_key_versions {
            let oldest = keys
                .iter()
                .filter(|(_, v)| !v.is_primary)
                .min_by_key(|(_, v)| v.created_at)
                .map(|(k, _)| k.clone());

            if let Some(k) = oldest {
                keys.remove(&k);
            }
        }

        Ok(())
    }

    pub fn rotate_to(&self, new_version: String, new_key: SecretBytes) -> Result<String, CryptoError> {
        let old_primary = self.primary_version.read().unwrap().clone();
        
        self.register_key(new_version.clone(), new_key, true)?;

        Ok(old_primary.unwrap_or_else(|| "none".to_string()))
    }

    pub fn get_primary_key(&self) -> Result<(String, Vec<u8>), CryptoError> {
        let version = self
            .primary_version
            .read()
            .unwrap()
            .clone()
            .ok_or(CryptoError::InvalidKeyLength)?;

        let keys = self.keys.read().unwrap();
        let key_version = keys.get(&version).ok_or(CryptoError::InvalidKeyLength)?;

        Ok((version.clone(), key_version.key.as_slice().to_vec()))
    }

    pub fn get_key(&self, version: &str) -> Result<Vec<u8>, CryptoError> {
        let keys = self.keys.read().unwrap();
        let key_version = keys.get(version).ok_or(CryptoError::InvalidKeyLength)?;
        Ok(key_version.key.as_slice().to_vec())
    }

    pub fn get_all_versions(&self) -> Vec<String> {
        self.keys.read().unwrap().keys().cloned().collect()
    }

    pub fn try_decrypt_with_all_keys(
        &self,
        nonce: &[u8],
        ciphertext: &[u8],
    ) -> Result<(String, Vec<u8>), CryptoError> {
        use crate::secret::XChaCha20Crypto;
        
        let crypto = XChaCha20Crypto::new();
        let keys = self.keys.read().unwrap();

        for (version, key_version) in keys.iter() {
            let key_bytes = key_version.key.as_slice();
            if let Ok(plaintext) = crypto.decrypt(nonce, ciphertext, key_bytes) {
                return Ok((version.clone(), plaintext));
            }
        }

        Err(CryptoError::DecryptionFailed)
    }

    pub fn add_provider(&self, provider: Arc<dyn KeyProvider>) {
        self.providers.write().unwrap().push(provider);
    }

    pub fn add_async_provider(&self, provider: Arc<dyn AsyncKeyProvider>) {
        self.async_providers.write().unwrap().push(provider);
    }

    pub async fn fetch_and_register(&self, version: &str) -> Result<Vec<u8>, CryptoError> {
        for provider in self.providers.read().unwrap().iter() {
            if let Ok(key) = provider.get_key() {
                let key_bytes = key.as_slice().to_vec();
                let _ = self.register_key(version.to_string(), SecretBytes::new(key_bytes.clone()), false);
                return Ok(key_bytes);
            }
        }

        for provider in self.async_providers.read().unwrap().iter() {
            if let Ok(key) = provider.get_key().await {
                let key_bytes = key.as_slice().to_vec();
                let _ = self.register_key(version.to_string(), SecretBytes::new(key_bytes.clone()), false);
                return Ok(key_bytes);
            }
        }

        Err(CryptoError::InvalidKeyLength)
    }

    pub fn version_count(&self) -> usize {
        self.keys.read().unwrap().len()
    }
}

pub struct KeyRegistryBuilder {
    config: KeyRotationConfig,
    providers: Vec<Arc<dyn KeyProvider>>,
    async_providers: Vec<Arc<dyn AsyncKeyProvider>>,
    initial_keys: Vec<(String, Vec<u8>, bool)>,
}

impl KeyRegistryBuilder {
    pub fn new() -> Self {
        Self {
            config: KeyRotationConfig::default(),
            providers: Vec::new(),
            async_providers: Vec::new(),
            initial_keys: Vec::new(),
        }
    }

    pub fn config(mut self, config: KeyRotationConfig) -> Self {
        self.config = config;
        self
    }

    pub fn max_versions(mut self, max: usize) -> Self {
        self.config.max_key_versions = max;
        self
    }

    pub fn cache_policy(mut self, policy: KeyCachePolicy) -> Self {
        self.config.cache_policy = policy;
        self
    }

    pub fn provider(mut self, provider: Arc<dyn KeyProvider>) -> Self {
        self.providers.push(provider);
        self
    }

    pub fn async_provider(mut self, provider: Arc<dyn AsyncKeyProvider>) -> Self {
        self.async_providers.push(provider);
        self
    }

    pub fn initial_key(mut self, version: String, key: Vec<u8>, is_primary: bool) -> Self {
        self.initial_keys.push((version, key, is_primary));
        self
    }

    pub fn build(self) -> KeyRegistry {
        let registry = KeyRegistry::new(self.config);

        for (version, key, is_primary) in self.initial_keys {
            let _ = registry.register_key(version, SecretBytes::new(key), is_primary);
        }

        for provider in self.providers {
            registry.add_provider(provider);
        }

        for provider in self.async_providers {
            registry.add_async_provider(provider);
        }

        registry
    }
}

impl Default for KeyRegistryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_and_get_key() {
        let registry = KeyRegistry::new(KeyRotationConfig::default());
        let key = vec![1u8; 32];
        
        registry.register_key("v1".to_string(), SecretBytes::new(key.clone()), true).unwrap();
        
        let (version, retrieved) = registry.get_primary_key().unwrap();
        assert_eq!(version, "v1");
        assert_eq!(retrieved, key);
    }

    #[test]
    fn test_key_rotation() {
        let registry = KeyRegistry::new(KeyRotationConfig::default());
        let key1 = vec![1u8; 32];
        let key2 = vec![2u8; 32];
        
        registry.register_key("v1".to_string(), SecretBytes::new(key1), true).unwrap();
        let old = registry.rotate_to("v2".to_string(), SecretBytes::new(key2)).unwrap();
        
        assert_eq!(old, "v1");
        let (version, _) = registry.get_primary_key().unwrap();
        assert_eq!(version, "v2");
    }

    #[test]
    fn test_max_versions() {
        let config = KeyRotationConfig {
            max_key_versions: 2,
            ..Default::default()
        };
        let registry = KeyRegistry::new(config);
        
        registry.register_key("v1".to_string(), SecretBytes::new(vec![1u8; 32]), true).unwrap();
        registry.register_key("v2".to_string(), SecretBytes::new(vec![2u8; 32]), false).unwrap();
        registry.register_key("v3".to_string(), SecretBytes::new(vec![3u8; 32]), false).unwrap();
        
        assert_eq!(registry.version_count(), 2);
    }
}
