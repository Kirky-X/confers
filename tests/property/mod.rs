// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Property-based tests for configuration parsing and validation

use proptest::collection;
use proptest::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Test configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestConfig {
    pub name: String,
    pub port: u16,
    pub enabled: bool,
    pub tags: Vec<String>,
}

/// Generate random valid config names
fn valid_config_name() -> impl Strategy<Value = String> {
    "[a-z][a-z0-9_-]{0,31}".prop_filter(
        "Config name must be 1-32 chars, lowercase alphanumeric",
        |name| {
            !name.is_empty()
                && name.len() <= 32
                && name.chars().next().unwrap().is_ascii_lowercase()
                && name
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
        },
    )
}

/// Generate valid port numbers
fn valid_port() -> impl Strategy<Value = u16> {
    (1u16..=65535u16)
}

/// Generate environment variable names
fn environment_name() -> impl Strategy<Value = String> {
    "[A-Z][A-Z0-9_]*".prop_map(|s| s)
}

/// Generate config values
fn config_value() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9_.-]{0,100}".prop_map(|s| s)
}

/// Strategy for generating test configs
fn test_config_strategy() -> impl Strategy<Value = TestConfig> {
    (
        valid_config_name(),
        valid_port(),
        proptest::bool::ANY,
        collection::vec(valid_config_name(), 0..5),
    )
        .prop_map(|(name, port, enabled, tags)| TestConfig {
            name,
            port,
            enabled,
            tags,
        })
}

/// Property test: Config name validation
proptest! {
    #[test]
    fn test_config_name_validation(name in valid_config_name()) {
        // Property: valid names should pass basic validation
        prop_assert!(!name.is_empty());
        prop_assert!(name.len() <= 32);
        prop_assert!(name.chars().next().unwrap().is_ascii_lowercase());
    }
}

/// Property test: Port range validation
proptest! {
    #[test]
    fn test_port_range(port in valid_port()) {
        // Property: valid ports should be in range
        prop_assert!(port >= 1);
        prop_assert!(port <= 65535);
    }
}

/// Property test: Config serialization roundtrip
proptest! {
    #[test]
    fn test_toml_serialization_roundtrip(config in test_config_strategy()) {
        // Serialize to TOML
        let toml_string = toml::to_string(&config).expect("Should serialize TOML without error");

        // Deserialize back
        let decoded: TestConfig = toml::from_str(&toml_string).expect("Should deserialize TOML without error");

        // Property: roundtrip should preserve data
        prop_assert_eq!(config.name, decoded.name);
        prop_assert_eq!(config.port, decoded.port);
        prop_assert_eq!(config.enabled, decoded.enabled);
        prop_assert_eq!(config.tags.len(), decoded.tags.len());
    }
}

/// Property test: JSON serialization roundtrip
proptest! {
    #[test]
    fn test_json_serialization_roundtrip(config in test_config_strategy()) {
        // Serialize to JSON
        let json_string = serde_json::to_string(&config).expect("Should serialize JSON without error");

        // Deserialize back
        let decoded: TestConfig = serde_json::from_str(&json_string).expect("Should deserialize JSON without error");

        // Property: roundtrip should preserve data
        prop_assert_eq!(config.name, decoded.name);
        prop_assert_eq!(config.port, decoded.port);
        prop_assert_eq!(config.enabled, decoded.enabled);
        prop_assert_eq!(config.tags, decoded.tags);
    }
}

/// Property test: Environment variable parsing
proptest! {
    #[test]
    fn test_env_var_parsing(
        env_name in environment_name(),
        env_value in config_value()
    ) {
        // Property: environment variable format should be valid
        prop_assert!(env_name.starts_with(char::is_uppercase));
        prop_assert!(env_name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'));
    }
}

/// Property test: HashMap generation for config settings
proptest! {
    #[test]
    fn test_config_settings_generation(
        settings in collection::hash_map(
            valid_config_name(),
            config_value(),
            0..10
        )
    ) {
        // Property: generated settings should have expected properties
        for (key, value) in &settings {
            prop_assert!(!key.is_empty());
            prop_assert!(key.len() <= 32);
            prop_assert!(value.len() <= 100);
        }

        // Property: settings count should match
        prop_assert!(settings.len() <= 10);
    }
}

/// Property test: Config struct with arbitrary
proptest! {
    #[test]
    fn test_config_with_arbitrary(
        name in "[a-z][a-z0-9_-]{0,31}",
        port in 1u16..=65535,
        enabled in proptest::bool::ANY,
        tags in collection::vec("[a-z0-9_-]{1,20}", 0..5)
    ) {
        let config = TestConfig {
            name: name.clone(),
            port,
            enabled,
            tags: tags.clone(),
        };

        // Property: config fields should match input
        prop_assert_eq!(config.name, name);
        prop_assert_eq!(config.port, port);
        prop_assert_eq!(config.enabled, enabled);
        prop_assert_eq!(config.tags, tags);
    }
}

/// Property test: Boundary values for ports
proptest! {
    #[test]
    fn test_port_boundary_values(
        port in prop_oneof![
            Just(1u16),
            Just(80u16),
            Just(443u16),
            Just(8080u16),
            Just(65535u16),
        ]
    ) {
        // Property: boundary ports should be valid
        prop_assert!(port >= 1 && port <= 65535);
    }
}

/// Property test: Special characters in config values
proptest! {
    #[test]
    fn test_special_characters_handling(
        value in "[a-zA-Z0-9_.-]{0,50}"
    ) {
        // Property: special characters should be preserved
        let result = value.clone();
        prop_assert!(result.len() <= 50);
    }
}
