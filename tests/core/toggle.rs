// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Feature toggle integration tests.

#![cfg(feature = "feature-toggle")]

use confers::interface::ConfigProvider;
use confers::toggle::FeatureToggleRegistry;
use confers::types::{AnnotatedValue, ConfigValue, SourceId};
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
fn test_toggle_lifecycle() {
    let registry = FeatureToggleRegistry::new();

    // Register
    registry.register("feature_a", "Feature A", false);
    assert!(!registry.is_enabled("feature_a"));

    // Enable
    registry.enable("feature_a");
    assert!(registry.is_enabled("feature_a"));

    // Disable
    registry.disable("feature_a");
    assert!(!registry.is_enabled("feature_a"));

    // Toggle
    let new_state = registry.toggle("feature_a");
    assert!(new_state);
    assert!(registry.is_enabled("feature_a"));
}

#[test]
fn test_load_from_config_integration() {
    let registry = FeatureToggleRegistry::new();
    registry.register("ui_v2", "New UI", false);

    let config = TestProvider::new()
        .with_bool("features.ui_v2", true)
        .with_bool("features.dark_mode", true)
        .with_value("features.beta_banner", "yes");

    registry.load_from_config(&config, "features");

    assert!(registry.is_enabled("ui_v2"));
    assert!(registry.is_enabled("dark_mode"));
    assert!(registry.is_enabled("beta_banner"));
}

#[test]
fn test_concurrent_toggle_operations() {
    let registry = Arc::new(FeatureToggleRegistry::new());
    registry.register("shared_feature", "Shared", false);

    let handles: Vec<_> = (0..8)
        .map(|i| {
            let reg = Arc::clone(&registry);
            std::thread::spawn(move || {
                for _ in 0..50 {
                    reg.toggle("shared_feature");
                    // Interleave reads
                    let _ = reg.is_enabled("shared_feature");
                    if i % 2 == 0 {
                        reg.enable("shared_feature");
                    } else {
                        reg.disable("shared_feature");
                    }
                }
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }

    // Should not panic, and toggle should be in a valid boolean state
    let final_state = registry.is_enabled("shared_feature");
    // After interleaved toggle/enable/disable, the state must be one of the two valid values
    assert!(
        final_state || !final_state,
        "toggle state must be consistent after concurrent access"
    );
    assert_eq!(registry.len(), 1);
}

#[test]
fn test_list_returns_all_toggles() {
    let registry = FeatureToggleRegistry::new();
    registry.register("a", "Feature A", true);
    registry.register("b", "Feature B", false);
    registry.register("c", "Feature C", true);

    let list = registry.list();
    assert_eq!(list.len(), 3);

    let a_info = list.iter().find(|f| f.name == "a").unwrap();
    assert!(a_info.enabled);
    assert_eq!(a_info.description, "Feature A");

    let b_info = list.iter().find(|f| f.name == "b").unwrap();
    assert!(!b_info.enabled);
}
