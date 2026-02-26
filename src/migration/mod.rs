//! Configuration version migration module.
//!
//! This module provides:
//! - [`Versioned`] trait for versioned configuration types
//! - [`MigrationRegistry`] for managing and executing migrations
//! - [`MigrationOnReload`] enum for reload behavior control
//!
//! # Design
//!
//! The migration system uses a graph-based approach where:
//! - Nodes represent configuration versions
//! - Edges represent migration functions between versions
//! - Path precomputation uses BFS to find optimal migration paths
//!
//! # Example
//!
//! ```rust
//! use confers::migration::{MigrationRegistry, Versioned};
//! use confers::value::{AnnotatedValue, ConfigValue, SourceId};
//!
//! // Define versioned configs
//! struct ConfigV1;
//! struct ConfigV2;
//!
//! impl Versioned for ConfigV1 { const VERSION: u32 = 1; }
//! impl Versioned for ConfigV2 { const VERSION: u32 = 2; }
//!
//! // Create a test value
//! let value = AnnotatedValue::new(
//!     ConfigValue::null(),
//!     SourceId::new("test"),
//!     "test"
//! );
//!
//! // Create registry and register migrations
//! let mut registry = MigrationRegistry::new();
//! registry.register(1, 2, |mut v| {
//!     // Migrate from v1 to v2
//!     v.version = 2;
//!     Ok(v)
//! });
//!
//! // Precompute paths and migrate
//! registry.precompute_paths();
//! let result = registry.migrate(value, 1, 2);
//! ```

use crate::error::{ConfigError, ConfigResult};
use crate::value::AnnotatedValue;
use std::collections::{BTreeMap, HashMap};

/// Versioned configuration trait.
///
/// Implement this trait on your configuration structs to declare their version.
/// The version is used for migration tracking and automatic schema evolution.
///
/// # Example
///
/// ```rust
/// use confers::migration::Versioned;
///
/// struct AppConfigV1 { /* fields */ }
/// struct AppConfigV2 { /* fields */ }
///
/// impl Versioned for AppConfigV1 { const VERSION: u32 = 1; }
/// impl Versioned for AppConfigV2 { const VERSION: u32 = 2; }
/// ```
pub trait Versioned {
    /// The version number of this configuration schema.
    const VERSION: u32;
}

/// Migration function type.
///
/// Functions of this type transform a configuration from one version to another.
/// They receive the source annotated value and return the migrated value or an error.
///
/// Note: This is stored as a boxed closure internally to support both simple function
/// pointers and closures that capture environment variables.
pub type MigrationFn = Box<dyn FnMut(AnnotatedValue) -> ConfigResult<AnnotatedValue> + Send + Sync>;

/// Migration registry for managing version migrations.
///
/// This struct maintains a collection of migration functions and precomputes
/// optimal migration paths between versions using BFS.
///
/// # DI Patterns (following di.md)
///
/// - [`new()`][MigrationRegistry::new] - Zero-config default
/// - [`builder()`][MigrationRegistry::builder] - Builder pattern for fluent configuration
/// - [`with_migrations()`][MigrationRegistry::with_migrations] - Full dependency injection
pub struct MigrationRegistry {
    /// Migration functions keyed by (from_version, to_version).
    migrations: BTreeMap<(u32, u32), MigrationFn>,
    /// Precomputed migration paths for fast lookups.
    path_cache: HashMap<(u32, u32), Vec<(u32, u32)>>,
    /// All available versions (for path computation).
    versions: BTreeMap<u32, Vec<u32>>,
}

impl MigrationRegistry {
    /// Create a new empty migration registry.
    ///
    /// # Example
    ///
    /// ```rust
    /// use confers::migration::MigrationRegistry;
    ///
    /// let registry = MigrationRegistry::new();
    /// ```
    pub fn new() -> Self {
        Self {
            migrations: BTreeMap::new(),
            path_cache: HashMap::new(),
            versions: BTreeMap::new(),
        }
    }

    /// Create a builder for fluent registration.
    ///
    /// # Example
    ///
    /// ```rust
    /// use confers::migration::MigrationRegistry;
    ///
    /// let registry = MigrationRegistry::builder()
    ///     .register(1, 2, |v| Ok(v))
    ///     .register(2, 3, |v| Ok(v))
    ///     .build();
    /// ```
    pub fn builder() -> MigrationRegistryBuilder {
        MigrationRegistryBuilder::new()
    }

