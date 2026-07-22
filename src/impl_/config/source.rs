//! Built-in configuration source implementations.
//!
//! The `Source` and `AsyncSource` traits are defined in `crate::interface`.
//! The `SourceKind` enum is defined in `crate::types`.
//! This module provides concrete implementations: FileSource, EnvSource,
//! MemorySource, DefaultSource.

use crate::error::{ConfigError, ConfigResult};
use crate::impl_::loader::{self, Format};
use crate::interface::Source;
use crate::types::{AnnotatedValue, ConfigValue, SourceId, SourceKind};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// File-based configuration source.
#[derive(Debug)]
pub struct FileSource {
    /// Path to the configuration file.
    path: PathBuf,
    /// Format of the file (auto-detected if None).
    format: Option<Format>,
    /// Priority of this source.
    priority: u8,
    /// Whether this source is optional.
    optional: bool,
    /// Source ID for tracking.
    source_id: SourceId,
    /// Loader configuration for security settings.
    loader_config: loader::LoaderConfig,
}

impl FileSource {
    /// Create a new file source.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        let source_id = SourceId::new(path.file_name().and_then(|n| n.to_str()).unwrap_or("file"));
        Self {
            path,
            format: None,
            priority: 0,
            optional: false,
            source_id,
            loader_config: loader::LoaderConfig::default(),
        }
    }

    /// Set the format explicitly.
    pub fn with_format(mut self, format: Format) -> Self {
        self.format = Some(format);
        self
    }

    /// Set the priority.
    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }

    /// Make this source optional.
    pub fn optional(mut self) -> Self {
        self.optional = true;
        self
    }

    /// Allow absolute paths (use with caution, mainly for testing).
    pub fn allow_absolute_paths(mut self) -> Self {
        self.loader_config = self.loader_config.allow_absolute();
        self
    }

    /// Set custom loader configuration.
    pub fn with_loader_config(mut self, config: loader::LoaderConfig) -> Self {
        self.loader_config = config;
        self
    }

    /// Get the file path.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Source for FileSource {
    fn collect(&self) -> ConfigResult<AnnotatedValue> {
        if !self.path.exists() {
            if self.optional {
                return Ok(AnnotatedValue::new(
                    ConfigValue::Map(std::sync::Arc::new(indexmap::IndexMap::new())),
                    self.source_id.clone(),
                    "",
                ));
            }
            return Err(ConfigError::FileNotFound {
                filename: self.path.clone(),
                source: None,
            });
        }

        loader::load_file(&self.path, &self.loader_config).map(|v| v.with_priority(self.priority))
    }

    fn priority(&self) -> u8 {
        self.priority
    }

    fn name(&self) -> &str {
        self.path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("file")
    }

    fn source_kind(&self) -> SourceKind {
        SourceKind::File
    }

    fn is_optional(&self) -> bool {
        self.optional
    }

    fn file_path(&self) -> Option<&Path> {
        Some(&self.path)
    }
}

/// Environment variable configuration source.
#[derive(Debug)]
pub struct EnvSource {
    /// Prefix for environment variables.
    prefix: Option<String>,
    /// Separator for nested keys.
    separator: String,
    /// Priority of this source.
    priority: u8,
    /// Source ID for tracking.
    source_id: SourceId,
    /// Whether to handle _FILE suffix for Docker secrets.
    file_suffix_enabled: bool,
    /// The file suffix for Docker secrets convention (default: "_FILE").
    file_suffix: &'static str,
}

impl EnvSource {
    /// Create a new environment source.
    pub fn new() -> Self {
        Self {
            prefix: None,
            separator: "_".to_string(),
            priority: 50,
            source_id: SourceId::new("env"),
            file_suffix_enabled: true,
            file_suffix: "_FILE",
        }
    }

    /// Create an environment source with a prefix.
    pub fn with_prefix(prefix: impl Into<String>) -> Self {
        Self {
            prefix: Some(prefix.into()),
            separator: "_".to_string(),
            priority: 50,
            source_id: SourceId::new("env"),
            file_suffix_enabled: true,
            file_suffix: "_FILE",
        }
    }

    /// Set the separator for nested keys.
    pub fn separator(mut self, separator: impl Into<String>) -> Self {
        self.separator = separator.into();
        self
    }

