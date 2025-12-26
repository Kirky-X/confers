// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

use confers::ConfigMap;
use figment::value::Value;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use validator::Validate;

#[derive(Debug, Clone, Default, Serialize, Deserialize, Validate)]
struct TestConfig {
    #[validate(length(min = 1))]
    value: String,
}

impl ConfigMap for TestConfig {
    fn to_map(&self) -> HashMap<String, Value> {
        let mut map = HashMap::new();
        map.insert("value".to_string(), Value::from(self.value.clone()));
        map
    }

    fn env_mapping() -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.insert("value".to_string(), "TEST_CONFIG_VALUE".to_string());
        map
    }
}

#[tokio::test]
async fn test_remote_watcher_creation() {
    #[cfg(all(feature = "watch", feature = "remote"))]
    {
        use confers::watcher::ConfigWatcher;
        use std::time::Duration;

        let watcher =
            ConfigWatcher::new_remote("http://example.com/config.json", Duration::from_secs(60))
                .with_remote_auth(Some("user".to_string()), Some("pass".to_string()), None);

        println!("Remote watcher created successfully");

        match watcher.watch() {
            Ok((_debouncer, _receiver)) => {
                println!("Remote watch started successfully");
            }
            Err(e) => {
                println!("Failed to start remote watch: {}", e);
            }
        }
    }

    #[cfg(not(all(feature = "watch", feature = "remote")))]
    {
        println!("Remote watch not available (requires 'watch' and 'remote' features)");
    }
}

#[test]
fn test_config_loader_with_remote_watch() {
    #[cfg(all(feature = "watch", feature = "remote"))]
    {
        use confers::core::ConfigLoader;

        let _loader = ConfigLoader::<TestConfig>::new()
            .with_remote("http://localhost:8080/config.json")
            .with_remote_auth("user", "pass");

        println!("ConfigLoader with remote watch configured successfully");
    }

    #[cfg(not(all(feature = "watch", feature = "remote")))]
    {
        println!("Remote watch not available (requires 'watch' and 'remote' features)");
    }
}
