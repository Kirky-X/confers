// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Unified file format detection utilities
//!
//! Provides consistent file format detection and parsing across the entire
//! codebase, eliminating code duplication in format detection and parsing logic.

use figment::value::{Dict, Value as FigmentValue};
use figment::{providers::Serialized, Figment, Profile};
use serde_json::Value as JsonValue;
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
    /// INI format
    Ini,
    /// Unknown format
    Unknown,
}

impl std::fmt::Display for FileFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FileFormat::Toml => write!(f, "toml"),
            FileFormat::Json => write!(f, "json"),
            FileFormat::Yaml => write!(f, "yaml"),
            FileFormat::Ini => write!(f, "ini"),
            FileFormat::Unknown => write!(f, "unknown"),
        }
    }
}

impl std::str::FromStr for FileFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "toml" => Ok(FileFormat::Toml),
            "json" => Ok(FileFormat::Json),
            "yaml" | "yml" => Ok(FileFormat::Yaml),
            "ini" => Ok(FileFormat::Ini),
            _ => Err(format!("Unknown file format: {}", s)),
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
        let trimmed = first_line.trim();
        // Improved JSON array detection
        // If it contains indicators of a JSON array (quotes, braces, commas, or is just opening bracket)
        if trimmed == "[" || trimmed.contains('"') || trimmed.contains('{') || trimmed.contains(',') {
            return Some(FileFormat::Json);
        }

        // If it's a single line like "[123]", default to JSON as it's a valid array
        // but unlikely to be a useful INI (empty section)
        if lines.len() == 1 {
            return Some(FileFormat::Json);
        }
        
        // Otherwise, if it looks like [section] and has more lines, 
        // let the full scan determine if it's INI/TOML (looking for key=value)
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
    let mut has_ini_section = false;
    let mut has_ini_comment = false;
    let mut has_ini_equal = false;

    for line in &lines {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if !has_ini_comment && trimmed.starts_with(';') {
            has_ini_comment = true;
        }

        if !has_ini_section
            && trimmed.starts_with('[')
            && trimmed.ends_with(']')
            && !trimmed.starts_with("[[")
            && !trimmed.contains('.')
            && trimmed.len() > 2
        {
            has_ini_section = true;
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

        if !has_ini_equal && trimmed.contains('=') && !trimmed.starts_with('[') {
            has_ini_equal = true;
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

    if has_ini_comment || (has_ini_section && has_ini_equal && !has_toml_dot_table) {
        return Some(FileFormat::Ini);
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
        "ini" => Some(FileFormat::Ini),
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
    fn test_detect_ini_by_content() {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(b"[server]\nport=8080\n").unwrap();
        assert_eq!(
            detect_format_by_content(file.path()),
            Some(FileFormat::Ini)
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

        let ini_file = NamedTempFile::with_suffix(".ini").unwrap();
        assert_eq!(
            detect_format_by_extension(ini_file.path()),
            Some(FileFormat::Ini)
        );
    }

    #[test]
    fn test_get_format_from_path() {
        let path = Path::new("config.json");
        assert_eq!(get_format_from_path(path), "json");
    }
}

/// Unified content parsing result
#[derive(Debug, Clone)]
pub struct ParsedContent {
    /// The parsed Figment configuration
    pub figment: Figment,
    /// The detected format
    pub format: FileFormat,
}

/// Unified content parsing function
///
/// Parses content string into Figment format based on content type or auto-detection.
/// This function eliminates code duplication in format parsing across the codebase.
///
/// # Arguments
/// * `content` - The raw content string to parse
/// * `content_type` - Optional content-type header (e.g., "application/json")
///
/// # Returns
/// Result containing ParsedContent with Figment and detected format
pub fn parse_content(content: &str, content_type: Option<&str>) -> Result<ParsedContent, String> {
    let format = detect_format_from_content_type(content_type)
        .unwrap_or_else(|| detect_format_by_string(content));

    let figment = match format {
        FileFormat::Json => {
            let json_value: JsonValue = serde_json::from_str(content)
                .map_err(|e| format!("Failed to parse JSON: {}", e))?;
            let dict: Dict = serde_json::from_value(json_value)
                .map_err(|e| format!("Failed to convert JSON to dict: {}", e))?;
            Figment::new().merge(Serialized::from(dict, Profile::Default))
        }
        FileFormat::Toml => {
            let toml_value: FigmentValue = toml::from_str(content)
                .map_err(|e| format!("Failed to parse TOML: {}", e))?;
            let dict: Dict = toml_value
                .deserialize()
                .map_err(|e| format!("Failed to convert TOML to dict: {}", e))?;
            Figment::new().merge(Serialized::from(dict, Profile::Default))
        }
        FileFormat::Yaml => {
            let yaml_value: FigmentValue = serde_yaml::from_str(content)
                .map_err(|e| format!("Failed to parse YAML: {}", e))?;
            let dict: Dict = yaml_value
                .deserialize()
                .map_err(|e| format!("Failed to convert YAML to dict: {}", e))?;
            Figment::new().merge(Serialized::from(dict, Profile::Default))
        }
        FileFormat::Ini => {
            let ini_value: JsonValue = serde_ini::from_str(content)
                .map_err(|e| format!("Failed to parse INI: {}", e))?;
            let dict: Dict = serde_json::from_value(ini_value)
                .map_err(|e| format!("Failed to convert INI to dict: {}", e))?;
            Figment::new().merge(Serialized::from(dict, Profile::Default))
        }
        FileFormat::Unknown => {
            // Try JSON as fallback
            match serde_json::from_str::<JsonValue>(content) {
                Ok(json_value) => {
                    let dict: Dict = serde_json::from_value(json_value)
                        .map_err(|e| format!("Failed to convert JSON to dict: {}", e))?;
                    Figment::new().merge(Serialized::from(dict, Profile::Default))
                }
                Err(_) => {
                    return Err("Unknown format and JSON fallback failed".to_string());
                }
            }
        }
    };

    Ok(ParsedContent { figment, format })
}

/// Detect format from HTTP content-type header
pub fn detect_format_from_content_type(content_type: Option<&str>) -> Option<FileFormat> {
    content_type.and_then(|ct| {
        let ct = ct.to_lowercase();
        if ct.contains("application/json") || ct.contains("text/json") {
            Some(FileFormat::Json)
        } else if ct.contains("application/toml") || ct.contains("text/toml") {
            Some(FileFormat::Toml)
        } else if ct.contains("application/yaml") || ct.contains("text/yaml") {
            Some(FileFormat::Yaml)
        } else if ct.contains("application/ini") || ct.contains("text/ini") {
            Some(FileFormat::Ini)
        } else {
            None
        }
    })
}

/// Detect format from content string (auto-detection)
pub fn detect_format_by_string(content: &str) -> FileFormat {
    let trimmed = content.trim_start();
    let first_line = trimmed.lines().next().unwrap_or("").trim();

    // JSON detection
    if first_line.starts_with('{') || first_line.starts_with('[') {
        return FileFormat::Json;
    }

    // YAML detection
    if first_line.starts_with("---") || trimmed.starts_with('#') {
        return FileFormat::Yaml;
    }

    // TOML detection - look for key=value or [section] patterns
    for line in trimmed.lines().take(20) {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') && !line.starts_with("[[") {
            if line.contains('.') {
                return FileFormat::Toml;
            }
            return FileFormat::Ini;
        }
        if line.contains('=') {
            return FileFormat::Toml;
        }
        if line.contains(':') && !line.contains("://") {
            return FileFormat::Yaml;
        }
    }

    FileFormat::Unknown
}

/// Serialize configuration dictionary to INI format
///
/// # Arguments
/// * `dict` - The configuration dictionary to serialize
///
/// # Returns
/// INI formatted string
pub fn serialize_to_ini(dict: &serde_json::Map<String, serde_json::Value>) -> String {
    let mut output = String::new();

    fn serialize_value(
        value: &serde_json::Value,
        prefix: &str,
        output: &mut String,
        indent: usize,
    ) {
        match value {
            serde_json::Value::Object(map) => {
                for (k, v) in map {
                    let new_prefix = if prefix.is_empty() {
                        k.clone()
                    } else {
                        format!("{}.{}", prefix, k)
                    };
                    serialize_value(v, &new_prefix, output, indent);
                }
            }
            serde_json::Value::Array(arr) => {
                for (i, item) in arr.iter().enumerate() {
                    let new_prefix = if prefix.is_empty() {
                        format!("{}.{}", prefix, i)
                    } else {
                        format!("{}.{}", prefix, i)
                    };
                    serialize_value(item, &new_prefix, output, indent);
                }
            }
            serde_json::Value::String(s) => {
                if s.contains('=') {
                    let escaped = s.replace('=', "\\=");
                    output.push_str(&format!("{}={}\n", prefix, escaped));
                } else {
                    output.push_str(&format!("{}={}\n", prefix, s));
                }
            }
            serde_json::Value::Number(n) => {
                output.push_str(&format!("{}={}\n", prefix, n));
            }
            serde_json::Value::Bool(b) => {
                output.push_str(&format!("{}={}\n", prefix, b));
            }
            serde_json::Value::Null => {
                output.push_str(&format!("{}=\n", prefix));
            }
        }
    }

    // Group by top-level keys as sections
    let mut sections: std::collections::HashMap<String, serde_json::Map<String, serde_json::Value>> =
        std::collections::HashMap::new();

    for (key, value) in dict {
        if let serde_json::Value::Object(map) = value {
            // This is a section
            if !map.is_empty() {
                sections.insert(key.clone(), map.clone());
            } else {
                // Empty section, just add as key=value
                output.push_str(&format!("{} =\n", key));
            }
        } else {
            // Top-level value, add to default section
            let default_section = sections.entry("DEFAULT".to_string()).or_default();
            default_section.insert(key.clone(), value.clone());
        }
    }

    // Write sections
    for (section_name, section_map) in sections {
        output.push('[');
        output.push_str(&section_name);
        output.push_str("]\n");
        for (key, value) in section_map {
            match value {
                serde_json::Value::Object(nested) => {
                    // Nested object - flatten with dots
                    for (k, v) in nested {
                        let full_key = format!("{}.{}", key, k);
                        serialize_value(&v, &full_key, &mut output, 0);
                    }
                }
                serde_json::Value::Array(arr) => {
                    for (i, item) in arr.iter().enumerate() {
                        let full_key = format!("{}.{}", key, i);
                        serialize_value(item, &full_key, &mut output, 0);
                    }
                }
                _ => {
                    let val_str = match &value {
                        serde_json::Value::String(s) => s.clone(),
                        serde_json::Value::Number(n) => n.to_string(),
                        serde_json::Value::Bool(b) => b.to_string(),
                        serde_json::Value::Null => String::new(),
                        _ => serde_json::to_string(&value).unwrap_or_default(),
                    };
                    // Escape = in values
                    let escaped = val_str.replace('=', "\\=");
                    output.push_str(&format!("{}={}\n", key, escaped));
                }
            }
        }
        output.push('\n');
    }

    // Remove trailing newline
    if output.ends_with("\n\n") {
        output.pop();
    }

    output
}

/// Unified file serialization function
///
/// Serializes configuration data to the specified format.
///
/// # Arguments
/// * `data` - The configuration data as JSON value
/// * `format` - The target format
///
/// # Returns
/// Serialized string
pub fn serialize_to_format(
    data: &serde_json::Value,
    format: FileFormat,
) -> Result<String, String> {
    match format {
        FileFormat::Json => serde_json::to_string_pretty(data)
            .map_err(|e| format!("Failed to serialize JSON: {}", e)),
        FileFormat::Toml => {
            toml::to_string(data).map_err(|e| format!("Failed to serialize TOML: {}", e))
        }
        FileFormat::Yaml => serde_yaml::to_string(data)
            .map_err(|e| format!("Failed to serialize YAML: {}", e)),
        FileFormat::Ini => {
            if let serde_json::Value::Object(map) = data {
                Ok(serialize_to_ini(map))
            } else {
                Err("INI format requires object data".to_string())
            }
        }
        FileFormat::Unknown => Err("Unknown format".to_string()),
    }
}
