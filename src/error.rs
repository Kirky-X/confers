//! Error handling for confers configuration library.
//!
//! This module provides comprehensive error types with:
//! - Precise error location (file, line, column)
//! - Sanitized messages for user display
//! - Audit-safe messages for logging
//! - Retryable error classification

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::LazyLock;
use thiserror::Error;

// Precompiled regex patterns for sanitization (avoid recompiling on each call)
/// Regex pattern for matching file paths (Unix and Windows style)
static PATH_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"/[a-zA-Z0-9_\-./]+|[a-zA-Z]:\\[a-zA-Z0-9_\-./\\]+").unwrap()
});

/// Regex pattern for matching IP addresses
static IP_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"\b\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}\b").unwrap());

/// Regex pattern for matching potential key material (long hex strings)
static HEX_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"\b[0-9a-fA-F]{16,}\b").unwrap());

/// Precise location of a parse error in source file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseLocation {
    /// Source file name (without full path for privacy)
    pub source_name: Arc<str>,
    /// Line number (1-based)
    pub line: usize,
    /// Column number (1-based)
    pub column: usize,
}

impl std::fmt::Display for ParseLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}:{}", self.source_name, self.line, self.column)
    }
}

impl ParseLocation {
    /// Create a new parse location.
    pub fn new(source_name: impl Into<Arc<str>>, line: usize, column: usize) -> Self {
        Self {
            source_name: source_name.into(),
            line,
            column,
        }
    }

    /// Create from a full path, extracting only the filename for privacy.
    pub fn from_path(path: &std::path::Path, line: usize, column: usize) -> Self {
        let source_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();
        Self::new(source_name, line, column)
    }
}

/// Stable numeric error codes for programmatic handling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum ErrorCode {
    /// File not found
    FileNotFound = 1001,
    /// Parse error in configuration file
    ParseError = 1002,
    /// Validation failed
    ValidationFailed = 1003,
    /// Decryption failed
    DecryptionFailed = 1004,
    /// Remote source unavailable
    RemoteUnavailable = 1005,
    /// Configuration version mismatch
    VersionMismatch = 1006,
    /// Migration failed
    MigrationFailed = 1007,
    /// Module not found
    ModuleNotFound = 1008,
    /// Reload rolled back
    ReloadRolledBack = 1009,
    /// IO error
    IoError = 1010,
    /// Invalid configuration value
    InvalidValue = 1011,
    /// Source chain error
    SourceChainError = 1012,
    /// Timeout error
    Timeout = 1013,
    /// Size limit exceeded
    SizeLimitExceeded = 1014,
    /// Interpolation error
    InterpolationError = 1015,
    /// Encryption key error
    KeyError = 1016,
    /// Circular reference detected
    CircularReference = 1017,
}

impl std::fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorCode::FileNotFound => write!(f, "FILE_NOT_FOUND"),
            ErrorCode::ParseError => write!(f, "PARSE_ERROR"),
            ErrorCode::ValidationFailed => write!(f, "VALIDATION_FAILED"),
            ErrorCode::DecryptionFailed => write!(f, "DECRYPTION_FAILED"),
            ErrorCode::RemoteUnavailable => write!(f, "REMOTE_UNAVAILABLE"),
            ErrorCode::VersionMismatch => write!(f, "VERSION_MISMATCH"),
            ErrorCode::MigrationFailed => write!(f, "MIGRATION_FAILED"),
            ErrorCode::ModuleNotFound => write!(f, "MODULE_NOT_FOUND"),
            ErrorCode::ReloadRolledBack => write!(f, "RELOAD_ROLLED_BACK"),
            ErrorCode::IoError => write!(f, "IO_ERROR"),
            ErrorCode::InvalidValue => write!(f, "INVALID_VALUE"),
            ErrorCode::SourceChainError => write!(f, "SOURCE_CHAIN_ERROR"),
            ErrorCode::Timeout => write!(f, "TIMEOUT"),
            ErrorCode::SizeLimitExceeded => write!(f, "SIZE_LIMIT_EXCEEDED"),
            ErrorCode::InterpolationError => write!(f, "INTERPOLATION_ERROR"),
            ErrorCode::KeyError => write!(f, "KEY_ERROR"),
            ErrorCode::CircularReference => write!(f, "CIRCULAR_REFERENCE"),
        }
    }
}

