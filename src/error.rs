//! Error handling for confers configuration library.
//!
//! This module provides comprehensive error types with:
//! - Precise error location (file, line, column)
//! - Sanitized messages for user display
//! - Audit-safe messages for logging
//! - Retryable error classification
//!
//! # BrickArchitecture: Phase Separation
//!
//! This module follows BrickArchitecture error phase separation:
//!
//! - **`ConfigConfigError`** — Configuration phase errors (initialization time)
//!   - See [`config_error`] module for full documentation
//!   - Returned by factory functions and builders
//!
//! - **`ConfersError`** — Runtime errors (use time)
//!   - Defined in this module
//!   - Returned by trait methods during configuration use
//!
//! # Backward Compatibility
//!
//! The following aliases are provided for backward compatibility:
//! - `ConfigError` → `ConfersError`
//! - `ConfigResult<T>` → `ConfersResult<T>`
//! - `ErrorCode` → Runtime error codes

// Configuration phase errors (initialization time)
pub mod config_error;

// Re-export configuration phase error types
pub use config_error::ConfigConfigError;
pub use config_error::{ConfigErrorCode, InitResult};

use std::path::PathBuf;
use std::sync::LazyLock;
use thiserror::Error;

// Re-export SourceLocation as ParseLocation for backward compatibility
pub use crate::types::SourceLocation as ParseLocation;

// Precompiled regex patterns for sanitization (avoid recompiling on each call)

/// Regex pattern for matching file paths (Unix and Windows style)
static PATH_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"/[a-zA-Z0-9_\-./]+|[a-zA-Z]:\\[a-zA-Z0-9_\-./\\]+")
        .expect("PATH_RE regex is valid")
});

/// Regex pattern for matching IP addresses
static IP_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"\b\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}\b").expect("IP_RE regex is valid")
});

/// Regex pattern for matching potential key material (long hex strings)
static HEX_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"\b[0-9a-fA-F]{16,}\b").expect("HEX_RE regex is valid"));

/// Regex pattern for matching URLs with embedded credentials
static URL_WITH_CREDS_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"https?://[^/]+:[^@]+@[^/\s]+[/\s]?")
        .expect("URL_WITH_CREDS_RE regex is valid")
});

/// Regex pattern for matching JWT tokens
static JWT_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"eyJ[A-Za-z0-9_=-]+\.[A-Za-z0-9_=-]*\.?[A-Za-z0-9_=-]*")
        .expect("JWT_RE regex is valid")
});

/// Regex pattern for matching AWS access key IDs
static AWS_AK_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"\bAKIA[0-9A-Z]{16}\b").expect("AWS_AK_RE regex is valid"));

/// Regex pattern for matching AWS secret access keys (40-char alphanumeric)
static AWS_SAK_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"\b[A-Za-z0-9/+=]{40}\b").expect("AWS_SAK_RE regex is valid")
});

/// Stable numeric error codes for programmatic handling.
/// Values follow category-based ranges per dev-v2.md spec:
///   1-9 = File/IO, 100-199 = Validation, 200-299 = Crypto,
///   300-399 = Remote, 400-499 = Reference/Processing,
///   500-599 = Size/Watcher, 600-699 = Version/Migration,
///   700-799 = Modules, 800-899 = Multi-source,
///   900-999 = Timeout/Concurrency/Other
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum ErrorCode {
    FileNotFound = 1,
    FilePermission = 2,
    FileParseError = 3,
    IoError = 10,
    ValidationFailed = 100,
    TypeMismatch = 101,
    InvalidValue = 102,
    SchemaValidationFailed = 103,
    DecryptionFailed = 200,
    KeyNotFound = 201,
    KeyTooWeak = 202,
    KeyRotationFailed = 203,
    RemoteUnavailable = 300,
    RemoteTimeout = 301,
    CircularReference = 400,
    OverrideBlocked = 401,
    InterpolationError = 402,
    SizeLimitExceeded = 500,
    WatcherError = 501,
    VersionMismatch = 600,
    MigrationFailed = 601,
    ModuleNotFound = 700,
    ReloadRolledBack = 701,
    MultipleSources = 800,
    Timeout = 900,
    ConcurrencyConflict = 901,
    LockPoisoned = 902,
    HealthCheckFailed = 903,
    Unknown = 999,
}

impl std::fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorCode::FileNotFound => write!(f, "FILE_NOT_FOUND"),
            ErrorCode::FilePermission => write!(f, "FILE_PERMISSION"),
            ErrorCode::FileParseError => write!(f, "FILE_PARSE_ERROR"),
            ErrorCode::IoError => write!(f, "IO_ERROR"),
            ErrorCode::ValidationFailed => write!(f, "VALIDATION_FAILED"),
            ErrorCode::TypeMismatch => write!(f, "TYPE_MISMATCH"),
            ErrorCode::InvalidValue => write!(f, "INVALID_VALUE"),
            ErrorCode::SchemaValidationFailed => write!(f, "SCHEMA_VALIDATION_FAILED"),
            ErrorCode::DecryptionFailed => write!(f, "DECRYPTION_FAILED"),
            ErrorCode::KeyNotFound => write!(f, "KEY_NOT_FOUND"),
            ErrorCode::KeyTooWeak => write!(f, "KEY_TOO_WEAK"),
            ErrorCode::KeyRotationFailed => write!(f, "KEY_ROTATION_FAILED"),
            ErrorCode::RemoteUnavailable => write!(f, "REMOTE_UNAVAILABLE"),
            ErrorCode::RemoteTimeout => write!(f, "REMOTE_TIMEOUT"),
            ErrorCode::CircularReference => write!(f, "CIRCULAR_REFERENCE"),
            ErrorCode::OverrideBlocked => write!(f, "OVERRIDE_BLOCKED"),
            ErrorCode::InterpolationError => write!(f, "INTERPOLATION_ERROR"),
            ErrorCode::SizeLimitExceeded => write!(f, "SIZE_LIMIT_EXCEEDED"),
            ErrorCode::WatcherError => write!(f, "WATCHER_ERROR"),
            ErrorCode::VersionMismatch => write!(f, "VERSION_MISMATCH"),
            ErrorCode::MigrationFailed => write!(f, "MIGRATION_FAILED"),
            ErrorCode::ModuleNotFound => write!(f, "MODULE_NOT_FOUND"),
            ErrorCode::ReloadRolledBack => write!(f, "RELOAD_ROLLED_BACK"),
            ErrorCode::MultipleSources => write!(f, "MULTIPLE_SOURCES"),
            ErrorCode::Timeout => write!(f, "TIMEOUT"),
            ErrorCode::ConcurrencyConflict => write!(f, "CONCURRENCY_CONFLICT"),
            ErrorCode::LockPoisoned => write!(f, "LOCK_POISONED"),
            ErrorCode::HealthCheckFailed => write!(f, "HEALTH_CHECK_FAILED"),
            ErrorCode::Unknown => write!(f, "UNKNOWN"),
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

    /// Schema validation failed with error count.
    #[error("schema validation failed with {count} error(s)")]
    SchemaValidationFailed {
        /// Number of schema validation errors
        count: usize,
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

    #[error("Lock poisoned for resource '{resource}'")]
    LockPoisoned { resource: String },

    /// Multi-source error.
    #[error("Multiple sources failed")]
    MultiSource {
        /// The wrapped multi-source error
        #[source]
        source: MultiSourceError,
    },

    /// Concurrency conflict during configuration access.
    #[error("Concurrency conflict on key '{key}': {message}")]
    ConcurrencyConflict {
        /// The key that had the conflict
        key: String,
        /// Conflict description
        message: String,
        /// Expected value type
        expected_type: Option<String>,
    },

    /// Key rotation failed.
    #[error("Key rotation failed from '{from_version}' to '{to_version}': {reason}")]
    KeyRotationFailed {
        /// Source key version
        from_version: String,
        /// Target key version
        to_version: String,
        /// Failure reason
        reason: String,
    },

    /// Watcher error.
    #[error("Configuration watcher error: {message}")]
    WatcherError {
        /// Error message
        message: String,
        /// Source path being watched
        path: Option<PathBuf>,
        /// Whether the error is recoverable
        recoverable: bool,
    },

    /// Override blocked by protection rules.
    #[error("Override blocked for key '{key}': {reason}")]
    OverrideBlocked {
        /// The key that was blocked
        key: String,
        /// Reason for blocking
        reason: String,
        /// Source attempting the override
        override_source: Option<String>,
    },

    /// Health check failed.
    #[error("Health check failed: {reason}")]
    HealthCheckFailed {
        /// Reason for health check failure
        reason: String,
    },
}

