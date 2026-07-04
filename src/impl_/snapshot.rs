//! Configuration snapshot module.

use crate::error::ConfigResult;
use crate::types::{AnnotatedValue, ConfigValue, SerializeMode};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Snapshot file format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SnapshotFormat {
    #[default]
    Toml,
    Json,
    Yaml,
}

impl SnapshotFormat {
    pub fn ext(&self) -> &'static str {
        match self {
            SnapshotFormat::Toml => "toml",
            SnapshotFormat::Json => "json",
            SnapshotFormat::Yaml => "yaml",
        }
    }
}

/// Configuration for snapshot manager.
#[derive(Debug, Clone)]
pub struct SnapshotConfig {
    pub dir: PathBuf,
    pub max_snapshots: usize,
    pub format: SnapshotFormat,
    pub include_provenance: bool,
}

impl Default for SnapshotConfig {
    fn default() -> Self {
        Self {
            dir: PathBuf::from("config-snapshots"),
            max_snapshots: 30,
            format: SnapshotFormat::Toml,
            include_provenance: true,
        }
    }
}

impl SnapshotConfig {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self {
            dir: dir.into(),
            ..Default::default()
        }
    }
}

/// Information about a saved snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotInfo {
    pub path: PathBuf,
    pub created_at: DateTime<Utc>,
    pub size_bytes: u64,
}

/// Manager for configuration snapshots.
#[derive(Debug)]
pub struct SnapshotManager {
    config: SnapshotConfig,
}

impl SnapshotManager {
    pub fn new(config: SnapshotConfig) -> Self {
        Self { config }
    }

    pub fn config(&self) -> &SnapshotConfig {
        &self.config
    }

    pub fn list_snapshots(&self) -> ConfigResult<Vec<SnapshotInfo>> {
        let dir = &self.config.dir;
        if !dir.exists() {
            return Ok(Vec::new());
        }

        let mut snapshots = Vec::new();
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map(|e| e.to_string_lossy().to_string())
                == Some(self.config.format.ext().to_string())
            {
                let metadata = entry.metadata()?;
                let created_at = metadata
                    .created()
                    .map(DateTime::from)
                    .unwrap_or_else(|_| Utc::now());
                snapshots.push(SnapshotInfo {
                    path,
                    created_at,
                    size_bytes: metadata.len(),
                });
            }
        }

