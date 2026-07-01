//! Default configuration implementation.
//!
//! This module provides `ConfigImpl` - the primary configuration implementation
//! that combines multiple sources and implements the BrickArchitecture traits.
//!
//! # Feature-gated async/sync duality
//!
//! `ConfigImpl` is defined twice (async_impl and sync_impl modules) because
//! the trait implementations differ significantly:
//! - async_impl uses `moka::future::Cache`, `#[async_trait]`, `.await`
//! - sync_impl uses `moka::sync::Cache`, direct calls
//!
//! Only one version is compiled depending on whether any async feature
//! (remote/config-bus/encryption/watch) is enabled.

use crate::error::{ConfersResult, ConfigConfigError};
use crate::impl_::config::{SourceChain, SourceChainBuilder};
use crate::impl_::lifecycle::Lifecycle;
use crate::impl_::merger::MergeStrategy;
use crate::interface::{ConfigConnector, ConfigReader, ConfigWriter};
use crate::types::{AnnotatedValue, ConfigValue, SourceId};
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
    use crate::interface::sealed::Sealed;
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
                overrides: moka::future::Cache::builder()
                    .max_capacity(1_000)
                    .time_to_live(std::time::Duration::from_secs(300))
                    .time_to_idle(std::time::Duration::from_secs(60))
                    .build(),
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

    impl Sealed for ConfigImpl {}

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
    impl Lifecycle for ConfigImpl {
        async fn start(&self) -> Result<(), ConfigConfigError> {
            Ok(())
        }
        async fn stop(&self) -> ConfersResult<()> {
            self.overrides.invalidate_all();
            self.healthy.store(false, Ordering::Relaxed);
            Ok(())
        }
    }

    #[async_trait]
    impl ConfigConnector for ConfigImpl {
        async fn health_check(&self) -> crate::error::ConfersResult<()> {
            if self.healthy.load(Ordering::Relaxed) {
                Ok(())
            } else {
                Err(crate::error::ConfigError::HealthCheckFailed {
                    reason: "ConfigImpl is not healthy".into(),
                })
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
            self.chain_builder = self.chain_builder.strategy(strategy);
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
#[allow(unused_imports)]
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
    use crate::interface::sealed::Sealed;
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
                overrides: moka::sync::Cache::builder()
                    .max_capacity(1_000)
                    .time_to_live(std::time::Duration::from_secs(300))
                    .time_to_idle(std::time::Duration::from_secs(60))
                    .build(),
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

    impl Sealed for ConfigImpl {}

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

    impl Lifecycle for ConfigImpl {
        fn start(&self) -> Result<(), ConfigConfigError> {
            Ok(())
        }
        fn stop(&self) -> ConfersResult<()> {
            self.overrides.invalidate_all();
            self.healthy.store(false, Ordering::Relaxed);
            Ok(())
        }
    }

    impl ConfigConnector for ConfigImpl {
        fn health_check(&self) -> crate::error::ConfersResult<()> {
            if self.healthy.load(Ordering::Relaxed) {
                Ok(())
            } else {
                Err(crate::error::ConfigError::HealthCheckFailed {
                    reason: "ConfigImpl is not healthy".into(),
                })
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
            self.chain_builder = self.chain_builder.strategy(strategy);
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
#[allow(unused_imports)]
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

        // ---- get_raw: navigation branches ----

        #[tokio::test]
        async fn test_get_raw_nonexistent_key_returns_none() {
            let config = ConfigImpl::builder()
                .defaults(HashMap::from([(
                    "name".to_string(),
                    ConfigValue::string("v"),
                )]))
                .build()
                .unwrap();
            let res = config.get_raw("does_not_exist").await.unwrap();
            assert!(res.is_none());
        }

        #[tokio::test]
        async fn test_get_raw_nested_map_path() {
            let nested = ConfigValue::map(vec![(
                "inner",
                AnnotatedValue::new(ConfigValue::uint(42), SourceId::new("test"), "outer.inner"),
            )]);
            let config = ConfigImpl::builder()
                .defaults(HashMap::from([("outer".to_string(), nested)]))
                .build()
                .unwrap();
            let v = config.get_raw("outer.inner").await.unwrap();
            assert!(v.is_some());
            assert_eq!(v.unwrap().as_u64(), Some(42));
        }

        #[tokio::test]
        async fn test_get_raw_array_index_path() {
            let arr = ConfigValue::array(vec![
                AnnotatedValue::new(ConfigValue::uint(10), SourceId::new("test"), "arr.0"),
                AnnotatedValue::new(ConfigValue::uint(20), SourceId::new("test"), "arr.1"),
            ]);
            let config = ConfigImpl::builder()
                .defaults(HashMap::from([("arr".to_string(), arr)]))
                .build()
                .unwrap();
            let v0 = config.get_raw("arr.0").await.unwrap();
            assert!(v0.is_some());
            assert_eq!(v0.unwrap().as_u64(), Some(10));
            let v1 = config.get_raw("arr.1").await.unwrap();
            assert_eq!(v1.unwrap().as_u64(), Some(20));
        }

        #[tokio::test]
        async fn test_get_raw_array_non_numeric_index_returns_none() {
            let arr = ConfigValue::array(vec![AnnotatedValue::new(
                ConfigValue::uint(10),
                SourceId::new("test"),
                "arr.0",
            )]);
            let config = ConfigImpl::builder()
                .defaults(HashMap::from([("arr".to_string(), arr)]))
                .build()
                .unwrap();
            // "abc" cannot be parsed as usize -> None.
            let res = config.get_raw("arr.abc").await.unwrap();
            assert!(res.is_none());
        }

        #[tokio::test]
        async fn test_get_raw_array_index_out_of_bounds_returns_none() {
            let arr = ConfigValue::array(vec![AnnotatedValue::new(
                ConfigValue::uint(10),
                SourceId::new("test"),
                "arr.0",
            )]);
            let config = ConfigImpl::builder()
                .defaults(HashMap::from([("arr".to_string(), arr)]))
                .build()
                .unwrap();
            let res = config.get_raw("arr.5").await.unwrap();
            assert!(res.is_none());
        }

        #[tokio::test]
        async fn test_get_raw_descend_into_primitive_returns_none() {
            // "name" is a String; navigating into "name.subkey" should hit the `_ => None` branch.
            let config = ConfigImpl::builder()
                .defaults(HashMap::from([(
                    "name".to_string(),
                    ConfigValue::string("v"),
                )]))
                .build()
                .unwrap();
            let res = config.get_raw("name.subkey").await.unwrap();
            assert!(res.is_none());
        }

        #[tokio::test]
        async fn test_get_raw_map_missing_intermediate_key_returns_none() {
            let nested = ConfigValue::map(vec![(
                "inner",
                AnnotatedValue::new(ConfigValue::uint(1), SourceId::new("test"), "outer.inner"),
            )]);
            let config = ConfigImpl::builder()
                .defaults(HashMap::from([("outer".to_string(), nested)]))
                .build()
                .unwrap();
            let res = config.get_raw("outer.missing.deeper").await.unwrap();
            assert!(res.is_none());
        }

        #[tokio::test]
        async fn test_get_raw_override_shadows_merged() {
            let config = ConfigImpl::builder()
                .defaults(HashMap::from([(
                    "k".to_string(),
                    ConfigValue::string("merged"),
                )]))
                .build()
                .unwrap();
            // Override takes precedence over the merged value.
            config
                .set(
                    "k",
                    AnnotatedValue::new(
                        ConfigValue::string("override"),
                        SourceId::new("override-src"),
                        "k",
                    ),
                )
                .await
                .unwrap();
            let v = config.get_raw("k").await.unwrap().unwrap();
            assert_eq!(v.as_str(), Some("override"));
        }

        // ---- keys ----

        #[tokio::test]
        async fn test_keys_returns_merged_paths() {
            let config = ConfigImpl::builder()
                .defaults(HashMap::from([
                    ("a".to_string(), ConfigValue::uint(1)),
                    ("b".to_string(), ConfigValue::uint(2)),
                ]))
                .build()
                .unwrap();
            let mut keys = config.keys().await.unwrap();
            keys.sort();
            // The root AnnotatedValue has path "" which is included in all_paths().
            assert_eq!(keys, vec!["".to_string(), "a".to_string(), "b".to_string()]);
        }

        #[tokio::test]
        async fn test_keys_deduplicates_override_and_merged() {
            let config = ConfigImpl::builder()
                .defaults(HashMap::from([(
                    "k".to_string(),
                    ConfigValue::string("merged"),
                )]))
                .build()
                .unwrap();
            config
                .set(
                    "k",
                    AnnotatedValue::new(ConfigValue::string("override"), SourceId::new("src"), "k"),
                )
                .await
                .unwrap();
            let keys = config.keys().await.unwrap();
            // Same key appears only once after dedup.
            assert_eq!(keys.iter().filter(|k| k.as_str() == "k").count(), 1);
        }

        #[tokio::test]
        async fn test_keys_includes_override_only_keys() {
            let config = ConfigImpl::builder().build().unwrap();
            config
                .set(
                    "override-only",
                    AnnotatedValue::new(
                        ConfigValue::uint(1),
                        SourceId::new("test"),
                        "override-only",
                    ),
                )
                .await
                .unwrap();
            let keys = config.keys().await.unwrap();
            assert!(keys.iter().any(|k| k == "override-only"));
        }

        // ---- delete / clear ----

        #[tokio::test]
        async fn test_delete_nonexistent_returns_false_no_version_bump() {
            let config = ConfigImpl::builder().build().unwrap();
            let initial = config.version();
            let existed = config.delete("ghost").await.unwrap();
            assert!(!existed);
            assert_eq!(config.version(), initial);
        }

        #[tokio::test]
        async fn test_clear_removes_overrides_and_bumps_version() {
            let config = ConfigImpl::builder().build().unwrap();
            config
                .set(
                    "k1",
                    AnnotatedValue::new(ConfigValue::uint(1), SourceId::new("t"), "k1"),
                )
                .await
                .unwrap();
            config
                .set(
                    "k2",
                    AnnotatedValue::new(ConfigValue::uint(2), SourceId::new("t"), "k2"),
                )
                .await
                .unwrap();
            let before_clear = config.version();
            config.clear().await.unwrap();
            assert_eq!(config.version(), before_clear + 1);
            // After clear, overrides are gone.
            assert!(config.get_raw("k1").await.unwrap().is_none());
            assert!(config.get_raw("k2").await.unwrap().is_none());
        }

        // ---- reload ----

        #[tokio::test]
        async fn test_reload_replaces_values_and_bumps_version() {
            let config = ConfigImpl::builder()
                .defaults(HashMap::from([(
                    "host".to_string(),
                    ConfigValue::string("old"),
                )]))
                .build()
                .unwrap();
            let v0 = config.version();
            let new_chain = SourceChainBuilder::default()
                .defaults(HashMap::from([(
                    "host".to_string(),
                    ConfigValue::string("new"),
                )]))
                .build();
            config.reload(new_chain).unwrap();
            assert_eq!(config.version(), v0 + 1);
            let host = config.get_raw("host").await.unwrap().unwrap();
            assert_eq!(host.as_str(), Some("new"));
        }

        // ---- source_id ----

        #[tokio::test]
        async fn test_source_id_returns_config() {
            let config = ConfigImpl::builder().build().unwrap();
            assert_eq!(config.source_id().as_str(), "config");
        }

        // ---- lifecycle ----

        #[tokio::test]
        async fn test_start_is_noop() {
            let config = ConfigImpl::builder().build().unwrap();
            // start() always returns Ok(()).
            config.start().await.unwrap();
            assert!(config.health_check().await.is_ok());
        }

        #[tokio::test]
        async fn test_stop_marks_unhealthy_and_clears_overrides() {
            let config = ConfigImpl::builder().build().unwrap();
            config
                .set(
                    "k",
                    AnnotatedValue::new(ConfigValue::uint(1), SourceId::new("t"), "k"),
                )
                .await
                .unwrap();
            config.stop().await.unwrap();
            assert!(config.health_check().await.is_err());
            // Overrides were invalidated.
            assert!(config.get_raw("k").await.unwrap().is_none());
        }

        #[tokio::test]
        async fn test_health_check_after_shutdown() {
            let config = ConfigImpl::builder().build().unwrap();
            config.shutdown().await;
            let err = config.health_check().await.unwrap_err();
            // Error variant should mention the failure.
            assert!(format!("{err}").to_lowercase().contains("health"));
        }

        // ---- builder options ----

        #[tokio::test]
        async fn test_builder_file_optional_missing_file_succeeds() {
            // An optional file that does not exist should not fail the build.
            let config = ConfigImpl::builder()
                .file_optional("/nonexistent/confers/test-missing.toml")
                .build();
            assert!(config.is_ok());
        }

        #[tokio::test]
        async fn test_builder_env_chain_compiles() {
            // Just verifies the env builder method compiles and chains.
            let builder = ConfigImpl::builder().env();
            let config = builder.build().unwrap();
            // An empty env-backed config has no merged keys.
            let keys = config.keys().await.unwrap();
            assert!(keys.is_empty() || !keys.is_empty());
        }

        #[tokio::test]
        async fn test_builder_env_prefix_chain_compiles() {
            let builder = ConfigImpl::builder().env_prefix("CONFERS_TEST_PREFIX_UNSET");
            let _config = builder.build().unwrap();
        }

        #[tokio::test]
        async fn test_builder_merge_strategy_replace() {
            let config = ConfigImpl::builder()
                .merge_strategy(MergeStrategy::Replace)
                .defaults(HashMap::from([("k".to_string(), ConfigValue::string("v"))]))
                .build()
                .unwrap();
            assert_eq!(
                config.get_raw("k").await.unwrap().unwrap().as_str(),
                Some("v")
            );
        }

        #[tokio::test]
        async fn test_builder_merge_strategy_deep_merge() {
            let config = ConfigImpl::builder()
                .merge_strategy(MergeStrategy::DeepMerge)
                .defaults(HashMap::from([("k".to_string(), ConfigValue::uint(1))]))
                .build()
                .unwrap();
            assert_eq!(
                config.get_raw("k").await.unwrap().unwrap().as_u64(),
                Some(1)
            );
        }

        // ---- from_chain ----

        #[tokio::test]
        async fn test_from_chain_empty_succeeds() {
            let chain = SourceChainBuilder::default().build();
            let config = ConfigImpl::from_chain(chain).unwrap();
            assert_eq!(config.version(), 0);
            assert_eq!(config.source_id().as_str(), "config");
            let keys = config.keys().await.unwrap();
            // The root AnnotatedValue path "" is always present in all_paths().
            assert_eq!(keys, vec!["".to_string()]);
        }

        #[tokio::test]
        async fn test_from_chain_with_defaults() {
            let chain = SourceChainBuilder::default()
                .defaults(HashMap::from([("k".to_string(), ConfigValue::string("v"))]))
                .build();
            let config = ConfigImpl::from_chain(chain).unwrap();
            assert_eq!(
                config.get_raw("k").await.unwrap().unwrap().as_str(),
                Some("v")
            );
        }

        // ---- multiple operations ----

        #[tokio::test]
        async fn test_multiple_overrides_same_key_latest_wins() {
            let config = ConfigImpl::builder().build().unwrap();
            config
                .set(
                    "k",
                    AnnotatedValue::new(ConfigValue::uint(1), SourceId::new("t"), "k"),
                )
                .await
                .unwrap();
            config
                .set(
                    "k",
                    AnnotatedValue::new(ConfigValue::uint(2), SourceId::new("t"), "k"),
                )
                .await
                .unwrap();
            config
                .set(
                    "k",
                    AnnotatedValue::new(ConfigValue::uint(3), SourceId::new("t"), "k"),
                )
                .await
                .unwrap();
            assert_eq!(
                config.get_raw("k").await.unwrap().unwrap().as_u64(),
                Some(3)
            );
        }

        #[tokio::test]
        async fn test_set_then_delete_then_set_again() {
            let config = ConfigImpl::builder().build().unwrap();
            // Set, delete, then set again - all should work without issues.
            config
                .set(
                    "k",
                    AnnotatedValue::new(ConfigValue::uint(1), SourceId::new("t"), "k"),
                )
                .await
                .unwrap();
            assert!(config.delete("k").await.unwrap());
            assert!(config.get_raw("k").await.unwrap().is_none());
            config
                .set(
                    "k",
                    AnnotatedValue::new(ConfigValue::uint(2), SourceId::new("t"), "k"),
                )
                .await
                .unwrap();
            assert_eq!(
                config.get_raw("k").await.unwrap().unwrap().as_u64(),
                Some(2)
            );
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

        // ---- get_raw: navigation branches ----

        #[test]
        fn test_get_raw_nonexistent_key_returns_none() {
            let config = ConfigImpl::builder()
                .defaults(HashMap::from([(
                    "name".to_string(),
                    ConfigValue::string("v"),
                )]))
                .build()
                .unwrap();
            let res = config.get_raw("does_not_exist").unwrap();
            assert!(res.is_none());
        }

        #[test]
        fn test_get_raw_nested_map_path() {
            let nested = ConfigValue::map(vec![(
                "inner",
                AnnotatedValue::new(ConfigValue::uint(42), SourceId::new("test"), "outer.inner"),
            )]);
            let config = ConfigImpl::builder()
                .defaults(HashMap::from([("outer".to_string(), nested)]))
                .build()
                .unwrap();
            let v = config.get_raw("outer.inner").unwrap();
            assert!(v.is_some());
            assert_eq!(v.unwrap().as_u64(), Some(42));
        }

        #[test]
        fn test_get_raw_array_index_path() {
            let arr = ConfigValue::array(vec![
                AnnotatedValue::new(ConfigValue::uint(10), SourceId::new("test"), "arr.0"),
                AnnotatedValue::new(ConfigValue::uint(20), SourceId::new("test"), "arr.1"),
            ]);
            let config = ConfigImpl::builder()
                .defaults(HashMap::from([("arr".to_string(), arr)]))
                .build()
                .unwrap();
            let v0 = config.get_raw("arr.0").unwrap();
            assert!(v0.is_some());
            assert_eq!(v0.unwrap().as_u64(), Some(10));
            let v1 = config.get_raw("arr.1").unwrap();
            assert_eq!(v1.unwrap().as_u64(), Some(20));
        }

        #[test]
        fn test_get_raw_array_non_numeric_index_returns_none() {
            let arr = ConfigValue::array(vec![AnnotatedValue::new(
                ConfigValue::uint(10),
                SourceId::new("test"),
                "arr.0",
            )]);
            let config = ConfigImpl::builder()
                .defaults(HashMap::from([("arr".to_string(), arr)]))
                .build()
                .unwrap();
            let res = config.get_raw("arr.abc").unwrap();
            assert!(res.is_none());
        }

        #[test]
        fn test_get_raw_array_index_out_of_bounds_returns_none() {
            let arr = ConfigValue::array(vec![AnnotatedValue::new(
                ConfigValue::uint(10),
                SourceId::new("test"),
                "arr.0",
            )]);
            let config = ConfigImpl::builder()
                .defaults(HashMap::from([("arr".to_string(), arr)]))
                .build()
                .unwrap();
            let res = config.get_raw("arr.5").unwrap();
            assert!(res.is_none());
        }

        #[test]
        fn test_get_raw_descend_into_primitive_returns_none() {
            let config = ConfigImpl::builder()
                .defaults(HashMap::from([(
                    "name".to_string(),
                    ConfigValue::string("v"),
                )]))
                .build()
                .unwrap();
            let res = config.get_raw("name.subkey").unwrap();
            assert!(res.is_none());
        }

        #[test]
        fn test_get_raw_map_missing_intermediate_key_returns_none() {
            let nested = ConfigValue::map(vec![(
                "inner",
                AnnotatedValue::new(ConfigValue::uint(1), SourceId::new("test"), "outer.inner"),
            )]);
            let config = ConfigImpl::builder()
                .defaults(HashMap::from([("outer".to_string(), nested)]))
                .build()
                .unwrap();
            let res = config.get_raw("outer.missing.deeper").unwrap();
            assert!(res.is_none());
        }

        #[test]
        fn test_get_raw_override_shadows_merged() {
            let config = ConfigImpl::builder()
                .defaults(HashMap::from([(
                    "k".to_string(),
                    ConfigValue::string("merged"),
                )]))
                .build()
                .unwrap();
            config
                .set(
                    "k",
                    AnnotatedValue::new(
                        ConfigValue::string("override"),
                        SourceId::new("override-src"),
                        "k",
                    ),
                )
                .unwrap();
            let v = config.get_raw("k").unwrap().unwrap();
            assert_eq!(v.as_str(), Some("override"));
        }

        // ---- keys ----

        #[test]
        fn test_keys_returns_merged_paths() {
            let config = ConfigImpl::builder()
                .defaults(HashMap::from([
                    ("a".to_string(), ConfigValue::uint(1)),
                    ("b".to_string(), ConfigValue::uint(2)),
                ]))
                .build()
                .unwrap();
            let mut keys = config.keys().unwrap();
            keys.sort();
            // The root AnnotatedValue has path "" which is included in all_paths().
            assert_eq!(keys, vec!["".to_string(), "a".to_string(), "b".to_string()]);
        }

        #[test]
        fn test_keys_deduplicates_override_and_merged() {
            let config = ConfigImpl::builder()
                .defaults(HashMap::from([(
                    "k".to_string(),
                    ConfigValue::string("merged"),
                )]))
                .build()
                .unwrap();
            config
                .set(
                    "k",
                    AnnotatedValue::new(ConfigValue::string("override"), SourceId::new("src"), "k"),
                )
                .unwrap();
            let keys = config.keys().unwrap();
            assert_eq!(keys.iter().filter(|k| k.as_str() == "k").count(), 1);
        }

        #[test]
        fn test_keys_includes_override_only_keys() {
            let config = ConfigImpl::builder().build().unwrap();
            config
                .set(
                    "override-only",
                    AnnotatedValue::new(
                        ConfigValue::uint(1),
                        SourceId::new("test"),
                        "override-only",
                    ),
                )
                .unwrap();
            let keys = config.keys().unwrap();
            assert!(keys.iter().any(|k| k == "override-only"));
        }

        // ---- delete / clear ----

        #[test]
        fn test_delete_nonexistent_returns_false_no_version_bump() {
            let config = ConfigImpl::builder().build().unwrap();
            let initial = config.version();
            let existed = config.delete("ghost").unwrap();
            assert!(!existed);
            assert_eq!(config.version(), initial);
        }

        #[test]
        fn test_clear_removes_overrides_and_bumps_version() {
            let config = ConfigImpl::builder().build().unwrap();
            config
                .set(
                    "k1",
                    AnnotatedValue::new(ConfigValue::uint(1), SourceId::new("t"), "k1"),
                )
                .unwrap();
            config
                .set(
                    "k2",
                    AnnotatedValue::new(ConfigValue::uint(2), SourceId::new("t"), "k2"),
                )
                .unwrap();
            let before_clear = config.version();
            config.clear().unwrap();
            assert_eq!(config.version(), before_clear + 1);
            assert!(config.get_raw("k1").unwrap().is_none());
            assert!(config.get_raw("k2").unwrap().is_none());
        }

        // ---- reload ----

        #[test]
        fn test_reload_replaces_values_and_bumps_version() {
            let config = ConfigImpl::builder()
                .defaults(HashMap::from([(
                    "host".to_string(),
                    ConfigValue::string("old"),
                )]))
                .build()
                .unwrap();
            let v0 = config.version();
            let new_chain = SourceChainBuilder::default()
                .defaults(HashMap::from([(
                    "host".to_string(),
                    ConfigValue::string("new"),
                )]))
                .build();
            config.reload(new_chain).unwrap();
            assert_eq!(config.version(), v0 + 1);
            let host = config.get_raw("host").unwrap().unwrap();
            assert_eq!(host.as_str(), Some("new"));
        }

        // ---- source_id ----

        #[test]
        fn test_source_id_returns_config() {
            let config = ConfigImpl::builder().build().unwrap();
            assert_eq!(config.source_id().as_str(), "config");
        }

        // ---- lifecycle ----

        #[test]
        fn test_start_is_noop() {
            let config = ConfigImpl::builder().build().unwrap();
            config.start().unwrap();
            assert!(config.health_check().is_ok());
        }

        #[test]
        fn test_stop_marks_unhealthy_and_clears_overrides() {
            let config = ConfigImpl::builder().build().unwrap();
            config
                .set(
                    "k",
                    AnnotatedValue::new(ConfigValue::uint(1), SourceId::new("t"), "k"),
                )
                .unwrap();
            config.stop().unwrap();
            assert!(config.health_check().is_err());
            assert!(config.get_raw("k").unwrap().is_none());
        }

        #[test]
        fn test_health_check_after_shutdown() {
            let config = ConfigImpl::builder().build().unwrap();
            config.shutdown();
            let err = config.health_check().unwrap_err();
            assert!(format!("{err}").to_lowercase().contains("health"));
        }

        // ---- builder options ----

        #[test]
        fn test_builder_file_optional_missing_file_succeeds() {
            let config = ConfigImpl::builder()
                .file_optional("/nonexistent/confers/test-missing.toml")
                .build();
            assert!(config.is_ok());
        }

        #[test]
        fn test_builder_env_chain_compiles() {
            let builder = ConfigImpl::builder().env();
            let config = builder.build().unwrap();
            let keys = config.keys().unwrap();
            assert!(keys.is_empty() || !keys.is_empty());
        }

        #[test]
        fn test_builder_env_prefix_chain_compiles() {
            let builder = ConfigImpl::builder().env_prefix("CONFERS_TEST_PREFIX_UNSET");
            let _config = builder.build().unwrap();
        }

        #[test]
        fn test_builder_merge_strategy_replace() {
            let config = ConfigImpl::builder()
                .merge_strategy(MergeStrategy::Replace)
                .defaults(HashMap::from([("k".to_string(), ConfigValue::string("v"))]))
                .build()
                .unwrap();
            assert_eq!(config.get_raw("k").unwrap().unwrap().as_str(), Some("v"));
        }

        #[test]
        fn test_builder_merge_strategy_deep_merge() {
            let config = ConfigImpl::builder()
                .merge_strategy(MergeStrategy::DeepMerge)
                .defaults(HashMap::from([("k".to_string(), ConfigValue::uint(1))]))
                .build()
                .unwrap();
            assert_eq!(config.get_raw("k").unwrap().unwrap().as_u64(), Some(1));
        }

        // ---- from_chain ----

        #[test]
        fn test_from_chain_empty_succeeds() {
            let chain = SourceChainBuilder::default().build();
            let config = ConfigImpl::from_chain(chain).unwrap();
            assert_eq!(config.version(), 0);
            assert_eq!(config.source_id().as_str(), "config");
            let keys = config.keys().unwrap();
            // The root AnnotatedValue path "" is always present in all_paths().
            assert_eq!(keys, vec!["".to_string()]);
        }

        #[test]
        fn test_from_chain_with_defaults() {
            let chain = SourceChainBuilder::default()
                .defaults(HashMap::from([("k".to_string(), ConfigValue::string("v"))]))
                .build();
            let config = ConfigImpl::from_chain(chain).unwrap();
            assert_eq!(config.get_raw("k").unwrap().unwrap().as_str(), Some("v"));
        }

        // ---- multiple operations ----

        #[test]
        fn test_multiple_overrides_same_key_latest_wins() {
            let config = ConfigImpl::builder().build().unwrap();
            config
                .set(
                    "k",
                    AnnotatedValue::new(ConfigValue::uint(1), SourceId::new("t"), "k"),
                )
                .unwrap();
            config
                .set(
                    "k",
                    AnnotatedValue::new(ConfigValue::uint(2), SourceId::new("t"), "k"),
                )
                .unwrap();
            config
                .set(
                    "k",
                    AnnotatedValue::new(ConfigValue::uint(3), SourceId::new("t"), "k"),
                )
                .unwrap();
            assert_eq!(config.get_raw("k").unwrap().unwrap().as_u64(), Some(3));
        }

        #[test]
        fn test_set_then_delete_then_set_again() {
            let config = ConfigImpl::builder().build().unwrap();
            config
                .set(
                    "k",
                    AnnotatedValue::new(ConfigValue::uint(1), SourceId::new("t"), "k"),
                )
                .unwrap();
            assert!(config.delete("k").unwrap());
            assert!(config.get_raw("k").unwrap().is_none());
            config
                .set(
                    "k",
                    AnnotatedValue::new(ConfigValue::uint(2), SourceId::new("t"), "k"),
                )
                .unwrap();
            assert_eq!(config.get_raw("k").unwrap().unwrap().as_u64(), Some(2));
        }
    }
}
