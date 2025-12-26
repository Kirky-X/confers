#[cfg(feature = "remote")]
use confers::core::ConfigLoader;
use serde::{Deserialize, Serialize};
use validator::Validate;

use confers::ConfigMap;
use figment::value::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default, Validate)]
struct TestConfig {
    name: String,
    value: i32,
}

impl ConfigMap for TestConfig {
    fn to_map(&self) -> HashMap<String, Value> {
        let mut map = HashMap::new();
        map.insert(
            "name".to_string(),
            Value::String(figment::value::Tag::Default, self.name.clone()),
        );
        map.insert(
            "value".to_string(),
            Value::Num(
                figment::value::Tag::Default,
                figment::value::Num::I32(self.value),
            ),
        );
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
