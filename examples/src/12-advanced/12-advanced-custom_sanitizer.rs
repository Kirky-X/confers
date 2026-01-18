// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! 自定义清理器示例
//!
//! 展示如何使用自定义清理器来处理配置数据，包括：
//! - 去除空白字符
//! - 转换大小写
//! - 验证和清理格式
//!
//! ## 前提条件
//!
//! - 需要启用的特性：`derive`
//!
//! ## 运行方式
//!
//! ```bash
//! cargo run --bin 12-advanced-custom_sanitizer
//! ```

use confers::Config;
use serde::{Deserialize, Serialize};

// === Structs ===

#[derive(Debug, Clone, Serialize, Deserialize, Config)]
pub struct SanitizerConfig {
    #[config(sanitize = "trim", description = "应用名称，自动去除前后空白")]
    pub app_name: String,

    #[config(sanitize = "lowercase", description = "环境名称，自动转换为小写")]
    pub environment: String,

    #[config(sanitize = "uppercase", description = "API 密钥，自动转换为大写")]
    pub api_key: String,

    #[config(
        sanitize = "normalize_path",
        description = "日志路径，自动规范化路径分隔符"
    )]
    pub log_path: String,
}

// === Main ===

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    println!("=== 自定义清理器示例 ===\n");

    // 1. 创建包含需要清理的数据的配置文件
    let config_content = r#"
# 注意：这些值包含需要清理的空白和大小写问题
app_name = "  My Awesome App  "
environment = "PRODUCTION"
api_key = "secret_key_123"
log_path = "./logs//app.log"
"#;

    std::fs::write("src/12-advanced/configs/sanitizer.toml", config_content)?;

    println!("原始配置内容:");
    println!("  app_name: '  My Awesome App  ' (带前后空白)");
    println!("  environment: 'PRODUCTION' (大写)");
    println!("  api_key: 'secret_key_123' (混合大小写)");
    println!("  log_path: './logs//app.log' (双斜杠)\n");

    // 2. 加载配置（自动应用清理器）
    println!("加载配置并应用清理器...");
    let config = SanitizerConfig::load()?;

    // 3. 显示清理后的结果
    println!("\n清理后的配置:");
    println!("  app_name: '{}' (已去除空白)", config.app_name);
    println!("  environment: '{}' (已转为小写)", config.environment);
    println!("  api_key: '{}' (已转为大写)", config.api_key);
    println!("  log_path: '{}' (已规范化路径)", config.log_path);

    // 4. 验证清理效果
    println!("\n=== 验证清理效果 ===");
    assert_eq!(config.app_name, "My Awesome App", "应用名称应去除前后空白");
    assert_eq!(config.environment, "production", "环境名称应转为小写");
    assert_eq!(config.api_key, "SECRET_KEY_123", "API 密钥应转为大写");
    assert!(!config.log_path.contains("//"), "路径应规范化");

    println!("✅ 所有断言通过！");

    // 5. 清理临时文件
    let _ = std::fs::remove_file("src/12-advanced/configs/sanitizer.toml");

    println!("\n=== 清理器类型说明 ===");
    println!("- trim: 去除字符串前后的空白字符");
    println!("- lowercase: 将字符串转换为小写");
    println!("- uppercase: 将字符串转换为大写");
    println!("- normalize_path: 规范化路径分隔符（将 // 替换为 /）");

    Ok(())
}
