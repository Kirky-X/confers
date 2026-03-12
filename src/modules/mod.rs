//! Configuration Groups / Modules support.
//!
//! This module provides support for composable configuration groups (modules),
//! allowing runtime selection of configuration module combinations.
//!
//! # Design
//!
//! The module system allows splitting configuration into interchangeable "config groups":
//! ```text
//! conf/
//! ├── database/
//! │   ├── mysql.toml
//! │   └── postgresql.toml
//! └── cache/
//!     ├── redis.toml
//!     └── in_memory.toml
//! ```
//!
//! # Example
//!
//! ```rust
//! use confers::modules::{ModuleConfig, ModuleRegistry};
//! use confers::loader::LoaderConfig;
//! use std::path::PathBuf;
//!
//! // Create a module registry
//! let mut registry = ModuleRegistry::new();
//!
//! // Register a group with multiple profiles
//! registry.register_group(
//!     "database",
//!     vec![
//!         ("mysql", PathBuf::from("conf/database/mysql.toml")),
//!         ("postgresql", PathBuf::from("conf/database/postgresql.toml")),
//!     ],
//!     Some("mysql"),
//! );
//!
//! // Load a module with a specific profile
//! let config = registry.load_module("database", "postgresql", &LoaderConfig::default());
//! ```

use crate::error::{ConfigError, ConfigResult};
use crate::loader::{load_file, LoaderConfig};
#[allow(unused_imports)]
use crate::value::{AnnotatedValue, ConfigValue};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

/// A configuration module containing profile paths and active profile.
///
/// Each module represents a group of interchangeable configurations
/// (e.g., database: mysql, postgresql; cache: redis, in_memory).
#[derive(Debug, Clone)]
pub struct ModuleConfig {
    /// The name of this module/group.
    pub name: Arc<str>,
    /// Available profile paths as (profile_name, path).
    pub paths: Vec<(Arc<str>, PathBuf)>,
    /// The currently active profile for this module.
    pub active_profile: Arc<str>,
}

impl ModuleConfig {
    /// Create a new module config.
    pub fn new(name: &str, paths: Vec<(&str, PathBuf)>, default: Option<&str>) -> Self {
        let name = Arc::from(name);
        let paths: Vec<(Arc<str>, PathBuf)> =
            paths.into_iter().map(|(n, p)| (Arc::from(n), p)).collect();

        let active_profile = default.map(Arc::from).unwrap_or_else(|| {
            paths
                .first()
                .map(|(n, _)| n.clone())
                .unwrap_or_else(|| Arc::from(""))
        });

        Self {
            name,
            paths,
            active_profile,
        }
    }

    /// Get a profile path by name.
    pub fn get_profile(&self, profile: &str) -> Option<&PathBuf> {
        self.paths
            .iter()
            .find(|(name, _)| name.as_ref() == profile)
            .map(|(_, path)| path)
    }

    /// Check if a profile exists.
    pub fn has_profile(&self, profile: &str) -> bool {
        self.paths.iter().any(|(name, _)| name.as_ref() == profile)
    }

    /// Get list of available profiles.
    pub fn profiles(&self) -> Vec<Arc<str>> {
        self.paths.iter().map(|(n, _)| n.clone()).collect()
    }
}

/// Registry for managing configuration groups (modules).
///
/// The ModuleRegistry maintains a collection of configuration groups,
/// each with multiple profile options. It allows loading specific
/// configurations based on the active profile.
///
/// # DI Patterns
///
/// - [`new()`][ModuleRegistry::new] - Zero-config default
/// - [`with_capacity()`][ModuleRegistry::with_capacity] - Pre-allocate capacity
#[derive(Debug, Default)]
pub struct ModuleRegistry {
    /// Registered configuration groups.
    groups: HashMap<Arc<str>, ModuleConfig>,
}