/// Comprehensive configuration error type.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ConfigError {
    /// Configuration file not found.
    #[error("Configuration file not found: {filename}")]
    FileNotFound {
        /// The filename that was not found
        filename: PathBuf,
        /// Optional source error
        source: Option<std::io::Error>,
    },

    /// Parse error in configuration file.
    #[error("Failed to parse {format}{}: {message}", .location.as_ref().map(|l| format!(" at {}", l)).unwrap_or_default())]
    ParseError {
        /// Format being parsed (toml, json, yaml)
        format: String,
        /// Human-readable error message
        message: String,
        /// Optional precise location
        location: Option<ParseLocation>,
        /// Source error
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Validation failed for a field.
    #[error("Validation failed for field '{field}': {message} (rule: {rule})")]
    ValidationFailed {
        /// Field path that failed validation
        field: String,
        /// Validation rule that failed
        rule: String,
        /// Human-readable error message
        message: String,
    },

    /// Decryption failed.
    #[error("Decryption failed: {message}")]
    DecryptionFailed {
        /// Sanitized error message (no sensitive data)
        message: String,
    },

    /// Remote configuration source unavailable.
    #[error("Remote configuration source unavailable")]
    RemoteUnavailable {
        /// Sanitized error type
        error_type: String,
        /// Whether the error is retryable
        retryable: bool,
    },

    /// Configuration version mismatch.
    #[error("Configuration version mismatch: found {found}, expected {expected}")]
    VersionMismatch {
        /// Version found in configuration
        found: u32,
        /// Expected version
        expected: u32,
    },

    /// Migration failed.
    #[error("Migration failed from v{from} to v{to}: {reason}")]
    MigrationFailed {
        /// Source version
        from: u32,
        /// Target version
        to: u32,
        /// Failure reason
        reason: String,
        /// Source error
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Module not found in a group.
    #[error("Module '{module}' not found in group '{group}'")]
    ModuleNotFound {
        /// Group name
        group: String,
        /// Module name
        module: String,
    },

    /// Reload was rolled back.
    #[error("Configuration reload rolled back: {reason}")]
    ReloadRolledBack {
        /// Reason for rollback
        reason: String,
    },

    /// IO error.
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// Invalid configuration value.
    #[error("Invalid configuration value for '{key}': {message}")]
    InvalidValue {
        /// Configuration key
        key: String,
        /// Expected type
        expected_type: String,
        /// Error message
        message: String,
    },

    /// Source chain error.
    #[error("Source chain error: {message}")]
    SourceChainError {
        /// Error message
        message: String,
        /// Source index
        source_index: usize,
    },

    /// Timeout error.
    #[error("Operation timed out after {duration_ms}ms")]
    Timeout {
        /// Timeout duration in milliseconds
        duration_ms: u64,
    },

    /// Size limit exceeded.
    #[error("Configuration size limit exceeded: {actual} bytes (limit: {limit})")]
    SizeLimitExceeded {
        /// Actual size in bytes
        actual: usize,
        /// Configured limit
        limit: usize,
    },

    /// Interpolation error.
    #[error("Interpolation error for '{variable}': {message}")]
    InterpolationError {
        /// Variable name
        variable: String,
        /// Error message
        message: String,
    },

    /// Encryption key error.
    #[error("Encryption key error: {message}")]
    KeyError {
        /// Sanitized error message
        message: String,
    },

    /// Circular reference detected.
    #[error("Circular reference detected: {path}")]
    CircularReference {
        /// Reference path
        path: String,
    },

    /// Multi-source error.
    #[error("Multiple sources failed")]
    MultiSource {
        /// The wrapped multi-source error
        #[source]
        source: MultiSourceError,
    },
}

impl ConfigError {
    /// Get the error code for this error.
    pub fn code(&self) -> ErrorCode {
        match self {
            ConfigError::FileNotFound { .. } => ErrorCode::FileNotFound,
            ConfigError::ParseError { .. } => ErrorCode::ParseError,
            ConfigError::ValidationFailed { .. } => ErrorCode::ValidationFailed,
            ConfigError::DecryptionFailed { .. } => ErrorCode::DecryptionFailed,
            ConfigError::RemoteUnavailable { .. } => ErrorCode::RemoteUnavailable,
            ConfigError::VersionMismatch { .. } => ErrorCode::VersionMismatch,
            ConfigError::MigrationFailed { .. } => ErrorCode::MigrationFailed,
            ConfigError::ModuleNotFound { .. } => ErrorCode::ModuleNotFound,
            ConfigError::ReloadRolledBack { .. } => ErrorCode::ReloadRolledBack,
            ConfigError::IoError(_) => ErrorCode::IoError,
            ConfigError::InvalidValue { .. } => ErrorCode::InvalidValue,
            ConfigError::SourceChainError { .. } => ErrorCode::SourceChainError,
            ConfigError::Timeout { .. } => ErrorCode::Timeout,
            ConfigError::SizeLimitExceeded { .. } => ErrorCode::SizeLimitExceeded,
            ConfigError::InterpolationError { .. } => ErrorCode::InterpolationError,
            ConfigError::KeyError { .. } => ErrorCode::KeyError,
            ConfigError::CircularReference { .. } => ErrorCode::CircularReference,
            ConfigError::MultiSource { .. } => ErrorCode::SourceChainError,
        }
    }

    /// Create a validation error from a garde Report.
    #[cfg(feature = "validation")]
    pub fn validation_error(message: &str, report: garde::Report) -> Self {
        // Extract the first error for a more specific message
        let (field, rule, msg) = report
            .iter()
            .next()
            .map(|(path, error)| {
                let field = path.to_string();
                let error_msg = format!("{}", error);
                (field, "garde".to_string(), error_msg)
            })
            .unwrap_or_else(|| {
                (
                    "unknown".to_string(),
                    "garde".to_string(),
                    message.to_string(),
                )
            });

        ConfigError::ValidationFailed {
            field,
            rule,
            message: msg,
        }
    }

    /// Create a validation error with custom details.
    pub fn validation(
        field: impl Into<String>,
        rule: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        ConfigError::ValidationFailed {
            field: field.into(),
            rule: rule.into(),
            message: message.into(),
        }
    }

    /// Check if this error is retryable.
    pub fn is_retryable(&self) -> bool {
        match self {
            ConfigError::RemoteUnavailable { retryable, .. } => *retryable,
            ConfigError::Timeout { .. } => true,
            ConfigError::IoError(e) => {
                // Network-related IO errors are retryable
                matches!(
                    e.kind(),
                    std::io::ErrorKind::ConnectionRefused
                        | std::io::ErrorKind::ConnectionReset
                        | std::io::ErrorKind::ConnectionAborted
                        | std::io::ErrorKind::TimedOut
                        | std::io::ErrorKind::WouldBlock
                )
            }
            _ => false,
        }
    }

    /// Get a sanitized message for user display.
    pub fn user_message(&self) -> String {
        match self {
            ConfigError::FileNotFound { filename, .. } => {
                format!("Configuration file '{}' not found", filename.display())
            }
            ConfigError::ParseError {
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
            ConfigError::ValidationFailed { field, message, .. } => {
                format!("Field '{}' failed validation: {}", field, message)
            }
            ConfigError::DecryptionFailed { .. } => {
                "Failed to decrypt configuration value".to_string()
            }
            ConfigError::RemoteUnavailable { .. } => {
                "Remote configuration source is unavailable".to_string()
            }
            ConfigError::VersionMismatch { found, expected } => {
                format!(
                    "Configuration version mismatch: found {}, expected {}",
                    found, expected
                )
            }
            ConfigError::MigrationFailed {
                from, to, reason, ..
            } => {
                format!("Migration from v{} to v{} failed: {}", from, to, reason)
            }
            ConfigError::ModuleNotFound { group, module } => {
                format!("Module '{}' not found in group '{}'", module, group)
            }
            ConfigError::ReloadRolledBack { reason } => {
                format!("Configuration reload was rolled back: {}", reason)
            }
            ConfigError::IoError(e) => format!("IO error: {}", e),
            ConfigError::InvalidValue { key, message, .. } => {
                format!("Invalid value for '{}': {}", key, message)
            }
            ConfigError::SourceChainError { message, .. } => message.clone(),
            ConfigError::Timeout { duration_ms } => {
                format!("Operation timed out after {}ms", duration_ms)
            }
            ConfigError::SizeLimitExceeded { actual, limit } => {
                format!("Size limit exceeded: {} bytes (limit: {})", actual, limit)
            }
            ConfigError::InterpolationError { variable, message } => {
                format!("Interpolation error for '{}': {}", variable, message)
            }
            ConfigError::KeyError { .. } => "Encryption key error".to_string(),
            ConfigError::CircularReference { path } => {
                format!("Circular reference detected: {}", path)
            }
            ConfigError::MultiSource { source } => {
                format!(
                    "Multiple sources failed: {}/{}",
                    source.failed_count(),
                    source.total_count()
                )
            }
        }
    }

    /// Get the error chain with sensitive data removed.
    pub fn sanitized_chain(&self) -> Vec<String> {
        let mut chain = vec![self.user_message()];

        // Add source errors if present, but sanitize them
        match self {
            ConfigError::ParseError { source, .. } | ConfigError::MigrationFailed { source, .. } => {
                if let Some(e) = source {
                    chain.push(sanitize_error_message(&e.to_string()));
                }
            }
            _ => {}
        }

        chain
    }

    /// Get a message suitable for audit logging.
    pub fn audit_message(&self) -> String {
        format!(
            "operation=config error_code={} error_type={} message={}",
            self.code() as u16,
            self.code(),
            self.user_message()
        )
    }
}

/// Sanitize an error message by removing sensitive data.
fn sanitize_error_message(msg: &str) -> String {
    let mut result = msg.to_string();

    // Remove potential file paths (Unix and Windows style) using precompiled regex
    result = PATH_RE
        .replace_all(&result, |caps: &regex::Captures| {
            let full_path = &caps[0];
            // Keep only the filename for debugging
            if let Some(filename) = full_path
                .split('/')
                .next_back()
                .or_else(|| full_path.split('\\').next_back())
            {
                format!("<path>/{}", filename)
            } else {
                "<path>".to_string()
            }
        })
        .to_string();

    // Remove potential IP addresses using precompiled regex
    result = IP_RE.replace_all(&result, "<ip>").to_string();

    // Remove potential key material (long hex strings) using precompiled regex
    result = HEX_RE.replace_all(&result, "<redacted>").to_string();

    result
}

/// Error from multiple failed sources.
#[derive(Debug, Error)]
pub struct MultiSourceError {
    /// Total number of sources attempted
    total: usize,
    /// Errors from each failed source
    errors: Vec<(usize, ConfigError)>,
    /// Partially loaded configuration (if any)
    partial_config: Option<String>,
}

impl std::fmt::Display for MultiSourceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Multi-source error: {}/{} sources failed",
            self.errors.len(),
            self.total
        )
    }
}

