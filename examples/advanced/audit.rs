// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

use confers::audit::Sanitize;
use confers::core::ConfigLoader;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use validator::Validate;

#[derive(Debug, Clone, Default, Serialize, Deserialize, Validate)]
struct AppConfig {
    #[serde(default)]
    #[validate(length(min = 1))]
    pub name: String,
    #[serde(default)]
    #[validate(range(min = 1, max = 65535))]
    pub port: u16,
    #[serde(default)]
    pub enabled: bool,
}

impl Sanitize for AppConfig {
    fn sanitize(&self) -> Value {
        serde_json::json!({
            "name": self.name,
            "port": self.port,
            "enabled": self.enabled
        })
    }
}

impl confers::ConfigMap for AppConfig {
    fn to_map(&self) -> HashMap<String, figment::value::Value> {
        let mut map = HashMap::new();
        map.insert(
            "name".to_string(),
            figment::value::Value::from(self.name.clone()),
        );
        map.insert("port".to_string(), figment::value::Value::from(self.port));
        map.insert(
            "enabled".to_string(),
            figment::value::Value::from(self.enabled),
        );
        map
    }

    fn env_mapping() -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.insert("name".to_string(), "APP_NAME".to_string());
        map.insert("port".to_string(), "APP_PORT".to_string());
        map.insert("enabled".to_string(), "APP_ENABLED".to_string());
        map
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    println!("=== Audit Logging Example ===\n");

    println!("1. Loading with defaults...");
    let config = ConfigLoader::new()
        .with_defaults(AppConfig {
            name: "default_app".to_string(),
            port: 8080,
            enabled: true,
        })
        .with_audit_log(true)
        .with_audit_log_path("./examples/configs/app.audit.toml")
        .load()
        .await
        .expect("Failed to load config with defaults");
    println!("   Loaded: {:?}\n", config);

    println!("2. Loading with environment variables...");
    std::env::set_var("APP_NAME", "env_app");
    std::env::set_var("APP_PORT", "9090");
    std::env::set_var("APP_ENABLED", "true");

    let config = ConfigLoader::new()
        .with_env_prefix("APP")
        .with_audit_log(true)
        .with_audit_log_path("./examples/configs/env.audit.toml")
        .load()
        .await
        .expect("Failed to load config from env");
    println!("   Loaded: {:?}\n", config);

    println!("3. Loading from file (app.toml)...");
    let config = ConfigLoader::new()
        .with_file("./examples/configs/app.toml")
        .with_audit_log(true)
        .with_audit_log_path("./examples/configs/file.audit.toml")
        .load()
        .await
        .expect("Failed to load config from file");
    println!("   Loaded: {:?}\n", config);

    println!("Audit logs have been saved to:");
    println!("  - ./examples/configs/app.audit.toml");
    println!("  - ./examples/configs/env.audit.toml");
    println!("  - ./examples/configs/file.audit.toml");
}
