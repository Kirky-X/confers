// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! Validation for path fragments that are interpolated into remote request URLs.
//!
//! A fragment that reaches a request URL must not be able to change which
//! path the request hits: the `url` crate normalizes `..` segments (and
//! their percent-encoded spellings) while parsing, so an unchecked fragment
//! can walk out of the prefix the caller intended — e.g. out of the tenant
//! prefix of a Vault secret path.

use crate::error::{ConfigError, ConfigResult};

/// Whether a single path segment is safe to interpolate into a request URL.
fn is_safe_segment(segment: &str) -> bool {
    !segment.is_empty()
        && !segment.contains("..")
        && segment
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

/// Validate a single path segment (no separator allowed).
///
/// Plain names (letters, digits, `.`, `_`, `-`) pass; path separators,
/// percent escapes, whitespace and `..` are rejected.
pub(crate) fn validate_single_segment(name: &str) -> ConfigResult<()> {
    if is_safe_segment(name) {
        Ok(())
    } else {
        Err(ConfigError::KeyError {
            message: format!(
                "'{name}' contains characters outside [A-Za-z0-9._-] or a reserved path segment"
            ),
        })
    }
}

/// Validate a `/`-separated path (e.g. `secret/data/tenant-a/db`).
///
/// Every segment must satisfy the single-segment rules, so the fragment can
/// neither escape its prefix nor smuggle an encoded separator.
pub(crate) fn validate_path_segments(path: &str) -> ConfigResult<()> {
    if path.is_empty() {
        return Err(ConfigError::KeyError {
            message: "path must not be empty".to_string(),
        });
    }
    for segment in path.split('/') {
        if !is_safe_segment(segment) {
            return Err(ConfigError::KeyError {
                message: format!(
                    "path segment '{segment}' contains characters outside [A-Za-z0-9._-] or a reserved path segment"
                ),
            });
        }
    }
    Ok(())
}
