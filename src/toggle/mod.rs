// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Runtime feature toggle system.
//!
//! Provides a thread-safe registry for dynamically enabling/disabling
//! features at runtime, complementing compile-time `cfg(feature = "...")`
//! flags with soft toggles.
//!
//! Enable the `feature-toggle` feature to use this module.

use crate::interface::ConfigProvider;
use dashmap::DashMap;
use std::time::Instant;

/// Trait describing a single feature toggle.
pub trait FeatureToggle: Send + Sync {
    /// Get the toggle name (unique identifier).
    fn name(&self) -> &str;

    /// Check if this feature is currently enabled.
    fn is_enabled(&self) -> bool;

    /// Get a human-readable description of this feature.
    fn description(&self) -> &str;
}

/// Information about a registered feature toggle.
#[derive(Debug, Clone)]
pub struct FeatureInfo {
    /// Toggle name.
    pub name: String,
    /// Whether the toggle is currently enabled.
    pub enabled: bool,
    /// Human-readable description.
    pub description: String,
    /// When the toggle was last modified.
    pub updated_at: Instant,
}

/// Internal entry stored in the registry.
struct ToggleEntry {
    enabled: bool,
    description: String,
    updated_at: Instant,
}

/// Thread-safe registry for runtime feature toggles.
///
/// Uses `DashMap` for high-concurrency reads and writes.
///
/// # Example
///
/// ```rust,ignore
/// use confers::toggle::FeatureToggleRegistry;
///
/// let registry = FeatureToggleRegistry::new();
/// registry.register("experimental_ui", "New experimental UI", false);
///
/// assert!(!registry.is_enabled("experimental_ui"));
/// registry.enable("experimental_ui");
/// assert!(registry.is_enabled("experimental_ui"));
/// ```
pub struct FeatureToggleRegistry {
    toggles: DashMap<String, ToggleEntry>,
}

impl FeatureToggleRegistry {
    /// Create an empty feature toggle registry.
    pub fn new() -> Self {
        Self {
            toggles: DashMap::new(),
        }
    }

    /// Register a new feature toggle.
    ///
    /// If a toggle with the same name already exists, it is updated with
    /// the new description but the enabled state is preserved.
    pub fn register(
        &self,
        name: impl Into<String>,
        description: impl Into<String>,
        default: bool,
    ) {
        let name = name.into();
        let description = description.into();

        // Only insert if not already present; if present, update description only
        if let Some(mut entry) = self.toggles.get_mut(&name) {
            entry.description = description;
        } else {
            self.toggles.insert(
                name,
                ToggleEntry {
                    enabled: default,
                    description,
                    updated_at: Instant::now(),
                },
            );
        }
    }

    /// Enable a feature toggle.
    ///
    /// Returns the previous enabled state, or `false` if the toggle was not registered.
    pub fn enable(&self, name: &str) -> bool {
        if let Some(mut entry) = self.toggles.get_mut(name) {
            let prev = entry.enabled;
            entry.enabled = true;
            entry.updated_at = Instant::now();
            prev
        } else {
            false
        }
    }

    /// Disable a feature toggle.
    ///
    /// Returns the previous enabled state, or `false` if the toggle was not registered.
    pub fn disable(&self, name: &str) -> bool {
        if let Some(mut entry) = self.toggles.get_mut(name) {
            let prev = entry.enabled;
            entry.enabled = false;
            entry.updated_at = Instant::now();
            prev
        } else {
            false
        }
    }

    /// Check if a feature toggle is enabled.
    ///
    /// Returns `false` if the toggle is not registered.
    pub fn is_enabled(&self, name: &str) -> bool {
        self.toggles
            .get(name)
            .map(|entry| entry.enabled)
            .unwrap_or(false)
    }

    /// Toggle a feature's enabled state (flip it).
    ///
    /// Returns the new state after flipping, or `false` if not registered.
    pub fn toggle(&self, name: &str) -> bool {
        if let Some(mut entry) = self.toggles.get_mut(name) {
            entry.enabled = !entry.enabled;
            entry.updated_at = Instant::now();
            entry.enabled
        } else {
            false
        }
    }

