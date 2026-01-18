// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! 最小化配置示例
//!
//! 展示 confers 最基本的配置加载功能，适合快速入门。
//!
//! ## 前提条件
//!
//! - 需要启用的特性：`derive`
//!
//! ## 运行方式
//!
//! ```bash
//! cargo run --example minimal
//! ```

use confers::Config;
use serde::{Deserialize, Serialize};

// === Structs ===

#[derive(Debug, Clone, Serialize, Deserialize, Config)]
pub struct MinimalConfig {
    pub name: String,
}

// === Main ===

fn main() {
    println!("Testing simple macro");

    // 创建配置文件
    let config_content = r#"
name = "minimal-example"
"#;
    if let Err(e) = std::fs::write("src/01-basics/configs/minimal.toml", config_content) {
        println!("❌ 创建配置文件失败: {}", e);
        return;
    }

    // 尝试加载配置
    match MinimalConfig::load() {
        Ok(config) => {
            println!("✅ 配置加载成功:");
            println!("   Name: {}", config.name);
        }
        Err(e) => {
            println!("❌ 配置加载失败: {}", e);
        }
    }
}
