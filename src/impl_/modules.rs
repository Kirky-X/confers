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
//! let mut registry = ModuleRegistry::default();
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
use crate::impl_::loader::{load_file, LoaderConfig};
#[allow(unused_imports)]
use crate::types::{AnnotatedValue, ConfigValue};
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
    name: Arc<str>,
    /// Available profile paths as (profile_name, path).
    paths: Vec<(Arc<str>, PathBuf)>,
    /// The currently active profile for this module.
    active_profile: Arc<str>,
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

    /// Get the module name.
    pub fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn name_arc(&self) -> Arc<str> {
        self.name.clone()
    }

    /// Get the active profile name.
    pub fn active_profile(&self) -> &str {
        &self.active_profile
    }

    pub(crate) fn active_profile_arc(&self) -> Arc<str> {
        self.active_profile.clone()
    }

    /// Get list of available profiles.
    pub fn profiles(&self) -> Vec<Arc<str>> {
        self.paths.iter().map(|(n, _)| n.clone()).collect()
    }

    pub fn profile_count(&self) -> usize {
        self.paths.len()
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

    pub fn set_active_profile(&mut self, profile: &str) -> Result<(), ConfigError> {
        if !self.has_profile(profile) {
            return Err(ConfigError::ModuleNotFound {
                group: self.name.to_string(),
                module: profile.to_string(),
            });
        }
        self.active_profile = Arc::from(profile);
        Ok(())
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
    /// let mut registry = ModuleRegistry::default();
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
        self.groups.insert(config.name_arc(), config);
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
    /// let mut registry = ModuleRegistry::default();
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

        self.load_module(name, module.active_profile(), config)
    }

    /// List all registered group names.
    ///
    /// # Example
    ///
    /// ```rust
    /// use confers::modules::ModuleRegistry;
    /// use std::path::PathBuf;
    ///
    /// let mut registry = ModuleRegistry::default();
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
    /// let mut registry = ModuleRegistry::default();
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
    /// let mut registry = ModuleRegistry::default();
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

        module.set_active_profile(profile)?;
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
    /// let mut registry = ModuleRegistry::default();
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
                    module.set_active_profile(&profile).ok();
                }
                // Silently ignore non-existent profile - not an error condition
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
                module.set_active_profile(&profile).ok();
                return Ok(true);
            }
            // Silently ignore non-existent profile - not an error condition
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
            .map(|(name, module)| (name.clone(), module.active_profile_arc()))
            .collect()
    }

    /// Validate that all active profiles have valid file paths.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if all active profiles are valid, or an error with details.
    pub fn validate_active_profiles(&self) -> ConfigResult<()> {
        for (name, module) in &self.groups {
            let path = module.get_profile(module.active_profile()).ok_or_else(|| {
                ConfigError::ModuleNotFound {
                    group: name.to_string(),
                    module: module.active_profile().to_string(),
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
        let registry = ModuleRegistry::default();
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
        let mut registry = ModuleRegistry::default();

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
        let mut registry = ModuleRegistry::default();

        registry.register_group("database", vec![], None);
        registry.register_group("cache", vec![], None);

        assert_eq!(registry.len(), 2);
    }

    #[test]
    fn test_list_groups() {
        let mut registry = ModuleRegistry::default();

        registry.register_group("database", vec![], None);
        registry.register_group("cache", vec![], None);
        registry.register_group("logging", vec![], None);

        let groups = registry.list_groups();
        assert_eq!(groups.len(), 3);
    }

    #[test]
    fn test_get_active_profile() {
        let mut registry = ModuleRegistry::default();

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
        let registry = ModuleRegistry::default();

        let profile = registry.get_active_profile("nonexistent");
        assert!(profile.is_none());
    }

    #[test]
    fn test_set_active_profile() {
        let mut registry = ModuleRegistry::default();

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
        let mut registry = ModuleRegistry::default();

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
        let mut registry = ModuleRegistry::default();

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
        assert_eq!(config.active_profile(), "postgresql");

        // Without default - should use first
        let config = ModuleConfig::new(
            "database",
            vec![
                ("mysql", PathBuf::from("conf/db/mysql.toml")),
                ("postgresql", PathBuf::from("conf/db/postgresql.toml")),
            ],
            None,
        );
        assert_eq!(config.active_profile(), "mysql");
    }

    #[test]
    fn test_load_module_not_found_group() {
        let registry = ModuleRegistry::default();

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
        let mut registry = ModuleRegistry::default();

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

        let mut registry = ModuleRegistry::default();

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

        let mut registry = ModuleRegistry::default();

        registry.register_group("database", vec![("mysql", mysql_path)], Some("mysql"));

        // Disable symlink checking, allow absolute paths, and clear allowed dirs for temp directory paths
        let config = LoaderConfig::new()
            .no_symlink_check()
            .allow_absolute()
            .allowed_dirs(Vec::<PathBuf>::new());
        let result = registry.load_module("database", "mysql", &config);

        if let Err(ref e) = result {
            eprintln!("Error loading module: {:?}", e);
        }
        assert!(result.is_ok(), "Module load should succeed");
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
        let registry = ModuleRegistry::default();

        let result = registry.load_active("nonexistent", &LoaderConfig::default());

        assert!(result.is_err());
    }

    #[test]
    fn test_get_module_config() {
        let mut registry = ModuleRegistry::default();

        registry.register_group(
            "database",
            vec![("mysql", PathBuf::from("conf/db/mysql.toml"))],
            Some("mysql"),
        );

        let config = registry.get("database");
        assert!(config.is_some());

        let config = config.unwrap();
        assert_eq!(config.name(), "database");
        assert_eq!(config.active_profile(), "mysql");

        let nonexistent = registry.get("nonexistent");
        assert!(nonexistent.is_none());
    }

    #[test]
    fn test_module_config_profile_count_zero() {
        let config = ModuleConfig::new("empty", vec![], None);
        assert_eq!(config.profile_count(), 0);
        assert!(config.profiles().is_empty());
    }

    #[test]
    fn test_module_config_profile_count_multiple() {
        let config = ModuleConfig::new(
            "database",
            vec![
                ("mysql", PathBuf::from("conf/db/mysql.toml")),
                ("postgresql", PathBuf::from("conf/db/postgresql.toml")),
                ("sqlite", PathBuf::from("conf/db/sqlite.toml")),
            ],
            None,
        );
        assert_eq!(config.profile_count(), 3);
        assert_eq!(config.profiles().len(), 3);
    }

    #[test]
    fn test_module_config_new_empty_paths_no_default() {
        // When paths is empty and no default provided, active_profile is "".
        let config = ModuleConfig::new("orphan", vec![], None);
        assert_eq!(config.active_profile(), "");
        assert_eq!(config.profile_count(), 0);
        assert!(config.profiles().is_empty());
    }

    #[test]
    fn test_module_config_new_empty_paths_with_default() {
        // Default is honored even when paths is empty (active_profile = default).
        let config = ModuleConfig::new("ghost", vec![], Some("phantom"));
        assert_eq!(config.active_profile(), "phantom");
    }

    #[test]
    fn test_module_config_set_active_profile_success() {
        let mut config = ModuleConfig::new(
            "database",
            vec![
                ("mysql", PathBuf::from("conf/db/mysql.toml")),
                ("postgresql", PathBuf::from("conf/db/postgresql.toml")),
            ],
            Some("mysql"),
        );
        assert_eq!(config.active_profile(), "mysql");
        config.set_active_profile("postgresql").expect("set_active");
        assert_eq!(config.active_profile(), "postgresql");
    }

    #[test]
    fn test_module_config_set_active_profile_nonexistent_errors() {
        let mut config = ModuleConfig::new(
            "database",
            vec![("mysql", PathBuf::from("conf/db/mysql.toml"))],
            Some("mysql"),
        );
        let err = config.set_active_profile("nonexistent").unwrap_err();
        match err {
            ConfigError::ModuleNotFound { group, module } => {
                assert_eq!(group, "database");
                assert_eq!(module, "nonexistent");
            }
            other => panic!("expected ModuleNotFound, got {:?}", other),
        }
        // Active profile must remain unchanged after the error
        assert_eq!(config.active_profile(), "mysql");
    }

    #[test]
    fn test_module_config_set_active_profile_idempotent() {
        let mut config = ModuleConfig::new(
            "database",
            vec![("mysql", PathBuf::from("conf/db/mysql.toml"))],
            Some("mysql"),
        );
        config
            .set_active_profile("mysql")
            .expect("set same profile");
        assert_eq!(config.active_profile(), "mysql");
    }

    #[test]
    fn test_register_group_returns_self_for_chaining() {
        let mut registry = ModuleRegistry::default();
        // register_group returns &mut Self, enabling builder-style chaining.
        registry
            .register_group("a", vec![], None)
            .register_group("b", vec![], None)
            .register_group("c", vec![], None);
        assert_eq!(registry.len(), 3);
        assert!(registry.contains("a"));
        assert!(registry.contains("b"));
        assert!(registry.contains("c"));
    }

    #[test]
    fn test_register_group_overwrites_existing() {
        let mut registry = ModuleRegistry::default();
        registry.register_group(
            "database",
            vec![("mysql", PathBuf::from("old/mysql.toml"))],
            Some("mysql"),
        );
        // Re-register the same group with different profiles.
        registry.register_group(
            "database",
            vec![("postgresql", PathBuf::from("new/postgresql.toml"))],
            Some("postgresql"),
        );
        assert_eq!(registry.len(), 1, "re-register should replace, not add");
        let config = registry.get("database").unwrap();
        assert_eq!(config.active_profile(), "postgresql");
        assert!(!config.has_profile("mysql"));
        assert!(config.has_profile("postgresql"));
    }

    #[test]
    fn test_contains_group() {
        let mut registry = ModuleRegistry::default();
        registry.register_group("present", vec![], None);
        assert!(registry.contains("present"));
        assert!(!registry.contains("absent"));
    }

    #[test]
    fn test_load_active_success() {
        let temp_dir = TempDir::new().unwrap();
        let mysql_path = create_test_config(&temp_dir, "mysql.toml", "host = \"localhost\"\n");
        let postgresql_path =
            create_test_config(&temp_dir, "postgresql.toml", "host = \"pg-host\"\n");

        let mut registry = ModuleRegistry::default();
        registry.register_group(
            "database",
            vec![("mysql", mysql_path), ("postgresql", postgresql_path)],
            Some("mysql"),
        );

        let config = LoaderConfig::new()
            .no_symlink_check()
            .allow_absolute()
            .allowed_dirs(Vec::<PathBuf>::new());
        // load_active uses the currently-active profile ("mysql").
        let result = registry.load_active("database", &config);
        assert!(result.is_ok(), "load_active should succeed: {:?}", result);

        // Now switch active profile and load again.
        registry
            .set_active_profile("database", "postgresql")
            .unwrap();
        let result = registry.load_active("database", &config);
        assert!(result.is_ok(), "load_active with new profile: {:?}", result);
    }

    #[test]
    fn test_active_profiles_empty_registry() {
        let registry = ModuleRegistry::default();
        let map = registry.active_profiles();
        assert!(map.is_empty());
    }

    #[test]
    fn test_active_profiles_returns_all_groups() {
        let mut registry = ModuleRegistry::default();
        registry.register_group(
            "database",
            vec![("mysql", PathBuf::from("conf/db/mysql.toml"))],
            Some("mysql"),
        );
        registry.register_group(
            "cache",
            vec![("redis", PathBuf::from("conf/cache/redis.toml"))],
            Some("redis"),
        );
        let map = registry.active_profiles();
        assert_eq!(map.len(), 2);
        assert_eq!(map.get("database").map(|s| s.as_ref()), Some("mysql"));
        assert_eq!(map.get("cache").map(|s| s.as_ref()), Some("redis"));
    }

    #[test]
    fn test_active_profiles_reflects_set_active_profile() {
        let mut registry = ModuleRegistry::default();
        registry.register_group(
            "database",
            vec![
                ("mysql", PathBuf::from("conf/db/mysql.toml")),
                ("postgresql", PathBuf::from("conf/db/postgresql.toml")),
            ],
            Some("mysql"),
        );
        let before = registry.active_profiles();
        assert_eq!(before.get("database").map(|s| s.as_ref()), Some("mysql"));

        registry
            .set_active_profile("database", "postgresql")
            .unwrap();
        let after = registry.active_profiles();
        assert_eq!(
            after.get("database").map(|s| s.as_ref()),
            Some("postgresql")
        );
    }

    #[test]
    fn test_validate_active_profiles_empty_registry_ok() {
        let registry = ModuleRegistry::default();
        registry
            .validate_active_profiles()
            .expect("empty registry has no profiles to validate");
    }

    #[test]
    fn test_validate_active_profiles_all_exist_ok() {
        let temp_dir = TempDir::new().unwrap();
        let mysql_path = create_test_config(&temp_dir, "mysql.toml", "host = \"localhost\"\n");

        let mut registry = ModuleRegistry::default();
        registry.register_group("database", vec![("mysql", mysql_path)], Some("mysql"));
        registry
            .validate_active_profiles()
            .expect("all active profile files exist");
    }

    #[test]
    fn test_validate_active_profiles_missing_file_errors() {
        let mut registry = ModuleRegistry::default();
        registry.register_group(
            "database",
            vec![("mysql", PathBuf::from("/nonexistent/mysql.toml"))],
            Some("mysql"),
        );
        let err = registry.validate_active_profiles().unwrap_err();
        match err {
            ConfigError::FileNotFound { filename, .. } => {
                assert!(filename.to_string_lossy().contains("mysql.toml"));
            }
            other => panic!("expected FileNotFound, got {:?}", other),
        }
    }

    #[test]
    fn test_validate_active_profiles_checks_active_profile_not_listed() {
        // Edge case: active_profile points to a profile that has no path entry.
        // This can only happen if the module was constructed with an empty paths
        // list but a non-empty default.
        let mut registry = ModuleRegistry::default();
        // Register a group with no profiles but a default active profile.
        registry.register_group("ghost", vec![], Some("phantom"));
        let err = registry.validate_active_profiles().unwrap_err();
        match err {
            ConfigError::ModuleNotFound { group, module } => {
                assert_eq!(group, "ghost");
                assert_eq!(module, "phantom");
            }
            other => panic!("expected ModuleNotFound, got {:?}", other),
        }
    }

    #[test]
    fn test_list_groups_empty_registry() {
        let registry = ModuleRegistry::default();
        assert!(registry.list_groups().is_empty());
    }

    #[test]
    fn test_list_groups_returns_all_registered() {
        let mut registry = ModuleRegistry::default();
        registry.register_group("alpha", vec![], None);
        registry.register_group("beta", vec![], None);
        registry.register_group("gamma", vec![], None);
        let groups = registry.list_groups();
        assert_eq!(groups.len(), 3);
        // Each group name should be present (order is not guaranteed by HashMap).
        let names: Vec<String> = groups.iter().map(|s| s.to_string()).collect();
        assert!(names.contains(&"alpha".to_string()));
        assert!(names.contains(&"beta".to_string()));
        assert!(names.contains(&"gamma".to_string()));
    }

    #[test]
    fn test_get_returns_none_for_missing_group() {
        let registry = ModuleRegistry::default();
        assert!(registry.get("does-not-exist").is_none());
    }

    #[test]
    fn test_len_and_is_empty_consistency() {
        let mut registry = ModuleRegistry::default();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
        registry.register_group("one", vec![], None);
        assert!(!registry.is_empty());
        assert_eq!(registry.len(), 1);
        registry.register_group("two", vec![], None);
        assert_eq!(registry.len(), 2);
    }

    #[test]
    #[serial_test::serial]
    fn test_resolve_module_from_env_changes_profile() {
        // Use a unique group name to avoid collision with other env-var tests.
        let unique_group = "envresolve_single_ok";
        let env_key = format!("{}_PROFILE", unique_group.to_uppercase());
        std::env::set_var(&env_key, "postgresql");

        let mut registry = ModuleRegistry::default();
        registry.register_group(
            unique_group,
            vec![
                ("mysql", PathBuf::from("conf/db/mysql.toml")),
                ("postgresql", PathBuf::from("conf/db/postgresql.toml")),
            ],
            Some("mysql"),
        );

        let changed = registry
            .resolve_module_from_env(unique_group, None)
            .expect("resolve_module_from_env");
        assert!(changed, "profile should have changed due to env var");
        assert_eq!(
            registry.get_active_profile(unique_group).unwrap().as_ref(),
            "postgresql"
        );

        std::env::remove_var(&env_key);
    }

    #[test]
    #[serial_test::serial]
    fn test_resolve_module_from_env_no_env_var_returns_false() {
        let unique_group = "envresolve_single_novar";
        let env_key = format!("{}_PROFILE", unique_group.to_uppercase());
        std::env::remove_var(&env_key);

        let mut registry = ModuleRegistry::default();
        registry.register_group(
            unique_group,
            vec![
                ("mysql", PathBuf::from("conf/db/mysql.toml")),
                ("postgresql", PathBuf::from("conf/db/postgresql.toml")),
            ],
            Some("mysql"),
        );

        let changed = registry
            .resolve_module_from_env(unique_group, None)
            .expect("resolve_module_from_env");
        assert!(!changed, "no env var set → must return false");
        assert_eq!(
            registry.get_active_profile(unique_group).unwrap().as_ref(),
            "mysql"
        );
    }

    #[test]
    #[serial_test::serial]
    fn test_resolve_module_from_env_nonexistent_group_errors() {
        let err = registry_helper_nonexistent_group();
        match err {
            ConfigError::ModuleNotFound { group, module } => {
                assert_eq!(group, "definitely_not_registered");
                assert_eq!(module, "env");
            }
            other => panic!("expected ModuleNotFound, got {:?}", other),
        }
    }

    fn registry_helper_nonexistent_group() -> ConfigError {
        let mut registry = ModuleRegistry::default();
        registry
            .resolve_module_from_env("definitely_not_registered", None)
            .unwrap_err()
    }

    #[test]
    #[serial_test::serial]
    fn test_resolve_module_from_env_invalid_profile_ignored() {
        // If the env var points to a non-existent profile, the method must NOT
        // error — it silently ignores the invalid profile and returns false.
        let unique_group = "envresolve_single_bad_profile";
        let env_key = format!("{}_PROFILE", unique_group.to_uppercase());
        std::env::set_var(&env_key, "nonexistent-profile");

        let mut registry = ModuleRegistry::default();
        registry.register_group(
            unique_group,
            vec![("mysql", PathBuf::from("conf/db/mysql.toml"))],
            Some("mysql"),
        );

        let changed = registry
            .resolve_module_from_env(unique_group, None)
            .expect("must not error for invalid profile");
        assert!(
            !changed,
            "invalid profile should be ignored (changed=false)"
        );
        assert_eq!(
            registry.get_active_profile(unique_group).unwrap().as_ref(),
            "mysql"
        );

        std::env::remove_var(&env_key);
    }

    #[test]
    #[serial_test::serial]
    fn test_resolve_module_from_env_with_prefix() {
        let unique_group = "envresolve_prefix";
        let env_key = format!("PREFIX_{}_PROFILE", unique_group.to_uppercase());
        std::env::set_var(&env_key, "postgresql");

        let mut registry = ModuleRegistry::default();
        registry.register_group(
            unique_group,
            vec![
                ("mysql", PathBuf::from("conf/db/mysql.toml")),
                ("postgresql", PathBuf::from("conf/db/postgresql.toml")),
            ],
            Some("mysql"),
        );

        let changed = registry
            .resolve_module_from_env(unique_group, Some("PREFIX_"))
            .expect("resolve_module_from_env with prefix");
        assert!(changed);
        assert_eq!(
            registry.get_active_profile(unique_group).unwrap().as_ref(),
            "postgresql"
        );

        std::env::remove_var(&env_key);
    }

    #[test]
    #[serial_test::serial]
    fn test_resolve_from_env_updates_all_matching_groups() {
        let unique_a = "envresolve_batch_a";
        let unique_b = "envresolve_batch_b";
        let env_a = format!("{}_PROFILE", unique_a.to_uppercase());
        let env_b = format!("{}_PROFILE", unique_b.to_uppercase());
        std::env::set_var(&env_a, "postgresql");
        std::env::set_var(&env_b, "redis");

        let mut registry = ModuleRegistry::default();
        registry.register_group(
            unique_a,
            vec![
                ("mysql", PathBuf::from("conf/db/mysql.toml")),
                ("postgresql", PathBuf::from("conf/db/postgresql.toml")),
            ],
            Some("mysql"),
        );
        registry.register_group(
            unique_b,
            vec![
                ("memory", PathBuf::from("conf/cache/memory.toml")),
                ("redis", PathBuf::from("conf/cache/redis.toml")),
            ],
            Some("memory"),
        );

        registry.resolve_from_env(None);
        assert_eq!(
            registry.get_active_profile(unique_a).unwrap().as_ref(),
            "postgresql"
        );
        assert_eq!(
            registry.get_active_profile(unique_b).unwrap().as_ref(),
            "redis"
        );

        std::env::remove_var(&env_a);
        std::env::remove_var(&env_b);
    }

    #[test]
    #[serial_test::serial]
    fn test_resolve_from_env_ignores_nonexistent_profile() {
        let unique_group = "envresolve_batch_ignore";
        let env_key = format!("{}_PROFILE", unique_group.to_uppercase());
        std::env::set_var(&env_key, "does-not-exist");

        let mut registry = ModuleRegistry::default();
        registry.register_group(
            unique_group,
            vec![("mysql", PathBuf::from("conf/db/mysql.toml"))],
            Some("mysql"),
        );

        // Must not panic and must not change the active profile.
        registry.resolve_from_env(None);
        assert_eq!(
            registry.get_active_profile(unique_group).unwrap().as_ref(),
            "mysql"
        );

        std::env::remove_var(&env_key);
    }

    #[test]
    #[serial_test::serial]
    fn test_resolve_from_env_no_env_vars_is_noop() {
        let unique_group = "envresolve_batch_noop";
        let env_key = format!("{}_PROFILE", unique_group.to_uppercase());
        std::env::remove_var(&env_key);

        let mut registry = ModuleRegistry::default();
        registry.register_group(
            unique_group,
            vec![
                ("mysql", PathBuf::from("conf/db/mysql.toml")),
                ("postgresql", PathBuf::from("conf/db/postgresql.toml")),
            ],
            Some("mysql"),
        );

        registry.resolve_from_env(None);
        assert_eq!(
            registry.get_active_profile(unique_group).unwrap().as_ref(),
            "mysql"
        );
    }

    #[test]
    #[serial_test::serial]
    fn test_resolve_from_env_with_prefix() {
        let unique_group = "envresolve_batch_prefix";
        let env_key = format!("APP_{}_PROFILE", unique_group.to_uppercase());
        std::env::set_var(&env_key, "postgresql");

        let mut registry = ModuleRegistry::default();
        registry.register_group(
            unique_group,
            vec![
                ("mysql", PathBuf::from("conf/db/mysql.toml")),
                ("postgresql", PathBuf::from("conf/db/postgresql.toml")),
            ],
            Some("mysql"),
        );

        registry.resolve_from_env(Some("APP_"));
        assert_eq!(
            registry.get_active_profile(unique_group).unwrap().as_ref(),
            "postgresql"
        );

        std::env::remove_var(&env_key);
    }

    #[test]
    #[serial_test::serial]
    fn test_resolve_from_env_returns_self_for_chaining() {
        let unique_group = "envresolve_chain";
        let env_key = format!("{}_PROFILE", unique_group.to_uppercase());
        std::env::set_var(&env_key, "postgresql");

        let mut registry = ModuleRegistry::default();
        registry.register_group(
            unique_group,
            vec![
                ("mysql", PathBuf::from("conf/db/mysql.toml")),
                ("postgresql", PathBuf::from("conf/db/postgresql.toml")),
            ],
            Some("mysql"),
        );

        // resolve_from_env returns &mut Self, allowing chaining.
        registry
            .resolve_from_env(None)
            .register_group("added-after-resolve", vec![], None);
        assert!(registry.contains("added-after-resolve"));

        std::env::remove_var(&env_key);
    }

    #[test]
    fn test_module_config_name() {
        let config = ModuleConfig::new("my-group", vec![], None);
        assert_eq!(config.name(), "my-group");
    }

    #[test]
    fn test_module_config_get_profile_returns_path() {
        let path = PathBuf::from("conf/db/mysql.toml");
        let config = ModuleConfig::new("database", vec![("mysql", path.clone())], Some("mysql"));
        let got = config.get_profile("mysql").expect("profile must exist");
        assert_eq!(got, &path);
    }

    #[test]
    fn test_module_config_get_profile_missing_returns_none() {
        let config = ModuleConfig::new(
            "database",
            vec![("mysql", PathBuf::from("conf/db/mysql.toml"))],
            Some("mysql"),
        );
        assert!(config.get_profile("oracle").is_none());
    }

    #[test]
    fn test_module_config_active_profile_after_construction() {
        // With default
        let with_default = ModuleConfig::new(
            "db",
            vec![("mysql", PathBuf::from("m.toml"))],
            Some("mysql"),
        );
        assert_eq!(with_default.active_profile(), "mysql");

        // Without default, uses first profile
        let first_used = ModuleConfig::new(
            "db",
            vec![
                ("mysql", PathBuf::from("m.toml")),
                ("postgresql", PathBuf::from("p.toml")),
            ],
            None,
        );
        assert_eq!(first_used.active_profile(), "mysql");
    }

    #[test]
    fn test_module_registry_with_capacity_zero() {
        let registry = ModuleRegistry::with_capacity(0);
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_load_module_with_wrong_profile_name_errors() {
        let mut registry = ModuleRegistry::default();
        registry.register_group(
            "database",
            vec![("mysql", PathBuf::from("conf/db/mysql.toml"))],
            Some("mysql"),
        );
        let err = registry
            .load_module("database", "oracle", &LoaderConfig::default())
            .unwrap_err();
        match err {
            ConfigError::ModuleNotFound { group, module } => {
                assert_eq!(group, "database");
                assert_eq!(module, "oracle");
            }
            other => panic!("expected ModuleNotFound, got {:?}", other),
        }
    }
}
