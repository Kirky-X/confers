//! Integration tests for validation support.

#![cfg(feature = "validation")]

use confers::Config;
use garde::Validate;
use serde::Deserialize;

#[derive(Debug, Config, Deserialize, Validate)]
#[config(validate)]
struct ValidatedConfig {
    #[config(default = "localhost".to_string())]
    #[garde(length(min = 1, max = 253))]
    host: String,

    #[config(default = 8080u16)]
    #[garde(range(min = 1, max = 65535))]
    port: u16,
}

#[test]
fn test_validated_config_defaults_pass() {
    // Default values should pass validation
    let config = ValidatedConfig::default();
    assert!(config.validate().is_ok());
}

#[test]
fn test_validated_config_host_env_override() {
    std::env::set_var("HOST", "example.com");

    let config = ValidatedConfig::load_sync().unwrap();
    assert_eq!(config.host, "example.com");
    assert_eq!(config.port, 8080);  // Default port

    // Validate the overridden host value
    assert!(config.validate().is_ok());

    std::env::remove_var("HOST");
}

#[test]
fn test_host_validation_fail_empty() {
    // Empty host should fail validation
    std::env::set_var("HOST", "");

    let config = ValidatedConfig::load_sync().unwrap();
    assert!(config.validate().is_err());

    std::env::remove_var("HOST");
}

#[derive(Debug, Config, Deserialize, Validate)]
#[config(validate)]
struct EmailConfig {
    #[config(default = "user@example.com".to_string())]
    #[garde(email)]
    email: String,
}

#[test]
fn test_email_validation_pass() {
    let config = EmailConfig::default();
    assert!(config.validate().is_ok());
}

#[test]
fn test_email_validation_fail() {
    // Set an invalid email
    std::env::set_var("EMAIL", "not-an-email");

    let config = EmailConfig::load_sync().unwrap();
    assert!(config.validate().is_err());

    std::env::remove_var("EMAIL");
}

#[test]
fn test_email_validation_valid_override() {
    std::env::set_var("EMAIL", "valid@example.org");

    let config = EmailConfig::load_sync().unwrap();
    assert_eq!(config.email, "valid@example.org");
    assert!(config.validate().is_ok());

    std::env::remove_var("EMAIL");
}

#[test]
fn test_validated_config_defaults() {
    // Test that defaults work correctly
    let config = ValidatedConfig::default();
    assert_eq!(config.host, "localhost");
    assert_eq!(config.port, 8080);
    assert!(config.validate().is_ok());
}

// Test nested struct with validation
#[derive(Debug, Clone, Deserialize, Validate)]
struct ServerConfig {
    #[garde(length(min = 1))]
    name: String,
    
    #[garde(range(min = 1, max = 65535))]
    port: u16,
}

#[derive(Debug, Config, Deserialize, Validate)]
#[config(validate)]
struct AppConfig {
    #[config(default = "localhost".to_string())]
    #[garde(length(min = 1))]
    server_host: String,
}

#[test]
fn test_nested_validation() {
    let config = AppConfig::default();
    assert!(config.validate().is_ok());
    
    // Test with valid override
    std::env::set_var("SERVER_HOST", "prod-server");
    let config = AppConfig::load_sync().unwrap();
    assert_eq!(config.server_host, "prod-server");
    assert!(config.validate().is_ok());
    
    std::env::remove_var("SERVER_HOST");
}

