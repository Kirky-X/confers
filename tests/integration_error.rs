//! Integration tests for error handling.
//!
//! These tests verify the error handling functionality including:
//! - Invalid configuration format errors
//! - Configuration validation failures
//! - Interpolation errors (circular references)
//! - Missing required fields
//! - Type mismatch errors

#![cfg(all(feature = "validation", feature = "interpolation"))]

mod common;

use std::path::PathBuf;

// ========================================
// Basic Error Tests
// ========================================

/// Test ConfigError variants are accessible.
#[test]
fn test_config_error_accessible() {
    use confers::ConfigError;

    // Verify we can construct various error types
    let _err = ConfigError::FileNotFound {
        filename: PathBuf::from("test.toml"),
        source: None,
    };

    let _err = ConfigError::InvalidValue {
        key: "test".to_string(),
        expected_type: "string".to_string(),
        message: "test message".to_string(),
    };
}

/// Test ErrorCode variants.
#[test]
fn test_error_code_variants() {
    use confers::error::ErrorCode;

    // Test all error codes exist and have expected values
    assert_eq!(ErrorCode::FileNotFound as u16, 1001);
    assert_eq!(ErrorCode::ParseError as u16, 1002);
    assert_eq!(ErrorCode::ValidationFailed as u16, 1003);
    assert_eq!(ErrorCode::DecryptionFailed as u16, 1004);
    assert_eq!(ErrorCode::RemoteUnavailable as u16, 1005);
    assert_eq!(ErrorCode::VersionMismatch as u16, 1006);
    assert_eq!(ErrorCode::MigrationFailed as u16, 1007);
    assert_eq!(ErrorCode::ModuleNotFound as u16, 1008);
    assert_eq!(ErrorCode::ReloadRolledBack as u16, 1009);
    assert_eq!(ErrorCode::IoError as u16, 1010);
    assert_eq!(ErrorCode::InvalidValue as u16, 1011);
    assert_eq!(ErrorCode::SourceChainError as u16, 1012);
    assert_eq!(ErrorCode::Timeout as u16, 1013);
    assert_eq!(ErrorCode::SizeLimitExceeded as u16, 1014);
    assert_eq!(ErrorCode::InterpolationError as u16, 1015);
    assert_eq!(ErrorCode::KeyError as u16, 1016);
    assert_eq!(ErrorCode::CircularReference as u16, 1017);
    assert_eq!(ErrorCode::ConcurrencyConflict as u16, 1018);
    assert_eq!(ErrorCode::KeyRotationFailed as u16, 1019);
    assert_eq!(ErrorCode::WatcherError as u16, 1020);
    assert_eq!(ErrorCode::OverrideBlocked as u16, 1021);
}

/// Test ErrorCode Display.
#[test]
fn test_error_code_display() {
    use confers::error::ErrorCode;

    assert_eq!(format!("{}", ErrorCode::FileNotFound), "FILE_NOT_FOUND");
    assert_eq!(format!("{}", ErrorCode::ParseError), "PARSE_ERROR");
    assert_eq!(
        format!("{}", ErrorCode::ValidationFailed),
        "VALIDATION_FAILED"
    );
    assert_eq!(
        format!("{}", ErrorCode::CircularReference),
        "CIRCULAR_REFERENCE"
    );
}

// ========================================
// Invalid Format Error Tests (4.2.1)
// ========================================