    /// List all registered feature toggles with their current state.
    pub fn list(&self) -> Vec<FeatureInfo> {
        self.toggles
            .iter()
            .map(|entry| FeatureInfo {
                name: entry.key().clone(),
                enabled: entry.value().enabled,
                description: entry.value().description.clone(),
                updated_at: entry.value().updated_at,
            })
            .collect()
    }

    /// Load toggle states from a configuration provider.
    ///
    /// Scans all keys with the given `prefix` (e.g., `"features"`) and
    /// parses their values as booleans. Registered toggles are updated;
    /// unregistered toggles are automatically registered.
    ///
    /// Values that cannot be parsed as boolean are silently skipped.
    pub fn load_from_config(&self, config: &dyn ConfigProvider, prefix: &str) {
        let prefix_dot = format!("{prefix}.");

        for key in config.keys() {
            if let Some(toggle_name) = key.strip_prefix(&prefix_dot) {
                if toggle_name.is_empty() {
                    continue;
                }

                // Try to parse value as boolean
                if let Some(value) = config.get_raw(&key) {
                    if let Some(bool_val) = value.as_bool() {
                        self.set_from_config(toggle_name, bool_val);
                    } else if let Some(str_val) = value.as_str() {
                        match str_val.to_lowercase().as_str() {
                            "true" | "1" | "yes" | "on" => {
                                self.set_from_config(toggle_name, true);
                            }
                            "false" | "0" | "no" | "off" => {
                                self.set_from_config(toggle_name, false);
                            }
                            _ => {
                                // Non-boolean value for toggle — skip (documented behavior)
                            }
                        }
                    }
                }
            }
        }
    }

    /// Internal helper: set toggle state from config, registering if needed.
    fn set_from_config(&self, name: &str, enabled: bool) {
        if let Some(mut entry) = self.toggles.get_mut(name) {
            entry.enabled = enabled;
            entry.updated_at = Instant::now();
        } else {
            self.toggles.insert(
                name.to_string(),
                ToggleEntry {
                    enabled,
                    description: format!("Auto-registered from configuration: {name}"),
                    updated_at: Instant::now(),
                },
            );
        }
    }

    /// Get the number of registered toggles.
    pub fn len(&self) -> usize {
        self.toggles.len()
    }

    /// Check if the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.toggles.is_empty()
    }
}

