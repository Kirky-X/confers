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
        assert!(first.exists(), "{ext} snapshot must land on disk");
        assert_eq!(
            first.extension().and_then(|e| e.to_str()),
            Some(ext),
            "extension must match the format"
        );
        let content = std::fs::read_to_string(&first).expect("read snapshot");
        assert!(
            content.contains("snapshot-value"),
            "{ext} content must contain the serialized value: {content}"
        );

        // 第二次保存触发保存路径内的 prune：max_snapshots=1 → 只保留最新
        let second = manager.save(&annotated(), &[]).await.expect("save2");
        assert!(second.exists());
        let remaining = std::fs::read_dir(&dir).expect("read dir").count();
        assert_eq!(
            remaining, 1,
            "only the newest snapshot is kept when max_snapshots=1; found {remaining} files"
        );
        // 显式 prune 入口幂等：已达上限时无可裁剪
        let pruned = manager.prune_old_snapshots().expect("prune");
        assert_eq!(
            pruned, 0,
            "explicit prune is a no-op once the cap is reached"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }
}
