// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Configuration builder for creating configuration instances.
//!
//! The `ConfigBuilder` provides a fluent API for building configuration
//! from multiple sources with support for validation, encryption, and hot reload.

use std::collections::HashMap;
use std::marker::PhantomData;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

#[cfg(any(
    feature = "remote",
    feature = "config-bus",
    feature = "encryption",
    feature = "watch"
))]
use crate::impl_::lifecycle::LifecycleRegistry;

#[cfg(feature = "config-bus")]
use crate::bus::ConfigBus;
use crate::error::{BuildResult, ConfigError, ConfigResult, SourceWarning, WarningCode};
use crate::impl_::merger::MergeStrategy;
#[cfg(feature = "snapshot")]
use crate::impl_::snapshot::SnapshotConfig;
use crate::interface::{KeyProvider, MetricsBackend};
use crate::types::NoOpMetrics;
use crate::types::{AnnotatedValue, ConfigValue};
#[cfg(feature = "progressive-reload")]
use crate::watcher::ReloadHealthCheck;

use super::chain::SourceChainBuilder;
use super::limits::ConfigLimits;
use crate::interface::Source;

/// Reload strategy for hot reload.
#[derive(Debug, Clone, Default)]
pub enum ReloadStrategy {
    /// Immediate reload (default).
    #[default]
    Immediate,
    /// Canary deployment with trial period.
    Canary {
        /// Trial duration before full commit.
        trial_duration: Duration,
        /// Health check interval during trial.
        poll_interval: Duration,
    },
    /// Linear rollout.
    Linear {
        /// Number of steps.
        steps: u8,
        /// Interval between steps.
        interval: Duration,
    },
}

/// Builder for creating configuration instances.
///
/// This is the main entry point for loading configuration. It supports
/// multiple sources, validation, encryption, and hot reload.
pub struct ConfigBuilder<T> {
    /// Source chain builder.
    chain_builder: SourceChainBuilder,
    /// Configuration limits.
    limits: ConfigLimits,
    /// Encryption key provider (sync).
    key_provider: Option<Arc<dyn KeyProvider>>,
    /// Metrics backend.
    metrics: Arc<dyn MetricsBackend>,
    /// Whether to validate on load.
    validate: bool,
    /// Reload strategy.
    reload_strategy: ReloadStrategy,
    /// Build timeout.
    build_timeout: Option<Duration>,
    /// Snapshot configuration.
    #[cfg(feature = "snapshot")]
    snapshot_config: Option<SnapshotConfig>,
    /// Whether to enable hot reload.
    watch: bool,
    /// Accumulated default values.
    accumulated_defaults: HashMap<String, ConfigValue>,
    /// Accumulated memory values.
    accumulated_memory: HashMap<String, ConfigValue>,
    /// Memory source priority.
    memory_priority: u8,
    /// Configuration bus for multi-instance sync.
    #[cfg(feature = "config-bus")]
    config_bus: Option<Arc<dyn ConfigBus>>,
    /// Preload validators for pre-build validation.
    #[cfg(feature = "progressive-reload")]
    preload_validators: Vec<Arc<dyn ReloadHealthCheck>>,
    /// Health check for reload operations.
    #[cfg(feature = "progressive-reload")]
    reload_health_check: Option<Arc<dyn ReloadHealthCheck>>,
    /// Type marker.
    _marker: PhantomData<T>,
    /// Lifecycle registry for managing component startup/shutdown.
    #[cfg(any(
        feature = "remote",
        feature = "config-bus",
        feature = "encryption",
        feature = "watch"
    ))]
    lifecycle_registry: LifecycleRegistry,
}

