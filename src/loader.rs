//! Configuration file loaders with precise error locations.

use crate::error::{ConfigError, ConfigResult, ParseLocation};
use crate::value::{AnnotatedValue, ConfigValue, SourceId};
use std::path::Path;
use std::sync::Arc;

/// Maximum file size in bytes (default: 10MB)
const DEFAULT_MAX_SIZE: usize = 10 * 1024 * 1024;

/// Configuration for loaders.
#[derive(Debug, Clone)]
pub struct LoaderConfig {
    pub max_size: usize,
}

impl Default for LoaderConfig {
    fn default() -> Self { Self { max_size: DEFAULT_MAX_SIZE } }
}

/// Supported configuration formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format { Toml, Json, Yaml, Ini }

impl std::fmt::Display for Format {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Format::Toml => write!(f, "TOML"),
            Format::Json => write!(f, "JSON"),
            Format::Yaml => write!(f, "YAML"),
            Format::Ini => write!(f, "INI"),
        }
    }
}

pub fn detect_format_from_path(path: &Path) -> Option<Format> {
    match path.extension()?.to_str()?.to_lowercase().as_str() {
        "toml" => Some(Format::Toml),
        "json" => Some(Format::Json),
        "yaml" | "yml" => Some(Format::Yaml),
        "ini" => Some(Format::Ini),
        _ => None,
    }
}

pub fn detect_format_from_content(content: &str) -> Option<Format> {
    let trimmed = content.trim_start();
    let first_char = trimmed.chars().next()?;

    // JSON detection: more robust check
    if first_char == '{' || first_char == '[' {
        // Verify it's not YAML (YAML can also start with { but uses different syntax)
        // JSON uses strict key-value pairs with quotes
        if trimmed.contains("\"") && (trimmed.contains(":") || trimmed.contains(",")) {
            return Some(Format::Json);
        }
    }

    // YAML detection: document start marker is definitive
    if trimmed.starts_with("---") {
        return Some(Format::Yaml);
    }

    // TOML detection: look for specific patterns
    // TOML uses "key = value" pattern (with =, not :)
    // This is more specific than checking for "=" or ":" alone
    if trimmed.contains(" = ") || trimmed.contains("=\t") {
        // Make sure it's not YAML (YAML uses "key: value" not "key = value")
        return Some(Format::Toml);
    }

    // YAML detection: look for "key: value" pattern
    // Only if not TOML (check for unquoted colons with spaces after)
    if trimmed.contains(": ") {
        // But exclude if it looks like JSON or TOML
        if !trimmed.contains(" = ") && !trimmed.contains("{") {
            return Some(Format::Yaml);
        }
    }

    // INI detection: look for [section] headers or key=value patterns
    if trimmed.contains('[') && trimmed.contains(']') {
        // Check for INI section header pattern [section]
        if trimmed.chars().next() == Some('[') {
            return Some(Format::Ini);
        }
    }

    // Default to unknown
    None
}

pub fn load_file(path: &Path, config: &LoaderConfig) -> ConfigResult<AnnotatedValue> {
    let metadata = std::fs::metadata(path).map_err(|e| ConfigError::FileNotFound {
        filename: path.to_path_buf(), source: Some(e),
    })?;
    if metadata.len() as usize > config.max_size {
        return Err(ConfigError::SizeLimitExceeded { actual: metadata.len() as usize, limit: config.max_size });
    }
    let format = detect_format_from_path(path).ok_or_else(|| ConfigError::ParseError {
        format: "unknown".into(), message: format!("Unknown extension: {:?}", path.extension()), location: None, source: None,
    })?;
    let content = std::fs::read_to_string(path).map_err(ConfigError::IoError)?;
    let source = SourceId::new(path.file_name().and_then(|n| n.to_str()).unwrap_or("unknown"));
    parse_content(&content, format, source, Some(path))
}

pub fn parse_content(content: &str, format: Format, source: SourceId, path: Option<&Path>) -> ConfigResult<AnnotatedValue> {
    match format {
        Format::Toml => parse_toml(content, source, path),
        Format::Json => parse_json(content, source, path),
        Format::Yaml => parse_yaml(content, source, path),
        Format::Ini => parse_ini(content, source, path),
    }
}

