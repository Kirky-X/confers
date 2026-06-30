//! Core trait definitions for confers.
//!
//! Follows Interface Segregation Principle (ISP) — traits are split by responsibility.
//!
//! This module defines the core abstractions:
//! - `ConfigReader` / `ConfigWriter` / `ConfigConnector` - Read/write access (feature-gated async/sync)
//! - `ConfigProvider` - Synchronous configuration access
//! - `ConfigProviderExt` - Extension trait with convenience methods
//! - `AsyncConfigProvider` - Asynchronous configuration access
//! - `KeyProvider` - Encryption key provider
//! - `MetricsBackend` - Metrics collection interface

use crate::error::{ConfersResult, ConfigResult};
use crate::types::{AnnotatedValue, KeyCachePolicy, SourceId, SourceKind, ZeroizingBytes};
use std::collections::HashMap;
use std::path::Path;

#[cfg(feature = "progressive-reload")]
use crate::HealthStatus;

// ============== Sealed Trait Pattern ==============

pub(crate) mod sealed {
    pub trait Sealed {}
}

// ============== Async Traits (feature-gated, mirrors sync below) ==============
// NOTE: ConfigReader/ConfigWriter/ConfigConnector are intentionally defined in both
// async (feature-gated) and sync (minimal builds) variants. The feature gate selects
// which version is active at compile time — they are never both visible.

#[cfg(any(
    feature = "remote",
    feature = "config-bus",
    feature = "encryption",
    feature = "watch"
))]
pub use async_traits_impl::*;

#[cfg(any(
    feature = "remote",
    feature = "config-bus",
    feature = "encryption",
    feature = "watch"
))]
mod async_traits_impl {
    use super::sealed::Sealed;
    use super::*;
    use async_trait::async_trait;
    use std::time::Duration;

    #[async_trait]
    pub trait ConfigReader: Sealed + Send + Sync {
        async fn get_raw(&self, key: &str) -> ConfersResult<Option<AnnotatedValue>>;
        async fn keys(&self) -> ConfersResult<Vec<String>>;
        async fn has(&self, key: &str) -> ConfersResult<bool> {
            Ok(self.get_raw(key).await?.is_some())
        }
        async fn get_string(&self, key: &str) -> ConfersResult<Option<String>> {
            #[allow(deprecated)]
            Ok(self.get_raw(key).await?.and_then(|v| v.as_string()))
        }
        async fn get_i64(&self, key: &str) -> ConfersResult<Option<i64>> {
            Ok(self.get_raw(key).await?.and_then(|v| v.as_i64()))
        }
        async fn get_u64(&self, key: &str) -> ConfersResult<Option<u64>> {
            Ok(self.get_raw(key).await?.and_then(|v| v.as_u64()))
        }
        async fn get_f64(&self, key: &str) -> ConfersResult<Option<f64>> {
            Ok(self.get_raw(key).await?.and_then(|v| v.as_f64()))
        }
        async fn get_bool(&self, key: &str) -> ConfersResult<Option<bool>> {
            Ok(self.get_raw(key).await?.and_then(|v| v.as_bool()))
        }
    }

    #[async_trait]
    pub trait ConfigWriter: Sealed + Send + Sync {
        async fn set(&self, key: &str, value: AnnotatedValue) -> ConfersResult<()>;
        async fn delete(&self, key: &str) -> ConfersResult<bool>;
        async fn clear(&self) -> ConfersResult<()>;
    }

    #[async_trait]
    pub trait ConfigConnector: ConfigReader + ConfigWriter + Sealed + Send + Sync {
        async fn health_check(&self) -> crate::error::ConfersResult<()>;
        async fn shutdown(&self);
    }

    #[async_trait]
    pub trait AsyncConfigProvider: Send + Sync {
        async fn get_string_async(&self, key: &str) -> ConfigResult<Option<String>>;
        async fn get_typed_async<T>(&self, key: &str) -> ConfigResult<T>
        where
            T: std::str::FromStr + Default + Send,
            T::Err: std::fmt::Display + Send;
        async fn refresh(&self) -> ConfigResult<()>;
    }

