// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! In-memory configuration implementation using moka cache.
//!
//! This module provides `InMemoryConfig` - a thread-safe, high-performance
//! in-memory configuration store backed by moka cache.
//!
//! # Feature-gated async/sync duality
//!
//! `InMemoryConfig` is defined twice: async_impl uses `moka::future::Cache`
//! with `#[async_trait]`; sync_impl uses `moka::sync::Cache`. Only one is
//! compiled based on feature flags (remote/config-bus/encryption/watch).
//!
//! # BrickArchitecture Compliance
//!
//! This module follows BrickArchitecture patterns:
//! - Factory functions return Result for initialization failures
//! - Configuration phase errors use `ConfigConfigError`
//! - Runtime errors use `ConfersError`

use crate::error::{ConfersResult, ConfigConfigError};
use crate::impl_::lifecycle::Lifecycle;
use crate::interface::sealed::Sealed;
use crate::interface::{ConfigConnector, ConfigReader, ConfigWriter};
use crate::types::AnnotatedValue;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

// ============== Async Implementation (feature-gated) ==============

/// Zero-copy read port: fetch the shared handle (`Arc`) to a stored value
/// instead of a deep clone.
///
/// Implemented by `InMemoryConfig`; pair it with [`crate::new_in_memory`]
/// for allocation-free hot-path reads of large values. Only available in
/// async builds (one of `remote`/`config-bus`/`encryption`/`watch`); the
/// minimal sync build keeps zero dependencies.
#[cfg(feature = "async-core")]
#[async_trait::async_trait]
pub trait SharedValueReader: Send + Sync {
    async fn get_shared(&self, key: &str) -> ConfersResult<Option<Arc<AnnotatedValue>>>;
}

#[cfg(feature = "async-core")]
mod async_impl {
    use super::*;
    use async_trait::async_trait;
    use moka::future::Cache;

    /// In-memory configuration store backed by moka async cache.
    ///
    /// Thread-safe and highly performant for concurrent access.
    /// Supports TTL and size-based eviction.
    #[derive(Debug)]
    pub struct InMemoryConfig {
        /// The underlying moka cache
        cache: Cache<String, Arc<AnnotatedValue>>,
        /// Health status
        healthy: AtomicBool,
    }

    impl InMemoryConfig {
        /// Create a new in-memory config with default settings.
        ///
        /// The cache is sized at 10 000 entries with an initial capacity of
        /// 128 and no TTL.
        pub fn new() -> Self {
            Self {
                cache: Cache::builder()
                    .max_capacity(10_000)
                    .initial_capacity(128)
                    .build(),
                healthy: AtomicBool::new(true),
            }
        }
    }

    impl Default for InMemoryConfig {
        fn default() -> Self {
            Self::new()
        }
    }

    impl Sealed for InMemoryConfig {}

    #[async_trait]
    impl ConfigReader for InMemoryConfig {
        async fn get_raw(&self, key: &str) -> ConfersResult<Option<AnnotatedValue>> {
            Ok(self.cache.get(&key.to_string()).await.map(|v| (*v).clone()))
        }

        async fn keys(&self) -> ConfersResult<Vec<String>> {
            Ok(self.cache.iter().map(|(k, _)| k.to_string()).collect())
        }
    }

    #[async_trait]
    impl super::SharedValueReader for InMemoryConfig {
        /// Zero-copy read: returns the shared handle to the stored value
        /// (an `Arc` clone) instead of deep-cloning the value tree.
        async fn get_shared(&self, key: &str) -> ConfersResult<Option<Arc<AnnotatedValue>>> {
            Ok(self.cache.get(&key.to_string()).await)
        }
    }

    #[async_trait]
    impl ConfigWriter for InMemoryConfig {
        async fn set(&self, key: &str, value: AnnotatedValue) -> ConfersResult<()> {
            self.cache.insert(key.to_string(), Arc::new(value)).await;
            Ok(())
        }

        async fn delete(&self, key: &str) -> ConfersResult<bool> {
            Ok(self.cache.remove(&key.to_string()).await.is_some())
        }

        async fn clear(&self) -> ConfersResult<()> {
            self.cache.invalidate_all();
            Ok(())
        }
    }

    #[async_trait]
    impl Lifecycle for InMemoryConfig {
        async fn start(&self) -> Result<(), ConfigConfigError> {
            Ok(())
        }
        async fn stop(&self) -> ConfersResult<()> {
            self.healthy.store(false, Ordering::Release);
            Ok(())
        }
    }