    /// Set the priority.
    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }

    /// Enable or disable _FILE suffix handling.
    pub fn with_file_suffix(mut self, enabled: bool) -> Self {
        self.file_suffix_enabled = enabled;
        self
    }

    /// Set a custom file suffix (default: "_FILE").
    pub fn file_suffix(mut self, suffix: &'static str) -> Self {
        self.file_suffix = suffix;
        self
    }

    /// Parse an environment variable name into a config path.
    fn parse_key(&self, env_key: &str) -> Option<String> {
        let key = if let Some(ref prefix) = self.prefix {
            if !env_key.starts_with(prefix) {
                return None;
            }
            &env_key[prefix.len()..]
        } else {
            env_key
        };

        // Handle _FILE suffix for Docker secrets convention:
        // Only active when a prefix is configured to avoid picking up
        // system env vars like CARGO_PKG_LICENSE_FILE.
        // MY_PASSWORD_FILE=/run/secrets/pass → key=my_password, reads file content
        let actual_key =
            if self.file_suffix_enabled && self.prefix.is_some() && key.ends_with(self.file_suffix)
            {
                &key[..key.len() - self.file_suffix.len()]
            } else if self.file_suffix_enabled
                && self.prefix.is_none()
                && key.ends_with(self.file_suffix)
            {
                // Without a prefix, skip _FILE vars to avoid accidentally
                // picking up CARGO_*, RUST_*, etc. vars ending with _FILE
                return None;
            } else {
                key
            };

        // Convert UPPER_SNAKE_CASE to lower.snake.case
        Some(actual_key.to_lowercase().replace(&self.separator, "."))
    }

    /// Resolve the value, handling _FILE suffix mode for Docker secrets.
    ///
    /// When `env_key` ends with `_FILE`, the `raw` value is treated as a file path,
    /// validated for security, and its contents are read instead.
    fn resolve_value(&self, raw: &str, env_key: &str) -> ConfigResult<String> {
        if self.file_suffix_enabled && env_key.ends_with(self.file_suffix) {
            // Docker secrets convention: value is a file path, read its content
            self.validate_file_path(raw)?;
            std::fs::read_to_string(raw).map_err(|_| ConfigError::InvalidValue {
                key: raw.to_string(),
                expected_type: "readable file".to_string(),
                message: format!("Cannot read file referenced by {}", env_key),
            })
        } else {
            Ok(raw.to_string())
        }
    }

    /// Validate file path for security (prevent path traversal).
    fn validate_file_path(&self, file_path: &str) -> ConfigResult<()> {
        // Skip empty file paths
        if file_path.is_empty() {
            return Ok(());
        }

        let path = Path::new(file_path);

        // Check for path traversal attempts
        let canonical = std::fs::canonicalize(path).map_err(|_| ConfigError::FileNotFound {
            filename: path.to_path_buf(),
            source: Some(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Cannot resolve file path",
            )),
        })?;

        // Block access to sensitive paths
        let sensitive_prefixes = [
            std::path::Path::new("/etc/shadow"),
            std::path::Path::new("/etc/passwd"),
            std::path::Path::new("/root"),
            std::path::Path::new("/home"),
        ];

        for prefix in &sensitive_prefixes {
            if canonical.starts_with(prefix) {
                return Err(ConfigError::InvalidValue {
                    key: "file_path".to_string(),
                    expected_type: "safe file path".to_string(),
                    message: format!("Access to {:?} is not allowed", prefix),
                });
            }
        }

        // Only allow reading regular files
        if !canonical.is_file() {
            return Err(ConfigError::InvalidValue {
                key: "file_path".to_string(),
                expected_type: "regular file".to_string(),
                message: "Only regular files can be read".to_string(),
            });
        }

        // Only allow specific extensions for security
        if let Some(ext) = canonical.extension() {
            let allowed = [
                "txt", "json", "yaml", "yml", "toml", "ini", "env", "secret", "key", "pem", "crt",
            ];
            if !allowed
                .iter()
                .any(|&e| ext.to_str().is_some_and(|s| s.eq_ignore_ascii_case(e)))
            {
                return Err(ConfigError::InvalidValue {
                    key: "file_path".to_string(),
                    expected_type: "allowed extension".to_string(),
                    message: format!("File extension {:?} is not allowed", ext),
                });
            }
        }

        Ok(())
    }
}

impl Default for EnvSource {
    fn default() -> Self {
        Self::new()
    }
}