    #[async_trait]
    pub trait AsyncKeyProvider: Send + Sync {
        async fn get_key(&self) -> ConfigResult<ZeroizingBytes>;
        fn provider_type(&self) -> &'static str;
        fn ttl(&self) -> Option<Duration> {
            None
        }
        fn cache_policy(&self) -> KeyCachePolicy {
            KeyCachePolicy::default()
        }
    }
}

// ============== Sync Traits (for minimal builds) ==============

#[cfg(not(any(
    feature = "remote",
    feature = "config-bus",
    feature = "encryption",
    feature = "watch"
)))]
pub use sync_traits::*;

#[cfg(not(any(
    feature = "remote",
    feature = "config-bus",
    feature = "encryption",
    feature = "watch"
)))]
mod sync_traits {
    use super::sealed::Sealed;
    use super::*;

    /// Configuration reader trait (sync).
    ///
    /// Provides read-only access to configuration values.
    pub trait ConfigReader: Sealed + Send + Sync {
        /// Get raw annotated value.
        fn get_raw(&self, key: &str) -> ConfersResult<Option<AnnotatedValue>>;

        /// Get all configuration keys.
        fn keys(&self) -> ConfersResult<Vec<String>>;

        /// Check if key exists.
        fn has(&self, key: &str) -> ConfersResult<bool> {
            Ok(self.get_raw(key)?.is_some())
        }

        /// Get string value.
        fn get_string(&self, key: &str) -> ConfersResult<Option<String>> {
            #[allow(deprecated)]
            Ok(self.get_raw(key)?.and_then(|v| v.as_string()))
        }

        /// Get i64 value.
        fn get_i64(&self, key: &str) -> ConfersResult<Option<i64>> {
            Ok(self.get_raw(key)?.and_then(|v| v.as_i64()))
        }

        /// Get u64 value.
        fn get_u64(&self, key: &str) -> ConfersResult<Option<u64>> {
            Ok(self.get_raw(key)?.and_then(|v| v.as_u64()))
        }

        /// Get f64 value.
        fn get_f64(&self, key: &str) -> ConfersResult<Option<f64>> {
            Ok(self.get_raw(key)?.and_then(|v| v.as_f64()))
        }

        /// Get bool value.
        fn get_bool(&self, key: &str) -> ConfersResult<Option<bool>> {
            Ok(self.get_raw(key)?.and_then(|v| v.as_bool()))
        }
    }

    /// Configuration writer trait (sync).
    ///
    /// Provides write access to configuration values.
    pub trait ConfigWriter: Sealed + Send + Sync {
        /// Set a configuration value.
        fn set(&self, key: &str, value: AnnotatedValue) -> ConfersResult<()>;

        /// Delete a configuration key.
        fn delete(&self, key: &str) -> ConfersResult<bool>;

        /// Clear all configuration.
        fn clear(&self) -> ConfersResult<()>;
    }

    /// Combined configuration connector trait (sync).
    ///
    /// Implements BrickArchitecture specification:
    /// - Inherits ConfigReader and ConfigWriter
    /// - Embeds lifecycle methods (health_check, shutdown)
    pub trait ConfigConnector: ConfigReader + ConfigWriter + Sealed + Send + Sync {
        /// Health check: verify the connector is operational.
        fn health_check(&self) -> crate::error::ConfersResult<()>;

        /// Graceful shutdown: release resources.
        fn shutdown(&self);
    }
}

/// Core trait for configuration access.
///
/// This trait provides the fundamental interface for accessing configuration
/// values. All configuration providers must implement this trait.
///
/// # Thread Safety
///
/// Implementations must be `Send + Sync` safe for concurrent access.
pub trait ConfigProvider: Send + Sync {
    /// Get a raw annotated value by key.
    ///
    /// Returns `None` if the key does not exist.
    fn get_raw(&self, key: &str) -> Option<&AnnotatedValue>;

