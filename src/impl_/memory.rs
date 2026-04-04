//! In-memory configuration implementation using moka cache.
//!
//! This module provides `InMemoryConfig` - a thread-safe, high-performance
//! in-memory configuration store backed by moka cache.

use crate::error::ConfersResult;
use crate::interface::{ConfigConnector, ConfigReader, ConfigWriter};
use crate::value::{AnnotatedValue, SourceId};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

// ============== Async Implementation (feature-gated) ==============

#[cfg(any(
    feature = "remote",
    feature = "config-bus",
    feature = "encryption",
    feature = "watch"
))]
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
        cache: Cache<String, AnnotatedValue>,
        /// Source ID for values created by this config
        source_id: SourceId,
        /// Default priority for new values
        default_priority: u8,
        /// Version counter for optimistic concurrency
        version: AtomicU64,
        /// Health status
        healthy: AtomicBool,
        /// Maximum capacity
        #[allow(dead_code)]
        max_capacity: u64,
    }

    impl InMemoryConfig {
        /// Create a new in-memory config with default settings.
        pub fn new() -> Self {
            Self::builder().build()
        }

        /// Create a builder for custom configuration.
        pub fn builder() -> InMemoryConfigBuilder {
            InMemoryConfigBuilder::default()
        }

        /// Get the current version.
        pub fn version(&self) -> u64 {
            self.version.load(Ordering::Relaxed)
        }

        /// Check if the config is healthy.
        pub fn is_healthy(&self) -> bool {
            self.healthy.load(Ordering::Relaxed)
        }

        /// Get max capacity.
        pub fn max_capacity(&self) -> u64 {
            self.max_capacity
        }

        /// Get default priority.
        pub fn default_priority(&self) -> u8 {
            self.default_priority
        }

        /// Get source ID.
        pub fn source_id(&self) -> &SourceId {
            &self.source_id
        }
    }

    impl Default for InMemoryConfig {
        fn default() -> Self {
            Self::new()
        }
    }

    #[async_trait]
    impl ConfigReader for InMemoryConfig {
        async fn get_raw(&self, key: &str) -> ConfersResult<Option<AnnotatedValue>> {
            Ok(self.cache.get(&key.to_string()).await)
        }

        async fn keys(&self) -> ConfersResult<Vec<String>> {
            Ok(self.cache.iter().map(|(k, _)| k.to_string()).collect())
        }
    }

    #[async_trait]
    impl ConfigWriter for InMemoryConfig {
        async fn set(&self, key: &str, value: AnnotatedValue) -> ConfersResult<()> {
            self.cache.insert(key.to_string(), value).await;
            self.version.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }

        async fn delete(&self, key: &str) -> ConfersResult<bool> {
            let existed = self.cache.remove(&key.to_string()).await.is_some();
            if existed {
                self.version.fetch_add(1, Ordering::Relaxed);
            }
            Ok(existed)
        }

        async fn clear(&self) -> ConfersResult<()> {
            self.cache.invalidate_all();
            self.version.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }
    }

    #[async_trait]
    impl ConfigConnector for InMemoryConfig {
        async fn health_check(&self) -> anyhow::Result<()> {
            if self.healthy.load(Ordering::Relaxed) {
                Ok(())
            } else {
                anyhow::bail!("InMemoryConfig is not healthy")
            }
        }

        async fn shutdown(&self) {
            self.cache.invalidate_all();
            self.healthy.store(false, Ordering::Relaxed);
        }
    }

    /// Builder for InMemoryConfig.
    #[derive(Debug, Clone)]
    pub struct InMemoryConfigBuilder {
        /// Maximum number of entries
        max_capacity: u64,
        /// Time-to-live in seconds (0 = no TTL)
        ttl_seconds: u64,
        /// Initial capacity
        initial_capacity: usize,
        /// Default priority for values
        default_priority: u8,
        /// Source ID
        source_id: Option<SourceId>,
    }

    impl Default for InMemoryConfigBuilder {
        fn default() -> Self {
            Self {
                max_capacity: 10_000,
                ttl_seconds: 0,
                initial_capacity: 128,
                default_priority: 0,
                source_id: None,
            }
        }
    }

    impl InMemoryConfigBuilder {
        /// Set maximum capacity.
        pub fn max_capacity(mut self, capacity: u64) -> Self {
            self.max_capacity = capacity;
            self
        }

        /// Set time-to-live in seconds.
        pub fn ttl_seconds(mut self, seconds: u64) -> Self {
            self.ttl_seconds = seconds;
            self
        }

        /// Set initial capacity.
        pub fn initial_capacity(mut self, capacity: usize) -> Self {
            self.initial_capacity = capacity;
            self
        }

        /// Set default priority.
        pub fn default_priority(mut self, priority: u8) -> Self {
            self.default_priority = priority;
            self
        }

        /// Set source ID.
        pub fn source_id(mut self, id: impl Into<SourceId>) -> Self {
            self.source_id = Some(id.into());
            self
        }

        /// Build the InMemoryConfig.
        pub fn build(self) -> InMemoryConfig {
            let mut builder = Cache::builder()
                .max_capacity(self.max_capacity)
                .initial_capacity(self.initial_capacity);

            if self.ttl_seconds > 0 {
                builder = builder.time_to_live(std::time::Duration::from_secs(self.ttl_seconds));
            }

            InMemoryConfig {
                cache: builder.build(),
                source_id: self.source_id.unwrap_or_else(|| SourceId::new("memory")),
                default_priority: self.default_priority,
                version: AtomicU64::new(0),
                healthy: AtomicBool::new(true),
                max_capacity: self.max_capacity,
            }
        }
    }
}

