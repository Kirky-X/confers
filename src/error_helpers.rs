// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

use crate::error::ConfigError;

/// Helper trait for converting errors to ConfigError with context
pub trait ResultExt<T, E>: Sized {
    /// Map error with context message
    fn with_context(self, context: &str) -> Result<T, ConfigError>;

    /// Map error for file operations
    fn with_file_context(self, operation: &str, path: &std::path::Path) -> Result<T, ConfigError>;

    /// Map error for remote operations
    fn with_remote_context(self, operation: &str) -> Result<T, ConfigError>;

    /// Map error for serialization operations
    fn with_serialization_context(self, operation: &str) -> Result<T, ConfigError>;

    /// Map error for parsing operations
    fn with_parse_context(self, operation: &str) -> Result<T, ConfigError>;
}

impl<T, E: std::fmt::Display> ResultExt<T, E> for Result<T, E> {
    fn with_context(self, context: &str) -> Result<T, ConfigError> {
        self.map_err(|e| ConfigError::RuntimeError(format!("{}: {}", context, e)))
    }

    fn with_file_context(self, operation: &str, path: &std::path::Path) -> Result<T, ConfigError> {
        self.map_err(|e| {
            ConfigError::IoError(format!(
                "{} failed for {}: {}",
                operation,
                path.display(),
                e
            ))
        })
    }

    fn with_remote_context(self, operation: &str) -> Result<T, ConfigError> {
        self.map_err(|e| ConfigError::RemoteError(format!("{}: {}", operation, e)))
    }

    fn with_serialization_context(self, operation: &str) -> Result<T, ConfigError> {
        self.map_err(|e| ConfigError::SerializationError(format!("{}: {}", operation, e)))
    }

    fn with_parse_context(self, operation: &str) -> Result<T, ConfigError> {
        self.map_err(|e| ConfigError::ParseError(format!("{}: {}", operation, e)))
    }
}

/// Helper trait for Option types
pub trait OptionExt<T>: Sized {
    /// Convert Option to Result with context
    fn ok_or_else_with<F>(self, f: F) -> Result<T, ConfigError>
    where
        F: FnOnce() -> String;

    /// Convert Option to Result with ConfigError
    fn ok_or_missing(self, item: &str) -> Result<T, ConfigError>;
}

impl<T> OptionExt<T> for Option<T> {
    fn ok_or_else_with<F>(self, f: F) -> Result<T, ConfigError>
    where
        F: FnOnce() -> String,
    {
        self.ok_or_else(|| ConfigError::RuntimeError(f()))
    }

    fn ok_or_missing(self, item: &str) -> Result<T, ConfigError> {
        self.ok_or_else(|| ConfigError::RuntimeError(format!("Missing required item: {}", item)))
    }
}

/// Helper functions for common error conversions
pub mod helpers {
    use super::*;

    /// Convert std::io::Error to ConfigError::IoError
    pub fn io_error(e: std::io::Error, context: &str) -> ConfigError {
        ConfigError::IoError(format!("{}: {}", context, e))
    }

    /// Convert serde error to ConfigError
    pub fn serde_error<E: std::fmt::Display>(e: E, context: &str) -> ConfigError {
        ConfigError::SerializationError(format!("{}: {}", context, e))
    }

    /// Convert parse error to ConfigError
    pub fn parse_error<E: std::fmt::Display>(e: E, context: &str) -> ConfigError {
        ConfigError::ParseError(format!("{}: {}", context, e))
    }

    /// Convert validation error to ConfigError
    pub fn validation_error<E: std::fmt::Display>(e: E, context: &str) -> ConfigError {
        ConfigError::ValidationError(format!("{}: {}", context, e))
    }

    /// Convert remote error to ConfigError
    pub fn remote_error<E: std::fmt::Display>(e: E, context: &str) -> ConfigError {
        ConfigError::RemoteError(format!("{}: {}", context, e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_result_ext_with_context() {
        let result: Result<(), &str> = Err("test error");
        let converted = result.with_context("operation failed");
        assert!(matches!(converted, Err(ConfigError::RuntimeError(_))));
    }

    #[test]
    fn test_result_ext_with_file_context() {
        let result: Result<(), &str> = Err("io error");
        let path = std::path::Path::new("/tmp/test.txt");
        let converted = result.with_file_context("read", path);
        assert!(matches!(converted, Err(ConfigError::IoError(_))));
    }

    #[test]
    fn test_option_ext_ok_or_else_with() {
        let option: Option<i32> = None;
        let result = option.ok_or_else_with(|| "value missing".to_string());
        assert!(matches!(result, Err(ConfigError::RuntimeError(_))));
    }

    #[test]
    fn test_option_ext_ok_or_missing() {
        let option: Option<i32> = None;
        let result = option.ok_or_missing("config value");
        assert!(matches!(result, Err(ConfigError::RuntimeError(_))));
    }
}
