// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! config-rs 兼容性示例
//!
//! 展示如何从 config-rs 迁移到 confers。
//!
//! ## 前提条件
//!
//! - 需要启用的特性：`derive`
//!
//! ## 运行方式
//!
//! ```bash
//! cargo run --bin 14-compat-config_rs_compat
//! ```

use confers::Config;
use serde::{Deserialize, Serialize};

// === Structs ===

#[derive(Debug, Clone, Serialize, Deserialize, Config)]
pub struct AppConfig {
    pub name: String,
    pub version: String,
    pub debug: bool,
}

// === Main ===

fn main() -> anyhow::Result<()> {
    println!("=== config-rs 兼容性示例 ===\n");

    // 1. 创建配置文件（config-rs 格式）
    let config_content = r#"
name = "myapp"
version = "1.0.0"
debug = true
"#;

    std::fs::write("src/14-compat/configs/compat.toml", config_content)?;

    println!("1. 使用 confers 加载配置文件...");
    let config: AppConfig = AppConfig::load()?;

    println!("   ✅ 配置加载成功:");
    println!("      Name: {}", config.name);
    println!("      Version: {}", config.version);
    println!("      Debug: {}", config.debug);

    println!("\n=== 迁移指南 ===");
    println!("config-rs → confers 的主要差异:");
    println!("- 使用 #[derive(Config)] 替代手动配置");
    println!("- 自动格式检测，无需指定文件格式");
    println!("- 内置验证和环境变量支持");
    println!("- 更简洁的 API 设计");

    // 清理临时文件
    let _ = std::fs::remove_file("src/14-compat/configs/compat.toml");

    Ok(())
}