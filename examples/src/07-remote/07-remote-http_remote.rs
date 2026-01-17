// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

#[cfg(feature = "remote")]
use confers::{core::ConfigLoader, Config};
#[cfg(feature = "remote")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "remote")]
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
#[config(
    remote = "http://localhost:8080/config",
    remote_timeout = "5",
    remote_fallback = true
)]
pub struct RemoteConfig {
    pub api_key: String,
    pub endpoint: String,
    #[serde(default = "default_timeout")]
    pub timeout: u32,
}

#[allow(dead_code)]
fn default_timeout() -> u32 {
    30
}

#[cfg(not(feature = "remote"))]
fn main() {
    println!("Please run with --features remote");
}

#[cfg(feature = "remote")]
fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    println!("This example requires a running HTTP server at http://localhost:8080/config");
    println!("Example JSON response: {{\"api_key\": \"secret\", \"endpoint\": \"https://api.example.com\"}}");

    // 演示启用回退时的加载
    println!("\nAttempting to load from remote (will fallback if server is down)...");

    // 创建本地回退文件
    std::fs::write(
        "src/07-remote/configs/remote.toml",
        "api_key = 'local-fallback'\nendpoint = 'http://localhost'\ntimeout = 60",
    )?;

    // 使用 ConfigLoader 加载
    let config: RemoteConfig = ConfigLoader::new()
        .with_file("src/07-remote/configs/remote.toml")
        .load_sync()?;
    println!("Loaded config: {:#?}", config);

    Ok(())
}
