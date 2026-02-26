//! Integration tests for the Migration module.

use confers::value::{AnnotatedValue, ConfigValue, SourceId};

#[cfg(feature = "migration")]
use confers::migration::{MigrationOnReload, MigrationRegistry, Versioned};

fn create_test_value() -> AnnotatedValue {
    AnnotatedValue::new(ConfigValue::string("test"), SourceId::new("memory"), "test")
}

#[cfg(feature = "migration")]
mod versioned_tests {
    use super::*;

    /// Test struct implementing Versioned trait
    struct TestConfigV1;

    impl Versioned for TestConfigV1 {
        const VERSION: u32 = 1;
    }

    #[test]
    fn test_versioned_trait_implementation() {
        assert_eq!(TestConfigV1::VERSION, 1);
    }

    #[test]
    fn test_versioned_different_versions() {
        struct ConfigV1;
        struct ConfigV2;
        struct ConfigV3;

        impl Versioned for ConfigV1 {
            const VERSION: u32 = 1;
        }
        impl Versioned for ConfigV2 {
            const VERSION: u32 = 2;
        }
        impl Versioned for ConfigV3 {
            const VERSION: u32 = 3;
        }

        assert_eq!(ConfigV1::VERSION, 1);
        assert_eq!(ConfigV2::VERSION, 2);
        assert_eq!(ConfigV3::VERSION, 3);
    }
}

// ============================================================================
// MigrationRegistry Tests
// ============================================================================

#[cfg(feature = "migration")]
mod registry_tests {
    use super::*;

    #[test]
    fn test_registry_new() {
        let registry = MigrationRegistry::new();
        assert!(registry.migrations().is_empty());
    }

    #[test]
    fn test_registry_register() {
        let mut registry = MigrationRegistry::new();
        registry.register(1, 2, |v| Ok(v));

        let migrations = registry.migrations();
        assert!(migrations.contains_key(&(1, 2)));
    }

    #[test]
    fn test_registry_register_multiple() {
        let mut registry = MigrationRegistry::new();

        registry.register(1, 2, |v| Ok(v));
        registry.register(2, 3, |v| Ok(v));
        registry.register(1, 3, |v| Ok(v));

        let migrations = registry.migrations();
        assert_eq!(migrations.len(), 3);
        assert!(migrations.contains_key(&(1, 2)));
        assert!(migrations.contains_key(&(2, 3)));
        assert!(migrations.contains_key(&(1, 3)));
    }

    #[test]
    fn test_registry_register_returns_self() {
        let mut registry = MigrationRegistry::new();

        let result = registry.register(1, 2, |v| Ok(v));

        // Should return mutable self for chaining
        assert!(result.migrations().contains_key(&(1, 2)));
    }

    #[test]
    fn test_registry_builder_pattern() {
        let registry = MigrationRegistry::builder()
            .register(1, 2, |v| Ok(v))
            .register(2, 3, |v| Ok(v))
            .build();

        let migrations = registry.migrations();
        assert_eq!(migrations.len(), 2);
    }

    #[test]
    fn test_registry_with_migrations() {
        let migrations = std::collections::HashMap::new();
        let registry = MigrationRegistry::with_migrations(migrations);
        assert!(registry.migrations().is_empty());
    }
}

// ============================================================================
// Path Precomputation Tests
// ============================================================================

#[cfg(feature = "migration")]
mod path_tests {
    use super::*;

    #[test]
    fn test_precompute_paths_direct() {
        let mut registry = MigrationRegistry::new();
        registry.register(1, 2, |v| Ok(v));
        registry.register(2, 3, |v| Ok(v));

        registry.precompute_paths();

        let path = registry.get_migration_path(1, 2);
        assert!(path.is_some());
        assert_eq!(path.unwrap(), vec![1, 2]);
    }

    #[test]
    fn test_precompute_paths_chain() {
        let mut registry = MigrationRegistry::new();
        registry.register(1, 2, |v| Ok(v));
        registry.register(2, 3, |v| Ok(v));

        registry.precompute_paths();

        let path = registry.get_migration_path(1, 3);
        assert!(path.is_some());
        assert_eq!(path.unwrap(), vec![1, 2, 3]);
    }

    #[test]
    fn test_precompute_paths_direct_preferred_over_chain() {
        let mut registry = MigrationRegistry::new();
        // Both direct and chain paths available
        registry.register(1, 2, |v| Ok(v));
        registry.register(2, 3, |v| Ok(v));
        registry.register(1, 3, |v| Ok(v));

        registry.precompute_paths();

        let path = registry.get_migration_path(1, 3);
        assert!(path.is_some());
        // Direct path should be preferred (shorter)
        assert_eq!(path.unwrap(), vec![1, 3]);
    }

    #[test]
    fn test_precompute_paths_no_path() {
        let mut registry = MigrationRegistry::new();
        registry.register(1, 2, |v| Ok(v));
        // No path from 1 to 3

        registry.precompute_paths();

        let path = registry.get_migration_path(1, 3);
        assert!(path.is_none());
    }