impl Default for FeatureToggleRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{AnnotatedValue, ConfigValue, SourceId};
    use std::collections::HashMap;
    use std::sync::Arc;

    struct TestProvider(HashMap<String, AnnotatedValue>);

    impl TestProvider {
        fn new() -> Self {
            Self(HashMap::new())
        }

        fn with_value(mut self, key: &str, value: &str) -> Self {
            self.0.insert(
                key.to_string(),
                AnnotatedValue::new(ConfigValue::string(value), SourceId::new("test"), key),
            );
            self
        }

        fn with_bool(mut self, key: &str, value: bool) -> Self {
            self.0.insert(
                key.to_string(),
                AnnotatedValue::new(ConfigValue::bool(value), SourceId::new("test"), key),
            );
            self
        }
    }

    impl ConfigProvider for TestProvider {
        fn get_raw(&self, key: &str) -> Option<&AnnotatedValue> {
            self.0.get(key)
        }

        fn keys(&self) -> Vec<String> {
            self.0.keys().cloned().collect()
        }
    }

    #[test]
    fn test_register_and_query() {
        let registry = FeatureToggleRegistry::new();
        registry.register("feature_a", "Feature A description", false);

        assert!(!registry.is_enabled("feature_a"));
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn test_enable_disable() {
        let registry = FeatureToggleRegistry::new();
        registry.register("feature_a", "Feature A", false);

        let prev = registry.enable("feature_a");
        assert!(!prev);
        assert!(registry.is_enabled("feature_a"));

        let prev = registry.disable("feature_a");
        assert!(prev);
        assert!(!registry.is_enabled("feature_a"));
    }

    #[test]
    fn test_toggle_flip() {
        let registry = FeatureToggleRegistry::new();
        registry.register("feature_a", "Feature A", false);

        let new_state = registry.toggle("feature_a");
        assert!(new_state);
        assert!(registry.is_enabled("feature_a"));

        let new_state = registry.toggle("feature_a");
        assert!(!new_state);
        assert!(!registry.is_enabled("feature_a"));
    }

    #[test]
    fn test_unregistered_returns_false() {
        let registry = FeatureToggleRegistry::new();
        assert!(!registry.is_enabled("nonexistent"));
        assert!(!registry.enable("nonexistent"));
        assert!(!registry.disable("nonexistent"));
        assert!(!registry.toggle("nonexistent"));
    }

    #[test]
    fn test_list() {
        let registry = FeatureToggleRegistry::new();
        registry.register("a", "Feature A", true);
        registry.register("b", "Feature B", false);

        let list = registry.list();
        assert_eq!(list.len(), 2);
        // DashMap doesn't guarantee order, so check both
        let names: Vec<&str> = list.iter().map(|f| f.name.as_str()).collect();
        assert!(names.contains(&"a"));
        assert!(names.contains(&"b"));
    }

    #[test]
    fn test_load_from_config_bool_values() {
        let registry = FeatureToggleRegistry::new();
        registry.register("ui_v2", "New UI", false);

        let config = TestProvider::new()
            .with_bool("features.ui_v2", true)
            .with_bool("features.dark_mode", true);

        registry.load_from_config(&config, "features");

        assert!(registry.is_enabled("ui_v2"));
        assert!(registry.is_enabled("dark_mode")); // auto-registered
    }

    #[test]
    fn test_load_from_config_string_values() {
        let registry = FeatureToggleRegistry::new();

        let config = TestProvider::new()
            .with_value("features.alpha", "true")
            .with_value("features.beta", "false")
            .with_value("features.gamma", "yes")
            .with_value("features.delta", "no")
            .with_value("features.epsilon", "1")
            .with_value("features.zeta", "0")
            .with_value("features.invalid", "maybe");

        registry.load_from_config(&config, "features");

        assert!(registry.is_enabled("alpha"));
        assert!(!registry.is_enabled("beta"));
        assert!(registry.is_enabled("gamma"));
        assert!(!registry.is_enabled("delta"));
        assert!(registry.is_enabled("epsilon"));
        assert!(!registry.is_enabled("zeta"));
        assert!(!registry.is_enabled("invalid")); // non-bool skipped
    }

    #[test]
    fn test_load_from_config_ignores_non_prefix_keys() {
        let registry = FeatureToggleRegistry::new();

        let config = TestProvider::new()
            .with_bool("features.alpha", true)
            .with_bool("other.beta", true);

        registry.load_from_config(&config, "features");

        assert!(registry.is_enabled("alpha"));
        assert!(!registry.is_enabled("beta")); // different prefix
    }

    #[test]
    fn test_concurrent_reads() {
        let registry = Arc::new(FeatureToggleRegistry::new());
        registry.register("feature", "Test feature", true);

        let handles: Vec<_> = (0..10)
            .map(|_| {
                let reg = Arc::clone(&registry);
                std::thread::spawn(move || {
                    for _ in 0..100 {
                        assert!(reg.is_enabled("feature"));
                    }
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }
    }

    #[test]
    fn test_concurrent_writes() {
        let registry = Arc::new(FeatureToggleRegistry::new());
        registry.register("counter", "Test", false);

        let handles: Vec<_> = (0..10)
            .map(|_| {
                let reg = Arc::clone(&registry);
                std::thread::spawn(move || {
                    for _ in 0..100 {
                        reg.toggle("counter");
                    }
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }

        // After 10 threads × 100 toggles = 1000 toggles (even), should be back to false
        assert!(!registry.is_enabled("counter"));
    }

    #[test]
    fn test_default_is_empty() {
        let registry = FeatureToggleRegistry::default();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_register_preserves_enabled_state() {
        let registry = FeatureToggleRegistry::new();
        registry.register("feature", "Original", false);
        registry.enable("feature");
        assert!(registry.is_enabled("feature"));

        // Re-register with different description but should preserve enabled state
        registry.register("feature", "Updated description", false);
        assert!(registry.is_enabled("feature")); // still enabled
    }
}