impl Source for EnvSource {
    fn collect(&self) -> ConfigResult<AnnotatedValue> {
        let mut map = indexmap::IndexMap::new();

        // Load .env file entries first (lower priority) if env feature is enabled
        #[cfg(feature = "env")]
        {
            if let Ok(iter) = dotenvy::dotenv_iter() {
                for item in iter.flatten() {
                    if let Some(config_path) = self.parse_key(&item.0) {
                        let resolved = self.resolve_value(&item.1, &item.0)?;
                        let value = AnnotatedValue::new(
                            Self::infer_config_value(&resolved),
                            self.source_id.clone(),
                            std::sync::Arc::from(config_path.as_str()),
                        )
                        .with_priority(self.priority.saturating_sub(10));
                        let parts: Vec<&str> = config_path.split('.').collect();
                        Self::insert_nested(&mut map, &parts, value);
                    }
                }
            }
        }

        // Process real environment variables (higher priority, override .env)
        for (key, value) in std::env::vars() {
            if let Some(config_path) = self.parse_key(&key) {
                let resolved = self.resolve_value(&value, &key)?;
                let value = AnnotatedValue::new(
                    Self::infer_config_value(&resolved),
                    self.source_id.clone(),
                    std::sync::Arc::from(config_path.as_str()),
                )
                .with_priority(self.priority);

                // Parse the path and build nested structure
                let parts: Vec<&str> = config_path.split('.').collect();
                Self::insert_nested(&mut map, &parts, value);
            }
        }

        Ok(AnnotatedValue::new(
            ConfigValue::Map(std::sync::Arc::new(map)),
            self.source_id.clone(),
            "",
        ))
    }

    fn priority(&self) -> u8 {
        self.priority
    }

    fn name(&self) -> &str {
        "env"
    }

    fn source_kind(&self) -> SourceKind {
        SourceKind::Environment
    }
}

impl EnvSource {
    /// Infer a `ConfigValue` from a raw string using deterministic type
    /// inference (Rule 5: deterministic logic must be explicit code, not
    /// delegated to a model).
    ///
    /// Inference order (first match wins):
    /// 1. bool — `eq_ignore_ascii_case` against "true"/"false"
    /// 2. i64 — `str::parse::<i64>()`
    /// 3. u64 — `str::parse::<u64>()` (catches values above i64::MAX)
    /// 4. f64 — only when the string contains `.`/`e`/`E`, then `str::parse::<f64>()`
    /// 5. fallback — `ConfigValue::String`
    pub fn infer_config_value(s: &str) -> ConfigValue {
        // 1. bool
        if s.eq_ignore_ascii_case("true") {
            return ConfigValue::Bool(true);
        }
        if s.eq_ignore_ascii_case("false") {
            return ConfigValue::Bool(false);
        }

        // 2. i64
        if let Ok(v) = s.parse::<i64>() {
            return ConfigValue::I64(v);
        }

        // 3. u64 (catches values above i64::MAX, e.g. 18446744073709551615)
        if let Ok(v) = s.parse::<u64>() {
            return ConfigValue::U64(v);
        }

        // 4. f64 — only attempt when the string looks like a float
        //    (contains '.', 'e', or 'E'). This prevents "123abc" from
        //    accidentally parsing via f64's permissive grammar.
        if s.contains('.') || s.contains('e') || s.contains('E') {
            if let Ok(v) = s.parse::<f64>() {
                return ConfigValue::F64(v);
            }
        }

        // 5. fallback
        ConfigValue::String(s.to_string())
    }

    /// Insert a value into a nested map structure.
    fn insert_nested(
        map: &mut indexmap::IndexMap<std::sync::Arc<str>, AnnotatedValue>,
        parts: &[&str],
        value: AnnotatedValue,
    ) {
        if parts.is_empty() {
            return;
        }

        if parts.len() == 1 {
            map.insert(std::sync::Arc::from(parts[0]), value);
            return;
        }

        // For nested paths, we need to traverse/build the structure
        let first = parts[0];
        let remaining = &parts[1..];

        // Get or create the nested map entry
        let nested = map.entry(std::sync::Arc::from(first)).or_insert_with(|| {
            AnnotatedValue::new(
                ConfigValue::Map(std::sync::Arc::new(indexmap::IndexMap::new())),
                value.source.clone(),
                std::sync::Arc::from(first),
            )
        });

        // Use Arc::get_mut to avoid cloning when possible (copy-on-write)
        if let ConfigValue::Map(ref mut inner_map) = nested.inner {
            if let Some(map_ref) = Arc::get_mut(inner_map) {
                // Arc is uniquely owned, we can mutate directly
                Self::insert_nested(map_ref, remaining, value);
            } else {
                // Arc is shared, need to clone (fallback to original behavior)
                let mut map_clone = (*inner_map).as_ref().clone();
                Self::insert_nested(&mut map_clone, remaining, value);
                *inner_map = Arc::new(map_clone);
            }
        }
    }
}

