// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

#[cfg(feature = "remote")]
use confers::core::ConfigLoader;
use serde::{Deserialize, Serialize};
use validator::Validate;

use confers::ConfigMap;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default, Validate)]
struct TestConfig {
    name: String,
    value: i32,
}

impl ConfigMap for TestConfig {
    fn to_map(&self) -> HashMap<String, serde_json::Value> {
        let mut map = HashMap::new();
        map.insert("name".to_string(), serde_json::json!(self.name));
        map.insert("value".to_string(), serde_json::json!(self.value));
        map
    }

    fn env_mapping() -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.insert("name".to_string(), "TEST_CONFIG_NAME".to_string());
        map.insert("value".to_string(), "TEST_CONFIG_VALUE".to_string());
        map
    }
}

#[cfg(feature = "remote")]
#[test]
fn test_builder_with_remote_config() {
    let _loader =
        ConfigLoader::<TestConfig>::new().with_remote_config("http://example.com/config.json");
}

#[cfg(feature = "remote")]
#[test]
fn test_builder_with_remote_auth() {
    let _loader = ConfigLoader::<TestConfig>::new()
        .with_remote_config("http://example.com/config.json")
        .with_remote_auth("user", "pass");
}

#[cfg(feature = "remote")]
#[test]
fn test_builder_with_remote_token() {
    let _loader = ConfigLoader::<TestConfig>::new()
        .with_remote_config("http://example.com/config.json")
        .with_remote_token("token123");
}

#[cfg(feature = "remote")]
#[test]
fn test_builder_with_remote_tls() {
    let _loader = ConfigLoader::<TestConfig>::new()
        .with_remote_config("https://example.com/config.json")
        .with_remote_ca_cert("/path/to/ca.crt")
        .with_remote_client_cert("/path/to/client.crt")
        .with_remote_client_key("/path/to/client.key");
}
