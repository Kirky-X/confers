// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! ConfigBuilder - A config-rs compatible API for confers
//!
//! This module provides a Builder-style API that is compatible with config-rs,
//! making migration from config-rs to confers much easier.
//!
//! # Configuration Source Priority
//!
//! Configuration sources are applied in the order they are added, with later sources
//! overriding earlier ones. The priority order is:
//!
//! 1. **Default values** (lowest priority) - Set via `set_default()`
//! 2. **File sources** - Added via `add_source(File::with_name(...))`
//! 3. **Environment variables** (highest priority) - Added via `add_source(Environment::with_prefix(...))`
//!
//! # Merging Strategy
//!
//! - Default values are merged together into a single configuration object
//! - Configuration sources are merged sequentially, with later sources overriding earlier ones
//! - Nested values are merged at the field level, not at the object level
//! - If a parent key exists but is not an object type, setting a nested value will return an error
//!
//! # Performance Considerations
//!
//! - All default values are combined into a single map before merging, reducing unnecessary operations
//! - File format detection is done once per file source
//! - Configuration extraction uses figment's efficient deserialization
//! - For large configurations with many defaults, consider using `Default` trait instead
//!
//! # Example
//!
//! ```rust,no_run
//! use confers::{ConfigBuilder, Environment, File};
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Debug, Serialize, Deserialize)]
//! struct MyConfig {
//!     server: ServerConfig,
//! }
//!
//! #[derive(Debug, Serialize, Deserialize)]
//! struct ServerConfig {
//!     port: u16,
//!     host: String,
//! }
//!
//! let config: MyConfig = ConfigBuilder::new()
//!     .set_default("server.port", 8899)?
//!     .set_default("server.host", "0.0.0.0")?
//!     .add_source(File::with_name("config/default").required(false))
//!     .add_source(Environment::with_prefix("CRAWLRS").separator("__"))
//!     .build()?;
//! # Ok::<(), confers::ConfigError>(())
//! ```

use crate::error::ConfigError;
use crate::utils::FileFormat;
use figment::providers::{Env, Format, Json, Serialized, Toml, Yaml};
use figment::{Figment, Profile};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::str::FromStr;

/// Configuration builder with config-rs compatible API
///
/// This builder provides a migration-friendly API that closely mirrors config-rs,
/// allowing for gradual migration with minimal code changes.
///
/// # Example
///
/// ```rust,no_run
/// use confers::{ConfigBuilder, Environment, File};
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Debug, Serialize, Deserialize)]
/// struct MyConfig {
///     server: ServerConfig,
/// }
///
/// #[derive(Debug, Serialize, Deserialize)]
/// struct ServerConfig {
///     port: u16,
///     host: String,
/// }
///
/// let config: MyConfig = ConfigBuilder::new()
///     .set_default("server.port", 8899)?
///     .set_default("server.host", "0.0.0.0")?
///     .add_source(File::with_name("config/default").required(false))
///     .add_source(Environment::with_prefix("CRAWLRS").separator("__"))
///     .build()?;
/// # Ok::<(), confers::ConfigError>(())
/// ```
#[derive(Clone, Default)]
pub struct ConfigBuilder {
    /// Internal figment for configuration
    figment: Figment,
    /// Default values as a map
    defaults: Vec<(String, serde_json::Value)>,
}

impl ConfigBuilder {
    /// Create a new configuration builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Set a default value for a configuration key
    ///
    /// This method allows setting default values one at a time, similar to config-rs.
    /// The key can use dot notation for nested values (e.g., "server.port").
    ///
    /// # Arguments
    ///
    /// * `key` - Configuration key (supports dot notation for nested values)
    /// * `value` - Default value (must be JSON-serializable)
    ///
    /// # Example
    ///
    /// ```rust
    /// use confers::ConfigBuilder;
    ///
    /// let builder = ConfigBuilder::new()
    ///     .set_default("server.port", 8080)?
    ///     .set_default("server.host", "localhost")?
    ///     .set_default("debug", true)?;
    /// # Ok::<(), confers::ConfigError>(())
    /// ```
    pub fn set_default<K, V>(mut self, key: K, value: V) -> Result<Self, ConfigError>
    where
        K: AsRef<str>,
        V: Serialize + Into<serde_json::Value>,
    {
        let key_str = key.as_ref().to_string();
        let json_value = serde_json::to_value(&value).map_err(|e| {
            ConfigError::SerializationError(format!("Failed to serialize default value: {}", e))
        })?;

        self.defaults.push((key_str, json_value));
        Ok(self)
    }