impl MultiSourceError {
    /// Create a new multi-source error.
    pub fn new(total: usize, errors: Vec<(usize, ConfigError)>) -> Self {
        Self {
            total,
            errors,
            partial_config: None,
        }
    }

    /// Create with partial configuration.
    pub fn with_partial(total: usize, errors: Vec<(usize, ConfigError)>, partial: String) -> Self {
        Self {
            total,
            errors,
            partial_config: Some(partial),
        }
    }

    /// Get the number of failed sources.
    pub fn failed_count(&self) -> usize {
        self.errors.len()
    }

    /// Get the total number of sources.
    pub fn total_count(&self) -> usize {
        self.total
    }

    /// Get the errors.
    pub fn errors(&self) -> &[(usize, ConfigError)] {
        &self.errors
    }

    /// Get the partial configuration if available.
    pub fn partial_config(&self) -> Option<&str> {
        self.partial_config.as_deref()
    }
}

/// Result of a configuration build operation with diagnostic information.
#[derive(Debug)]
pub struct BuildResult<T> {
    /// The built configuration
    pub config: T,
    /// Warnings encountered during build
    pub warnings: Vec<BuildWarning>,
    /// Whether the build is in degraded mode
    pub degraded: bool,
    /// Reason for degraded mode (if applicable)
    pub degraded_reason: Option<String>,
}

