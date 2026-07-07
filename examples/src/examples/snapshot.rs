// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! 配置快照持久化示例
//!
//! 本示例展示如何使用 confers 的 SnapshotManager 进行配置快照管理：
//! - 创建内存配置并写入值
//! - 构建配置树并保存快照（含敏感字段脱敏）
//! - 列出历史快照
//! - 清理超出上限的旧快照

use confers::snapshot::{SnapshotConfig, SnapshotFormat, SnapshotManager};
use confers::{new_in_memory, AnnotatedValue, ConfigValue, ConfigWriter, SourceId};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("========================================");
    println!("  配置快照持久化示例");
    println!("========================================\n");

    // 1. 使用 confers 创建内存配置并设置值
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
    println!("✓ 内存配置已创建，写入了 server.host 和 server.port");

    // 2. 构建用于快照的 AnnotatedValue 树（含敏感字段 password）
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

    // 3. 创建快照管理器（输出到临时目录）
    let snapshot_dir =
        std::env::temp_dir().join(format!("confers-snapshot-demo-{}", std::process::id()));
    let manager = SnapshotManager::new(SnapshotConfig {
        dir: snapshot_dir.clone(),
        max_snapshots: 5,
        format: SnapshotFormat::Json,
        include_provenance: true,
    });
    println!("✓ 快照管理器已创建，输出目录: {:?}", snapshot_dir);

    // 4. 保存快照（脱敏 password 字段）
    let path = manager.save(&snapshot_value, &["password"]).await?;
    println!("✓ 快照已保存: {:?}", path.file_name().unwrap_or_default());

    // 验证脱敏效果
    let content = std::fs::read_to_string(&path)?;
    assert!(content.contains("[REDACTED]"));
    assert!(!content.contains("super-secret"));
    println!("✓ 敏感字段 password 已脱敏为 [REDACTED]");

    // 5. 列出快照
    let snapshots = manager.list_snapshots()?;
    println!("\n--- 快照列表（共 {} 个）---", snapshots.len());
    for info in &snapshots {
        println!(
            "  {} | {} 字节 | {}",
            info.path.file_name().unwrap().to_string_lossy(),
            info.size_bytes,
            info.created_at.format("%Y-%m-%d %H:%M:%S")
        );
    }

    // 6. 演示清理：多保存几个快照触发自动清理
    println!("\n--- 演示快照清理（max_snapshots=5）---");
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
    println!("✓ 手动清理删除了 {} 个旧快照", removed);

    let remaining = manager.list_snapshots()?;
    println!("  当前剩余快照数: {}", remaining.len());

    // 清理临时目录
    let _ = std::fs::remove_dir_all(&snapshot_dir);

    println!("\n========================================");
    println!("  示例运行完成");
    println!("========================================");
    Ok(())
}
