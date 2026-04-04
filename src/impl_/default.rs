//! Default configuration implementation.
//!
//! This module provides `ConfigImpl` - the primary configuration implementation
//! that combines multiple sources and implements the BrickArchitecture traits.

use crate::config::{ConfigLimits, SourceChain, SourceChainBuilder};
use crate::error::ConfersResult;
use crate::interface::{ConfigConnector, ConfigReader, ConfigWriter};
use crate::merger::MergeStrategy;
use crate::value::{AnnotatedValue, ConfigValue, SourceId};
use std::collections::HashMap;
use std::path::PathBuf;
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

    /// Primary configuration implementation with multiple source support.
    ///
    /// Implements BrickArchitecture specification:
    /// - ConfigReader for read operations
    /// - ConfigWriter for write operations
    /// - ConfigConnector for lifecycle management
    pub struct ConfigImpl {
        /// Merged configuration value (cached)
        merged: std::sync::RwLock<AnnotatedValue>,
        /// In-memory overrides (higher priority)
        overrides: Cache<String, AnnotatedValue>,
        /// Version counter
        version: AtomicU64,
        /// Health status
        healthy: AtomicBool,
        /// Source ID
        source_id: SourceId,
    }

    impl ConfigImpl {
        /// Create a new ConfigImpl from a source chain.
        pub fn from_chain(chain: SourceChain) -> ConfersResult<Self> {
            let merged = chain.collect()?;
            Ok(Self {
                merged: std::sync::RwLock::new(merged),
                overrides: moka::future::Cache::builder().max_capacity(1_000).build(),
                version: AtomicU64::new(0),
                healthy: AtomicBool::new(true),
                source_id: SourceId::new("config"),
            })
        }

        /// Create a builder for constructing ConfigImpl.
        pub fn builder() -> ConfigImplBuilder {
            ConfigImplBuilder::default()
        }

        /// Get the current version.
        pub fn version(&self) -> u64 {
            self.version.load(Ordering::Relaxed)
        }

        /// Get the source ID.
        pub fn source_id(&self) -> &SourceId {
            &self.source_id
        }

        /// Reload configuration from a new chain.
        pub fn reload(&self, chain: SourceChain) -> ConfersResult<()> {
            let merged = chain.collect()?;
            *self.merged.write().unwrap() = merged;
            self.version.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }

        /// Get all keys from the configuration.
        async fn collect_keys(&self) -> ConfersResult<Vec<String>> {
            // First collect from overrides
            let mut keys: Vec<String> = self.overrides.iter().map(|(k, _)| k.to_string()).collect();

            // Then collect from the merged value
            let merged = self.merged.read().unwrap();
            keys.extend(merged.all_paths().iter().map(|p| p.to_string()));

            // Deduplicate
            keys.sort();
            keys.dedup();

            Ok(keys)
        }
    }

    #[async_trait]
    impl ConfigReader for ConfigImpl {
        async fn get_raw(&self, key: &str) -> ConfersResult<Option<AnnotatedValue>> {
            // Check overrides first
            if let Some(value) = self.overrides.get(&key.to_string()).await {
                return Ok(Some(value));
            }

            // Read from merged value
            let merged = self.merged.read().unwrap();
            if let Some(value) = Self::extract_value(&merged, key) {
                return Ok(Some(value));
            }

            Ok(None)
        }

        async fn keys(&self) -> ConfersResult<Vec<String>> {
            self.collect_keys().await
        }
    }

    #[async_trait]
    impl ConfigWriter for ConfigImpl {
        async fn set(&self, key: &str, value: AnnotatedValue) -> ConfersResult<()> {
            self.overrides.insert(key.to_string(), value).await;
            self.version.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }

        async fn delete(&self, key: &str) -> ConfersResult<bool> {
            let existed = self.overrides.remove(&key.to_string()).await.is_some();
            if existed {
                self.version.fetch_add(1, Ordering::Relaxed);
            }
            Ok(existed)
        }

        async fn clear(&self) -> ConfersResult<()> {
            self.overrides.invalidate_all();
            self.version.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }
    }

    #[async_trait]
    impl ConfigConnector for ConfigImpl {
        async fn health_check(&self) -> anyhow::Result<()> {
            if self.healthy.load(Ordering::Relaxed) {
                Ok(())
            } else {
                anyhow::bail!("ConfigImpl is not healthy")
            }
        }

        async fn shutdown(&self) {
            self.overrides.invalidate_all();
            self.healthy.store(false, Ordering::Relaxed);
        }
    }

    impl ConfigImpl {
        /// Extract a value from an AnnotatedValue by key path.
        fn extract_value(merged: &AnnotatedValue, key: &str) -> Option<AnnotatedValue> {
            let parts: Vec<&str> = key.split('.').collect();
            Self::navigate_path(merged, &parts)
        }

        /// Navigate through a path in an AnnotatedValue.
        fn navigate_path(value: &AnnotatedValue, parts: &[&str]) -> Option<AnnotatedValue> {
            if parts.is_empty() {
                return Some(value.clone());
            }

            match &value.inner {
                ConfigValue::Map(map) => {
                    let first = parts[0];
                    if let Some(child) = map.get(first) {
                        Self::navigate_path(child, &parts[1..])
                    } else {
                        None
                    }
                }
                ConfigValue::Array(arr) => {
                    if let Ok(idx) = parts[0].parse::<usize>() {
                        if idx < arr.len() {
                            Self::navigate_path(&arr[idx], &parts[1..])
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
                _ => None,
            }
        }
    }

    /// Builder for ConfigImpl.
    #[derive(Default)]
    pub struct ConfigImplBuilder {
        chain_builder: SourceChainBuilder,
        limits: Option<ConfigLimits>,
        merge_strategy: MergeStrategy,
    }

    impl ConfigImplBuilder {
        /// Add a file source.
        pub fn file(mut self, path: impl Into<PathBuf>) -> Self {
            self.chain_builder = self.chain_builder.file(path);
            self
        }

        /// Add an optional file source.
        pub fn file_optional(mut self, path: impl Into<PathBuf>) -> Self {
            self.chain_builder = self.chain_builder.file_optional(path);
            self
        }

        /// Add an environment source.
        pub fn env(mut self) -> Self {
            self.chain_builder = self.chain_builder.env();
            self
        }

        /// Add an environment source with prefix.
        pub fn env_prefix(mut self, prefix: impl Into<String>) -> Self {
            self.chain_builder = self.chain_builder.env_with_prefix(prefix);
            self
        }

        /// Add default values.
        pub fn defaults(mut self, defaults: HashMap<String, ConfigValue>) -> Self {
            self.chain_builder = self.chain_builder.defaults(defaults);
            self
        }

        /// Set the merge strategy.
        pub fn merge_strategy(mut self, strategy: MergeStrategy) -> Self {
            self.merge_strategy = strategy.clone();
            self.chain_builder = self.chain_builder.strategy(strategy);
            self
        }

        /// Set configuration limits.
        pub fn limits(mut self, limits: ConfigLimits) -> Self {
            self.limits = Some(limits);
            self
        }

        /// Build the ConfigImpl.
        pub fn build(self) -> ConfersResult<ConfigImpl> {
            let chain = self.chain_builder.build();
            ConfigImpl::from_chain(chain)
        }
    }
}

#[cfg(any(
    feature = "remote",
    feature = "config-bus",
    feature = "encryption",
    feature = "watch"
))]
pub use async_impl::{ConfigImpl, ConfigImplBuilder};

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

    /// Primary configuration implementation with multiple source support.
    ///
    /// Implements BrickArchitecture specification:
    /// - ConfigReader for read operations
    /// - ConfigWriter for write operations
    /// - ConfigConnector for lifecycle management
    pub struct ConfigImpl {
        /// Merged configuration value (cached)
        merged: std::sync::RwLock<AnnotatedValue>,
        /// In-memory overrides (higher priority)
        overrides: Cache<String, AnnotatedValue>,
        /// Version counter
        version: AtomicU64,
        /// Health status
        healthy: AtomicBool,
        /// Source ID
        source_id: SourceId,
    }

    impl ConfigImpl {
        /// Create a new ConfigImpl from a source chain.
        pub fn from_chain(chain: SourceChain) -> ConfersResult<Self> {
            let merged = chain.collect()?;
            Ok(Self {
                merged: std::sync::RwLock::new(merged),
                overrides: moka::sync::Cache::builder().max_capacity(1_000).build(),
                version: AtomicU64::new(0),
                healthy: AtomicBool::new(true),
                source_id: SourceId::new("config"),
            })
        }

        /// Create a builder for constructing ConfigImpl.
        pub fn builder() -> ConfigImplBuilder {
            ConfigImplBuilder::default()
        }

        /// Get the current version.
        pub fn version(&self) -> u64 {
            self.version.load(Ordering::Relaxed)
        }

        /// Get the source ID.
        pub fn source_id(&self) -> &SourceId {
            &self.source_id
        }

        /// Reload configuration from a new chain.
        pub fn reload(&self, chain: SourceChain) -> ConfersResult<()> {
            let merged = chain.collect()?;
            *self.merged.write().unwrap() = merged;
            self.version.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }

        /// Get all keys from the configuration.
        fn collect_keys(&self) -> ConfersResult<Vec<String>> {
            // First collect from overrides
            let mut keys: Vec<String> = self.overrides.iter().map(|(k, _)| k.to_string()).collect();

            // Then collect from the merged value
            let merged = self.merged.read().unwrap();
            keys.extend(merged.all_paths().iter().map(|p| p.to_string()));

            // Deduplicate
            keys.sort();
            keys.dedup();

            Ok(keys)
        }
    }

    impl ConfigReader for ConfigImpl {
        fn get_raw(&self, key: &str) -> ConfersResult<Option<AnnotatedValue>> {
            // Check overrides first
            if let Some(value) = self.overrides.get(&key.to_string()) {
                return Ok(Some(value));
            }

            // Read from merged value
            let merged = self.merged.read().unwrap();
            if let Some(value) = Self::extract_value(&merged, key) {
                return Ok(Some(value));
            }

            Ok(None)
        }

        fn keys(&self) -> ConfersResult<Vec<String>> {
            self.collect_keys()
        }
    }

    impl ConfigWriter for ConfigImpl {
        fn set(&self, key: &str, value: AnnotatedValue) -> ConfersResult<()> {
            self.overrides.insert(key.to_string(), value);
            self.version.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }

        fn delete(&self, key: &str) -> ConfersResult<bool> {
            let existed = self.overrides.remove(&key.to_string()).is_some();
            if existed {
                self.version.fetch_add(1, Ordering::Relaxed);
            }
            Ok(existed)
        }

        fn clear(&self) -> ConfersResult<()> {
            self.overrides.invalidate_all();
            self.version.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }
    }

    impl ConfigConnector for ConfigImpl {
        fn health_check(&self) -> anyhow::Result<()> {
            if self.healthy.load(Ordering::Relaxed) {
                Ok(())
            } else {
                anyhow::bail!("ConfigImpl is not healthy")
            }
        }

        fn shutdown(&self) {
            self.overrides.invalidate_all();
            self.healthy.store(false, Ordering::Relaxed);
        }
    }

    impl ConfigImpl {
        /// Extract a value from an AnnotatedValue by key path.
        fn extract_value(merged: &AnnotatedValue, key: &str) -> Option<AnnotatedValue> {
            let parts: Vec<&str> = key.split('.').collect();
            Self::navigate_path(merged, &parts)
        }

        /// Navigate through a path in an AnnotatedValue.
        fn navigate_path(value: &AnnotatedValue, parts: &[&str]) -> Option<AnnotatedValue> {
            if parts.is_empty() {
                return Some(value.clone());
            }

            match &value.inner {
                ConfigValue::Map(map) => {
                    let first = parts[0];
                    if let Some(child) = map.get(first) {
                        Self::navigate_path(child, &parts[1..])
                    } else {
                        None
                    }
                }
                ConfigValue::Array(arr) => {
                    if let Ok(idx) = parts[0].parse::<usize>() {
                        if idx < arr.len() {
                            Self::navigate_path(&arr[idx], &parts[1..])
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
                _ => None,
            }
        }
    }

    /// Builder for ConfigImpl.
    #[derive(Default)]
    pub struct ConfigImplBuilder {
        chain_builder: SourceChainBuilder,
        limits: Option<ConfigLimits>,
        merge_strategy: MergeStrategy,
    }

    impl ConfigImplBuilder {
        /// Add a file source.
        pub fn file(mut self, path: impl Into<PathBuf>) -> Self {
            self.chain_builder = self.chain_builder.file(path);
            self
        }

        /// Add an optional file source.
        pub fn file_optional(mut self, path: impl Into<PathBuf>) -> Self {
            self.chain_builder = self.chain_builder.file_optional(path);
            self
        }

        /// Add an environment source.
        pub fn env(mut self) -> Self {
            self.chain_builder = self.chain_builder.env();
            self
        }

        /// Add an environment source with prefix.
        pub fn env_prefix(mut self, prefix: impl Into<String>) -> Self {
            self.chain_builder = self.chain_builder.env_with_prefix(prefix);
            self
        }

        /// Add default values.
        pub fn defaults(mut self, defaults: HashMap<String, ConfigValue>) -> Self {
            self.chain_builder = self.chain_builder.defaults(defaults);
            self
        }

        /// Set the merge strategy.
        pub fn merge_strategy(mut self, strategy: MergeStrategy) -> Self {
            self.merge_strategy = strategy.clone();
            self.chain_builder = self.chain_builder.strategy(strategy);
            self
        }

        /// Set configuration limits.
        pub fn limits(mut self, limits: ConfigLimits) -> Self {
            self.limits = Some(limits);
            self
        }

        /// Build the ConfigImpl.
        pub fn build(self) -> ConfersResult<ConfigImpl> {
            let chain = self.chain_builder.build();
            ConfigImpl::from_chain(chain)
        }
    }
}

#[cfg(not(any(
    feature = "remote",
    feature = "config-bus",
    feature = "encryption",
    feature = "watch"
)))]
pub use sync_impl::{ConfigImpl, ConfigImplBuilder};

