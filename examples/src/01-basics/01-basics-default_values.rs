// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! 默认值示例
//!
//! 展示如何为配置字段设置默认值。
//!
//! ## 前提条件
//!
//! - 需要启用的特性：`derive`
//!
//! ## 运行方式
//!
//! ```bash
//! cargo run --example 01-basics-default_values
//! ```

use confers::Config;
use serde::{Deserialize, Serialize};

// === Structs ===

#[derive(Debug, Clone, Serialize, Deserialize, Config)]
#[config(env_prefix = "APP_")]
pub struct DefaultConfig {
    #[serde(default = "default_name")]
    pub name: String,

    #[serde(default = "default_port")]
    pub port: u16,

    #[serde(default = "default_debug")]
    pub debug: bool,

    #[serde(default = "default_workers")]
    pub workers: usize,
}

fn default_name() -> String {
    "default-app".to_string()
}

fn default_port() -> u16 {
    8080
}

fn default_debug() -> bool {
    false
}

fn default_workers() -> usize {
    4
}

// === Main ===

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    // 1. 创建不完整的配置文件（只包含部分字段）
    let config_content = r#"
name = "my-app"
port = 9000
"#;
    std::fs::write("src/01-basics/configs/default_values.toml", config_content)?;

    // 2. 加载配置（缺失的字段将使用默认值）
    println!("Loading configuration with default values...");
    let config = DefaultConfig::load()?;

    // 3. 打印配置
    println!("Configuration loaded:");
    println!("  Name: {} (from config)", config.name);
    println!("  Port: {} (from config)", config.port);
    println!("  Debug: {} (default)", config.debug);
    println!("  Workers: {} (default)", config.workers);

    Ok(())
}
