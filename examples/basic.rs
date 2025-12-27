// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

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

    // 2. 创建示例配置文件（如果不存在）
    let config_content = r#"
name = "basic-example"
port = 8080
debug = true
tags = ["rust", "config", "example"]
"#;
    std::fs::write("examples/config.toml", config_content)?;

    // 3. 加载配置
    println!("Loading configuration...");
    let config = BasicConfig::load()?;

    // 4. 打印配置
    println!("Loaded configuration: {:#?}", config);

    // 5. 当 validate = true 时，配置在加载过程中会被验证
    println!("Configuration loaded successfully!");

    // 6. 演示环境变量覆盖
    println!("\nDemonstrating environment variable override...");
    std::env::set_var("APP_PORT", "9090");

    let config_with_env = BasicConfig::load()?;
    println!("Port after env override: {}", config_with_env.port);

    Ok(())
}