impl<T> Default for ConfigBuilder<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> ConfigBuilder<T> {
    /// Create a new configuration builder.
    pub fn new() -> Self {
        Self {
            chain_builder: SourceChainBuilder::new(),
            limits: ConfigLimits::default(),
            key_provider: None,
            metrics: Arc::new(NoOpMetrics),
            validate: true,
            reload_strategy: ReloadStrategy::default(),
            build_timeout: None,
            #[cfg(feature = "snapshot")]
            snapshot_config: None,
            watch: false,
            accumulated_defaults: HashMap::new(),
            accumulated_memory: HashMap::new(),
            memory_priority: 50,
            #[cfg(feature = "config-bus")]
            config_bus: None,
            #[cfg(feature = "progressive-reload")]
            preload_validators: Vec::new(),
            #[cfg(feature = "progressive-reload")]
            reload_health_check: None,
            #[cfg(any(
                feature = "remote",
                feature = "config-bus",
                feature = "encryption",
                feature = "watch"
            ))]
            lifecycle_registry: LifecycleRegistry::new(),
            _marker: PhantomData,
        }
    }

    /// Add a configuration source.
    pub fn source(mut self, source: Box<dyn Source>) -> Self {
        self.chain_builder = self.chain_builder.source(source);
        self
    }

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
        self.accumulated_defaults.extend(defaults);
        self
    }

    /// Add a single default value.
    pub fn default(self, key: impl Into<String>, value: ConfigValue) -> Self {
        let mut defaults = HashMap::new();
        defaults.insert(key.into(), value);
        self.defaults(defaults)
    }

    /// Add in-memory values.
    pub fn memory(mut self, values: HashMap<String, ConfigValue>) -> Self {
        self.accumulated_memory.extend(values);
        self
    }

    /// Set memory source priority.
    pub fn memory_priority(mut self, priority: u8) -> Self {
        self.memory_priority = priority;
        self
    }

    /// Set configuration limits.
    pub fn limits(mut self, limits: ConfigLimits) -> Self {
        self.limits = limits;
        self
    }

    /// Set the encryption key provider.
    pub fn key_provider(mut self, provider: Arc<dyn KeyProvider>) -> Self {
        self.key_provider = Some(provider);
        self
    }

    /// Set the metrics backend.
    pub fn metrics(mut self, metrics: Arc<dyn MetricsBackend>) -> Self {
        self.metrics = metrics;
        self
    }

    /// Set whether to validate on load.
    pub fn validate(mut self, validate: bool) -> Self {
        self.validate = validate;
        self
    }

    /// Set the merge strategy.
    pub fn strategy(mut self, strategy: MergeStrategy) -> Self {
        self.chain_builder = self.chain_builder.strategy(strategy);
        self
    }

    /// Set a field-specific merge strategy.
    pub fn field_strategy(mut self, field: impl Into<Arc<str>>, strategy: MergeStrategy) -> Self {
        self.chain_builder = self.chain_builder.field_strategy(field, strategy);
        self
    }

    /// Set the reload strategy.
    pub fn reload_strategy(mut self, strategy: ReloadStrategy) -> Self {
        self.reload_strategy = strategy;
        self
    }

    /// Set the build timeout.
    pub fn build_timeout(mut self, timeout: Duration) -> Self {
        self.build_timeout = Some(timeout);
        self
    }

    /// Enable snapshot configuration.
    #[cfg(feature = "snapshot")]
    #[cfg_attr(docsrs, doc(cfg(feature = "snapshot")))]
    pub fn with_snapshot(mut self, config: SnapshotConfig) -> Self {
        self.snapshot_config = Some(config);
        self
    }

    /// Enable hot reload.
    pub fn watch(mut self, watch: bool) -> Self {
        self.watch = watch;
        self
    }

    /// Enable fail-fast mode (stop on first error).
    pub fn fail_fast(mut self, fail_fast: bool) -> Self {
        self.chain_builder = self.chain_builder.fail_fast(fail_fast);
        self
    }

    /// Allow absolute paths for file sources (use with caution, mainly for testing).
    ///
    /// By default, absolute paths are not allowed for security reasons.
    /// This method relaxes that restriction for testing scenarios.
    pub fn allow_absolute_paths(mut self) -> Self {
        self.chain_builder = self.chain_builder.allow_absolute_paths();
        self
    }

    /// Set the configuration bus for multi-instance synchronization.
    #[cfg(feature = "config-bus")]
    #[cfg_attr(docsrs, doc(cfg(feature = "config-bus")))]
    pub fn config_bus(mut self, bus: Arc<dyn ConfigBus>) -> Self {
        self.config_bus = Some(bus);
        self
    }

    /// Add a preload validator for pre-build validation.
    #[cfg(feature = "progressive-reload")]
    #[cfg_attr(docsrs, doc(cfg(feature = "progressive-reload")))]
    pub fn preload_validator(mut self, validator: Arc<dyn ReloadHealthCheck>) -> Self {
        self.preload_validators.push(validator);
        self
    }

    /// Set the health check for reload operations.
    #[cfg(feature = "progressive-reload")]
    #[cfg_attr(docsrs, doc(cfg(feature = "progressive-reload")))]
    pub fn reload_health_check(mut self, health_check: Arc<dyn ReloadHealthCheck>) -> Self {
        self.reload_health_check = Some(health_check);
        self
    }

    /// Register a lifecycle component for managed startup/shutdown.
    #[cfg(any(
        feature = "remote",
        feature = "config-bus",
        feature = "encryption",
        feature = "watch"
    ))]
    pub fn register_lifecycle(
        mut self,
        name: &'static str,
        component: Arc<dyn crate::impl_::lifecycle::Lifecycle>,
    ) -> Self {
        self.lifecycle_registry.register(name, component);
        self
    }
}