    /// Get all non-sensitive configuration keys.
    ///
    /// Returns keys in dot-notation format (e.g., "database.host").
    /// Sensitive fields marked `#[config(sensitive = true)]` or `#[config(encrypt = "...")]`
    /// SHALL NOT appear in the returned list.
    fn keys(&self) -> Vec<String>;

    /// Check if a key exists.
    fn has(&self, key: &str) -> bool {
        self.get_raw(key).is_some()
    }

    /// Get all configuration keys including sensitive fields.
    ///
    /// Only available in debug builds. In release builds, use `keys()` instead.
    #[cfg(debug_assertions)]
    fn keys_all(&self) -> Vec<String> {
        self.keys()
    }
}

/// Extension trait with convenience methods for `ConfigProvider`.
///
/// This trait provides default implementations for type-safe accessors.
pub trait ConfigProviderExt: ConfigProvider {
    /// Get a string value by key.
    fn get_string(&self, key: &str) -> Option<String> {
        #[allow(deprecated)]
        self.get_raw(key).and_then(|v| v.as_string())
    }

    /// Get an integer value by key.
    fn get_int(&self, key: &str) -> Option<i64> {
        self.get_raw(key).and_then(|v| v.as_i64())
    }

    /// Get an unsigned integer value by key.
    fn get_uint(&self, key: &str) -> Option<u64> {
        self.get_raw(key).and_then(|v| v.as_u64())
    }

    /// Get a float value by key.
    fn get_float(&self, key: &str) -> Option<f64> {
        self.get_raw(key).and_then(|v| v.as_f64())
    }

    /// Get a boolean value by key.
    fn get_bool(&self, key: &str) -> Option<bool> {
        self.get_raw(key).and_then(|v| v.as_bool())
    }

    /// Get a typed value by key.
    ///
    /// Returns an error if the value cannot be converted to the target type.
    fn get_typed<T>(&self, key: &str) -> ConfigResult<T>
    where
        T: std::str::FromStr + Default,
        T::Err: std::fmt::Display,
    {
        let value = self
            .get_raw(key)
            .ok_or_else(|| crate::error::ConfigError::InvalidValue {
                key: key.to_string(),
                expected_type: std::any::type_name::<T>().to_string(),
                message: "key not found".to_string(),
            })?;

        let s = {
            #[allow(deprecated)]
            let s = value.as_string();
            s
        }
        .ok_or_else(|| crate::error::ConfigError::InvalidValue {
            key: key.to_string(),
            expected_type: std::any::type_name::<T>().to_string(),
            message: "value is not a string".to_string(),
        })?;

        s.parse::<T>()
            .map_err(|e| crate::error::ConfigError::InvalidValue {
                key: key.to_string(),
                expected_type: std::any::type_name::<T>().to_string(),
                message: e.to_string(),
            })
    }

    /// Get multiple values efficiently.
    ///
    /// Returns a HashMap with the requested keys. Missing keys will have `None` values.
    fn get_many<'a>(&self, keys: &[&'a str]) -> HashMap<&'a str, Option<&AnnotatedValue>> {
        keys.iter().map(|&k| (k, self.get_raw(k))).collect()
    }

    /// Get a nested value by path segments.
    fn get_by_path(&self, path: &[&str]) -> Option<&AnnotatedValue> {
        if path.is_empty() {
            return None;
        }
        let key = path.join(".");
        self.get_raw(&key)
    }
}

// Blanket implementation for all ConfigProvider types
impl<T: ConfigProvider + ?Sized> ConfigProviderExt for T {}

