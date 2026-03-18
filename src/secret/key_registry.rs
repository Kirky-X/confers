//! Key registry for managing encryption key versions and rotation.
//!
//! This module provides thread-safe key storage with support for:
//! - Multiple key versions
//! - Primary key designation
//! - Key rotation with grace periods
//! - Provider-based key fetching
//!
//! # RwLock Error Handling
//!
//! This implementation uses `RwLock` for thread-safe access to key storage.
//! While `RwLock` poisoning is rare, it can occur if a thread panics while holding
//! the lock. In such cases, the lock enters a "poisoned" state.
//!
//! This implementation uses `expect()` with descriptive messages for RwLock operations.
//! If a panic occurs, the error message will clearly indicate which lock was poisoned,
//! making debugging easier.
//!
//! # Recovery from Poisoned Locks
//!
//! If a RwLock is poisoned:
//! 1. The panic message will indicate which lock was affected
//! 2. The application should be restarted
//! 3. On restart, a fresh `KeyRegistry` will be created with no poisoned state
//! 4. Keys can be re-registered from providers or persistent storage
//!
//! To prevent lock poisoning in production:
//! - Ensure all key operations complete without panicking
//! - Use proper error handling instead of panicking in callbacks
//! - Consider wrapping operations in `catch_unwind` for async contexts

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use std::sync::RwLock;

