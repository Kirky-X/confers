// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! 自定义验证器示例
//!
//! 展示如何创建和使用自定义验证器。
//!
//! ## 前提条件
//!
//! - 需要启用的特性：`validation`
//!
//! ## 运行方式
//!
//! ```bash
//! cargo run --example 02-validation-custom_validators --features validation
//! ```

use confers::Config;
use serde::{Deserialize, Serialize};

#[cfg(feature = "validation")]
use validator::Validate;

// === Structs ===

#[derive(Debug, Clone, Serialize, Deserialize, Config)]
#[config(env_prefix = "APP_")]
pub struct CustomValidationConfig {
    pub password: String,
    pub username: String,
    pub port: u16,
}

// === Custom Validators ===

#[cfg(feature = "validation")]
fn validate_password(password: &str) -> Result<(), validator::ValidationError> {
    if password.len() < 8 {
        return Err(validator::ValidationError::new("password_too_short"));
    }
    if !password.chars().any(|c| c.is_ascii_uppercase()) {
        return Err(validator::ValidationError::new("password_no_uppercase"));
    }
    if !password.chars().any(|c| c.is_ascii_digit()) {
        return Err(validator::ValidationError::new("password_no_digit"));
    }
    Ok(())
}

#[cfg(feature = "validation")]
fn validate_username(username: &str) -> Result<(), validator::ValidationError> {
    if username.is_empty() {
        return Err(validator::ValidationError::new("username_empty"));
    }
    if username.len() > 20 {
        return Err(validator::ValidationError::new("username_too_long"));
    }
    if !username
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
    {
        return Err(validator::ValidationError::new("username_invalid_chars"));
    }
    Ok(())
}

#[cfg(feature = "validation")]
fn validate_port(port: u16) -> Result<(), validator::ValidationError> {
    if port < 1024 {
        return Err(validator::ValidationError::new("port_privileged"));
    }
    if port > 65535 {
        return Err(validator::ValidationError::new("port_out_of_range"));
    }
    Ok(())
}

// === Main ===

fn main() -> anyhow::Result<()> {
    #[cfg(feature = "validation")]
    {
        tracing_subscriber::fmt::init();

        // 1. 测试有效配置
        println!("--- 测试有效配置 ---");
        let valid_content = r#"
username = "valid_user_123"
password = "SecurePass123"
port = 8080
"#;
        std::fs::write("src/02-validation/configs/custom_valid.toml", valid_content)?;

        let config = CustomValidationConfig::load()?;
        println!("✅ 配置加载成功");
        println!("  Username: {}", config.username);
        println!("  Port: {}", config.port);

        // 2. 测试无效密码
        println!("\n--- 测试无效密码（太短） ---");
        let invalid_content = r#"
username = "valid_user_123"
password = "short"
port = 8080
"#;
        std::fs::write(
            "src/02-validation/configs/custom_valid.toml",
            invalid_content,
        )?;

        let config = CustomValidationConfig::load()?;
        println!("❌ 密码太短");

        // 3. 测试无效用户名
        println!("\n--- 测试无效用户名（包含非法字符） ---");
        let invalid_content = r#"
username = "invalid@user"
password = "SecurePass123"
port = 8080
"#;
        std::fs::write(
            "src/02-validation/configs/custom_valid.toml",
            invalid_content,
        )?;

        let config = CustomValidationConfig::load()?;
        println!("❌ 用户名包含非法字符");

        // 4. 测试特权端口
        println!("\n--- 测试特权端口 ---");
        let invalid_content = r#"
username = "valid_user_123"
password = "SecurePass123"
port = 80
"#;
        std::fs::write(
            "src/02-validation/configs/custom_valid.toml",
            invalid_content,
        )?;

        let config = CustomValidationConfig::load()?;
        println!("❌ 端口号是特权端口");
    }

    #[cfg(not(feature = "validation"))]
    {
        println!("This example requires the 'validation' feature.");
        println!(
            "Run with: cargo run --example 02-validation-custom_validators --features validation"
        );
    }

    Ok(())
}
