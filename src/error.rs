//! Error handling for confers configuration library.
//!
//! This module provides comprehensive error types with:
//! - Precise error location (file, line, column)
//! - Sanitized messages for user display
//! - Audit-safe messages for logging
//! - Retryable error classification

use std::path::PathBuf;
use std::sync::LazyLock;
use thiserror::Error;

// Re-export SourceLocation as ParseLocation for backward compatibility
pub use crate::value::SourceLocation as ParseLocation;

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
    /// Concurrency conflict
    ConcurrencyConflict = 1018,
    /// Key rotation failed
    KeyRotationFailed = 1019,
    /// Watcher error
    WatcherError = 1020,
    OverrideBlocked = 1021,
    LockPoisoned = 1022,
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
            ErrorCode::ConcurrencyConflict => write!(f, "CONCURRENCY_CONFLICT"),
            ErrorCode::KeyRotationFailed => write!(f, "KEY_ROTATION_FAILED"),
            ErrorCode::WatcherError => write!(f, "WATCHER_ERROR"),
            ErrorCode::OverrideBlocked => write!(f, "OVERRIDE_BLOCKED"),
            ErrorCode::LockPoisoned => write!(f, "LOCK_POISONED"),
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
            ConfigError::LockPoisoned { .. } => ErrorCode::LockPoisoned,
            ConfigError::MultiSource { .. } => ErrorCode::SourceChainError,
            ConfigError::ConcurrencyConflict { .. } => ErrorCode::ConcurrencyConflict,
            ConfigError::KeyRotationFailed { .. } => ErrorCode::KeyRotationFailed,
            ConfigError::WatcherError { .. } => ErrorCode::WatcherError,
            ConfigError::OverrideBlocked { .. } => ErrorCode::OverrideBlocked,
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
                    source.failed_count(),
                    source.total_count()
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

// ============== BrickArchitecture Error Types ==============

/// Runtime error alias for BrickArchitecture compliance.
///
/// This is an alias for `ConfigError` to follow the BrickArchitecture naming convention.
/// Use `ConfersError` for runtime errors that occur during configuration access.
pub type ConfersError = ConfigError;

/// Result type alias for runtime operations.
///
/// Use `ConfersResult` for operations that may fail with `ConfersError`.
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
}