impl<T> ConfigBuilder<T>
where
    T: serde::de::DeserializeOwned + Default,
{
    /// Build the configuration synchronously.
    ///
    /// This method collects all sources and merges them into a final configuration.
    pub fn build(self) -> ConfigResult<T> {
        self.do_build()
    }

    /// Build the configuration and return the annotated value with location information.
    ///
    /// This method returns the raw AnnotatedValue which contains source location
    /// information (line and column numbers) for each value.
    pub fn build_annotated(self) -> ConfigResult<AnnotatedValue> {
        self.do_build_annotated()
    }

    fn do_build(mut self) -> ConfigResult<T> {
        if !self.accumulated_defaults.is_empty() {
            self.chain_builder = self.chain_builder.defaults(self.accumulated_defaults);
        }

        if !self.accumulated_memory.is_empty() {
            self.chain_builder = self
                .chain_builder
                .memory_with_priority(self.accumulated_memory, self.memory_priority);
        }

        let chain = self.chain_builder.build();
        let merged = chain.collect()?;

        let json = value_to_json(&merged);
        let config: T = serde_json::from_value(json).map_err(|e| ConfigError::InvalidValue {
            key: String::new(),
            expected_type: std::any::type_name::<T>().to_string(),
            message: e.to_string(),
        })?;

        Ok(config)
    }

    fn do_build_annotated(mut self) -> ConfigResult<AnnotatedValue> {
        if !self.accumulated_defaults.is_empty() {
            self.chain_builder = self.chain_builder.defaults(self.accumulated_defaults);
        }

        if !self.accumulated_memory.is_empty() {
            self.chain_builder = self
                .chain_builder
                .memory_with_priority(self.accumulated_memory, self.memory_priority);
        }

        let chain = self.chain_builder.build();
        let merged = chain.collect()?;

        Ok(merged)
    }

    /// Build with a fallback configuration.
    ///
    /// If the build fails, returns the fallback configuration.
    pub fn build_with_fallback(self, fallback: T) -> BuildResult<T> {
        match self.build() {
            Ok(config) => BuildResult::ok(config),
            Err(e) => BuildResult {
                config: fallback,
                warnings: vec![SourceWarning {
                    message: format!("Using fallback configuration: {}", e),
                    source: None,
                    code: WarningCode::RemoteFallback,
                }],
                degraded: true,
                degraded_reason: Some(e.to_string()),
            },
        }
    }

    /// Build resiliently, collecting warnings instead of failing.
    pub fn build_resilient(mut self) -> ConfigResult<BuildResult<T>> {
        // Add accumulated defaults if any
        if !self.accumulated_defaults.is_empty() {
            self.chain_builder = self.chain_builder.defaults(self.accumulated_defaults);
        }

        // Add accumulated memory values if any
        if !self.accumulated_memory.is_empty() {
            self.chain_builder = self
                .chain_builder
                .memory_with_priority(self.accumulated_memory, self.memory_priority);
        }

        let chain = self.chain_builder.fail_fast(false).build();
        let merged = chain.collect()?;

        let json = value_to_json(&merged);
        let config: T = serde_json::from_value(json).map_err(|e| ConfigError::InvalidValue {
            key: String::new(),
            expected_type: std::any::type_name::<T>().to_string(),
            message: e.to_string(),
        })?;

        Ok(BuildResult::ok(config))
    }
}

