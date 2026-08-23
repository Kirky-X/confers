//! Integration test for the Config derive macro.

use super::common;
use confers::Config;
#[cfg(feature = "cli")]
use confers::ConfigClap;
use confers::ConfigMigration;
use confers::ConfigModules;
use confers::ConfigSchema;
use serde::Deserialize;
use serial_test::serial;

#[derive(Debug, Config, Deserialize, PartialEq)]
struct SimpleConfig {
    #[config(default = "localhost".to_string())]
    host: String,

    #[config(default = 8080u16)]
    port: u16,
}

#[derive(Debug, Config, Deserialize, PartialEq)]
#[config(env_prefix = "MYAPP_")]
struct PrefixedConfig {
    #[config(default = "default-value".to_string())]
    name: String,

    #[config(default = 3000u32)]
    timeout_ms: u32,
}

#[derive(Debug, Config, Deserialize, PartialEq)]
struct OptionalConfig {
    #[config(default = None::<String>)]
    optional_field: Option<String>,

    #[config(default = Vec::<String>::new())]
    items: Vec<String>,
}

#[test]
fn test_simple_config_default() {
    let config = SimpleConfig::default();
    assert_eq!(config.host, "localhost");
    assert_eq!(config.port, 8080);
}

#[test]
#[serial]
fn test_simple_config_load() {
    // Load with defaults (no env vars set)
    let config = SimpleConfig::load_sync().unwrap();
    assert_eq!(config.host, "localhost");
    assert_eq!(config.port, 8080);
}

#[test]
fn test_simple_config_env_mapping() {
    let mapping = SimpleConfig::env_mapping();
    assert_eq!(mapping.len(), 2);

    let host_mapping = mapping.iter().find(|(f, _, _)| f == "host").unwrap();
    assert_eq!(host_mapping.1, "host");
    assert_eq!(host_mapping.2, "HOST");

    let port_mapping = mapping.iter().find(|(f, _, _)| f == "port").unwrap();
    assert_eq!(port_mapping.1, "port");
    assert_eq!(port_mapping.2, "PORT");
}

#[test]
fn test_prefixed_config_env_mapping() {
    let mapping = PrefixedConfig::env_mapping();

    let name_mapping = mapping.iter().find(|(f, _, _)| f == "name").unwrap();
    assert_eq!(name_mapping.2, "MYAPP_NAME");

    let timeout_mapping = mapping.iter().find(|(f, _, _)| f == "timeout_ms").unwrap();
    assert_eq!(timeout_mapping.2, "MYAPP_TIMEOUT_MS");
}

#[test]
fn test_optional_config_default() {
    let config = OptionalConfig::default();
    assert_eq!(config.optional_field, None);
    assert!(config.items.is_empty());
}

#[test]
#[serial]
fn test_config_with_env_var() {
    common::with_env_var("HOST", "env-host", || {
        let config = SimpleConfig::load_sync().unwrap();
        assert_eq!(config.host, "env-host");
    });
}

#[test]
#[serial]
fn test_prefixed_config_with_env_var() {
    common::with_env_var("MYAPP_NAME", "env-name", || {
        let config = PrefixedConfig::load_sync().unwrap();
        assert_eq!(config.name, "env-name");
    });
}

// ===== Regression: env override for numeric fields (Bug 3) =====

#[derive(Debug, Config, Deserialize, PartialEq)]
struct NumericEnvConfig {
    #[config(default = 0u32)]
    port: u32,

    #[config(default = 0.0f64)]
    rate: f64,

    #[config(default = false)]
    enabled: bool,

    #[config(default = "".to_string())]
    host: String,
}

#[test]
#[serial]
fn test_numeric_env_config_default() {
    let config = NumericEnvConfig::load_sync().unwrap();
    assert_eq!(config.port, 0);
    assert_eq!(config.rate, 0.0);
    assert!(!config.enabled);
    assert_eq!(config.host, "");
}

#[test]
#[serial]
fn test_numeric_env_override_u32() {
    common::with_env_var("PORT", "8080", || {
        let config = NumericEnvConfig::load_sync().unwrap();
        assert_eq!(config.port, 8080);
    });
}

#[test]
#[serial]
fn test_numeric_env_override_f64() {
    common::with_env_var("RATE", "42.5", || {
        let config = NumericEnvConfig::load_sync().unwrap();
        assert_eq!(config.rate, 42.5);
    });
}

#[test]
#[serial]
fn test_numeric_env_override_bool() {
    common::with_env_var("ENABLED", "true", || {
        let config = NumericEnvConfig::load_sync().unwrap();
        assert!(config.enabled);
    });
}

#[test]
#[serial]
fn test_numeric_env_override_all() {
    std::env::set_var("PORT", "9090");
    std::env::set_var("RATE", "99.9");
    std::env::set_var("ENABLED", "true");
    std::env::set_var("HOST", "example.com");

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let config = NumericEnvConfig::load_sync().unwrap();
        assert_eq!(config.port, 9090);
        assert_eq!(config.rate, 99.9);
        assert!(config.enabled);
        assert_eq!(config.host, "example.com");
    }));

    std::env::remove_var("PORT");
    std::env::remove_var("RATE");
    std::env::remove_var("ENABLED");
    std::env::remove_var("HOST");

    match result {
        Ok(()) => {}
        Err(panic) => std::panic::resume_unwind(panic),
    }
}