impl ConfigError {
    /// Get the error code for this error.
    pub fn code(&self) -> ErrorCode {
        match self {
            ConfigError::FileNotFound { .. } => ErrorCode::FileNotFound,
            ConfigError::ParseError { .. } => ErrorCode::FileParseError,
            ConfigError::ValidationFailed { .. } => ErrorCode::ValidationFailed,
            ConfigError::SchemaValidationFailed { .. } => ErrorCode::SchemaValidationFailed,
            ConfigError::DecryptionFailed { .. } => ErrorCode::DecryptionFailed,
            ConfigError::RemoteUnavailable { .. } => ErrorCode::RemoteUnavailable,
            ConfigError::VersionMismatch { .. } => ErrorCode::VersionMismatch,
            ConfigError::MigrationFailed { .. } => ErrorCode::MigrationFailed,
            ConfigError::ModuleNotFound { .. } => ErrorCode::ModuleNotFound,
            ConfigError::ReloadRolledBack { .. } => ErrorCode::ReloadRolledBack,
            ConfigError::IoError(_) => ErrorCode::IoError,
            ConfigError::InvalidValue { .. } => ErrorCode::InvalidValue,
            ConfigError::SourceChainError { .. } => ErrorCode::MultipleSources,
            ConfigError::Timeout { .. } => ErrorCode::Timeout,
            ConfigError::SizeLimitExceeded { .. } => ErrorCode::SizeLimitExceeded,
            ConfigError::InterpolationError { .. } => ErrorCode::InterpolationError,
            ConfigError::KeyError { .. } => ErrorCode::KeyNotFound,
            ConfigError::CircularReference { .. } => ErrorCode::CircularReference,
            ConfigError::LockPoisoned { .. } => ErrorCode::LockPoisoned,
            ConfigError::MultiSource { .. } => ErrorCode::MultipleSources,
            ConfigError::ConcurrencyConflict { .. } => ErrorCode::ConcurrencyConflict,
            ConfigError::KeyRotationFailed { .. } => ErrorCode::KeyRotationFailed,
            ConfigError::WatcherError { .. } => ErrorCode::WatcherError,
            ConfigError::OverrideBlocked { .. } => ErrorCode::OverrideBlocked,
            ConfigError::HealthCheckFailed { .. } => ErrorCode::HealthCheckFailed,
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
            ConfigError::WatcherError { recoverable, .. } => *recoverable,
            ConfigError::ConcurrencyConflict { .. } => true,
            _ => false,
        }
    }

