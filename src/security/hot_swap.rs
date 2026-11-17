// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! Secure hot-swap semantics for the reload pipeline.
//!
//! [`SecurityPolicy`] is the declarative switchboard for three behaviors
//! around hot reload and snapshots:
//!
//! - `zeroize_on_replace`: scrub the pipeline-owned previous config tree
//!   after a successful reload commit (see [`zeroize_sensitive_values`]).
//! - `snapshot_sensitive_mode`: how snapshot saves treat sensitive values
//!   (redact / encrypt / plain).
//! - `audit_on_reload`: emit an `AuditEvent::ReloadTrigger` per successful
//!   reload commit.
//!
//! The default is `(false, Redacted, false)` — byte-for-byte the pre-policy
//! behavior; every opt-in is explicit.

use crate::sensitive_names::is_sensitive_name;
use crate::types::ConfigValue;

/// How snapshot saves treat sensitive values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SnapshotSensitiveMode {
    /// Sensitive values are replaced by a placeholder (the pre-policy
    /// behavior; `SerializeMode::Redacted`).
    Redacted,
    /// Sensitive values are written as `enc:v1:<key_version>:<payload>`
    /// envelopes (requires the `encryption` feature at runtime; the master
    /// key must travel with the snapshot to restore).
    Encrypted,
    /// Sensitive values are written in clear text. Explicit opt-in for
    /// trusted export scenarios; the auditability burden stays with the
    /// caller.
    Plain,
}

/// Declarative security policy for the reload pipeline.
///
/// Boundary note: `zeroize_on_replace` only covers the tree the pipeline
/// itself owns at swap time. Readers may hold `Arc<T>` clones of a previous
/// value for arbitrary long (arc-swap semantics); the library cannot scrub
/// memory those readers own — the zeroizer skips any tree that is not
/// exclusively held instead of introducing a data race.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct SecurityPolicy {
    /// Zeroize the pipeline-owned previous config tree after a reload
    /// commit. Defaults to `false` (behavior compatibility).
    pub zeroize_on_replace: bool,
    /// Snapshot handling of sensitive values. Defaults to [`SnapshotSensitiveMode::Redacted`].
    pub snapshot_sensitive_mode: SnapshotSensitiveMode,
    /// Emit `AuditEvent::ReloadTrigger` after each successful reload commit
    /// (requires an injected `AuditWriter`). Defaults to `false`.
    pub audit_on_reload: bool,
}

impl Default for SecurityPolicy {
    fn default() -> Self {
        Self {
            zeroize_on_replace: false,
            snapshot_sensitive_mode: SnapshotSensitiveMode::Redacted,
            audit_on_reload: false,
        }
    }
}

/// Zeroize every `ConfigValue::String` leaf under a sensitive key, in place.
///
/// A key "hits" via [`is_sensitive_name`] (the same heuristic the snapshot
/// redaction uses); under a hitting key every string leaf is scrubbed — a
/// direct string value and strings nested in arrays alike. Non-sensitive
/// leaves are left untouched. Interior `Arc` nodes are only entered when
/// exclusively held (`Arc::get_mut`); shared subtrees are skipped rather
/// than cloned, so the scrub never degenerates into zeroizing a private
/// copy while the published bytes survive.
///
/// The string bytes are overwritten with zeroes (via `zeroize::Zeroize`)
/// and the string is then `clear()`ed, so the original bytes are not
/// recoverable from the allocation. Returns the number of scrubbed string
/// leaves (for logs and tests).
pub fn zeroize_sensitive_values(value: &mut ConfigValue) -> usize {
    match value {
        ConfigValue::Map(map) => match Arc::get_mut(map) {
            Some(map) => map
                .iter_mut()
                .map(|(key, child)| {
                    if is_sensitive_name(key) {
                        zeroize_string_leaves(&mut child.inner)
                    } else {
                        zeroize_sensitive_values(&mut child.inner)
                    }
                })
                .sum(),
            None => 0,
        },
        ConfigValue::Array(items) => match Arc::get_mut(items) {
            Some(items) => items
                .iter_mut()
                .map(|child| zeroize_sensitive_values(&mut child.inner))
                .sum(),
            None => 0,
        },
        _ => 0,
    }
}

/// Scrub every string leaf in the subtree rooted at `value` (used under a
/// sensitive key, where the whole subtree's strings count as sensitive).
fn zeroize_string_leaves(value: &mut ConfigValue) -> usize {
    match value {
        ConfigValue::String(s) => {
            // SAFETY: zero bytes are valid UTF-8, so zeroing the raw buffer
            // keeps the string in a representable state before `clear()`.
            unsafe { s.as_mut_vec() }.zeroize();
            s.clear();
            1
        }
        ConfigValue::Map(map) => match Arc::get_mut(map) {
            Some(map) => map
                .values_mut()
                .map(|child| zeroize_string_leaves(&mut child.inner))
                .sum(),
            None => 0,
        },
        ConfigValue::Array(items) => match Arc::get_mut(items) {
            Some(items) => items
                .iter_mut()
                .map(|child| zeroize_string_leaves(&mut child.inner))
                .sum(),
            None => 0,
        },
        _ => 0,
    }
}