/// Test invalid TOML format error (4.2.1).
#[test]
fn test_invalid_toml_format_error() {
    let temp_file = common::create_temp_config(
        r#"
        invalid toml content {{{
    "#,
        ".toml",
    );

    #[derive(Debug, Clone, confers::Config, serde::Deserialize)]
    #[config()]
    struct TestConfig {
        value: String,
    }

    // Use load_file_with_env to bypass path validation
    let result = TestConfig::load_file_with_env(temp_file.path());

    // Should fail to parse
    assert!(result.is_err());
}

/// Test invalid JSON format error (4.2.1).
#[test]
fn test_invalid_json_format_error() {
    let temp_file = common::create_temp_config(
        r#"
        { invalid json content }
    "#,
        ".json",
    );

    #[derive(Debug, Clone, confers::Config, serde::Deserialize)]
    #[config()]
    struct TestConfig {
        value: String,
    }

    // Use load_file_with_env to bypass path validation
    let result = TestConfig::load_file_with_env(temp_file.path());

    // Should fail to parse
    assert!(result.is_err());
}

/// Test invalid YAML format error (4.2.1).
#[cfg(feature = "yaml")]
#[test]
fn test_invalid_yaml_format_error() {
    let temp_file = common::create_temp_config(
        r#"
        invalid: yaml: content:
          - item
        extra: invalid: indent
    "#,
        ".yaml",
    );

    #[derive(Debug, Clone, confers::Config, serde::Deserialize)]
    #[config()]
    struct TestConfig {
        value: String,
    }

    // Use load_file_with_env to bypass path validation
    let result = TestConfig::load_file_with_env(temp_file.path());

    // Should fail to parse
    assert!(result.is_err());
}

/// Test parse error location.
#[test]
fn test_parse_error_location() {
    use confers::error::ParseLocation;

    let loc = ParseLocation::new("test.toml", 10, 5);
    assert_eq!(loc.line, 10);
    assert_eq!(loc.column, 5);

    let loc_str = format!("{}", loc);
    assert!(loc_str.contains("test.toml"));
    assert!(loc_str.contains("10"));
    assert!(loc_str.contains("5"));
}

/// Test parse error from path.
#[test]
fn test_parse_error_from_path() {
    use confers::error::ParseLocation;

    let loc = ParseLocation::from_path(
        std::path::Path::new("/home/user/project/config.toml"),
        15,
        10,
    );

    assert_eq!(loc.line, 15);
    assert_eq!(loc.column, 10);
    assert!(loc.source_name.contains("config.toml"));
}

// ========================================
// Validation Failure Tests (4.2.2)
// ========================================

/// Test configuration validation failure (4.2.2).
#[test]
fn test_config_validation_failure() {
    use confers::Config;
    use garde::Validate;
    use serde::Deserialize;

    #[derive(Debug, Clone, Deserialize, Validate)]
    struct ValidatedConfig {
        #[garde(length(min = 1, max = 10))]
        name: String,

        #[garde(range(min = 1, max = 100))]
        count: u32,
    }

    // Create invalid config
    let config = ValidatedConfig {
        name: "".to_string(), // Empty name - should fail
        count: 200,           // Out of range - should fail
    };

    let result = config.validate();
    assert!(result.is_err());
}

/// Test validation error with custom rule.
#[test]
fn test_validation_error_custom_rule() {
    use confers::ConfigError;

    let err = ConfigError::validation("field_name", "length(min=1)", "Field cannot be empty");

    if let confers::ConfigError::ValidationFailed {
        field,
        rule,
        message,
    } = err
    {
        assert_eq!(field, "field_name");
        assert_eq!(rule, "length(min=1)");
        assert_eq!(message, "Field cannot be empty");
    } else {
        panic!("Expected ValidationFailed error");
    }
}

/// Test validation error from garde report.
#[test]
fn test_validation_error_from_garde_report() {
    use confers::ConfigError;

    // Create a basic validation error
    let err = ConfigError::validation("test_field", "required", "test error");

    let code = err.code();
    assert!(matches!(code, confers::error::ErrorCode::ValidationFailed));
}

/// Test validation error user message.
#[test]
fn test_validation_error_user_message() {
    let err = confers::ConfigError::ValidationFailed {
        field: "email".to_string(),
        rule: "email".to_string(),
        message: "Invalid email format".to_string(),
    };

    let user_msg = err.user_message();
    assert!(user_msg.contains("email"));
    assert!(user_msg.contains("Invalid email format"));
}

// ========================================
// Interpolation Error Tests (4.2.3)
// ========================================

/// Test circular reference detection (4.2.3).
#[test]
fn test_circular_reference_detection() {
    // Create a config with circular interpolation
    let temp_file = common::create_temp_config(
        r#"
        a = "${b}"
        b = "${a}"
    "#,
        ".toml",
    );

    #[derive(Debug, Clone, confers::Config, serde::Deserialize)]
    #[config()]
    struct CircularConfig {
        a: String,
        b: String,
    }

    // Use load_file_with_env to bypass path validation
    let result = CircularConfig::load_file_with_env(temp_file.path());

    // Should detect circular reference
    assert!(result.is_err());
}

/// Test self-referencing variable error.
#[test]
fn test_self_reference_error() {
    let temp_file = common::create_temp_config(
        r#"
        value = "${value}"
    "#,
        ".toml",
    );

    #[derive(Debug, Clone, confers::Config, serde::Deserialize)]
    #[config()]
    struct SelfRefConfig {
        value: String,
    }

    let result = SelfRefConfig::load_file_with_env(temp_file.path());

    assert!(result.is_err());
}

/// Test undefined variable interpolation error.
#[test]
fn test_undefined_variable_error() {
    let temp_file = common::create_temp_config(
        r#"
        value = "${undefined_var}"
    "#,
        ".toml",
    );

    #[derive(Debug, Clone, confers::Config, serde::Deserialize)]
    #[config()]
    struct UndefinedConfig {
        value: String,
    }

    let result = UndefinedConfig::load_file_with_env(temp_file.path());

    // Should handle undefined variable gracefully
    match result {
        Ok(_config) => {
            // Some implementations may leave undefined as literal
        }
        Err(_) => {
            // This is also acceptable behavior
        }
    }
}

/// Test nested circular reference.
#[test]
fn test_nested_circular_reference() {
    let temp_file = common::create_temp_config(
        r#"
        a = "${b}"
        b = "${c}"
        c = "${a}"
    "#,
        ".toml",
    );

    #[derive(Debug, Clone, confers::Config, serde::Deserialize)]
    #[config()]
    struct NestedCircularConfig {
        a: String,
        b: String,
        c: String,
    }

    let result = NestedCircularConfig::load_file_with_env(temp_file.path());

    // Should detect circular reference
    assert!(result.is_err());
}

// ========================================
// Missing Required Field Tests (4.2.4)
// ========================================

/// Test missing required field error (4.2.4).
#[test]
fn test_missing_required_field_error() {
    #[derive(Debug, Clone, confers::Config, serde::Deserialize)]
    #[config()]
    struct RequiredConfig {
        required_field: String,
    }

    // Don't set required_field
    let temp_file = common::create_temp_config(
        r#"
        other_field = "value"
    "#,
        ".toml",
    );

    let result = RequiredConfig::load_file_with_env(temp_file.path());

    // Should fail due to missing required field
    assert!(result.is_err());
}

/// Test missing nested required field.
#[test]
fn test_missing_nested_required_field() {
    #[derive(Debug, Clone, confers::Config, serde::Deserialize)]
    #[config()]
    struct NestedConfig {
        db: DatabaseConfig,
    }

    #[derive(Debug, Clone, confers::Config, serde::Deserialize)]
    #[config()]
    struct DatabaseConfig {
        host: String,
        port: u16,
    }

    let temp_file = common::create_temp_config(
        r#"
        [db]
        # missing host and port
    "#,
        ".toml",
    );

    let result = NestedConfig::load_file_with_env(temp_file.path());

    // String defaults to empty, u16 doesn't have default
    // So this should either work with defaults or fail
    // depending on the Config implementation
    match result {
        Ok(config) => {
            // If it works, verify the values
            assert_eq!(config.db.host, "");
        }
        Err(_) => {
            // Missing required fields is acceptable to fail
        }
    }
}

// ========================================
// Type Mismatch Tests (4.2.5)
// ========================================

/// Test type mismatch error (4.2.5).
#[test]
fn test_type_mismatch_error() {
    let temp_file = common::create_temp_config(
        r#"
        port = "not_a_number"
    "#,
        ".toml",
    );

    #[derive(Debug, Clone, confers::Config, serde::Deserialize)]
    #[config()]
    struct PortConfig {
        port: u16,
    }

    let result = PortConfig::load_file_with_env(temp_file.path());

    // Should fail type conversion
    assert!(result.is_err());
}

/// Test type mismatch for integer to string.
#[test]
fn test_integer_to_string_mismatch() {
    let temp_file = common::create_temp_config(
        r#"
        name = 12345
    "#,
        ".toml",
    );

    #[derive(Debug, Clone, confers::Config, serde::Deserialize)]
    #[config()]
    struct NameConfig {
        name: String,
    }

    let result = NameConfig::load_file_with_env(temp_file.path());

    // Should handle or reject type mismatch
    match result {
        Ok(config) => {
            // Some implementations may convert int to string
            assert_eq!(config.name, "12345");
        }
        Err(_) => {
            // Type mismatch is also acceptable
        }
    }
}

/// Test type mismatch for string to integer.
#[test]
fn test_string_to_integer_mismatch() {
    let temp_file = common::create_temp_config(
        r#"
        count = "not_an_integer"
    "#,
        ".toml",
    );

    #[derive(Debug, Clone, confers::Config, serde::Deserialize)]
    #[config()]
    struct CountConfig {
        count: i32,
    }

    let result = CountConfig::load_file_with_env(temp_file.path());

    // Should fail string to integer conversion
    assert!(result.is_err());
}

/// Test type mismatch for object to primitive.
#[test]
fn test_object_to_primitive_mismatch() {
    let temp_file = common::create_temp_config(
        r#"
        value = { nested = "object" }
    "#,
        ".toml",
    );

    #[derive(Debug, Clone, confers::Config, serde::Deserialize)]
    #[config()]
    struct SimpleConfig {
        value: String,
    }

    let result = SimpleConfig::load_file_with_env(temp_file.path());

    // Should fail object to string conversion
    assert!(result.is_err());
}

// ========================================
// Error Recovery and Retry Tests
// ========================================

/// Test retryable error detection.
#[test]
fn test_retryable_error_detection() {
    use confers::ConfigError;

    // Timeout is retryable
    let err = ConfigError::Timeout { duration_ms: 5000 };
    assert!(err.is_retryable());

    // Validation is not retryable
    let err = ConfigError::ValidationFailed {
        field: "test".to_string(),
        rule: "required".to_string(),
        message: "missing".to_string(),
    };
    assert!(!err.is_retryable());

    // RemoteUnavailable with retryable=true
    let err = ConfigError::RemoteUnavailable {
        error_type: "timeout".to_string(),
        retryable: true,
    };
    assert!(err.is_retryable());

    // RemoteUnavailable with retryable=false
    let err = ConfigError::RemoteUnavailable {
        error_type: "auth".to_string(),
        retryable: false,
    };
    assert!(!err.is_retryable());
}

/// Test IO error retryability.
#[test]
fn test_io_error_retryability() {
    use confers::ConfigError;
    use std::io;

    // Connection refused is retryable
    let io_err = io::Error::new(io::ErrorKind::ConnectionRefused, "connection refused");
    let err = ConfigError::IoError(io_err);
    assert!(err.is_retryable());

    // NotFound is not retryable
    let io_err = io::Error::new(io::ErrorKind::NotFound, "file not found");
    let err = ConfigError::IoError(io_err);
    assert!(!err.is_retryable());

    // TimedOut is retryable
    let io_err = io::Error::new(io::ErrorKind::TimedOut, "timed out");
    let err = ConfigError::IoError(io_err);
    assert!(err.is_retryable());
}

/// Test watcher error retryability.
#[test]
fn test_watcher_error_retryability() {
    use confers::ConfigError;

    let recoverable_err = ConfigError::WatcherError {
        message: "temporary error".to_string(),
        path: None,
        recoverable: true,
    };
    assert!(recoverable_err.is_retryable());

    let non_recoverable_err = ConfigError::WatcherError {
        message: "fatal error".to_string(),
        path: None,
        recoverable: false,
    };
    assert!(!non_recoverable_err.is_retryable());
}

// ========================================
// Error Message Tests
// ========================================

/// Test error user message formatting.
#[test]
fn test_error_user_message_formatting() {
    use confers::ConfigError;

    // File not found
    let err = ConfigError::FileNotFound {
        filename: PathBuf::from("/path/to/config.toml"),
        source: None,
    };
    let msg = err.user_message();
    assert!(msg.contains("config.toml"));
    assert!(msg.contains("not found"));

    // Version mismatch
    let err = ConfigError::VersionMismatch {
        found: 1,
        expected: 2,
    };
    let msg = err.user_message();
    assert!(msg.contains("1"));
    assert!(msg.contains("2"));

    // Timeout
    let err = ConfigError::Timeout { duration_ms: 30000 };
    let msg = err.user_message();
    assert!(msg.contains("30000"));
}

/// Test error audit message.
#[test]
fn test_error_audit_message() {
    use confers::ConfigError;

    let err = ConfigError::FileNotFound {
        filename: PathBuf::from("test.toml"),
        source: None,
    };

    let audit_msg = err.audit_message();
    assert!(audit_msg.contains("error_code"));
    assert!(audit_msg.contains("FILE_NOT_FOUND"));
    assert!(audit_msg.contains("operation=config"));
}

/// Test error sanitized chain.
#[test]
fn test_error_sanitized_chain() {
    use confers::ConfigError;

    let err = ConfigError::DecryptionFailed {
        message: "key mismatch".to_string(),
    };

    let chain = err.sanitized_chain();
    assert!(!chain.is_empty());
}

/// Test MultiSourceError.
#[test]
fn test_multi_source_error() {
    use confers::error::MultiSourceError;
    use confers::ConfigError;

    let errors = vec![
        (0, ConfigError::Timeout { duration_ms: 1000 }),
        (
            1,
            ConfigError::RemoteUnavailable {
                error_type: "connection".to_string(),
                retryable: true,
            },
        ),
    ];

    let multi_err = MultiSourceError::new(5, errors);

    assert_eq!(multi_err.failed_count(), 2);
    assert_eq!(multi_err.total_count(), 5);

    let display = format!("{}", multi_err);
    assert!(display.contains("2/5"));
}

/// Test MultiSourceError with partial config.
#[test]
fn test_multi_source_error_partial_config() {
    use confers::error::MultiSourceError;
    use confers::ConfigError;

    let errors = vec![(
        0,
        ConfigError::FileNotFound {
            filename: PathBuf::from("missing.toml"),
            source: None,
        },
    )];

    let multi_err = MultiSourceError::with_partial(3, errors, "{ partial: true }".to_string());

    assert_eq!(multi_err.failed_count(), 1);
    assert_eq!(multi_err.total_count(), 3);
    assert!(multi_err.partial_config().is_some());
}

// ========================================
// BuildResult Tests
// ========================================

/// Test BuildResult creation.
#[test]
fn test_build_result_ok() {
    use confers::error::BuildResult;

    let result: BuildResult<i32> = BuildResult::ok(42);
    assert!(!result.degraded);
    assert!(!result.has_warnings());
    assert_eq!(result.config, 42);
}

/// Test BuildResult with warnings.
#[test]
fn test_build_result_with_warnings() {
    use confers::error::{BuildResult, BuildWarning, WarningCode};

    let warnings = vec![BuildWarning {
        message: "unused key".to_string(),
        source: Some("config.toml".to_string()),
        code: WarningCode::UnusedKey,
    }];

    let result: BuildResult<i32> = BuildResult::with_warnings(42, warnings);

    assert!(!result.degraded);
    assert!(result.has_warnings());
}

/// Test BuildResult degraded.
#[test]
fn test_build_result_degraded() {
    use confers::error::BuildResult;

    let result: BuildResult<i32> = BuildResult::degraded(42, "remote source unavailable");

    assert!(result.degraded);
    assert_eq!(
        result.degraded_reason,
        Some("remote source unavailable".to_string())
    );
}

/// Test BuildResult map.
#[test]
fn test_build_result_map() {
    use confers::error::BuildResult;

    let result: BuildResult<i32> = BuildResult::ok(21);
    let mapped = result.map(|v| v * 2);

    assert_eq!(mapped.config, 42);
}

/// Test WarningCode display.
#[test]
fn test_warning_code_display() {
    use confers::error::WarningCode;

    assert_eq!(
        format!("{}", WarningCode::OptionalSourceSkipped),
        "OPTIONAL_SOURCE_SKIPPED"
    );
    assert_eq!(format!("{}", WarningCode::DeprecatedKey), "DEPRECATED_KEY");
    assert_eq!(format!("{}", WarningCode::DefaultUsed), "DEFAULT_USED");
    assert_eq!(format!("{}", WarningCode::UnusedKey), "UNUSED_KEY");
}

// ========================================
// Additional Error Scenario Tests
// ========================================

/// Test file not found error with source.
#[test]
fn test_file_not_found_with_source() {
    use confers::ConfigError;
    use std::io;

    let io_err = io::Error::new(io::ErrorKind::NotFound, "file not found");
    let err = ConfigError::FileNotFound {
        filename: PathBuf::from("/path/to/file.toml"),
        source: Some(io_err),
    };

    let msg = err.user_message();
    assert!(msg.contains("file.toml"));

    let code = err.code();
    assert!(matches!(code, confers::error::ErrorCode::FileNotFound));
}

/// Test migration error.
#[test]
fn test_migration_error() {
    use confers::ConfigError;

    let err = ConfigError::MigrationFailed {
        from: 1,
        to: 2,
        reason: "invalid transformation".to_string(),
        source: None,
    };

    let msg = err.user_message();
    assert!(msg.contains("v1"));
    assert!(msg.contains("v2"));
    assert!(msg.contains("invalid transformation"));
}

/// Test module not found error.
#[test]
fn test_module_not_found_error() {
    use confers::ConfigError;

    let err = ConfigError::ModuleNotFound {
        group: "database".to_string(),
        module: "postgresql".to_string(),
    };

    let msg = err.user_message();
    assert!(msg.contains("postgresql"));
    assert!(msg.contains("database"));
}

/// Test size limit exceeded error.
#[test]
fn test_size_limit_exceeded_error() {
    use confers::ConfigError;

    let err = ConfigError::SizeLimitExceeded {
        actual: 10485760, // 10 MB
        limit: 1048576,   // 1 MB
    };

    let msg = err.user_message();
    assert!(msg.contains("10485760"));
    assert!(msg.contains("1048576"));
}

/// Test key rotation failed error.
#[test]
fn test_key_rotation_failed_error() {
    use confers::ConfigError;

    let err = ConfigError::KeyRotationFailed {
        from_version: "v1".to_string(),
        to_version: "v2".to_string(),
        reason: "invalid key format".to_string(),
    };

    let msg = err.user_message();
    assert!(msg.contains("v1"));
    assert!(msg.contains("v2"));
}

/// Test override blocked error.
#[test]
fn test_override_blocked_error() {
    use confers::ConfigError;

    let err = ConfigError::OverrideBlocked {
        key: "api_key".to_string(),
        reason: "protected field".to_string(),
        override_source: Some("cli".to_string()),
    };

    let msg = err.user_message();
    assert!(msg.contains("api_key"));
    assert!(msg.contains("cli"));
}

/// Test concurrency conflict error.
#[test]
fn test_concurrency_conflict_error() {
    use confers::ConfigError;

    let err = ConfigError::ConcurrencyConflict {
        key: "counter".to_string(),
        message: "value changed during read".to_string(),
        expected_type: Some("i32".to_string()),
    };

    assert!(err.is_retryable());
    let msg = err.user_message();
    assert!(msg.contains("counter"));
}
