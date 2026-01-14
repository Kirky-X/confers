// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Unified file format detection utilities
//!
//! Provides consistent file format detection across the entire codebase,
//! eliminating code duplication in format detection logic.

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// Supported configuration file formats
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileFormat {
    /// TOML format
    Toml,
    /// JSON format
    Json,
    /// YAML format
    Yaml,
    /// Unknown format
    Unknown,
}

impl std::fmt::Display for FileFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FileFormat::Toml => write!(f, "toml"),
            FileFormat::Json => write!(f, "json"),
            FileFormat::Yaml => write!(f, "yaml"),
            FileFormat::Unknown => write!(f, "unknown"),
        }
    }
}

/// Detect file format by examining file content with improved heuristics
/// Reads only the first 20 lines to avoid loading large files
pub fn detect_format_by_content(path: &Path) -> Option<FileFormat> {
    let file = File::open(path).ok()?;
    let reader = BufReader::new(file);
    let lines: Vec<String> = reader.lines().map_while(Result::ok).take(20).collect();

    if lines.is_empty() {
        return None;
    }

    let first_line = lines.first().map(|s| s.trim()).unwrap_or("");
    let second_line = lines.get(1).map(|s| s.trim());

    // JSON detection
    if first_line.starts_with('{') {
        return Some(FileFormat::Json);
    }

    if first_line.starts_with('[') {
        if lines.len() == 1 {
            return Some(FileFormat::Json);
        } else if let Some(second) = second_line {
            if second.starts_with('{') || second.starts_with('[') {
                return Some(FileFormat::Json);
            }
        }
        return Some(FileFormat::Json);
    }

    // YAML detection
    if first_line.starts_with("---") {
        return Some(FileFormat::Yaml);
    }

    if first_line.starts_with('#') {
        if let Some(second) = second_line {
            if second.starts_with('%') && (second.contains("YAML") || second.contains("yml")) {
                return Some(FileFormat::Yaml);
            }
        }
        return Some(FileFormat::Yaml);
    }

    let has_yaml_indicator = lines.iter().any(|line| {
        let trimmed = line.trim();
        trimmed.starts_with("---") || trimmed.ends_with(':')
    });

    if has_yaml_indicator {
        return Some(FileFormat::Yaml);
    }

    // Count format indicators
    let mut has_toml_equal = false;
    let mut has_toml_dot_table = false;
    let mut has_json_brace = false;
    let mut has_yaml_colon = false;

    for line in &lines {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // TOML: key=value pattern
        if !has_toml_equal && trimmed.contains('=') {
            let before_eq = trimmed.split('=').next().unwrap_or("").trim();
            if !before_eq.is_empty() {
                let after_eq = trimmed.split('=').nth(1).unwrap_or("").trim();
                if !after_eq.is_empty() && after_eq != "true" && after_eq != "false" {
                    has_toml_equal = true;
                }
            }
        }

        // TOML: [section.name] pattern
        if !has_toml_dot_table
            && (trimmed.starts_with('[') || trimmed.ends_with(']'))
            && trimmed.contains('.')
        {
            has_toml_dot_table = true;
        }

        // JSON: {"..." pattern
        if !has_json_brace
            && (trimmed.contains("{\"") || trimmed.contains("\":") || trimmed.contains("{\""))
        {
            has_json_brace = true;
        }

        // YAML: key: value pattern (but not URLs)
        if !has_yaml_colon && trimmed.contains(':') && !trimmed.contains("://") {
            has_yaml_colon = true;
        }
    }

    // Determine format based on indicators
    if has_toml_equal && !has_json_brace {
        return Some(FileFormat::Toml);
    }

    if has_yaml_colon && !has_toml_equal {
        return Some(FileFormat::Yaml);
    }

    if has_json_brace && has_toml_equal {
        return Some(FileFormat::Json);
    }

    if has_toml_dot_table {
        return Some(FileFormat::Toml);
    }

    // XML detection (rare for config files)
    if first_line.starts_with("<?xml") {
        return Some(FileFormat::Unknown);
    }

    None
}

/// Detect file format by extension
pub fn detect_format_by_extension(path: &Path) -> Option<FileFormat> {
    match path.extension()?.to_str()?.to_lowercase().as_str() {
        "toml" => Some(FileFormat::Toml),
        "json" => Some(FileFormat::Json),
        "yaml" | "yml" => Some(FileFormat::Yaml),
        _ => None,
    }
}

/// Smart format detection: try extension first, then content as fallback
pub fn detect_format_smart(path: &Path) -> Option<FileFormat> {
    // Try extension first (fastest)
    if let Some(format) = detect_format_by_extension(path) {
        return Some(format);
    }

    // Fall back to content detection
    detect_format_by_content(path)
}

/// Get format string from Path (simple extension-based detection)
pub fn get_format_from_path(path: &Path) -> String {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_lowercase())
        .unwrap_or_else(|| "unknown".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_detect_json_by_content() {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(b"{\n  \"name\": \"test\"\n}").unwrap();
        assert_eq!(
            detect_format_by_content(file.path()),
            Some(FileFormat::Json)
        );
    }

    #[test]
    fn test_detect_toml_by_content() {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(b"key = \"value\"\n").unwrap();
        assert_eq!(
            detect_format_by_content(file.path()),
            Some(FileFormat::Toml)
        );
    }

    #[test]
    fn test_detect_yaml_by_content() {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(b"---\nkey: value\n").unwrap();
        assert_eq!(
            detect_format_by_content(file.path()),
            Some(FileFormat::Yaml)
        );
    }

    #[test]
    fn test_detect_by_extension() {
        // Create temp file with .json extension
        let file = NamedTempFile::with_suffix(".json").unwrap();
        assert_eq!(
            detect_format_by_extension(file.path()),
            Some(FileFormat::Json)
        );
    }

    #[test]
    fn test_get_format_from_path() {
        let path = Path::new("config.json");
        assert_eq!(get_format_from_path(path), "json");
    }
}
