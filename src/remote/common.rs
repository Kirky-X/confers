//! Shared utilities for remote configuration sources.

use crate::loader::{detect_format_from_content, Format};
use crate::types::{AnnotatedValue, SourceId};
use std::sync::Arc;

/// Try to parse a value as config format.
pub(crate) fn try_parse_value(content: &str, source_name: &str) -> Option<AnnotatedValue> {
    let format = detect_format_from_content(content)?;

    match format {
        Format::Toml => {
            let table: toml::Table = toml::from_str(content).ok()?;
            Some(crate::loader::parse_toml_table(
                &table,
                &SourceId::new(source_name),
                "",
            ))
        }
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
        #[cfg(not(feature = "yaml"))]
        Format::Yaml => None,
        _ => None,
    }
}

/// Merge a key-value pair into a config map.
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
    use crate::types::ConfigValue;

    #[test]
    fn test_try_parse_value_toml() {
        let content = "key = \"value\"\n";
        let result = try_parse_value(content, "test_source");
        assert!(result.is_some());
        let val = result.unwrap();
        assert!(val.is_map());
    }

    #[test]
    fn test_try_parse_value_json() {
        let content = "{\"key\": \"value\"}";
        let result = try_parse_value(content, "test_source");
        assert!(result.is_some());
        let val = result.unwrap();
        assert!(val.is_map());
    }

    #[cfg(feature = "yaml")]
    #[test]
    fn test_try_parse_value_yaml() {
        let content = "---\nkey: value\n";
        let result = try_parse_value(content, "test_source");
        assert!(result.is_some());
        let val = result.unwrap();
        assert!(val.is_map());
    }

    #[test]
    fn test_try_parse_value_invalid_content() {
        // Content that does not match any known format pattern
        let content = "totally unrecognizable content @#$%";
        let result = try_parse_value(content, "test_source");
        assert!(result.is_none());
    }

    #[test]
    fn test_try_parse_value_empty_content() {
        let result = try_parse_value("", "test_source");
        assert!(result.is_none());
    }

    #[test]
    fn test_try_parse_value_whitespace_only() {
        let result = try_parse_value("   \n\t  ", "test_source");
        assert!(result.is_none());
    }

    #[test]
    fn test_try_parse_value_invalid_toml() {
        // Recognized as TOML (has " = ") but fails to parse
        let content = "key = = invalid";
        let result = try_parse_value(content, "test_source");
        assert!(result.is_none());
    }

    #[test]
    fn test_try_parse_value_invalid_json() {
        // Recognized as JSON (starts with {, has quotes and colon) but fails to parse
        let content = "{\"key\": invalid_value}";
        let result = try_parse_value(content, "test_source");
        assert!(result.is_none());
    }

    #[test]
    fn test_try_parse_value_source_name_in_result() {
        let content = "key = \"value\"\n";
        let result = try_parse_value(content, "my_source");
        assert!(result.is_some());
        let val = result.unwrap();
        assert_eq!(val.source.as_str(), "my_source");
    }

    #[test]
    fn test_merge_into_map_insert_single() {
        let mut map = indexmap::IndexMap::new();
        let val = AnnotatedValue::new(ConfigValue::string("hello"), SourceId::new("test"), "key");
        merge_into_map(&mut map, "key", val);
        assert_eq!(map.len(), 1);
        assert!(map.contains_key("key"));
    }

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
