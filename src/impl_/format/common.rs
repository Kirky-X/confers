//! Common parsing utilities for configuration formats.
//!
//! This module provides shared functionality for parsing TOML, JSON, and YAML
//! configuration formats.

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