// ============== Tests ==============

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(any(
        feature = "remote",
        feature = "config-bus",
        feature = "encryption",
        feature = "watch"
    ))]
    mod async_tests {
        use super::*;

        #[tokio::test]
        async fn test_builder() {
            let config = ConfigImpl::builder()
                .defaults(HashMap::from([
                    ("name".to_string(), ConfigValue::string("test")),
                    ("port".to_string(), ConfigValue::uint(8080)),
                ]))
                .build()
                .unwrap();

            let name = config.get_raw("name").await.unwrap();
            assert!(name.is_some());
            assert_eq!(name.unwrap().as_str(), Some("test"));
        }

        #[tokio::test]
        async fn test_read_write() {
            let config = ConfigImpl::builder()
                .defaults(HashMap::from([(
                    "host".to_string(),
                    ConfigValue::string("localhost"),
                )]))
                .build()
                .unwrap();

            // Read existing
            let host = config.get_raw("host").await.unwrap();
            assert!(host.is_some());

            // Write new
            config
                .set(
                    "port",
                    AnnotatedValue::new(ConfigValue::uint(5432), SourceId::new("test"), "port"),
                )
                .await
                .unwrap();

            let port = config.get_raw("port").await.unwrap();
            assert!(port.is_some());

            // Override existing
            config
                .set(
                    "host",
                    AnnotatedValue::new(
                        ConfigValue::string("127.0.0.1"),
                        SourceId::new("override"),
                        "host",
                    ),
                )
                .await
                .unwrap();

            let host = config.get_raw("host").await.unwrap();
            assert_eq!(host.unwrap().as_str(), Some("127.0.0.1"));
        }

        #[tokio::test]
        async fn test_health_check() {
            let config = ConfigImpl::builder().build().unwrap();
            assert!(config.health_check().await.is_ok());

            config.shutdown().await;
            assert!(config.health_check().await.is_err());
        }

        #[tokio::test]
        async fn test_version() {
            let config = ConfigImpl::builder().build().unwrap();
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
        fn test_builder() {
            let config = ConfigImpl::builder()
                .defaults(HashMap::from([
                    ("name".to_string(), ConfigValue::string("test")),
                    ("port".to_string(), ConfigValue::uint(8080)),
                ]))
                .build()
                .unwrap();

            let name = config.get_raw("name").unwrap();
            assert!(name.is_some());
            assert_eq!(name.unwrap().as_str(), Some("test"));
        }

        #[test]
        fn test_read_write() {
            let config = ConfigImpl::builder()
                .defaults(HashMap::from([(
                    "host".to_string(),
                    ConfigValue::string("localhost"),
                )]))
                .build()
                .unwrap();

            // Read existing
            let host = config.get_raw("host").unwrap();
            assert!(host.is_some());

            // Write new
            config
                .set(
                    "port",
                    AnnotatedValue::new(ConfigValue::uint(5432), SourceId::new("test"), "port"),
                )
                .unwrap();

            let port = config.get_raw("port").unwrap();
            assert!(port.is_some());

            // Override existing
            config
                .set(
                    "host",
                    AnnotatedValue::new(
                        ConfigValue::string("127.0.0.1"),
                        SourceId::new("override"),
                        "host",
                    ),
                )
                .unwrap();

            let host = config.get_raw("host").unwrap();
            assert_eq!(host.unwrap().as_str(), Some("127.0.0.1"));
        }

        #[test]
        fn test_health_check() {
            let config = ConfigImpl::builder().build().unwrap();
            assert!(config.health_check().is_ok());

            config.shutdown();
            assert!(config.health_check().is_err());
        }

        #[test]
        fn test_version() {
            let config = ConfigImpl::builder().build().unwrap();
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