#[cfg(feature = "toml")]
pub fn parse_toml(content: &str, source: SourceId, path: Option<&Path>) -> ConfigResult<AnnotatedValue> {
    use toml::de::Error as TomlError;
    
    let table: toml::Table = toml::from_str(content).map_err(|e: TomlError| {
        let location = e.span().map(|span| {
            let line = content[..span.start].matches('\n').count() + 1;
            let col = span.start - content[..span.start].rfind('\n').map(|i| i + 1).unwrap_or(0) + 1;
            path.map(|p| ParseLocation::from_path(p, line, col)).unwrap_or_else(|| ParseLocation::new(source.as_str(), line, col))
        });
        ConfigError::ParseError { format: "TOML".into(), message: e.message().to_string(), location, source: Some(Box::new(e)) }
    })?;
    
    Ok(AnnotatedValue::new(toml_table_to_config_value(&table, &source, ""), source, ""))
}

#[cfg(feature = "toml")]
fn toml_table_to_config_value(table: &toml::Table, source: &SourceId, prefix: &str) -> ConfigValue {
    let entries: Vec<(Arc<str>, AnnotatedValue)> = table.iter().map(|(k, v)| {
        let path = if prefix.is_empty() { k.clone() } else { format!("{}.{}", prefix, k) };
        (Arc::from(path.clone()), AnnotatedValue::new(toml_value_to_config_value(v, source, &path), source.clone(), k.clone()))
    }).collect();
    ConfigValue::map(entries)
}

#[cfg(feature = "toml")]
fn toml_value_to_config_value(value: &toml::Value, source: &SourceId, prefix: &str) -> ConfigValue {
    match value {
        toml::Value::String(s) => ConfigValue::String(s.clone()),
        toml::Value::Integer(i) => ConfigValue::I64(*i),
        toml::Value::Float(f) => ConfigValue::F64(*f),
        toml::Value::Boolean(b) => ConfigValue::Bool(*b),
        toml::Value::Datetime(dt) => ConfigValue::String(dt.to_string()),
        toml::Value::Array(arr) => {
            ConfigValue::Array(arr.iter().enumerate().map(|(i, v)| {
                let path = format!("{}.{}", prefix, i);
                AnnotatedValue::new(toml_value_to_config_value(v, source, &path), source.clone(), path)
            }).collect::<Vec<_>>().into())
        }
        toml::Value::Table(t) => toml_table_to_config_value(t, source, prefix),
    }
}

#[cfg(feature = "json")]
pub fn parse_json(content: &str, source: SourceId, _: Option<&Path>) -> ConfigResult<AnnotatedValue> {
    let v: serde_json::Value = serde_json::from_str(content).map_err(|e| ConfigError::ParseError {
        format: "JSON".into(), message: e.to_string(), location: None, source: Some(Box::new(e))
    })?;
    Ok(AnnotatedValue::new(json_to_config_value(&v, &source, ""), source, ""))
}

#[cfg(feature = "json")]
fn json_to_config_value(v: &serde_json::Value, src: &SourceId, pre: &str) -> ConfigValue {
    match v {
        serde_json::Value::Null => ConfigValue::Null,
        serde_json::Value::Bool(b) => ConfigValue::Bool(*b),
        serde_json::Value::Number(n) => n.as_i64().map(ConfigValue::I64)
            .or_else(|| n.as_u64().map(ConfigValue::U64)).or_else(|| n.as_f64().map(ConfigValue::F64)).unwrap_or(ConfigValue::Null),
        serde_json::Value::String(s) => ConfigValue::String(s.clone()),
        serde_json::Value::Array(a) => ConfigValue::Array(a.iter().enumerate().map(|(i, v)| {
            let p = format!("{}.{}", pre, i); AnnotatedValue::new(json_to_config_value(v, src, &p), src.clone(), p)
        }).collect::<Vec<_>>().into()),
        serde_json::Value::Object(o) => ConfigValue::map(o.iter().map(|(k, v)| {
            let p = if pre.is_empty() { k.clone() } else { format!("{}.{}", pre, k) };
            (Arc::from(p.clone()), AnnotatedValue::new(json_to_config_value(v, src, &p), src.clone(), k.clone()))
        }).collect()),
    }
}

