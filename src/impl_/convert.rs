//! Shared format-to-ConfigValue conversion functions.
//!
//! These functions are used by both the loader and the format converter modules
//! to avoid duplicating the same conversion logic.

use crate::types::{AnnotatedValue, ConfigValue, SourceId};
use std::sync::Arc;

#[cfg(feature = "toml")]
pub(crate) fn toml_table_to_config_value(
    table: &toml::Table,
    source: &SourceId,
    prefix: &str,
) -> ConfigValue {
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
    ConfigValue::map(entries)
}

#[cfg(feature = "toml")]
pub(crate) fn toml_value_to_config_value(
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

#[cfg(feature = "json")]
pub(crate) fn json_to_config_value(
    v: &serde_json::Value,
    source: &SourceId,
    prefix: &str,
) -> ConfigValue {
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
                    let p = format!("{}.{}", prefix, i);
                    AnnotatedValue::new(json_to_config_value(v, source, &p), source.clone(), p)
                })
                .collect::<Vec<_>>()
                .into(),
        ),
        serde_json::Value::Object(o) => ConfigValue::map(
            o.iter()
                .map(|(k, v)| {
                    let p = if prefix.is_empty() {
                        k.clone()
                    } else {
                        format!("{}.{}", prefix, k)
                    };
                    (
                        Arc::from(p.clone()),
                        AnnotatedValue::new(
                            json_to_config_value(v, source, &p),
                            source.clone(),
                            k.clone(),
                        ),
                    )
                })
                .collect(),
        ),
    }
}

#[cfg(feature = "yaml")]
pub(crate) fn yaml_to_config_value(
    v: &serde_yaml_ng::Value,
    source: &SourceId,
    prefix: &str,
) -> ConfigValue {
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
                    let p = format!("{}.{}", prefix, i);
                    AnnotatedValue::new(yaml_to_config_value(v, source, &p), source.clone(), p)
                })
                .collect::<Vec<_>>()
                .into(),
        ),
        serde_yaml_ng::Value::Mapping(m) => ConfigValue::map(
            m.iter()
                .filter_map(|(k, v)| {
                    k.as_str().map(|key| {
                        let p = if prefix.is_empty() {
                            key.to_string()
                        } else {
                            format!("{}.{}", prefix, key)
                        };
                        (
                            Arc::from(p.clone()),
                            AnnotatedValue::new(
                                yaml_to_config_value(v, source, &p),
                                source.clone(),
                                key,
                            ),
                        )
                    })
                })
                .collect(),
        ),
        serde_yaml_ng::Value::Tagged(t) => yaml_to_config_value(&t.value, source, prefix),
    }
}