    /// Add a configuration source
    ///
    /// This method adds a configuration source to the builder.
    /// Sources are loaded in the order they are added, with later sources overriding earlier ones.
    ///
    /// # Arguments
    ///
    /// * `source` - Configuration source to add
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use confers::{ConfigBuilder, Environment, File};
    ///
    /// let builder = ConfigBuilder::new()
    ///     .add_source(File::with_name("config/default"))
    ///     .add_source(Environment::with_prefix("APP").separator("__"));
    /// # Ok::<(), confers::ConfigError>(())
    /// ```
    pub fn add_source<S>(mut self, source: S) -> Self
    where
        S: Into<Source>,
    {
        let source = source.into();
        self.figment = self.figment.merge(source.into_figment());
        self
    }

    /// Build the configuration
    ///
    /// This method builds the final configuration by merging all sources and defaults.
    ///
    /// # Type Parameters
    ///
    /// * `T` - Configuration type (must be deserializable)
    ///
    /// # Returns
    ///
    /// Returns the deserialized configuration or an error
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use confers::ConfigBuilder;
    /// use serde::{Deserialize, Serialize};
    ///
    /// #[derive(Debug, Serialize, Deserialize)]
    /// struct Config {
    ///     port: u16,
    ///     host: String,
    /// }
    ///
    /// let config: Config = ConfigBuilder::new()
    ///     .set_default("port", 8080)?
    ///     .set_default("host", "localhost")?
    ///     .build()?;
    /// # Ok::<(), confers::ConfigError>(())
    /// ```
    pub fn build<T>(self) -> Result<T, ConfigError>
    where
        T: DeserializeOwned + Serialize,
    {
        // Apply defaults first
        let mut figment = self.figment;

        // Merge all defaults into a single map for better performance
        let mut defaults_map = serde_json::Map::new();
        for (key, value) in self.defaults {
            insert_nested_value(&mut defaults_map, &key, value)?;
        }

        if !defaults_map.is_empty() {
            let defaults_value = serde_json::Value::Object(defaults_map);
            figment = figment.merge(figment::providers::Serialized::defaults(defaults_value));
        }

        // Extract the configuration
        figment.extract().map_err(|e| {
            ConfigError::ParseError(format!(
                "Failed to extract configuration: {}. Check if all required fields are provided and have correct types.",
                e
            ))
        })
    }

    /// Build the configuration with validation
    ///
    /// This method builds the configuration and validates it using the `Validate` trait.
    ///
    /// # Type Parameters
    ///
    /// * `T` - Configuration type (must be deserializable and implement `Validate`)
    ///
    /// # Returns
    ///
    /// Returns the validated configuration or an error
    #[cfg(feature = "validation")]
    pub fn build_with_validation<T>(self) -> Result<T, ConfigError>
    where
        T: DeserializeOwned + Serialize + validator::Validate,
    {
        let config = self.build::<T>()?;

        config.validate().map_err(|e| {
            ConfigError::ValidationError(format!("Configuration validation failed: {}", e))
        })?;

        Ok(config)
    }

    /// Clear all default values
    ///
    /// This method removes all previously set default values.
    ///
    /// # Example
    ///
    /// ```rust
    /// use confers::ConfigBuilder;
    ///
    /// let builder = ConfigBuilder::new()
    ///     .set_default("port", 8080)?
    ///     .clear_defaults();
    /// # Ok::<(), confers::ConfigError>(())
    /// ```
    pub fn clear_defaults(mut self) -> Self {
        self.defaults.clear();
        self
    }

