// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! 验证错误处理示例
//!
//! 展示如何优雅地处理配置验证错误。
//!
//! ## 前提条件
//!
//! - 需要启用的特性：`validation`
//!
//! ## 运行方式
//!
//! ```bash
//! cargo run --example 02-validation-validation_errors --features validation
//! ```

use confers::Config;
use serde::{Deserialize, Serialize};

#[cfg(feature = "validation")]
use validator::Validate;

// === Structs ===

#[derive(Debug, Clone, Serialize, Deserialize, Config)]
#[config(env_prefix = "APP_")]
pub struct ErrorHandlingConfig {
    pub name: String,
    pub email: String,
    pub port: u16,
}

// === Main ===

fn main() -> anyhow::Result<()> {
    #[cfg(feature = "validation")]
    {
        tracing_subscriber::fmt::init();

        println!("=== 验证错误处理示例 ===\n");

        // 1. 加载有效配置
        println!("1. 加载有效配置:");
        let valid_content = r#"
name = "example"
email = "user@example.com"
port = 8080
"#;
        std::fs::write("src/02-validation/configs/error_handling.toml", valid_content)?;

        match ErrorHandlingConfig::load() {
            Ok(config) => {
                println!("   ✅ 配置有效");
                println!("   Name: {}, Email: {}, Port: {}", config.name, config.email, config.port);
            }
            Err(e) => println!("   ❌ 加载失败: {}", e),
        }

        // 2. 加载无效配置 - 多个错误
        println!("\n2. 加载无效配置（多个错误）:");
        let invalid_content = r#"
name = "x"
email = "invalid-email"
port = 70000
"#;
        std::fs::write("src/02-validation/configs/error_handling.toml", invalid_content)?;

        match ErrorHandlingConfig::load() {
            Ok(config) => {
                println!("   ❌ 发现验证错误");
            }
            Err(e) => println!("   ❌ 加载失败: {}", e),
        }

        // 3. 使用 Result 处理
        println!("\n3. 使用 Result 处理验证:");
        let invalid_content = r#"
name = "valid-name"
email = "valid@example.com"
port = 8080
"#;
        std::fs::write("src/02-validation/configs/error_handling.toml", invalid_content)?;

        let result = ErrorHandlingConfig::load();

        match result {
            Ok(config) => {
                println!("   ✅ 配置有效");
                println!("   Name: {}, Email: {}, Port: {}", config.name, config.email, config.port);
            }
            Err(e) => println!("   ❌ 配置无效: {}", e),
        }
    }

    #[cfg(not(feature = "validation"))]
    {
        println!("This example requires the 'validation' feature.");
        println!("Run with: cargo run --example 02-validation-validation_errors --features validation");
    }

    Ok(())
}