// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

use confers::audit::{AuditConfig, AuditLogger, Sanitize};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensitiveAuditConfig {
    pub username: String,
    #[serde(skip_serializing)]
    pub password: String,
    pub api_key: String,
    pub database_url: String,
}

impl Sanitize for SensitiveAuditConfig {
    fn sanitize(&self) -> Value {
        serde_json::json!({
            "username": self.username,
            "password": "***REDACTED***",
            "api_key": "***REDACTED***",
            "database_url": "***REDACTED***"
        })
    }
}

fn main() -> anyhow::Result<()> {
    println!("=== 敏感字段脱敏审计示例 ===\n");

    // 创建配置（包含敏感信息）
    let config = SensitiveAuditConfig {
        username: "admin".to_string(),
        password: "super_secret_password_123".to_string(),
        api_key: "sk-1234567890abcdef".to_string(),
        database_url: "postgres://user:password@localhost:5432/db".to_string(),
    };

    println!("原始配置:");
    println!("  - 用户名: {}", config.username);
    println!("  - 密码: {}", config.password);
    println!("  - API 密钥: {}", config.api_key);
    println!("  - 数据库 URL: {}", config.database_url);

    // 脱敏配置
    println!("\n--- 脱敏后的配置 ---");
    let sanitized = config.sanitize();
    println!("{}", serde_json::to_string_pretty(&sanitized)?);

    // 创建审计日志目录
    let audit_dir = PathBuf::from("src/08-audit/configs");
    std::fs::create_dir_all(&audit_dir)?;

    // 记录审计日志
    println!("\n--- 记录审计日志 ---");
    let audit_config = AuditConfig {
        validation_error: None,
        config_source: Some("config.toml".to_string()),
        load_duration: Some(Duration::from_millis(100)),
        config_sources_status: None,
        files_attempted: None,
        files_loaded: None,
        format_distribution: None,
        env_vars_count: None,
        memory_usage_mb: None,
    };

    let audit_log_path = audit_dir.join("audit.log");
    AuditLogger::log_to_file_with_source(&config, &audit_log_path, audit_config)?;
    println!("✓ 审计日志已记录到: {:?}", audit_log_path);

    // 读取审计日志
    println!("\n--- 审计日志内容 ---");
    if let Ok(log_content) = std::fs::read_to_string(&audit_log_path) {
        // 显示前 500 个字符
        let preview = if log_content.len() > 500 {
            &log_content[..500]
        } else {
            log_content.as_str()
        };
        println!("{}...", preview);
    }

    println!("\n=== 敏感字段脱敏审计示例完成 ===");
    println!("\n安全提示:");
    println!("⚠️  敏感字段在审计日志中已自动脱敏");
    println!("⚠️  原始敏感信息不会记录到日志文件中");
    println!("⚠️  审计日志应存储在安全位置，限制访问权限");
    println!("⚠️  定期审查审计日志以检测可疑活动");

    Ok(())
}