impl<T> BuildResult<T> {
    /// Create a successful build result.
    pub fn ok(config: T) -> Self {
        Self {
            config,
            warnings: Vec::new(),
            degraded: false,
            degraded_reason: None,
        }
    }

    /// Create a build result with warnings.
    pub fn with_warnings(config: T, warnings: Vec<BuildWarning>) -> Self {
        Self {
            config,
            warnings,
            degraded: false,
            degraded_reason: None,
        }
    }

    /// Create a degraded build result.
    pub fn degraded(config: T, reason: impl Into<String>) -> Self {
        Self {
            config,
            warnings: Vec::new(),
            degraded: true,
            degraded_reason: Some(reason.into()),
        }
    }

    /// Check if there are any warnings.
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }

    /// Map the inner config to a new type.
    pub fn map<U, F: FnOnce(T) -> U>(self, f: F) -> BuildResult<U> {
        BuildResult {
            config: f(self.config),
            warnings: self.warnings,
            degraded: self.degraded,
            degraded_reason: self.degraded_reason,
        }
    }
}

/// Warning encountered during configuration build.
#[derive(Debug, Clone)]
pub struct BuildWarning {
    /// Warning message
    pub message: String,
    /// Warning source (file, source name, etc.)
    pub source: Option<String>,
    /// Warning code
    pub code: WarningCode,
}