/// Filter sensitive keys from a list of configuration keys.
///
/// This utility can be used by `ConfigProvider` implementers to filter
/// the output of `keys()` when sensitive path information is available.
///
/// # Example
///
/// ```rust
/// use confers::interface::filter_sensitive_keys;
///
/// let all_keys = vec!["host".into(), "password".into()];
/// let sensitive = &["password"];
/// let filtered = filter_sensitive_keys(all_keys, sensitive);
/// assert_eq!(filtered, vec!["host"]);
/// ```
pub fn filter_sensitive_keys(keys: Vec<String>, sensitive_paths: &[&str]) -> Vec<String> {
    keys.into_iter()
        .filter(|key| {
            !sensitive_paths
                .iter()
                .any(|s| key == s || key.starts_with(s))
        })
        .collect()
}

/// Synchronous encryption key provider.
///
/// Implementations provide encryption keys for sensitive field encryption.
pub trait KeyProvider: Send + Sync {
    /// Get the encryption key.
    ///
    /// Returns the key as secret bytes that should be zeroized after use.
    fn get_key(&self) -> ConfigResult<ZeroizingBytes>;

    /// Get the provider type name for logging.
    fn provider_type(&self) -> &'static str;

    /// Get the cache policy for this provider.
    fn cache_policy(&self) -> KeyCachePolicy {
        KeyCachePolicy::default()
    }
}

/// Health check trait for progressive reload.
#[cfg(feature = "progressive-reload")]
#[cfg_attr(docsrs, doc(cfg(feature = "progressive-reload")))]
#[async_trait::async_trait]
pub trait ReloadHealthCheck: Send + Sync {
    /// Perform a health check on the configuration.
    async fn check(&self) -> HealthStatus;
}

/// Metrics backend for collecting configuration metrics.
///
/// Public extension point for integrating custom metrics systems.
/// Not used by the library itself — provided for downstream consumers.
pub trait MetricsBackend: Send + Sync {
    /// Increment a counter metric.
    fn counter(&self, name: &str, labels: &[(&str, &str)]);

    /// Record a histogram value.
    fn histogram(&self, name: &str, value: f64, labels: &[(&str, &str)]);
}

/// Trait for versioned configurations.
pub trait Versioned {
    /// The configuration version constant.
    const VERSION: u32;
}

// ============== Source traits (migrated from config/source.rs) ==============

/// Trait for configuration sources.
///
/// Sources are responsible for collecting configuration values.
/// They are combined in a `SourceChain` with priority ordering.
pub trait Source: Send + Sync {
    /// Collect configuration values from this source.
    fn collect(&self) -> ConfigResult<AnnotatedValue>;

    /// Get the priority of this source (higher = more important).
    fn priority(&self) -> u8;

    /// Get the name of this source for debugging.
    fn name(&self) -> &str;

    /// Get the kind of this source.
    fn source_kind(&self) -> SourceKind;

    /// Check if this source is optional (errors are non-fatal).
    fn is_optional(&self) -> bool {
        false
    }

    /// Get the file path if this is a file source.
    fn file_path(&self) -> Option<&Path> {
        None
    }
}

/// Trait for asynchronous configuration sources.
///
/// This trait is used for remote sources that require async I/O,
/// such as HTTP endpoints, etcd, Consul, etc.
#[cfg(feature = "remote")]
#[async_trait::async_trait]
pub trait AsyncSource: Send + Sync {
    /// Load configuration values from this source asynchronously.
    async fn load(&self) -> ConfigResult<AnnotatedValue>;

    /// Get the source ID for tracking.
    fn source_id(&self) -> &SourceId;

    /// Get the priority of this source (higher = more important).
    fn priority(&self) -> u8 {
        50
    }

    /// Get the name of this source for debugging.
    fn name(&self) -> &str;
}

/// Context-aware configuration provider.
///
/// This trait provides context-aware value resolution, allowing values
/// to be dynamically computed based on runtime context (e.g., tenant, environment).
#[cfg(feature = "context-aware")]
#[cfg_attr(docsrs, doc(cfg(feature = "context-aware")))]
pub trait ContextAwareProvider: Send + Sync {
    /// The context type used for resolution.
    type Context: Clone + Send + Sync;

    /// Get a value with context.
    fn get_with_context(&self, key: &str, context: &Self::Context) -> Option<AnnotatedValue>;

