// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Configuration limits for safety and resource management.

use crate::impl_::loader::Format;

/// Configuration size and resource limits.
#[derive(Debug, Clone)]
pub struct ConfigLimits {
    /// Maximum configuration file size in bytes.
    pub max_file_size_bytes: u64,
    /// Maximum total configuration size in bytes.
    pub max_total_size: u64,
    /// Maximum depth of nested configuration.
    pub max_nesting_depth: usize,
    /// Maximum number of keys.
    pub max_total_fields: usize,
    /// Maximum array length.
    pub max_array_length: usize,
    /// Maximum string length.
    pub max_string_length: usize,
    /// Allowed file extensions.
    pub allowed_extensions: Vec<String>,
    /// Whether to allow remote sources.
    pub allow_remote: bool,
    /// Maximum number of sources.
    pub max_sources: usize,
}

impl Default for ConfigLimits {
    fn default() -> Self {
        Self {
            max_file_size_bytes: 10 * 1024 * 1024, // 10 MB
            max_total_size: 100 * 1024 * 1024,     // 100 MB
            max_nesting_depth: 20,
            max_total_fields: 10_000,
            max_array_length: 10_000,
            max_string_length: 1024 * 1024, // 1 MB
            allowed_extensions: Format::all().iter().map(|f| f.ext().to_string()).collect(),
            allow_remote: false, // Secure by default
            max_sources: 50,
        }
    }
}

impl ConfigLimits {
    /// Set the maximum file size.
    pub fn with_max_file_size_bytes(mut self, bytes: u64) -> Self {
        self.max_file_size_bytes = bytes;
        self
    }

    /// Set the maximum total size.
    pub fn with_max_total_size(mut self, bytes: u64) -> Self {
        self.max_total_size = bytes;
        self
    }

    /// Set the maximum nesting depth.
    pub fn with_max_nesting_depth(mut self, depth: usize) -> Self {
        self.max_nesting_depth = depth;
        self
    }

    /// Set the maximum number of keys.
    pub fn with_max_total_fields(mut self, count: usize) -> Self {
        self.max_total_fields = count;
        self
    }

    /// Set the maximum array length.
    pub fn with_max_array_length(mut self, length: usize) -> Self {
        self.max_array_length = length;
        self
    }

    /// Set the maximum string length.
    pub fn with_max_string_length(mut self, length: usize) -> Self {
        self.max_string_length = length;
        self
    }

    /// Set allowed file extensions.
    pub fn with_allowed_extensions(mut self, extensions: Vec<String>) -> Self {
        self.allowed_extensions = extensions;
        self
    }

    /// Set whether to allow remote sources.
    pub fn with_allow_remote(mut self, allow: bool) -> Self {
        self.allow_remote = allow;
        self
    }

    /// Set the maximum number of sources.
    pub fn with_max_sources(mut self, count: usize) -> Self {
        self.max_sources = count;
        self
    }

    /// Check if a file extension is allowed.
    pub fn is_extension_allowed(&self, path: &std::path::Path) -> bool {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| {
                self.allowed_extensions
                    .iter()
                    .any(|allowed| allowed.eq_ignore_ascii_case(ext))
            })
            .unwrap_or(false)
    }

    /// Check if a file size is within limits.
    pub fn is_file_size_ok(&self, size: u64) -> bool {
        size <= self.max_file_size_bytes
    }

    /// Check if total size is within limits.
    pub fn is_total_size_ok(&self, size: u64) -> bool {
        size <= self.max_total_size
    }

    /// Create a strict limits configuration.
    pub fn strict() -> Self {
        Self {
            max_file_size_bytes: 1024 * 1024, // 1 MB
            max_total_size: 10 * 1024 * 1024, // 10 MB
            max_nesting_depth: 10,
            max_total_fields: 1_000,
            max_array_length: 1_000,
            max_string_length: 100 * 1024, // 100 KB
            allowed_extensions: vec!["toml".to_string(), "json".to_string()],
            allow_remote: false,
            max_sources: 10,
        }
    }

    /// Create a permissive limits configuration.
    pub fn permissive() -> Self {
        Self {
            max_file_size_bytes: 100 * 1024 * 1024, // 100 MB
            max_total_size: 1024 * 1024 * 1024,     // 1 GB
            max_nesting_depth: 50,
            max_total_fields: 100_000,
            max_array_length: 100_000,
            max_string_length: 10 * 1024 * 1024, // 10 MB
            allowed_extensions: Format::all().iter().map(|f| f.ext().to_string()).collect(),
            allow_remote: true,
            max_sources: 100,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_default_limits() {
        let limits = ConfigLimits::default();
        assert_eq!(limits.max_file_size_bytes, 10 * 1024 * 1024);
        assert_eq!(limits.max_nesting_depth, 20);
        assert_eq!(limits.max_total_fields, 10_000);
        assert!(!limits.allow_remote); // Secure by default
    }

    #[test]
    fn test_builder_pattern() {
        let limits = ConfigLimits::default()
            .with_max_file_size_bytes(1024)
            .with_max_nesting_depth(5)
            .with_allow_remote(false);

        assert_eq!(limits.max_file_size_bytes, 1024);
        assert_eq!(limits.max_nesting_depth, 5);
        assert!(!limits.allow_remote);
    }

    #[test]
    fn test_extension_check() {
        let limits = ConfigLimits::default();

        assert!(limits.is_extension_allowed(PathBuf::from("config.toml").as_path()));
        assert!(limits.is_extension_allowed(PathBuf::from("config.json").as_path()));
        assert!(limits.is_extension_allowed(PathBuf::from("config.yaml").as_path()));
        assert!(!limits.is_extension_allowed(PathBuf::from("config.xml").as_path()));
    }

    #[test]
    fn test_size_checks() {
        let limits = ConfigLimits::default()
            .with_max_file_size_bytes(1000)
            .with_max_total_size(5000);

        assert!(limits.is_file_size_ok(500));
        assert!(limits.is_file_size_ok(1000));
        assert!(!limits.is_file_size_ok(1001));

        assert!(limits.is_total_size_ok(4000));
        assert!(!limits.is_total_size_ok(6000));
    }

    #[test]
    fn test_strict_limits() {
        let limits = ConfigLimits::strict();
        assert!(!limits.allow_remote);
        assert_eq!(limits.max_file_size_bytes, 1024 * 1024);
    }

    #[test]
    fn test_permissive_limits() {
        let limits = ConfigLimits::permissive();
        assert!(limits.allow_remote);
        assert_eq!(limits.max_file_size_bytes, 100 * 1024 * 1024);
    }

    #[test]
    fn test_limits_string_length() {
        let l = ConfigLimits::default().with_max_string_length(500);
        assert_eq!(l.max_string_length, 500);
    }

    #[test]
    fn test_limits_array_length() {
        let l = ConfigLimits::default().with_max_array_length(50);
        assert_eq!(l.max_array_length, 50);
    }
}