#[cfg(any(
    feature = "remote",
    feature = "config-bus",
    feature = "encryption",
    feature = "watch"
))]
pub use async_impl::{InMemoryConfig, InMemoryConfigBuilder};

// ============== Sync Implementation (for minimal builds) ==============

#[cfg(not(any(
    feature = "remote",
    feature = "config-bus",
    feature = "encryption",
    feature = "watch"
)))]
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
        cache: Cache<String, AnnotatedValue>,
        /// Source ID for values created by this config
        source_id: SourceId,
        /// Default priority for new values
        default_priority: u8,
        /// Version counter for optimistic concurrency
        version: AtomicU64,
        /// Health status
        healthy: AtomicBool,
        /// Maximum capacity
        #[allow(dead_code)]
        max_capacity: u64,
    }

    impl InMemoryConfig {
        /// Create a new in-memory config with default settings.
        pub fn new() -> Self {
            Self::builder().build()
        }

        /// Create a builder for custom configuration.
        pub fn builder() -> InMemoryConfigBuilder {
            InMemoryConfigBuilder::default()
        }

        /// Get the current version.
        pub fn version(&self) -> u64 {
            self.version.load(Ordering::Relaxed)
        }

        /// Check if the config is healthy.
        pub fn is_healthy(&self) -> bool {
            self.healthy.load(Ordering::Relaxed)
        }

        /// Get max capacity.
        pub fn max_capacity(&self) -> u64 {
            self.max_capacity
        }

        /// Get default priority.
        pub fn default_priority(&self) -> u8 {
            self.default_priority
        }

        /// Get source ID.
        pub fn source_id(&self) -> &SourceId {
            &self.source_id
        }
    }

    impl Default for InMemoryConfig {
        fn default() -> Self {
            Self::new()
        }
    }

    impl ConfigReader for InMemoryConfig {
        fn get_raw(&self, key: &str) -> ConfersResult<Option<AnnotatedValue>> {
            Ok(self.cache.get(&key.to_string()))
        }

        fn keys(&self) -> ConfersResult<Vec<String>> {
            Ok(self.cache.iter().map(|(k, _)| k.to_string()).collect())
        }
    }

    impl ConfigWriter for InMemoryConfig {
        fn set(&self, key: &str, value: AnnotatedValue) -> ConfersResult<()> {
            self.cache.insert(key.to_string(), value);
            self.version.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }

        fn delete(&self, key: &str) -> ConfersResult<bool> {
            let existed = self.cache.remove(&key.to_string()).is_some();
            if existed {
                self.version.fetch_add(1, Ordering::Relaxed);
            }
            Ok(existed)
        }

        fn clear(&self) -> ConfersResult<()> {
            self.cache.invalidate_all();
            self.version.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }
    }

    impl ConfigConnector for InMemoryConfig {
        fn health_check(&self) -> anyhow::Result<()> {
            if self.healthy.load(Ordering::Relaxed) {
                Ok(())
            } else {
                anyhow::bail!("InMemoryConfig is not healthy")
            }
        }

        fn shutdown(&self) {
            self.cache.invalidate_all();
            self.healthy.store(false, Ordering::Relaxed);
        }
    }

    /// Builder for InMemoryConfig.
    #[derive(Debug, Clone)]
    pub struct InMemoryConfigBuilder {
        /// Maximum number of entries
        max_capacity: u64,
        /// Time-to-live in seconds (0 = no TTL)
        ttl_seconds: u64,
        /// Initial capacity
        initial_capacity: usize,
        /// Default priority for values
        default_priority: u8,
        /// Source ID
        source_id: Option<SourceId>,
    }

    impl Default for InMemoryConfigBuilder {
        fn default() -> Self {
            Self {
                max_capacity: 10_000,
                ttl_seconds: 0,
                initial_capacity: 128,
                default_priority: 0,
                source_id: None,
            }
        }
    }

    impl InMemoryConfigBuilder {
        /// Set maximum capacity.
        pub fn max_capacity(mut self, capacity: u64) -> Self {
            self.max_capacity = capacity;
            self
        }

        /// Set time-to-live in seconds.
        pub fn ttl_seconds(mut self, seconds: u64) -> Self {
            self.ttl_seconds = seconds;
            self
        }

        /// Set initial capacity.
        pub fn initial_capacity(mut self, capacity: usize) -> Self {
            self.initial_capacity = capacity;
            self
        }

        /// Set default priority.
        pub fn default_priority(mut self, priority: u8) -> Self {
            self.default_priority = priority;
            self
        }

        /// Set source ID.
        pub fn source_id(mut self, id: impl Into<SourceId>) -> Self {
            self.source_id = Some(id.into());
            self
        }

        /// Build the InMemoryConfig.
        pub fn build(self) -> InMemoryConfig {
            let mut builder = Cache::builder()
                .max_capacity(self.max_capacity)
                .initial_capacity(self.initial_capacity);

            if self.ttl_seconds > 0 {
                builder = builder.time_to_live(std::time::Duration::from_secs(self.ttl_seconds));
            }

            InMemoryConfig {
                cache: builder.build(),
                source_id: self.source_id.unwrap_or_else(|| SourceId::new("memory")),
                default_priority: self.default_priority,
                version: AtomicU64::new(0),
                healthy: AtomicBool::new(true),
                max_capacity: self.max_capacity,
            }
        }
    }
}

