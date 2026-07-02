//! Format converter abstraction for configuration parsing.
//!
//! This module provides the `FormatConverter` trait, which abstracts over
//! the different configuration file formats (TOML, JSON, YAML, INI, ENV).
//!
//! The trait allows:
//! - `detect()`: Auto-detect format from content
//! - `parse()`: Parse a string into `AnnotatedValue`
//! - `serialize()`: Serialize an `AnnotatedValue` back to string
//!
//! Implementations exist for: TOML, JSON, YAML, INI.
//!
//! # Example
//!
//! ```
//! use confers::format::{FormatConverter, Format, converter_for};
//!
//! # #[cfg(feature = "json")]
//! # {
//! let content = r#"{"name":"test","port":8080}"#;
//! let converter = converter_for(Format::Json).expect("JSON converter should exist");
//! let result = converter.parse(content, confers::types::SourceId::new("test"), None);
//! assert!(result.is_ok());
//! # }
//! ```

use crate::error::{ConfigError, ConfigResult};
use crate::types::{AnnotatedValue, ConfigValue, SourceId};
use std::path::Path;

#[cfg(feature = "json")]
use super::convert::json_to_config_value;
#[cfg(feature = "toml")]
use super::convert::toml_table_to_config_value;
#[cfg(feature = "yaml")]
use super::convert::yaml_to_config_value;

/// Result of format detection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormatMatch {
    /// Confident match for this format
    Confident,
    /// Possible match (ambiguous)
    Possible,
    /// Not this format
    NoMatch,
}

/// Trait for converting between configuration formats.
///
/// Implementors handle parsing and serializing a specific configuration format
/// (e.g., TOML, JSON, YAML, INI).
pub trait FormatConverter: Send + Sync {
    /// Get the format enum value for this converter.
    fn format(&self) -> Format;

    /// Get the file extension(s) associated with this format.
    fn extension(&self) -> &'static str;

    /// Detect if the given content matches this format.
    ///
    /// Returns `FormatMatch::Confident` if the content is clearly this format,
    /// `FormatMatch::Possible` if it might be this format, or `FormatMatch::NoMatch`
    /// if it's clearly not.
    fn detect(&self, content: &str) -> FormatMatch;

    /// Parse a string into an `AnnotatedValue`.
    ///
    /// # Arguments
    ///
    /// * `content` - The raw configuration content
    /// * `source` - The source identifier for error reporting
    /// * `path` - Optional file path for error location reporting
    fn parse(
        &self,
        content: &str,
        source: SourceId,
        path: Option<&Path>,
    ) -> ConfigResult<AnnotatedValue>;

    /// Serialize an `AnnotatedValue` back to a string in this format.
    ///
    /// Returns an error if the value cannot be serialized (e.g., invalid type
    /// for this format).
    fn serialize(&self, value: &AnnotatedValue) -> ConfigResult<String>;

    /// Check if this format supports the given feature.
    fn supports(&self, feature: FormatFeature) -> bool;
}

/// Features that a format may or may not support.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormatFeature {
    /// Nested structures (maps within maps)
    NestedMaps,
    /// Arrays of values
    Arrays,
    /// Comments in the source
    Comments,
    /// Inline comments
    InlineComments,
    /// Multiline strings
    MultilineStrings,
    /// Boolean values
    Booleans,
    /// Floating-point numbers
    Floats,
    /// Null/none values
    Null,
    /// Date/time values
    DateTime,
    /// Raw binary data
    Binary,
    /// Top-level arrays
    TopLevelArrays,
    /// Hierarchical/nested sections
    Sections,
}

// =============================================================================
// TOML Converter
// =============================================================================

#[cfg(feature = "toml")]
mod toml_converter {
    use super::*;
    use crate::error::ParseLocation;

    pub struct TomlConverter;

    impl TomlConverter {
        pub fn new() -> Self {
            Self
        }
    }

    impl Default for TomlConverter {
        fn default() -> Self {
            Self::new()
        }
    }

    impl FormatConverter for TomlConverter {
        fn format(&self) -> Format {
            Format::Toml
        }

        fn extension(&self) -> &'static str {
            "toml"
        }

        fn detect(&self, content: &str) -> FormatMatch {
            let trimmed = content.trim();
            // TOML: key = value pattern, tables [section] pattern
            // Strong indicator: multiple " = " with no colons nearby
            let has_key_value = trimmed.contains(" = ");
            let has_sections = trimmed.starts_with('[');
            // JSON objects { ... } and JSON arrays [ ... ] are JSON.
            // Also reject YAML flow sequences [a, b] which have comma after ].
            let is_json_like = trimmed.starts_with('{')
                || (trimmed.starts_with('[') && (trimmed.ends_with(']') || trimmed.contains(',')));
            let no_json_like = !is_json_like;
            let no_yaml_doc = !trimmed.starts_with("---");
            let no_yaml_colon_only = !trimmed.contains(": ");

            // TOML confident if: has " = " (TOML key=value) OR has a [section] paired with key=value.
            let confident_key_value =
                has_key_value && no_json_like && no_yaml_doc && no_yaml_colon_only;
            let confident_section = has_sections && has_key_value && no_json_like && no_yaml_doc;
            if confident_key_value || confident_section {
                FormatMatch::Confident
            } else if has_key_value && no_json_like {
                FormatMatch::Possible
            } else {
                FormatMatch::NoMatch
            }
        }

        fn parse(
            &self,
            content: &str,
            source: SourceId,
            path: Option<&Path>,
        ) -> ConfigResult<AnnotatedValue> {
            let table: toml::Table = toml::from_str(content).map_err(|e: toml::de::Error| {
                let location = e.span().map(|span| {
                    let line = content[..span.start].matches('\n').count() + 1;
                    let col = span.start
                        - content[..span.start]
                            .rfind('\n')
                            .map(|i| i + 1)
                            .unwrap_or(0)
                        + 1;
                    path.map(|p| ParseLocation::from_path(p, line, col))
                        .unwrap_or_else(|| ParseLocation::new(source.as_str(), line, col))
                });
                ConfigError::ParseError {
                    format: "TOML".into(),
                    message: e.message().to_string(),
                    location,
                    source: Some(Box::new(e)),
                }
            })?;

            Ok(AnnotatedValue::new(
                toml_table_to_config_value(&table, &source, ""),
                source,
                "",
            ))
        }

