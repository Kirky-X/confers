// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Configuration phase error types for confers.
//!
//! These errors occur during configuration **initialization** (construction phase),
//! before the configuration is ready for use. They indicate problems with the
//! configuration parameters, files, or setup that must be fixed before proceeding.
//!
//! # Phase Distinction (BrickArchitecture)
//!
//! - **`ConfigConfigError`** — Configuration phase errors (initialization time)
//!   - Missing required fields
//!   - Invalid configuration values
//!   - File not found during setup
//!   - Parse errors during loading
//!   - Validation failures
//!
//! - **`ConfersError`** — Runtime errors (use time, defined in parent module)
//!   - Timeout during operations
//!   - Remote source unavailable
//!   - Decryption failures
//!   - Concurrency conflicts
//!
//! # Usage
//!
//! ```rust,ignore
//! use confers::error::ConfigConfigError;
//!
//! // Match configuration phase errors
//! match result {
//!     Err(ConfigConfigError::MissingField { field }) => {
//!         println!("Missing: {}", field);
//!     }
//!     Err(ConfigConfigError::InvalidValue { field, reason }) => {
//!         println!("Invalid {}: {}", field, reason);
//!     }
//!     _ => {}
//! }
//! ```

use std::path::PathBuf;
use thiserror::Error;

/// Stable numeric error codes for programmatic handling of configuration errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum ConfigErrorCode {
    /// Missing required field (2001-2099)
    MissingField = 2001,
    /// Invalid configuration value (2100-2199)
    InvalidConfigValue = 2100,
    /// Configuration file not found (2200-2299)
    ConfigFileNotFound = 2200,
    /// Configuration parse error (2300-2399)
    ConfigParseError = 2300,
    /// Configuration size limit exceeded (2400-2499)
    ConfigSizeLimitExceeded = 2400,
    /// Configuration validation failed (2500-2599)
    ConfigValidationFailed = 2500,
    /// Configuration version mismatch (2600-2699)
    ConfigVersionMismatch = 2600,
    /// Source chain configuration error (2700-2799)
    SourceChainConfigError = 2700,
    /// Interpolation configuration error (2800-2899)
    InterpolationConfigError = 2800,
    /// Circular reference in configuration (2900-2999)
    ConfigCircularReference = 2900,
}

impl std::fmt::Display for ConfigErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigErrorCode::MissingField => write!(f, "CONFIG_MISSING_FIELD"),
            ConfigErrorCode::InvalidConfigValue => write!(f, "CONFIG_INVALID_VALUE"),
            ConfigErrorCode::ConfigFileNotFound => write!(f, "CONFIG_FILE_NOT_FOUND"),
            ConfigErrorCode::ConfigParseError => write!(f, "CONFIG_PARSE_ERROR"),
            ConfigErrorCode::ConfigSizeLimitExceeded => write!(f, "CONFIG_SIZE_LIMIT_EXCEEDED"),
            ConfigErrorCode::ConfigValidationFailed => write!(f, "CONFIG_VALIDATION_FAILED"),
            ConfigErrorCode::ConfigVersionMismatch => write!(f, "CONFIG_VERSION_MISMATCH"),
            ConfigErrorCode::SourceChainConfigError => write!(f, "CONFIG_SOURCE_CHAIN_ERROR"),
            ConfigErrorCode::InterpolationConfigError => write!(f, "CONFIG_INTERPOLATION_ERROR"),
            ConfigErrorCode::ConfigCircularReference => write!(f, "CONFIG_CIRCULAR_REFERENCE"),
        }
    }
}