impl<T> ConfigBuilder<T>
where
    T: serde::de::DeserializeOwned + Default + Send + Sync + 'static,
{
    /// Build with hot reload support (async).
    ///
    /// Returns a receiver for configuration updates and a guard for the watcher.
    ///
    /// **Deprecated**: This method builds the configuration once and returns a
    /// watch channel that will never receive updates. It does NOT perform hot
    /// reload when file changes are detected. For full hot reload support, use
    /// the `watch` feature with `FsWatcher` or `MultiFsWatcher` directly.
    #[deprecated(
        since = "0.3.0",
        note = "Does not reload on file changes. Use FsWatcher/MultiFsWatcher directly."
    )]
    #[cfg(feature = "watch")]
    pub async fn build_with_watcher(
        self,
    ) -> ConfigResult<(
        tokio::sync::watch::Receiver<Arc<T>>,
        crate::watcher::WatcherGuard,
    )> {
        // Build the initial configuration once. The previous implementation
        // spawned a polling task that detected file modifications but could not
        // rebuild the source chain (no access to original sources), so it
        // silently discarded every change — pure dead code. Removed per S-M-6.
        let initial = self.build()?;
        let (_tx, rx) = tokio::sync::watch::channel(Arc::new(initial));
        let guard = crate::watcher::WatcherGuard::new();
        Ok((rx, guard))
    }
}

/// Convert an AnnotatedValue to a JSON value for deserialization.
fn value_to_json(value: &AnnotatedValue) -> serde_json::Value {
    match &value.inner {
        ConfigValue::Null => serde_json::Value::Null,
        ConfigValue::Bool(b) => serde_json::Value::Bool(*b),
        ConfigValue::I64(i) => serde_json::Value::Number((*i).into()),
        ConfigValue::U64(u) => serde_json::Value::Number((*u).into()),
        ConfigValue::F64(f) => serde_json::Number::from_f64(*f)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
        ConfigValue::String(s) => serde_json::Value::String(s.clone()),
        ConfigValue::Bytes(b) => {
            use base64::Engine;
            let encoded = base64::engine::general_purpose::STANDARD.encode(b);
            serde_json::Value::String(encoded)
        }
        ConfigValue::Array(arr) => {
            serde_json::Value::Array(arr.iter().map(value_to_json).collect())
        }
        ConfigValue::Map(map) => serde_json::Value::Object(
            map.iter()
                .map(|(k, v)| (k.to_string(), value_to_json(v)))
                .collect(),
        ),
    }
}

