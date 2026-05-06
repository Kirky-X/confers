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
use crate::value::AnnotatedValue;
use std::collections::HashMap;

// ============== Sealed Trait Pattern ==============

pub(crate) mod sealed {
    pub trait Sealed {}
}

// ============== Async Traits (feature-gated, mirrors sync below) ==============

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

        let s = value
            .as_string()
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
/// use confers::traits::filter_sensitive_keys;
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

/// Caching policy for key providers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum KeyCachePolicy {
    /// Cache with time-to-live
    #[default]
    Ttl,
    /// Cache indefinitely
    Forever,
    /// Never cache
    Never,
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

/// A wrapper for bytes that zeroizes on drop.
#[derive(Debug)]
pub struct ZeroizingBytes(Vec<u8>);

impl ZeroizingBytes {
    /// Create new zeroizing bytes.
    pub fn new(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }

    /// Get a reference to the bytes.
    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }

    /// Get the length of the bytes.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl Drop for ZeroizingBytes {
    fn drop(&mut self) {
        // Zeroize the bytes on drop
        for byte in &mut self.0 {
            *byte = 0;
        }
    }
}

// ZeroizingBytes does not implement Clone to prevent bypassing memory protection.
// The Drop trait ensures sensitive data is zeroized on drop.
// Note: Cloning ZeroizingBytes would leave copies in memory that cannot be zeroized.

/// Health status for progressive reload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HealthStatus {
    /// Configuration is healthy
    Healthy,
    /// Configuration is degraded but functional
    Degraded {
        /// Reason for degraded status
        reason: String,
    },
    /// Configuration is critical and should be rolled back
    Critical {
        /// Reason for critical status
        reason: String,
    },
}

impl HealthStatus {
    /// Check if the status is healthy.
    pub fn is_healthy(&self) -> bool {
        matches!(self, HealthStatus::Healthy)
    }

    /// Check if the status requires rollback.
    pub fn requires_rollback(&self) -> bool {
        matches!(self, HealthStatus::Critical { .. })
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
pub trait MetricsBackend: Send + Sync {
    /// Increment a counter metric.
    fn counter(&self, name: &str, labels: &[(&str, &str)]);

    /// Record a histogram value.
    fn histogram(&self, name: &str, value: f64, labels: &[(&str, &str)]);
}

/// No-op metrics backend for when metrics are disabled.
#[derive(Debug, Clone, Default)]
pub struct NoOpMetrics;

impl MetricsBackend for NoOpMetrics {
    fn counter(&self, _name: &str, _labels: &[(&str, &str)]) {
        // No-op
    }

    fn histogram(&self, _name: &str, _value: f64, _labels: &[(&str, &str)]) {
        // No-op
    }
}

/// Trait for versioned configurations.
pub trait Versioned {
    /// The configuration version constant.
    const VERSION: u32;
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
        assert_eq!(KeyCachePolicy::default(), KeyCachePolicy::Ttl);
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
        let metrics = NoOpMetrics::default();
        metrics.counter("test", &[("status", "ok")]);
        metrics.histogram("duration", 1.5, &[("source", "file")]);
    }

    #[test]
    fn test_versioned() {
        struct MyConfig;
        impl Versioned for MyConfig {
            const VERSION: u32 = 2;
        }
        assert_eq!(MyConfig::VERSION, 2);
    }
}