    /// Create a registry with pre-injected migrations.
    ///
    /// # Example
    ///
    /// ```rust
    /// use confers::migration::{MigrationRegistry, MigrationFn};
    /// use std::collections::HashMap;
    ///
    /// let mut migrations: HashMap<(u32, u32), MigrationFn> = HashMap::new();
    /// migrations.insert((1, 2), Box::new(|v| Ok(v)));
    ///
    /// let registry = MigrationRegistry::with_migrations(migrations);
    /// ```
    pub fn with_migrations(migrations: HashMap<(u32, u32), MigrationFn>) -> Self {
        let mut registry = Self::new();
        for ((from, to), f) in migrations {
            registry.register(from, to, f);
        }
        registry
    }

    /// Register a migration function from one version to another.
    ///
    /// Returns `self` for method chaining.
    ///
    /// # Example
    ///
    /// ```rust
    /// use confers::migration::MigrationRegistry;
    ///
    /// let mut registry = MigrationRegistry::new();
    /// registry
    ///     .register(1, 2, |mut v| { v.version = 2; Ok(v) })
    ///     .register(2, 3, |mut v| { v.version = 3; Ok(v) });
    /// ```
    pub fn register<F>(&mut self, from: u32, to: u32, f: F) -> &mut Self
    where
        F: FnMut(AnnotatedValue) -> ConfigResult<AnnotatedValue> + Send + Sync + 'static,
    {
        let boxed: MigrationFn = Box::new(f);
        self.migrations.insert((from, to), boxed);

        // Update adjacency list - add 'from' -> 'to'
        self.versions.entry(from).or_default().push(to);
        // Also ensure 'to' exists as a key (might be target-only)
        self.versions.entry(to).or_default();

        // Clear path cache when migrations change
        self.path_cache.clear();

        self
    }

    /// Get the migrations map (for testing/inspection).
    pub fn migrations(&self) -> &BTreeMap<(u32, u32), MigrationFn> {
        &self.migrations
    }

    /// Precompute all migration paths using BFS.
    ///
    /// This method should be called after all migrations are registered
    /// and before any migration execution. It builds an optimal path lookup table.
    ///
    /// # Example
    ///
    /// ```rust
    /// use confers::migration::MigrationRegistry;
    ///
    /// let mut registry = MigrationRegistry::new();
    /// registry.register(1, 2, |v| Ok(v));
    /// registry.register(2, 3, |v| Ok(v));
    ///
    /// registry.precompute_paths();
    ///
    /// let path = registry.get_migration_path(1, 3);
    /// assert!(path.is_some());
    /// ```
    pub fn precompute_paths(&mut self) {
        for (&from, targets) in &self.versions {
            for &to in targets {
                self.path_cache.insert((from, to), vec![(from, to)]);
            }
        }

        let all_versions: std::collections::HashSet<u32> = {
            let mut set = std::collections::HashSet::new();
            for (&from, targets) in &self.versions {
                set.insert(from);
                for &to in targets {
                    set.insert(to);
                }
            }
            set
        };
        let versions: Vec<u32> = all_versions.into_iter().collect();

        for _ in 0..versions.len() {
            let mut updated = false;
            for &mid in &versions {
                for &from in &versions {
                    for &to in &versions {
                        if from == to {
                            continue;
                        }

                        let path_from_mid = self.path_cache.get(&(from, mid)).cloned();
                        let path_mid_to = self.path_cache.get(&(mid, to)).cloned();

                        if let (Some(p1), Some(p2)) = (path_from_mid, path_mid_to) {
                            let new_path_len = p1.len() + p2.len();
                            let should_update = match self.path_cache.get(&(from, to)) {
                                Some(existing) => new_path_len < existing.len(),
                                None => true,
                            };

                            if should_update {
                                let mut new_path = p1;
                                new_path.extend(p2);
                                self.path_cache.insert((from, to), new_path);
                                updated = true;
                            }
                        }
                    }
                }
            }
            if !updated {
                break;
            }
        }
    }

