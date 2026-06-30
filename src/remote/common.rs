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