    /// Get all keys that are context-dependent.
    fn context_dependent_keys(&self) -> Vec<String>;

    /// Check if a key requires context for resolution.
    fn requires_context(&self, key: &str) -> bool;
}

/// Preload validator for configuration validation before build.
///
/// This trait allows validation hooks to run before the configuration
/// is fully built, enabling early detection of configuration issues.
#[cfg(feature = "progressive-reload")]
#[cfg_attr(docsrs, doc(cfg(feature = "progressive-reload")))]
#[async_trait::async_trait]
pub trait AsyncPreloadValidator: Send + Sync {
    /// Validate configuration before it is committed.
    ///
    /// Returns `Ok(())` if validation passes, or an error with details.
    async fn validate(&self, config: &impl ConfigProvider) -> ConfigResult<()>;

    /// Get the validator name for logging.
    fn name(&self) -> &'static str;

    /// Get the priority of this validator (lower = higher priority).
    fn priority(&self) -> u8 {
        100
    }

    /// Whether this validator can be skipped on error.
    fn is_optional(&self) -> bool {
        false
    }
}

/// Type-safe configuration key.
///
/// Binds a configuration path to a specific type for compile-time safety.
#[derive(Debug, Clone)]
pub struct TypedConfigKey<T> {
    /// The configuration path in dot notation
    path: &'static str,
    /// Optional description for documentation
    description: Option<&'static str>,
    /// Phantom data for type binding
    _marker: std::marker::PhantomData<T>,
}

impl<T> TypedConfigKey<T> {
    /// Create a new typed configuration key.
    pub const fn new(path: &'static str) -> Self {
        Self {
            path,
            description: None,
            _marker: std::marker::PhantomData,
        }
    }

    /// Add a description to the key.
    pub const fn with_description(mut self, description: &'static str) -> Self {
        self.description = Some(description);
        self
    }

    /// Get the configuration path.
    pub fn path(&self) -> &'static str {
        self.path
    }

    /// Get the description.
    pub fn description(&self) -> Option<&'static str> {
        self.description
    }
}