    /// Get the migration path from one version to another.
    ///
    /// Returns `None` if no path exists.
    ///
    /// # Example
    ///
    /// ```rust
    /// use confers::migration::MigrationRegistry;
    ///
    /// let mut registry = MigrationRegistry::new();
    /// registry.register(1, 2, |v| Ok(v));
    /// registry.register(2, 3, |v| Ok(v));
    /// registry.precompute_paths();
    ///
    /// let path = registry.get_migration_path(1, 3);
    /// assert_eq!(path, Some(vec![1, 2, 3]));
    /// ```
    pub fn get_migration_path(&self, from: u32, to: u32) -> Option<Vec<u32>> {
        if from == to {
            return Some(vec![from]);
        }

        // Get edges from path cache
        let edges = self.path_cache.get(&(from, to))?;

        // Convert edges to version sequence
        let mut versions = vec![from];
        for &(_v_from, v_to) in edges {
            versions.push(v_to);
        }

        Some(versions)
    }

    /// Execute migration from one version to another.
    ///
    /// This method automatically finds and executes the optimal migration path.
    ///
    /// # Example
    ///
    /// ```rust
    /// use confers::migration::MigrationRegistry;
    /// use confers::value::{AnnotatedValue, ConfigValue, SourceId};
    ///
    /// let mut registry = MigrationRegistry::new();
    /// registry.register(1, 2, |mut v| { v.version = 2; Ok(v) });
    /// registry.precompute_paths();
    ///
    /// let value = AnnotatedValue::new(ConfigValue::null(), SourceId::new("test"), "test");
    /// let result = registry.migrate(value, 1, 2);
    /// assert!(result.is_ok());
    /// ```
    pub fn migrate(
        &mut self,
        mut value: AnnotatedValue,
        from: u32,
        to: u32,
    ) -> ConfigResult<AnnotatedValue> {
        if from == to {
            return Ok(value);
        }

        // Get migration path
        let path = self
            .get_migration_path(from, to)
            .ok_or_else(|| ConfigError::migration_not_found(from, to))?;

        // Execute migrations along the path
        for i in 0..path.len() - 1 {
            let current_version = path[i];
            let next_version = path[i + 1];

            // Get mutable reference to the migration function
            let migration = self
                .migrations
                .get_mut(&(current_version, next_version))
                .ok_or_else(|| ConfigError::migration_not_found(current_version, next_version))?;

            value = migration(value)?;
        }

        Ok(value)
    }
}

impl Default for MigrationRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for MigrationRegistry.
///
/// Provides a fluent API for registering migrations.
pub struct MigrationRegistryBuilder {
    migrations: HashMap<(u32, u32), MigrationFn>,
}

impl MigrationRegistryBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self {
            migrations: HashMap::new(),
        }
    }

    /// Register a migration.
    pub fn register<F>(mut self, from: u32, to: u32, f: F) -> Self
    where
        F: FnMut(AnnotatedValue) -> ConfigResult<AnnotatedValue> + Send + Sync + 'static,
    {
        let boxed: MigrationFn = Box::new(f);
        self.migrations.insert((from, to), boxed);
        self
    }

    /// Build the MigrationRegistry.
    pub fn build(self) -> MigrationRegistry {
        MigrationRegistry::with_migrations(self.migrations)
    }
}

impl Default for MigrationRegistryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Migration behavior on configuration reload.
///
/// Controls when migrations are applied during hot reload scenarios.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MigrationOnReload {
    /// Always migrate on every reload.
    ///
    /// This ensures the configuration is always at the latest version,
    /// but may be slower for frequent reloads.
    Always,

    /// Only migrate when version changes.
    ///
    /// This is the default behavior and provides a balance between
    /// correctness and performance.
    #[default]
    OnVersionChange,

    /// Disable migration on reload.
    ///
    /// Use this when you want to manually manage version migrations
    /// or when the configuration format is stable.
    Disabled,
}

// ============================================================================
// ConfigError extensions for migration
// ============================================================================

impl ConfigError {
    /// Create a migration not found error.
    pub fn migration_not_found(from: u32, to: u32) -> Self {
        ConfigError::MigrationFailed {
            from,
            to,
            reason: format!(
                "No migration path found from version {} to version {}",
                from, to
            ),
            source: None,
        }
    }

    /// Create a migration failed error.
    pub fn migration_failed(from: u32, to: u32, reason: impl Into<String>) -> Self {
        ConfigError::MigrationFailed {
            from,
            to,
            reason: reason.into(),
            source: None,
        }
    }
}