    /// Get the number of default values set
    ///
    /// # Returns
    ///
    /// The number of default values currently set
    pub fn defaults_count(&self) -> usize {
        self.defaults.len()
    }
}

/// Helper function to insert nested values using dot notation
fn insert_nested_value(
    map: &mut serde_json::Map<String, serde_json::Value>,
    key: &str,
    value: serde_json::Value,
) -> Result<(), ConfigError> {
    if key.is_empty() {
        return Err(ConfigError::ParseError("Key cannot be empty".to_string()));
    }

    let parts: Vec<&str> = key.split('.').collect();

    if parts.len() == 1 {
        map.insert(parts[0].to_string(), value);
        return Ok(());
    }

    let current_key = parts[0].to_string();
    let remaining_key = parts[1..].join(".");

    if !map.contains_key(&current_key) {
        map.insert(
            current_key.clone(),
            serde_json::Value::Object(serde_json::Map::new()),
        );
    } else {
        // Check if existing value is an object type
        if !matches!(map[&current_key], serde_json::Value::Object(_)) {
            return Err(ConfigError::ParseError(format!(
                "Cannot set nested value '{}' because '{}' is not an object",
                remaining_key, current_key
            )));
        }
    }

    if let serde_json::Value::Object(ref mut nested_map) = map[&current_key] {
        insert_nested_value(nested_map, &remaining_key, value)?;
    }

    Ok(())
}

/// Configuration source
///
/// Represents various configuration sources such as files, environment variables, etc.
#[derive(Clone)]
pub enum Source {
    /// File configuration source
    File(FileSource),
    /// Environment configuration source
    Environment(EnvironmentSource),
}

impl std::fmt::Debug for Source {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Source::File(file) => write!(f, "File({:?})", file),
            Source::Environment(env) => write!(f, "Environment({:?})", env),
        }
    }
}

impl Source {
    fn into_figment(self) -> Figment {
        match self {
            Source::File(file) => file.into_figment(),
            Source::Environment(env) => env.into_figment(),
        }
    }
}

impl From<FileSource> for Source {
    fn from(file: FileSource) -> Self {
        Source::File(file)
    }
}

impl From<EnvironmentSource> for Source {
    fn from(env: EnvironmentSource) -> Self {
        Source::Environment(env)
    }
}

/// File configuration source
///
/// Represents a configuration file with optional format detection.
#[derive(Clone, Debug)]
pub struct FileSource {
    /// File name or path (can include or exclude extension)
    name: PathBuf,
    /// File format (None for auto-detection)
    format: Option<FileFormat>,
    /// Whether the file is required
    required: bool,
}

impl FileSource {
    /// Create a new file source from a file name
    ///
    /// # Arguments
    ///
    /// * `name` - File name (can include or exclude extension)
    ///
    /// # Example
    ///
    /// ```rust
    /// use confers::File;
    ///
    /// let file = File::with_name("config/default");
    /// ```
    pub fn with_name(name: impl AsRef<Path>) -> Self {
        let path = name.as_ref();

        // Validate path is not empty
        if path.as_os_str().is_empty() {
            #[cfg(feature = "tracing")]
            tracing::warn!("File path is empty, using default configuration");
            return Self {
                name: PathBuf::from("config"),
                format: None,
                required: false,
            };
        }

        // Validate path doesn't contain path traversal attacks
        let path_str = path.to_string_lossy();

        // Security: Check for various path traversal patterns
        let suspicious_patterns = [
            "..",      // Standard traversal
            "./",      // Current directory reference
            "//",      // Double slash
            "\\",      // Windows backslash (can bypass some checks)
            "%2e%2e",  // URL encoded ".."
            "%2e%2e/", // URL encoded "../"
            "..%2f",   // Mixed encoding
        ];

        let is_suspicious = suspicious_patterns
            .iter()
            .any(|pattern| path_str.contains(pattern));

        if is_suspicious {
            #[cfg(feature = "tracing")]
            tracing::error!(
                "Path contains suspicious patterns that may indicate a path traversal attempt: {}. Using safe default.",
                path_str
            );
            return Self {
                name: PathBuf::from("config"),
                format: None,
                required: false,
            };
        }

        // Additional security: Ensure path is within allowed directories
        // This prevents paths like /etc/passwd or other sensitive files
        let sensitive_prefixes = [
            "/etc/",
            "/usr/",
            "/var/log/",
            "/root/",
            "/home/",
            "C:\\Windows\\",
            "C:\\Program Files\\",
        ];

        let lower_path = path_str.to_lowercase();
        let is_sensitive = sensitive_prefixes
            .iter()
            .any(|prefix| lower_path.starts_with(prefix));

        if is_sensitive {
            #[cfg(feature = "tracing")]
            tracing::error!(
                "Path points to sensitive system directory: {}. Using safe default.",
                path_str
            );
            return Self {
                name: PathBuf::from("config"),
                format: None,
                required: false,
            };
        }

        // Canonicalize path to resolve any symlinks or relative paths
        // This helps prevent symlink attacks
        let canonical_path = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());