    #[async_trait]
    impl ConfigConnector for InMemoryConfig {
        async fn health_check(&self) -> crate::error::ConfersResult<()> {
            if self.healthy.load(Ordering::Acquire) {
                Ok(())
            } else {
                Err(crate::error::ConfigError::HealthCheckFailed {
                    reason: "InMemoryConfig is not healthy".into(),
                })
            }
        }

        async fn shutdown(&self) {
            self.cache.invalidate_all();
            self.healthy.store(false, Ordering::Release);
        }
    }
}

#[cfg(feature = "async-core")]
pub use async_impl::InMemoryConfig;

// ============== Sync Implementation (for minimal builds) ==============

#[cfg(not(feature = "async-core"))]
mod sync_impl {
    use super::*;
    use moka::sync::Cache;

    /// In-memory configuration store backed by moka sync cache.
    ///
    /// Thread-safe and highly performant for concurrent access.
    /// Supports TTL and size-based eviction.
    #[derive(Debug)]
    pub struct InMemoryConfig {
        /// The underlying moka cache
        cache: Cache<String, Arc<AnnotatedValue>>,
        /// Health status
        healthy: AtomicBool,
    }

    impl InMemoryConfig {
        /// Create a new in-memory config with default settings.
        ///
        /// The cache is sized at 10 000 entries with an initial capacity of
        /// 128 and no TTL.
        pub fn new() -> Self {
            Self {
                cache: Cache::builder()
                    .max_capacity(10_000)
                    .initial_capacity(128)
                    .build(),
                healthy: AtomicBool::new(true),
            }
        }
    }

    impl Default for InMemoryConfig {
        fn default() -> Self {
            Self::new()
        }
    }

    impl Sealed for InMemoryConfig {}

    impl ConfigReader for InMemoryConfig {
        fn get_raw(&self, key: &str) -> ConfersResult<Option<AnnotatedValue>> {
            Ok(self.cache.get(&key.to_string()).map(|v| (*v).clone()))
        }

        fn keys(&self) -> ConfersResult<Vec<String>> {
            Ok(self.cache.iter().map(|(k, _)| k.to_string()).collect())
        }
    }

    impl ConfigWriter for InMemoryConfig {
        fn set(&self, key: &str, value: AnnotatedValue) -> ConfersResult<()> {
            self.cache.insert(key.to_string(), Arc::new(value));
            Ok(())
        }

        fn delete(&self, key: &str) -> ConfersResult<bool> {
            Ok(self.cache.remove(&key.to_string()).is_some())
        }

        fn clear(&self) -> ConfersResult<()> {
            self.cache.invalidate_all();
            Ok(())
        }
    }

    impl ConfigConnector for InMemoryConfig {
        fn health_check(&self) -> crate::error::ConfersResult<()> {
            if self.healthy.load(Ordering::Acquire) {
                Ok(())
            } else {
                Err(crate::error::ConfigError::HealthCheckFailed {
                    reason: "InMemoryConfig is not healthy".into(),
                })
            }
        }

        fn shutdown(&self) {
            self.cache.invalidate_all();
            self.healthy.store(false, Ordering::Release);
        }
    }

    impl Lifecycle for InMemoryConfig {
        fn start(&self) -> Result<(), ConfigConfigError> {
            Ok(())
        }
        fn stop(&self) -> ConfersResult<()> {
            self.healthy.store(false, Ordering::Release);
            Ok(())
        }
    }
}

#[cfg(not(feature = "async-core"))]
pub use sync_impl::InMemoryConfig;

