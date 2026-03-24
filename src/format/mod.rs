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
//! let content = r#"{"name":"test","port":8080}"#;
//! let converter = converter_for(Format::Json).expect("JSON converter should exist");
//! let result = converter.parse(content, confers::value::SourceId::new("test"), None);
//! assert!(result.is_ok());
//! ```

use crate::error::{ConfigError, ConfigResult};
use crate::value::{AnnotatedValue, ConfigValue, SourceId};
use std::path::Path;

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

    fn toml_table_to_config_value(
        table: &toml::Table,
        source: &SourceId,
        prefix: &str,
    ) -> ConfigValue {
        use indexmap::IndexMap;
        use std::sync::Arc;
        let entries: Vec<(Arc<str>, AnnotatedValue)> = table
            .iter()
            .map(|(k, v)| {
                let path = if prefix.is_empty() {
                    k.clone()
                } else {
                    format!("{}.{}", prefix, k)
                };
                (
                    Arc::from(path.clone()),
                    AnnotatedValue::new(
                        toml_value_to_config_value(v, source, &path),
                        source.clone(),
                        k.clone(),
                    ),
                )
            })
            .collect();
        ConfigValue::Map(Arc::new(entries.into_iter().collect::<IndexMap<_, _>>()))
    }

    fn toml_value_to_config_value(
        value: &toml::Value,
        source: &SourceId,
        prefix: &str,
    ) -> ConfigValue {
        match value {
            toml::Value::String(s) => ConfigValue::String(s.clone()),
            toml::Value::Integer(i) => ConfigValue::I64(*i),
            toml::Value::Float(f) => ConfigValue::F64(*f),
            toml::Value::Boolean(b) => ConfigValue::Bool(*b),
            toml::Value::Datetime(dt) => ConfigValue::String(dt.to_string()),
            toml::Value::Array(arr) => ConfigValue::Array(
                arr.iter()
                    .enumerate()
                    .map(|(i, v)| {
                        let path = format!("{}.{}", prefix, i);
                        AnnotatedValue::new(
                            toml_value_to_config_value(v, source, &path),
                            source.clone(),
                            path,
                        )
                    })
                    .collect::<Vec<_>>()
                    .into(),
            ),
            toml::Value::Table(t) => toml_table_to_config_value(t, source, prefix),
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

    fn json_to_config_value(v: &serde_json::Value, src: &SourceId, pre: &str) -> ConfigValue {
        use indexmap::IndexMap;
        use std::sync::Arc;
        match v {
            serde_json::Value::Null => ConfigValue::Null,
            serde_json::Value::Bool(b) => ConfigValue::Bool(*b),
            serde_json::Value::Number(n) => n
                .as_i64()
                .map(ConfigValue::I64)
                .or_else(|| n.as_u64().map(ConfigValue::U64))
                .or_else(|| n.as_f64().map(ConfigValue::F64))
                .unwrap_or(ConfigValue::Null),
            serde_json::Value::String(s) => ConfigValue::String(s.clone()),
            serde_json::Value::Array(a) => ConfigValue::Array(
                a.iter()
                    .enumerate()
                    .map(|(i, v)| {
                        let p = format!("{}.{}", pre, i);
                        AnnotatedValue::new(json_to_config_value(v, src, &p), src.clone(), p)
                    })
                    .collect::<Vec<_>>()
                    .into(),
            ),
            serde_json::Value::Object(o) => ConfigValue::Map(Arc::new(
                o.iter()
                    .map(|(k, v)| {
                        let p = if pre.is_empty() {
                            k.clone()
                        } else {
                            format!("{}.{}", pre, k)
                        };
                        (
                            Arc::from(p.clone()),
                            AnnotatedValue::new(
                                json_to_config_value(v, src, &p),
                                src.clone(),
                                k.clone(),
                            ),
                        )
                    })
                    .collect::<IndexMap<_, _>>(),
            )),
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

    fn yaml_to_config_value(v: &serde_yaml_ng::Value, src: &SourceId, pre: &str) -> ConfigValue {
        use indexmap::IndexMap;
        use std::sync::Arc;
        match v {
            serde_yaml_ng::Value::Null => ConfigValue::Null,
            serde_yaml_ng::Value::Bool(b) => ConfigValue::Bool(*b),
            serde_yaml_ng::Value::Number(n) => n
                .as_i64()
                .map(ConfigValue::I64)
                .or_else(|| n.as_u64().map(ConfigValue::U64))
                .or_else(|| n.as_f64().map(ConfigValue::F64))
                .unwrap_or(ConfigValue::Null),
            serde_yaml_ng::Value::String(s) => ConfigValue::String(s.clone()),
            serde_yaml_ng::Value::Sequence(s) => ConfigValue::Array(
                s.iter()
                    .enumerate()
                    .map(|(i, v)| {
                        let p = format!("{}.{}", pre, i);
                        AnnotatedValue::new(yaml_to_config_value(v, src, &p), src.clone(), p)
                    })
                    .collect::<Vec<_>>()
                    .into(),
            ),
            serde_yaml_ng::Value::Mapping(m) => ConfigValue::Map(
                m.iter()
                    .filter_map(|(k, v)| {
                        k.as_str().map(|key| {
                            let p = if pre.is_empty() {
                                key.to_string()
                            } else {
                                format!("{}.{}", pre, key)
                            };
                            (
                                Arc::from(p.clone()),
                                AnnotatedValue::new(
                                    yaml_to_config_value(v, src, &p),
                                    src.clone(),
                                    key,
                                ),
                            )
                        })
                    })
                    .collect::<IndexMap<_, _>>()
                    .into(),
            ),
            serde_yaml_ng::Value::Tagged(t) => yaml_to_config_value(&t.value, src, pre),
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
                tracing::warn!(
                    "INI parsing found {} potentially invalid line(s): {:?}",
                    invalid_lines.len(),
                    invalid_lines.iter().take(5).collect::<Vec<_>>()
                );
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
        #[cfg(feature = "json")]
        Format::Json => Some(Box::new(json_converter::JsonConverter::new())),
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

pub use crate::loader::Format;

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
        use crate::value::ConfigValue;
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
}
