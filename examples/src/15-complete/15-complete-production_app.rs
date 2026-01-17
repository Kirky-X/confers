// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! 生产环境应用示例
//!
//! 展示一个完整的生产环境应用配置，包含所有高级功能。
//!
//! ## 前提条件
//!
//! - 需要启用的特性：`derive`, `validation`
//!
//! ## 运行方式
//!
//! ```bash
//! cargo run --bin 15-complete-production_app
//! ```

use confers::Config;
use serde::{Deserialize, Serialize};

// === Structs ===

#[derive(Debug, Clone, Serialize, Deserialize, Config)]
pub struct ProductionConfig {
    pub app_name: String,
    pub version: String,
    pub http_port: u16,
    pub https_port: u16,
    pub workers: u32,
    pub database_url: String,
    pub redis_url: String,
    pub api_endpoint: String,
    pub admin_email: String,
}

// === Main ===

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    println!("=== 生产环境应用示例 ===\n");

    // 1. 创建生产环境配置
    let config_content = r#"
app_name = "production-app"
version = "1.0.0"
http_port = 8080
https_port = 8443
workers = 8
database_url = "postgres://user:pass@localhost:5432/prod"
redis_url = "redis://localhost:6379"
api_endpoint = "https://api.example.com"
admin_email = "admin@example.com"
"#;

    std::fs::write("src/15-complete/configs/prod.toml", config_content)?;

    println!("1. 加载生产环境配置...");
    let config = ProductionConfig::load()?;

    println!("   ✅ 配置加载成功");

    // 2. 显示配置摘要
    println!("\n2. 配置摘要:");
    println!("   应用: {} v{}", config.app_name, config.version);
    println!("   端口: HTTP={}, HTTPS={}", config.http_port, config.https_port);
    println!("   工作线程: {}", config.workers);
    println!("   API 端点: {}", config.api_endpoint);
    println!("   管理员: {}", config.admin_email);

    // 3. 清理临时文件
    let _ = std::fs::remove_file("src/15-complete/configs/prod.toml");

    println!("\n=== 生产环境最佳实践 ===");
    println!("- 使用加密保护敏感信息");
    println!("- 启用配置验证");
    println!("- 配置文件监控和热重载");
    println!("- 环境变量覆盖");
    println!("- 定期备份配置");
    println!("- 使用版本控制管理配置");

    Ok(())
}