use std::sync::Arc;
use zeroize::Zeroize;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{AnnotatedValue, SourceId};

    fn annotated(v: ConfigValue) -> AnnotatedValue {
        AnnotatedValue::new(v, SourceId::default(), "")
    }

    fn map(entries: Vec<(&str, ConfigValue)>) -> ConfigValue {
        let mut indexed = indexmap::IndexMap::new();
        for (k, v) in entries {
            indexed.insert(Arc::from(k), annotated(v));
        }
        ConfigValue::Map(Arc::new(indexed))
    }

    #[test]
    fn default_policy_is_current_behavior() {
        let policy = SecurityPolicy::default();
        assert!(!policy.zeroize_on_replace);
        assert_eq!(
            policy.snapshot_sensitive_mode,
            SnapshotSensitiveMode::Redacted
        );
        assert!(!policy.audit_on_reload);
    }

    #[test]
    fn policy_serde_roundtrip_and_missing_fields() {
        let policy = SecurityPolicy {
            zeroize_on_replace: true,
            snapshot_sensitive_mode: SnapshotSensitiveMode::Encrypted,
            audit_on_reload: true,
        };
        let json = serde_json::to_string(&policy).unwrap();
        assert!(json.contains("\"zeroize_on_replace\":true"), "{json}");
        assert!(
            json.contains("\"snapshot_sensitive_mode\":\"encrypted\""),
            "{json}"
        );
        let back: SecurityPolicy = serde_json::from_str(&json).unwrap();
        assert_eq!(back, policy);

        // Old JSON with missing fields deserializes to defaults.
        let partial: SecurityPolicy = serde_json::from_str("{}").unwrap();
        assert_eq!(partial, SecurityPolicy::default());
    }

    #[test]
    fn zeroize_scrubs_sensitive_leaves_and_keeps_the_rest() {
        let nested = map(vec![
            ("password", ConfigValue::String("s3cret-pw".to_string())),
            ("timeout", ConfigValue::U64(30)),
            (
                "db",
                map(vec![
                    ("api_key", ConfigValue::String("hunter2".to_string())),
                    ("host", ConfigValue::String("db.local".to_string())),
                ]),
            ),
        ]);

        let mut value = nested;
        let zeroized = zeroize_sensitive_values(&mut value);
        // password + nested db.api_key
        assert_eq!(zeroized, 2, "{value:?}");

        let ConfigValue::Map(map) = &value else {
            panic!("map")
        };
        assert_eq!(
            map.get("password").unwrap().inner,
            ConfigValue::String(String::new()),
            "password must be scrubbed"
        );
        assert_eq!(map.get("timeout").unwrap().inner, ConfigValue::U64(30));
        let ConfigValue::Map(db) = &map.get("db").unwrap().inner else {
            panic!("db map")
        };
        assert_eq!(
            db.get("api_key").unwrap().inner,
            ConfigValue::String(String::new()),
            "nested api_key must be scrubbed"
        );
        assert_eq!(
            db.get("host").unwrap().inner,
            ConfigValue::String("db.local".to_string()),
            "non-sensitive leaf keeps its value"
        );
    }

    #[test]
    fn zeroize_hits_strings_inside_arrays_under_sensitive_key() {
        let with_array = map(vec![
            (
                "tokens",
                ConfigValue::Array(Arc::new([
                    annotated(ConfigValue::String("tok-a".to_string())),
                    annotated(ConfigValue::String("tok-b".to_string())),
                ])),
            ),
            (
                "hosts",
                ConfigValue::Array(Arc::new([annotated(ConfigValue::String("h1".to_string()))])),
            ),
        ]);

        let mut value = with_array;
        let zeroized = zeroize_sensitive_values(&mut value);
        assert_eq!(zeroized, 2, "both tokens scrubbed, hosts untouched");
    }

    #[test]
    fn zeroize_skips_shared_subtrees_instead_of_cloning() {
        let inner = map(vec![(
            "password",
            ConfigValue::String("shared".to_string()),
        )]);
        let shared = match &inner {
            ConfigValue::Map(m) => Arc::clone(m),
            _ => unreachable!(),
        };
        let mut outer = map(vec![("db", inner)]);
        // `shared` keeps the interior Arc alive → the subtree is not
        // exclusively held and must be skipped (no zeroizing a copy).
        let zeroized = zeroize_sensitive_values(&mut outer);
        assert_eq!(
            zeroized, 0,
            "shared subtree must not be cloned-and-zeroized"
        );
        let ConfigValue::Map(map) = &outer else {
            panic!()
        };
        let db = map.get("db").unwrap();
        let ConfigValue::Map(dbm) = &db.inner else {
            panic!()
        };
        // Same Arc identity as the one we held: skipped, still intact.
        assert!(Arc::ptr_eq(dbm, &shared));
    }
}