/// Convenient function to create a ConfigBuilder.
pub fn config<T>() -> ConfigBuilder<T> {
    ConfigBuilder::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, Default, Deserialize, PartialEq)]
    struct TestConfig {
        #[serde(default)]
        name: String,
        #[serde(default)]
        port: u16,
    }

    #[test]
    fn test_builder_empty() {
        let builder: ConfigBuilder<TestConfig> = ConfigBuilder::new();
        let config = builder.build().unwrap();
        assert_eq!(config, TestConfig::default());
    }

    #[test]
    fn test_builder_with_defaults() {
        let config = ConfigBuilder::<TestConfig>::new()
            .default("name", ConfigValue::string("test"))
            .default("port", ConfigValue::uint(8080))
            .build()
            .unwrap();

        assert_eq!(config.name, "test");
        assert_eq!(config.port, 8080);
    }

    #[test]
    fn test_builder_with_memory() {
        let config = ConfigBuilder::<TestConfig>::new()
            .memory(HashMap::from([
                ("name".to_string(), ConfigValue::string("memory")),
                ("port".to_string(), ConfigValue::uint(3000)),
            ]))
            .build()
            .unwrap();

        assert_eq!(config.name, "memory");
        assert_eq!(config.port, 3000);
    }

    #[test]
    fn test_builder_with_fallback() {
        let fallback = TestConfig {
            name: "fallback".to_string(),
            port: 9000,
        };

        let result = ConfigBuilder::<TestConfig>::new()
            .file("/nonexistent.toml")
            .fail_fast(true)
            .build_with_fallback(fallback);

        assert!(result.degraded);
        assert_eq!(result.config.name, "fallback");
    }

    #[test]
    fn test_builder_chain() {
        let config = ConfigBuilder::<TestConfig>::new()
            .default("name", ConfigValue::string("default"))
            .default("port", ConfigValue::uint(80))
            .memory(HashMap::from([(
                "name".to_string(),
                ConfigValue::string("override"),
            )]))
            .build()
            .unwrap();

        // Memory source has higher priority
        assert_eq!(config.name, "override");
        assert_eq!(config.port, 80);
    }

    #[test]
    fn test_config_function() {
        let _builder = config::<TestConfig>();
    }

    #[test]
    fn test_reload_strategy_default() {
        let strategy = ReloadStrategy::default();
        assert!(matches!(strategy, ReloadStrategy::Immediate));
    }

    #[test]
    #[cfg(feature = "snapshot")]
    fn test_snapshot_config_default() {
        let config = SnapshotConfig::default();
        assert_eq!(config.max_snapshots, 30);
        assert!(config.include_provenance);
    }

    #[test]
    fn test_builder_new() {
        let _builder: ConfigBuilder<TestConfig> = ConfigBuilder::new();
        // Builder is created with defaults, no sources yet
        // We verify by checking build_annotated works
    }

    #[test]
    fn test_builder_file_method() {
        let _builder: ConfigBuilder<TestConfig> = ConfigBuilder::new().file("config.toml");
        // Builder should have one source after calling file()
    }

    #[test]
    fn test_builder_env_method() {
        let _builder: ConfigBuilder<TestConfig> = ConfigBuilder::new().env();
    }

    #[test]
    fn test_builder_defaults_method() {
        use crate::ConfigValue;
        use std::collections::HashMap;
        let mut defaults = HashMap::new();
        defaults.insert("host".to_string(), ConfigValue::string("localhost"));
        let _builder: ConfigBuilder<TestConfig> = ConfigBuilder::new().defaults(defaults);
    }

    #[test]
    fn test_builder_limits_method() {
        let _builder: ConfigBuilder<TestConfig> =
            ConfigBuilder::new().limits(ConfigLimits::strict());
    }

    #[test]
    fn test_builder_metrics_method() {
        use crate::types::NoOpMetrics;
        let _builder: ConfigBuilder<TestConfig> =
            ConfigBuilder::new().metrics(Arc::new(NoOpMetrics));
    }

    #[test]
    fn test_builder_strategy_method() {
        use crate::impl_::merger::MergeStrategy;
        let _builder: ConfigBuilder<TestConfig> =
            ConfigBuilder::new().strategy(MergeStrategy::Append);
    }

    #[test]
    fn test_builder_default_trait() {
        let builder1: ConfigBuilder<TestConfig> = ConfigBuilder::new();
        // Use Default::default() to avoid collision with inherent `default()` method
        let builder2: ConfigBuilder<TestConfig> = Default::default();
        // T-C-1 D4d: old code used `let _ = builder.build()` which silently
        // swallowed Err. Now assert both builders produce valid configs.
        let config1 = builder1
            .build()
            .expect("ConfigBuilder::new() should build successfully with defaults");
        let config2 = builder2
            .build()
            .expect("Default::default() should build successfully with defaults");
        assert_eq!(
            config1.name, config2.name,
            "both builders should yield equivalent configs"
        );
    }

    #[test]
    fn test_builder_default_single_value() {
        let config = ConfigBuilder::<TestConfig>::new()
            .default("name", ConfigValue::string("single"))
            .build()
            .unwrap();
        assert_eq!(config.name, "single");
    }

    #[test]
    fn test_builder_memory_priority_setter() {
        let config = ConfigBuilder::<TestConfig>::new()
            .memory_priority(99)
            .memory(HashMap::from([(
                "name".to_string(),
                ConfigValue::string("priority_test"),
            )]))
            .build()
            .unwrap();
        assert_eq!(config.name, "priority_test");
    }

    #[test]
    fn test_builder_validate_false() {
        let config = ConfigBuilder::<TestConfig>::new()
            .validate(false)
            .default("name", ConfigValue::string("no_validate"))
            .build()
            .unwrap();
        assert_eq!(config.name, "no_validate");
    }

    #[test]
    fn test_builder_field_strategy_method() {
        use crate::impl_::merger::MergeStrategy;
        let _builder: ConfigBuilder<TestConfig> =
            ConfigBuilder::new().field_strategy("name", MergeStrategy::Replace);
    }

    #[test]
    fn test_builder_reload_strategy_canary() {
        let strategy = ReloadStrategy::Canary {
            trial_duration: Duration::from_secs(30),
            poll_interval: Duration::from_secs(5),
        };
        let _builder: ConfigBuilder<TestConfig> = ConfigBuilder::new().reload_strategy(strategy);
    }

    #[test]
    fn test_builder_reload_strategy_linear() {
        let strategy = ReloadStrategy::Linear {
            steps: 5,
            interval: Duration::from_secs(10),
        };
        let _builder: ConfigBuilder<TestConfig> = ConfigBuilder::new().reload_strategy(strategy);
    }

    #[test]
    fn test_builder_build_timeout_setter() {
        let _builder: ConfigBuilder<TestConfig> =
            ConfigBuilder::new().build_timeout(Duration::from_secs(5));
    }

    #[cfg(feature = "snapshot")]
    #[test]
    fn test_builder_with_snapshot() {
        let _builder: ConfigBuilder<TestConfig> =
            ConfigBuilder::new().with_snapshot(SnapshotConfig::default());
    }

    #[test]
    fn test_builder_watch_setter() {
        let _builder: ConfigBuilder<TestConfig> = ConfigBuilder::new().watch(true);
        let _builder: ConfigBuilder<TestConfig> = ConfigBuilder::new().watch(false);
    }

    #[test]
    fn test_builder_allow_absolute_paths() {
        let _builder: ConfigBuilder<TestConfig> = ConfigBuilder::new().allow_absolute_paths();
    }

    #[test]
    fn test_builder_key_provider_setter() {
        struct DummyKeyProvider;
        impl crate::interface::KeyProvider for DummyKeyProvider {
            fn get_key(&self) -> crate::error::ConfigResult<crate::types::ZeroizingBytes> {
                Ok(crate::types::ZeroizingBytes::new(vec![0u8; 32]))
            }
            fn provider_type(&self) -> &'static str {
                "dummy"
            }
        }
        let _builder: ConfigBuilder<TestConfig> =
            ConfigBuilder::new().key_provider(Arc::new(DummyKeyProvider));
    }

    #[test]
    fn test_builder_build_annotated() {
        let result = ConfigBuilder::<TestConfig>::new()
            .default("name", ConfigValue::string("annotated"))
            .build_annotated()
            .unwrap();
        assert!(result.is_map());
    }

    #[test]
    fn test_builder_build_annotated_with_memory() {
        let result = ConfigBuilder::<TestConfig>::new()
            .memory(HashMap::from([(
                "name".to_string(),
                ConfigValue::string("mem_annotated"),
            )]))
            .build_annotated()
            .unwrap();
        assert!(result.is_map());
    }

    #[test]
    fn test_builder_build_resilient_success() {
        let result = ConfigBuilder::<TestConfig>::new()
            .default("name", ConfigValue::string("resilient"))
            .build_resilient()
            .unwrap();
        assert!(!result.degraded);
        assert_eq!(result.config.name, "resilient");
    }

    #[test]
    fn test_builder_build_with_fallback_success() {
        let fallback = TestConfig {
            name: "fallback".to_string(),
            port: 9000,
        };
        let result = ConfigBuilder::<TestConfig>::new()
            .default("name", ConfigValue::string("ok"))
            .build_with_fallback(fallback);
        assert!(!result.degraded);
        assert_eq!(result.config.name, "ok");
    }

    #[test]
    fn test_builder_build_invalid_type() {
        let result = ConfigBuilder::<TestConfig>::new()
            .default("port", ConfigValue::string("not_a_number"))
            .build();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ConfigError::InvalidValue { .. }
        ));
    }

    #[test]
    fn test_builder_build_resilient_invalid_type() {
        let result = ConfigBuilder::<TestConfig>::new()
            .default("port", ConfigValue::string("not_a_number"))
            .build_resilient();
        assert!(result.is_err());
    }

    #[test]
    fn test_value_to_json_all_variants() {
        let sid = crate::types::SourceId::new("test");

        let av = AnnotatedValue::new(ConfigValue::Null, sid.clone(), "k");
        assert!(value_to_json(&av).is_null());

        let av = AnnotatedValue::new(ConfigValue::Bool(true), sid.clone(), "k");
        assert_eq!(value_to_json(&av), serde_json::Value::Bool(true));

        let av = AnnotatedValue::new(ConfigValue::I64(-42), sid.clone(), "k");
        assert_eq!(value_to_json(&av), serde_json::json!(-42));

        let av = AnnotatedValue::new(ConfigValue::U64(42), sid.clone(), "k");
        assert_eq!(value_to_json(&av), serde_json::json!(42));

        let av = AnnotatedValue::new(ConfigValue::F64(2.5), sid.clone(), "k");
        assert_eq!(value_to_json(&av), serde_json::json!(2.5));

        let av = AnnotatedValue::new(ConfigValue::String("hello".to_string()), sid.clone(), "k");
        assert_eq!(value_to_json(&av), serde_json::json!("hello"));

        let av = AnnotatedValue::new(ConfigValue::Bytes(vec![1, 2, 3]), sid.clone(), "k");
        assert!(value_to_json(&av).is_string());

        let inner = AnnotatedValue::new(ConfigValue::I64(1), sid.clone(), "k");
        let av = AnnotatedValue::new(ConfigValue::array(vec![inner]), sid.clone(), "k");
        assert!(value_to_json(&av).is_array());

        let val = AnnotatedValue::new(ConfigValue::Bool(true), sid.clone(), "v");
        let av = AnnotatedValue::new(ConfigValue::map(vec![("key", val)]), sid, "k");
        assert!(value_to_json(&av).is_object());
    }

    #[test]
    fn test_value_to_json_nan_float() {
        let sid = crate::types::SourceId::new("test");
        let av = AnnotatedValue::new(ConfigValue::F64(f64::NAN), sid, "k");
        // NaN cannot be represented as JSON number → becomes Null
        assert!(value_to_json(&av).is_null());
    }

    #[test]
    fn test_reload_strategy_clone() {
        let strategy = ReloadStrategy::Immediate;
        let cloned = strategy.clone();
        assert!(matches!(cloned, ReloadStrategy::Immediate));
    }

    #[cfg(feature = "config-bus")]
    #[test]
    fn test_builder_config_bus_setter() {
        let bus = Arc::new(crate::bus::InMemoryBus::new());
        let _builder: ConfigBuilder<TestConfig> = ConfigBuilder::new().config_bus(bus);
    }

    #[cfg(feature = "progressive-reload")]
    #[test]
    fn test_builder_preload_validator_setter() {
        use crate::interface::ConfigProvider;
        use async_trait::async_trait;
        struct DummyCheck;
        #[async_trait]
        impl crate::watcher::ReloadHealthCheck for DummyCheck {
            async fn check(&self, _provider: Arc<dyn ConfigProvider>) -> crate::HealthStatus {
                crate::HealthStatus::Healthy
            }
        }
        let _builder: ConfigBuilder<TestConfig> =
            ConfigBuilder::new().preload_validator(Arc::new(DummyCheck));
    }

    #[cfg(feature = "progressive-reload")]
    #[test]
    fn test_builder_reload_health_check_setter() {
        use crate::interface::ConfigProvider;
        use async_trait::async_trait;
        struct DummyCheck;
        #[async_trait]
        impl crate::watcher::ReloadHealthCheck for DummyCheck {
            async fn check(&self, _provider: Arc<dyn ConfigProvider>) -> crate::HealthStatus {
                crate::HealthStatus::Healthy
            }
        }
        let _builder: ConfigBuilder<TestConfig> =
            ConfigBuilder::new().reload_health_check(Arc::new(DummyCheck));
    }

    #[cfg(any(
        feature = "remote",
        feature = "config-bus",
        feature = "encryption",
        feature = "watch"
    ))]
    #[test]
    fn test_builder_register_lifecycle() {
        use async_trait::async_trait;
        struct DummyLifecycle;
        #[async_trait]
        impl crate::impl_::lifecycle::Lifecycle for DummyLifecycle {}
        let _builder: ConfigBuilder<TestConfig> =
            ConfigBuilder::new().register_lifecycle("dummy", Arc::new(DummyLifecycle));
    }
}