// ===== Regression: env override with negative f64 (edge case) =====

#[derive(Debug, Config, Deserialize, PartialEq)]
struct SignedNumericConfig {
    #[config(default = 0.0f64)]
    temperature: f64,
}

#[test]
#[serial]
fn test_numeric_env_override_negative_f64() {
    common::with_env_var("TEMPERATURE", "-5.5", || {
        let config = SignedNumericConfig::load_sync().unwrap();
        assert_eq!(config.temperature, -5.5);
    });
}

// ===== ConfigSchema / ConfigMigration / ConfigModules / ConfigClap derives =====

#[derive(Debug, ConfigSchema)]
#[allow(dead_code)] // Fields are exercised only through the derive-generated schema.
struct SchemaConfig {
    #[config(name = "host")]
    pub host: String,

    #[config(name = "port")]
    pub port: u16,

    #[config(name = "debug")]
    pub debug: bool,

    #[config(name = "tags")]
    pub tags: Vec<String>,
}

#[test]
fn test_config_schema_derive_generates_json_schema() {
    let schema = SchemaConfig::json_schema();
    let obj = schema.as_object().expect("schema is an object");
    assert_eq!(obj.get("type").and_then(|v| v.as_str()), Some("object"));
    assert_eq!(
        obj.get("title").and_then(|v| v.as_str()),
        Some("SchemaConfig")
    );

    let properties = obj
        .get("properties")
        .and_then(|v| v.as_object())
        .expect("properties object");
    assert_eq!(properties.len(), 4, "all non-skipped fields in schema");

    assert_eq!(
        properties
            .get("host")
            .and_then(|v| v.get("type"))
            .and_then(|v| v.as_str()),
        Some("string")
    );
    assert_eq!(
        properties
            .get("port")
            .and_then(|v| v.get("type"))
            .and_then(|v| v.as_str()),
        Some("integer")
    );
    assert_eq!(
        properties
            .get("debug")
            .and_then(|v| v.get("type"))
            .and_then(|v| v.as_str()),
        Some("boolean")
    );
    assert_eq!(
        properties
            .get("tags")
            .and_then(|v| v.get("type"))
            .and_then(|v| v.as_str()),
        Some("array")
    );
}

#[test]
fn test_config_schema_derive_generates_typescript_type() {
    let ts = SchemaConfig::typescript_type();
    assert!(
        ts.contains("export interface SchemaConfig"),
        "TypeScript interface name, got: {ts}"
    );
}

#[derive(Debug, ConfigMigration)]
#[config(version = 3)]
#[allow(dead_code)] // Fields are exercised only through the derive-generated versioning.
struct MigrationConfig {
    pub name: String,
    pub port: u16,
}

#[test]
fn test_config_migration_derive_generates_versioned() {
    use confers::migration::Versioned;
    assert_eq!(MigrationConfig::VERSION, 3);

    let registry = migration_registry();
    assert_eq!(registry.migrations().len(), 0, "empty registry by default");
}

#[derive(Debug, ConfigModules)]
#[allow(dead_code)] // Fields are exercised only through the derive-generated registry.
struct ModularConfig {
    #[config(module_group = "core")]
    pub name: String,

    #[config(module_group = "core")]
    pub port: u16,

    #[config(module_group = "telemetry")]
    pub metrics_enabled: bool,
}

#[test]
fn test_config_modules_derive_generates_registry() {
    let registry = ModularConfig::module_registry();
    let groups: Vec<String> = registry
        .list_groups()
        .into_iter()
        .map(|g| g.to_string())
        .collect();
    assert!(
        groups.contains(&"core".to_string()),
        "core group registered, got {groups:?}"
    );
    assert!(
        groups.contains(&"telemetry".to_string()),
        "telemetry group registered, got {groups:?}"
    );

    let module_groups = ModularConfig::module_groups();
    assert_eq!(module_groups.len(), 2, "two unique module groups");
    assert!(module_groups.contains(&"core"));
    assert!(module_groups.contains(&"telemetry"));
}

#[cfg(feature = "cli")]
#[derive(Debug, ConfigClap)]
#[allow(dead_code)] // Fields are exercised only through the derive-generated clap parser.
struct CliConfig {
    #[config(name = "host", default = "localhost".to_string(), name_clap_long = "host", name_clap_short = 'o')]
    pub host: String,

    #[config(name = "port", default = 8080u16)]
    pub port: u16,
}

#[cfg(feature = "cli")]
#[test]
fn test_config_clap_derive_parses_args() {
    use std::ffi::OsString;

    let args = CliConfig::clap_args_from(
        vec![
            OsString::from("app"),
            OsString::from("--host"),
            OsString::from("example.com"),
            OsString::from("--port"),
            OsString::from("9090"),
        ]
        .into_iter(),
    );

    let map = args.to_config_map();
    assert_eq!(
        map.get("host").and_then(|v| v.as_str()),
        Some("example.com")
    );
    assert_eq!(
        map.get("port").and_then(|v| v.as_u64()),
        Some(9090),
        "port parsed as u64 from CLI arg"
    );
}

#[cfg(feature = "cli")]
#[test]
fn test_config_clap_derive_clap_app() {
    let app = CliConfig::clap_app();
    let name = app.get_name();
    assert_eq!(name, "app", "app name from clap command");
}
