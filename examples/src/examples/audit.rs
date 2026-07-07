// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! 本示例展示 confers 的审计日志功能：
//! - 使用 `AuditConfigBuilder` 构建 `AuditConfig`
//! - 使用 `AuditWriterBuilder` 构建 `AuditWriter`
//! - 创建不同类型的 `AuditEvent`
//! - 展示 `AuditLevel`（Durable / BestEffort）的事件映射
//! - 将审计事件写入日志文件并查看输出

use chrono::Utc;
use confers::audit::{AuditConfig, AuditEvent, AuditLevel, AuditWriter};
use std::fs;
use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    println!("========================================");
    println!("  Audit - 审计日志示例");
    println!("========================================");

    // 创建临时日志目录
    let log_dir = std::env::temp_dir().join("confers_audit_example");
    fs::create_dir_all(&log_dir)?;
    println!("\n日志目录: {}", log_dir.display());

    // 1. 使用 AuditConfigBuilder 构建 AuditConfig
    println!("\n[构建 AuditConfig]");
    let config = AuditConfig::builder()
        .enabled(true)
        .log_dir(log_dir.clone())
        .durable_wal(true)
        .channel_size(2048)
        .build();
    println!("  enabled: {}", config.enabled);
    println!("  channel_size: {}", config.channel_size);

    // 2. 使用 AuditWriterBuilder 构建 AuditWriter
    println!("\n[构建 AuditWriter]");
    let writer = AuditWriter::builder()
        .enabled(true)
        .log_dir(log_dir.clone())
        .durable_wal(true)
        .build();
    println!("  已启用: {}", writer.is_enabled());

    // 3. 展示 AuditLevel 映射（Durable / BestEffort）
    println!("\n[AuditLevel 事件映射]");
    let now = Utc::now();
    let events: Vec<AuditEvent> = vec![
        AuditEvent::KeyAccess {
            key: "master-key".to_string(),
            timestamp: now,
        },
        AuditEvent::KeyRotation {
            old_version: "v1".to_string(),
            new_version: "v2".to_string(),
            timestamp: now,
        },
        AuditEvent::Decrypt {
            field: "api_token".to_string(),
            success: true,
            timestamp: now,
        },
        AuditEvent::LoadSuccess {
            source: "config.toml".to_string(),
            timestamp: now,
        },
        AuditEvent::ReloadTrigger {
            source: "watcher".to_string(),
            timestamp: now,
        },
    ];
    for (i, event) in events.iter().enumerate() {
        let level = AuditLevel::for_event(event);
        let level_str = match level {
            AuditLevel::Durable => "Durable",
            AuditLevel::BestEffort => "BestEffort",
        };
        println!("  事件 {} -> {}", i + 1, level_str);
    }

    // 4. 使用便捷方法写入审计事件
    println!("\n[写入审计事件]");
    writer.log_load("config.toml");
    writer.log_key_access("master-key");
    writer.log_decrypt("api_token", true);
    writer.log_decrypt("password_field", false); // 敏感字段名会被脱敏
    writer.log_key_rotation("v1", "v2");
    writer.write(AuditEvent::ReloadTrigger {
        source: "watcher".to_string(),
        timestamp: Utc::now(),
    });
    println!("  已写入 6 条审计事件");

    // 5. 读取并展示日志文件内容
    let filename = format!("audit_{}.log", Utc::now().format("%Y%m%d"));
    let log_path: PathBuf = log_dir.join(&filename);
    println!("\n[审计日志内容] {}", log_path.display());
    match fs::read_to_string(&log_path) {
        Ok(content) => {
            for (i, line) in content.lines().enumerate() {
                println!("  行 {}: {}", i + 1, line);
            }
            if content.is_empty() {
                println!("  (日志文件为空)");
            }
        }
        Err(e) => println!("  读取日志失败: {}", e),
    }

    // 6. 清理临时目录
    let _ = fs::remove_dir_all(&log_dir);

    println!("\n========================================");
    println!("  示例运行完成!");
    println!("========================================");
    Ok(())
}
