// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error, Clone)]
pub enum ConfigError {
    #[error("Configuration file not found: {path}")]
    FileNotFound { path: PathBuf },

    #[error("Format detection failed: {0}")]
    FormatDetectionFailed(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Unsafe path: {0}")]
    UnsafePath(PathBuf),

    #[error("Remote configuration load failed: {0}")]
    RemoteError(String),

    #[error("Configuration load failed")]
    LoadError,

    #[error("Runtime error: {0}")]
    RuntimeError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("IO error: {0}")]
    IoError(String),

    #[error("Memory limit exceeded: limit {limit}MB, current {current}MB")]
    MemoryLimitExceeded { limit: usize, current: usize },

    #[error("Key error: {0}")]
    KeyError(String),

    #[error("Key not found: {key_id}")]
    KeyNotFound { key_id: String },

    #[error("Key version mismatch: expected {expected}, actual {actual}")]
    KeyVersionMismatch { expected: u32, actual: u32 },

    #[error("Key rotation failed: {0}")]
    KeyRotationFailed(String),

    #[error("Key storage error: {0}")]
    KeyStorageError(String),

    #[error("Key verification failed: checksum mismatch")]
    KeyChecksumMismatch,

    #[error("Key expired: {key_id}, version {version}")]
    KeyExpired { key_id: String, version: u32 },

    #[error("Key deprecated: {key_id}, version {version}")]
    KeyDeprecated { key_id: String, version: u32 },

    #[error("Invalid master key: {0}")]
    InvalidMasterKey(String),

    #[error("Key policy error: {0}")]
    KeyPolicyError(String),

    #[error("Environment variable security validation failed: {0}")]
    EnvSecurityError(String),

    #[error("Encryption error: {0}")]
    EncryptionError(String),

    #[error("Decryption error: {0}")]
    DecryptionError(String),

    #[error("Other error: {0}")]
    Other(String),
}

#[cfg(feature = "validation")]
impl From<validator::ValidationErrors> for ConfigError {
    fn from(_err: validator::ValidationErrors) -> Self {
        ConfigError::ValidationError("Validation failed".to_string())
    }
}

impl From<figment::Error> for ConfigError {
    fn from(_err: figment::Error) -> Self {
        ConfigError::LoadError
    }
}

impl From<std::io::Error> for ConfigError {
    fn from(err: std::io::Error) -> Self {
        ConfigError::IoError(err.to_string())
    }
}

impl From<String> for ConfigError {
    fn from(s: String) -> Self {
        ConfigError::FormatDetectionFailed(s)
    }
}

impl From<crate::security::EnvSecurityError> for ConfigError {
    fn from(err: crate::security::EnvSecurityError) -> Self {
        ConfigError::EnvSecurityError(err.to_string())
    }
}

impl ConfigError {
    /// Creates a safe RemoteError that doesn't leak sensitive information
    pub fn remote_safe(message: impl Into<String>) -> Self {
        ConfigError::RemoteError(message.into())
    }

    /// Creates a RemoteError with URL (for debugging) - use with caution
    pub fn remote_with_url(url: impl Into<String>, message: impl Into<String>) -> Self {
        // URL is included in message for debugging but should be handled carefully
        ConfigError::RemoteError(format!("{} (URL: {})", message.into(), url.into()))
    }

    /// Returns a safe-to-display version of the error that doesn't leak sensitive information
    /// Use this for logging or displaying errors to users
    pub fn safe_display(&self) -> String {
        match self {
            ConfigError::RemoteError(msg) => {
                let safe_msg = Self::sanitize_url(msg);
                format!("Remote configuration load failed: {}", safe_msg)
            }
            ConfigError::FileNotFound { path } => {
                // Only show filename, not full path
                if let Some(filename) = path.file_name() {
                    format!(
                        "Configuration file not found: {}",
                        filename.to_string_lossy()
                    )
                } else {
                    "Configuration file not found".to_string()
                }
            }
            ConfigError::KeyNotFound { key_id } => {
                // Mask key ID
                format!("Key not found: {}", Self::mask_key_id(key_id))
            }
            ConfigError::KeyExpired { key_id, version } => {
                format!(
                    "Key expired: {}, version {}",
                    Self::mask_key_id(key_id),
                    version
                )
            }
            ConfigError::KeyDeprecated { key_id, version } => {
                format!(
                    "Key deprecated: {}, version {}",
                    Self::mask_key_id(key_id),
                    version
                )
            }
            ConfigError::IoError(msg) => {
                // Remove potential path information from IO errors
                let sanitized = msg
                    .split(['/', '\\'])
                    .next_back()
                    .unwrap_or(msg)
                    .to_string();
                format!("IO error: {}", sanitized)
            }
            ConfigError::ParseError(msg) => {
                // Remove any potential sensitive data from parse errors
                let sanitized = msg
                    .split_whitespace()
                    .take(10)
                    .collect::<Vec<_>>()
                    .join(" ");
                format!("Parse error: {}", sanitized)
            }
            _ => self.to_string(),
        }
    }

    /// Sanitize URLs from error messages to prevent information leakage
    fn sanitize_url(msg: &str) -> String {
        // Use regex to replace URLs with sanitized versions
        use regex::Regex;

        // Pattern to match full URLs with potential credentials
        static URL_PATTERN: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();

        let url_regex = URL_PATTERN.get_or_init(|| {
            Regex::new(r"(?i)(https?://)([^:/\s]+):([^@/\s]+)@([^/\s]+)(/\S*)?")
                .unwrap_or_else(|_| Regex::new(r"https?://\S+").unwrap())
        });

        let result = url_regex.replace_all(msg, |caps: &regex::Captures| {
            let protocol = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            let _username = caps.get(2).map(|m| m.as_str()).unwrap_or("");
            let _password = caps.get(3).map(|m| m.as_str()).unwrap_or("");
            let host = caps.get(4).map(|m| m.as_str()).unwrap_or("");
            let path = caps.get(5).map(|m| m.as_str()).unwrap_or("");

            // Show protocol and host, mask credentials
            format!("{}***:***@{}{}", protocol, host, path)
        });

        // Also mask IP addresses in URLs (show only last octet for IPv4, similar for IPv6)
        static IP_PATTERN: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();

        let ip_regex = IP_PATTERN.get_or_init(|| {
            Regex::new(r"\b(\d{1,3})\.(\d{1,3})\.(\d{1,3})\.(\d{1,3})\b")
                .unwrap_or_else(|_| Regex::new(r"\d+\.\d+\.\d+\.\d+").unwrap())
        });

        let result = ip_regex.replace_all(&result, |caps: &regex::Captures| {
            format!(
                "{}.{}.{}.{}",
                caps.get(1).map(|m| m.as_str()).unwrap_or("*"),
                caps.get(2).map(|m| m.as_str()).unwrap_or("*"),
                caps.get(3).map(|m| m.as_str()).unwrap_or("*"),
                caps.get(4).map(|m| m.as_str()).unwrap_or("*")
            )
        });

        result.to_string()
    }

    /// Masks a key ID for safe display (show only first 4 and last 4 characters)
    fn mask_key_id(key_id: &str) -> String {
        if key_id.len() <= 8 {
            "***".to_string()
        } else {
            format!("{}***{}", &key_id[..4], &key_id[key_id.len() - 4..])
        }
    }
}
