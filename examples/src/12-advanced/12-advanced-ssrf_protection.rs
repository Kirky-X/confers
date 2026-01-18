// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! SSRF 保护示例
//!
//! 展示如何使用 SSRF（服务端请求伪造）保护来防止配置中的恶意 URL。
//!
//! ## 前提条件
//!
//! - 需要启用的特性：`derive`, `remote`
//!
//! ## 运行方式
//!
//! ```bash
//! cargo run --bin 12-advanced-ssrf_protection
//! ```

#[cfg(feature = "remote")]
use confers::Config;
#[cfg(feature = "remote")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "remote")]
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
#[config(ssrf_protection = true, description = "配置结构，启用 SSRF 保护")]
pub struct SSRFProtectedConfig {
    #[config(
        validate = "url",
        ssrf_allowlist = "https://api.example.com,https://cdn.example.com",
        description = "API 端点（仅允许特定域名）"
    )]
    pub api_endpoint: String,

    #[config(
        validate = "url",
        ssrf_blocklist = "127.0.0.1,localhost,169.254.169.254",
        description = "CDN 端点（阻止内网地址）"
    )]
    pub cdn_endpoint: String,

    #[config(
        validate = "url",
        ssrf_allow_schemes = "https",
        description = "Webhook 端点（仅允许 HTTPS）"
    )]
    pub webhook_url: String,
}

#[cfg(not(feature = "remote"))]
fn main() {
    println!("请使用 --features remote 运行此示例");
}

#[cfg(feature = "remote")]
fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    println!("=== SSRF 保护示例 ===\n");

    // 1. 测试安全配置
    println!("1. 测试安全配置...");
    let safe_config = r#"
api_endpoint = "https://api.example.com/v1"
cdn_endpoint = "https://cdn.example.com/assets"
webhook_url = "https://hooks.example.com/webhook"
"#;

    std::fs::write("src/12-advanced/configs/ssrf_safe.toml", safe_config)?;

    match SSRFProtectedConfig::load_file("src/12-advanced/configs/ssrf_safe.toml").load() {
        Ok(config) => {
            println!("   ✅ 安全配置加载成功:");
            println!("      API Endpoint: {}", config.api_endpoint);
            println!("      CDN Endpoint: {}", config.cdn_endpoint);
            println!("      Webhook URL: {}", config.webhook_url);
        }
        Err(e) => {
            println!("   ❌ 加载失败: {}", e);
        }
    }

    // 2. 测试内网地址（应该被阻止）
    println!("\n2. 测试内网地址（应该被阻止）...");
    let internal_config = r#"
api_endpoint = "https://api.example.com/v1"
cdn_endpoint = "http://127.0.0.1:8080/assets"
webhook_url = "https://hooks.example.com/webhook"
"#;

    std::fs::write(
        "src/12-advanced/configs/ssrf_internal.toml",
        internal_config,
    )?;

    match SSRFProtectedConfig::load_file("src/12-advanced/configs/ssrf_internal.toml").load() {
        Ok(_) => {
            println!("   ❌ 意外：内网地址未被阻止");
        }
        Err(e) => {
            println!("   ✅ 正确阻止内网地址");
            println!("      错误信息: {}", e);
        }
    }

    // 3. 测试元数据服务（应该被阻止）
    println!("\n3. 测试元数据服务（应该被阻止）...");
    let metadata_config = r#"
api_endpoint = "https://api.example.com/v1"
cdn_endpoint = "https://cdn.example.com/assets"
webhook_url = "http://169.254.169.254/latest/meta-data/"
"#;

    std::fs::write(
        "src/12-advanced/configs/ssrf_metadata.toml",
        metadata_config,
    )?;

    match SSRFProtectedConfig::load_file("src/12-advanced/configs/ssrf_metadata.toml").load() {
        Ok(_) => {
            println!("   ❌ 意外：元数据服务地址未被阻止");
        }
        Err(e) => {
            println!("   ✅ 正确阻止元数据服务地址");
            println!("      错误信息: {}", e);
        }
    }

    // 4. 测试 HTTP 协议（应该被阻止，仅允许 HTTPS）
    println!("\n4. 测试 HTTP 协议（应该被阻止，仅允许 HTTPS）...");
    let http_config = r#"
api_endpoint = "https://api.example.com/v1"
cdn_endpoint = "https://cdn.example.com/assets"
webhook_url = "http://insecure.example.com/webhook"
"#;

    std::fs::write("src/12-advanced/configs/ssrf_http.toml", http_config)?;

    match SSRFProtectedConfig::load_file("src/12-advanced/configs/ssrf_http.toml").load() {
        Ok(_) => {
            println!("   ❌ 意外：HTTP URL 未被阻止");
        }
        Err(e) => {
            println!("   ✅ 正确阻止 HTTP URL");
            println!("      错误信息: {}", e);
        }
    }

    // 5. 清理临时文件
    let _ = std::fs::remove_file("src/12-advanced/configs/ssrf_safe.toml");
    let _ = std::fs::remove_file("src/12-advanced/configs/ssrf_internal.toml");
    let _ = std::fs::remove_file("src/12-advanced/configs/ssrf_metadata.toml");
    let _ = std::fs::remove_file("src/12-advanced/configs/ssrf_http.toml");

    println!("\n=== SSRF 保护的最佳实践 ===");
    println!("- 使用白名单机制，只允许受信任的域名");
    println!("- 使用黑名单机制，阻止已知危险的内网地址");
    println!("- 限制允许的协议（如仅允许 HTTPS）");
    println!("- 定期审查和更新白名单/黑名单");
    println!("- 对所有外部 URL 进行验证和清理");

    Ok(())
}
