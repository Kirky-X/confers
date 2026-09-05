// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! E2E: 快照(tests/e2e/snapshot_e2e.rs)
//!
//! 场景固化(docs/TEST_SCENARIOS.md §2.14):
//! - SNP-01 `SnapshotManager::save` JSON 落盘,文件名含时间戳
//! - SNP-02 `SnapshotFormat::{Toml,Yaml,Json}` save→load roundtrip
//! - SNP-03 `list_snapshots` 按时间倒序返回 `SnapshotInfo`
//! - SNP-04 `load_snapshot` 恢复为 `AnnotatedValue` 且与保存值等价
//! - SNP-05 `prune_old_snapshots` 清理超限快照并返回数量
//! - SNP-06 快照目录不存在 → list 返回空列表 / prune 返回 0(不 panic)
//! - SNP-07 `load_snapshot(不存在文件)` → 错误
//! - SNP-08 损坏快照(垃圾内容)load → 解析错误
//! - SNP-09 `max_snapshots` 上限:保存 N+1 份后最旧消失
//! - SNP-10 `ConfigBuilder::with_snapshot` 构建时自动快照端到端(构建→文件存在→可回放)
//!
//! SNP-10 依赖修复提交:`with_snapshot` 此前为静默 no-op,
//! 现经 `SnapshotManager::save_blocking` 接入构建管线。

use confers::snapshot::{SnapshotConfig, SnapshotFormat, SnapshotManager};
use confers::types::{ConfigValue, SourceId};
use confers::{AnnotatedValue, ConfigBuilder};

fn sample_value() -> AnnotatedValue {
    AnnotatedValue::new(
        ConfigValue::map(vec![
            (
                "host",
                AnnotatedValue::new(
                    ConfigValue::string("db.example"),
                    SourceId::new("test"),
                    "host",
                ),
            ),
            (
                "port",
                AnnotatedValue::new(ConfigValue::integer(5432), SourceId::new("test"), "port"),
            ),
        ]),
        SourceId::new("test"),
        "",
    )
}

#[tokio::test]
async fn snp01_save_json_writes_timestamped_file() {
    let dir = tempfile::tempdir().unwrap();
    let config = SnapshotConfig {
        dir: dir.path().join("snaps"),
        format: SnapshotFormat::Json,
        include_provenance: false,
        ..SnapshotConfig::default()
    };
    let manager = SnapshotManager::new(config);

    let path = manager
        .save(&sample_value(), &[])
        .await
        .expect("save must succeed");

    assert!(path.exists(), "snapshot file must exist");
    let name = path.file_name().unwrap().to_string_lossy().into_owned();
    assert!(name.starts_with("config-"), "timestamped name: {name}");
    assert!(name.ends_with(".json"));

    let body = std::fs::read_to_string(&path).unwrap();
    let json: serde_json::Value = serde_json::from_str(&body).expect("valid json body");
    // save 落盘的是去标注的配置树(快照的公开契约)。
    assert_eq!(json["host"], "db.example");
    assert_eq!(json["port"], 5432);
}

#[tokio::test]
async fn snp02_all_formats_roundtrip() {
    for format in [
        SnapshotFormat::Toml,
        SnapshotFormat::Yaml,
        SnapshotFormat::Json,
    ] {
        let dir = tempfile::tempdir().unwrap();
        let config = SnapshotConfig {
            dir: dir.path().join("snaps"),
            format,
            include_provenance: false,
            ..SnapshotConfig::default()
        };
        let manager = SnapshotManager::new(config);

        let path = manager.save(&sample_value(), &[]).await.expect("save");
        let loaded = manager.load_snapshot(&path).await.expect("load");

        assert_eq!(
            loaded.to_json()["host"].as_str(),
            Some("db.example"),
            "{format:?} roundtrip"
        );
        assert_eq!(loaded.to_json()["port"].as_i64(), Some(5432));
    }
}

