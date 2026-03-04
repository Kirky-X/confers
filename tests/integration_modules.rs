//! Integration tests for Modules (Config Groups).

#![cfg(feature = "modules")]

use confers::loader::LoaderConfig;
use confers::modules::{ModuleConfig, ModuleRegistry};
use std::path::PathBuf;

// ============================================================================
// ModuleConfig Tests
// ============================================================================

#[test]
fn test_module_config_new() {
    let config = ModuleConfig::new(
        "database",
        vec![
            ("mysql", PathBuf::from("conf/db/mysql.toml")),
            ("postgresql", PathBuf::from("conf/db/postgresql.toml")),
        ],
        Some("mysql"),
    );

    assert_eq!(config.name.as_ref(), "database");
    assert_eq!(config.active_profile.as_ref(), "mysql");
    assert_eq!(config.paths.len(), 2);
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
fn test_module_config_default_profile_with_default() {
    let config = ModuleConfig::new(
        "database",
        vec![
            ("mysql", PathBuf::from("conf/db/mysql.toml")),
            ("postgresql", PathBuf::from("conf/db/postgresql.toml")),
        ],
        Some("postgresql"),
    );
    assert_eq!(config.active_profile.as_ref(), "postgresql");
}

#[test]
fn test_module_config_default_profile_without_default() {
    let config = ModuleConfig::new(
        "database",
        vec![
            ("mysql", PathBuf::from("conf/db/mysql.toml")),
            ("postgresql", PathBuf::from("conf/db/postgresql.toml")),
        ],
        None,
    );
    // Should use first profile as default
    assert_eq!(config.active_profile.as_ref(), "mysql");
}

// ============================================================================
// ModuleRegistry Tests
// ============================================================================

#[test]
fn test_registry_new() {
    let registry = ModuleRegistry::new();
    assert!(registry.is_empty());
    assert_eq!(registry.len(), 0);
}

#[test]
fn test_registry_with_capacity() {
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

    if let Err(confers::ConfigError::ModuleNotFound { group, module }) = result {
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

// ============================================================================
// Load Module Tests
// ============================================================================

#[test]
fn test_load_module_not_found_group() {
    let registry = ModuleRegistry::new();

    let result = registry.load_module("nonexistent", "profile", &LoaderConfig::default());

    assert!(result.is_err());
    if let Err(confers::ConfigError::ModuleNotFound { group, module }) = result {
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
    let temp_dir = tempfile::TempDir::new().unwrap();

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
fn test_load_module_success_toml() {
    let temp_dir = tempfile::TempDir::new().unwrap();

    let mysql_path = temp_dir.path().join("mysql.toml");
    std::fs::write(&mysql_path, "host = \"localhost\"\nport = 3306\n").unwrap();

    let mut registry = ModuleRegistry::new();

    registry.register_group("database", vec![("mysql", mysql_path)], Some("mysql"));

    let result = registry.load_module("database", "mysql", &LoaderConfig::default());

    assert!(result.is_ok());
    let value = result.unwrap();
    assert!(value
        .inner
        .as_map()
        .map(|m| m.contains_key("host"))
        .unwrap_or(false));
}

#[test]
fn test_load_module_success_json() {
    let temp_dir = tempfile::TempDir::new().unwrap();

    let config_path = temp_dir.path().join("config.json");
    std::fs::write(&config_path, "{\"host\": \"localhost\", \"port\": 8080}").unwrap();

    let mut registry = ModuleRegistry::new();

    registry.register_group("app", vec![("dev", config_path)], Some("dev"));

    let result = registry.load_module("app", "dev", &LoaderConfig::default());

    assert!(result.is_ok());
    let value = result.unwrap();
    assert!(value
        .inner
        .as_map()
        .map(|m| m.contains_key("host"))
        .unwrap_or(false));
    assert!(value
        .inner
        .as_map()
        .map(|m| m.contains_key("port"))
        .unwrap_or(false));
}

#[test]
fn test_load_active_not_found() {
    let registry = ModuleRegistry::new();

    let result = registry.load_active("nonexistent", &LoaderConfig::default());

    assert!(result.is_err());
}

#[test]
fn test_load_active_success() {
    let temp_dir = tempfile::TempDir::new().unwrap();

    let mysql_path = temp_dir.path().join("mysql.toml");
    std::fs::write(&mysql_path, "host = \"localhost\"\nport = 3306\n").unwrap();

    let postgresql_path = temp_dir.path().join("postgresql.toml");
    std::fs::write(&postgresql_path, "host = \"pg.example.com\"\nport = 5432\n").unwrap();

    let mut registry = ModuleRegistry::new();

    registry.register_group(
        "database",
        vec![("mysql", mysql_path), ("postgresql", postgresql_path)],
        Some("mysql"),
    );

    // Load active (should be mysql)
    let result = registry.load_active("database", &LoaderConfig::default());
    assert!(result.is_ok());
    let value = result.unwrap();
    assert!(value
        .inner
        .as_map()
        .map(|m| m.contains_key("host"))
        .unwrap_or(false));

    // Change active profile and load again
    registry
        .set_active_profile("database", "postgresql")
        .unwrap();

    let result = registry.load_active("database", &LoaderConfig::default());
    assert!(result.is_ok());
    let value = result.unwrap();
    assert!(value
        .inner
        .as_map()
        .map(|m| m.contains_key("host"))
        .unwrap_or(false));
}

// ============================================================================
// Edge Cases Tests
// ============================================================================

#[test]
fn test_registry_chaining() {
    let mut registry = ModuleRegistry::new();

    // Test method chaining
    registry
        .register_group("db", vec![], None)
        .register_group("cache", vec![], None);

    assert_eq!(registry.len(), 2);
}

#[test]
fn test_multiple_groups_load() {
    let temp_dir = tempfile::TempDir::new().unwrap();

    let mysql_path = temp_dir.path().join("mysql.toml");
    std::fs::write(&mysql_path, "host = \"localhost\"").unwrap();

    let redis_path = temp_dir.path().join("redis.toml");
    std::fs::write(&redis_path, "host = \"redis.local\"").unwrap();

    let mut registry = ModuleRegistry::new();

    registry.register_group("database", vec![("mysql", mysql_path)], Some("mysql"));

    registry.register_group("cache", vec![("redis", redis_path)], Some("redis"));

    // Load database
    let db_result = registry.load_module("database", "mysql", &LoaderConfig::default());
    assert!(db_result.is_ok());

    // Load cache
    let cache_result = registry.load_module("cache", "redis", &LoaderConfig::default());
    assert!(cache_result.is_ok());
}
