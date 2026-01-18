// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! 环境变量覆盖示例
//!
//! 展示如何使用环境变量覆盖配置文件中的值。
//!
//! ## 前提条件
//!
//! - 需要启用的特性：`derive`
//!
//! ## 运行方式
//!
//! ```bash
//! cargo run --example env_override
//! ```

use confers::Config;
use serde::{Deserialize, Serialize};

// === Structs ===

#[derive(Debug, Clone, Serialize, Deserialize, Config)]
#[config(env_prefix = "APP_")]
pub struct EnvConfig {
    pub name: String,
    pub port: u16,
    pub debug: bool,
}

// === Main ===

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    // 1. 创建配置文件
    let config_content = r#"
name = "config-file-name"
port = 8080
debug = false
"#;
    std::fs::write("src/01-basics/configs/env_override.toml", config_content)?;

    // 2. 加载配置（不使用环境变量）
    println!("1. Loading from config file only:");
    let config = EnvConfig::load()?;
    println!("   Name: {}", config.name);
    println!("   Port: {}", config.port);
    println!("   Debug: {}", config.debug);

    // 3. 设置环境变量
    println!("\n2. Setting environment variables:");
    std::env::set_var("APP_NAME", "env-var-name");
    std::env::set_var("APP_PORT", "9090");
    std::env::set_var("APP_DEBUG", "true");
    println!("   APP_NAME=env-var-name");
    println!("   APP_PORT=9090");
    println!("   APP_DEBUG=true");

    // 4. 加载配置（环境变量会覆盖配置文件）
    println!("\n3. Loading with environment override:");
    let config = EnvConfig::load()?;
    println!("   Name: {} (from env)", config.name);
    println!("   Port: {} (from env)", config.port);
    println!("   Debug: {} (from env)", config.debug);

    // 5. 清理环境变量
    std::env::remove_var("APP_NAME");
    std::env::remove_var("APP_PORT");
    std::env::remove_var("APP_DEBUG");

    Ok(())
}
