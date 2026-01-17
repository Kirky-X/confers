// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

use confers::Config;
use serde::{Deserialize, Serialize};

// === Structs ===

#[derive(Debug, Clone, Serialize, Deserialize, Config)]
pub struct DatabaseConfig {
    pub url: String,
    pub connections: u32,
    pub timeout: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Config)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize, Config)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
}

// === Main ===

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    // 1. 创建嵌套 JSON 配置
    let json_content = r#"{
  "server": {
    "host": "localhost",
    "port": 3000
  },
  "database": {
    "url": "postgres://user:pass@localhost/db",
    "connections": 20,
    "timeout": 30
  },
  "features": ["auth", "logging", "metrics"]
}"#;
    std::fs::write("src/04-nested/configs/database.json", json_content)?;

    // 2. 加载配置
    println!("Loading nested configuration...");
    let config = AppConfig::load_file("src/04-nested/configs/database.json")
        .load()
        .await?;

    // 3. 打印嵌套结构
    println!("Host: {}", config.server.host);
    println!("Database URL: {}", config.database.url);

    Ok(())
}