    #[test]
    fn test_precompute_paths_complex_graph() {
        let mut registry = MigrationRegistry::new();
        // Graph: 1->2->3->4 and 1->3, 2->4
        registry.register(1, 2, |v| Ok(v));
        registry.register(2, 3, |v| Ok(v));
        registry.register(3, 4, |v| Ok(v));
        registry.register(1, 3, |v| Ok(v));
        registry.register(2, 4, |v| Ok(v));

        registry.precompute_paths();

        // 1->4 could be 1->2->3->4 or 1->3->4 or 1->2->4
        // All have different lengths, shortest is 1->3->4 (2 edges, 3 versions)
        let path = registry.get_migration_path(1, 4);
        assert!(path.is_some());
        let path = path.unwrap();
        // Both 1->2->4 and 1->3->4 are valid shortest paths (2 edges, 3 versions)
        assert_eq!(path.len(), 3, "Path: {:?}", path);
        assert!(path == vec![1, 2, 4] || path == vec![1, 3, 4]);
    }
}

// ============================================================================
// Migration Execution Tests
// ============================================================================

#[cfg(feature = "migration")]
mod migration_tests {
    use super::*;

    #[test]
    fn test_migrate_direct() {
        let mut registry = MigrationRegistry::new();
        let called = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));

        let called_clone = called.clone();
        registry.register(1, 2, move |mut v| {
            called_clone.store(true, std::sync::atomic::Ordering::SeqCst);
            v.version = 2;
            Ok(v)
        });

        registry.precompute_paths();

        let result = registry.migrate(create_test_value(), 1, 2);
        assert!(result.is_ok());
        assert!(called.load(std::sync::atomic::Ordering::SeqCst));
        assert_eq!(result.unwrap().version, 2);
    }

    #[test]
    fn test_migrate_chain() {
        let mut registry = MigrationRegistry::new();
        let call_order = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));

        let order = call_order.clone();
        registry.register(1, 2, move |mut v| {
            order.lock().unwrap().push(1);
            v.version = 2;
            Ok(v)
        });

        let order = call_order.clone();
        registry.register(2, 3, move |mut v| {
            order.lock().unwrap().push(2);
            v.version = 3;
            Ok(v)
        });

        registry.precompute_paths();

        let result = registry.migrate(create_test_value(), 1, 3);
        assert!(result.is_ok());

        let calls = call_order.lock().unwrap();
        assert_eq!(*calls, vec![1, 2]); // Should execute in order
        assert_eq!(result.unwrap().version, 3);
    }

    #[test]
    fn test_migrate_same_version() {
        let mut registry = MigrationRegistry::new();
        registry.register(1, 2, |v| Ok(v));
        registry.precompute_paths();

        let value = create_test_value();
        let result = registry.migrate(value, 1, 1);
        assert!(result.is_ok());
        // No migration needed, should return original
    }

    #[test]
    fn test_migrate_no_path() {
        let mut registry = MigrationRegistry::new();
        registry.register(1, 2, |v| Ok(v));
        // No path to version 3
        registry.precompute_paths();

        let result = registry.migrate(create_test_value(), 1, 3);
        assert!(result.is_err());
    }

    #[test]
    fn test_migrate_failure() {
        let mut registry = MigrationRegistry::new();

        registry.register(1, 2, |_v| {
            Err(confers::error::ConfigError::migration_failed(
                1,
                2,
                "Migration failed",
            ))
        });

        registry.precompute_paths();

        let result = registry.migrate(create_test_value(), 1, 2);
        assert!(result.is_err());
    }
}

// ============================================================================
// MigrationOnReload Tests
// ============================================================================

#[cfg(feature = "migration")]
mod migration_on_reload_tests {
    use super::*;

    #[test]
    fn test_migration_on_reload_variants() {
        // Test all variants exist
        let _ = MigrationOnReload::Always;
        let _ = MigrationOnReload::OnVersionChange;
        let _ = MigrationOnReload::Disabled;
    }

    #[test]
    fn test_migration_on_reload_debug() {
        let debug_always = format!("{:?}", MigrationOnReload::Always);
        let debug_version_change = format!("{:?}", MigrationOnReload::OnVersionChange);
        let debug_disabled = format!("{:?}", MigrationOnReload::Disabled);

        assert!(debug_always.contains("Always"));
        assert!(debug_version_change.contains("OnVersionChange"));
        assert!(debug_disabled.contains("Disabled"));
    }

    #[test]
    fn test_migration_on_reload_default() {
        let default = MigrationOnReload::default();
        assert!(matches!(default, MigrationOnReload::OnVersionChange));
    }

    #[test]
    fn test_migration_on_reload_clone() {
        let original = MigrationOnReload::Always;
        let cloned = original.clone();
        assert!(matches!(cloned, MigrationOnReload::Always));
    }
}