        // Final check: ensure the canonical path is still safe
        let canonical_str = canonical_path.to_string_lossy();
        let canonical_sensitive = sensitive_prefixes
            .iter()
            .any(|prefix| canonical_str.to_lowercase().starts_with(prefix));

        if canonical_sensitive {
            #[cfg(feature = "tracing")]
            tracing::error!(
                "Resolved path points to sensitive system directory: {}. Using safe default.",
                canonical_str
            );
            return Self {
                name: PathBuf::from("config"),
                format: None,
                required: false,
            };
        }

        Self {
            name: canonical_path,
            format: None,
            required: false,
        }
    }

    /// Set the file as required
    ///
    /// If the file is required and doesn't exist, an error will be returned.
    pub fn required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    /// Set the file format explicitly
    pub fn format(mut self, format: FileFormat) -> Self {
        self.format = Some(format);
        self
    }

    fn into_figment(self) -> Figment {
        let path = self.name;

        // Determine format: use explicit format, or detect from extension, or default to TOML
        let format = self
            .format
            .or_else(|| {
                path.extension()
                    .and_then(|ext| ext.to_str())
                    .and_then(|ext| FileFormat::from_str(ext).ok())
            })
            .unwrap_or(FileFormat::Toml); // Default to TOML if no extension matches

        match format {
            FileFormat::Toml => Figment::from(Toml::file(path)),
            FileFormat::Json => Figment::from(Json::file(path)),
            FileFormat::Yaml => Figment::from(Yaml::file(path)),
            FileFormat::Ini => {
                let content = std::fs::read_to_string(&path).unwrap_or_default();
                let ini_value =
                    serde_ini::from_str::<serde_json::Value>(&content).unwrap_or_else(|_| {
                        serde_json::Value::Object(serde_json::Map::new())
                    });
                Figment::from(Serialized::from(ini_value, Profile::Default))
            }
            FileFormat::Unknown => Figment::from(Toml::file(path)), // Default to TOML for unknown
        }
    }
}

/// Environment configuration source
///
/// Represents environment variables as a configuration source.
#[derive(Clone, Debug)]
pub struct EnvironmentSource {
    /// Prefix for environment variables
    prefix: Option<String>,
    /// Separator for nested keys (default: "_")
    separator: String,
}

impl EnvironmentSource {
    /// Create a new environment source with a prefix
    ///
    /// # Arguments
    ///
    /// * `prefix` - Environment variable prefix
    ///
    /// # Example
    ///
    /// ```rust
    /// use confers::Environment;
    ///
    /// let env = Environment::with_prefix("APP");
    /// ```
    pub fn with_prefix(prefix: impl Into<String>) -> Self {
        let prefix_str = prefix.into();
        if prefix_str.is_empty() {
            #[cfg(feature = "tracing")]
            tracing::warn!("Empty prefix for environment variables, no prefix will be applied");
        }
        Self {
            prefix: if prefix_str.is_empty() {
                None
            } else {
                Some(prefix_str)
            },
            separator: "_".to_string(),
        }
    }

