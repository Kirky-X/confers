// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! 配置快照持久化示例
//!
//! 本示例展示如何使用 confers 的 SnapshotManager 进行配置快照管理：
//! - 创建内存配置并写入值
//! - 构建配置树并保存快照（含敏感字段脱敏）
//! - 列出历史快照
//! - 清理超出上限的旧快照

use confers::i18n::{tr, tr_args};
use confers::snapshot::{SnapshotConfig, SnapshotFormat, SnapshotManager};
use confers::{AnnotatedValue, ConfigValue, ConfigWriter, SourceId, new_in_memory};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("========================================");
    println!("  {}", tr("example-snapshot-title"));
    println!("========================================\n");

    let config = new_in_memory();
    config
        .set(
            "server.host",
            AnnotatedValue::new(
                ConfigValue::string("0.0.0.0"),
                SourceId::new("demo"),
                "server.host",
            ),
        )
        .await?;
    config
        .set(
            "server.port",
            AnnotatedValue::new(
                ConfigValue::uint(8080),
                SourceId::new("demo"),
                "server.port",
            ),
        )
        .await?;
    println!("{}", tr("example-snapshot-in-memory-created"));

    let snapshot_value = AnnotatedValue::new(
        ConfigValue::map(vec![
            (
                "host",
                AnnotatedValue::new(
                    ConfigValue::string("0.0.0.0"),
                    SourceId::new("demo"),
                    "host",
                ),
            ),
            (
                "port",
                AnnotatedValue::new(ConfigValue::uint(8080), SourceId::new("demo"), "port"),
            ),
            (
                "password",
                AnnotatedValue::new(
                    ConfigValue::string("super-secret"),
                    SourceId::new("demo"),
                    "password",
                ),
            ),
        ]),
        SourceId::new("demo"),
        "",
    );

    let snapshot_dir =
        std::env::temp_dir().join(format!("confers-snapshot-demo-{}", std::process::id()));
    let manager = SnapshotManager::new(SnapshotConfig {
        dir: snapshot_dir.clone(),
        max_snapshots: 5,
        format: SnapshotFormat::Json,
        include_provenance: true,
    });
    println!(
        "{}",
        tr_args(
            "example-snapshot-manager-created",
            &[("dir", format!("{snapshot_dir:?}"))]
        )
    );

    let path = manager.save(&snapshot_value, &["password"]).await?;
    println!(
        "{}",
        tr_args(
            "example-snapshot-saved",
            &[(
                "file",
                format!("{:?}", path.file_name().unwrap_or_default())
            )]
        )
    );

    let content = std::fs::read_to_string(&path)?;
    assert!(content.contains("[REDACTED]"));
    assert!(!content.contains("super-secret"));
    println!("{}", tr("example-snapshot-redacted"));

    let snapshots = manager.list_snapshots()?;
    println!(
        "\n{}",
        tr_args(
            "example-snapshot-list-header",
            &[("count", snapshots.len().to_string())]
        )
    );
    for info in &snapshots {
        println!(
            "  {}",
            tr_args(
                "example-snapshot-list-row",
                &[
                    (
                        "name",
                        info.path.file_name().unwrap().to_string_lossy().to_string()
                    ),
                    ("size", info.size_bytes.to_string()),
                    (
                        "created",
                        info.created_at.format("%Y-%m-%d %H:%M:%S").to_string()
                    ),
                ]
            )
        );
    }

    println!("\n{}", tr("example-snapshot-cleanup-demo"));
    for i in 0..6u64 {
        let value = AnnotatedValue::new(
            ConfigValue::map(vec![(
                "version",
                AnnotatedValue::new(ConfigValue::uint(i), SourceId::new("demo"), "version"),
            )]),
            SourceId::new("demo"),
            "",
        );
        manager.save(&value, &[]).await?;
        // 间隔 1.1 秒确保文件名时间戳不同
        tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
    }

    let removed = manager.prune_old_snapshots()?;
    println!(
        "{}",
        tr_args(
            "example-snapshot-cleanup-removed",
            &[("count", removed.to_string())]
        )
    );

    let remaining = manager.list_snapshots()?;
    println!(
        "  {}",
        tr_args(
            "example-snapshot-remaining",
            &[("count", remaining.len().to_string())]
        )
    );

    let _ = std::fs::remove_dir_all(&snapshot_dir);

    println!("\n========================================");
    println!("  {}", tr("example-snapshot-completed"));
    println!("========================================");
    Ok(())
}
