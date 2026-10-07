// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! 快照管理面覆盖：三种序列化格式的保存分支 + 原子写 + prune 保留策略。
//!
//! `SnapshotManager` 此前无测试：`save()` 的 Json/Toml/Yaml 序列化分支、
//! spawn_blocking 原子写与 prune 均未执行。

use std::sync::Arc;

use confers::{
    AnnotatedValue, ConfigValue, SnapshotConfig, SnapshotFormat, SnapshotManager, SourceId,
};

/// 根为 Map（TOML 不接受标量根，需表/文档根）
fn annotated() -> AnnotatedValue {
    let mut map: indexmap::IndexMap<Arc<str>, AnnotatedValue> = indexmap::IndexMap::new();
    map.insert(
        Arc::from("key"),
        AnnotatedValue::new(
            ConfigValue::from("snapshot-value"),
            SourceId::new("snapshot-test"),
            "key",
        ),
    );
    AnnotatedValue::new(
        ConfigValue::Map(Arc::new(map)),
        SourceId::new("snapshot-test"),
        "root",
    )
}

fn temp_dir(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("confers-snap-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    dir
}

#[tokio::test]
async fn snapshot_save_covers_all_formats_and_prune_keeps_latest() {
    let formats = [
        (SnapshotFormat::Json, "json"),
        (SnapshotFormat::Toml, "toml"),
        (SnapshotFormat::Yaml, "yaml"),
    ];

    for (format, ext) in formats {
        let dir = temp_dir(ext);
        let manager = SnapshotManager::new(SnapshotConfig {
            dir: dir.clone(),
            max_snapshots: 1,
            format,
            include_provenance: true,
        });

        let first = manager.save(&annotated(), &[]).await.expect("save");
        assert!(first.exists(), "{ext} 快照应落盘");
        assert_eq!(
            first.extension().and_then(|e| e.to_str()),
            Some(ext),
            "扩展名应与格式一致"
        );
        let content = std::fs::read_to_string(&first).expect("read snapshot");
        assert!(
            content.contains("snapshot-value"),
            "{ext} 内容应含序列化值: {content}"
        );

        // 第二次保存触发保存路径内的 prune：max_snapshots=1 → 只保留最新
        let second = manager.save(&annotated(), &[]).await.expect("save2");
        assert!(second.exists());
        let remaining = std::fs::read_dir(&dir).expect("read dir").count();
        assert_eq!(
            remaining, 1,
            "max_snapshots=1 时只应保留最新快照，实际 {remaining} 个文件"
        );
        // 显式 prune 入口幂等：已达上限时无可裁剪
        let pruned = manager.prune_old_snapshots().expect("prune");
        assert_eq!(pruned, 0, "已达上限后显式 prune 应为 no-op");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