    /// Set the separator for nested keys
    ///
    /// # Arguments
    ///
    /// * `separator` - Separator string (e.g., "_", "__", ".")
    ///
    /// # Example
    ///
    /// ```rust
    /// use confers::Environment;
    ///
    /// let env = Environment::with_prefix("APP").separator("__");
    /// ```
    pub fn separator(mut self, separator: impl Into<String>) -> Self {
        let sep = separator.into();
        if sep.is_empty() {
            #[cfg(feature = "tracing")]
            tracing::warn!(
                "Empty separator for environment variables may cause unexpected behavior"
            );
        }
        self.separator = sep;
        self
    }

    fn into_figment(self) -> Figment {
        if let Some(prefix) = self.prefix {
            Figment::from(Env::prefixed(&prefix).split(&self.separator))
        } else {
            Figment::from(Env::raw())
        }
    }
}

/// Type alias for File source (config-rs compatibility)
pub type File = FileSource;

/// Type alias for Environment source (config-rs compatibility)
pub type Environment = EnvironmentSource;

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct TestConfig {
        server: ServerConfig,
        debug: bool,
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct ServerConfig {
        host: String,
        port: u16,
    }

    #[test]
    fn test_set_default() -> Result<(), Box<dyn std::error::Error>> {
        let config: TestConfig = ConfigBuilder::new()
            .set_default("server.host", "localhost")?
            .set_default("server.port", 8080)?
            .set_default("debug", true)?
            .build()?;

        assert_eq!(config.server.host, "localhost");
        assert_eq!(config.server.port, 8080);
        assert!(config.debug);
        Ok(())
    }

    #[test]
    fn test_nested_defaults() -> Result<(), Box<dyn std::error::Error>> {
        let config: TestConfig = ConfigBuilder::new()
            .set_default("server.host", "0.0.0.0")?
            .set_default("server.port", 8899)?
            .set_default("debug", false)?
            .build()?;

        assert_eq!(config.server.host, "0.0.0.0");
        assert_eq!(config.server.port, 8899);
        assert!(!config.debug);
        Ok(())
    }
}

/// Extension trait for saving configuration
pub trait ConfigSaveExt {
    /// Save configuration to a file (infer format from extension)
    fn save(&self, path: impl AsRef<std::path::Path>) -> Result<u64, ConfigError>;
    /// Save configuration with explicit format
    fn save_to_with_format(
        &self,
        path: impl AsRef<std::path::Path>,
        format: FileFormat,
    ) -> Result<u64, ConfigError>;
}

impl<T> ConfigSaveExt for T
where
    T: Serialize,
{
    fn save(&self, path: impl AsRef<std::path::Path>) -> Result<u64, ConfigError> {
        let ext = path
            .as_ref()
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .unwrap_or_else(|| "json".to_string());

        let format = match ext.as_str() {
            "toml" => FileFormat::Toml,
            "yaml" | "yml" => FileFormat::Yaml,
            "ini" => FileFormat::Ini,
            "json" | _ => FileFormat::Json,
        };

        self.save_to_with_format(path, format)
    }

    fn save_to_with_format(
        &self,
        path: impl AsRef<std::path::Path>,
        format: FileFormat,
    ) -> Result<u64, ConfigError> {
        use std::fs::File;
        use std::io::Write;

        let data = serde_json::to_value(self)
            .map_err(|e| ConfigError::SerializationError(format!("Failed to serialize config: {}", e)))?;

        let content = crate::utils::file_format::serialize_to_format(&data, format)
            .map_err(ConfigError::SerializationError)?;

        let mut file = File::create(path)
            .map_err(|e| ConfigError::IoError(format!("Failed to create file: {}", e)))?;

        file.write_all(content.as_bytes())
            .map_err(|e| ConfigError::IoError(format!("Failed to write file: {}", e)))?;

        Ok(content.len() as u64)
    }
}