/// In-memory configuration source.
#[derive(Debug)]
pub struct MemorySource {
    /// Configuration values.
    values: HashMap<String, ConfigValue>,
    /// Priority of this source.
    priority: u8,
    /// Source ID for tracking.
    source_id: SourceId,
    /// Source name.
    name: String,
}

impl MemorySource {
    /// Create a new memory source.
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
            priority: 0,
            source_id: SourceId::new("memory"),
            name: "memory".to_string(),
        }
    }

    /// Create a memory source with initial values.
    pub fn with_values(values: HashMap<String, ConfigValue>) -> Self {
        Self {
            values,
            priority: 0,
            source_id: SourceId::new("memory"),
            name: "memory".to_string(),
        }
    }

    /// Set a value.
    pub fn set(mut self, key: impl Into<String>, value: ConfigValue) -> Self {
        self.values.insert(key.into(), value);
        self
    }

    /// Set the priority.
    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }

    /// Set the source name.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }
}

impl Default for MemorySource {
    fn default() -> Self {
        Self::new()
    }
}

impl Source for MemorySource {
    fn collect(&self) -> ConfigResult<AnnotatedValue> {
        let mut map = indexmap::IndexMap::new();

        for (key, value) in &self.values {
            let annotated = AnnotatedValue::new(
                value.clone(),
                self.source_id.clone(),
                std::sync::Arc::from(key.as_str()),
            )
            .with_priority(self.priority);

            let parts: Vec<&str> = key.split('.').collect();
            EnvSource::insert_nested(&mut map, &parts, annotated);
        }

        Ok(AnnotatedValue::new(
            ConfigValue::Map(std::sync::Arc::new(map)),
            self.source_id.clone(),
            "",
        )
        .with_priority(self.priority))
    }

    fn priority(&self) -> u8 {
        self.priority
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn source_kind(&self) -> SourceKind {
        SourceKind::Memory
    }
}

/// Default value source.
#[derive(Debug)]
pub struct DefaultSource {
    /// Default configuration values.
    defaults: HashMap<String, ConfigValue>,
    /// Source ID for tracking.
    source_id: SourceId,
}

impl DefaultSource {
    /// Create a new default source.
    pub fn new() -> Self {
        Self {
            defaults: HashMap::new(),
            source_id: SourceId::new("default"),
        }
    }

    /// Create with initial defaults.
    pub fn with_defaults(defaults: HashMap<String, ConfigValue>) -> Self {
        Self {
            defaults,
            source_id: SourceId::new("default"),
        }
    }

    /// Set a default value.
    pub fn set(mut self, key: impl Into<String>, value: ConfigValue) -> Self {
        self.defaults.insert(key.into(), value);
        self
    }
}

impl Default for DefaultSource {
    fn default() -> Self {
        Self::new()
    }
}

impl Source for DefaultSource {
    fn collect(&self) -> ConfigResult<AnnotatedValue> {
        let mut map = indexmap::IndexMap::new();

        for (key, value) in &self.defaults {
            let annotated = AnnotatedValue::new(
                value.clone(),
                self.source_id.clone(),
                std::sync::Arc::from(key.as_str()),
            )
            .with_priority(0); // Defaults have lowest priority

            let parts: Vec<&str> = key.split('.').collect();
            EnvSource::insert_nested(&mut map, &parts, annotated);
        }

        Ok(AnnotatedValue::new(
            ConfigValue::Map(std::sync::Arc::new(map)),
            self.source_id.clone(),
            "",
        ))
    }

    fn priority(&self) -> u8 {
        0 // Defaults always have lowest priority
    }

    fn name(&self) -> &str {
        "default"
    }

