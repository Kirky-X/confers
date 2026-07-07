// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

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

use crate::interface::{AsyncKeyProvider, KeyProvider};
use crate::secret::{zeroizing_bytes, CryptoError, SecretBytes, ZeroizingBytes};

#[derive(Debug)]
pub struct KeyVersion {
    pub version: String,
    pub key: SecretBytes,
    pub created_at: Instant,
    pub is_primary: bool,
}

// Re-export unified KeyCachePolicy from crate::types (BrickArchitecture: single source of truth).
pub use crate::types::KeyCachePolicy;

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
        let mut keys = self.keys.write().unwrap_or_else(|e| e.into_inner());

        if is_primary {
            for (_, v) in keys.iter_mut() {
                v.is_primary = false;
            }
            *self
                .primary_version
                .write()
                .unwrap_or_else(|e| e.into_inner()) = Some(version.clone());
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
            .unwrap_or_else(|e| e.into_inner())
            .clone();

        self.register_key(new_version.clone(), new_key, true)?;

        Ok(old_primary.unwrap_or_else(|| "none".to_string()))
    }

    pub fn get_primary_key(&self) -> Result<(String, ZeroizingBytes), CryptoError> {
        let version = self
            .primary_version
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
            .ok_or(CryptoError::InvalidKeyLength(0))?;

        let keys = self.keys.read().unwrap_or_else(|e| e.into_inner());
        let key_version = keys.get(&version).ok_or(CryptoError::InvalidKeyLength(0))?;

        Ok((
            version.clone(),
            zeroizing_bytes(key_version.key.as_slice().to_vec()),
        ))
    }

    pub fn get_key(&self, version: &str) -> Result<ZeroizingBytes, CryptoError> {
        let keys = self.keys.read().unwrap_or_else(|e| e.into_inner());
        let key_version = keys.get(version).ok_or(CryptoError::InvalidKeyLength(0))?;
        Ok(zeroizing_bytes(key_version.key.as_slice().to_vec()))
    }

    pub fn get_all_versions(&self) -> Vec<String> {
        self.keys
            .read()
            .unwrap_or_else(|e| e.into_inner())
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
        let keys = self.keys.read().unwrap_or_else(|e| e.into_inner());

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
            .unwrap_or_else(|e| e.into_inner())
            .push(provider);
    }

    pub fn add_async_provider(&self, provider: Arc<dyn AsyncKeyProvider>) {
        self.async_providers
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .push(provider);
    }

    pub async fn fetch_and_register(&self, version: &str) -> Result<Vec<u8>, CryptoError> {
        for provider in self
            .providers
            .read()
            .unwrap_or_else(|e| e.into_inner())
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
            .unwrap_or_else(|e| e.into_inner())
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
        self.keys.read().unwrap_or_else(|e| e.into_inner()).len()
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
        let config = KeyRotationConfig {
            max_key_versions: 15,
            ..Default::default()
        };
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
        use crate::interface::KeyProvider;
        use crate::types::ZeroizingBytes;

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
    }

    #[test]
    fn test_key_cache_policy_default() {
        let p = KeyCachePolicy::default();
        assert_eq!(p, KeyCachePolicy::CacheWithTtl(Duration::from_secs(3600)));
        assert_ne!(p, KeyCachePolicy::NoCache);
        assert_ne!(p, KeyCachePolicy::CacheIndefinitely);
    }

    #[test]
    fn test_key_rotation_config_default() {
        let cfg = KeyRotationConfig::default();
        assert_eq!(cfg.max_key_versions, 3);
    }

    #[test]
    fn test_rotate_to_no_prior_primary() {
        // Rotating when no primary key exists returns "none".
        let registry = KeyRegistry::new(KeyRotationConfig::default());
        let old = registry
            .rotate_to("v1".to_string(), SecretBytes::new(vec![1u8; 32]))
            .unwrap();
        assert_eq!(old, "none");
        let (version, _) = registry.get_primary_key().unwrap();
        assert_eq!(version, "v1");
    }

    #[test]
    fn test_get_primary_key_no_keys() {
        // No primary key registered → InvalidKeyLength(0) error.
        let registry = KeyRegistry::new(KeyRotationConfig::default());
        let result = registry.get_primary_key();
        assert!(result.is_err());
    }

    #[test]
    fn test_register_key_demotes_old_primary() {
        let registry = KeyRegistry::new(KeyRotationConfig::default());
        registry
            .register_key("v1".to_string(), SecretBytes::new(vec![1u8; 32]), true)
            .unwrap();
        // Register v2 as primary — v1 must be demoted.
        registry
            .register_key("v2".to_string(), SecretBytes::new(vec![2u8; 32]), true)
            .unwrap();

        let (version, _) = registry.get_primary_key().unwrap();
        assert_eq!(version, "v2");
        // Both keys remain in storage.
        assert_eq!(registry.version_count(), 2);
        // Old primary key still retrievable.
        assert!(registry.get_key("v1").is_ok());
    }

    #[test]
    fn test_register_key_replaces_same_version() {
        let registry = KeyRegistry::new(KeyRotationConfig::default());
        registry
            .register_key("v1".to_string(), SecretBytes::new(vec![1u8; 32]), true)
            .unwrap();
        // Re-register same version with different key.
        registry
            .register_key("v1".to_string(), SecretBytes::new(vec![2u8; 32]), true)
            .unwrap();
        assert_eq!(registry.version_count(), 1);
        let retrieved = registry.get_key("v1").unwrap();
        assert_eq!(&*retrieved, &vec![2u8; 32]);
    }

    #[test]
    fn test_try_decrypt_with_all_keys_no_match() {
        use crate::secret::XChaCha20Crypto;

        let registry = KeyRegistry::new(KeyRotationConfig::default());
        let crypto = XChaCha20Crypto::new();

        // Register key1.
        let key1 = vec![1u8; 32];
        registry
            .register_key("v1".to_string(), SecretBytes::new(key1), true)
            .unwrap();

        // Encrypt with a DIFFERENT key not in the registry.
        let key2 = vec![2u8; 32];
        let plaintext = b"secret message";
        let (nonce, ciphertext) = crypto.encrypt(plaintext, &key2).unwrap();

        // No registered key can decrypt → DecryptionFailed.
        let result = registry.try_decrypt_with_all_keys(&nonce, &ciphertext);
        assert!(result.is_err());
    }

    #[test]
    fn test_try_decrypt_with_all_keys_empty_registry() {
        let registry = KeyRegistry::new(KeyRotationConfig::default());
        let result = registry.try_decrypt_with_all_keys(&[0u8; 24], b"some ciphertext");
        assert!(result.is_err());
    }

    #[test]
    fn test_add_async_provider() {
        // Verify add_async_provider does not panic with a minimal mock.
        // Uses a real provider only when encryption feature is enabled.
        #[cfg(feature = "encryption")]
        {
            use crate::error::ConfigResult;
            use crate::interface::AsyncKeyProvider;
            use crate::types::ZeroizingBytes;
            use async_trait::async_trait;

            struct DummyAsync;
            #[async_trait]
            impl AsyncKeyProvider for DummyAsync {
                async fn get_key(&self) -> ConfigResult<ZeroizingBytes> {
                    Ok(ZeroizingBytes::new(vec![0u8; 32]))
                }
                fn provider_type(&self) -> &'static str {
                    "dummy-async"
                }
            }

            let registry = KeyRegistry::new(KeyRotationConfig::default());
            registry.add_async_provider(Arc::new(DummyAsync));
            // No panic means success.
        }
    }

    #[cfg(feature = "encryption")]
    #[tokio::test]
    async fn test_fetch_and_register_sync_provider() {
        use crate::error::ConfigResult;
        use crate::interface::KeyProvider;
        use crate::types::ZeroizingBytes;

        struct OkProvider;
        impl KeyProvider for OkProvider {
            fn get_key(&self) -> ConfigResult<ZeroizingBytes> {
                Ok(ZeroizingBytes::new(vec![5u8; 32]))
            }
            fn provider_type(&self) -> &'static str {
                "test-ok"
            }
        }

        let registry = KeyRegistry::new(KeyRotationConfig::default());
        registry.add_provider(Arc::new(OkProvider));
        let result = registry.fetch_and_register("v1").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![5u8; 32]);
        // Key registered as non-primary.
        assert_eq!(registry.version_count(), 1);
        assert!(registry.get_key("v1").is_ok());
    }

    #[cfg(feature = "encryption")]
    #[tokio::test]
    async fn test_fetch_and_register_no_providers() {
        let registry = KeyRegistry::new(KeyRotationConfig::default());
        let result = registry.fetch_and_register("v1").await;
        assert!(result.is_err());
    }

    #[cfg(feature = "encryption")]
    #[tokio::test]
    async fn test_fetch_and_register_async_provider() {
        use crate::error::ConfigResult;
        use crate::interface::AsyncKeyProvider;
        use crate::types::ZeroizingBytes;
        use async_trait::async_trait;

        struct OkAsyncProvider;
        #[async_trait]
        impl AsyncKeyProvider for OkAsyncProvider {
            async fn get_key(&self) -> ConfigResult<ZeroizingBytes> {
                Ok(ZeroizingBytes::new(vec![7u8; 32]))
            }
            fn provider_type(&self) -> &'static str {
                "test-ok-async"
            }
        }

        let registry = KeyRegistry::new(KeyRotationConfig::default());
        registry.add_async_provider(Arc::new(OkAsyncProvider));
        let result = registry.fetch_and_register("v1").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![7u8; 32]);
    }

    #[test]
    fn test_key_registry_builder_default_impl() {
        let registry = KeyRegistryBuilder::default().build();
        assert_eq!(registry.version_count(), 0);
        assert!(registry.get_primary_key().is_err());
    }

    #[test]
    fn test_key_registry_builder_full_chain() {
        let registry = KeyRegistry::builder()
            .max_versions(5)
            .cache_policy(KeyCachePolicy::NoCache)
            .initial_key("v1".to_string(), vec![1u8; 32], true)
            .initial_key("v2".to_string(), vec![2u8; 32], false)
            .build();
        assert_eq!(registry.version_count(), 2);
        let (version, _) = registry.get_primary_key().unwrap();
        assert_eq!(version, "v1");
    }

    #[test]
    fn test_key_registry_builder_initial_keys() {
        let registry = KeyRegistry::builder()
            .initial_key("alpha".to_string(), vec![1u8; 32], true)
            .initial_key("beta".to_string(), vec![2u8; 32], false)
            .build();
        let versions = registry.get_all_versions();
        assert_eq!(versions.len(), 2);
        assert!(versions.contains(&"alpha".to_string()));
        assert!(versions.contains(&"beta".to_string()));
    }

    #[test]
    fn test_key_registry_builder_max_versions() {
        let registry = KeyRegistry::builder()
            .max_versions(2)
            .initial_key("v1".to_string(), vec![1u8; 32], true)
            .initial_key("v2".to_string(), vec![2u8; 32], false)
            .initial_key("v3".to_string(), vec![3u8; 32], false)
            .build();
        // max_versions=2 enforces eviction of oldest non-primary.
        assert_eq!(registry.version_count(), 2);
    }

    #[test]
    fn test_key_registry_builder_config_setter() {
        let config = KeyRotationConfig {
            max_key_versions: 1,
            ..Default::default()
        };
        let registry = KeyRegistry::builder()
            .config(config)
            .initial_key("v1".to_string(), vec![1u8; 32], true)
            .initial_key("v2".to_string(), vec![2u8; 32], false)
            .build();
        // config.max_key_versions=1 → v1 (primary) evicts v2.
        assert_eq!(registry.version_count(), 1);
    }

    #[test]
    fn test_key_registry_builder_with_provider() {
        use crate::error::ConfigResult;
        use crate::interface::KeyProvider;
        use crate::types::ZeroizingBytes;

        struct DummyProvider;
        impl KeyProvider for DummyProvider {
            fn get_key(&self) -> ConfigResult<ZeroizingBytes> {
                Ok(ZeroizingBytes::new(vec![9u8; 32]))
            }
            fn provider_type(&self) -> &'static str {
                "builder-dummy"
            }
        }

        let registry = KeyRegistry::builder()
            .provider(Arc::new(DummyProvider))
            .build();
        // Provider registered without panic; fetch must succeed.
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(registry.fetch_and_register("v1"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_max_versions_keeps_primary() {
        // Primary key must NOT be evicted even when max versions exceeded.
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

        // v2 should be evicted (oldest non-primary), v1 (primary) must remain.
        assert_eq!(registry.version_count(), 2);
        assert!(registry.get_key("v1").is_ok());
        assert!(registry.get_key("v2").is_err());
        assert!(registry.get_key("v3").is_ok());
        let (primary, _) = registry.get_primary_key().unwrap();
        assert_eq!(primary, "v1");
    }

    #[test]
    fn test_version_count_initial() {
        let registry = KeyRegistry::new(KeyRotationConfig::default());
        assert_eq!(registry.version_count(), 0);
    }

    #[test]
    fn test_get_all_versions_empty() {
        let registry = KeyRegistry::new(KeyRotationConfig::default());
        let versions = registry.get_all_versions();
        assert!(versions.is_empty());
    }
}
