// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! Key-remapping view over a configuration provider.

use std::collections::HashMap;
use std::sync::Arc;

use crate::interface::ConfigProvider;
use crate::types::AnnotatedValue;

/// Presents a [`ConfigProvider`] under different key names.
///
/// Security validators look up canonical keys (`jwt.secret`,
/// `cors.allowed_origins`, …); embedders whose configuration uses their own
/// key names can wrap their provider in this view instead of renaming
/// configuration: `get_raw("canonical")` resolves through the mapping to the
/// actual key and otherwise falls back to the key as given, so unmapped keys
/// behave exactly like the unwrapped provider.
///
/// # Caveats
///
/// * `keys()` passes the inner key set through unchanged — it is not
///   remapped and not filtered.
/// * Validators whose skip/missing logic spans several keys (e.g. the CORS
///   validator skips only when *all three* `cors.*` keys are absent) will
///   keep reporting missing-key findings for the keys that still resolve to
///   nothing. Map either none or all of such a group — see
///   [`CorsValidator`](super::CorsValidator) for the documented failure
///   shape.
pub struct RemappedConfigProvider {
    inner: Arc<dyn ConfigProvider>,
    map: HashMap<String, String>,
}

impl RemappedConfigProvider {
    /// Wrap a provider without any mapping (pure pass-through).
    pub fn new(inner: Arc<dyn ConfigProvider>) -> Self {
        Self {
            inner,
            map: HashMap::new(),
        }
    }

    /// Route lookups of `canonical` to `actual` on the inner provider.
    pub fn with_key_mapping(
        mut self,
        canonical: impl Into<String>,
        actual: impl Into<String>,
    ) -> Self {
        self.map.insert(canonical.into(), actual.into());
        self
    }
}

impl ConfigProvider for RemappedConfigProvider {
    fn get_raw(&self, key: &str) -> Option<&AnnotatedValue> {
        let lookup = match self.map.get(key) {
            Some(actual) => actual.as_str(),
            None => key,
        };
        self.inner.get_raw(lookup)
    }

    fn keys(&self) -> Vec<String> {
        self.inner.keys()
    }
}