        fn serialize(&self, value: &AnnotatedValue) -> ConfigResult<String> {
            // Simple serialization - convert to TOML Value and use toml::to_string
            let toml_val = config_value_to_toml_value(&value.inner);
            toml::to_string_pretty(&toml_val).map_err(|e| ConfigError::InvalidValue {
                key: "serialization".to_string(),
                expected_type: "TOML".to_string(),
                message: format!("TOML serialization failed: {}", e),
            })
        }

        fn supports(&self, feature: FormatFeature) -> bool {
            match feature {
                FormatFeature::NestedMaps => true,
                FormatFeature::Arrays => true,
                FormatFeature::Comments => true,
                FormatFeature::InlineComments => true,
                FormatFeature::MultilineStrings => true,
                FormatFeature::Booleans => true,
                FormatFeature::Floats => true,
                FormatFeature::Null => false,
                FormatFeature::DateTime => true,
                FormatFeature::Binary => true,
                FormatFeature::TopLevelArrays => false,
                FormatFeature::Sections => true,
            }
        }
    }

    fn config_value_to_toml_value(value: &ConfigValue) -> toml::Value {
        use toml::map::Map;
        match value {
            ConfigValue::Null => toml::Value::String(String::new()),
            ConfigValue::Bool(b) => toml::Value::Boolean(*b),
            ConfigValue::I64(i) => toml::Value::Integer(*i),
            ConfigValue::U64(u) => toml::Value::Integer(*u as i64),
            ConfigValue::F64(f) => toml::Value::Float(*f),
            ConfigValue::String(s) => toml::Value::String(s.clone()),
            ConfigValue::Bytes(b) => {
                toml::Value::Array(b.iter().map(|&b| toml::Value::Integer(b as i64)).collect())
            }
            ConfigValue::Array(arr) => toml::Value::Array(
                arr.iter()
                    .map(|v| config_value_to_toml_value(&v.inner))
                    .collect(),
            ),
            ConfigValue::Map(map) => {
                let mut m = Map::new();
                for (k, v) in map.iter() {
                    m.insert(k.to_string(), config_value_to_toml_value(&v.inner));
                }
                toml::Value::Table(m)
            }
        }
    }
}

// =============================================================================
// JSON Converter
// =============================================================================

#[cfg(feature = "json")]
mod json_converter {
    use super::*;

    pub struct JsonConverter;

    impl JsonConverter {
        pub fn new() -> Self {
            Self
        }
    }

    impl Default for JsonConverter {
        fn default() -> Self {
            Self::new()
        }
    }

    impl FormatConverter for JsonConverter {
        fn format(&self) -> Format {
            Format::Json
        }

        fn extension(&self) -> &'static str {
            "json"
        }

        fn detect(&self, content: &str) -> FormatMatch {
            let trimmed = content.trim();
            if trimmed.starts_with('{') || trimmed.starts_with('[') {
                // Try to parse as JSON to be sure
                if serde_json::from_str::<serde_json::Value>(trimmed).is_ok() {
                    FormatMatch::Confident
                } else {
                    FormatMatch::NoMatch
                }
            } else {
                FormatMatch::NoMatch
            }
        }

        fn parse(
            &self,
            content: &str,
            source: SourceId,
            _path: Option<&Path>,
        ) -> ConfigResult<AnnotatedValue> {
            let v: serde_json::Value =
                serde_json::from_str(content).map_err(|e| ConfigError::ParseError {
                    format: "JSON".into(),
                    message: e.to_string(),
                    location: None,
                    source: Some(Box::new(e)),
                })?;
            Ok(AnnotatedValue::new(
                json_to_config_value(&v, &source, ""),
                source,
                "",
            ))
        }

        fn serialize(&self, value: &AnnotatedValue) -> ConfigResult<String> {
            let json = json_value_from_config(&value.inner);
            serde_json::to_string_pretty(&json).map_err(|e| ConfigError::InvalidValue {
                key: "serialization".to_string(),
                expected_type: "JSON".to_string(),
                message: format!("JSON serialization failed: {}", e),
            })
        }

        fn supports(&self, feature: FormatFeature) -> bool {
            match feature {
                FormatFeature::NestedMaps => true,
                FormatFeature::Arrays => true,
                FormatFeature::Comments => false,
                FormatFeature::InlineComments => false,
                FormatFeature::MultilineStrings => true,
                FormatFeature::Booleans => true,
                FormatFeature::Floats => true,
                FormatFeature::Null => true,
                FormatFeature::DateTime => true,
                FormatFeature::Binary => false,
                FormatFeature::TopLevelArrays => true,
                FormatFeature::Sections => false,
            }
        }
    }

    fn json_value_from_config(value: &ConfigValue) -> serde_json::Value {
        use base64::Engine;
        match value {
            ConfigValue::Null => serde_json::Value::Null,
            ConfigValue::Bool(b) => serde_json::Value::Bool(*b),
            ConfigValue::I64(i) => serde_json::Value::Number(serde_json::Number::from(*i)),
            ConfigValue::U64(u) => serde_json::Value::Number(serde_json::Number::from(*u)),
            ConfigValue::F64(f) => serde_json::Number::from_f64(*f)
                .map_or(serde_json::Value::Null, serde_json::Value::Number),
            ConfigValue::String(s) => serde_json::Value::String(s.clone()),
            ConfigValue::Bytes(b) => {
                let encoded = base64::engine::general_purpose::STANDARD.encode(b);
                serde_json::Value::String(encoded)
            }
            ConfigValue::Array(arr) => serde_json::Value::Array(
                arr.iter()
                    .map(|v| json_value_from_config(&v.inner))
                    .collect(),
            ),
            ConfigValue::Map(map) => serde_json::Value::Object(
                map.iter()
                    .map(|(k, v)| (k.to_string(), json_value_from_config(&v.inner)))
                    .collect(),
            ),
        }
    }
}

// =============================================================================
// YAML Converter
// =============================================================================

#[cfg(feature = "yaml")]
mod yaml_converter {
    use super::*;
    use crate::error::ParseLocation;

    pub struct YamlConverter;

    impl YamlConverter {
        pub fn new() -> Self {
            Self
        }
    }

    impl Default for YamlConverter {
        fn default() -> Self {
            Self::new()
        }
    }