#[cfg(not(any(
    feature = "remote",
    feature = "config-bus",
    feature = "encryption",
    feature = "watch"
)))]
pub use sync_impl::{InMemoryConfig, InMemoryConfigBuilder};

// ============== Helper Methods (common to both) ==============

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::ConfigValue;

    #[cfg(any(
        feature = "remote",
        feature = "config-bus",
        feature = "encryption",
        feature = "watch"
    ))]
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
            assert!(!config.is_healthy());
        }

        #[tokio::test]
        async fn test_builder() {
            let config = InMemoryConfig::builder()
                .max_capacity(1000)
                .ttl_seconds(60)
                .default_priority(10)
                .source_id("custom")
                .build();

            assert_eq!(config.max_capacity(), 1000);
            assert_eq!(config.default_priority(), 10);
        }

        #[tokio::test]
        async fn test_version() {
            let config = InMemoryConfig::new();
            assert_eq!(config.version(), 0);

            config
                .set(
                    "key",
                    AnnotatedValue::new(ConfigValue::string("value"), SourceId::new("test"), "key"),
                )
                .await
                .unwrap();
            assert_eq!(config.version(), 1);

            config.delete("key").await.unwrap();
            assert_eq!(config.version(), 2);
        }
    }

    #[cfg(not(any(
        feature = "remote",
        feature = "config-bus",
        feature = "encryption",
        feature = "watch"
    )))]
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
            assert!(!config.is_healthy());
        }

        #[test]
        fn test_builder() {
            let config = InMemoryConfig::builder()
                .max_capacity(1000)
                .ttl_seconds(60)
                .default_priority(10)
                .source_id("custom")
                .build();

            assert_eq!(config.max_capacity(), 1000);
            assert_eq!(config.default_priority(), 10);
        }

        #[test]
        fn test_version() {
            let config = InMemoryConfig::new();
            assert_eq!(config.version(), 0);

            config
                .set(
                    "key",
                    AnnotatedValue::new(ConfigValue::string("value"), SourceId::new("test"), "key"),
                )
                .unwrap();
            assert_eq!(config.version(), 1);

            config.delete("key").unwrap();
            assert_eq!(config.version(), 2);
        }
    }
}