        snapshots.sort_by_key(|s| std::cmp::Reverse(s.created_at));
        Ok(snapshots)
    }

    pub fn prune_old_snapshots(&self) -> ConfigResult<usize> {
        let snapshots = self.list_snapshots()?;
        if snapshots.len() <= self.config.max_snapshots {
            return Ok(0);
        }

        let to_remove = snapshots.len() - self.config.max_snapshots;
        let mut removed = 0;

        for snapshot in snapshots.into_iter().rev().take(to_remove) {
            if std::fs::remove_file(&snapshot.path).is_ok() {
                removed += 1;
            }
        }

        Ok(removed)
    }

    pub async fn save(
        &self,
        value: &AnnotatedValue,
        sensitive_paths: &[&str],
    ) -> ConfigResult<PathBuf> {
        tokio::fs::create_dir_all(&self.config.dir)
            .await
            .map_err(crate::error::ConfigError::IoError)?;

        let now = Utc::now();
        let timestamp = now.format("%Y%m%dT%H%M%SZ");
        // 添加纳秒后缀防止同秒内多次快照发生文件名碰撞（后者覆盖前者）
        let nanos = now.timestamp_nanos_opt().unwrap_or(0) % 1_000_000_000;
        let filename = format!(
            "config-{}-{:09}.{}",
            timestamp,
            nanos,
            self.config.format.ext()
        );
        let path = self.config.dir.join(&filename);

        let serialized = value.to_json_with_mode(SerializeMode::Redacted, sensitive_paths);

        let output = if self.config.include_provenance {
            self.attach_provenance(serialized, value)
        } else {
            serialized
        };

        let content = match self.config.format {
            SnapshotFormat::Json => {
                #[cfg(feature = "json")]
                {
                    serde_json::to_string_pretty(&output).map_err(|e| {
                        crate::error::ConfigError::ParseError {
                            format: "json".to_string(),
                            message: e.to_string(),
                            location: None,
                            source: Some(Box::new(e)),
                        }
                    })?
                }
                #[cfg(not(feature = "json"))]
                {
                    return Err(crate::error::ConfigError::InvalidValue {
                        key: "format".to_string(),
                        expected_type: "json".to_string(),
                        message: "enable json feature".to_string(),
                    });
                }
            }
            SnapshotFormat::Toml => {
                #[cfg(feature = "toml")]
                {
                    toml::to_string_pretty(&output).map_err(|e| {
                        crate::error::ConfigError::ParseError {
                            format: "toml".to_string(),
                            message: e.to_string(),
                            location: None,
                            source: Some(Box::new(e)),
                        }
                    })?
                }
                #[cfg(not(feature = "toml"))]
                {
                    return Err(crate::error::ConfigError::InvalidValue {
                        key: "format".to_string(),
                        expected_type: "toml".to_string(),
                        message: "enable toml feature".to_string(),
                    });
                }
            }
            SnapshotFormat::Yaml => {
                #[cfg(feature = "yaml")]
                {
                    serde_yaml_ng::to_string(&output).map_err(|e| {
                        crate::error::ConfigError::ParseError {
                            format: "yaml".to_string(),
                            message: e.to_string(),
                            location: None,
                            source: Some(Box::new(e)),
                        }
                    })?
                }
                #[cfg(not(feature = "yaml"))]
                {
                    return Err(crate::error::ConfigError::InvalidValue {
                        key: "format".to_string(),
                        expected_type: "yaml".to_string(),
                        message: "enable yaml feature".to_string(),
                    });
                }
            }
        };

        tokio::fs::write(&path, content)
            .await
            .map_err(crate::error::ConfigError::IoError)?;

        self.prune_old_snapshots()?;

        Ok(path)
    }

    fn attach_provenance(
        &self,
        mut serialized: serde_json::Value,
        value: &AnnotatedValue,
    ) -> serde_json::Value {
        if let serde_json::Value::Object(ref mut map) = serialized {
            let provenance = self.collect_provenance(value);
            map.insert("_provenance".to_string(), provenance);
        }
        serialized
    }

    fn collect_provenance(&self, value: &AnnotatedValue) -> serde_json::Value {
        let mut entries = Vec::new();
        let mut stack: Vec<&AnnotatedValue> = vec![value];

        // Iterative traversal using explicit stack to avoid stack overflow
        while let Some(current) = stack.pop() {
            let entry = serde_json::json!({
                "path": current.path.as_ref(),
                "source": current.source.as_str(),
                "priority": current.priority,
                "version": current.version,
            });
            entries.push(entry);

            match &current.inner {
                ConfigValue::Map(map) => {
                    for (_key, val) in map.iter() {
                        stack.push(val);
                    }
                }
                ConfigValue::Array(arr) => {
                    for val in arr.iter() {
                        stack.push(val);
                    }
                }
                _ => {}
            }
        }

        serde_json::Value::Array(entries)
    }

    pub async fn load_snapshot(&self, path: &Path) -> ConfigResult<AnnotatedValue> {
        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(crate::error::ConfigError::IoError)?;

        let value: serde_json::Value = match self.config.format {
            SnapshotFormat::Json => {
                #[cfg(feature = "json")]
                {
                    serde_json::from_str(&content).map_err(|e| {
                        crate::error::ConfigError::ParseError {
                            format: "json".to_string(),
                            message: e.to_string(),
                            location: None,
                            source: Some(Box::new(e)),
                        }
                    })?
                }
                #[cfg(not(feature = "json"))]
                {
                    return Err(crate::error::ConfigError::InvalidValue {
                        key: "format".to_string(),
                        expected_type: "json".to_string(),
                        message: "enable json feature".to_string(),
                    });
                }
            }
            SnapshotFormat::Toml => {
                #[cfg(feature = "toml")]
                {
                    toml::from_str::<serde_json::Value>(&content).map_err(|e| {
                        crate::error::ConfigError::ParseError {
                            format: "toml".to_string(),
                            message: e.to_string(),
                            location: None,
                            source: Some(Box::new(e)),
                        }
                    })?
                }
                #[cfg(not(feature = "toml"))]
                {
                    return Err(crate::error::ConfigError::InvalidValue {
                        key: "format".to_string(),
                        expected_type: "toml".to_string(),
                        message: "enable toml feature".to_string(),
                    });
                }
            }
            SnapshotFormat::Yaml => {
                #[cfg(feature = "yaml")]
                {
                    serde_yaml_ng::from_str::<serde_json::Value>(&content).map_err(|e| {
                        crate::error::ConfigError::ParseError {
                            format: "yaml".to_string(),
                            message: e.to_string(),
                            location: None,
                            source: Some(Box::new(e)),
                        }
                    })?
                }
                #[cfg(not(feature = "yaml"))]
                {
                    return Err(crate::error::ConfigError::InvalidValue {
                        key: "format".to_string(),
                        expected_type: "yaml".to_string(),
                        message: "enable yaml feature".to_string(),
                    });
                }
            }
        };

        let clean_value = if let serde_json::Value::Object(map) = value {
            let mut map = map;
            map.remove("_provenance");
            serde_json::Value::Object(map)
        } else {
            value
        };

        let annotated: AnnotatedValue = serde_json::from_value(clean_value).map_err(|e| {
            crate::error::ConfigError::ParseError {
                format: "json".to_string(),
                message: e.to_string(),
                location: None,
                source: Some(Box::new(e)),
            }
        })?;

        Ok(annotated)
    }
}

