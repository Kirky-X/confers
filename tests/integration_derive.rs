//! Integration test for the Config derive macro.

use confers::Config;
use serde::Deserialize;

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
fn test_config_with_env_var() {
    std::env::set_var("HOST", "env-host");
    let config = SimpleConfig::load_sync().unwrap();
    assert_eq!(config.host, "env-host");
    std::env::remove_var("HOST");
}

#[test]
fn test_prefixed_config_with_env_var() {
    std::env::set_var("MYAPP_NAME", "env-name");
    let config = PrefixedConfig::load_sync().unwrap();
    assert_eq!(config.name, "env-name");
    std::env::remove_var("MYAPP_NAME");
}
