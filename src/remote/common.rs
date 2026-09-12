// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Shared utilities for remote configuration sources.

// AnnotatedValue 仅被下方 toml/json/yaml 门控的解析函数与 etcd/consul 门控的
// 合并函数使用；confers/remote 被单独启用时（如 trait-kit presets-remote）两者皆关。
#[cfg(any(
    feature = "toml",
    feature = "json",
    feature = "yaml",
    feature = "etcd",
    feature = "consul"
))]
use crate::types::AnnotatedValue;

#[cfg(any(feature = "toml", feature = "json", feature = "yaml"))]
use crate::loader::{Format, detect_format_from_content};

#[cfg(any(feature = "toml", feature = "json", feature = "yaml"))]
use crate::types::SourceId;

#[cfg(any(feature = "etcd", feature = "consul"))]
use std::sync::Arc;

/// Try to parse a value with an explicit [`Format`], falling back to content
/// sniffing when `format` is `None`.
///
/// Used by sources that let the user pin a format for KV values (issue #334):
/// an explicit format always wins over sniffing. Returns `None` when the
/// content cannot be parsed as the chosen format (callers then treat the
/// value as a plain string).
#[cfg(any(feature = "toml", feature = "json", feature = "yaml"))]
pub(crate) fn try_parse_value_with_format(
    content: &str,
    format: Option<Format>,
    source_name: &str,
) -> Option<AnnotatedValue> {
    let format = format.or_else(|| detect_format_from_content(content))?;

    match format {
        #[cfg(feature = "toml")]
        Format::Toml => {
            let table: toml::Table = toml::from_str(content).ok()?;
            Some(crate::loader::parse_toml_table(
                &table,
                &SourceId::new(source_name),
                "",
            ))
        }
        #[cfg(feature = "json")]
        Format::Json => {
            let v: serde_json::Value = serde_json::from_str(content).ok()?;
            Some(crate::loader::parse_json_value(
                &v,
                &SourceId::new(source_name),
                "",
            ))
        }
        #[cfg(feature = "yaml")]
        Format::Yaml => {
            let v: serde_yaml_ng::Value = serde_yaml_ng::from_str(content).ok()?;
            Some(crate::loader::parse_yaml_value(
                &v,
                &SourceId::new(source_name),
                "",
            ))
        }
        #[cfg(not(feature = "toml"))]
        Format::Toml => None,
        #[cfg(not(feature = "json"))]
        Format::Json => None,
        #[cfg(not(feature = "yaml"))]
        Format::Yaml => None,
        Format::Ini => None,
    }
}

