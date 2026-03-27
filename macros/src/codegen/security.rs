//! Security utilities for safe file path handling and sensitive data protection.
//!
//! This module provides path validation to prevent directory traversal attacks
//! and secure handling of sensitive configuration values.

use std::collections::HashSet;
use std::path::{Component, PathBuf};

/// Default allowed base directories for secret files.
#[allow(dead_code)]
pub const ALLOWED_SECRET_BASE_DIRS: &[&str] = &["/run/secrets", "/var/secrets"];

/// Maximum allowed path length to prevent DoS attacks.
#[allow(dead_code)]
const MAX_PATH_LENGTH: usize = 4096;

/// Path validator for secure file access.
///
/// This validator ensures that file paths used for loading secrets
/// are safe and cannot be used for directory traversal attacks.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct PathValidator {
    allowed_base_dirs: HashSet<PathBuf>,
    max_path_length: usize,
}

impl PathValidator {
    /// Create a new path validator with default settings.
    #[allow(dead_code)]
    pub fn new() -> Self {
        let allowed_base_dirs = ALLOWED_SECRET_BASE_DIRS.iter().map(PathBuf::from).collect();

        Self {
            allowed_base_dirs,
            max_path_length: MAX_PATH_LENGTH,
        }
    }

    /// Create a path validator with custom allowed directories.
    #[allow(dead_code)]
    pub fn with_allowed_dirs(allowed_dirs: Vec<PathBuf>) -> Self {
        Self {
            allowed_base_dirs: allowed_dirs.into_iter().collect(),
            max_path_length: MAX_PATH_LENGTH,
        }
    }

    /// Validate and resolve a file path.
    ///
    /// This method performs comprehensive security checks:
    /// 1. Length validation to prevent DoS
    /// 2. Absolute path rejection
    /// 3. Directory traversal detection (including encoded variants)
    /// 4. Path canonicalization
    /// 5. Verification that the final path is within allowed directories
    ///
    /// # Arguments
    ///
    /// * `file_path` - The file path to validate
    ///
    /// # Returns
    ///
    /// Returns the canonicalized path if validation succeeds, or an error if it fails.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let validator = PathValidator::new();
    /// let result = validator.validate_and_resolve("secrets/api_key.txt");
    /// ```
    #[allow(dead_code)]
    pub fn validate_and_resolve(&self, file_path: &str) -> Result<PathBuf, PathValidationError> {
        if file_path.len() > self.max_path_length {
            return Err(PathValidationError::TooLong);
        }

        let path = PathBuf::from(file_path);

        if path.is_absolute() {
            return Err(PathValidationError::AbsolutePath);
        }

        for component in path.components() {
            match component {
                Component::ParentDir => {
                    return Err(PathValidationError::ParentDirectoryReference);
                }
                Component::Prefix(_) => {
                    return Err(PathValidationError::InvalidComponent);
                }
                Component::RootDir => {
                    return Err(PathValidationError::AbsolutePath);
                }
                Component::CurDir => continue,
                Component::Normal(_) => continue,
            }
        }

        if contains_encoded_traversal(file_path) {
            return Err(PathValidationError::EncodedTraversal);
        }

        let current_dir =
            std::env::current_dir().map_err(|_| PathValidationError::CurrentDirUnavailable)?;
        let full_path = current_dir.join(&path);

        let canonical_path = full_path
            .canonicalize()
            .map_err(|_| PathValidationError::NotFound)?;

        let is_allowed = self
            .allowed_base_dirs
            .iter()
            .any(|base| canonical_path.starts_with(base))
            || canonical_path.starts_with(&current_dir);

        if !is_allowed {
            return Err(PathValidationError::OutsideAllowedDirectory);
        }

        Ok(canonical_path)
    }
}

impl Default for PathValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Check if a string contains URL-encoded directory traversal patterns.
#[allow(dead_code)]
fn contains_encoded_traversal(s: &str) -> bool {
    let lower = s.to_lowercase();
    lower.contains("%2e")
        || lower.contains("%252e")
        || lower.contains("%5c")
        || lower.contains("%255c")
}

/// Errors that can occur during path validation.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathValidationError {
    /// The path exceeds the maximum allowed length.
    TooLong,
    /// Absolute paths are not allowed.
    AbsolutePath,
    /// Parent directory references (..) are not allowed.
    ParentDirectoryReference,
    /// Invalid path component detected.
    InvalidComponent,
    /// URL-encoded directory traversal detected.
    EncodedTraversal,
    /// The specified path does not exist.
    NotFound,
    /// Cannot determine the current directory.
    CurrentDirUnavailable,
    /// The path is outside the allowed directories.
    OutsideAllowedDirectory,
}

impl std::fmt::Display for PathValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooLong => write!(
                f,
                "Path exceeds maximum length of {} bytes",
                MAX_PATH_LENGTH
            ),
            Self::AbsolutePath => write!(f, "Absolute paths are not allowed for security reasons"),
            Self::ParentDirectoryReference => {
                write!(f, "Parent directory references (..) are not allowed")
            }
            Self::InvalidComponent => write!(f, "Invalid path component detected"),
            Self::EncodedTraversal => {
                write!(f, "URL-encoded directory traversal pattern detected")
            }
            Self::NotFound => write!(f, "The specified path does not exist"),
            Self::CurrentDirUnavailable => write!(f, "Cannot determine the current directory"),
            Self::OutsideAllowedDirectory => {
                write!(f, "Path is outside the allowed directories")
            }
        }
    }
}

impl std::error::Error for PathValidationError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rejects_absolute_path() {
        let validator = PathValidator::new();
        let result = validator.validate_and_resolve("/etc/passwd");
        assert_eq!(result.err(), Some(PathValidationError::AbsolutePath));
    }

    #[test]
    fn test_rejects_parent_directory() {
        let validator = PathValidator::new();
        let result = validator.validate_and_resolve("../../../etc/passwd");
        assert_eq!(
            result.err(),
            Some(PathValidationError::ParentDirectoryReference)
        );
    }

    #[test]
    fn test_rejects_encoded_traversal() {
        let validator = PathValidator::new();

        let result = validator.validate_and_resolve("%2e%2e/etc/passwd");
        assert_eq!(result.err(), Some(PathValidationError::EncodedTraversal));

        let result = validator.validate_and_resolve("%252e%252e/etc/passwd");
        assert_eq!(result.err(), Some(PathValidationError::EncodedTraversal));
    }

    #[test]
    fn test_rejects_too_long_path() {
        let validator = PathValidator::new();
        let long_path = "a".repeat(5000);
        let result = validator.validate_and_resolve(&long_path);
        assert_eq!(result.err(), Some(PathValidationError::TooLong));
    }

    #[test]
    fn test_path_validator_default() {
        let validator = PathValidator::default();
        assert!(!validator.allowed_base_dirs.is_empty());
        assert_eq!(validator.max_path_length, MAX_PATH_LENGTH);
    }
}