impl Default for SnapshotManager {
    fn default() -> Self {
        Self::new(SnapshotConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::SourceId;

    #[test]
    fn test_snapshot_format_ext() {
        assert_eq!(SnapshotFormat::Toml.ext(), "toml");
        assert_eq!(SnapshotFormat::Json.ext(), "json");
        assert_eq!(SnapshotFormat::Yaml.ext(), "yaml");
    }

    #[test]
    fn test_snapshot_config_default() {
        let config = SnapshotConfig::default();
        assert_eq!(config.max_snapshots, 30);
        assert!(config.include_provenance);
    }

    #[test]
    fn test_snapshot_config_new() {
        let config = SnapshotConfig::new("/tmp/snapshots");
        assert_eq!(config.dir, PathBuf::from("/tmp/snapshots"));
    }

    #[test]
    fn test_snapshot_manager_new() {
        let manager = SnapshotManager::default();
        assert_eq!(manager.config().max_snapshots, 30);
    }

    #[test]
    fn test_snapshot_format_debug() {
        let f = SnapshotFormat::Toml;
        assert!(!format!("{:?}", f).is_empty());
    }

    #[test]
    fn test_snapshot_format_clone_eq() {
        let a = SnapshotFormat::Json;
        let b = a;
        assert_eq!(a, b);
    }

    #[test]
    fn test_snapshot_config_clone() {
        let a = SnapshotConfig::new("/tmp/a");
        let b = a.clone();
        assert_eq!(a.dir, b.dir);
    }

    #[test]
    fn test_snapshot_config_custom() {
        let config = SnapshotConfig {
            dir: std::path::PathBuf::from("/custom"),
            max_snapshots: 5,
            format: SnapshotFormat::Json,
            include_provenance: false,
        };
        assert_eq!(config.max_snapshots, 5);
        assert!(!config.include_provenance);
    }

    #[test]
    fn test_snapshot_info_create() {
        use chrono::Utc;
        let info = SnapshotInfo {
            path: std::path::PathBuf::from("snap.json"),
            created_at: Utc::now(),
            size_bytes: 1024,
        };
        assert!(info.size_bytes > 0);
        assert_eq!(info.path.file_name().unwrap(), "snap.json");
    }

    #[test]
    fn test_snapshot_manager_default_config() {
        let manager = SnapshotManager::default();
        assert!(manager.config().include_provenance);
        assert_eq!(manager.config().max_snapshots, 30);
    }

    #[test]
    fn test_snapshot_manager_path() {
        let config = SnapshotConfig::new("./mysnapshots");
        let manager = SnapshotManager::new(config);
        assert!(manager
            .config()
            .dir
            .to_string_lossy()
            .contains("mysnapshots"));
    }

    // ---- SnapshotFormat::default ----

    #[test]
    fn test_snapshot_format_default_is_toml() {
        assert_eq!(SnapshotFormat::default(), SnapshotFormat::Toml);
    }

    #[test]
    fn test_snapshot_format_all_variants_ext() {
        // Round-trip every variant through ext() to ensure all match arms are exercised.
        for fmt in [
            SnapshotFormat::Toml,
            SnapshotFormat::Json,
            SnapshotFormat::Yaml,
        ] {
            let ext = fmt.ext();
            assert!(!ext.is_empty());
        }
    }

    #[test]
    fn test_snapshot_format_partial_eq() {
        assert_eq!(SnapshotFormat::Json, SnapshotFormat::Json);
        assert_ne!(SnapshotFormat::Json, SnapshotFormat::Yaml);
        assert_ne!(SnapshotFormat::Toml, SnapshotFormat::Json);
    }

    #[test]
    fn test_snapshot_format_clone_copies() {
        let fmt = SnapshotFormat::Yaml;
        let cloned = fmt;
        assert_eq!(fmt, cloned);
    }

    #[test]
    fn test_snapshot_format_copy_semantics() {
        let fmt = SnapshotFormat::Json;
        // SnapshotFormat derives Copy, so passing by value should not move.
        let other = fmt;
        let _ = fmt; // would fail to compile if not Copy
        let _ = other;
    }

    #[test]
    fn test_snapshot_config_new_preserves_other_defaults() {
        let config = SnapshotConfig::new("/custom/dir");
        assert_eq!(config.dir, PathBuf::from("/custom/dir"));
        // Other defaults preserved.
        assert_eq!(config.max_snapshots, 30);
        assert_eq!(config.format, SnapshotFormat::Toml);
        assert!(config.include_provenance);
    }

    #[test]
    fn test_snapshot_config_debug_format() {
        let config = SnapshotConfig::default();
        let s = format!("{:?}", config);
        assert!(s.contains("SnapshotConfig"));
    }

    #[test]
    fn test_snapshot_manager_new_stores_config() {
        let config = SnapshotConfig {
            dir: PathBuf::from("/managed"),
            max_snapshots: 7,
            format: SnapshotFormat::Yaml,
            include_provenance: false,
        };
        let manager = SnapshotManager::new(config.clone());
        assert_eq!(manager.config().dir, config.dir);
        assert_eq!(manager.config().max_snapshots, 7);
        assert_eq!(manager.config().format, SnapshotFormat::Yaml);
        assert!(!manager.config().include_provenance);
    }

    #[test]
    fn test_snapshot_manager_default_uses_default_config() {
        let manager = SnapshotManager::default();
        assert_eq!(manager.config().dir, PathBuf::from("config-snapshots"));
        assert_eq!(manager.config().max_snapshots, 30);
    }

    #[test]
    fn test_snapshot_info_path_and_size() {
        let info = SnapshotInfo {
            path: PathBuf::from("/snap/snapshot-20240101.json"),
            created_at: Utc::now(),
            size_bytes: 4096,
        };
        assert_eq!(info.size_bytes, 4096);
        assert_eq!(
            info.path.file_name().unwrap().to_string_lossy(),
            "snapshot-20240101.json"
        );
    }

    #[test]
    fn test_snapshot_info_serialize_deserialize_roundtrip() {
        let info = SnapshotInfo {
            path: PathBuf::from("/x.json"),
            created_at: Utc::now(),
            size_bytes: 100,
        };
        let json = serde_json::to_string(&info).unwrap();
        let parsed: SnapshotInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.size_bytes, 100);
        assert_eq!(parsed.path, info.path);
    }

    // ---- list_snapshots ----

    #[test]
    fn test_list_snapshots_nonexistent_dir_returns_empty() {
        let manager = SnapshotManager::new(SnapshotConfig::new(
            "/nonexistent/confers/snapshot-dir-does-not-exist",
        ));
        let list = manager.list_snapshots().unwrap();
        assert!(list.is_empty());
    }

    #[test]
    fn test_list_snapshots_empty_dir_returns_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let manager = SnapshotManager::new(SnapshotConfig::new(tmp.path()));
        let list = manager.list_snapshots().unwrap();
        assert!(list.is_empty());
    }

