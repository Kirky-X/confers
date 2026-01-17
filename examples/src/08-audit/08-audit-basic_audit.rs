// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

use confers::Config;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Config)]
struct AppConfig {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub port: u16,
    #[serde(default)]
    pub enabled: bool,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    println!("=== Audit Logging Example ===\n");

    println!("1. Loading with defaults...");
    let config = AppConfig::load().expect("Failed to load config");
    println!("   Loaded: {:?}\n", config);

    println!("2. Loading with environment variables...");
    std::env::set_var("APP_NAME", "env_app");
    std::env::set_var("APP_PORT", "9090");
    std::env::set_var("APP_ENABLED", "true");

    let config = AppConfig::load().expect("Failed to load config from env");
    println!("   Loaded: {:?}\n", config);

    println!("3. Loading from file (app.toml)...");
    let config = AppConfig::load().expect("Failed to load config from file");
    println!("   Loaded: {:?}\n", config);

    println!("Note: Audit logging requires specific configuration.");
}