    fn source_kind(&self) -> SourceKind {
        SourceKind::Default
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    fn test_memory_source() {
        let source = MemorySource::new()
            .set("database.host", ConfigValue::string("localhost"))
            .set("database.port", ConfigValue::uint(5432))
            .with_priority(10);

        let result = source.collect().unwrap();
        assert_eq!(result.priority, 10);
        assert!(result.is_map());
    }

    #[test]
    fn test_default_source() {
        let source = DefaultSource::new()
            .set("app.name", ConfigValue::string("myapp"))
            .set("app.debug", ConfigValue::bool(false));

        let result = source.collect().unwrap();
        assert_eq!(result.priority, 0);
    }

    #[test]
    fn test_file_source_missing() {
        let source = FileSource::new("/nonexistent/config.toml").optional();
        let result = source.collect().unwrap();
        assert!(result.is_map());
    }

    #[test]
    #[serial]
    fn test_env_source_prefix() {
        // Set test environment variables
        std::env::set_var("TEST_APP_HOST", "localhost");
        std::env::set_var("TEST_APP_PORT", "5432");

        let source = EnvSource::with_prefix("TEST_APP_");
        let result = source.collect().unwrap();

        assert!(result.is_map());

        // Cleanup
        std::env::remove_var("TEST_APP_HOST");
        std::env::remove_var("TEST_APP_PORT");
    }

    #[test]
    fn test_source_kind() {
        let mem = MemorySource::new();
        assert_eq!(mem.source_kind(), SourceKind::Memory);

        let def = DefaultSource::new();
        assert_eq!(def.source_kind(), SourceKind::Default);

        let env = EnvSource::new();
        assert_eq!(env.source_kind(), SourceKind::Environment);
    }

    #[test]
    fn test_default_source_priority() {
        let source = DefaultSource::new();
        assert_eq!(source.priority(), 0);
    }

    #[test]
    fn test_memory_source_priority() {
        let source = MemorySource::new();
        assert_eq!(source.priority(), 0);
    }

    #[test]
    fn test_env_source_builder_priority() {
        let source = EnvSource::with_prefix("X_").with_priority(99);
        assert_eq!(source.priority(), 99);
    }

    #[test]
    fn test_env_source_name_default() {
        let source = EnvSource::new();
        assert_eq!(source.name(), "env");
    }

    #[test]
    fn test_memory_source_name() {
        let source = MemorySource::new();
        assert_eq!(source.name(), "memory");
    }

    #[test]
    fn test_default_source_name() {
        let source = DefaultSource::new();
        assert_eq!(source.name(), "default");
    }

    #[test]
    fn test_file_source_name() {
        let source = FileSource::new("test.toml");
        assert_eq!(source.name(), "test.toml");
    }

    #[test]
    fn test_file_source_path() {
        let source = FileSource::new("test.toml");
        assert!(source.file_path().is_some());
        assert!(!source.is_optional());
    }

    #[test]
    fn test_file_source_optional_flag() {
        let source = FileSource::new("missing.toml").optional();
        assert!(source.is_optional());
    }

    #[test]
    fn test_file_source_format() {
        let source = FileSource::new("config.toml").with_format(crate::impl_::loader::Format::Toml);
        assert_eq!(source.name(), "config.toml");
    }

    #[test]
    fn test_memory_source_is_not_optional() {
        let source = MemorySource::new();
        assert!(!source.is_optional());
    }

    #[test]
    fn test_source_kind_memory() {
        let source = MemorySource::new();
        assert_eq!(source.source_kind(), SourceKind::Memory);
    }

    #[test]
    fn test_source_kind_default() {
        let source = DefaultSource::new();
        assert_eq!(source.source_kind(), SourceKind::Default);
    }

    #[test]
    fn test_source_kind_file() {
        let source = FileSource::new("test.toml");
        assert_eq!(source.source_kind(), SourceKind::File);
    }

    #[test]
    fn test_env_source_default_priority() {
        let source = EnvSource::new();
        assert_eq!(source.priority(), 50);
    }

    #[test]
    fn test_env_source_default_trait() {
        let source = EnvSource::default();
        assert_eq!(source.name(), "env");
        assert_eq!(source.priority(), 50);
    }

    #[test]
    fn test_env_source_separator_setter() {
        let source = EnvSource::new().separator("__");
        // collect should succeed; separator affects key parsing only
        let _ = source.collect();
    }

    #[test]
    fn test_env_source_with_file_suffix_disabled() {
        let source = EnvSource::with_prefix("X_").with_file_suffix(false);
        assert_eq!(source.priority(), 50);
        let _ = source.collect();
    }

    #[test]
    fn test_env_source_custom_file_suffix() {
        let source = EnvSource::with_prefix("X_").file_suffix("_SECRET"); // pragma: allowlist secret
        assert_eq!(source.priority(), 50);
        let _ = source.collect();
    }

    #[test]
    fn test_file_source_with_priority() {
        let source = FileSource::new("test.toml").with_priority(75);
        assert_eq!(source.priority(), 75);
    }

    #[test]
    fn test_file_source_with_loader_config() {
        let config = loader::LoaderConfig::default();
        let source = FileSource::new("test.toml").with_loader_config(config);
        assert_eq!(source.name(), "test.toml");
    }

    #[test]
    fn test_file_source_allow_absolute_paths() {
        let source = FileSource::new("test.toml").allow_absolute_paths();
        assert_eq!(source.name(), "test.toml");
    }

    #[test]
    fn test_file_source_path_accessor() {
        let source = FileSource::new("/some/path/config.json");
        assert_eq!(source.path(), Path::new("/some/path/config.json"));
    }

    #[test]
    fn test_file_source_missing_required_error() {
        let source = FileSource::new("/nonexistent/required.toml");
        let result = source.collect();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ConfigError::FileNotFound { .. }
        ));
    }