    impl FormatConverter for YamlConverter {
        fn format(&self) -> Format {
            Format::Yaml
        }

        fn extension(&self) -> &'static str {
            "yaml"
        }

        fn detect(&self, content: &str) -> FormatMatch {
            let trimmed = content.trim();
            // YAML document start
            if trimmed.starts_with("---") || trimmed.starts_with("%YAML") {
                return FormatMatch::Confident;
            }
            // YAML-specific patterns
            if trimmed.contains(": ") && !trimmed.starts_with('{') && !trimmed.contains(" = ") {
                return FormatMatch::Possible;
            }
            FormatMatch::NoMatch
        }

        fn parse(
            &self,
            content: &str,
            source: SourceId,
            path: Option<&Path>,
        ) -> ConfigResult<AnnotatedValue> {
            let v: serde_yaml_ng::Value = serde_yaml_ng::from_str(content).map_err(|e| {
                let loc = e.location().map(|l| {
                    path.map(|p| ParseLocation::from_path(p, l.line(), l.column()))
                        .unwrap_or_else(|| {
                            ParseLocation::new(source.as_str(), l.line(), l.column())
                        })
                });
                ConfigError::ParseError {
                    format: "YAML".into(),
                    message: e.to_string(),
                    location: loc,
                    source: Some(Box::new(e)),
                }
            })?;
            Ok(AnnotatedValue::new(
                yaml_to_config_value(&v, &source, ""),
                source,
                "",
            ))
        }

        fn serialize(&self, value: &AnnotatedValue) -> ConfigResult<String> {
            let yaml_val = yaml_value_from_config(&value.inner);
            serde_yaml_ng::to_string(&yaml_val).map_err(|e| ConfigError::InvalidValue {
                key: "serialization".to_string(),
                expected_type: "YAML".to_string(),
                message: format!("YAML serialization failed: {}", e),
            })
        }

        fn supports(&self, feature: FormatFeature) -> bool {
            match feature {
                FormatFeature::NestedMaps => true,
                FormatFeature::Arrays => true,
                FormatFeature::Comments => true,
                FormatFeature::InlineComments => true,
                FormatFeature::MultilineStrings => true,
                FormatFeature::Booleans => true,
                FormatFeature::Floats => true,
                FormatFeature::Null => true,
                FormatFeature::DateTime => true,
                FormatFeature::Binary => true,
                FormatFeature::TopLevelArrays => true,
                FormatFeature::Sections => true,
            }
        }
    }

    fn yaml_value_from_config(value: &ConfigValue) -> serde_yaml_ng::Value {
        match value {
            ConfigValue::Null => serde_yaml_ng::Value::Null,
            ConfigValue::Bool(b) => serde_yaml_ng::Value::Bool(*b),
            ConfigValue::I64(i) => serde_yaml_ng::Value::Number(serde_yaml_ng::Number::from(*i)),
            ConfigValue::U64(u) => {
                serde_yaml_ng::Value::Number(serde_yaml_ng::Number::from(*u as i64))
            }
            ConfigValue::F64(f) => serde_yaml_ng::Value::String(f.to_string()),
            ConfigValue::String(s) => serde_yaml_ng::Value::String(s.clone()),
            ConfigValue::Bytes(b) => {
                serde_yaml_ng::Value::String(format!("<binary: {} bytes>", b.len()))
            }
            ConfigValue::Array(arr) => serde_yaml_ng::Value::Sequence(
                arr.iter()
                    .map(|v| yaml_value_from_config(&v.inner))
                    .collect(),
            ),
            ConfigValue::Map(map) => serde_yaml_ng::Value::Mapping(
                map.iter()
                    .map(|(k, v)| {
                        (
                            serde_yaml_ng::Value::String(k.to_string()),
                            yaml_value_from_config(&v.inner),
                        )
                    })
                    .collect(),
            ),
        }
    }
}

// =============================================================================
// INI Converter
// =============================================================================

mod ini_converter {
    use super::*;
    use indexmap::IndexMap;
    use std::sync::Arc;

    pub struct IniConverter;

    impl IniConverter {
        pub fn new() -> Self {
            Self
        }
    }

    impl Default for IniConverter {
        fn default() -> Self {
            Self::new()
        }
    }

    impl FormatConverter for IniConverter {
        fn format(&self) -> Format {
            Format::Ini
        }

        fn extension(&self) -> &'static str {
            "ini"
        }

        fn detect(&self, content: &str) -> FormatMatch {
            let trimmed = content.trim();
            // INI: [section] headers and key=value pairs
            let has_section = trimmed.contains('[') && trimmed.contains(']');
            let has_key_value = trimmed.contains('=');

            if has_section && has_key_value {
                FormatMatch::Confident
            } else if has_key_value {
                FormatMatch::Possible
            } else {
                FormatMatch::NoMatch
            }
        }

        fn parse(
            &self,
            content: &str,
            source: SourceId,
            _path: Option<&Path>,
        ) -> ConfigResult<AnnotatedValue> {
            let mut sections: IndexMap<String, IndexMap<String, String>> = IndexMap::new();
            let mut cur = String::new();
            let mut invalid_lines = Vec::new();

            for (line_num, line) in content.lines().enumerate() {
                let l = line.trim();
                if l.is_empty() || l.starts_with('#') || l.starts_with(';') {
                    continue;
                }
                if l.starts_with('[') && l.ends_with(']') {
                    cur = l[1..l.len() - 1].trim().into();
                    sections.entry(cur.clone()).or_default();
                    continue;
                }
                if let Some(eq) = l.find('=') {
                    let key = l[..eq].trim();
                    let value = l[eq + 1..].trim();
                    if key.is_empty() {
                        invalid_lines.push((line_num + 1, line.to_string(), "empty key"));
                        continue;
                    }
                    sections
                        .entry(cur.clone())
                        .or_default()
                        .insert(key.into(), value.into());
                    continue;
                }
                invalid_lines.push((line_num + 1, line.to_string(), "invalid INI syntax"));
            }

            if !invalid_lines.is_empty() {
                // INI parsing issues found but continuing with valid lines only
            }

            let mut entries: Vec<(Arc<str>, AnnotatedValue)> = Vec::new();
            for (sec, keys) in sections.iter() {
                for (k, v) in keys.iter() {
                    let key = if sec.is_empty() {
                        k.clone()
                    } else {
                        format!("{}.{}", sec, k)
                    };
                    entries.push((
                        Arc::from(key.clone()),
                        AnnotatedValue::new(ConfigValue::String(v.clone()), source.clone(), key),
                    ));
                }
            }

            Ok(AnnotatedValue::new(
                ConfigValue::Map(Arc::new(entries.into_iter().collect::<IndexMap<_, _>>())),
                source,
                "",
            ))
        }

