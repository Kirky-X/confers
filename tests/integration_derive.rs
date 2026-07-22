//! Integration test for the Config derive macro.

mod common;

use confers::Config;
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
    assert_eq!(config.enabled, false);
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
    common::with_env_var("RATE", "3.14", || {
        let config = NumericEnvConfig::load_sync().unwrap();
        assert_eq!(config.rate, 3.14);
    });
}

#[test]
#[serial]
fn test_numeric_env_override_bool() {
    common::with_env_var("ENABLED", "true", || {
        let config = NumericEnvConfig::load_sync().unwrap();
        assert_eq!(config.enabled, true);
    });
}

#[test]
#[serial]
fn test_numeric_env_override_all() {
    std::env::set_var("PORT", "9090");
    std::env::set_var("RATE", "2.718");
    std::env::set_var("ENABLED", "true");
    std::env::set_var("HOST", "example.com");

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let config = NumericEnvConfig::load_sync().unwrap();
        assert_eq!(config.port, 9090);
        assert_eq!(config.rate, 2.718);
        assert_eq!(config.enabled, true);
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