    #[test]
    fn test_file_source_new_no_file_name() {
        // Path with no file_name component → source_id defaults to "file"
        let source = FileSource::new("/");
        assert_eq!(source.name(), "file");
    }

    #[test]
    fn test_memory_source_with_values_constructor() {
        let mut values = HashMap::new();
        values.insert("key".to_string(), ConfigValue::string("val"));
        let source = MemorySource::with_values(values);
        let result = source.collect().unwrap();
        assert!(result.is_map());
    }

    #[test]
    fn test_memory_source_with_priority() {
        let source = MemorySource::new().with_priority(42);
        assert_eq!(source.priority(), 42);
    }

    #[test]
    fn test_memory_source_with_name() {
        let source = MemorySource::new().with_name("custom");
        assert_eq!(source.name(), "custom");
    }

    #[test]
    fn test_memory_source_collect_returns_priority() {
        let source = MemorySource::new()
            .set("key", ConfigValue::string("val"))
            .with_priority(30);
        let result = source.collect().unwrap();
        assert_eq!(result.priority, 30);
    }

    #[test]
    fn test_memory_source_default_trait() {
        let source = MemorySource::default();
        assert_eq!(source.name(), "memory");
        assert_eq!(source.priority(), 0);
    }

    #[test]
    fn test_memory_source_nested_keys() {
        let source = MemorySource::new()
            .set("database.host", ConfigValue::string("localhost"))
            .set("database.port", ConfigValue::uint(5432));
        let result = source.collect().unwrap();
        assert!(result.is_map());
        // Verify nested structure was built
        if let ConfigValue::Map(map) = &result.inner {
            assert!(map.contains_key("database"));
        } else {
            panic!("expected map");
        }
    }

    #[test]
    fn test_default_source_with_defaults_constructor() {
        let mut defaults = HashMap::new();
        defaults.insert("key".to_string(), ConfigValue::string("val"));
        let source = DefaultSource::with_defaults(defaults);
        let result = source.collect().unwrap();
        assert!(result.is_map());
        assert_eq!(result.priority, 0);
    }

    #[test]
    fn test_default_source_set_chained() {
        let source = DefaultSource::new()
            .set("a", ConfigValue::string("1"))
            .set("b", ConfigValue::string("2"));
        let result = source.collect().unwrap();
        assert!(result.is_map());
    }

    #[test]
    fn test_default_source_default_trait() {
        let source = DefaultSource::default();
        assert_eq!(source.name(), "default");
    }

    #[test]
    fn test_default_source_collect_priority_zero() {
        let source = DefaultSource::new().set("key", ConfigValue::string("val"));
        let result = source.collect().unwrap();
        assert_eq!(result.priority, 0);
    }