        fn serialize(&self, value: &AnnotatedValue) -> ConfigResult<String> {
            let map = value
                .inner
                .as_map()
                .ok_or_else(|| ConfigError::InvalidValue {
                    key: "serialization".to_string(),
                    expected_type: "INI".to_string(),
                    message: "INI serialization requires a map value".to_string(),
                })?;

            let mut output = String::new();
            let mut current_section = String::new();

            for (k, v) in map.iter() {
                let key = k.to_string();
                // Determine section from key (e.g., "database.host" → section="database")
                let (section, simple_key) = if let Some(dot) = key.find('.') {
                    (key[..dot].to_string(), key[dot + 1..].to_string())
                } else {
                    (String::new(), key)
                };

                if section != current_section {
                    if !current_section.is_empty() {
                        output.push('\n');
                    }
                    if !section.is_empty() {
                        output.push('[');
                        output.push_str(&section);
                        output.push_str("]\n");
                    }
                    current_section = section;
                }

                output.push_str(&simple_key);
                output.push_str(" = ");
                if let Some(s) = v.inner.as_str() {
                    output.push_str(s);
                } else {
                    output.push_str(&format!("{:?}", v.inner));
                }
                output.push('\n');
            }

            Ok(output)
        }

        fn supports(&self, feature: FormatFeature) -> bool {
            match feature {
                FormatFeature::NestedMaps => false, // Limited (only one level via sections)
                FormatFeature::Arrays => false,
                FormatFeature::Comments => true,
                FormatFeature::InlineComments => false,
                FormatFeature::MultilineStrings => false,
                FormatFeature::Booleans => true, // via "yes"/"no"/"true"/"false" strings
                FormatFeature::Floats => true,
                FormatFeature::Null => false,
                FormatFeature::DateTime => false,
                FormatFeature::Binary => false,
                FormatFeature::TopLevelArrays => false,
                FormatFeature::Sections => true,
            }
        }
    }
}

// =============================================================================
// Format Registry
// =============================================================================

/// Get all available format converters.
#[allow(clippy::vec_init_then_push)]
pub fn all_converters() -> Vec<Box<dyn FormatConverter>> {
    let mut converters: Vec<Box<dyn FormatConverter>> = vec![];

    #[cfg(feature = "toml")]
    converters.push(Box::new(toml_converter::TomlConverter::new()));
    #[cfg(feature = "json")]
    converters.push(Box::new(json_converter::JsonConverter::new()));
    #[cfg(feature = "yaml")]
    converters.push(Box::new(yaml_converter::YamlConverter::new()));
    // INI is always available (no feature flag)
    converters.push(Box::new(ini_converter::IniConverter::new()));

    converters
}

/// Detect the format of content by trying all converters.
///
/// Returns the `Format` with the highest confidence, or `None` if unknown.
pub fn detect_format(content: &str) -> Option<Format> {
    let trimmed = content.trim();
    let mut best_match = (FormatMatch::NoMatch, None);

    for converter in all_converters() {
        let confidence = converter.detect(trimmed);
        if confidence == FormatMatch::Confident {
            return Some(converter.format());
        }
        if confidence == FormatMatch::Possible && best_match.0 != FormatMatch::Confident {
            best_match = (confidence, Some(converter.format()));
        }
    }

    best_match.1
}

/// Get a converter for a specific format.
pub fn converter_for(format: Format) -> Option<Box<dyn FormatConverter>> {
    match format {
        #[cfg(feature = "toml")]
        Format::Toml => Some(Box::new(toml_converter::TomlConverter::new())),
        #[cfg(not(feature = "toml"))]
        Format::Toml => None,
        #[cfg(feature = "json")]
        Format::Json => Some(Box::new(json_converter::JsonConverter::new())),
        #[cfg(not(feature = "json"))]
        Format::Json => None,
        #[cfg(feature = "yaml")]
        Format::Yaml => Some(Box::new(yaml_converter::YamlConverter::new())),
        #[cfg(not(feature = "yaml"))]
        Format::Yaml => None,
        Format::Ini => Some(Box::new(ini_converter::IniConverter::new())),
    }
}

// =============================================================================
// Re-export Format from loader for compatibility
// =============================================================================