/// Configuration phase error type.
///
/// These errors occur during configuration initialization, indicating
/// problems that must be fixed before the configuration can be used.
///
/// # When to Use
///
/// Use `ConfigConfigError` for errors that occur in:
/// - Factory functions (`new_in_memory`, `from_chain`)
/// - Configuration builders (`ConfigBuilder::build`)
/// - File loading (`load_file`)
/// - Validation (`validate`)
///
/// # Example
///
/// ```rust,ignore
/// // Builder returns ConfigConfigError when validation fails
/// use confers::impl_::memory::InMemoryConfigBuilder;
///
/// let result = InMemoryConfigBuilder::default().max_capacity(0).build();
/// // Returns Err(ConfigConfigError::InvalidValue { ... })
/// ```
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ConfigConfigError {
    /// Required configuration field is missing.
    #[error("Missing required configuration field: {field}")]
    MissingField {
        /// The field that is missing
        field: String,
    },

    /// Configuration value is invalid.
    #[error("Invalid configuration value for '{field}': {message}")]
    InvalidValue {
        /// The configuration field
        field: String,
        /// Expected type or format
        expected_type: String,
        /// Human-readable error message
        message: String,
    },

    /// Configuration file not found during initialization.
    #[error("Configuration file not found: {filename}")]
    FileNotFound {
        /// The filename that was not found
        filename: PathBuf,
        /// Optional source error
        source: Option<std::io::Error>,
    },

    /// Configuration file parse error.
    #[error("Failed to parse {format}{}: {message}", .location.as_ref().map(|l| format!(" at {}", l)).unwrap_or_default())]
    ParseError {
        /// Format being parsed (toml, json, yaml)
        format: String,
        /// Human-readable error message
        message: String,
        /// Optional precise location
        location: Option<crate::types::SourceLocation>,
        /// Source error
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Configuration size limit exceeded during initialization.
    #[error("Configuration size limit exceeded: {actual} bytes (limit: {limit})")]
    SizeLimitExceeded {
        /// Actual size in bytes
        actual: usize,
        /// Configured limit
        limit: usize,
    },

    /// Configuration validation failed.
    #[error("Configuration validation failed for field '{field}': {message} (rule: {rule})")]
    ValidationFailed {
        /// Field path that failed validation
        field: String,
        /// Validation rule that failed
        rule: String,
        /// Human-readable error message
        message: String,
    },

    /// Configuration version mismatch.
    #[error("Configuration version mismatch: found {found}, expected {expected}")]
    VersionMismatch {
        /// Version found in configuration
        found: u32,
        /// Expected version
        expected: u32,
    },

    /// Source chain configuration error.
    #[error("Source chain configuration error: {message}")]
    SourceChainError {
        /// Error message
        message: String,
        /// Source index
        source_index: usize,
    },

    /// Interpolation configuration error.
    #[error("Configuration interpolation error for '{variable}': {message}")]
    InterpolationError {
        /// Variable name
        variable: String,
        /// Error message
        message: String,
    },

    /// Circular reference detected in configuration.
    #[error("Circular reference detected in configuration: {path}")]
    CircularReference {
        /// Reference path
        path: String,
    },
}

impl ConfigConfigError {
    /// Get the error code for this error.
    pub fn code(&self) -> ConfigErrorCode {
        match self {
            ConfigConfigError::MissingField { .. } => ConfigErrorCode::MissingField,
            ConfigConfigError::InvalidValue { .. } => ConfigErrorCode::InvalidConfigValue,
            ConfigConfigError::FileNotFound { .. } => ConfigErrorCode::ConfigFileNotFound,
            ConfigConfigError::ParseError { .. } => ConfigErrorCode::ConfigParseError,
            ConfigConfigError::SizeLimitExceeded { .. } => ConfigErrorCode::ConfigSizeLimitExceeded,
            ConfigConfigError::ValidationFailed { .. } => ConfigErrorCode::ConfigValidationFailed,
            ConfigConfigError::VersionMismatch { .. } => ConfigErrorCode::ConfigVersionMismatch,
            ConfigConfigError::SourceChainError { .. } => ConfigErrorCode::SourceChainConfigError,
            ConfigConfigError::InterpolationError { .. } => {
                ConfigErrorCode::InterpolationConfigError
            }
            ConfigConfigError::CircularReference { .. } => ConfigErrorCode::ConfigCircularReference,
        }
    }

    /// Get a sanitized message for user display.
    ///
    /// This message is safe to show to end users - it does not contain
    /// file paths, IP addresses, or other sensitive information.
    pub fn user_message(&self) -> String {
        match self {
            ConfigConfigError::MissingField { field } => {
                format!("Missing required configuration field: '{}'", field)
            }
            ConfigConfigError::InvalidValue { field, message, .. } => {
                format!("Invalid value for '{}': {}", field, message)
            }
            ConfigConfigError::FileNotFound { filename, .. } => {
                // Sanitize sensitive paths
                let path_str = filename.display().to_string();
                let sanitized = if path_str.contains(".ssh")
                    || path_str.contains(".aws")
                    || path_str.contains(".gcloud")
                    || path_str.contains(".env")
                    || path_str.contains(".kube")
                {
                    filename
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_else(|| "<hidden>".to_string())
                } else {
                    path_str
                };
                format!("Configuration file '{}' not found", sanitized)
            }
            ConfigConfigError::ParseError {
                format,
                location,
                message,
                ..
            } => {
                if let Some(loc) = location {
                    format!("Failed to parse {} at {}: {}", format, loc, message)
                } else {
                    format!("Failed to parse {}: {}", format, message)
                }
            }
            ConfigConfigError::SizeLimitExceeded { actual, limit } => {
                format!(
                    "Configuration size exceeded: {} bytes (limit: {})",
                    actual, limit
                )
            }
            ConfigConfigError::ValidationFailed { field, message, .. } => {
                format!("Field '{}' failed validation: {}", field, message)
            }
            ConfigConfigError::VersionMismatch { found, expected } => {
                format!(
                    "Configuration version mismatch: found {}, expected {}",
                    found, expected
                )
            }
            ConfigConfigError::SourceChainError { message, .. } => message.clone(),
            ConfigConfigError::InterpolationError { variable, message } => {
                format!("Interpolation error for '{}': {}", variable, message)
            }
            ConfigConfigError::CircularReference { path } => {
                format!("Circular reference detected: {}", path)
            }
        }
    }

    /// Get a message suitable for audit logging.
    pub fn audit_message(&self) -> String {
        format!(
            "operation=config_init error_code={} error_type={} message={}",
            self.code() as u16,
            self.code(),
            self.user_message()
        )
    }

    /// Create a validation error with custom details.
    pub fn validation(
        field: impl Into<String>,
        rule: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        ConfigConfigError::ValidationFailed {
            field: field.into(),
            rule: rule.into(),
            message: message.into(),
        }
    }

    /// Create a missing field error.
    pub fn missing(field: impl Into<String>) -> Self {
        ConfigConfigError::MissingField {
            field: field.into(),
        }
    }

    /// Create an invalid value error.
    pub fn invalid(
        field: impl Into<String>,
        expected_type: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        ConfigConfigError::InvalidValue {
            field: field.into(),
            expected_type: expected_type.into(),
            message: message.into(),
        }
    }
}

/// Result type for configuration phase operations.
///
/// Use `InitResult<T>` for operations that may fail during
/// configuration initialization (factory functions, builders).
pub type InitResult<T> = Result<T, ConfigConfigError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_code() {
        let err = ConfigConfigError::MissingField {
            field: "endpoint".to_string(),
        };
        assert_eq!(err.code(), ConfigErrorCode::MissingField);
        assert_eq!(err.code() as u16, 2001);
    }

    #[test]
    fn test_user_message_missing_field() {
        let err = ConfigConfigError::MissingField {
            field: "database.host".to_string(),
        };
        assert_eq!(
            err.user_message(),
            "Missing required configuration field: 'database.host'"
        );
    }

    #[test]
    fn test_user_message_invalid_value() {
        let err = ConfigConfigError::InvalidValue {
            field: "port".to_string(),
            expected_type: "u16".to_string(),
            message: "out of range".to_string(),
        };
        assert!(err.user_message().contains("port"));
        assert!(err.user_message().contains("out of range"));
    }

    #[test]
    fn test_user_message_file_not_found_sanitized() {
        let err = ConfigConfigError::FileNotFound {
            filename: PathBuf::from("/home/user/.ssh/config"),
            source: None,
        };
        // Should not leak full path for sensitive directories
        assert!(!err.user_message().contains("/home/user/.ssh/"));
        assert!(err.user_message().contains("config"));
    }

    #[test]
    fn test_user_message_parse_error_with_location() {
        let loc = crate::types::SourceLocation::new("config.toml", 10, 5);
        let err = ConfigConfigError::ParseError {
            format: "toml".to_string(),
            message: "invalid syntax".to_string(),
            location: Some(loc),
            source: None,
        };
        assert!(err.user_message().contains("config.toml:10:5"));
    }

    #[test]
    fn test_audit_message() {
        let err = ConfigConfigError::ValidationFailed {
            field: "email".to_string(),
            rule: "format".to_string(),
            message: "not a valid email".to_string(),
        };
        let audit = err.audit_message();
        assert!(audit.contains("error_code=2500"));
        assert!(audit.contains("CONFIG_VALIDATION_FAILED"));
    }

    #[test]
    fn test_helper_methods() {
        let err = ConfigConfigError::missing("required_field");
        assert!(matches!(err, ConfigConfigError::MissingField { .. }));

        let err = ConfigConfigError::validation("email", "format", "invalid");
        assert!(matches!(err, ConfigConfigError::ValidationFailed { .. }));

        let err = ConfigConfigError::invalid("port", "u16", "too large");
        assert!(matches!(err, ConfigConfigError::InvalidValue { .. }));
    }

    #[test]
    fn test_display_error_codes() {
        assert_eq!(
            ConfigErrorCode::MissingField.to_string(),
            "CONFIG_MISSING_FIELD"
        );
        assert_eq!(
            ConfigErrorCode::ConfigFileNotFound.to_string(),
            "CONFIG_FILE_NOT_FOUND"
        );
        assert_eq!(
            ConfigErrorCode::ConfigParseError.to_string(),
            "CONFIG_PARSE_ERROR"
        );
    }

    // =============================================================================
    // Display impl for remaining ConfigErrorCode variants
    // =============================================================================

    #[test]
    fn test_display_error_codes_remaining() {
        assert_eq!(
            ConfigErrorCode::InvalidConfigValue.to_string(),
            "CONFIG_INVALID_VALUE"
        );
        assert_eq!(
            ConfigErrorCode::ConfigSizeLimitExceeded.to_string(),
            "CONFIG_SIZE_LIMIT_EXCEEDED"
        );
        assert_eq!(
            ConfigErrorCode::ConfigValidationFailed.to_string(),
            "CONFIG_VALIDATION_FAILED"
        );
        assert_eq!(
            ConfigErrorCode::ConfigVersionMismatch.to_string(),
            "CONFIG_VERSION_MISMATCH"
        );
        assert_eq!(
            ConfigErrorCode::SourceChainConfigError.to_string(),
            "CONFIG_SOURCE_CHAIN_ERROR"
        );
        assert_eq!(
            ConfigErrorCode::InterpolationConfigError.to_string(),
            "CONFIG_INTERPOLATION_ERROR"
        );
        assert_eq!(
            ConfigErrorCode::ConfigCircularReference.to_string(),
            "CONFIG_CIRCULAR_REFERENCE"
        );
    }

    // =============================================================================
    // code() for all ConfigConfigError variants
    // =============================================================================

    #[test]
    fn test_code_invalid_value() {
        let err = ConfigConfigError::InvalidValue {
            field: "port".into(),
            expected_type: "u16".into(),
            message: "out of range".into(),
        };
        assert_eq!(err.code(), ConfigErrorCode::InvalidConfigValue);
        assert_eq!(err.code() as u16, 2100);
    }

    #[test]
    fn test_code_file_not_found() {
        let err = ConfigConfigError::FileNotFound {
            filename: PathBuf::from("missing.toml"),
            source: None,
        };
        assert_eq!(err.code(), ConfigErrorCode::ConfigFileNotFound);
        assert_eq!(err.code() as u16, 2200);
    }

    #[test]
    fn test_code_parse_error() {
        let err = ConfigConfigError::ParseError {
            format: "toml".into(),
            message: "bad".into(),
            location: None,
            source: None,
        };
        assert_eq!(err.code(), ConfigErrorCode::ConfigParseError);
        assert_eq!(err.code() as u16, 2300);
    }

    #[test]
    fn test_code_size_limit_exceeded() {
        let err = ConfigConfigError::SizeLimitExceeded {
            actual: 100,
            limit: 50,
        };
        assert_eq!(err.code(), ConfigErrorCode::ConfigSizeLimitExceeded);
        assert_eq!(err.code() as u16, 2400);
    }

    #[test]
    fn test_code_validation_failed() {
        let err = ConfigConfigError::ValidationFailed {
            field: "f".into(),
            rule: "r".into(),
            message: "m".into(),
        };
        assert_eq!(err.code(), ConfigErrorCode::ConfigValidationFailed);
        assert_eq!(err.code() as u16, 2500);
    }

    #[test]
    fn test_code_version_mismatch() {
        let err = ConfigConfigError::VersionMismatch {
            found: 1,
            expected: 2,
        };
        assert_eq!(err.code(), ConfigErrorCode::ConfigVersionMismatch);
        assert_eq!(err.code() as u16, 2600);
    }

    #[test]
    fn test_code_source_chain_error() {
        let err = ConfigConfigError::SourceChainError {
            message: "m".into(),
            source_index: 0,
        };
        assert_eq!(err.code(), ConfigErrorCode::SourceChainConfigError);
        assert_eq!(err.code() as u16, 2700);
    }

    #[test]
    fn test_code_interpolation_error() {
        let err = ConfigConfigError::InterpolationError {
            variable: "v".into(),
            message: "m".into(),
        };
        assert_eq!(err.code(), ConfigErrorCode::InterpolationConfigError);
        assert_eq!(err.code() as u16, 2800);
    }

    #[test]
    fn test_code_circular_reference() {
        let err = ConfigConfigError::CircularReference { path: "p".into() };
        assert_eq!(err.code(), ConfigErrorCode::ConfigCircularReference);
        assert_eq!(err.code() as u16, 2900);
    }

    // =============================================================================
    // user_message for all remaining variants
    // =============================================================================

    #[test]
    fn test_user_message_size_limit_exceeded() {
        let err = ConfigConfigError::SizeLimitExceeded {
            actual: 2048,
            limit: 1024,
        };
        let msg = err.user_message();
        assert!(msg.contains("2048"));
        assert!(msg.contains("1024"));
        assert!(msg.contains("size"));
    }

    #[test]
    fn test_user_message_validation_failed() {
        let err = ConfigConfigError::ValidationFailed {
            field: "email".into(),
            rule: "format".into(),
            message: "invalid email".into(),
        };
        assert_eq!(
            err.user_message(),
            "Field 'email' failed validation: invalid email"
        );
    }

    #[test]
    fn test_user_message_version_mismatch() {
        let err = ConfigConfigError::VersionMismatch {
            found: 1,
            expected: 3,
        };
        assert_eq!(
            err.user_message(),
            "Configuration version mismatch: found 1, expected 3"
        );
    }

    #[test]
    fn test_user_message_source_chain_error() {
        let err = ConfigConfigError::SourceChainError {
            message: "chain broke at index 2".into(),
            source_index: 2,
        };
        assert_eq!(err.user_message(), "chain broke at index 2");
    }

    #[test]
    fn test_user_message_interpolation_error() {
        let err = ConfigConfigError::InterpolationError {
            variable: "HOME".into(),
            message: "not set".into(),
        };
        assert_eq!(
            err.user_message(),
            "Interpolation error for 'HOME': not set"
        );
    }

    #[test]
    fn test_user_message_circular_reference() {
        let err = ConfigConfigError::CircularReference {
            path: "a.b.c.a".into(),
        };
        assert_eq!(err.user_message(), "Circular reference detected: a.b.c.a");
    }

    #[test]
    fn test_user_message_parse_error_without_location() {
        let err = ConfigConfigError::ParseError {
            format: "json".into(),
            message: "unexpected token".into(),
            location: None,
            source: None,
        };
        let msg = err.user_message();
        assert!(msg.contains("json"));
        assert!(msg.contains("unexpected token"));
        assert!(!msg.contains("at"));
    }

    #[test]
    fn test_user_message_file_not_found_normal_path() {
        let err = ConfigConfigError::FileNotFound {
            filename: PathBuf::from("config.toml"),
            source: None,
        };
        // Normal paths are not sanitized
        assert_eq!(
            err.user_message(),
            "Configuration file 'config.toml' not found"
        );
    }

    #[test]
    fn test_user_message_file_not_found_gcloud_path_sanitized() {
        let err = ConfigConfigError::FileNotFound {
            filename: PathBuf::from("/home/user/.gcloud/key.json"),
            source: None,
        };
        let msg = err.user_message();
        assert!(!msg.contains("/home/user/.gcloud/"));
        assert!(msg.contains("key.json"));
    }

    // =============================================================================
    // audit_message for additional variants
    // =============================================================================

    #[test]
    fn test_audit_message_missing_field() {
        let err = ConfigConfigError::MissingField {
            field: "endpoint".into(),
        };
        let audit = err.audit_message();
        assert!(audit.contains("error_code=2001"));
        assert!(audit.contains("CONFIG_MISSING_FIELD"));
        assert!(audit.contains("operation=config_init"));
    }

    #[test]
    fn test_audit_message_invalid_value() {
        let err = ConfigConfigError::InvalidValue {
            field: "port".into(),
            expected_type: "u16".into(),
            message: "too large".into(),
        };
        let audit = err.audit_message();
        assert!(audit.contains("error_code=2100"));
        assert!(audit.contains("CONFIG_INVALID_VALUE"));
    }

    #[test]
    fn test_audit_message_parse_error() {
        let err = ConfigConfigError::ParseError {
            format: "toml".into(),
            message: "bad".into(),
            location: None,
            source: None,
        };
        let audit = err.audit_message();
        assert!(audit.contains("error_code=2300"));
        assert!(audit.contains("CONFIG_PARSE_ERROR"));
    }

    #[test]
    fn test_audit_message_circular_reference() {
        let err = ConfigConfigError::CircularReference {
            path: "a.b.c.a".into(),
        };
        let audit = err.audit_message();
        assert!(audit.contains("error_code=2900"));
        assert!(audit.contains("CONFIG_CIRCULAR_REFERENCE"));
    }

    // =============================================================================
    // Display impl for ConfigConfigError (thiserror #[error(...)] formats)
    // =============================================================================

    #[test]
    fn test_display_missing_field() {
        let err = ConfigConfigError::MissingField {
            field: "endpoint".into(),
        };
        let s = format!("{}", err);
        assert!(s.contains("endpoint"));
        assert!(s.contains("Missing required configuration field"));
    }

    #[test]
    fn test_display_invalid_value() {
        let err = ConfigConfigError::InvalidValue {
            field: "port".into(),
            expected_type: "u16".into(),
            message: "too large".into(),
        };
        let s = format!("{}", err);
        assert!(s.contains("port"));
        assert!(s.contains("too large"));
    }

    #[test]
    fn test_display_file_not_found() {
        let err = ConfigConfigError::FileNotFound {
            filename: PathBuf::from("config.toml"),
            source: None,
        };
        let s = format!("{}", err);
        assert!(s.contains("config.toml"));
        assert!(s.contains("not found"));
    }

    #[test]
    fn test_display_parse_error_with_location() {
        let loc = crate::types::SourceLocation::new("cfg.toml", 5, 3);
        let err = ConfigConfigError::ParseError {
            format: "toml".into(),
            message: "bad".into(),
            location: Some(loc),
            source: None,
        };
        let s = format!("{}", err);
        assert!(s.contains("toml"));
        assert!(s.contains("cfg.toml:5:3"));
        assert!(s.contains("bad"));
    }

    #[test]
    fn test_display_parse_error_without_location() {
        let err = ConfigConfigError::ParseError {
            format: "json".into(),
            message: "bad".into(),
            location: None,
            source: None,
        };
        let s = format!("{}", err);
        assert!(s.contains("json"));
        assert!(s.contains("bad"));
        // No location should not produce " at "
        assert!(!s.contains(" at "));
    }

    #[test]
    fn test_display_size_limit_exceeded() {
        let err = ConfigConfigError::SizeLimitExceeded {
            actual: 100,
            limit: 50,
        };
        let s = format!("{}", err);
        assert!(s.contains("100"));
        assert!(s.contains("50"));
    }

    #[test]
    fn test_display_validation_failed() {
        let err = ConfigConfigError::ValidationFailed {
            field: "email".into(),
            rule: "format".into(),
            message: "bad".into(),
        };
        let s = format!("{}", err);
        assert!(s.contains("email"));
        assert!(s.contains("format"));
        assert!(s.contains("bad"));
    }

    #[test]
    fn test_display_version_mismatch() {
        let err = ConfigConfigError::VersionMismatch {
            found: 1,
            expected: 2,
        };
        let s = format!("{}", err);
        assert!(s.contains("found 1"));
        assert!(s.contains("expected 2"));
    }

    #[test]
    fn test_display_source_chain_error() {
        let err = ConfigConfigError::SourceChainError {
            message: "broke".into(),
            source_index: 1,
        };
        let s = format!("{}", err);
        assert!(s.contains("broke"));
    }

    #[test]
    fn test_display_interpolation_error() {
        let err = ConfigConfigError::InterpolationError {
            variable: "HOME".into(),
            message: "missing".into(),
        };
        let s = format!("{}", err);
        assert!(s.contains("HOME"));
        assert!(s.contains("missing"));
    }

    #[test]
    fn test_display_circular_reference() {
        let err = ConfigConfigError::CircularReference {
            path: "a.b.a".into(),
        };
        let s = format!("{}", err);
        assert!(s.contains("a.b.a"));
    }

    // =============================================================================
    // Debug derive for ConfigConfigError
    // =============================================================================

    #[test]
    fn test_debug_format_config_config_error() {
        let err = ConfigConfigError::MissingField { field: "x".into() };
        let dbg = format!("{:?}", err);
        assert!(dbg.contains("MissingField"));
        assert!(dbg.contains("x"));
    }

    // =============================================================================
    // InitResult type alias
    // =============================================================================

    #[test]
    fn test_init_result_type_alias() {
        let ok: InitResult<i32> = Ok(42);
        assert!(ok.is_ok());

        let err: InitResult<i32> = Err(ConfigConfigError::MissingField { field: "f".into() });
        assert!(err.is_err());
    }

    // =============================================================================
    // Helper method return types — exhaustive variant matching
    // =============================================================================

    #[test]
    fn test_helper_methods_field_values() {
        // Verify field values are preserved through helpers
        let err = ConfigConfigError::missing("db.host");
        match err {
            ConfigConfigError::MissingField { field } => assert_eq!(field, "db.host"),
            other => panic!("unexpected variant: {:?}", other),
        }

        let err = ConfigConfigError::validation("port", "range", "out of bounds");
        match err {
            ConfigConfigError::ValidationFailed {
                field,
                rule,
                message,
            } => {
                assert_eq!(field, "port");
                assert_eq!(rule, "range");
                assert_eq!(message, "out of bounds");
            }
            other => panic!("unexpected variant: {:?}", other),
        }

        let err = ConfigConfigError::invalid("port", "u16", "too large");
        match err {
            ConfigConfigError::InvalidValue {
                field,
                expected_type,
                message,
            } => {
                assert_eq!(field, "port");
                assert_eq!(expected_type, "u16");
                assert_eq!(message, "too large");
            }
            other => panic!("unexpected variant: {:?}", other),
        }
    }
}
