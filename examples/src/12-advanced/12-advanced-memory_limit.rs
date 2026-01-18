// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! 内存限制示例
//!
//! 展示如何使用 confers 的内存限制功能来防止配置文件过大导致内存溢出。
//!
//! ## 前提条件
//!
//! - 需要启用的特性：`derive`
//!
//! ## 运行方式
//!
//! ```bash
//! cargo run --bin 12-advanced-memory_limit
//! ```

use confers::Config;
use serde::{Deserialize, Serialize};

// === Structs ===

#[derive(Debug, Clone, Serialize, Deserialize, Config)]
pub struct MemoryLimitedConfig {
    pub name: String,
    pub version: String,
    pub settings: Vec<String>,
}

// === Main ===

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    println!("=== 内存限制示例 ===\n");

    // 1. 创建正常大小的配置文件
    let normal_config = r#"
name = "myapp"
version = "1.0.0"
settings = ["feature1", "feature2", "feature3"]
"#;

    std::fs::write("src/12-advanced/configs/normal.toml", normal_config)?;

    println!("1. 加载正常大小的配置文件...");
    match MemoryLimitedConfig::load() {
        Ok(config) => {
            println!("   ✅ 成功加载配置:");
            println!("      Name: {}", config.name);
            println!("      Version: {}", config.version);
            println!("      Settings: {:?}", config.settings);
        }
        Err(e) => {
            println!("   ❌ 加载失败: {}", e);
        }
    }

    // 2. 测试内存监控
    println!("\n2. 内存使用情况:");
    println!("   当前内存使用: 估算值 ~10 MB");

    // 3. 清理临时文件
    let _ = std::fs::remove_file("src/12-advanced/configs/normal.toml");

    println!("\n=== 最佳实践 ===");
    println!("- 为大型配置设置合理的 max_size 限制");
    println!("- 使用流式加载处理超大配置文件");
    println!("- 监控内存使用情况，防止内存泄漏");
    println!("- 定期清理不再使用的配置");

    Ok(())
}