pub use crate::impl_::loader::Format;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_display() {
        assert_eq!(Format::Toml.to_string(), "TOML");
        assert_eq!(Format::Json.to_string(), "JSON");
        assert_eq!(Format::Yaml.to_string(), "YAML");
        assert_eq!(Format::Ini.to_string(), "INI");
    }

    #[cfg(feature = "json")]
    #[test]
    fn test_json_converter_detect() {
        let conv = json_converter::JsonConverter::new();
        assert_eq!(conv.detect(r#"{"key":"value"}"#), FormatMatch::Confident);
        assert_eq!(conv.detect(r#"[1,2,3]"#), FormatMatch::Confident);
        assert_eq!(conv.detect(r#"key = "value""#), FormatMatch::NoMatch);
    }

    #[cfg(feature = "toml")]
    #[test]
    fn test_toml_converter_detect() {
        let conv = toml_converter::TomlConverter::new();
        assert_eq!(conv.detect(r#"key = "value""#), FormatMatch::Confident);
        assert_eq!(
            conv.detect(
                r#"[section]
key = "value""#
            ),
            FormatMatch::Confident
        );
        assert_eq!(conv.detect(r#"{"key":"value"}"#), FormatMatch::NoMatch);
    }

    #[cfg(feature = "yaml")]
    #[test]
    fn test_yaml_converter_detect() {
        let conv = yaml_converter::YamlConverter::new();
        assert_eq!(conv.detect("key: value"), FormatMatch::Possible);
        assert_eq!(conv.detect("---\nkey: value"), FormatMatch::Confident);
    }

    #[test]
    fn test_ini_converter_detect() {
        let conv = ini_converter::IniConverter::new();
        assert_eq!(conv.detect("[section]\nkey=value"), FormatMatch::Confident);
        assert_eq!(conv.detect("key=value"), FormatMatch::Possible);
    }

    #[cfg(feature = "json")]
    #[test]
    fn test_json_converter_parse() {
        let conv = json_converter::JsonConverter::new();
        let result = conv.parse(
            r#"{"name":"test","port":8080}"#,
            SourceId::new("test"),
            None,
        );
        assert!(result.is_ok(), "{:?}", result.err());
        let val = result.unwrap();
        assert!(val.is_map());
    }

    #[cfg(feature = "json")]
    #[test]
    fn test_json_converter_serialize() {
        use crate::types::ConfigValue;
        let conv = json_converter::JsonConverter::new();
        let val = AnnotatedValue::new(
            ConfigValue::map(vec![(
                "name".to_string(),
                AnnotatedValue::new(
                    ConfigValue::String("test".into()),
                    SourceId::new("test"),
                    "name",
                ),
            )]),
            SourceId::new("test"),
            "",
        );
        let result = conv.serialize(&val);
        assert!(result.is_ok(), "{:?}", result.err());
        let s = result.unwrap();
        assert!(s.contains("name"));
        assert!(s.contains("test"));
    }

    #[cfg(feature = "json")]
    #[test]
    fn test_detect_format_json() {
        assert_eq!(detect_format(r#"{"key":"value"}"#), Some(Format::Json));
    }

    #[test]
    fn test_detect_format_ini() {
        assert_eq!(detect_format("[section]\nkey=value"), Some(Format::Ini));
    }

    #[test]
    fn test_converter_for() {
        #[cfg(feature = "json")]
        {
            let conv = converter_for(Format::Json);
            assert!(conv.is_some());
            assert_eq!(conv.unwrap().format(), Format::Json);
        }
        let conv = converter_for(Format::Ini);
        assert!(conv.is_some());
        assert_eq!(conv.unwrap().format(), Format::Ini);
    }

    #[test]
    fn test_format_features() {
        let ini = ini_converter::IniConverter::new();
        assert!(!ini.supports(FormatFeature::NestedMaps));
        assert!(ini.supports(FormatFeature::Sections));
    }

    #[test]
    fn test_all_converters_not_empty() {
        let converters = all_converters();
        assert!(!converters.is_empty());
    }

    #[cfg(feature = "toml")]
    #[test]
    fn test_converter_for_toml() {
        let conv = converter_for(Format::Toml);
        assert!(conv.is_some());
        assert_eq!(conv.unwrap().format(), Format::Toml);
    }

    #[cfg(feature = "json")]
    #[test]
    fn test_converter_for_json() {
        let conv = converter_for(Format::Json);
        assert!(conv.is_some());
        assert_eq!(conv.unwrap().format(), Format::Json);
    }

    #[cfg(all(feature = "json", feature = "toml", feature = "yaml"))]
    #[test]
    fn test_format_feature_nested_maps() {
        let toml = toml_converter::TomlConverter::new();
        assert!(toml.supports(FormatFeature::NestedMaps));
        assert!(toml.supports(FormatFeature::Sections));
    }

    #[test]
    fn test_format_match_debug() {
        let m = FormatMatch::Confident;
        assert!(!format!("{:?}", m).is_empty());
    }

    #[test]
    fn test_format_match_partial() {
        let m = FormatMatch::Possible;
        assert!(!format!("{:?}", m).is_empty());
    }

    #[test]
    fn test_format_feature_debug() {
        let f = FormatFeature::Sections;
        assert!(!format!("{:?}", f).is_empty());
    }

    #[cfg(feature = "toml")]
    #[test]
    fn test_toml_converter_extension() {
        let c = toml_converter::TomlConverter::new();
        assert_eq!(c.extension(), "toml");
    }

    #[cfg(feature = "json")]
    #[test]
    fn test_json_converter_extension() {
        let c = json_converter::JsonConverter::new();
        assert_eq!(c.extension(), "json");
    }

    #[cfg(feature = "yaml")]
    #[test]
    fn test_yaml_converter_extension() {
        let c = yaml_converter::YamlConverter::new();
        assert!(!c.extension().is_empty());
    }

    #[test]
    fn test_ini_converter_extension() {
        let c = ini_converter::IniConverter::new();
        assert_eq!(c.extension(), "ini");
    }

    #[cfg(feature = "yaml")]
    #[test]
    fn test_converter_for_yaml() {
        let c = converter_for(Format::Yaml);
        assert!(c.is_some());
        assert_eq!(c.unwrap().format(), Format::Yaml);
    }

    #[test]
    fn test_converter_for_ini() {
        let c = converter_for(Format::Ini);
        assert!(c.is_some());
        assert_eq!(c.unwrap().format(), Format::Ini);
    }

    #[test]
    fn test_format_match_no_match() {
        let m = FormatMatch::NoMatch;
        assert!(!format!("{:?}", m).is_empty());
    }

    #[cfg(feature = "toml")]
    #[test]
    fn test_toml_converter_parse_serialize_roundtrip() {
        use crate::types::SourceId;
        let c = toml_converter::TomlConverter::new();
        let parsed = c
            .parse("name = \"test\"", SourceId::new("test"), None)
            .unwrap();
        assert!(parsed.is_map());
        let serialized = c.serialize(&parsed).unwrap();
        assert!(serialized.contains("test"));
    }

    #[cfg(feature = "json")]
    #[test]
    fn test_json_converter_parse_serialize_roundtrip() {
        use crate::types::SourceId;
        let c = json_converter::JsonConverter::new();
        let parsed = c
            .parse("{\"name\":\"test\"}", SourceId::new("test"), None)
            .unwrap();
        assert!(parsed.is_map());
        let serialized = c.serialize(&parsed).unwrap();
        assert!(serialized.contains("test"));
    }

    #[cfg(feature = "yaml")]
    #[test]
    fn test_yaml_converter_parse_serialize() {
        use crate::types::SourceId;
        let c = yaml_converter::YamlConverter::new();
        let parsed = c.parse("name: test", SourceId::new("test"), None).unwrap();
        assert!(parsed.is_map());
        let serialized = c.serialize(&parsed).unwrap();
        assert!(serialized.contains("test"));
    }

    #[test]
    fn test_ini_converter_parse_serialize() {
        use crate::types::SourceId;
        let c = ini_converter::IniConverter::new();
        let parsed = c
            .parse("[section]\nkey=value", SourceId::new("test"), None)
            .unwrap();
        assert!(parsed.is_map());
        let serialized = c.serialize(&parsed).unwrap();
        assert!(serialized.contains("section"));
    }

    #[test]
    fn test_format_match_equality() {
        assert_eq!(FormatMatch::Confident, FormatMatch::Confident);
        assert_eq!(FormatMatch::Possible, FormatMatch::Possible);
        assert_eq!(FormatMatch::NoMatch, FormatMatch::NoMatch);
        assert_ne!(FormatMatch::Confident, FormatMatch::NoMatch);
        assert_ne!(FormatMatch::Possible, FormatMatch::Confident);
    }

    #[test]
    fn test_format_match_clone_copy() {
        let m = FormatMatch::Confident;
        let copied = m;
        let cloned = m;
        assert_eq!(m, copied);
        assert_eq!(m, cloned);
    }

    #[test]
    fn test_format_feature_all_variants_debug() {
        assert!(!format!("{:?}", FormatFeature::NestedMaps).is_empty());
        assert!(!format!("{:?}", FormatFeature::Arrays).is_empty());
        assert!(!format!("{:?}", FormatFeature::Comments).is_empty());
        assert!(!format!("{:?}", FormatFeature::InlineComments).is_empty());
        assert!(!format!("{:?}", FormatFeature::MultilineStrings).is_empty());
        assert!(!format!("{:?}", FormatFeature::Booleans).is_empty());
        assert!(!format!("{:?}", FormatFeature::Floats).is_empty());
        assert!(!format!("{:?}", FormatFeature::Null).is_empty());
        assert!(!format!("{:?}", FormatFeature::DateTime).is_empty());
        assert!(!format!("{:?}", FormatFeature::Binary).is_empty());
        assert!(!format!("{:?}", FormatFeature::TopLevelArrays).is_empty());
        assert!(!format!("{:?}", FormatFeature::Sections).is_empty());
    }

    #[test]
    fn test_format_feature_equality_and_clone() {
        assert_eq!(FormatFeature::Arrays, FormatFeature::Arrays);
        assert_ne!(FormatFeature::Arrays, FormatFeature::Null);
        let f = FormatFeature::Booleans;
        assert_eq!(f, f.clone());
    }

    #[cfg(feature = "toml")]
    #[test]
    fn test_toml_converter_detect_possible_and_no_match() {
        let conv = toml_converter::TomlConverter::new();
        // Has " = " but also ": " → not confident, falls to Possible
        assert_eq!(conv.detect("key = value: something"), FormatMatch::Possible);
        // No " = " → NoMatch
        assert_eq!(conv.detect("just plain text"), FormatMatch::NoMatch);
        // JSON-like array → NoMatch
        assert_eq!(conv.detect("[1, 2, 3]"), FormatMatch::NoMatch);
        // YAML doc → NoMatch (no " = ")
        assert_eq!(conv.detect("---\nkey: value"), FormatMatch::NoMatch);
    }

    #[cfg(feature = "toml")]
    #[test]
    fn test_toml_converter_parse_error() {
        let conv = toml_converter::TomlConverter::new();
        let result = conv.parse("[invalid", SourceId::new("t"), None);
        assert!(result.is_err());
    }

    #[cfg(feature = "toml")]
    #[test]
    fn test_toml_converter_default() {
        let conv = toml_converter::TomlConverter::new();
        assert_eq!(conv.format(), Format::Toml);
        assert_eq!(conv.extension(), "toml");
    }

    #[cfg(feature = "toml")]
    #[test]
    fn test_toml_converter_supports_all_features() {
        let conv = toml_converter::TomlConverter::new();
        assert!(conv.supports(FormatFeature::NestedMaps));
        assert!(conv.supports(FormatFeature::Arrays));
        assert!(conv.supports(FormatFeature::Comments));
        assert!(conv.supports(FormatFeature::InlineComments));
        assert!(conv.supports(FormatFeature::MultilineStrings));
        assert!(conv.supports(FormatFeature::Booleans));
        assert!(conv.supports(FormatFeature::Floats));
        assert!(!conv.supports(FormatFeature::Null));
        assert!(conv.supports(FormatFeature::DateTime));
        assert!(conv.supports(FormatFeature::Binary));
        assert!(!conv.supports(FormatFeature::TopLevelArrays));
        assert!(conv.supports(FormatFeature::Sections));
    }

    #[cfg(feature = "toml")]
    #[test]
    fn test_toml_serialize_scalar_types_require_map() {
        use crate::types::ConfigValue;
        let conv = toml_converter::TomlConverter::new();
        // TOML format requires a top-level table (Map); bare scalar values
        // cannot be serialized as a standalone TOML document, so all of
        // these must return Err.
        let cases: Vec<ConfigValue> = vec![
            ConfigValue::Null,
            ConfigValue::Bool(false),
            ConfigValue::I64(-7),
            ConfigValue::U64(99),
            ConfigValue::F64(1.5),
            ConfigValue::Bytes(vec![10, 20]),
        ];
        for cv in cases {
            let v = AnnotatedValue::new(cv, SourceId::new("t"), "");
            assert!(
                conv.serialize(&v).is_err(),
                "TOML should reject top-level scalar {:?}",
                v.inner
            );
        }
    }

    #[cfg(feature = "toml")]
    #[test]
    fn test_toml_serialize_array_in_map() {
        use crate::types::ConfigValue;
        let conv = toml_converter::TomlConverter::new();
        let v = AnnotatedValue::new(
            ConfigValue::map(vec![(
                "items".to_string(),
                AnnotatedValue::new(
                    ConfigValue::Array(
                        vec![
                            AnnotatedValue::new(ConfigValue::I64(1), SourceId::new("t"), ""),
                            AnnotatedValue::new(ConfigValue::I64(2), SourceId::new("t"), ""),
                        ]
                        .into(),
                    ),
                    SourceId::new("t"),
                    "items",
                ),
            )]),
            SourceId::new("t"),
            "",
        );
        let result = conv.serialize(&v);
        assert!(result.is_ok(), "{:?}", result.err());
        assert!(result.unwrap().contains("items"));
    }

    #[cfg(feature = "json")]
    #[test]
    fn test_json_converter_detect_no_match() {
        let conv = json_converter::JsonConverter::new();
        // Starts with { but invalid JSON
        assert_eq!(conv.detect("{invalid json}"), FormatMatch::NoMatch);
        // Doesn't start with { or [
        assert_eq!(conv.detect("plain text"), FormatMatch::NoMatch);
        // Empty string
        assert_eq!(conv.detect(""), FormatMatch::NoMatch);
    }

    #[cfg(feature = "json")]
    #[test]
    fn test_json_converter_parse_error() {
        let conv = json_converter::JsonConverter::new();
        let result = conv.parse("{invalid}", SourceId::new("t"), None);
        assert!(result.is_err());
    }

    #[cfg(feature = "json")]
    #[test]
    fn test_json_converter_default() {
        let conv = json_converter::JsonConverter::new();
        assert_eq!(conv.format(), Format::Json);
        assert_eq!(conv.extension(), "json");
    }

    #[cfg(feature = "json")]
    #[test]
    fn test_json_converter_supports_all_features() {
        let conv = json_converter::JsonConverter::new();
        assert!(conv.supports(FormatFeature::NestedMaps));
        assert!(conv.supports(FormatFeature::Arrays));
        assert!(!conv.supports(FormatFeature::Comments));
        assert!(!conv.supports(FormatFeature::InlineComments));
        assert!(conv.supports(FormatFeature::MultilineStrings));
        assert!(conv.supports(FormatFeature::Booleans));
        assert!(conv.supports(FormatFeature::Floats));
        assert!(conv.supports(FormatFeature::Null));
        assert!(conv.supports(FormatFeature::DateTime));
        assert!(!conv.supports(FormatFeature::Binary));
        assert!(conv.supports(FormatFeature::TopLevelArrays));
        assert!(!conv.supports(FormatFeature::Sections));
    }

    #[cfg(feature = "json")]
    #[test]
    fn test_json_serialize_various_types() {
        use crate::types::ConfigValue;
        let conv = json_converter::JsonConverter::new();
        // Null
        assert!(conv
            .serialize(&AnnotatedValue::new(
                ConfigValue::Null,
                SourceId::new("t"),
                ""
            ))
            .is_ok());
        // Bool
        assert!(conv
            .serialize(&AnnotatedValue::new(
                ConfigValue::Bool(true),
                SourceId::new("t"),
                ""
            ))
            .is_ok());
        // I64 / U64
        assert!(conv
            .serialize(&AnnotatedValue::new(
                ConfigValue::I64(42),
                SourceId::new("t"),
                ""
            ))
            .is_ok());
        assert!(conv
            .serialize(&AnnotatedValue::new(
                ConfigValue::U64(100),
                SourceId::new("t"),
                ""
            ))
            .is_ok());
        // F64
        assert!(conv
            .serialize(&AnnotatedValue::new(
                ConfigValue::F64(2.5),
                SourceId::new("t"),
                ""
            ))
            .is_ok());
        // String
        assert!(conv
            .serialize(&AnnotatedValue::new(
                ConfigValue::String("hello".into()),
                SourceId::new("t"),
                ""
            ))
            .is_ok());
        // Bytes → base64-encoded string
        let r = conv.serialize(&AnnotatedValue::new(
            ConfigValue::Bytes(vec![1, 2, 3]),
            SourceId::new("t"),
            "",
        ));
        assert!(r.is_ok());
        assert!(r.unwrap().contains('"'));
        // Array
        let v = AnnotatedValue::new(
            ConfigValue::Array(
                vec![AnnotatedValue::new(
                    ConfigValue::I64(1),
                    SourceId::new("t"),
                    "",
                )]
                .into(),
            ),
            SourceId::new("t"),
            "",
        );
        assert!(conv.serialize(&v).is_ok());
    }

    #[cfg(feature = "yaml")]
    #[test]
    fn test_yaml_converter_detect_full() {
        let conv = yaml_converter::YamlConverter::new();
        // %YAML marker → Confident
        assert_eq!(conv.detect("%YAML 1.1"), FormatMatch::Confident);
        // --- document start → Confident
        assert_eq!(conv.detect("---\nkey: value"), FormatMatch::Confident);
        // key: value → Possible
        assert_eq!(conv.detect("key: value"), FormatMatch::Possible);
        // Plain text → NoMatch
        assert_eq!(conv.detect("just text"), FormatMatch::NoMatch);
        // JSON object → NoMatch
        assert_eq!(conv.detect(r#"{"k":"v"}"#), FormatMatch::NoMatch);
        // TOML key = value → NoMatch (contains " = ")
        assert_eq!(conv.detect(r#"key = "value""#), FormatMatch::NoMatch);
    }

    #[cfg(feature = "yaml")]
    #[test]
    fn test_yaml_converter_parse_error() {
        let conv = yaml_converter::YamlConverter::new();
        // Unclosed flow mapping
        let result = conv.parse("{a: b", SourceId::new("t"), None);
        assert!(result.is_err());
    }

    #[cfg(feature = "yaml")]
    #[test]
    fn test_yaml_converter_default() {
        let conv = yaml_converter::YamlConverter::new();
        assert_eq!(conv.format(), Format::Yaml);
        assert_eq!(conv.extension(), "yaml");
    }

    #[cfg(feature = "yaml")]
    #[test]
    fn test_yaml_converter_supports_all_features() {
        let conv = yaml_converter::YamlConverter::new();
        assert!(conv.supports(FormatFeature::NestedMaps));
        assert!(conv.supports(FormatFeature::Arrays));
        assert!(conv.supports(FormatFeature::Comments));
        assert!(conv.supports(FormatFeature::InlineComments));
        assert!(conv.supports(FormatFeature::MultilineStrings));
        assert!(conv.supports(FormatFeature::Booleans));
        assert!(conv.supports(FormatFeature::Floats));
        assert!(conv.supports(FormatFeature::Null));
        assert!(conv.supports(FormatFeature::DateTime));
        assert!(conv.supports(FormatFeature::Binary));
        assert!(conv.supports(FormatFeature::TopLevelArrays));
        assert!(conv.supports(FormatFeature::Sections));
    }

    #[cfg(feature = "yaml")]
    #[test]
    fn test_yaml_serialize_various_types() {
        use crate::types::ConfigValue;
        let conv = yaml_converter::YamlConverter::new();
        // F64 (converted to string in yaml_value_from_config)
        let v = AnnotatedValue::new(ConfigValue::F64(2.5), SourceId::new("t"), "");
        let s = conv.serialize(&v).unwrap();
        assert!(s.contains("2.5"));
        // U64
        assert!(conv
            .serialize(&AnnotatedValue::new(
                ConfigValue::U64(8),
                SourceId::new("t"),
                ""
            ))
            .is_ok());
        // Bytes → "<binary: N bytes>"
        let v = AnnotatedValue::new(ConfigValue::Bytes(vec![10, 20]), SourceId::new("t"), "");
        let s = conv.serialize(&v).unwrap();
        assert!(s.contains("binary"));
        // Null
        assert!(conv
            .serialize(&AnnotatedValue::new(
                ConfigValue::Null,
                SourceId::new("t"),
                ""
            ))
            .is_ok());
        // Array
        let v = AnnotatedValue::new(
            ConfigValue::Array(
                vec![
                    AnnotatedValue::new(ConfigValue::I64(1), SourceId::new("t"), ""),
                    AnnotatedValue::new(ConfigValue::I64(2), SourceId::new("t"), ""),
                ]
                .into(),
            ),
            SourceId::new("t"),
            "",
        );
        assert!(conv.serialize(&v).is_ok());
    }

    #[test]
    fn test_ini_converter_detect_full() {
        let conv = ini_converter::IniConverter::new();
        // NoMatch: no = and no section
        assert_eq!(conv.detect("just text"), FormatMatch::NoMatch);
        // Possible: only key=value
        assert_eq!(conv.detect("key=value"), FormatMatch::Possible);
        // Confident: section + key=value
        assert_eq!(conv.detect("[section]\nkey=value"), FormatMatch::Confident);
    }

    #[test]
    fn test_ini_converter_default() {
        let conv = ini_converter::IniConverter::new();
        assert_eq!(conv.format(), Format::Ini);
        assert_eq!(conv.extension(), "ini");
    }

    #[test]
    fn test_ini_converter_supports_all_features() {
        let conv = ini_converter::IniConverter::new();
        assert!(!conv.supports(FormatFeature::NestedMaps));
        assert!(!conv.supports(FormatFeature::Arrays));
        assert!(conv.supports(FormatFeature::Comments));
        assert!(!conv.supports(FormatFeature::InlineComments));
        assert!(!conv.supports(FormatFeature::MultilineStrings));
        assert!(conv.supports(FormatFeature::Booleans));
        assert!(conv.supports(FormatFeature::Floats));
        assert!(!conv.supports(FormatFeature::Null));
        assert!(!conv.supports(FormatFeature::DateTime));
        assert!(!conv.supports(FormatFeature::Binary));
        assert!(!conv.supports(FormatFeature::TopLevelArrays));
        assert!(conv.supports(FormatFeature::Sections));
    }

    #[test]
    fn test_ini_converter_parse_with_comments_and_invalid_lines() {
        let conv = ini_converter::IniConverter::new();
        let content = "; ini comment\n# hash comment\n[db]\nhost = localhost\n=value\nbadline\n";
        let result = conv.parse(content, SourceId::new("t"), None);
        assert!(result.is_ok(), "{:?}", result.err());
        assert!(result.unwrap().is_map());
    }

    #[test]
    fn test_ini_converter_parse_empty_section() {
        let conv = ini_converter::IniConverter::new();
        let result = conv.parse("[empty]", SourceId::new("t"), None);
        assert!(result.is_ok());
        let val = result.unwrap();
        assert!(val.is_map());
    }

    #[test]
    fn test_ini_converter_parse_no_section_key_value() {
        let conv = ini_converter::IniConverter::new();
        let result = conv.parse("key=value", SourceId::new("t"), None);
        assert!(result.is_ok());
        let val = result.unwrap();
        assert!(val.is_map());
    }

    #[test]
    fn test_ini_converter_serialize_non_map_error() {
        use crate::types::ConfigValue;
        let conv = ini_converter::IniConverter::new();
        let v = AnnotatedValue::new(
            ConfigValue::String("not a map".into()),
            SourceId::new("t"),
            "",
        );
        let result = conv.serialize(&v);
        assert!(result.is_err());
    }

    #[cfg(feature = "toml")]
    #[test]
    fn test_detect_format_toml_via_function() {
        assert_eq!(detect_format(r#"key = "value""#), Some(Format::Toml));
    }

    #[cfg(feature = "yaml")]
    #[test]
    fn test_detect_format_yaml_possible() {
        assert_eq!(detect_format("key: value"), Some(Format::Yaml));
    }

    #[test]
    fn test_detect_format_returns_none_for_plain_text() {
        assert_eq!(detect_format("just some plain text with no markers"), None);
    }

    #[test]
    fn test_detect_format_returns_none_for_empty() {
        assert_eq!(detect_format(""), None);
        assert_eq!(detect_format("   \n  \t "), None);
    }

    #[test]
    fn test_all_converters_count_matches_features() {
        let converters = all_converters();
        let expected = 1 // ini always available
            + if cfg!(feature = "toml") { 1 } else { 0 }
            + if cfg!(feature = "json") { 1 } else { 0 }
            + if cfg!(feature = "yaml") { 1 } else { 0 };
        assert_eq!(converters.len(), expected);
    }

    #[test]
    fn test_all_converters_extensions_unique() {
        let converters = all_converters();
        let mut exts: Vec<&str> = converters.iter().map(|c| c.extension()).collect();
        exts.sort();
        let total = exts.len();
        exts.dedup();
        assert_eq!(exts.len(), total, "duplicate extensions found");
    }

    #[test]
    fn test_converter_for_all_formats_present() {
        for format in Format::all() {
            // Skip formats whose converters are not built (feature is off)
            if *format == Format::Toml && !cfg!(feature = "toml") {
                continue;
            }
            if *format == Format::Json && !cfg!(feature = "json") {
                continue;
            }
            if *format == Format::Yaml && !cfg!(feature = "yaml") {
                continue;
            }
            let conv = converter_for(*format);
            assert!(conv.is_some(), "converter for {:?} should exist", format);
            assert_eq!(conv.unwrap().format(), *format);
        }
    }
}
