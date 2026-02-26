//! Integration tests for Snapshot module.

#![cfg(feature = "snapshot")]

use confers::snapshot::{SnapshotConfig, SnapshotFormat, SnapshotManager};
use std::path::PathBuf;

#[test]
fn test_snapshot_format_ext() {
    assert_eq!(SnapshotFormat::Toml.ext(), "toml");
    assert_eq!(SnapshotFormat::Json.ext(), "json");
    assert_eq!(SnapshotFormat::Yaml.ext(), "yaml");
}

#[test]
fn test_snapshot_format_default() {
    let format = SnapshotFormat::default();
    assert!(matches!(format, SnapshotFormat::Toml));
}

#[test]
fn test_snapshot_config_default() {
    let config = SnapshotConfig::default();
    assert_eq!(config.dir, PathBuf::from("config-snapshots"));
    assert_eq!(config.max_snapshots, 30);
    assert!(config.include_provenance);
    assert!(matches!(config.format, SnapshotFormat::Toml));
}

#[test]
fn test_snapshot_config_new() {
    let config = SnapshotConfig::new("/tmp/snapshots");
    assert_eq!(config.dir, PathBuf::from("/tmp/snapshots"));
    assert_eq!(config.max_snapshots, 30); // default
}

#[test]
fn test_snapshot_manager_new() {
    let config = SnapshotConfig::new("/tmp/test-snapshots");
    let manager = SnapshotManager::new(config);
    assert_eq!(manager.config().dir, PathBuf::from("/tmp/test-snapshots"));
}

#[test]
fn test_snapshot_manager_default() {
    let manager = SnapshotManager::default();
    assert_eq!(manager.config().max_snapshots, 30);
}

#[test]
fn test_list_snapshots_empty() {
    let config = SnapshotConfig::new("/tmp/nonexistent-snapshots-test");
    let manager = SnapshotManager::new(config);
    
    let snapshots = manager.list_snapshots().unwrap();
    assert!(snapshots.is_empty());
}

#[test]
fn test_prune_empty_directory() {
    let config = SnapshotConfig::new("/tmp/nonexistent-snapshots-prune");
    let manager = SnapshotManager::new(config);
    
    let removed = manager.prune_old_snapshots().unwrap();
    assert_eq!(removed, 0);
}