/// Merge a key-value pair into a config map.
#[cfg(any(feature = "etcd", feature = "consul"))]
pub(crate) fn merge_into_map(
    map: &mut indexmap::IndexMap<Arc<str>, AnnotatedValue>,
    key: &str,
    value: AnnotatedValue,
) {
    map.insert(Arc::from(key), value);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ConfigValue, SourceId};

    #[cfg(feature = "toml")]
    #[test]
    fn test_try_parse_value_toml() {
        let content = "key = \"value\"\n";
        let result = try_parse_value_with_format(content, None, "test_source");
        assert!(result.is_some());
        let val = result.unwrap();
        assert!(val.is_map());
    }

    #[cfg(feature = "json")]
    #[test]
    fn test_try_parse_value_json() {
        let content = "{\"key\": \"value\"}";
        let result = try_parse_value_with_format(content, None, "test_source");
        assert!(result.is_some());
        let val = result.unwrap();
        assert!(val.is_map());
    }

    #[cfg(feature = "yaml")]
    #[test]
    fn test_try_parse_value_yaml() {
        let content = "---\nkey: value\n";
        let result = try_parse_value_with_format(content, None, "test_source");
        assert!(result.is_some());
        let val = result.unwrap();
        assert!(val.is_map());
    }

    #[cfg(any(feature = "toml", feature = "json", feature = "yaml"))]
    #[test]
    fn test_try_parse_value_invalid_content() {
        // Content that does not match any known format pattern
        let content = "totally unrecognizable content @#$%";
        let result = try_parse_value_with_format(content, None, "test_source");
        assert!(result.is_none());
    }

    #[cfg(any(feature = "toml", feature = "json", feature = "yaml"))]
    #[test]
    fn test_try_parse_value_empty_content() {
        let result = try_parse_value_with_format("", None, "test_source");
        assert!(result.is_none());
    }

    #[cfg(any(feature = "toml", feature = "json", feature = "yaml"))]
    #[test]
    fn test_try_parse_value_whitespace_only() {
        let result = try_parse_value_with_format("   \n\t  ", None, "test_source");
        assert!(result.is_none());
    }

    #[cfg(feature = "toml")]
    #[test]
    fn test_try_parse_value_invalid_toml() {
        // Recognized as TOML (has " = ") but fails to parse
        let content = "key = = invalid";
        let result = try_parse_value_with_format(content, None, "test_source");
        assert!(result.is_none());
    }

    #[cfg(feature = "json")]
    #[test]
    fn test_try_parse_value_invalid_json() {
        // Recognized as JSON (starts with {, has quotes and colon) but fails to parse
        let content = "{\"key\": invalid_value}";
        let result = try_parse_value_with_format(content, None, "test_source");
        assert!(result.is_none());
    }

    /// Issue #334: an explicit format must be used instead of sniffing.
    #[cfg(feature = "json")]
    #[test]
    fn test_try_parse_value_with_explicit_format_json() {
        let content = "{\"key\": \"value\"}";
        let val = try_parse_value_with_format(content, Some(Format::Json), "test_source")
            .expect("explicit JSON format should parse JSON content");
        assert!(val.is_map());
    }

    /// Issue #334: the explicit format wins over content sniffing — content
    /// that sniffs as JSON is NOT parsed when TOML is pinned (parse failure
    /// yields None so callers keep the raw string).
    #[cfg(all(feature = "json", feature = "toml"))]
    #[test]
    fn test_try_parse_value_explicit_format_overrides_sniffing() {
        let content = "{\"key\": \"value\"}";
        let result = try_parse_value_with_format(content, Some(Format::Toml), "test_source");
        assert!(
            result.is_none(),
            "JSON content must not parse as the pinned TOML format"
        );
    }

    /// `None` falls back to content sniffing (the previous default behavior).
    #[cfg(feature = "json")]
    #[test]
    fn test_try_parse_value_with_format_none_sniffs() {
        let content = "{\"key\": \"value\"}";
        let val = try_parse_value_with_format(content, None, "test_source")
            .expect("sniffing should detect JSON");
        assert!(val.is_map());
    }

    #[cfg(any(feature = "toml", feature = "json", feature = "yaml"))]
    #[test]
    fn test_try_parse_value_source_name_in_result() {
        let content = "key = \"value\"\n";
        let result = try_parse_value_with_format(content, None, "my_source");
        assert!(result.is_some());
        let val = result.unwrap();
        assert_eq!(val.source.as_str(), "my_source");
    }

    #[cfg(any(feature = "etcd", feature = "consul"))]
    #[test]
    fn test_merge_into_map_insert_single() {
        let mut map = indexmap::IndexMap::new();
        let val = AnnotatedValue::new(ConfigValue::string("hello"), SourceId::new("test"), "key");
        merge_into_map(&mut map, "key", val);
        assert_eq!(map.len(), 1);
        assert!(map.contains_key("key"));
    }

    #[cfg(any(feature = "etcd", feature = "consul"))]
    #[test]
    fn test_merge_into_map_overwrite() {
        let mut map = indexmap::IndexMap::new();
        let val1 = AnnotatedValue::new(ConfigValue::string("old"), SourceId::new("test"), "key");
        let val2 = AnnotatedValue::new(ConfigValue::string("new"), SourceId::new("test"), "key");
        merge_into_map(&mut map, "key", val1);
        merge_into_map(&mut map, "key", val2);
        assert_eq!(map.len(), 1);
        assert_eq!(map.get("key").unwrap().as_str(), Some("new"));
    }

    #[cfg(any(feature = "etcd", feature = "consul"))]
    #[test]
    fn test_merge_into_map_multiple_keys() {
        let mut map = indexmap::IndexMap::new();
        for i in 0..3i64 {
            let key = format!("key{}", i);
            let val =
                AnnotatedValue::new(ConfigValue::integer(i), SourceId::new("test"), key.as_str());
            merge_into_map(&mut map, &key, val);
        }
        assert_eq!(map.len(), 3);
        assert!(map.contains_key("key0"));
        assert!(map.contains_key("key1"));
        assert!(map.contains_key("key2"));
    }

    #[cfg(any(feature = "etcd", feature = "consul"))]
    #[test]
    fn test_merge_into_map_preserves_insertion_order() {
        let mut map = indexmap::IndexMap::new();
        for name in &["alpha", "beta", "gamma"] {
            let val = AnnotatedValue::new(ConfigValue::string(*name), SourceId::new("test"), *name);
            merge_into_map(&mut map, name, val);
        }
        let keys: Vec<&str> = map.keys().map(|k| k.as_ref()).collect();
        assert_eq!(keys, vec!["alpha", "beta", "gamma"]);
    }
}