impl ModuleRegistry {
    /// Create a new empty module registry.
    ///
    /// # Example
    ///
    /// ```rust
    /// use confers::modules::ModuleRegistry;
    ///
    /// let registry = ModuleRegistry::new();
    /// ```
    pub fn new() -> Self {
        Self {
            groups: HashMap::new(),
        }
    }

    /// Create a new module registry with pre-allocated capacity.
    ///
    /// # Example
    ///
    /// ```rust
    /// use confers::modules::ModuleRegistry;
    ///
    /// let registry = ModuleRegistry::with_capacity(10);
    /// ```
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            groups: HashMap::with_capacity(capacity),
        }
    }

    /// Register a new configuration group with profiles.
    ///
    /// # Arguments
    ///
    /// * `name` - The group name (e.g., "database", "cache")
    /// * `profiles` - Iterator of (profile_name, path) pairs
    /// * `default` - Optional default profile name
    ///
    /// # Example
    ///
    /// ```rust
    /// use confers::modules::ModuleRegistry;
    /// use std::path::PathBuf;
    ///
    /// let mut registry = ModuleRegistry::new();
    /// registry.register_group(
    ///     "database",
    ///     vec![
    ///         ("mysql", PathBuf::from("conf/db/mysql.toml")),
    ///         ("postgresql", PathBuf::from("conf/db/postgresql.toml")),
    ///     ],
    ///     Some("mysql"),
    /// );
    /// ```
    pub fn register_group(
        &mut self,
        name: &str,
        profiles: Vec<(&str, PathBuf)>,
        default: Option<&str>,
    ) -> &mut Self {
        let config = ModuleConfig::new(name, profiles, default);
        self.groups.insert(config.name.clone(), config);
        self
    }

    /// Load a module configuration by group and profile name.
    ///
    /// # Arguments
    ///
    /// * `name` - The group name
    /// * `profile` - The profile name to load
    /// * `config` - Loader configuration
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::ModuleNotFound`] if the group or profile doesn't exist.
    ///
    /// # Example
    ///
    /// ```rust
    /// use confers::modules::ModuleRegistry;
    /// use confers::loader::LoaderConfig;
    /// use std::path::PathBuf;
    ///
    /// let mut registry = ModuleRegistry::new();
    /// registry.register_group(
    ///     "database",
    ///     vec![
    ///         ("mysql", PathBuf::from("conf/db/mysql.toml")),
    ///     ],
    ///     Some("mysql"),
    /// );
    ///
    /// let config = registry.load_module("database", "mysql", &LoaderConfig::default());
    /// ```
    pub fn load_module(
        &self,
        name: &str,
        profile: &str,
        config: &LoaderConfig,
    ) -> ConfigResult<AnnotatedValue> {
        let module = self
            .groups
            .get(name)
            .ok_or_else(|| ConfigError::ModuleNotFound {
                group: name.to_string(),
                module: profile.to_string(),
            })?;

        let path = module
            .get_profile(profile)
            .ok_or_else(|| ConfigError::ModuleNotFound {
                group: name.to_string(),
                module: profile.to_string(),
            })?;

        load_file(path, config)
    }

    /// Load the active module configuration for a group.
    ///
    /// Uses the currently active profile for the group.
    ///
    /// # Arguments
    ///
    /// * `name` - The group name
    /// * `config` - Loader configuration
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::ModuleNotFound`] if the group doesn't exist
    /// or the active profile's file cannot be loaded.
    pub fn load_active(&self, name: &str, config: &LoaderConfig) -> ConfigResult<AnnotatedValue> {
        let module = self
            .groups
            .get(name)
            .ok_or_else(|| ConfigError::ModuleNotFound {
                group: name.to_string(),
                module: "active".to_string(),
            })?;

        self.load_module(name, &module.active_profile, config)
    }

    /// List all registered group names.
    ///
    /// # Example
    ///
    /// ```rust
    /// use confers::modules::ModuleRegistry;
    /// use std::path::PathBuf;
    ///
    /// let mut registry = ModuleRegistry::new();
    /// registry.register_group("database", vec![], None);
    /// registry.register_group("cache", vec![], None);
    ///
    /// let groups = registry.list_groups();
    /// assert_eq!(groups.len(), 2);
    /// ```
    pub fn list_groups(&self) -> Vec<Arc<str>> {
        self.groups.keys().cloned().collect()
    }

    /// Get the active profile name for a group.
    ///
    /// # Arguments
    ///
    /// * `name` - The group name
    ///
    /// # Returns
    ///
    /// Returns [`Some`] with the active profile name, or [`None`] if the group doesn't exist.
    ///
    /// # Example
    ///
    /// ```rust
    /// use confers::modules::ModuleRegistry;
    /// use std::path::PathBuf;
    ///
    /// let mut registry = ModuleRegistry::new();
    /// registry.register_group("database", vec![], Some("mysql"));
    ///
    /// let profile = registry.get_active_profile("database");
    /// assert_eq!(profile.as_deref(), Some("mysql"));
    /// ```
    pub fn get_active_profile(&self, name: &str) -> Option<Arc<str>> {
        self.groups.get(name).map(|m| m.active_profile.clone())
    }

    /// Set the active profile for a group.
    ///
    /// # Arguments
    ///
    /// * `name` - The group name
    /// * `profile` - The profile name to set as active
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::ModuleNotFound`] if the group or profile doesn't exist.
    ///
    /// # Example
    ///
    /// ```rust
    /// use confers::modules::ModuleRegistry;
    /// use std::path::PathBuf;
    ///
    /// let mut registry = ModuleRegistry::new();
    /// registry.register_group(
    ///     "database",
    ///     vec![
    ///         ("mysql", PathBuf::from("conf/db/mysql.toml")),
    ///         ("postgresql", PathBuf::from("conf/db/postgresql.toml")),
    ///     ],
    ///     Some("mysql"),
    /// );
    ///
    /// registry.set_active_profile("database", "postgresql");
    /// assert_eq!(registry.get_active_profile("database").unwrap().as_ref(), "postgresql");
    /// ```
    pub fn set_active_profile(&mut self, name: &str, profile: &str) -> ConfigResult<()> {
        let module = self
            .groups
            .get_mut(name)
            .ok_or_else(|| ConfigError::ModuleNotFound {
                group: name.to_string(),
                module: profile.to_string(),
            })?;

        if !module.has_profile(profile) {
            return Err(ConfigError::ModuleNotFound {
                group: name.to_string(),
                module: profile.to_string(),
            });
        }

        module.active_profile = Arc::from(profile);
        Ok(())
    }

    /// Get a module config by name.
    ///
    /// # Arguments
    ///
    /// * `name` - The group name
    ///
    /// # Returns
    ///
    /// Returns [`Some`] with the module config, or [`None`] if the group doesn't exist.
    pub fn get(&self, name: &str) -> Option<&ModuleConfig> {
        self.groups.get(name)
    }

    /// Check if a group exists.
    ///
    /// # Arguments
    ///
    /// * `name` - The group name
    pub fn contains(&self, name: &str) -> bool {
        self.groups.contains_key(name)
    }

    /// Get the number of registered groups.
    pub fn len(&self) -> usize {
        self.groups.len()
    }

    /// Check if the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.groups.is_empty()
    }

    /// Resolve module profiles from environment variables.
    ///
    /// This method allows runtime configuration of which profile to use for each group
    /// via environment variables. The environment variable format is:
    /// `{prefix}{GROUP_NAME}_PROFILE` (uppercase).
    ///
    /// # Arguments
    ///
    /// * `prefix` - Optional prefix for environment variables (e.g., "APP_")
    ///
    /// # Example
    ///
    /// ```rust
    /// use confers::modules::ModuleRegistry;
    /// use std::path::PathBuf;
    ///
    /// // Given environment variable APP_DATABASE_PROFILE=postgresql
    /// std::env::set_var("APP_DATABASE_PROFILE", "postgresql");
    ///
    /// let mut registry = ModuleRegistry::new();
    /// registry.register_group(
    ///     "database",
    ///     vec![
    ///         ("mysql", PathBuf::from("conf/db/mysql.toml")),
    ///         ("postgresql", PathBuf::from("conf/db/postgresql.toml")),
    ///     ],
    ///     Some("mysql"),
    /// );
    ///
    /// registry.resolve_from_env(Some("APP_"));
    /// // Active profile for "database" is now "postgresql"
    ///
    /// std::env::remove_var("APP_DATABASE_PROFILE");
    /// ```
    pub fn resolve_from_env(&mut self, prefix: Option<&str>) -> &mut Self {
        let prefix_str = prefix.unwrap_or("");

        for (name, module) in self.groups.iter_mut() {
            let env_key = format!("{}{}_PROFILE", prefix_str, name.to_uppercase());

            if let Ok(profile) = std::env::var(&env_key) {
                if module.has_profile(&profile) {
                    module.active_profile = Arc::from(profile);
                    tracing::debug!(
                        group = name.as_ref(),
                        profile = module.active_profile.as_ref(),
                        env_key = env_key,
                        "Resolved module profile from environment"
                    );
                } else {
                    tracing::warn!(
                        group = name.as_ref(),
                        requested = profile,
                        available = ?module.profiles(),
                        env_key = env_key,
                        "Environment variable specifies non-existent profile, ignoring"
                    );
                }
            }
        }

        self
    }

    /// Resolve a single module's profile from environment variable.
    ///
    /// # Arguments
    ///
    /// * `group_name` - The group name to resolve
    /// * `prefix` - Optional prefix for environment variables
    ///
    /// # Returns
    ///
    /// Returns `Ok(true)` if the profile was changed, `Ok(false)` if no env var was set,
    /// or an error if the group doesn't exist.
    pub fn resolve_module_from_env(
        &mut self,
        group_name: &str,
        prefix: Option<&str>,
    ) -> ConfigResult<bool> {
        let module =
            self.groups
                .get_mut(group_name)
                .ok_or_else(|| ConfigError::ModuleNotFound {
                    group: group_name.to_string(),
                    module: "env".to_string(),
                })?;

        let prefix_str = prefix.unwrap_or("");
        let env_key = format!("{}{}_PROFILE", prefix_str, group_name.to_uppercase());

        if let Ok(profile) = std::env::var(&env_key) {
            if module.has_profile(&profile) {
                module.active_profile = Arc::from(profile);
                return Ok(true);
            } else {
                tracing::warn!(
                    group = group_name,
                    requested = profile,
                    available = ?module.profiles(),
                    "Environment variable specifies non-existent profile"
                );
            }
        }

        Ok(false)
    }

    /// Get all active profiles as a map.
    ///
    /// # Returns
    ///
    /// A HashMap mapping group names to their active profile names.
    pub fn active_profiles(&self) -> HashMap<Arc<str>, Arc<str>> {
        self.groups
            .iter()
            .map(|(name, module)| (name.clone(), module.active_profile.clone()))
            .collect()
    }

    /// Validate that all active profiles have valid file paths.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if all active profiles are valid, or an error with details.
    pub fn validate_active_profiles(&self) -> ConfigResult<()> {
        for (name, module) in &self.groups {
            let path = module.get_profile(&module.active_profile).ok_or_else(|| {
                ConfigError::ModuleNotFound {
                    group: name.to_string(),
                    module: module.active_profile.to_string(),
                }
            })?;

            if !path.exists() {
                return Err(ConfigError::FileNotFound {
                    filename: path.clone(),
                    source: None,
                });
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_config(temp_dir: &TempDir, filename: &str, content: &str) -> PathBuf {
        let path = temp_dir.path().join(filename);
        std::fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn test_module_registry_new() {
        let registry = ModuleRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_module_registry_with_capacity() {
        let registry = ModuleRegistry::with_capacity(10);
        assert!(registry.is_empty());
    }

    #[test]
    fn test_register_group() {
        let mut registry = ModuleRegistry::new();

        registry.register_group(
            "database",
            vec![
                ("mysql", PathBuf::from("conf/db/mysql.toml")),
                ("postgresql", PathBuf::from("conf/db/postgresql.toml")),
            ],
            Some("mysql"),
        );

        assert!(!registry.is_empty());
        assert_eq!(registry.len(), 1);
        assert!(registry.contains("database"));
    }

    #[test]
    fn test_register_multiple_groups() {
        let mut registry = ModuleRegistry::new();

        registry.register_group("database", vec![], None);
        registry.register_group("cache", vec![], None);

        assert_eq!(registry.len(), 2);
    }

    #[test]
    fn test_list_groups() {
        let mut registry = ModuleRegistry::new();

        registry.register_group("database", vec![], None);
        registry.register_group("cache", vec![], None);
        registry.register_group("logging", vec![], None);

        let groups = registry.list_groups();
        assert_eq!(groups.len(), 3);
    }

    #[test]
    fn test_get_active_profile() {
        let mut registry = ModuleRegistry::new();

        registry.register_group(
            "database",
            vec![
                ("mysql", PathBuf::from("conf/db/mysql.toml")),
                ("postgresql", PathBuf::from("conf/db/postgresql.toml")),
            ],
            Some("postgresql"),
        );

        let profile = registry.get_active_profile("database");
        assert_eq!(profile.unwrap().as_ref(), "postgresql");
    }

    #[test]
    fn test_get_active_profile_nonexistent() {
        let registry = ModuleRegistry::new();

        let profile = registry.get_active_profile("nonexistent");
        assert!(profile.is_none());
    }

    #[test]
    fn test_set_active_profile() {
        let mut registry = ModuleRegistry::new();

        registry.register_group(
            "database",
            vec![
                ("mysql", PathBuf::from("conf/db/mysql.toml")),
                ("postgresql", PathBuf::from("conf/db/postgresql.toml")),
            ],
            Some("mysql"),
        );

        let result = registry.set_active_profile("database", "postgresql");
        assert!(result.is_ok());

        let profile = registry.get_active_profile("database").unwrap();
        assert_eq!(profile.as_ref(), "postgresql");
    }

    #[test]
    fn test_set_active_profile_nonexistent_group() {
        let mut registry = ModuleRegistry::new();

        let result = registry.set_active_profile("nonexistent", "profile");
        assert!(result.is_err());

        if let Err(ConfigError::ModuleNotFound { group, module }) = result {
            assert_eq!(group, "nonexistent");
            assert_eq!(module, "profile");
        } else {
            panic!("Expected ModuleNotFound error");
        }
    }

    #[test]
    fn test_set_active_profile_nonexistent_profile() {
        let mut registry = ModuleRegistry::new();

        registry.register_group(
            "database",
            vec![("mysql", PathBuf::from("conf/db/mysql.toml"))],
            Some("mysql"),
        );

        let result = registry.set_active_profile("database", "nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_module_config_get_profile() {
        let config = ModuleConfig::new(
            "database",
            vec![
                ("mysql", PathBuf::from("conf/db/mysql.toml")),
                ("postgresql", PathBuf::from("conf/db/postgresql.toml")),
            ],
            Some("mysql"),
        );

        let path = config.get_profile("mysql");
        assert!(path.is_some());
        assert_eq!(path.unwrap(), &PathBuf::from("conf/db/mysql.toml"));

        let path = config.get_profile("postgresql");
        assert!(path.is_some());

        let path = config.get_profile("nonexistent");
        assert!(path.is_none());
    }

    #[test]
    fn test_module_config_has_profile() {
        let config = ModuleConfig::new(
            "database",
            vec![("mysql", PathBuf::from("conf/db/mysql.toml"))],
            Some("mysql"),
        );

        assert!(config.has_profile("mysql"));
        assert!(!config.has_profile("postgresql"));
    }

    #[test]
    fn test_module_config_profiles() {
        let config = ModuleConfig::new(
            "database",
            vec![
                ("mysql", PathBuf::from("conf/db/mysql.toml")),
                ("postgresql", PathBuf::from("conf/db/postgresql.toml")),
                ("sqlite", PathBuf::from("conf/db/sqlite.toml")),
            ],
            None,
        );

        let profiles = config.profiles();
        assert_eq!(profiles.len(), 3);
    }

    #[test]
    fn test_module_config_default_profile() {
        // With default
        let config = ModuleConfig::new(
            "database",
            vec![
                ("mysql", PathBuf::from("conf/db/mysql.toml")),
                ("postgresql", PathBuf::from("conf/db/postgresql.toml")),
            ],
            Some("postgresql"),
        );
        assert_eq!(config.active_profile.as_ref(), "postgresql");

        // Without default - should use first
        let config = ModuleConfig::new(
            "database",
            vec![
                ("mysql", PathBuf::from("conf/db/mysql.toml")),
                ("postgresql", PathBuf::from("conf/db/postgresql.toml")),
            ],
            None,
        );
        assert_eq!(config.active_profile.as_ref(), "mysql");
    }

    #[test]
    fn test_load_module_not_found_group() {
        let registry = ModuleRegistry::new();

        let result = registry.load_module("nonexistent", "profile", &LoaderConfig::default());

        assert!(result.is_err());
        if let Err(ConfigError::ModuleNotFound { group, module }) = result {
            assert_eq!(group, "nonexistent");
            assert_eq!(module, "profile");
        } else {
            panic!("Expected ModuleNotFound error");
        }
    }

    #[test]
    fn test_load_module_not_found_profile() {
        let mut registry = ModuleRegistry::new();

        registry.register_group(
            "database",
            vec![("mysql", PathBuf::from("conf/db/mysql.toml"))],
            Some("mysql"),
        );

        let result = registry.load_module("database", "nonexistent", &LoaderConfig::default());

        assert!(result.is_err());
    }

    #[test]
    fn test_load_module_file_not_found() {
        let temp_dir = TempDir::new().unwrap();

        let mut registry = ModuleRegistry::new();

        registry.register_group(
            "database",
            vec![("mysql", temp_dir.path().join("nonexistent.toml"))],
            Some("mysql"),
        );

        let result = registry.load_module("database", "mysql", &LoaderConfig::default());

        assert!(result.is_err());
    }

    #[test]
    fn test_load_module_success() {
        let temp_dir = TempDir::new().unwrap();

        let mysql_path = create_test_config(
            &temp_dir,
            "mysql.toml",
            "host = \"localhost\"\nport = 3306\n",
        );

        let mut registry = ModuleRegistry::new();

        registry.register_group("database", vec![("mysql", mysql_path)], Some("mysql"));

        let result = registry.load_module("database", "mysql", &LoaderConfig::default());

        assert!(result.is_ok());
        let value = result.unwrap();
        // Check that the value contains the expected key
        if let ConfigValue::Map(m) = &value.inner {
            assert!(m.contains_key("host"));
        } else {
            // The value might be a different type, just check it exists
            assert!(!matches!(value.inner, ConfigValue::Null));
        }
    }

    #[test]
    fn test_load_active_not_found() {
        let registry = ModuleRegistry::new();

        let result = registry.load_active("nonexistent", &LoaderConfig::default());

        assert!(result.is_err());
    }

    #[test]
    fn test_get_module_config() {
        let mut registry = ModuleRegistry::new();

        registry.register_group(
            "database",
            vec![("mysql", PathBuf::from("conf/db/mysql.toml"))],
            Some("mysql"),
        );

        let config = registry.get("database");
        assert!(config.is_some());

        let config = config.unwrap();
        assert_eq!(config.name.as_ref(), "database");
        assert_eq!(config.active_profile.as_ref(), "mysql");

        let nonexistent = registry.get("nonexistent");
        assert!(nonexistent.is_none());
    }
}
