//! Common helper functions for benchmarks.
//!
//! This module provides shared utility functions used across multiple
//! benchmark files to reduce code duplication.

use confers::types::{AnnotatedValue, ConfigValue};
use confers::SourceId;
use std::sync::Arc;

/// Helper to create a simple AnnotatedValue.
#[allow(dead_code)] // shared bench helper; used by cow_efficiency_bench
pub fn av(value: ConfigValue, path: &str) -> AnnotatedValue {
    AnnotatedValue::new(value, SourceId::new("bench"), path)
}

/// Create a nested config structure with specified depth and width.
///
/// - `depth`: How deep the nesting goes (0 = leaf node with String value)
/// - `width`: Number of children at each level
/// - `path`: Base path for the root node
#[allow(dead_code)] // shared bench helper; used by incremental_merge_bench
pub fn create_nested_config(depth: usize, width: usize, path: &str) -> AnnotatedValue {
    if depth == 0 {
        return AnnotatedValue::new(
            ConfigValue::String("value".to_string()),
            SourceId::new("bench"),
            path,
        );
    }

    let mut map = indexmap::IndexMap::new();
    for i in 0..width {
        let key = format!("key_{}", i);
        let child_path = format!("{}.{}", path, key);
        let value = create_nested_config(depth - 1, width, &child_path);
        map.insert(Arc::from(key), value);
    }
    AnnotatedValue::new(
        ConfigValue::Map(Arc::new(map)),
        SourceId::new("bench"),
        path,
    )
}

/// Create a map with many string key-value pairs.
///
/// - `key_count`: Number of key-value pairs to create
/// - `prefix`: Prefix for the ConfigValue (e.g., "value" creates "value_0", "value_1", ...)
#[allow(dead_code)] // shared bench helper; used by cow_efficiency_bench
pub fn create_large_map(key_count: usize, prefix: &str) -> ConfigValue {
    let mut map = indexmap::IndexMap::new();
    for i in 0..key_count {
        let value = av(
            ConfigValue::String(format!("{}_{}", prefix, i)),
            &format!("k{}", i),
        );
        map.insert(Arc::from(format!("key_{}", i)), value);
    }
    ConfigValue::Map(Arc::new(map))
}

/// Create a map for override/testing purposes.
///
/// Similar to `create_large_map` but uses "updated" prefix by default.
#[allow(dead_code)] // shared bench helper; used by cow_efficiency_bench
pub fn create_override_map(key_count: usize) -> ConfigValue {
    let mut map = indexmap::IndexMap::new();
    for i in 0..key_count {
        let value = av(
            ConfigValue::String(format!("updated_{}", i)),
            &format!("k{}", i),
        );
        map.insert(Arc::from(format!("key_{}", i)), value);
    }
    ConfigValue::Map(Arc::new(map))
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_create_nested_config_depth_zero() {
        let result = super::create_nested_config(0, 10, "test");
        assert!(!result.is_null());
    }

    #[test]
    fn test_create_nested_config_depth_one() {
        let result = super::create_nested_config(1, 5, "test");
        assert!(result.is_map());
    }

    #[test]
    fn test_create_large_map() {
        let result = super::create_large_map(100, "val");
        assert!(matches!(result, confers::types::ConfigValue::Map(_)));
    }
}