    /// Get a sanitized message for user display.
    pub fn user_message(&self) -> String {
        match self {
            ConfigError::FileNotFound { filename, .. } => {
                // Sanitize paths that may contain sensitive information
                let path_str = filename.display().to_string();
                // Truncate sensitive paths (.ssh, .aws, etc.) to just the filename
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
            ConfigError::SchemaValidationFailed { count } => {
                format!("Schema validation failed with {} error(s)", count)
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
            ConfigError::LockPoisoned { resource } => {
                format!("Lock poisoned for resource '{}'", resource)
            }
            ConfigError::MultiSource { source } => {
                format!(
                    "Multiple sources failed: {}/{}",
                    source.failed_count, source.total_count
                )
            }
            ConfigError::ConcurrencyConflict { key, message, .. } => {
                format!("Concurrency conflict on key '{}': {}", key, message)
            }
            ConfigError::KeyRotationFailed {
                from_version,
                to_version,
                reason,
            } => {
                format!(
                    "Key rotation failed from '{}' to '{}': {}",
                    from_version, to_version, reason
                )
            }
            ConfigError::WatcherError {
                message,
                path,
                recoverable,
            } => {
                let path_str = path
                    .as_ref()
                    .map(|p| format!(" for '{}'", p.display()))
                    .unwrap_or_default();
                let recovery_str = if *recoverable { " (recoverable)" } else { "" };
                format!("Watcher error{}: {}{}", path_str, message, recovery_str)
            }
            ConfigError::OverrideBlocked {
                key,
                reason,
                override_source,
            } => {
                let source_str = override_source
                    .as_ref()
                    .map(|s| format!(" from '{}'", s))
                    .unwrap_or_default();
                format!(
                    "Override blocked for key '{}'{}: {}",
                    key, source_str, reason
                )
            }
            ConfigError::HealthCheckFailed { reason } => {
                format!("Health check failed: {}", reason)
            }
        }
    }

    /// Get a detailed debug message for internal logging.
    ///
    /// Unlike `user_message()` which is safe to show to end users, this method
    /// may include file paths, IP addresses, and other diagnostic information
    /// useful for debugging. Do NOT expose this message to end users.
    ///
    /// For structured logging, prefer using the `error_code()` and field accessors.
    pub fn debug_message(&self) -> String {
        // Use the Display impl which gives full details
        let full = format!("{}", self);

        // Apply additional sanitization that still keeps some context
        let mut result = full;

        // Remove credentials from URLs but keep the URL structure
        result = URL_WITH_CREDS_RE
            .replace_all(&result, "https://<creds>@<host>/")
            .to_string();

        // Keep file paths but redact the directory part
        result = PATH_RE
            .replace_all(&result, |caps: &regex::Captures| {
                let full_path = &caps[0];
                full_path
                    .split('/')
                    .next_back()
                    .or_else(|| full_path.split('\\').next_back())
                    .map(|f| format!("<path>/{}", f))
                    .unwrap_or_else(|| "<path>".to_string())
            })
            .to_string();

        result
    }

    /// Check if this error may contain sensitive data.
    ///
    /// Returns `true` for errors that are likely to contain sensitive information
    /// such as keys, passwords, tokens, or credentials. Use this to determine
    /// whether to sanitize error messages before logging or displaying.
    ///
    /// Note: This is a heuristic check and may return `false` positives.
    /// Always prefer explicit sanitization via `sanitize_error_message()`.
    pub fn is_sensitive(&self) -> bool {
        // Check if the raw error message contains sensitive patterns
        let raw = format!("{}", self);

        // Check for sensitive patterns in the raw error
        JWT_RE.is_match(&raw)
            || AWS_AK_RE.is_match(&raw)
            || URL_WITH_CREDS_RE.is_match(&raw)
            || (HEX_RE.is_match(&raw) && raw.len() > 50) // Long hex strings are more likely keys
            || {
                // Check for common key/password field names
                let lower = raw.to_lowercase();
                lower.contains("secret")
                    || lower.contains("password")
                    || lower.contains("token")
                    || lower.contains("api_key")
                    || lower.contains("private_key")
                    || lower.contains("credential")
                    || lower.contains(" key ")  // standalone "key" word
                    || lower.ends_with("key")   // suffix "key" (e.g., "encryption key")
            }
    }

    /// Get the error chain with sensitive data removed.
    pub fn sanitized_chain(&self) -> Vec<String> {
        let mut chain = vec![self.user_message()];

        // Add source errors if present, but sanitize them
        match self {
            ConfigError::ParseError { source, .. }
            | ConfigError::MigrationFailed { source, .. } => {
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
///
/// This is the central sanitization function used by `user_message()` and
/// `sanitized_chain()`. It removes:
/// - File paths (replaced with `<path>/filename`)
/// - IP addresses (replaced with `<ip>`)
/// - Long hex strings / key material (replaced with `<redacted>`)
/// - URLs with embedded credentials
/// - JWT tokens
/// - AWS access key IDs
///
/// The user-facing message will not contain any of these sensitive patterns.
fn sanitize_error_message(msg: &str) -> String {
    let mut result = msg.to_string();

    // Remove URLs with embedded credentials first (before other replacements)
    result = URL_WITH_CREDS_RE
        .replace_all(&result, "<redacted_url>")
        .to_string();

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

    // Remove JWT tokens using precompiled regex
    result = JWT_RE.replace_all(&result, "<jwt_token>").to_string();

    // Remove AWS access key IDs using precompiled regex
    result = AWS_AK_RE
        .replace_all(&result, "<aws_access_key>")
        .to_string();

    // Remove AWS secret access keys (40-char strings near AWS context)
    // Only redact if surrounded by whitespace or common delimiters
    result = AWS_SAK_RE
        .replace_all(&result, "<aws_secret_key>")
        .to_string();

    // Remove potential key material (long hex strings) using precompiled regex
    result = HEX_RE.replace_all(&result, "<redacted>").to_string();

    result
}

/// Error from multiple failed sources.
#[derive(Debug, Error)]
#[error("multiple sources failed: {failed_count}/{total_count}")]
pub struct MultiSourceError {
    /// Errors from each failed source (source_name, error)
    pub errors: Vec<(String, ConfigError)>,
    /// Partially loaded configuration (if any), as JSON Value
    pub partial_config: Option<serde_json::Value>,
    /// Whether a fallback configuration was used
    pub fallback_used: bool,
    /// Number of failed sources
    pub failed_count: usize,
    /// Total number of sources attempted
    pub total_count: usize,
}

impl MultiSourceError {
    /// Create a new multi-source error.
    pub fn new<T: Into<String>>(total_count: usize, errors: Vec<(T, ConfigError)>) -> Self {
        let errors: Vec<(String, ConfigError)> =
            errors.into_iter().map(|(n, e)| (n.into(), e)).collect();
        let failed_count = errors.len();
        Self {
            errors,
            partial_config: None,
            fallback_used: false,
            failed_count,
            total_count,
        }
    }

    /// Create with partial configuration.
    pub fn with_partial<T: Into<String>>(
        total_count: usize,
        errors: Vec<(T, ConfigError)>,
        partial: serde_json::Value,
    ) -> Self {
        let errors: Vec<(String, ConfigError)> =
            errors.into_iter().map(|(n, e)| (n.into(), e)).collect();
        let failed_count = errors.len();
        Self {
            errors,
            partial_config: Some(partial),
            fallback_used: false,
            failed_count,
            total_count,
        }
    }

    /// Get the errors.
    pub fn errors(&self) -> &[(String, ConfigError)] {
        &self.errors
    }

    /// Get the partial configuration if available.
    pub fn partial_config(&self) -> Option<&serde_json::Value> {
        self.partial_config.as_ref()
    }

    /// Check if the build had partial success.
    pub fn has_partial_success(&self) -> bool {
        self.partial_config.is_some()
    }

    /// Count errors of a specific type.
    pub fn count_error_type(&self, code: ErrorCode) -> usize {
        self.errors.iter().filter(|(_, e)| e.code() == code).count()
    }
}

/// Result of a configuration build operation with diagnostic information.
#[derive(Debug)]
pub struct BuildResult<T> {
    /// The built configuration
    pub config: T,
    /// Warnings encountered during build
    pub warnings: Vec<SourceWarning>,
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
    pub fn with_warnings(config: T, warnings: Vec<SourceWarning>) -> Self {
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
pub struct SourceWarning {
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

// ============== BrickArchitecture Error Types ==============

/// Runtime error type for BrickArchitecture compliance.
///
/// This is the canonical name for runtime configuration errors. It is
/// structurally identical to `ConfigError` (same enum), following the
/// BrickArchitecture convention that library users interact with `ConfersError`
/// while `ConfigError` serves as the internal implementation detail.
///
/// # Phase Distinction
///
/// | Phase | Error Type | When |
/// |-------|-----------|------|
/// | Configuration (init) | [`ConfigConfigError`] | Factory functions, builders |
/// | Runtime (use) | `ConfersError` | get/set/delete operations |
///
/// # Example
///
/// ```rust
/// use confers::ConfersError;
/// # fn main() {}
/// ```
pub type ConfersError = ConfigError;

/// Result type alias for runtime operations.
///
/// Equivalent to `Result<T, ConfersError>`. Use this for all runtime
/// configuration operations (get/set/delete/health_check/shutdown).
pub type ConfersResult<T> = Result<T, ConfersError>;

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
        assert_eq!(err.code() as u16, 1);
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
        assert!(audit.contains("error_code=1"));
        assert!(audit.contains("FILE_NOT_FOUND"));
    }

    #[test]
    fn test_multi_source_error() {
        let err = MultiSourceError::new(
            5,
            vec![
                (
                    "source_a".to_string(),
                    ConfigError::Timeout { duration_ms: 1000 },
                ),
                (
                    "source_b".to_string(),
                    ConfigError::RemoteUnavailable {
                        error_type: "connection".to_string(),
                        retryable: true,
                    },
                ),
            ],
        );
        assert_eq!(err.failed_count, 2);
        assert_eq!(err.total_count, 5);
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
        if let ConfigError::FileNotFound { filename, .. } = &err {
            assert_eq!(filename, &PathBuf::from("test"));
        }
    }

    // =============================================================================
    // Error Sanitization Tests (9.3.6)
    // =============================================================================

    #[test]
    fn test_sanitize_error_message_full_path() {
        let msg = "Failed to load /home/user/project/config.toml";
        let sanitized = sanitize_error_message(msg);
        // Full paths should be converted to <path>/filename
        assert!(!sanitized.contains("/home/user/project/"));
        assert!(sanitized.contains("config.toml"));
        assert!(sanitized.contains("<path>"));
    }

    #[test]
    fn test_sanitize_error_message_url_with_credentials() {
        let msg = "Failed to fetch https://user:secret123@example.com/config.json"; // pragma: allowlist secret
        let sanitized = sanitize_error_message(msg);
        // Should not contain the credentials
        assert!(!sanitized.contains("user:secret123"));
        assert!(sanitized.contains("<redacted_url>") || sanitized.contains("<redacted>"));
    }

    #[test]
    fn test_sanitize_error_message_jwt_token() {
        let msg = "Validation failed for token eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4ifQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c"; // pragma: allowlist secret
        let sanitized = sanitize_error_message(msg);
        assert!(!sanitized.contains("eyJ"));
        assert!(sanitized.contains("<jwt_token>"));
    }

    #[test]
    fn test_sanitize_error_message_aws_access_key() {
        let msg = "AWS error: AKIAIOSFODNN7EXAMPLE is invalid"; // pragma: allowlist secret
        let sanitized = sanitize_error_message(msg);
        assert!(!sanitized.contains("AKIAIOSFODNN7EXAMPLE"));
        assert!(sanitized.contains("<aws_access_key>"));
    }

    #[test]
    fn test_sanitize_error_message_ip_address() {
        let msg = "Connection refused from 192.168.1.100";
        let sanitized = sanitize_error_message(msg);
        assert!(!sanitized.contains("192.168.1.100"));
        assert!(sanitized.contains("<ip>"));
    }

    #[test]
    fn test_sanitize_error_message_hex_key() {
        let msg = "Key mismatch: abcdef0123456789abcdef0123456789";
        let sanitized = sanitize_error_message(msg);
        assert!(!sanitized.contains("abcdef0123456789abcdef0123456789"));
        assert!(sanitized.contains("<redacted>"));
    }

    #[test]
    fn test_user_message_does_not_leak_sensitive_data() {
        // FileNotFound with sensitive-looking path
        let err = ConfigError::FileNotFound {
            filename: PathBuf::from("/home/user/.ssh/id_rsa"),
            source: None,
        };
        let user_msg = err.user_message();
        // Should show filename but not full path
        assert!(!user_msg.contains("/home/user/.ssh/"));
        assert!(user_msg.contains("id_rsa"));
    }

    #[test]
    fn test_debug_message_contains_file_path() {
        let err = ConfigError::FileNotFound {
            filename: PathBuf::from("/home/user/project/config.toml"),
            source: None,
        };
        let debug = err.debug_message();
        // Debug message should contain the full path for diagnostics
        assert!(debug.contains("config.toml") || debug.contains("<path>"));
    }

    #[test]
    fn test_is_sensitive_decryption_error() {
        let err = ConfigError::DecryptionFailed {
            message: "key mismatch".to_string(),
        };
        assert!(err.is_sensitive()); // "key" in message
    }

    #[test]
    fn test_is_sensitive_file_not_found() {
        // Normal file not found should not be sensitive
        let err = ConfigError::FileNotFound {
            filename: PathBuf::from("config.toml"),
            source: None,
        };
        assert!(!err.is_sensitive());
    }

    #[test]
    fn test_is_sensitive_key_error() {
        let err = ConfigError::KeyError {
            message: "encryption key error".to_string(),
        };
        assert!(err.is_sensitive()); // "key" in message
    }

    #[test]
    fn test_is_sensitive_aws_key_in_message() {
        let err = ConfigError::InvalidValue {
            key: "aws_access_key".to_string(),
            expected_type: "string".to_string(),
            message: "AKIAIOSFODNN7EXAMPLE is invalid".to_string(), // pragma: allowlist secret
        };
        assert!(err.is_sensitive()); // Contains AWS access key
    }

    // =============================================================================
    // Additional coverage for ErrorCode Display impl
    // =============================================================================

    #[test]
    fn test_error_code_display_all_variants() {
        assert_eq!(ErrorCode::FileNotFound.to_string(), "FILE_NOT_FOUND");
        assert_eq!(ErrorCode::FilePermission.to_string(), "FILE_PERMISSION");
        assert_eq!(ErrorCode::FileParseError.to_string(), "FILE_PARSE_ERROR");
        assert_eq!(ErrorCode::IoError.to_string(), "IO_ERROR");
        assert_eq!(ErrorCode::ValidationFailed.to_string(), "VALIDATION_FAILED");
        assert_eq!(ErrorCode::TypeMismatch.to_string(), "TYPE_MISMATCH");
        assert_eq!(ErrorCode::InvalidValue.to_string(), "INVALID_VALUE");
        assert_eq!(
            ErrorCode::SchemaValidationFailed.to_string(),
            "SCHEMA_VALIDATION_FAILED"
        );
        assert_eq!(ErrorCode::DecryptionFailed.to_string(), "DECRYPTION_FAILED");
        assert_eq!(ErrorCode::KeyNotFound.to_string(), "KEY_NOT_FOUND");
        assert_eq!(ErrorCode::KeyTooWeak.to_string(), "KEY_TOO_WEAK");
        assert_eq!(
            ErrorCode::KeyRotationFailed.to_string(),
            "KEY_ROTATION_FAILED"
        );
        assert_eq!(
            ErrorCode::RemoteUnavailable.to_string(),
            "REMOTE_UNAVAILABLE"
        );
        assert_eq!(ErrorCode::RemoteTimeout.to_string(), "REMOTE_TIMEOUT");
        assert_eq!(
            ErrorCode::CircularReference.to_string(),
            "CIRCULAR_REFERENCE"
        );
        assert_eq!(ErrorCode::OverrideBlocked.to_string(), "OVERRIDE_BLOCKED");
        assert_eq!(
            ErrorCode::InterpolationError.to_string(),
            "INTERPOLATION_ERROR"
        );
        assert_eq!(
            ErrorCode::SizeLimitExceeded.to_string(),
            "SIZE_LIMIT_EXCEEDED"
        );
        assert_eq!(ErrorCode::WatcherError.to_string(), "WATCHER_ERROR");
        assert_eq!(ErrorCode::VersionMismatch.to_string(), "VERSION_MISMATCH");
        assert_eq!(ErrorCode::MigrationFailed.to_string(), "MIGRATION_FAILED");
        assert_eq!(ErrorCode::ModuleNotFound.to_string(), "MODULE_NOT_FOUND");
        assert_eq!(
            ErrorCode::ReloadRolledBack.to_string(),
            "RELOAD_ROLLED_BACK"
        );
        assert_eq!(ErrorCode::MultipleSources.to_string(), "MULTIPLE_SOURCES");
        assert_eq!(ErrorCode::Timeout.to_string(), "TIMEOUT");
        assert_eq!(
            ErrorCode::ConcurrencyConflict.to_string(),
            "CONCURRENCY_CONFLICT"
        );
        assert_eq!(ErrorCode::LockPoisoned.to_string(), "LOCK_POISONED");
        assert_eq!(
            ErrorCode::HealthCheckFailed.to_string(),
            "HEALTH_CHECK_FAILED"
        );
        assert_eq!(ErrorCode::Unknown.to_string(), "UNKNOWN");
    }

    // =============================================================================
    // ConfigError::code() mapping for all variants
    // =============================================================================

    #[test]
    fn test_config_error_code_mapping_all_variants() {
        let err = ConfigError::ParseError {
            format: "toml".into(),
            message: "bad".into(),
            location: None,
            source: None,
        };
        assert_eq!(err.code(), ErrorCode::FileParseError);

        let err = ConfigError::ValidationFailed {
            field: "f".into(),
            rule: "r".into(),
            message: "m".into(),
        };
        assert_eq!(err.code(), ErrorCode::ValidationFailed);

        let err = ConfigError::SchemaValidationFailed { count: 1 };
        assert_eq!(err.code(), ErrorCode::SchemaValidationFailed);

        let err = ConfigError::DecryptionFailed {
            message: "m".into(),
        };
        assert_eq!(err.code(), ErrorCode::DecryptionFailed);

        let err = ConfigError::RemoteUnavailable {
            error_type: "timeout".into(),
            retryable: false,
        };
        assert_eq!(err.code(), ErrorCode::RemoteUnavailable);

        let err = ConfigError::VersionMismatch {
            found: 1,
            expected: 2,
        };
        assert_eq!(err.code(), ErrorCode::VersionMismatch);

        let err = ConfigError::MigrationFailed {
            from: 1,
            to: 2,
            reason: "r".into(),
            source: None,
        };
        assert_eq!(err.code(), ErrorCode::MigrationFailed);

        let err = ConfigError::ModuleNotFound {
            group: "g".into(),
            module: "m".into(),
        };
        assert_eq!(err.code(), ErrorCode::ModuleNotFound);

        let err = ConfigError::ReloadRolledBack { reason: "r".into() };
        assert_eq!(err.code(), ErrorCode::ReloadRolledBack);

        let err = ConfigError::InvalidValue {
            key: "k".into(),
            expected_type: "t".into(),
            message: "m".into(),
        };
        assert_eq!(err.code(), ErrorCode::InvalidValue);

        let err = ConfigError::SourceChainError {
            message: "m".into(),
            source_index: 0,
        };
        assert_eq!(err.code(), ErrorCode::MultipleSources);

        let err = ConfigError::Timeout { duration_ms: 10 };
        assert_eq!(err.code(), ErrorCode::Timeout);

        let err = ConfigError::SizeLimitExceeded {
            actual: 10,
            limit: 5,
        };
        assert_eq!(err.code(), ErrorCode::SizeLimitExceeded);

        let err = ConfigError::InterpolationError {
            variable: "v".into(),
            message: "m".into(),
        };
        assert_eq!(err.code(), ErrorCode::InterpolationError);

        let err = ConfigError::KeyError {
            message: "m".into(),
        };
        assert_eq!(err.code(), ErrorCode::KeyNotFound);

        let err = ConfigError::CircularReference { path: "p".into() };
        assert_eq!(err.code(), ErrorCode::CircularReference);

        let err = ConfigError::LockPoisoned {
            resource: "r".into(),
        };
        assert_eq!(err.code(), ErrorCode::LockPoisoned);

        let err = ConfigError::ConcurrencyConflict {
            key: "k".into(),
            message: "m".into(),
            expected_type: None,
        };
        assert_eq!(err.code(), ErrorCode::ConcurrencyConflict);

        let err = ConfigError::KeyRotationFailed {
            from_version: "v1".into(),
            to_version: "v2".into(),
            reason: "r".into(),
        };
        assert_eq!(err.code(), ErrorCode::KeyRotationFailed);

        let err = ConfigError::WatcherError {
            message: "m".into(),
            path: None,
            recoverable: false,
        };
        assert_eq!(err.code(), ErrorCode::WatcherError);

        let err = ConfigError::OverrideBlocked {
            key: "k".into(),
            reason: "r".into(),
            override_source: None,
        };
        assert_eq!(err.code(), ErrorCode::OverrideBlocked);

        let err = ConfigError::HealthCheckFailed { reason: "r".into() };
        assert_eq!(err.code(), ErrorCode::HealthCheckFailed);

        // MultiSource wraps a MultiSourceError
        let inner = MultiSourceError::new(1, vec![("s", ConfigError::Timeout { duration_ms: 1 })]);
        let err = ConfigError::MultiSource { source: inner };
        assert_eq!(err.code(), ErrorCode::MultipleSources);
    }

    // =============================================================================
    // validation() helper
    // =============================================================================

    #[test]
    fn test_validation_helper_constructs_validation_failed() {
        let err = ConfigError::validation("email", "format", "not a valid email");
        match err {
            ConfigError::ValidationFailed {
                field,
                rule,
                message,
            } => {
                assert_eq!(field, "email");
                assert_eq!(rule, "format");
                assert_eq!(message, "not a valid email");
            }
            other => panic!("expected ValidationFailed, got {:?}", other),
        }
    }

    // =============================================================================
    // is_retryable for IO error kinds, watcher, concurrency, remote
    // =============================================================================

    #[test]
    fn test_is_retryable_io_error_connection_refused() {
        let err = ConfigError::IoError(std::io::Error::new(
            std::io::ErrorKind::ConnectionRefused,
            "refused",
        ));
        assert!(err.is_retryable());
    }

    #[test]
    fn test_is_retryable_io_error_connection_reset() {
        let err = ConfigError::IoError(std::io::Error::new(
            std::io::ErrorKind::ConnectionReset,
            "reset",
        ));
        assert!(err.is_retryable());
    }

    #[test]
    fn test_is_retryable_io_error_timed_out() {
        let err = ConfigError::IoError(std::io::Error::new(
            std::io::ErrorKind::TimedOut,
            "timed out",
        ));
        assert!(err.is_retryable());
    }

    #[test]
    fn test_is_retryable_io_error_not_retryable() {
        let err = ConfigError::IoError(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "not found",
        ));
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_is_retryable_watcher_error_recoverable() {
        let err = ConfigError::WatcherError {
            message: "transient".into(),
            path: None,
            recoverable: true,
        };
        assert!(err.is_retryable());
    }

    #[test]
    fn test_is_retryable_watcher_error_not_recoverable() {
        let err = ConfigError::WatcherError {
            message: "fatal".into(),
            path: None,
            recoverable: false,
        };
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_is_retryable_concurrency_conflict() {
        let err = ConfigError::ConcurrencyConflict {
            key: "k".into(),
            message: "m".into(),
            expected_type: None,
        };
        assert!(err.is_retryable());
    }

    #[test]
    fn test_is_retryable_remote_unavailable_not_retryable() {
        let err = ConfigError::RemoteUnavailable {
            error_type: "auth".into(),
            retryable: false,
        };
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_is_retryable_other_variants_not_retryable() {
        let err = ConfigError::FileNotFound {
            filename: PathBuf::from("x"),
            source: None,
        };
        assert!(!err.is_retryable());

        let err = ConfigError::ValidationFailed {
            field: "f".into(),
            rule: "r".into(),
            message: "m".into(),
        };
        assert!(!err.is_retryable());
    }

    // =============================================================================
    // From<std::io::Error> impl
    // =============================================================================

    #[test]
    fn test_from_io_error() {
        let io_err = std::io::Error::other("disk full");
        let config_err: ConfigError = io_err.into();
        assert!(matches!(config_err, ConfigError::IoError(_)));
        assert_eq!(config_err.code(), ErrorCode::IoError);
    }

    // =============================================================================
    // user_message for all variants
    // =============================================================================

    #[test]
    fn test_user_message_parse_error_with_location() {
        let loc = ParseLocation::new("config.toml", 10, 5);
        let err = ConfigError::ParseError {
            format: "toml".into(),
            message: "bad syntax".into(),
            location: Some(loc),
            source: None,
        };
        let msg = err.user_message();
        assert!(msg.contains("toml"));
        assert!(msg.contains("config.toml:10:5"));
        assert!(msg.contains("bad syntax"));
    }

    #[test]
    fn test_user_message_parse_error_without_location() {
        let err = ConfigError::ParseError {
            format: "json".into(),
            message: "bad".into(),
            location: None,
            source: None,
        };
        let msg = err.user_message();
        assert!(msg.contains("json"));
        assert!(msg.contains("bad"));
        assert!(!msg.contains("at"));
    }

    #[test]
    fn test_user_message_validation_failed() {
        let err = ConfigError::ValidationFailed {
            field: "port".into(),
            rule: "range".into(),
            message: "out of range".into(),
        };
        assert_eq!(
            err.user_message(),
            "Field 'port' failed validation: out of range"
        );
    }

    #[test]
    fn test_user_message_schema_validation_failed() {
        let err = ConfigError::SchemaValidationFailed { count: 3 };
        assert_eq!(
            err.user_message(),
            "Schema validation failed with 3 error(s)"
        );
    }

    #[test]
    fn test_user_message_decryption_failed_is_sanitized() {
        let err = ConfigError::DecryptionFailed {
            message: "key mismatch detail".into(),
        };
        // user_message returns a generic sanitized message, not the raw message
        assert_eq!(err.user_message(), "Failed to decrypt configuration value");
    }

    #[test]
    fn test_user_message_remote_unavailable() {
        let err = ConfigError::RemoteUnavailable {
            error_type: "timeout".into(),
            retryable: true,
        };
        assert_eq!(
            err.user_message(),
            "Remote configuration source is unavailable"
        );
    }

    #[test]
    fn test_user_message_migration_failed() {
        let err = ConfigError::MigrationFailed {
            from: 1,
            to: 2,
            reason: "schema change".into(),
            source: None,
        };
        assert_eq!(
            err.user_message(),
            "Migration from v1 to v2 failed: schema change"
        );
    }

    #[test]
    fn test_user_message_module_not_found() {
        let err = ConfigError::ModuleNotFound {
            group: "g1".into(),
            module: "m1".into(),
        };
        assert_eq!(err.user_message(), "Module 'm1' not found in group 'g1'");
    }

    #[test]
    fn test_user_message_reload_rolled_back() {
        let err = ConfigError::ReloadRolledBack {
            reason: "validation".into(),
        };
        assert_eq!(
            err.user_message(),
            "Configuration reload was rolled back: validation"
        );
    }

    #[test]
    fn test_user_message_io_error() {
        let err = ConfigError::IoError(std::io::Error::other("disk full"));
        let msg = err.user_message();
        assert!(msg.contains("IO error"));
        assert!(msg.contains("disk full"));
    }

    #[test]
    fn test_user_message_invalid_value() {
        let err = ConfigError::InvalidValue {
            key: "port".into(),
            expected_type: "u16".into(),
            message: "too large".into(),
        };
        assert_eq!(err.user_message(), "Invalid value for 'port': too large");
    }

    #[test]
    fn test_user_message_source_chain_error() {
        let err = ConfigError::SourceChainError {
            message: "chain broke".into(),
            source_index: 2,
        };
        assert_eq!(err.user_message(), "chain broke");
    }

    #[test]
    fn test_user_message_timeout() {
        let err = ConfigError::Timeout { duration_ms: 5000 };
        assert_eq!(err.user_message(), "Operation timed out after 5000ms");
    }

    #[test]
    fn test_user_message_size_limit_exceeded() {
        let err = ConfigError::SizeLimitExceeded {
            actual: 1024,
            limit: 512,
        };
        assert_eq!(
            err.user_message(),
            "Size limit exceeded: 1024 bytes (limit: 512)"
        );
    }

    #[test]
    fn test_user_message_interpolation_error() {
        let err = ConfigError::InterpolationError {
            variable: "HOME".into(),
            message: "not set".into(),
        };
        assert_eq!(
            err.user_message(),
            "Interpolation error for 'HOME': not set"
        );
    }

    #[test]
    fn test_user_message_key_error_is_sanitized() {
        let err = ConfigError::KeyError {
            message: "detailed key info".into(),
        };
        // user_message returns generic message, not the raw message
        assert_eq!(err.user_message(), "Encryption key error");
    }

    #[test]
    fn test_user_message_circular_reference() {
        let err = ConfigError::CircularReference {
            path: "a.b.c.a".into(),
        };
        assert_eq!(err.user_message(), "Circular reference detected: a.b.c.a");
    }

    #[test]
    fn test_user_message_lock_poisoned() {
        let err = ConfigError::LockPoisoned {
            resource: "config".into(),
        };
        assert_eq!(err.user_message(), "Lock poisoned for resource 'config'");
    }

    #[test]
    fn test_user_message_multi_source() {
        let inner = MultiSourceError::new(3, vec![("s1", ConfigError::Timeout { duration_ms: 1 })]);
        let err = ConfigError::MultiSource { source: inner };
        assert_eq!(err.user_message(), "Multiple sources failed: 1/3");
    }

    #[test]
    fn test_user_message_concurrency_conflict() {
        let err = ConfigError::ConcurrencyConflict {
            key: "k".into(),
            message: "m".into(),
            expected_type: Some("string".into()),
        };
        assert_eq!(err.user_message(), "Concurrency conflict on key 'k': m");
    }

    #[test]
    fn test_user_message_key_rotation_failed() {
        let err = ConfigError::KeyRotationFailed {
            from_version: "v1".into(),
            to_version: "v2".into(),
            reason: "invalid".into(),
        };
        assert_eq!(
            err.user_message(),
            "Key rotation failed from 'v1' to 'v2': invalid"
        );
    }

    #[test]
    fn test_user_message_watcher_error_with_path() {
        let err = ConfigError::WatcherError {
            message: "lost file".into(),
            path: Some(PathBuf::from("/etc/config.toml")),
            recoverable: true,
        };
        let msg = err.user_message();
        assert!(msg.contains("Watcher error"));
        assert!(msg.contains("config.toml"));
        assert!(msg.contains("(recoverable)"));
    }

    #[test]
    fn test_user_message_watcher_error_no_path_not_recoverable() {
        let err = ConfigError::WatcherError {
            message: "fatal error".into(),
            path: None,
            recoverable: false,
        };
        let msg = err.user_message();
        assert!(msg.contains("Watcher error"));
        assert!(!msg.contains("(recoverable)"));
    }

    #[test]
    fn test_user_message_override_blocked_with_source() {
        let err = ConfigError::OverrideBlocked {
            key: "k".into(),
            reason: "protected".into(),
            override_source: Some("cli".into()),
        };
        let msg = err.user_message();
        assert!(msg.contains("Override blocked for key 'k'"));
        assert!(msg.contains("from 'cli'"));
        assert!(msg.contains("protected"));
    }

    #[test]
    fn test_user_message_override_blocked_no_source() {
        let err = ConfigError::OverrideBlocked {
            key: "k".into(),
            reason: "protected".into(),
            override_source: None,
        };
        let msg = err.user_message();
        assert!(msg.contains("Override blocked for key 'k'"));
        assert!(!msg.contains("from '"));
    }

    #[test]
    fn test_user_message_health_check_failed() {
        let err = ConfigError::HealthCheckFailed {
            reason: "db down".into(),
        };
        assert_eq!(err.user_message(), "Health check failed: db down");
    }

    #[test]
    fn test_user_message_file_not_found_normal_path() {
        let err = ConfigError::FileNotFound {
            filename: PathBuf::from("config.toml"),
            source: None,
        };
        // Normal paths should be shown as-is (no sanitization)
        assert_eq!(
            err.user_message(),
            "Configuration file 'config.toml' not found"
        );
    }

    #[test]
    fn test_user_message_file_not_found_aws_path_sanitized() {
        let err = ConfigError::FileNotFound {
            filename: PathBuf::from("/home/user/.aws/credentials"),
            source: None,
        };
        let msg = err.user_message();
        assert!(!msg.contains("/home/user/.aws/"));
        assert!(msg.contains("credentials"));
    }

    #[test]
    fn test_user_message_file_not_found_kube_path_sanitized() {
        let err = ConfigError::FileNotFound {
            filename: PathBuf::from("/root/.kube/config"),
            source: None,
        };
        let msg = err.user_message();
        assert!(!msg.contains("/root/.kube/"));
        assert!(msg.contains("config"));
    }

    #[test]
    fn test_user_message_file_not_found_env_path_sanitized() {
        let err = ConfigError::FileNotFound {
            filename: PathBuf::from("/app/.env"),
            source: None,
        };
        let msg = err.user_message();
        assert!(!msg.contains("/app/"));
        assert!(msg.contains(".env") || msg.contains("env"));
    }

    // =============================================================================
    // debug_message with URL credentials
    // =============================================================================

    #[test]
    fn test_debug_message_redacts_url_credentials() {
        let err = ConfigError::DecryptionFailed {
            message: "fetch https://user:passw0rd123@example.com/keys failed".into(), // pragma: allowlist secret
        };
        let dbg = err.debug_message();
        assert!(!dbg.contains("user:passw0rd123"));
        assert!(dbg.contains("<creds>") || dbg.contains("<host>"));
    }

    // =============================================================================
    // is_sensitive for sensitive patterns
    // =============================================================================

    #[test]
    fn test_is_sensitive_url_with_credentials() {
        let err = ConfigError::DecryptionFailed {
            message: "fetch https://user:passw0rd123@example.com/keys failed".into(), // pragma: allowlist secret
        };
        assert!(err.is_sensitive());
    }

    #[test]
    fn test_is_sensitive_jwt_token() {
        let err = ConfigError::DecryptionFailed {
            message: "token eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.payload.sig invalid".into(), // pragma: allowlist secret
        };
        assert!(err.is_sensitive());
    }

    #[test]
    fn test_is_sensitive_password_field() {
        let err = ConfigError::InvalidValue {
            key: "db.password".into(),
            expected_type: "string".into(),
            message: "too short".into(),
        };
        assert!(err.is_sensitive());
    }

    #[test]
    fn test_is_sensitive_token_field() {
        let err = ConfigError::InvalidValue {
            key: "auth.token".into(),
            expected_type: "string".into(),
            message: "expired".into(),
        };
        assert!(err.is_sensitive());
    }

    #[test]
    fn test_is_sensitive_api_key_field() {
        let err = ConfigError::InvalidValue {
            key: "service.api_key".into(),
            expected_type: "string".into(),
            message: "missing".into(),
        };
        assert!(err.is_sensitive());
    }

    #[test]
    fn test_is_sensitive_credential_field() {
        let err = ConfigError::InvalidValue {
            key: "credential".into(),
            expected_type: "string".into(),
            message: "invalid".into(),
        };
        assert!(err.is_sensitive());
    }

    #[test]
    fn test_is_sensitive_secret_field() {
        let err = ConfigError::InvalidValue {
            key: "client_secret".into(),
            expected_type: "string".into(),
            message: "missing".into(),
        };
        assert!(err.is_sensitive());
    }

    #[test]
    fn test_is_sensitive_clean_error_returns_false() {
        let err = ConfigError::Timeout { duration_ms: 100 };
        assert!(!err.is_sensitive());

        let err = ConfigError::VersionMismatch {
            found: 1,
            expected: 2,
        };
        assert!(!err.is_sensitive());
    }

    // =============================================================================
    // sanitized_chain with source errors
    // =============================================================================

    #[test]
    fn test_sanitized_chain_parse_error_with_source() {
        let source: Box<dyn std::error::Error + Send + Sync> =
            Box::new(std::io::Error::other("inner cause"));
        let err = ConfigError::ParseError {
            format: "toml".into(),
            message: "outer".into(),
            location: None,
            source: Some(source),
        };
        let chain = err.sanitized_chain();
        assert_eq!(chain.len(), 2);
        // First entry is the user_message (sanitized)
        assert!(chain[0].contains("toml"));
        // Second entry is the sanitized source message
        assert!(chain[1].contains("inner cause"));
    }

    #[test]
    fn test_sanitized_chain_parse_error_no_source() {
        let err = ConfigError::ParseError {
            format: "toml".into(),
            message: "outer".into(),
            location: None,
            source: None,
        };
        let chain = err.sanitized_chain();
        assert_eq!(chain.len(), 1);
    }

    #[test]
    fn test_sanitized_chain_migration_failed_with_source() {
        let source: Box<dyn std::error::Error + Send + Sync> =
            Box::new(std::io::Error::other("migration cause"));
        let err = ConfigError::MigrationFailed {
            from: 1,
            to: 2,
            reason: "outer".into(),
            source: Some(source),
        };
        let chain = err.sanitized_chain();
        assert_eq!(chain.len(), 2);
        assert!(chain[1].contains("migration cause"));
    }

    #[test]
    fn test_sanitized_chain_other_variants_no_source() {
        // Variants without source only have a single entry
        let err = ConfigError::Timeout { duration_ms: 1 };
        let chain = err.sanitized_chain();
        assert_eq!(chain.len(), 1);

        let err = ConfigError::FileNotFound {
            filename: PathBuf::from("x"),
            source: None,
        };
        let chain = err.sanitized_chain();
        assert_eq!(chain.len(), 1);
    }

    // =============================================================================
    // MultiSourceError additional methods
    // =============================================================================

    #[test]
    fn test_multi_source_error_with_partial() {
        let partial = serde_json::json!({ "host": "localhost" });
        let err = MultiSourceError::with_partial(
            4,
            vec![(
                "source_a".to_string(),
                ConfigError::Timeout { duration_ms: 100 },
            )],
            partial,
        );
        assert_eq!(err.failed_count, 1);
        assert_eq!(err.total_count, 4);
        assert!(err.has_partial_success());
        let partial = err.partial_config().expect("partial config present");
        assert_eq!(partial["host"], "localhost");
    }

    #[test]
    fn test_multi_source_error_no_partial_success() {
        let err = MultiSourceError::new(2, vec![("s", ConfigError::Timeout { duration_ms: 1 })]);
        assert!(!err.has_partial_success());
        assert!(err.partial_config().is_none());
    }

    #[test]
    fn test_multi_source_error_errors_accessor() {
        let err = MultiSourceError::new(
            3,
            vec![
                ("s1".to_string(), ConfigError::Timeout { duration_ms: 1 }),
                (
                    "s2".to_string(),
                    ConfigError::RemoteUnavailable {
                        error_type: "conn".into(),
                        retryable: false,
                    },
                ),
            ],
        );
        let errors = err.errors();
        assert_eq!(errors.len(), 2);
        assert_eq!(errors[0].0, "s1");
        assert_eq!(errors[1].0, "s2");
    }

    #[test]
    fn test_multi_source_error_count_error_type() {
        let err = MultiSourceError::new(
            4,
            vec![
                ("s1".to_string(), ConfigError::Timeout { duration_ms: 1 }),
                ("s2".to_string(), ConfigError::Timeout { duration_ms: 2 }),
                (
                    "s3".to_string(),
                    ConfigError::RemoteUnavailable {
                        error_type: "conn".into(),
                        retryable: false,
                    },
                ),
            ],
        );
        assert_eq!(err.count_error_type(ErrorCode::Timeout), 2);
        assert_eq!(err.count_error_type(ErrorCode::RemoteUnavailable), 1);
        assert_eq!(err.count_error_type(ErrorCode::FileNotFound), 0);
    }

    // =============================================================================
    // BuildResult additional methods
    // =============================================================================

    #[test]
    fn test_build_result_with_warnings() {
        let warnings = vec![SourceWarning {
            message: "deprecated".into(),
            source: Some("file.toml".into()),
            code: WarningCode::DeprecatedKey,
        }];
        let result: BuildResult<i32> = BuildResult::with_warnings(42, warnings);
        assert!(!result.degraded);
        assert!(result.has_warnings());
        assert_eq!(result.warnings.len(), 1);
        assert_eq!(result.warnings[0].code, WarningCode::DeprecatedKey);
    }

    #[test]
    fn test_build_result_ok_has_no_warnings() {
        let result: BuildResult<i32> = BuildResult::ok(0);
        assert!(!result.has_warnings());
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn test_build_result_degraded_has_no_warnings() {
        let result: BuildResult<i32> = BuildResult::degraded(0, "reason");
        assert!(result.degraded);
        assert!(!result.has_warnings());
        assert_eq!(result.degraded_reason, Some("reason".into()));
    }

    #[test]
    fn test_build_result_map_preserves_metadata() {
        let warnings = vec![SourceWarning {
            message: "w".into(),
            source: None,
            code: WarningCode::DefaultUsed,
        }];
        let result: BuildResult<i32> = BuildResult::with_warnings(42, warnings);
        let mapped: BuildResult<String> = result.map(|n| format!("val={}", n));
        assert_eq!(mapped.config, "val=42");
        assert_eq!(mapped.warnings.len(), 1);
        assert!(!mapped.degraded);
    }

    // =============================================================================
    // WarningCode Display for all variants
    // =============================================================================

    #[test]
    fn test_warning_code_display_all_variants() {
        assert_eq!(
            WarningCode::OptionalSourceSkipped.to_string(),
            "OPTIONAL_SOURCE_SKIPPED"
        );
        assert_eq!(WarningCode::DeprecatedKey.to_string(), "DEPRECATED_KEY");
        assert_eq!(WarningCode::DefaultUsed.to_string(), "DEFAULT_USED");
        assert_eq!(WarningCode::ValueTruncated.to_string(), "VALUE_TRUNCATED");
        assert_eq!(WarningCode::RemoteFallback.to_string(), "REMOTE_FALLBACK");
        assert_eq!(
            WarningCode::UnencryptedSensitive.to_string(),
            "UNENCRYPTED_SENSITIVE"
        );
        assert_eq!(WarningCode::UnusedKey.to_string(), "UNUSED_KEY");
    }

    // =============================================================================
    // ConfersError / ConfersResult type aliases
    // =============================================================================

    #[test]
    fn test_confers_error_type_alias_matches_config_error() {
        let err: ConfersError = ConfigError::Timeout { duration_ms: 1 };
        // ConfersError is a type alias for ConfigError, so it should be usable
        assert_eq!(err.code(), ErrorCode::Timeout);
        // Verify it can be used where ConfigError is expected
        let _: &ConfigError = &err;
    }

    #[test]
    fn test_confers_result_type_alias() {
        // T-C-1 C1: old test asserted `Ok(42).is_ok()` which is tautological.
        // Now verify the type alias participates in error code mapping by
        // destructuring with match (avoids clippy unnecessary_literal_unwrap).
        let ok: ConfersResult<i32> = Ok(42);
        match ok {
            Ok(v) => assert_eq!(v, 42),
            Err(e) => panic!("expected Ok(42), got Err: {e:?}"),
        }

        let err: ConfersResult<i32> = Err(ConfigError::Timeout { duration_ms: 100 });
        match err {
            Ok(v) => panic!("expected Err, got Ok({v})"),
            Err(e) => assert_eq!(e.code(), ErrorCode::Timeout),
        }
    }

    #[test]
    fn test_config_result_type_alias() {
        // T-C-1 C2: old test asserted `Ok(0).is_ok()` which is tautological.
        // Now verify the type alias propagates error codes correctly.
        let ok: ConfigResult<i32> = Ok(0);
        match ok {
            Ok(v) => assert_eq!(v, 0),
            Err(e) => panic!("expected Ok(0), got Err: {e:?}"),
        }

        let err: ConfigResult<i32> = Err(ConfigError::FileNotFound {
            filename: PathBuf::from("missing"),
            source: None,
        });
        match err {
            Ok(v) => panic!("expected Err, got Ok({v})"),
            Err(e) => assert_eq!(e.code(), ErrorCode::FileNotFound),
        }
    }

    // =============================================================================
    // Display impl for ConfigError (exercises thiserror #[error(...)] formats)
    // =============================================================================

    #[test]
    fn test_config_error_display_file_not_found() {
        let err = ConfigError::FileNotFound {
            filename: PathBuf::from("config.toml"),
            source: None,
        };
        let s = format!("{}", err);
        assert!(s.contains("config.toml"));
        assert!(s.contains("not found"));
    }

    #[test]
    fn test_config_error_display_validation_failed() {
        let err = ConfigError::ValidationFailed {
            field: "port".into(),
            rule: "range".into(),
            message: "too big".into(),
        };
        let s = format!("{}", err);
        assert!(s.contains("port"));
        assert!(s.contains("range"));
        assert!(s.contains("too big"));
    }

    #[test]
    fn test_config_error_display_migration_failed() {
        let err = ConfigError::MigrationFailed {
            from: 1,
            to: 2,
            reason: "schema".into(),
            source: None,
        };
        let s = format!("{}", err);
        assert!(s.contains("v1"));
        assert!(s.contains("v2"));
        assert!(s.contains("schema"));
    }

    #[test]
    fn test_config_error_display_module_not_found() {
        let err = ConfigError::ModuleNotFound {
            group: "g".into(),
            module: "m".into(),
        };
        let s = format!("{}", err);
        assert!(s.contains("'m'"));
        assert!(s.contains("'g'"));
    }

    #[test]
    fn test_config_error_display_size_limit_exceeded() {
        let err = ConfigError::SizeLimitExceeded {
            actual: 100,
            limit: 50,
        };
        let s = format!("{}", err);
        assert!(s.contains("100"));
        assert!(s.contains("50"));
    }

    #[test]
    fn test_config_error_display_concurrency_conflict_with_expected_type() {
        let err = ConfigError::ConcurrencyConflict {
            key: "k".into(),
            message: "m".into(),
            expected_type: Some("string".into()),
        };
        let s = format!("{}", err);
        assert!(s.contains("k"));
        assert!(s.contains("m"));
    }

    #[test]
    fn test_config_error_display_key_rotation_failed() {
        let err = ConfigError::KeyRotationFailed {
            from_version: "v1".into(),
            to_version: "v2".into(),
            reason: "bad".into(),
        };
        let s = format!("{}", err);
        assert!(s.contains("v1"));
        assert!(s.contains("v2"));
        assert!(s.contains("bad"));
    }

    #[test]
    fn test_config_error_display_watcher_error() {
        let err = ConfigError::WatcherError {
            message: "lost".into(),
            path: Some(PathBuf::from("/x/y.toml")),
            recoverable: false,
        };
        let s = format!("{}", err);
        assert!(s.contains("lost"));
    }

    #[test]
    fn test_config_error_display_override_blocked() {
        let err = ConfigError::OverrideBlocked {
            key: "k".into(),
            reason: "r".into(),
            override_source: Some("cli".into()),
        };
        let s = format!("{}", err);
        assert!(s.contains("k"));
        assert!(s.contains("r"));
    }

    #[test]
    fn test_config_error_display_health_check_failed() {
        let err = ConfigError::HealthCheckFailed {
            reason: "down".into(),
        };
        let s = format!("{}", err);
        assert!(s.contains("down"));
    }

    #[test]
    fn test_config_error_display_multi_source() {
        let inner = MultiSourceError::new(2, vec![("s", ConfigError::Timeout { duration_ms: 1 })]);
        let err = ConfigError::MultiSource { source: inner };
        let s = format!("{}", err);
        assert!(s.contains("Multiple sources failed"));
    }

    #[test]
    fn test_config_error_display_lock_poisoned() {
        let err = ConfigError::LockPoisoned {
            resource: "config".into(),
        };
        let s = format!("{}", err);
        assert!(s.contains("config"));
        assert!(s.contains("Lock poisoned"));
    }

    // =============================================================================
    // Debug derive for ConfigError and MultiSourceError
    // =============================================================================

    #[test]
    fn test_config_error_debug_format() {
        let err = ConfigError::Timeout { duration_ms: 42 };
        let dbg = format!("{:?}", err);
        assert!(dbg.contains("Timeout"));
        assert!(dbg.contains("42"));
    }

    #[test]
    fn test_multi_source_error_debug_format() {
        let err = MultiSourceError::new(2, vec![("s", ConfigError::Timeout { duration_ms: 1 })]);
        let dbg = format!("{:?}", err);
        assert!(dbg.contains("MultiSourceError"));
        assert!(dbg.contains("failed_count"));
    }

    // =============================================================================
    // sanitize_error_message: AWS secret access key (40-char)
    // =============================================================================

    #[test]
    fn test_sanitize_error_message_aws_secret_key() {
        // Exactly 40 chars of [A-Za-z0-9/+=] surrounded by spaces
        let msg = " secret: abcdefghijklmnopqrstuvwxyz0123456789ABCD "; // pragma: allowlist secret
        let sanitized = sanitize_error_message(msg);
        assert!(
            sanitized.contains("<aws_secret_key>"),
            "expected AWS secret key to be redacted, got: {}",
            sanitized
        );
    }

    // =============================================================================
    // audit_message for additional variants
    // =============================================================================

    #[test]
    fn test_audit_message_timeout() {
        let err = ConfigError::Timeout { duration_ms: 100 };
        let audit = err.audit_message();
        assert!(audit.contains("error_code=900"));
        assert!(audit.contains("TIMEOUT"));
        assert!(audit.contains("operation=config"));
    }

    #[test]
    fn test_audit_message_concurrency_conflict() {
        let err = ConfigError::ConcurrencyConflict {
            key: "k".into(),
            message: "m".into(),
            expected_type: None,
        };
        let audit = err.audit_message();
        assert!(audit.contains("error_code=901"));
        assert!(audit.contains("CONCURRENCY_CONFLICT"));
    }
}