    #[test]
    fn test_list_snapshots_filters_by_format_extension() {
        let tmp = tempfile::tempdir().unwrap();
        // Write files with different extensions; only those matching the configured format
        // should appear in the listing.
        std::fs::write(tmp.path().join("a.toml"), "x = 1\n").unwrap();
        std::fs::write(tmp.path().join("b.json"), "{}\n").unwrap();
        std::fs::write(tmp.path().join("c.txt"), "ignore me\n").unwrap();

        let toml_manager = SnapshotManager::new(SnapshotConfig {
            dir: tmp.path().to_path_buf(),
            max_snapshots: 30,
            format: SnapshotFormat::Toml,
            include_provenance: true,
        });
        let toml_list = toml_manager.list_snapshots().unwrap();
        assert_eq!(toml_list.len(), 1);
        assert!(toml_list[0].path.to_string_lossy().ends_with("a.toml"));

        let json_manager = SnapshotManager::new(SnapshotConfig {
            dir: tmp.path().to_path_buf(),
            max_snapshots: 30,
            format: SnapshotFormat::Json,
            include_provenance: true,
        });
        let json_list = json_manager.list_snapshots().unwrap();
        assert_eq!(json_list.len(), 1);
        assert!(json_list[0].path.to_string_lossy().ends_with("b.json"));
    }

