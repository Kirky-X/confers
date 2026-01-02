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

    #[error("Other error: {0}")]
    Other(String),
}

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