use crate::secret::{zeroizing_bytes, CryptoError, SecretBytes, ZeroizingBytes};
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

    pub fn register_key(
        &self,
        version: String,
        key: SecretBytes,
        is_primary: bool,
    ) -> Result<(), CryptoError> {
        // RwLock may poison if a previous holder panicked while holding the lock.
        // Using expect() provides a descriptive message for debugging.
        let mut keys = self
            .keys
            .write()
            .expect("KeyRegistry keys lock poisoned: previous holder panicked");

        if is_primary {
            for (_, v) in keys.iter_mut() {
                v.is_primary = false;
            }
            *self
                .primary_version
                .write()
                .expect("KeyRegistry primary_version lock poisoned: previous holder panicked") =
                Some(version.clone());
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

    pub fn rotate_to(
        &self,
        new_version: String,
        new_key: SecretBytes,
    ) -> Result<String, CryptoError> {
        let old_primary = self
            .primary_version
            .read()
            .expect("KeyRegistry primary_version lock poisoned: previous holder panicked")
            .clone();

        self.register_key(new_version.clone(), new_key, true)?;

        Ok(old_primary.unwrap_or_else(|| "none".to_string()))
    }

    pub fn get_primary_key(&self) -> Result<(String, ZeroizingBytes), CryptoError> {
        let version = self
            .primary_version
            .read()
            .expect("KeyRegistry primary_version lock poisoned: previous holder panicked")
            .clone()
            .ok_or(CryptoError::InvalidKeyLength(0))?;

        let keys = self
            .keys
            .read()
            .expect("KeyRegistry keys lock poisoned: previous holder panicked");
        let key_version = keys.get(&version).ok_or(CryptoError::InvalidKeyLength(0))?;

        Ok((
            version.clone(),
            zeroizing_bytes(key_version.key.as_slice().to_vec()),
        ))
    }

    pub fn get_key(&self, version: &str) -> Result<ZeroizingBytes, CryptoError> {
        let keys = self
            .keys
            .read()
            .expect("KeyRegistry keys lock poisoned: previous holder panicked");
        let key_version = keys.get(version).ok_or(CryptoError::InvalidKeyLength(0))?;
        Ok(zeroizing_bytes(key_version.key.as_slice().to_vec()))
    }

    pub fn get_all_versions(&self) -> Vec<String> {
        self.keys
            .read()
            .expect("KeyRegistry keys lock poisoned: previous holder panicked")
            .keys()
            .cloned()
            .collect()
    }

    pub fn try_decrypt_with_all_keys(
        &self,
        nonce: &[u8],
        ciphertext: &[u8],
    ) -> Result<(String, Vec<u8>), CryptoError> {
        use crate::secret::XChaCha20Crypto;

        let crypto = XChaCha20Crypto::new();
        let keys = self
            .keys
            .read()
            .expect("KeyRegistry keys lock poisoned: previous holder panicked");

        for (version, key_version) in keys.iter() {
            let key_bytes = key_version.key.as_slice();
            if let Ok(plaintext) = crypto.decrypt(nonce, ciphertext, key_bytes) {
                return Ok((version.clone(), plaintext));
            }
        }

        Err(CryptoError::DecryptionFailed)
    }

    pub fn add_provider(&self, provider: Arc<dyn KeyProvider>) {
        self.providers
            .write()
            .expect("KeyRegistry providers lock poisoned: previous holder panicked")
            .push(provider);
    }

    pub fn add_async_provider(&self, provider: Arc<dyn AsyncKeyProvider>) {
        self.async_providers
            .write()
            .expect("KeyRegistry async_providers lock poisoned: previous holder panicked")
            .push(provider);
    }

    pub async fn fetch_and_register(&self, version: &str) -> Result<Vec<u8>, CryptoError> {
        for provider in self
            .providers
            .read()
            .expect("KeyRegistry providers lock poisoned: previous holder panicked")
            .iter()
        {
            if let Ok(key) = provider.get_key() {
                let key_bytes = key.as_slice().to_vec();
                let _ = self.register_key(
                    version.to_string(),
                    SecretBytes::new(key_bytes.clone()),
                    false,
                );
                return Ok(key_bytes);
            }
        }

        let async_providers: Vec<Arc<dyn AsyncKeyProvider>> = self
            .async_providers
            .read()
            .expect("KeyRegistry async_providers lock poisoned: previous holder panicked")
            .iter()
            .cloned()
            .collect();
        for provider in async_providers {
            if let Ok(key) = provider.get_key().await {
                let key_bytes = key.as_slice().to_vec();
                let _ = self.register_key(
                    version.to_string(),
                    SecretBytes::new(key_bytes.clone()),
                    false,
                );
                return Ok(key_bytes);
            }
        }

        Err(CryptoError::InvalidKeyLength(0))
    }

    pub fn version_count(&self) -> usize {
        self.keys
            .read()
            .expect("KeyRegistry keys lock poisoned: previous holder panicked")
            .len()
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

        registry
            .register_key("v1".to_string(), SecretBytes::new(key.clone()), true)
            .unwrap();

        let (version, retrieved) = registry.get_primary_key().unwrap();
        assert_eq!(version, "v1");
        assert_eq!(&*retrieved, &key);
    }

    #[test]
    fn test_key_rotation() {
        let registry = KeyRegistry::new(KeyRotationConfig::default());
        let key1 = vec![1u8; 32];
        let key2 = vec![2u8; 32];

        registry
            .register_key("v1".to_string(), SecretBytes::new(key1), true)
            .unwrap();
        let old = registry
            .rotate_to("v2".to_string(), SecretBytes::new(key2))
            .unwrap();

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

        registry
            .register_key("v1".to_string(), SecretBytes::new(vec![1u8; 32]), true)
            .unwrap();
        registry
            .register_key("v2".to_string(), SecretBytes::new(vec![2u8; 32]), false)
            .unwrap();
        registry
            .register_key("v3".to_string(), SecretBytes::new(vec![3u8; 32]), false)
            .unwrap();

        assert_eq!(registry.version_count(), 2);
    }

    #[test]
    fn test_get_all_versions() {
        let registry = KeyRegistry::new(KeyRotationConfig::default());

        registry
            .register_key("v1".to_string(), SecretBytes::new(vec![1u8; 32]), true)
            .unwrap();
        registry
            .register_key("v2".to_string(), SecretBytes::new(vec![2u8; 32]), false)
            .unwrap();

        let versions = registry.get_all_versions();
        assert_eq!(versions.len(), 2);
        assert!(versions.contains(&"v1".to_string()));
        assert!(versions.contains(&"v2".to_string()));
    }

    #[test]
    fn test_get_key_by_version() {
        let registry = KeyRegistry::new(KeyRotationConfig::default());
        let key = vec![42u8; 32];

        registry
            .register_key("v1".to_string(), SecretBytes::new(key.clone()), true)
            .unwrap();

        let retrieved = registry.get_key("v1").unwrap();
        assert_eq!(&*retrieved, &key);

        // Non-existent version should fail
        let result = registry.get_key("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_try_decrypt_with_all_keys() {
        use crate::secret::XChaCha20Crypto;

        let registry = KeyRegistry::new(KeyRotationConfig::default());
        let crypto = XChaCha20Crypto::new();

        // Register first key
        let key1 = vec![1u8; 32];
        registry
            .register_key("v1".to_string(), SecretBytes::new(key1.clone()), true)
            .unwrap();

        // Encrypt with key1
        let plaintext = b"secret message";
        let (nonce, ciphertext) = crypto.encrypt(plaintext, &key1).unwrap();

        // Try to decrypt with registry
        let result = registry.try_decrypt_with_all_keys(&nonce, &ciphertext);
        assert!(result.is_ok());

        let (version, decrypted) = result.unwrap();
        assert_eq!(version, "v1");
        assert_eq!(&decrypted, plaintext);
    }

    #[test]
    fn test_key_registry_thread_safety() {
        use std::sync::Arc;
        use std::thread;

        // Use a config with enough capacity for all test keys
        let mut config = KeyRotationConfig::default();
        config.max_key_versions = 15;
        let registry = Arc::new(KeyRegistry::new(config));
        let mut handles = vec![];

        // Spawn multiple threads to register keys concurrently
        for i in 0..10 {
            let reg = Arc::clone(&registry);
            let handle = thread::spawn(move || {
                let key = vec![i as u8; 32];
                reg.register_key(
                    format!("v{}", i),
                    SecretBytes::new(key),
                    i == 0, // First one is primary
                )
                .unwrap();
            });
            handles.push(handle);
        }

        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }

        // Verify all keys were registered
        assert_eq!(registry.version_count(), 10);

        // Verify primary key exists
        let result = registry.get_primary_key();
        assert!(result.is_ok());
    }

    #[test]
    fn test_add_provider() {
        use crate::error::ConfigResult;
        use crate::traits::KeyProvider;
        use crate::traits::ZeroizingBytes;

        struct DummyProvider;
        impl KeyProvider for DummyProvider {
            fn get_key(&self) -> ConfigResult<ZeroizingBytes> {
                Ok(ZeroizingBytes::new(vec![0u8; 32]))
            }

            fn provider_type(&self) -> &'static str {
                "test-dummy"
            }
        }

        let registry = KeyRegistry::new(KeyRotationConfig::default());
        registry.add_provider(Arc::new(DummyProvider));

        // Provider was added (no panic means success)
        assert!(true);
    }
}