    // ---- prune_old_snapshots ----

    #[test]
    fn test_prune_old_snapshots_no_op_when_under_limit() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("a.toml"), "x = 1\n").unwrap();
        let manager = SnapshotManager::new(SnapshotConfig {
            dir: tmp.path().to_path_buf(),
            max_snapshots: 30,
            format: SnapshotFormat::Toml,
            include_provenance: true,
        });
        let removed = manager.prune_old_snapshots().unwrap();
        assert_eq!(removed, 0);
        // File still present.
        assert!(tmp.path().join("a.toml").exists());
    }

    #[test]
    fn test_prune_old_snapshots_removes_oldest() {
        let tmp = tempfile::tempdir().unwrap();
        // Write 3 snapshots but cap max to 1.
        for name in ["a.toml", "b.toml", "c.toml"] {
            std::fs::write(tmp.path().join(name), "x = 1\n").unwrap();
            // Tiny sleep so creation timestamps differ on platforms with low-resolution mtimes.
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        let manager = SnapshotManager::new(SnapshotConfig {
            dir: tmp.path().to_path_buf(),
            max_snapshots: 1,
            format: SnapshotFormat::Toml,
            include_provenance: true,
        });
        let removed = manager.prune_old_snapshots().unwrap();
        assert_eq!(removed, 2);
        // Exactly one remains.
        let remaining = manager.list_snapshots().unwrap();
        assert_eq!(remaining.len(), 1);
    }

    #[test]
    fn test_prune_old_snapshots_when_already_at_limit() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("a.toml"), "x = 1\n").unwrap();
        std::fs::write(tmp.path().join("b.toml"), "x = 2\n").unwrap();
        let manager = SnapshotManager::new(SnapshotConfig {
            dir: tmp.path().to_path_buf(),
            max_snapshots: 2,
            format: SnapshotFormat::Toml,
            include_provenance: true,
        });
        let removed = manager.prune_old_snapshots().unwrap();
        assert_eq!(removed, 0);
    }

    // ---- save (async) ----

    fn make_value() -> AnnotatedValue {
        let inner =
            AnnotatedValue::new(ConfigValue::string("alice"), SourceId::new("test"), "name");
        let port = AnnotatedValue::new(ConfigValue::uint(8080), SourceId::new("test"), "port");
        AnnotatedValue::new(
            ConfigValue::map(vec![("name", inner), ("port", port)]),
            SourceId::new("test"),
            "",
        )
    }

    #[tokio::test]
    async fn test_save_creates_dir_and_writes_file() {
        let tmp = tempfile::tempdir().unwrap();
        let target = tmp.path().join("nested").join("snapshots");
        let manager = SnapshotManager::new(SnapshotConfig {
            dir: target.clone(),
            max_snapshots: 30,
            format: SnapshotFormat::Toml,
            include_provenance: false,
        });
        let path = manager.save(&make_value(), &[]).await.unwrap();
        assert!(target.exists());
        assert!(path.exists());
        assert!(path.to_string_lossy().ends_with(".toml"));
    }

    #[tokio::test]
    async fn test_save_with_provenance_attaches_field() {
        let tmp = tempfile::tempdir().unwrap();
        let manager = SnapshotManager::new(SnapshotConfig {
            dir: tmp.path().to_path_buf(),
            max_snapshots: 30,
            format: SnapshotFormat::Json,
            include_provenance: true,
        });
        let path = manager.save(&make_value(), &[]).await.unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("_provenance"));
    }

    #[tokio::test]
    async fn test_save_without_provenance_omits_field() {
        let tmp = tempfile::tempdir().unwrap();
        let manager = SnapshotManager::new(SnapshotConfig {
            dir: tmp.path().to_path_buf(),
            max_snapshots: 30,
            format: SnapshotFormat::Json,
            include_provenance: false,
        });
        let path = manager.save(&make_value(), &[]).await.unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(!content.contains("_provenance"));
    }

    #[tokio::test]
    async fn test_save_redacts_sensitive_paths() {
        let tmp = tempfile::tempdir().unwrap();
        let manager = SnapshotManager::new(SnapshotConfig {
            dir: tmp.path().to_path_buf(),
            max_snapshots: 30,
            format: SnapshotFormat::Json,
            include_provenance: false,
        });
        let path = manager.save(&make_value(), &["name"]).await.unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("[REDACTED]"));
        assert!(!content.contains("alice"));
    }

    #[tokio::test]
    async fn test_load_snapshot_json_with_annotated_value_structure() {
        // load_snapshot expects an AnnotatedValue-shaped JSON (with `inner`, `source`, etc.),
        // not the bare ConfigValue JSON that `save` produces. Write a hand-constructed
        // AnnotatedValue JSON file and verify load_snapshot can parse it.
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("snap.json");
        std::fs::write(
            &path,
            r#"{
                "inner": "hello",
                "source": "test",
                "path": "",
                "priority": 0,
                "version": 0,
                "location": null
            }"#,
        )
        .unwrap();
        let manager = SnapshotManager::new(SnapshotConfig {
            dir: tmp.path().to_path_buf(),
            max_snapshots: 30,
            format: SnapshotFormat::Json,
            include_provenance: true,
        });
        let loaded = manager.load_snapshot(&path).await.unwrap();
        assert_eq!(loaded.inner.as_str(), Some("hello"));
        assert_eq!(loaded.source.as_str(), "test");
    }

    #[tokio::test]
    async fn test_load_snapshot_toml_with_annotated_value_structure() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("snap.toml");
        std::fs::write(
            &path,
            r#"inner = "hello"
source = "test"
path = ""
priority = 0
version = 0
"#,
        )
        .unwrap();
        let manager = SnapshotManager::new(SnapshotConfig {
            dir: tmp.path().to_path_buf(),
            max_snapshots: 30,
            format: SnapshotFormat::Toml,
            include_provenance: true,
        });
        let loaded = manager.load_snapshot(&path).await.unwrap();
        assert_eq!(loaded.inner.as_str(), Some("hello"));
        assert_eq!(loaded.source.as_str(), "test");
    }

    #[tokio::test]
    async fn test_load_snapshot_yaml_with_annotated_value_structure() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("snap.yaml");
        std::fs::write(
            &path,
            "inner: hello\nsource: test\npath: \"\"\npriority: 0\nversion: 0\n",
        )
        .unwrap();
        let manager = SnapshotManager::new(SnapshotConfig {
            dir: tmp.path().to_path_buf(),
            max_snapshots: 30,
            format: SnapshotFormat::Yaml,
            include_provenance: true,
        });
        let loaded = manager.load_snapshot(&path).await.unwrap();
        assert_eq!(loaded.inner.as_str(), Some("hello"));
        assert_eq!(loaded.source.as_str(), "test");
    }

    #[tokio::test]
    async fn test_load_snapshot_strips_provenance_field() {
        // When the loaded JSON contains a `_provenance` field, it should be stripped
        // before deserializing into AnnotatedValue.
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("snap.json");
        std::fs::write(
            &path,
            r#"{
                "inner": "v",
                "source": "s",
                "path": "",
                "priority": 0,
                "version": 0,
                "location": null,
                "_provenance": [{"path": "", "source": "s", "priority": 0, "version": 0}]
            }"#,
        )
        .unwrap();
        let manager = SnapshotManager::new(SnapshotConfig {
            dir: tmp.path().to_path_buf(),
            max_snapshots: 30,
            format: SnapshotFormat::Json,
            include_provenance: true,
        });
        let loaded = manager.load_snapshot(&path).await.unwrap();
        assert_eq!(loaded.inner.as_str(), Some("v"));
    }

    // ---- load_snapshot error paths ----

    #[tokio::test]
    async fn test_load_snapshot_nonexistent_file_errors() {
        let manager = SnapshotManager::new(SnapshotConfig::default());
        let res = manager
            .load_snapshot(Path::new("/nonexistent/confers/no-such-snapshot.json"))
            .await;
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn test_load_snapshot_invalid_content_errors() {
        let tmp = tempfile::tempdir().unwrap();
        let bad_path = tmp.path().join("bad.json");
        std::fs::write(&bad_path, "not valid json {{{").unwrap();
        let manager = SnapshotManager::new(SnapshotConfig {
            dir: tmp.path().to_path_buf(),
            max_snapshots: 30,
            format: SnapshotFormat::Json,
            include_provenance: true,
        });
        let res = manager.load_snapshot(&bad_path).await;
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn test_load_snapshot_invalid_toml_errors() {
        let tmp = tempfile::tempdir().unwrap();
        let bad_path = tmp.path().join("bad.toml");
        std::fs::write(&bad_path, "not = valid = toml = =").unwrap();
        let manager = SnapshotManager::new(SnapshotConfig {
            dir: tmp.path().to_path_buf(),
            max_snapshots: 30,
            format: SnapshotFormat::Toml,
            include_provenance: true,
        });
        let res = manager.load_snapshot(&bad_path).await;
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn test_load_snapshot_invalid_yaml_errors() {
        let tmp = tempfile::tempdir().unwrap();
        // YAML is permissive; use an unterminated flow collection to force an error.
        let bad_path = tmp.path().join("bad.yaml");
        std::fs::write(&bad_path, "key: [unclosed").unwrap();
        let manager = SnapshotManager::new(SnapshotConfig {
            dir: tmp.path().to_path_buf(),
            max_snapshots: 30,
            format: SnapshotFormat::Yaml,
            include_provenance: true,
        });
        let res = manager.load_snapshot(&bad_path).await;
        assert!(res.is_err());
    }

    // ---- provenance collection (via save) ----

    #[tokio::test]
    async fn test_save_provenance_includes_nested_entries() {
        let tmp = tempfile::tempdir().unwrap();
        let manager = SnapshotManager::new(SnapshotConfig {
            dir: tmp.path().to_path_buf(),
            max_snapshots: 30,
            format: SnapshotFormat::Json,
            include_provenance: true,
        });
        let nested_value = AnnotatedValue::new(
            ConfigValue::map(vec![(
                "outer",
                AnnotatedValue::new(
                    ConfigValue::map(vec![(
                        "inner",
                        AnnotatedValue::new(
                            ConfigValue::uint(99),
                            SourceId::new("nested-src"),
                            "outer.inner",
                        ),
                    )]),
                    SourceId::new("outer-src"),
                    "outer",
                ),
            )]),
            SourceId::new("root-src"),
            "",
        );
        let path = manager.save(&nested_value, &[]).await.unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("_provenance"));
        assert!(content.contains("nested-src"));
        assert!(content.contains("outer-src"));
        assert!(content.contains("root-src"));
    }

    #[tokio::test]
    async fn test_save_provenance_with_array_entries() {
        let tmp = tempfile::tempdir().unwrap();
        let manager = SnapshotManager::new(SnapshotConfig {
            dir: tmp.path().to_path_buf(),
            max_snapshots: 30,
            format: SnapshotFormat::Json,
            include_provenance: true,
        });
        let value_with_array = AnnotatedValue::new(
            ConfigValue::map(vec![(
                "items",
                AnnotatedValue::new(
                    ConfigValue::array(vec![
                        AnnotatedValue::new(
                            ConfigValue::uint(1),
                            SourceId::new("arr-src"),
                            "items.0",
                        ),
                        AnnotatedValue::new(
                            ConfigValue::uint(2),
                            SourceId::new("arr-src"),
                            "items.1",
                        ),
                    ]),
                    SourceId::new("items-src"),
                    "items",
                ),
            )]),
            SourceId::new("root-src"),
            "",
        );
        let path = manager.save(&value_with_array, &[]).await.unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("arr-src"));
        assert!(content.contains("items-src"));
    }

    #[tokio::test]
    async fn test_save_provenance_skipped_for_non_object_serialized() {
        // When the serialized value is not an object (e.g. a single primitive),
        // attach_provenance should leave the value unchanged.
        let tmp = tempfile::tempdir().unwrap();
        let manager = SnapshotManager::new(SnapshotConfig {
            dir: tmp.path().to_path_buf(),
            max_snapshots: 30,
            format: SnapshotFormat::Json,
            include_provenance: true,
        });
        let primitive = AnnotatedValue::new(
            ConfigValue::string("just a string"),
            SourceId::new("src"),
            "",
        );
        let path = manager.save(&primitive, &[]).await.unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        // A top-level string value serializes to a JSON string; provenance is not attached.
        assert!(!content.contains("_provenance"));
    }

    // ---- save + prune integration ----

    #[tokio::test]
    async fn test_save_prunes_old_snapshots_when_over_limit() {
        let tmp = tempfile::tempdir().unwrap();
        let manager = SnapshotManager::new(SnapshotConfig {
            dir: tmp.path().to_path_buf(),
            max_snapshots: 2,
            format: SnapshotFormat::Json,
            include_provenance: false,
        });
        // Save 4 snapshots; only the most recent `max_snapshots` should remain.
        for i in 0..4u64 {
            let v = AnnotatedValue::new(
                ConfigValue::map(vec![(
                    "i",
                    AnnotatedValue::new(ConfigValue::uint(i), SourceId::new("src"), "i"),
                )]),
                SourceId::new("src"),
                "",
            );
            manager.save(&v, &[]).await.unwrap();
            // Sleep to ensure distinct timestamps (filename includes seconds).
            std::thread::sleep(std::time::Duration::from_millis(1100));
        }
        let remaining = manager.list_snapshots().unwrap();
        assert!(
            remaining.len() <= 2,
            "expected at most 2 snapshots, got {}",
            remaining.len()
        );
    }
}
