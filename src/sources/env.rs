//! Environment variable source with `_FILE` suffix support.
//!
//! This module implements environment variable configuration source with
//! support for Docker Secrets and Kubernetes Secrets file mounting pattern.
//!
//! When an environment variable name ends with `_FILE`, the value is treated
//! as a file path, and the file contents are read as the actual configuration value.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use crate::error::{ConfigError, ConfigResult};
use crate::value::{AnnotatedValue, ConfigValue, SourceId};
use crate::traits::ConfigProvider;

/// Default suffix for file-based secrets
const DEFAULT_FILE_SUFFIX: &str = "_FILE";

/// Environment variable source configuration
pub struct EnvSource {
    /// Optional prefix for environment variables
    prefix: Option<String>,
    /// Suffix for file-based secrets (default: "_FILE")
    file_suffix: &'static str,
    /// Separator for nested keys (default: "__")
    nested_separator: &'static str,
}

impl EnvSource {
    /// Create a new environment variable source
    pub fn new() -> Self {
        Self {
            prefix: None,
            file_suffix: DEFAULT_FILE_SUFFIX,
            nestedseparator: "__",
        }
    }

    /// Set the prefix for environment variables
    pub fn with_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = Some(prefix.into());
        self
    }

    /// Set a custom file suffix for secrets
    pub fn with_file_suffix(mut self, suffix: &'static str) -> Self {
        self.file_suffix = suffix;
        self
    }

    /// Set a custom nested key separator
    pub fn with_nested_separator(mut self, sep: &'static str) -> Self {
        self.nestedseparator = sep;
        self
    }

    /// Resolve a value from an environment variable
    fn resolve_value(&self, raw: &str, var_name: &str) -> ConfigResult<String> {
        if var_name.ends_with(self.file_suffix) {
            let path = PathBuf::from(raw);
            if !path.exists() {
                return Err(ConfigError::FileNotFound {
                    filename: raw.to_string(),
                    source: None,
                });
            }
            std::fs::read_to_string(&path).map_err(|e| ConfigError::FileNotFound {
                filename: raw.to_string(),
                source: Some(Arc::new(e)),
            })
        } else {
            Ok(raw.to_string())
        }
    }

    /// Convert environment variable name to config key
    fn var_to_key(&self, var_name: &str) -> String {
        let key = if let Some(prefix) = &self.prefix {
            var_name.strip_prefix(prefix).unwrap_or(var_name)
        } else {
            var_name
        };
        key.replace(self.nestedseparator, ".")
    }

    /// Collect all environment variables with the configured prefix
    fn collect_vars(&self) -> HashMap<String, String> {
        let mut result = HashMap::new();
        
        for (key, value) in std::env::vars() {
            if let Some(prefix) = &self.prefix {
                if key.starts_with(prefix) {
                    result.insert(key, value);
                }
            } else {
                result.insert(key, value);
            }
        }
        
        result
    }
}

impl Default for EnvSource {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigProvider for EnvSource {
    fn get_raw(&self, key: &str) -> Option<&AnnotatedValue> {
        None
    }

    fn keys(&self) -> Vec<String> {
        let vars = self.collect_vars();
        vars.into_keys()
            .map(|k| self.var_to_key(&k))
            .filter(|k| !k.ends_with(self.file_suffix))
            .collect()
    }
}

impl EnvSource {
    /// Load all environment variables into a map
    pub fn load(&self) -> ConfigResult<HashMap<String, AnnotatedValue>> {
        let vars = self.collect_vars();
        let mut result = HashMap::new();
        
        for (var_name, raw_value) in vars {
            if var_name.ends_with(self.file_suffix) {
                continue;
            }
            
            let config_key = self.var_to_key(&var_name);
            let resolved = self.resolve_value(&raw_value, &var_name)?;
            
            result.insert(
                config_key,
                AnnotatedValue::new(ConfigValue::String(resolved))
                    .with_source(SourceId::from_static("env"))
            );
        }
        
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_env_source_basic() {
        env::set_var("TEST_KEY", "value");
        let source = EnvSource::new().with_prefix("TEST_");
        let loaded = source.load().unwrap();
        assert!(loaded.contains_key("KEY"));
    }

    #[test]
    fn test_file_suffix_mode() {
        use tempfile::NamedTempFile;
        
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "secret_value").unwrap();
        let path = file.path().to_string_lossy().to_string();
        
        env::set_var("APP_PASSWORD_FILE", &path);
        
        let source = EnvSource::new().with_prefix("APP_");
        let loaded = source.load().unwrap();
        
        assert!(loaded.contains_key("PASSWORD_FILE"));
    }
}