#[cfg(feature = "yaml")]
pub fn parse_yaml(content: &str, source: SourceId, path: Option<&Path>) -> ConfigResult<AnnotatedValue> {
    let v: serde_yaml_ng::Value = serde_yaml_ng::from_str(content).map_err(|e| {
        let loc = e.location().map(|l| path.map(|p| ParseLocation::from_path(p, l.line(), l.column()))
            .unwrap_or_else(|| ParseLocation::new(source.as_str(), l.line(), l.column())));
        ConfigError::ParseError { format: "YAML".into(), message: e.to_string(), location: loc, source: Some(Box::new(e)) }
    })?;
    Ok(AnnotatedValue::new(yaml_to_config_value(&v, &source, ""), source, ""))
}

#[cfg(feature = "yaml")]
fn yaml_to_config_value(v: &serde_yaml_ng::Value, src: &SourceId, pre: &str) -> ConfigValue {
    match v {
        serde_yaml_ng::Value::Null => ConfigValue::Null,
        serde_yaml_ng::Value::Bool(b) => ConfigValue::Bool(*b),
        serde_yaml_ng::Value::Number(n) => n.as_i64().map(ConfigValue::I64)
            .or_else(|| n.as_u64().map(ConfigValue::U64)).or_else(|| n.as_f64().map(ConfigValue::F64)).unwrap_or(ConfigValue::Null),
        serde_yaml_ng::Value::String(s) => ConfigValue::String(s.clone()),
        serde_yaml_ng::Value::Sequence(s) => ConfigValue::Array(s.iter().enumerate().map(|(i, v)| {
            let p = format!("{}.{}", pre, i); AnnotatedValue::new(yaml_to_config_value(v, src, &p), src.clone(), p)
        }).collect::<Vec<_>>().into()),
        serde_yaml_ng::Value::Mapping(m) => ConfigValue::map(m.iter().filter_map(|(k, v)| k.as_str().map(|key| {
            let p = if pre.is_empty() { key.to_string() } else { format!("{}.{}", pre, key) };
            (Arc::from(p.clone()), AnnotatedValue::new(yaml_to_config_value(v, src, &p), src.clone(), key))
        })).collect()),
        serde_yaml_ng::Value::Tagged(t) => yaml_to_config_value(&t.value, src, pre),
    }
}

#[cfg(not(feature = "toml"))]
pub fn parse_toml(_: &str, _: SourceId, _: Option<&Path>) -> ConfigResult<AnnotatedValue> {
    Err(ConfigError::ParseError { format: "TOML".into(), message: "Add 'toml' feature".into(), location: None, source: None })
}
#[cfg(not(feature = "json"))]
pub fn parse_json(_: &str, _: SourceId, _: Option<&Path>) -> ConfigResult<AnnotatedValue> {
    Err(ConfigError::ParseError { format: "JSON".into(), message: "Add 'json' feature".into(), location: None, source: None })
}
#[cfg(not(feature = "yaml"))]
pub fn parse_yaml(_: &str, _: SourceId, _: Option<&Path>) -> ConfigResult<AnnotatedValue> {
    Err(ConfigError::ParseError { format: "YAML".into(), message: "Add 'yaml' feature".into(), location: None, source: None })
}
#[cfg(not(feature = "ini"))]
pub fn parse_ini(_: &str, _: SourceId, _: Option<&Path>) -> ConfigResult<AnnotatedValue> {
    Err(ConfigError::ParseError { format: "INI".into(), message: "Add 'ini' feature".into(), location: None, source: None })
}

/// Parse a TOML table into AnnotatedValue (public helper for remote sources).
#[cfg(feature = "toml")]
pub fn parse_toml_table(table: &toml::Table, source: &SourceId, prefix: &str) -> AnnotatedValue {
    AnnotatedValue::new(toml_table_to_config_value(table, source, prefix), source.clone(), prefix)
}

/// Parse a JSON value into ConfigValue (public helper for remote sources).
#[cfg(feature = "json")]
pub fn parse_json_value(v: &serde_json::Value, source: &SourceId, prefix: &str) -> AnnotatedValue {
    AnnotatedValue::new(json_to_config_value(v, source, prefix), source.clone(), prefix)
}