    #[serial_test::serial]
    #[test]
    fn test_env_source_file_suffix_reads_file() {
        use std::io::Write;
        let mut tmp = tempfile::Builder::new().suffix(".txt").tempfile().unwrap();
        write!(tmp, "the_secret_content").unwrap(); // pragma: allowlist secret
        let path = tmp.path().to_str().unwrap().to_string();
        std::env::set_var("MYTEST_VAL_FILE", &path); // pragma: allowlist secret
        let source = EnvSource::with_prefix("MYTEST_");
        let result = source.collect();
        std::env::remove_var("MYTEST_VAL_FILE"); // pragma: allowlist secret
        let result = result.unwrap();

        // Verify the value was read from the file
        if let ConfigValue::Map(map) = &result.inner {
            if let Some(av) = map.get("val") {
                if let ConfigValue::String(s) = &av.inner {
                    assert_eq!(s, "the_secret_content"); // pragma: allowlist secret
                    return;
                }
            }
        }
        panic!("expected val key with string value read from file");
    }

    #[serial_test::serial]
    #[test]
    fn test_env_source_file_suffix_nonexistent_file() {
        std::env::set_var("MYTEST_MISSING_FILE", "/nonexistent/path.txt"); // pragma: allowlist secret
        let source = EnvSource::with_prefix("MYTEST_");
        let result = source.collect();
        std::env::remove_var("MYTEST_MISSING_FILE"); // pragma: allowlist secret
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ConfigError::FileNotFound { .. }
        ));
    }

    #[serial_test::serial]
    #[test]
    fn test_env_source_file_suffix_blocks_sensitive_path() {
        // /etc/passwd is in the sensitive path list and should be blocked
        std::env::set_var("MYTEST_BLOCK_FILE", "/etc/passwd"); // pragma: allowlist secret
        let source = EnvSource::with_prefix("MYTEST_");
        let result = source.collect();
        std::env::remove_var("MYTEST_BLOCK_FILE"); // pragma: allowlist secret
                                                   // May fail with FileNotFound (if /etc/passwd missing) or InvalidValue (sensitive)
        assert!(result.is_err());
    }

    #[serial_test::serial]
    #[test]
    fn test_env_source_file_suffix_rejects_directory() {
        // /tmp is a directory, not a regular file
        std::env::set_var("MYTEST_DIR_FILE", "/tmp"); // pragma: allowlist secret
        let source = EnvSource::with_prefix("MYTEST_");
        let result = source.collect();
        std::env::remove_var("MYTEST_DIR_FILE"); // pragma: allowlist secret
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ConfigError::InvalidValue { .. }
        ));
    }

    #[serial_test::serial]
    #[test]
    fn test_env_source_file_suffix_rejects_bad_extension() {
        use std::io::Write;
        let mut tmp = tempfile::Builder::new().suffix(".exe").tempfile().unwrap();
        write!(tmp, "data").unwrap();
        let path = tmp.path().to_str().unwrap().to_string();
        std::env::set_var("MYTEST_EXT_FILE", &path); // pragma: allowlist secret
        let source = EnvSource::with_prefix("MYTEST_");
        let result = source.collect();
        std::env::remove_var("MYTEST_EXT_FILE"); // pragma: allowlist secret
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ConfigError::InvalidValue { .. }
        ));
    }

    #[serial_test::serial]
    #[test]
    fn test_env_source_file_suffix_empty_path() {
        std::env::set_var("MYTEST_EMPTY_FILE", ""); // pragma: allowlist secret
        let source = EnvSource::with_prefix("MYTEST_");
        let result = source.collect();
        std::env::remove_var("MYTEST_EMPTY_FILE"); // pragma: allowlist secret
                                                   // Empty path → validate returns Ok, then read_to_string("") fails
        assert!(result.is_err());
    }

    #[serial_test::serial]
    #[test]
    fn test_env_source_skips_file_suffix_without_prefix() {
        // Without prefix, _FILE vars are skipped (returns None from parse_key)
        std::env::set_var("MYTEST_NOPREFIX_FILE", "/tmp/x.txt"); // pragma: allowlist secret
        let source = EnvSource::new(); // no prefix
                                       // This should not error — _FILE vars without prefix are skipped
        let result = source.collect();
        std::env::remove_var("MYTEST_NOPREFIX_FILE"); // pragma: allowlist secret
        assert!(result.is_ok());
    }

    // ===== infer_config_value (fix-0.4.1 Bug 2) =====

    #[test]
    fn test_infer_config_value() {
        // bool
        assert_eq!(
            EnvSource::infer_config_value("true"),
            ConfigValue::Bool(true)
        );
        assert_eq!(
            EnvSource::infer_config_value("false"),
            ConfigValue::Bool(false)
        );
        // bool is case-insensitive
        assert_eq!(
            EnvSource::infer_config_value("TRUE"),
            ConfigValue::Bool(true)
        );
        assert_eq!(
            EnvSource::infer_config_value("False"),
            ConfigValue::Bool(false)
        );

        // i64
        assert_eq!(
            EnvSource::infer_config_value("5432"),
            ConfigValue::I64(5432)
        );
        assert_eq!(EnvSource::infer_config_value("-7"), ConfigValue::I64(-7));
        assert_eq!(EnvSource::infer_config_value("0"), ConfigValue::I64(0));

        // u64 (above i64::MAX)
        assert_eq!(
            EnvSource::infer_config_value("18446744073709551615"),
            ConfigValue::U64(u64::MAX)
        );

        // f64
        assert_eq!(
            EnvSource::infer_config_value("3.14"),
            ConfigValue::F64(3.14)
        );
        assert_eq!(
            EnvSource::infer_config_value("1e10"),
            ConfigValue::F64(1e10)
        );
        assert_eq!(
            EnvSource::infer_config_value("-2.5E-3"),
            ConfigValue::F64(-2.5e-3)
        );

        // fallback: plain strings
        assert_eq!(
            EnvSource::infer_config_value("hello"),
            ConfigValue::String("hello".to_string())
        );
        assert_eq!(
            EnvSource::infer_config_value(""),
            ConfigValue::String("".to_string())
        );
        assert_eq!(
            EnvSource::infer_config_value("123abc"),
            ConfigValue::String("123abc".to_string())
        );
        // "abc123" contains no float marker, not a valid int → string
        assert_eq!(
            EnvSource::infer_config_value("abc123"),
            ConfigValue::String("abc123".to_string())
        );
    }

    // ===== collect() type inference integration (fix-0.4.1 Bug 2) =====

    #[serial_test::serial]
    #[test]
    fn test_env_source_collect_infers_types() {
        // Set typed env vars under a unique prefix
        std::env::set_var("TESTCFG_PORT", "5432");
        std::env::set_var("TESTCFG_DEBUG", "true");
        std::env::set_var("TESTCFG_HOST", "localhost");

        let source = EnvSource::with_prefix("TESTCFG_");
        let result = source.collect();

        // Cleanup before assertions so panics don't leak env vars
        std::env::remove_var("TESTCFG_PORT");
        std::env::remove_var("TESTCFG_DEBUG");
        std::env::remove_var("TESTCFG_HOST");

        let result = result.expect("collect should succeed");
        let map = match &result.inner {
            ConfigValue::Map(m) => m,
            _ => panic!("expected map, got {:?}", result.inner),
        };

        // port → I64(5432)
        let port = map
            .get("port")
            .expect("map should contain 'port' key")
            .inner
            .as_i64()
            .expect("port should be I64");
        assert_eq!(port, 5432, "port should infer as i64 5432");

        // debug → Bool(true)
        let debug = map
            .get("debug")
            .expect("map should contain 'debug' key")
            .inner
            .as_bool()
            .expect("debug should be Bool");
        assert!(debug, "debug should infer as bool true");

        // host → String("localhost")
        let host = map
            .get("host")
            .expect("map should contain 'host' key")
            .inner
            .as_str()
            .expect("host should be String");
        assert_eq!(host, "localhost", "host should remain a string 'localhost'");
    }

    #[serial_test::serial]
    #[test]
    fn test_env_source_file_suffix_infers_type() {
        // _FILE suffix reads file content, which should also go through
        // type inference (specs/env-source R-003).
        use std::io::Write;
        let mut tmp = tempfile::Builder::new().suffix(".txt").tempfile().unwrap();
        write!(tmp, "8080").unwrap();
        let path = tmp.path().to_str().unwrap().to_string();

        std::env::set_var("MYTEST_PORT_FILE", &path); // pragma: allowlist secret
        let source = EnvSource::with_prefix("MYTEST_");
        let result = source.collect();
        std::env::remove_var("MYTEST_PORT_FILE"); // pragma: allowlist secret

        let result = result.expect("collect should succeed");
        let map = match &result.inner {
            ConfigValue::Map(m) => m,
            _ => panic!("expected map, got {:?}", result.inner),
        };

        let port = map
            .get("port")
            .expect("map should contain 'port' key (from _FILE suffix)")
            .inner
            .as_i64()
            .expect("port from _FILE should infer as I64");
        assert_eq!(
            port, 8080,
            "_FILE content '8080' should infer as i64 8080, not stay as string"
        );
    }
}
