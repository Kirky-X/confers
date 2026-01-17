// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! TOML 格式配置示例
//!
//! 展示如何使用 TOML 格式的配置文件。
//!
//! ## 前提条件
//!
//! - 需要启用的特性：`derive`
//!
//! ## 运行方式
//!
//! ```bash
//! cargo run --example toml_example
//! ```

use confers::Config;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Config)]
pub struct TomlConfig {
    pub name: String,
    pub version: String,
    pub port: u16,
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let config_content = r#"
name = "toml-example"
version = "1.0.0"
port = 8080
"#;
    std::fs::write("src/03-formats/configs/config.toml", config_content)?;

    let config = TomlConfig::load()?;
    println!("TOML Configuration loaded:");
    println!("  Name: {}", config.name);
    println!("  Version: {}", config.version);
    println!("  Port: {}", config.port);

    Ok(())
}
