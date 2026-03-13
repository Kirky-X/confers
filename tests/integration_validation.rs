//! Integration tests for validation support.

#![cfg(feature = "validation")]

mod common;

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
    common::with_env_var("HOST", "example.com", || {
        let config = ValidatedConfig::load_sync().unwrap();
        assert_eq!(config.host, "example.com");
        assert_eq!(config.port, 8080); // Default port

        // Validate the overridden host value
        assert!(config.validate().is_ok());
    });
}

#[test]
fn test_host_validation_fail_empty() {
    // Empty host should fail validation
    common::with_env_var("HOST", "", || {
        let config = ValidatedConfig::load_sync().unwrap();
        assert!(config.validate().is_err());
    });
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
    common::with_env_var("EMAIL", "not-an-email", || {
        let config = EmailConfig::load_sync().unwrap();
        assert!(config.validate().is_err());
    });
}

#[test]
fn test_email_validation_valid_override() {
    common::with_env_var("EMAIL", "valid@example.org", || {
        let config = EmailConfig::load_sync().unwrap();
        assert_eq!(
            config.email, "valid@example.org",
            "Email should be overridden by env var"
        );
        assert!(
            config.validate().is_ok(),
            "Valid email should pass validation"
        );
    });
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
    common::with_env_var("SERVER_HOST", "prod-server", || {
        let config = AppConfig::load_sync().unwrap();
        assert_eq!(config.server_host, "prod-server");
        assert!(config.validate().is_ok());
    });
}

// ========================================
// Boundary Condition Tests
// ========================================

#[test]
fn test_empty_string_validation() {
    #[derive(Debug, Clone, Config, Deserialize, Validate)]
    #[config(validate)]
    struct EmptyStringConfig {
        #[config(default = String::new())]
        #[garde(length(min = 1))]
        name: String,
    }

    let config = EmptyStringConfig::default();
    // Should fail validation with empty string
    assert!(
        config.validate().is_err(),
        "Empty string should fail validation"
    );
}

#[test]
fn test_long_string_validation() {
    #[derive(Debug, Clone, Config, Deserialize, Validate)]
    #[config(validate)]
    struct LongStringConfig {
        #[config(default = String::new())]
        #[garde(length(max = 100))]
        description: String,
    }

    // Create a string longer than 100 characters
    let long_string = "x".repeat(1000);

    common::with_env_var("DESCRIPTION", &long_string, || {
        let config = LongStringConfig::load_sync().unwrap();
        // Should fail validation with string > 100 chars
        assert!(
            config.validate().is_err(),
            "String longer than max_length should fail validation"
        );
    });
}

#[test]
fn test_special_characters_validation() {
    #[derive(Debug, Clone, Config, Deserialize, Validate)]
    #[config(validate)]
    struct SpecialCharConfig {
        #[config(default = "valid_user".to_string())]
        #[garde(pattern(r"^[a-zA-Z0-9_]+$"))]
        username: String,
    }

    // Test with special characters - should fail
    common::with_env_var("USERNAME", "user@name!", || {
        let config = SpecialCharConfig::load_sync().unwrap();
        assert!(
            config.validate().is_err(),
            "Username with special characters should fail validation"
        );
    });

    // Test with valid characters - should pass
    common::with_env_var("USERNAME", "valid_user_123", || {
        let config = SpecialCharConfig::load_sync().unwrap();
        assert!(
            config.validate().is_ok(),
            "Username with only alphanumeric and underscore should pass validation"
        );
    });
}

#[test]
fn test_unicode_validation() {
    #[derive(Debug, Clone, Config, Deserialize, Validate)]
    #[config(validate)]
    struct UnicodeConfig {
        #[config(default = String::new())]
        #[garde(custom(validate_unicode_letters))]
        name: String,
    }

    fn validate_unicode_letters(s: &str, _context: &()) -> garde::Result {
        // Check that all characters are Unicode letters
        if s.chars().all(|c| c.is_alphabetic()) {
            Ok(())
        } else {
            Err(garde::Error::new("name must contain only Unicode letters"))
        }
    }

    // Test with Unicode letters - should pass
    common::with_env_var("NAME", "张伟", || {
        let config = UnicodeConfig::load_sync().unwrap();
        assert!(
            config.validate().is_ok(),
            "Unicode letters should pass validation"
        );
    });

    // Test with numbers - should fail
    common::with_env_var("NAME", "张伟123", || {
        let config = UnicodeConfig::load_sync().unwrap();
        assert!(
            config.validate().is_err(),
            "Name with numbers should fail validation"
        );
    });

    // Test with symbols - should fail
    common::with_env_var("NAME", "John-Doe", || {
        let config = UnicodeConfig::load_sync().unwrap();
        assert!(
            config.validate().is_err(),
            "Name with symbols should fail validation"
        );
    });
}
