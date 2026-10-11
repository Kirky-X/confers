// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! 本示例展示 confers 的审计日志功能：
//! - 使用 `AuditConfigBuilder` 构建 `AuditConfig`
//! - 使用 `AuditWriterBuilder` 构建 `AuditWriter`
//! - 创建不同类型的 `AuditEvent`
//! - 展示 `AuditLevel`（Durable / BestEffort）的事件映射
//! - 将审计事件写入日志文件并查看输出

use chrono::Utc;
use confers::audit::{AuditConfig, AuditEvent, AuditLevel, AuditWriter};
use confers::i18n::{tr, tr_args};
use std::fs;
use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    println!("========================================");
    println!("  {}", tr("example-audit-title"));
    println!("========================================");

    let log_dir = std::env::temp_dir().join("confers_audit_example");
    fs::create_dir_all(&log_dir)?;
    println!(
        "\n{}",
        tr_args(
            "example-audit-log-dir",
            &[("path", log_dir.display().to_string())]
        )
    );

    println!("\n{}", tr("example-audit-build-config"));
    let config = AuditConfig::builder()
        .enabled(true)
        .log_dir(log_dir.clone())
        .build();
    println!("  enabled: {}", config.enabled);

    println!("\n{}", tr("example-audit-build-writer"));
    let writer = AuditWriter::builder()
        .enabled(true)
        .log_dir(log_dir.clone())
        .build();
    println!("  enabled: {}", writer.is_enabled());

    println!("\n{}", tr("example-audit-level-mapping"));
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
        println!(
            "  {}",
            tr_args(
                "example-audit-event-level",
                &[
                    ("index", (i + 1).to_string()),
                    ("level", level_str.to_string())
                ]
            )
        );
    }

    println!("\n{}", tr("example-audit-write-events"));
    let _ = writer.log_load("config.toml");
    let _ = writer.log_key_access("master-key");
    let _ = writer.log_decrypt("api_token", true);
    let _ = writer.log_decrypt("password_field", false); // 敏感字段名会被脱敏
    let _ = writer.log_key_rotation("v1", "v2");
    let _ = writer.write(AuditEvent::ReloadTrigger {
        source: "watcher".to_string(),
        timestamp: Utc::now(),
    });
    println!("  {}", tr("example-audit-wrote-events"));

    let filename = format!("audit_{}.log", Utc::now().format("%Y%m%d"));
    let log_path: PathBuf = log_dir.join(&filename);
    println!(
        "\n{}",
        tr_args(
            "example-audit-log-contents",
            &[("path", log_path.display().to_string())]
        )
    );
    match fs::read_to_string(&log_path) {
        Ok(content) => {
            for (i, line) in content.lines().enumerate() {
                println!(
                    "  {}",
                    tr_args(
                        "example-audit-log-line",
                        &[("index", (i + 1).to_string()), ("line", line.to_string())]
                    )
                );
            }
            if content.is_empty() {
                println!("  {}", tr("example-audit-log-empty"));
            }
        }
        Err(e) => println!(
            "  {}",
            tr_args("example-audit-read-log-failed", &[("error", e.to_string())])
        ),
    }

    let _ = fs::remove_dir_all(&log_dir);

    println!("\n========================================");
    println!("  {}", tr("example-audit-completed"));
    println!("========================================");
    Ok(())
}
