// Copyright (c) 2025 Kirky.X
//
// Licensed under MIT License
// See LICENSE file in the project root for full license information.

//! 基本配置加载示例
//!
//! 展示 confers 的基本配置加载功能，包括验证和环境变量覆盖。
//!
//! ## 前提条件
//!
//! - 需要启用的特性：`derive`, `validation`
//!
//! ## 运行方式
//!
//! ```bash
//! cargo run --example basic --features "derive,validation"
//! ```

use confers::Config;
use serde::{Deserialize, Serialize};

// === Structs ===

#[derive(Debug, Clone, Serialize, Deserialize, Config)]
#[config(validate)]
#[config(env_prefix = "APP_", format_detection = "Auto")]
pub struct BasicConfig {
    pub name: String,
    pub port: u16,
    pub debug: bool,
}

// === Main ===

fn main() -> anyhow::Result<()> {
    // 1. 初始化日志
    tracing_subscriber::fmt::init();

    // 2. 创建示例配置文件
    let config_content = r#"name = "basic-example"
port = 8080
debug = true
tags = ["rust", "config", "example"]
"#;

    // 使用正确的配置路径 - 写入当前目录
    let config_path = "config.toml";
    std::fs::write(config_path, config_content)?;

    // 3. 加载配置
    println!("Loading configuration...");

    match BasicConfig::load() {
        Ok(config) => {
            println!("✅ 配置加载成功!");
            println!("   name: '{}'", config.name);
            println!("   port: {}", config.port);
            println!("   debug: {}", config.debug);
        }
        Err(e) => {
            println!("❌ 配置加载失败: {}", e);
        }
    }

    // 4. 当 validate = true 时，配置在加载过程中会被验证
    println!("Configuration loaded successfully!");

    // 5. 演示环境变量覆盖
    println!("\nDemonstrating environment variable override...");
    std::env::set_var("APP_PORT", "9090");

    match BasicConfig::load() {
        Ok(config_with_env) => {
            println!("Port after env override: {}", config_with_env.port);
        }
        Err(e) => {
            println!("Failed to load with env override: {}", e);
        }
    }

    // 清理
    let _ = std::fs::remove_file(config_path);

    Ok(())
}
