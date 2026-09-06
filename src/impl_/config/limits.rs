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
        // Clamp: file size must not exceed total size.
        if self.max_file_size_bytes > self.max_total_size {
            self.max_file_size_bytes = self.max_total_size;
        }
        self
    }

    /// Set the maximum total size.
    pub fn with_max_total_size(mut self, bytes: u64) -> Self {
        self.max_total_size = bytes;
        // Clamp: file size must not exceed total size.
        if self.max_file_size_bytes > self.max_total_size {
            self.max_file_size_bytes = self.max_total_size;
        }
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

    /// Validate a merged configuration tree against these limits.
    ///
    /// Walks the merged [`AnnotatedValue`] and enforces the structural limits:
    /// `max_nesting_depth`, `max_total_fields`, `max_array_length` and
    /// `max_string_length`. The first violation is returned as a
    /// [`ConfigError::InvalidValue`] describing the offending path.
    ///
    /// [`AnnotatedValue`]: crate::types::AnnotatedValue
    /// [`ConfigError::InvalidValue`]: crate::error::ConfigError::InvalidValue
    pub fn validate_value(
        &self,
        value: &crate::types::AnnotatedValue,
    ) -> crate::error::ConfigResult<()> {
        let fields = std::cell::Cell::new(0usize);
        self.walk(value, 0, value.path.as_ref(), &fields)
    }

    fn walk(
        &self,
        value: &crate::types::AnnotatedValue,
        depth: usize,
        path: &str,
        fields: &std::cell::Cell<usize>,
    ) -> crate::error::ConfigResult<()> {
        use crate::types::ConfigValue;

        if depth > self.max_nesting_depth {
            return Err(limit_violation(
                path,
                "nesting depth",
                depth,
                self.max_nesting_depth,
            ));
        }

        match &value.inner {
            ConfigValue::Map(map) => {
                let total = fields.get() + map.len();
                if total > self.max_total_fields {
                    return Err(limit_violation(
                        path,
                        "total field count",
                        total,
                        self.max_total_fields,
                    ));
                }
                fields.set(total);
                for (key, child) in map.iter() {
                    let child_path = if path.is_empty() {
                        key.to_string()
                    } else {
                        format!("{path}.{key}")
                    };
                    self.walk(child, depth + 1, &child_path, fields)?;
                }
            }
            ConfigValue::Array(items) => {
                if items.len() > self.max_array_length {
                    return Err(limit_violation(
                        path,
                        "array length",
                        items.len(),
                        self.max_array_length,
                    ));
                }
                for (index, child) in items.iter().enumerate() {
                    let child_path = format!("{path}[{index}]");
                    self.walk(child, depth + 1, &child_path, fields)?;
                }
            }
            ConfigValue::String(s) if s.len() > self.max_string_length => {
                return Err(limit_violation(
                    path,
                    "string length",
                    s.len(),
                    self.max_string_length,
                ));
            }
            _ => {}
        }

        Ok(())
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

/// Build a descriptive [`ConfigError::InvalidValue`] for a limit violation.
fn limit_violation(
    path: &str,
    kind: &str,
    actual: usize,
    limit: usize,
) -> crate::error::ConfigError {
    let path = if path.is_empty() { "<root>" } else { path };
    crate::error::ConfigError::InvalidValue {
        key: path.to_string(),
        expected_type: format!("{kind} within limit"),
        message: format!("{kind} {actual} exceeds configured limit {limit}"),
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

    fn leaf(value: crate::types::ConfigValue, path: &str) -> crate::types::AnnotatedValue {
        crate::types::AnnotatedValue::new(value, crate::types::SourceId::new("test"), path)
    }

    fn annotated_map(
        entries: Vec<(&str, crate::types::AnnotatedValue)>,
    ) -> crate::types::AnnotatedValue {
        leaf(crate::types::ConfigValue::map(entries), "")
    }

    #[test]
    fn test_validate_value_accepts_within_limits() {
        let value = annotated_map(vec![
            (
                "name",
                leaf(crate::types::ConfigValue::string("app"), "name"),
            ),
            (
                "tags",
                leaf(
                    crate::types::ConfigValue::array(vec![leaf(
                        crate::types::ConfigValue::string("a"),
                        "tags[0]",
                    )]),
                    "tags",
                ),
            ),
        ]);
        assert!(ConfigLimits::default().validate_value(&value).is_ok());
    }

    #[test]
    fn test_validate_value_rejects_deep_nesting() {
        let limits = ConfigLimits::default().with_max_nesting_depth(2);
        let deep = annotated_map(vec![(
            "a",
            leaf(
                crate::types::ConfigValue::map(vec![(
                    "b",
                    leaf(
                        crate::types::ConfigValue::map(vec![(
                            "c",
                            leaf(crate::types::ConfigValue::string("too-deep"), "a.b.c"),
                        )]),
                        "a.b",
                    ),
                )]),
                "a",
            ),
        )]);
        let err = limits.validate_value(&deep).unwrap_err();
        assert!(err.to_string().contains("nesting depth"), "{err}");
    }

    #[test]
    fn test_validate_value_rejects_total_fields() {
        let limits = ConfigLimits::default().with_max_total_fields(2);
        let value = annotated_map(vec![
            ("a", leaf(crate::types::ConfigValue::integer(1), "a")),
            ("b", leaf(crate::types::ConfigValue::integer(2), "b")),
            ("c", leaf(crate::types::ConfigValue::integer(3), "c")),
        ]);
        let err = limits.validate_value(&value).unwrap_err();
        assert!(err.to_string().contains("total field count"), "{err}");
    }

    #[test]
    fn test_validate_value_rejects_array_length() {
        let limits = ConfigLimits::default().with_max_array_length(1);
        let value = annotated_map(vec![(
            "items",
            leaf(
                crate::types::ConfigValue::array(vec![
                    leaf(crate::types::ConfigValue::integer(1), "items[0]"),
                    leaf(crate::types::ConfigValue::integer(2), "items[1]"),
                ]),
                "items",
            ),
        )]);
        let err = limits.validate_value(&value).unwrap_err();
        assert!(err.to_string().contains("array length"), "{err}");
    }

    #[test]
    fn test_validate_value_rejects_string_length() {
        let limits = ConfigLimits::default().with_max_string_length(4);
        let value = leaf(crate::types::ConfigValue::string("too-long"), "s");
        let err = limits.validate_value(&value).unwrap_err();
        assert!(err.to_string().contains("string length"), "{err}");
    }
}