#[tokio::test]
async fn snp03_list_snapshots_returns_newest_first() {
    let dir = tempfile::tempdir().unwrap();
    let config = SnapshotConfig {
        dir: dir.path().join("snaps"),
        format: SnapshotFormat::Json,
        include_provenance: false,
        ..SnapshotConfig::default()
    };
    let manager = SnapshotManager::new(config);

    for i in 0..3 {
        manager.save(&sample_value(), &[]).await.expect("save");
        if i < 2 {
            // 文件名含毫秒纳秒与序号,但仍留出 mtime 分辨率。
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    }

    let list = manager.list_snapshots().expect("list");
    assert_eq!(list.len(), 3);
    for pair in list.windows(2) {
        assert!(
            pair[0].created_at >= pair[1].created_at,
            "list must be newest-first"
        );
    }
    assert!(list[0].size_bytes > 0);
}

#[tokio::test]
async fn snp04_load_snapshot_restores_equivalent_value() {
    let dir = tempfile::tempdir().unwrap();
    let config = SnapshotConfig {
        dir: dir.path().join("snaps"),
        format: SnapshotFormat::Json,
        include_provenance: false,
        ..SnapshotConfig::default()
    };
    let manager = SnapshotManager::new(config);

    let original = sample_value();
    let path = manager.save(&original, &[]).await.expect("save");
    let loaded = manager.load_snapshot(&path).await.expect("load");

    assert_eq!(
        original.to_json(),
        loaded.to_json(),
        "restored value must equal saved value"
    );
    // 溯源信息随 AnnotatedValue 结构保留。
    assert!(!loaded.all_paths().is_empty());
}

#[tokio::test]
async fn snp05_prune_old_snapshots_returns_pruned_count() {
    let dir = tempfile::tempdir().unwrap();
    let snap_dir = dir.path().join("snaps");
    let config = SnapshotConfig {
        dir: snap_dir.clone(),
        format: SnapshotFormat::Json,
        include_provenance: false,
        max_snapshots: 5,
    };
    let manager = SnapshotManager::new(config);

    for _ in 0..5 {
        manager.save(&sample_value(), &[]).await.unwrap();
    }
    assert_eq!(manager.list_snapshots().unwrap().len(), 5);

    // 外部一次性塞入超限旧快照,再断言 prune 清理并返回数量。
    for i in 0..3 {
        std::fs::write(snap_dir.join(format!("config-manual-{i}.json")), "{}").unwrap();
    }
    let pruned = manager.prune_old_snapshots().expect("prune");
    assert_eq!(pruned, 3, "8 files over max 5 → prune 3");
    assert_eq!(manager.list_snapshots().unwrap().len(), 5);
}

#[test]
fn snp06_missing_directory_is_empty_not_error() {
    let dir = tempfile::tempdir().unwrap();
    let manager = SnapshotManager::new(SnapshotConfig::new(dir.path().join("never-created")));

    let list = manager.list_snapshots().expect("missing dir → empty list");
    assert!(list.is_empty());

    let pruned = manager
        .prune_old_snapshots()
        .expect("missing dir → prune 0");
    assert_eq!(pruned, 0);
}

#[tokio::test]
async fn snp07_load_nonexistent_snapshot_is_an_error() {
    let dir = tempfile::tempdir().unwrap();
    let manager = SnapshotManager::new(SnapshotConfig::new(dir.path().join("snaps")));

    let missing = dir.path().join("snaps").join("no-such-snapshot.json");
    let err = manager
        .load_snapshot(&missing)
        .await
        .expect_err("missing file must error");
    assert!(
        matches!(err, confers::ConfigError::IoError(_)),
        "expected IoError, got: {err:?}"
    );
}

#[tokio::test]
async fn snp08_corrupt_snapshot_fails_to_parse() {
    let dir = tempfile::tempdir().unwrap();
    let snap_dir = dir.path().join("snaps");
    std::fs::create_dir_all(&snap_dir).unwrap();
    let corrupt = snap_dir.join("config-corrupt.json");
    std::fs::write(&corrupt, "{{{ not json at all }}}").unwrap();

    let manager = SnapshotManager::new(SnapshotConfig {
        dir: snap_dir.clone(),
        format: SnapshotFormat::Json,
        ..SnapshotConfig::default()
    });
    let err = manager
        .load_snapshot(&corrupt)
        .await
        .expect_err("corrupt snapshot must fail");
    assert!(
        matches!(err, confers::ConfigError::ParseError { .. }),
        "expected ParseError, got: {err:?}"
    );
}

#[tokio::test]
async fn snp09_max_snapshots_rolls_off_oldest() {
    let dir = tempfile::tempdir().unwrap();
    let snap_dir = dir.path().join("snaps");
    // save() 内部会自动 prune 到 max_snapshots。
    let config = SnapshotConfig {
        dir: snap_dir.clone(),
        format: SnapshotFormat::Json,
        include_provenance: false,
        max_snapshots: 3,
    };
    let manager = SnapshotManager::new(config);

    let mut first_path: Option<std::path::PathBuf> = None;
    for i in 0..4 {
        let path = manager.save(&sample_value(), &[]).await.unwrap();
        if i == 0 {
            first_path = Some(path);
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }

    let first_path = first_path.expect("first snapshot path");
    let list = manager.list_snapshots().unwrap();
    assert_eq!(list.len(), 3, "rolling window keeps max_snapshots entries");
    assert!(
        !list.iter().any(|s| s.path == first_path),
        "oldest snapshot must have been pruned"
    );
}

#[tokio::test]
async fn snp10_with_snapshot_auto_snapshots_on_build() {
    let dir = tempfile::tempdir().unwrap();
    let snap_dir = dir.path().join("auto-snaps");
    let config_path = dir.path().join("app.toml");
    std::fs::write(&config_path, "name = \"snapshotted\"\nport = 80\n").unwrap();

    #[derive(Debug, serde::Deserialize, Default)]
    struct App {
        name: String,
        port: u16,
    }

    let snapshot_config = SnapshotConfig {
        dir: snap_dir.clone(),
        format: SnapshotFormat::Json,
        include_provenance: false,
        ..SnapshotConfig::default()
    };

    let app: App = ConfigBuilder::new()
        .allow_absolute_paths()
        .with_snapshot(snapshot_config)
        .file(&config_path)
        .build()
        .expect("build with snapshot");
    assert_eq!(app.name, "snapshotted");
    assert_eq!(app.port, 80);

    // 构建 → 快照文件存在 → 可回放。
    let manager = SnapshotManager::new(SnapshotConfig {
        dir: snap_dir.clone(),
        format: SnapshotFormat::Json,
        include_provenance: false,
        ..SnapshotConfig::default()
    });
    let list = manager
        .list_snapshots()
        .expect("snapshots written by build");
    assert_eq!(list.len(), 1, "exactly one auto-snapshot: {list:?}");

    let restored = manager.load_snapshot(&list[0].path).await.expect("replay");
    assert_eq!(restored.to_json()["name"], "snapshotted");
    assert_eq!(restored.to_json()["port"], 80);
}