impl<T: Clone + Send + Sync + 'static> TypedConfigKey<T> {
    /// Get the value from a provider.
    pub fn get<'a>(&self, provider: &'a impl ConfigProvider) -> Option<&'a AnnotatedValue> {
        provider.get_raw(self.path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_typed_config_key() {
        static DB_HOST: TypedConfigKey<String> =
            TypedConfigKey::new("database.host").with_description("Database hostname");

        assert_eq!(DB_HOST.path(), "database.host");
        assert_eq!(DB_HOST.description(), Some("Database hostname"));
    }

    #[test]
    #[cfg(feature = "progressive-reload")]
    fn test_health_status() {
        let healthy = HealthStatus::Healthy;
        assert!(healthy.is_healthy());
        assert!(!healthy.requires_rollback());

        let critical = HealthStatus::Critical {
            reason: "test".to_string(),
        };
        assert!(!critical.is_healthy());
        assert!(critical.requires_rollback());
    }

    #[test]
    fn test_key_cache_policy() {
        assert_eq!(
            KeyCachePolicy::default(),
            KeyCachePolicy::CacheWithTtl(std::time::Duration::from_secs(3600))
        );
    }

    #[test]
    fn test_zeroizing_bytes() {
        let bytes = ZeroizingBytes::new(vec![1, 2, 3, 4]);
        assert_eq!(bytes.len(), 4);
        assert_eq!(bytes.as_slice(), &[1, 2, 3, 4]);
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_noop_metrics() {
        use crate::types::NoOpMetrics;
        let metrics = NoOpMetrics;
        metrics.counter("test", &[("status", "ok")]);
        metrics.histogram("duration", 1.5, &[("source", "file")]);
    }

    #[cfg(feature = "migration")]
    #[test]
    fn test_versioned() {
        use crate::migration::Versioned;
        struct MyConfig;
        impl Versioned for MyConfig {
            const VERSION: u32 = 2;
        }
        assert_eq!(MyConfig::VERSION, 2);
    }

    #[test]
    fn test_filter_sensitive_keys_exact_match() {
        let keys = vec!["host".into(), "password".into(), "port".into()];
        let sensitive = &["password"];
        let filtered = filter_sensitive_keys(keys, sensitive);
        assert_eq!(filtered, vec!["host", "port"]);
    }

    #[test]
    fn test_filter_sensitive_keys_prefix_match() {
        let keys = vec!["db.host".into(), "db.password".into(), "db.port".into()];
        let sensitive = &["db.password"];
        let filtered = filter_sensitive_keys(keys, sensitive);
        assert_eq!(filtered, vec!["db.host", "db.port"]);
    }

    #[test]
    fn test_filter_sensitive_keys_nested_path() {
        let keys = vec![
            "server.host".into(),
            "server.tls.key".into(),
            "server.tls.cert".into(),
            "server.port".into(),
        ];
        let sensitive = &["server.tls"];
        let filtered = filter_sensitive_keys(keys, sensitive);
        assert_eq!(filtered, vec!["server.host", "server.port"]);
    }

    #[test]
    fn test_filter_sensitive_keys_no_match() {
        let keys = vec!["host".into(), "port".into()];
        let sensitive = &["password"];
        let filtered = filter_sensitive_keys(keys, sensitive);
        assert_eq!(filtered, vec!["host", "port"]);
    }

    #[test]
    fn test_filter_sensitive_keys_empty() {
        let keys: Vec<String> = vec![];
        let sensitive = &["password"];
        let filtered = filter_sensitive_keys(keys, sensitive);
        assert!(filtered.is_empty());
    }

    #[test]
    fn test_config_provider_ext_get_typed_not_found() {
        use crate::types::{AnnotatedValue, ConfigValue, SourceId};
        use std::collections::HashMap;

        struct SimpleProvider(HashMap<String, AnnotatedValue>);
        impl ConfigProvider for SimpleProvider {
            fn get_raw(&self, key: &str) -> Option<&AnnotatedValue> {
                self.0.get(key)
            }
            fn keys(&self) -> Vec<String> {
                self.0.keys().cloned().collect()
            }
        }

        let mut map = HashMap::new();
        map.insert(
            "host".into(),
            AnnotatedValue::new(
                ConfigValue::string("localhost"),
                SourceId::new("test"),
                "host",
            ),
        );
        let provider = SimpleProvider(map);

        let result: Result<String, crate::error::ConfigError> = provider.get_typed("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_config_provider_ext_get_many() {
        use crate::types::{AnnotatedValue, ConfigValue, SourceId};
        use std::collections::HashMap;

        struct SimpleProvider(HashMap<String, AnnotatedValue>);
        impl ConfigProvider for SimpleProvider {
            fn get_raw(&self, key: &str) -> Option<&AnnotatedValue> {
                self.0.get(key)
            }
            fn keys(&self) -> Vec<String> {
                self.0.keys().cloned().collect()
            }
        }

        let mut map = HashMap::new();
        map.insert(
            "a".into(),
            AnnotatedValue::new(ConfigValue::string("1"), SourceId::new("test"), "a"),
        );
        map.insert(
            "b".into(),
            AnnotatedValue::new(ConfigValue::string("2"), SourceId::new("test"), "b"),
        );
        let provider = SimpleProvider(map);

        let result = provider.get_many(&["a", "c"]);
        assert!(result.get("a").unwrap().is_some());
        assert!(result.get("c").unwrap().is_none());
    }

    #[test]
    fn test_config_provider_ext_get_by_path_joins_segments() {
        use crate::types::{AnnotatedValue, ConfigValue, SourceId};
        use std::collections::HashMap;
        struct FlatProvider(HashMap<String, AnnotatedValue>);
        impl ConfigProvider for FlatProvider {
            fn get_raw(&self, key: &str) -> Option<&AnnotatedValue> {
                self.0.get(key)
            }
            fn keys(&self) -> Vec<String> {
                self.0.keys().cloned().collect()
            }
        }
        let mut map = HashMap::new();
        map.insert(
            "db.host".into(),
            AnnotatedValue::new(
                ConfigValue::string("localhost"),
                SourceId::new("test"),
                "db.host",
            ),
        );
        let provider = FlatProvider(map);
        let result = provider.get_by_path(&["db", "host"]);
        assert!(result.is_some());
        assert_eq!(result.unwrap().as_str(), Some("localhost"));
    }

    #[test]
    fn test_config_provider_ext_has() {
        use crate::types::{AnnotatedValue, ConfigValue, SourceId};
        use std::collections::HashMap;
        struct P(HashMap<String, AnnotatedValue>);
        impl ConfigProvider for P {
            fn get_raw(&self, key: &str) -> Option<&AnnotatedValue> {
                self.0.get(key)
            }
            fn keys(&self) -> Vec<String> {
                self.0.keys().cloned().collect()
            }
        }
        let mut map = HashMap::new();
        map.insert(
            "x".into(),
            AnnotatedValue::new(ConfigValue::string("1"), SourceId::new("t"), "x"),
        );
        let p = P(map);
        assert!(p.has("x"));
        assert!(!p.has("y"));
    }

    #[test]
    fn test_config_provider_ext_get_typed_success() {
        use crate::types::{AnnotatedValue, ConfigValue, SourceId};
        use std::collections::HashMap;
        struct P(HashMap<String, AnnotatedValue>);
        impl ConfigProvider for P {
            fn get_raw(&self, key: &str) -> Option<&AnnotatedValue> {
                self.0.get(key)
            }
            fn keys(&self) -> Vec<String> {
                self.0.keys().cloned().collect()
            }
        }
        let mut map = HashMap::new();
        map.insert(
            "port".into(),
            AnnotatedValue::new(ConfigValue::string("8080"), SourceId::new("t"), "port"),
        );
        let p = P(map);
        let result: Result<u16, crate::error::ConfigError> = p.get_typed("port");
        assert_eq!(result.unwrap(), 8080);
    }

    #[test]
    fn test_config_provider_ext_get_string() {
        use crate::types::{AnnotatedValue, ConfigValue, SourceId};
        use std::collections::HashMap;
        struct P(HashMap<String, AnnotatedValue>);
        impl ConfigProvider for P {
            fn get_raw(&self, key: &str) -> Option<&AnnotatedValue> {
                self.0.get(key)
            }
            fn keys(&self) -> Vec<String> {
                self.0.keys().cloned().collect()
            }
        }
        let mut map = HashMap::new();
        map.insert(
            "host".into(),
            AnnotatedValue::new(ConfigValue::string("localhost"), SourceId::new("t"), "host"),
        );
        let p = P(map);
        assert_eq!(p.get_string("host"), Some("localhost".into()));
        assert_eq!(p.get_string("missing"), None);
    }

    #[test]
    fn test_config_provider_ext_get_int() {
        use crate::types::{AnnotatedValue, ConfigValue, SourceId};
        use std::collections::HashMap;
        struct P(HashMap<String, AnnotatedValue>);
        impl ConfigProvider for P {
            fn get_raw(&self, key: &str) -> Option<&AnnotatedValue> {
                self.0.get(key)
            }
            fn keys(&self) -> Vec<String> {
                self.0.keys().cloned().collect()
            }
        }
        let mut map = HashMap::new();
        map.insert(
            "count".into(),
            AnnotatedValue::new(ConfigValue::integer(42), SourceId::new("t"), "count"),
        );
        let p = P(map);
        assert_eq!(p.get_int("count"), Some(42));
        assert_eq!(p.get_uint("count"), Some(42));
        assert_eq!(p.get_float("count"), Some(42.0));
        assert_eq!(p.get_bool("count"), None);
    }
}
