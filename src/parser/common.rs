//! Common parsing utilities for configuration formats.
//!
//! This module provides shared functionality for parsing TOML, JSON, and YAML
//! configuration formats.

use crate::value::{AnnotatedValue, ConfigValue, SourceId};
use std::sync::Arc;

#[inline]
pub fn build_path(prefix: &str, key: &str) -> String {
    if prefix.is_empty() {
        key.to_string()
    } else {
        format!("{}.{}", prefix, key)
    }
}

#[inline]
pub fn build_index_path(prefix: &str, index: usize) -> String {
    if prefix.is_empty() {
        index.to_string()
    } else {
        format!("{}.{}", prefix, index)
    }
}

pub fn convert_array<T, F>(
    items: &[T],
    source: &SourceId,
    prefix: &str,
    converter: F,
) -> Arc<[AnnotatedValue]>
where
    F: Fn(&T, &SourceId, &str) -> ConfigValue,
{
    items
        .iter()
        .enumerate()
        .map(|(i, v)| {
            let path = build_index_path(prefix, i);
            AnnotatedValue::new(converter(v, source, &path), source.clone(), path)
        })
        .collect()
}

pub fn convert_map_entries<K, V, F>(
    entries: impl Iterator<Item = (K, V)>,
    source: &SourceId,
    prefix: &str,
    converter: F,
) -> Vec<(Arc<str>, AnnotatedValue)>
where
    K: AsRef<str>,
    F: Fn(&V, &SourceId, &str) -> ConfigValue,
{
    entries
        .map(|(k, v)| {
            let key = k.as_ref();
            let path = build_path(prefix, key);
            (
                Arc::from(path.clone()),
                AnnotatedValue::new(
                    converter(&v, source, &path),
                    source.clone(),
                    key.to_string(),
                ),
            )
        })
        .collect()
}