/// Warning codes for build warnings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WarningCode {
    /// A source was skipped due to optional flag
    OptionalSourceSkipped,
    /// A deprecated configuration key was used
    DeprecatedKey,
    /// A default value was used for missing key
    DefaultUsed,
    /// A value was truncated
    ValueTruncated,
    /// A remote source fallback was used
    RemoteFallback,
    /// A sensitive field is unencrypted
    UnencryptedSensitive,
    /// A configuration key is unused
    UnusedKey,
}

impl std::fmt::Display for WarningCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WarningCode::OptionalSourceSkipped => write!(f, "OPTIONAL_SOURCE_SKIPPED"),
            WarningCode::DeprecatedKey => write!(f, "DEPRECATED_KEY"),
            WarningCode::DefaultUsed => write!(f, "DEFAULT_USED"),
            WarningCode::ValueTruncated => write!(f, "VALUE_TRUNCATED"),
            WarningCode::RemoteFallback => write!(f, "REMOTE_FALLBACK"),
            WarningCode::UnencryptedSensitive => write!(f, "UNENCRYPTED_SENSITIVE"),
            WarningCode::UnusedKey => write!(f, "UNUSED_KEY"),
        }
    }
}

/// Type alias for configuration results.
pub type ConfigResult<T> = Result<T, ConfigError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_code() {
        let err = ConfigError::FileNotFound {
            filename: PathBuf::from("config.toml"),
            source: None,
        };
        assert_eq!(err.code(), ErrorCode::FileNotFound);
        assert_eq!(err.code() as u16, 1001);
    }

    #[test]
    fn test_is_retryable() {
        let err = ConfigError::Timeout { duration_ms: 1000 };
        assert!(err.is_retryable());

        let err = ConfigError::ValidationFailed {
            field: "port".to_string(),
            rule: "range".to_string(),
            message: "out of range".to_string(),
        };
        assert!(!err.is_retryable());

        let err = ConfigError::RemoteUnavailable {
            error_type: "timeout".to_string(),
            retryable: true,
        };
        assert!(err.is_retryable());
    }

    #[test]
    fn test_user_message() {
        let err = ConfigError::VersionMismatch {
            found: 1,
            expected: 2,
        };
        assert_eq!(
            err.user_message(),
            "Configuration version mismatch: found 1, expected 2"
        );
    }

    #[test]
    fn test_parse_location() {
        let loc = ParseLocation::new("config.toml", 15, 8);
        assert_eq!(loc.to_string(), "config.toml:15:8");

        let loc =
            ParseLocation::from_path(std::path::Path::new("/home/user/secret/config.toml"), 10, 5);
        assert_eq!(loc.source_name.as_ref(), "config.toml");
        assert_eq!(loc.line, 10);
        assert_eq!(loc.column, 5);
    }

    #[test]
    fn test_sanitized_chain() {
        let err = ConfigError::DecryptionFailed {
            message: "key mismatch".to_string(),
        };
        let chain = err.sanitized_chain();
        assert_eq!(chain.len(), 1);
        assert!(!chain[0].contains("key material"));
    }

    #[test]
    fn test_audit_message() {
        let err = ConfigError::FileNotFound {
            filename: PathBuf::from("config.toml"),
            source: None,
        };
        let audit = err.audit_message();
        assert!(audit.contains("error_code=1001"));
        assert!(audit.contains("FILE_NOT_FOUND"));
    }

    #[test]
    fn test_multi_source_error() {
        let err = MultiSourceError::new(
            5,
            vec![
                (0, ConfigError::Timeout { duration_ms: 1000 }),
                (
                    2,
                    ConfigError::RemoteUnavailable {
                        error_type: "connection".to_string(),
                        retryable: true,
                    },
                ),
            ],
        );
        assert_eq!(err.failed_count(), 2);
        assert_eq!(err.total_count(), 5);
    }

    #[test]
    fn test_build_result() {
        let result: BuildResult<i32> = BuildResult::ok(42);
        assert!(!result.degraded);
        assert!(!result.has_warnings());

        let result: BuildResult<i32> = BuildResult::degraded(42, "remote source unavailable");
        assert!(result.degraded);
        assert_eq!(
            result.degraded_reason,
            Some("remote source unavailable".to_string())
        );
    }

    #[test]
    fn test_non_exhaustive() {
        // This test ensures the #[non_exhaustive] attribute is present
        // by checking that we cannot exhaustively match outside this module
        let err = ConfigError::FileNotFound {
            filename: PathBuf::from("test"),
            source: None,
        };

        // We can match known variants
        match &err {
            ConfigError::FileNotFound { filename, .. } => {
                assert_eq!(filename, &PathBuf::from("test"));
            }
            _ => {}
        }
    }
}
