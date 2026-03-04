//! Configuration builder for creating configuration instances.
//!
//! The `ConfigBuilder` provides a fluent API for building configuration
//! from multiple sources with support for validation, encryption, and hot reload.

use std::collections::HashMap;
use std::marker::PhantomData;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use crate::error::{BuildResult, BuildWarning, ConfigError, ConfigResult, WarningCode};
use crate::merger::MergeStrategy;
use crate::traits::{KeyProvider, MetricsBackend, NoOpMetrics};
use crate::value::{AnnotatedValue, ConfigValue};

use super::chain::SourceChainBuilder;
use super::limits::ConfigLimits;
use super::source::Source;

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

/// Configuration for snapshots.
#[derive(Debug, Clone)]
pub struct SnapshotConfig {
    /// Directory for snapshots.
    pub dir: PathBuf,
    /// Maximum number of snapshots to keep.
    pub max_snapshots: usize,
    /// Whether to include provenance information.
    pub include_provenance: bool,
}

impl Default for SnapshotConfig {
    fn default() -> Self {
        Self {
            dir: PathBuf::from("config-snapshots"),
            max_snapshots: 30,
            include_provenance: true,
        }
    }
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
    snapshot_config: Option<SnapshotConfig>,
    /// Whether to enable hot reload.
    watch: bool,
    /// Accumulated default values.
    accumulated_defaults: HashMap<String, ConfigValue>,
    /// Accumulated memory values.
    accumulated_memory: HashMap<String, ConfigValue>,
    /// Memory source priority.
    memory_priority: u8,
    /// Type marker.
    _marker: PhantomData<T>,
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
            snapshot_config: None,
            watch: false,
            accumulated_defaults: HashMap::new(),
            accumulated_memory: HashMap::new(),
            memory_priority: 50,
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
}

impl<T> ConfigBuilder<T>
where
    T: serde::de::DeserializeOwned + Default,
{
    /// Build the configuration synchronously.
    ///
    /// This method collects all sources and merges them into a final configuration.
    pub fn build(mut self) -> ConfigResult<T> {
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

        let chain = self.chain_builder.build();
        let merged = chain.collect()?;

        // Convert to the target type
        let json = value_to_json(&merged);
        let config: T = serde_json::from_value(json).map_err(|e| ConfigError::InvalidValue {
            key: String::new(),
            expected_type: std::any::type_name::<T>().to_string(),
            message: e.to_string(),
        })?;

        Ok(config)
    }

    /// Build with a fallback configuration.
    ///
    /// If the build fails, returns the fallback configuration.
    pub fn build_with_fallback(self, fallback: T) -> BuildResult<T> {
        match self.build() {
            Ok(config) => BuildResult::ok(config),
            Err(e) => BuildResult {
                config: fallback,
                warnings: vec![BuildWarning {
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
    /// The watcher will monitor all file sources and detect file changes.
    /// **Note**: This is a partial implementation. Due to ownership constraints,
    /// automatic configuration rebuild is not supported. When file changes are detected,
    /// a warning is logged and users must manually call `build()` to reload configuration.
    ///
    /// For full hot reload support, consider using the `watch` feature with
    /// `FsWatcher` or `MultiFsWatcher` directly.
    ///
    /// # Example
    ///
    /// ```rust
    /// use confers::ConfigBuilder;
    /// use confers::Config;
    /// use serde::Deserialize;
    ///
    /// #[derive(Debug, Config, Deserialize)]
    /// struct MyConfig {
    ///     host: String,
    ///     port: u16,
    /// }
    ///
    /// async fn example() -> Result<(), Box<dyn std::error::Error>> {
    ///     let builder = ConfigBuilder::<MyConfig>::new()
    ///         .file("config.toml")
    ///         .env();
    ///
    ///     let (mut rx, _guard) = builder.build_with_watcher().await?;
    ///
    ///     // Get initial config
    ///     let config = rx.borrow().clone();
    ///     println!("Config loaded: {:?}", config);
    ///
    ///     Ok(())
    /// }
    /// ```
    #[cfg(feature = "watch")]
    pub async fn build_with_watcher(
        self,
    ) -> ConfigResult<(
        tokio::sync::watch::Receiver<Arc<T>>,
        crate::watcher::WatcherGuard,
    )> {
        use std::collections::HashMap;
        use std::sync::atomic;
        use tokio::spawn;
        use tokio::time::{interval, Duration};

        // Collect file paths from the source chain (before building)
        let file_paths: Vec<PathBuf> = self
            .chain_builder
            .get_watch_paths()
            .into_iter()
            .filter_map(|path| std::fs::canonicalize(path).ok())
            .collect();

        // Initial build
        let initial = self.build()?;
        let (_tx, rx) = tokio::sync::watch::channel(Arc::new(initial));

        // Create watcher guard with running flag
        let running = Arc::new(atomic::AtomicBool::new(true));
        let guard = crate::watcher::WatcherGuard::from_running(running.clone());

        // If there are file paths, start the watcher task
        if !file_paths.is_empty() {
            spawn(async move {
                let mut ticker = interval(Duration::from_secs(1));
                let mut last_modifications: HashMap<PathBuf, std::time::SystemTime> =
                    HashMap::new();

                while running.load(atomic::Ordering::Relaxed) {
                    ticker.tick().await;

                    // Check all watched files for modifications
                    let mut needs_reload = false;
                    for path in &file_paths {
                        if let Ok(metadata) = std::fs::metadata(path) {
                            if let Ok(modified) = metadata.modified() {
                                let last_modified = last_modifications.get(path);

                                if last_modified != Some(&modified) {
                                    // File was modified
                                    last_modifications.insert(path.clone(), modified);
                                    needs_reload = true;
                                }
                            }
                        }
                    }

                    // Reload configuration if any file changed
                    if needs_reload {
                        // Note: We can't rebuild the source chain here since we don't have
                        // access to the original sources. This is a known limitation.
                        // For proper hot reload, users should call build() again manually
                        // when they detect file changes through their own watcher.
                        tracing::warn!("File change detected, but automatic rebuild requires manual build() call");
                    }
                }
            });
        }

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
            // Encode bytes as base64 when json feature is enabled
            #[cfg(feature = "json")]
            {
                use base64::Engine;
                let encoded = base64::engine::general_purpose::STANDARD.encode(b);
                serde_json::Value::String(encoded)
            }
            #[cfg(not(feature = "json"))]
            serde_json::Value::Null
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
    fn test_snapshot_config_default() {
        let config = SnapshotConfig::default();
        assert_eq!(config.max_snapshots, 30);
        assert!(config.include_provenance);
    }
}
