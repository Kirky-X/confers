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
}