/// Parse a YAML value into ConfigValue (public helper for remote sources).
#[cfg(feature = "yaml")]
pub fn parse_yaml_value(v: &serde_yaml_ng::Value, source: &SourceId, prefix: &str) -> AnnotatedValue {
    AnnotatedValue::new(yaml_to_config_value(v, source, prefix), source.clone(), prefix)
}

#[cfg(feature = "ini")]
pub fn parse_ini(content: &str, source: SourceId, path: Option<&Path>) -> ConfigResult<AnnotatedValue> {
    let mut sections: indexmap::IndexMap<String, indexmap::IndexMap<String, String>> = indexmap::IndexMap::new();
    let mut cur = String::new();
    let mut invalid_lines = Vec::new();

    for (line_num, line) in content.lines().enumerate() {
        let l = line.trim();
        // Skip empty lines and comments
        if l.is_empty() || l.starts_with('#') || l.starts_with(';') { continue; }
        // Section header
        if l.starts_with('[') && l.ends_with(']') {
            cur = l[1..l.len()-1].trim().into();
            sections.entry(cur.clone()).or_default();
            continue;
        }
        // Key-value pair
        if let Some(eq) = l.find('=') {
            let key = l[..eq].trim();
            let value = l[eq+1..].trim();
            if key.is_empty() {
                invalid_lines.push((line_num + 1, line.to_string(), "empty key"));
                continue;
            }
            sections.entry(cur.clone()).or_default().insert(key.into(), value.into());
            continue;
        }
        // Track invalid lines for debugging
        invalid_lines.push((line_num + 1, line.to_string(), "invalid INI syntax"));
    }

    // Log warnings for invalid lines if any were found
    if !invalid_lines.is_empty() {
        tracing::warn!(
            "INI parsing found {} potentially invalid line(s) in {:?}: {:?}",
            invalid_lines.len(),
            path.map(|p| p.to_string_lossy().to_string()).unwrap_or_else(|| source.as_str().to_string()),
            invalid_lines.iter().take(5).collect::<Vec<_>>()
        );
    }

    // Build the map manually to avoid closure borrow issues
    let mut entries: Vec<(Arc<str>, AnnotatedValue)> = Vec::new();
    for (sec, keys) in sections.iter() {
        for (k, v) in keys.iter() {
            let key = if sec.is_empty() { k.clone() } else { format!("{}.{}", sec, k) };
            entries.push((
                Arc::from(key.clone()),
                AnnotatedValue::new(ConfigValue::String(v.clone()), source.clone(), key),
            ));
        }
    }

    Ok(AnnotatedValue::new(ConfigValue::map(entries), source, ""))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_detect_format_from_path() {
        assert_eq!(detect_format_from_path(Path::new("a.toml")), Some(Format::Toml));
        assert_eq!(detect_format_from_path(Path::new("a.json")), Some(Format::Json));
    }
    #[test] fn test_detect_format_from_content() {
        assert_eq!(detect_format_from_content(r#"{"k":"v"}"#), Some(Format::Json));
        assert_eq!(detect_format_from_content(r#"k = "v""#), Some(Format::Toml));
    }
    #[test] #[cfg(feature = "toml")] fn test_parse_toml() {
        let r = parse_toml("\n[db]\nhost = \"localhost\"\nport = 5432\n", SourceId::new("test"), None);
        assert!(r.is_ok(), "{:?}", r.err());
        assert!(r.unwrap().is_map());
    }
    #[test] #[cfg(feature = "json")] fn test_parse_json() {
        let r = parse_json(r#"{"db":{"host":"localhost","port":5432}}"#, SourceId::new("test"), None);
        assert!(r.is_ok(), "{:?}", r.err());
        assert!(r.unwrap().is_map());
    }
    #[test] #[cfg(feature = "yaml")] fn test_parse_yaml() {
        let r = parse_yaml("\ndb:\n  host: localhost\n  port: 5432\n", SourceId::new("test"), None);
        assert!(r.is_ok(), "{:?}", r.err());
        assert!(r.unwrap().is_map());
    }
    #[test] #[cfg(feature = "toml")] fn test_parse_toml_error() {
        assert!(parse_toml("[section", SourceId::new("t"), None).is_err());
    }
    #[test] fn test_loader_config_default() { assert_eq!(LoaderConfig::default().max_size, DEFAULT_MAX_SIZE); }
}