// ============== Helper Methods (common to both) ==============

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ConfigValue, SourceId};

    #[cfg(feature = "async-core")]
    mod async_tests {
        use super::*;

        #[tokio::test]
        async fn test_basic_operations() {
            let config = InMemoryConfig::new();

            // Set a value
            let value = AnnotatedValue::new(
                ConfigValue::string("test_value"),
                SourceId::new("test"),
                "test.key",
            );
            config.set("test.key", value).await.unwrap();

            // Get the value
            let result = config.get_raw("test.key").await.unwrap();
            assert!(result.is_some());
            assert_eq!(result.unwrap().as_str(), Some("test_value"));

            // Check keys
            let keys = config.keys().await.unwrap();
            assert!(keys.contains(&"test.key".to_string()));

            // Delete
            let deleted = config.delete("test.key").await.unwrap();
            assert!(deleted);

            // Verify deleted
            let result = config.get_raw("test.key").await.unwrap();
            assert!(result.is_none());
        }

        #[tokio::test]
        async fn test_health_check() {
            let config = InMemoryConfig::new();
            assert!(config.health_check().await.is_ok());
        }

        #[tokio::test]
        async fn test_shutdown() {
            let config = InMemoryConfig::new();
            config
                .set(
                    "key",
                    AnnotatedValue::new(ConfigValue::string("value"), SourceId::new("test"), "key"),
                )
                .await
                .unwrap();

            config.shutdown().await;
            assert!(config.health_check().await.is_err());
        }

        #[tokio::test]
        async fn test_default_impl() {
            let config = InMemoryConfig::default();
            assert!(config.health_check().await.is_ok());
        }

        #[tokio::test]
        async fn test_clear() {
            let config = InMemoryConfig::new();
            config
                .set(
                    "k1",
                    AnnotatedValue::new(ConfigValue::string("v1"), SourceId::new("t"), "k1"),
                )
                .await
                .unwrap();

            config.clear().await.unwrap();
            let keys = config.keys().await.unwrap();
            assert!(keys.is_empty());
        }

        #[tokio::test]
        async fn test_keys_empty() {
            let config = InMemoryConfig::new();
            let keys = config.keys().await.unwrap();
            assert!(keys.is_empty());
        }

        #[tokio::test]
        async fn test_health_check_after_shutdown() {
            let config = InMemoryConfig::new();
            assert!(config.health_check().await.is_ok());

            config.shutdown().await;

            let err = config.health_check().await.unwrap_err();
            match err {
                crate::error::ConfigError::HealthCheckFailed { reason } => {
                    assert!(reason.contains("not healthy"));
                }
                other => panic!("expected HealthCheckFailed, got {:?}", other),
            }
        }

        #[tokio::test]
        async fn test_start_lifecycle() {
            let config = InMemoryConfig::new();
            assert!(config.start().await.is_ok());
            assert!(config.health_check().await.is_ok());
        }

        #[tokio::test]
        async fn test_stop_lifecycle() {
            let config = InMemoryConfig::new();
            assert!(config.health_check().await.is_ok());
            assert!(config.stop().await.is_ok());
            assert!(config.health_check().await.is_err());
        }
    }

    #[cfg(not(feature = "async-core"))]
    mod sync_tests {
        use super::*;

        #[test]
        fn test_basic_operations() {
            let config = InMemoryConfig::new();

            // Set a value
            let value = AnnotatedValue::new(
                ConfigValue::string("test_value"),
                SourceId::new("test"),
                "test.key",
            );
            config.set("test.key", value).unwrap();

            // Get the value
            let result = config.get_raw("test.key").unwrap();
            assert!(result.is_some());
            assert_eq!(result.unwrap().as_str(), Some("test_value"));

            // Check keys
            let keys = config.keys().unwrap();
            assert!(keys.contains(&"test.key".to_string()));

            // Delete
            let deleted = config.delete("test.key").unwrap();
            assert!(deleted);

            // Verify deleted
            let result = config.get_raw("test.key").unwrap();
            assert!(result.is_none());
        }

        #[test]
        fn test_health_check() {
            let config = InMemoryConfig::new();
            assert!(config.health_check().is_ok());
        }

        #[test]
        fn test_shutdown() {
            let config = InMemoryConfig::new();
            config
                .set(
                    "key",
                    AnnotatedValue::new(ConfigValue::string("value"), SourceId::new("test"), "key"),
                )
                .unwrap();

            config.shutdown();
            assert!(config.health_check().is_err());
        }
    }
}

#[cfg(all(test, feature = "async-core"))]
mod zero_copy_tests {
    use super::*;
    use crate::types::{ConfigValue, SourceId};

    #[tokio::test]
    async fn get_shared_returns_same_underlying_value() {
        let config = InMemoryConfig::new();
        config
            .set(
                "large.value",
                AnnotatedValue::new(
                    ConfigValue::string("x".repeat(10_000)),
                    SourceId::default(),
                    "large.value",
                ),
            )
            .await
            .unwrap();

        let shared1 = SharedValueReader::get_shared(&config, "large.value")
            .await
            .unwrap()
            .unwrap();
        let shared2 = SharedValueReader::get_shared(&config, "large.value")
            .await
            .unwrap()
            .unwrap();

        // Zero-copy: both handles wrap the same allocation.
        assert!(
            Arc::ptr_eq(&shared1, &shared2),
            "get_shared must return the shared handle, not a clone"
        );
        assert_eq!(shared1.as_str(), shared2.as_str());
    }

    #[tokio::test]
    async fn get_shared_absent_key_is_none() {
        let config = InMemoryConfig::new();
        assert!(
            SharedValueReader::get_shared(&config, "missing")
                .await
                .unwrap()
                .is_none()
        );
    }
}
