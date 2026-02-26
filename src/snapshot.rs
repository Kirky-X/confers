//! Configuration snapshot module.

use crate::error::ConfigResult;
use crate::value::{AnnotatedValue, ConfigValue, SerializeMode};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[allow(unused_imports)]
use std::sync::Arc;

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

pub type SnapshotOptions = SnapshotConfig;

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
                    .map(|t| DateTime::from(t))
                    .unwrap_or_else(|_| Utc::now());
                snapshots.push(SnapshotInfo {
                    path,
                    created_at,
                    size_bytes: metadata.len(),
                });
            }
        }

        snapshots.sort_by(|a, b| b.created_at.cmp(&a.created_at));
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
            .map_err(|e| crate::error::ConfigError::IoError(e))?;

        let timestamp = Utc::now().format("%Y%m%dT%H%M%SZ");
        let filename = format!("config-{}.{}", timestamp, self.config.format.ext());
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
                    serde_json::to_string_pretty(&output)
                        .map_err(|e| crate::error::ConfigError::ParseError {
                            format: "json".to_string(),
                            message: e.to_string(),
                            location: None,
                            source: Some(Box::new(e)),
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
                    toml::to_string_pretty(&output)
                        .map_err(|e| crate::error::ConfigError::ParseError {
                            format: "toml".to_string(),
                            message: e.to_string(),
                            location: None,
                            source: Some(Box::new(e)),
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
                    serde_yaml_ng::to_string(&output)
                        .map_err(|e| crate::error::ConfigError::ParseError {
                            format: "yaml".to_string(),
                            message: e.to_string(),
                            location: None,
                            source: Some(Box::new(e)),
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
            .map_err(|e| crate::error::ConfigError::IoError(e))?;

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
            .map_err(|e| crate::error::ConfigError::IoError(e))?;

        let value: serde_json::Value = match self.config.format {
            SnapshotFormat::Json => {
                #[cfg(feature = "json")]
                {
                    serde_json::from_str(&content)
                        .map_err(|e| crate::error::ConfigError::ParseError {
                            format: "json".to_string(),
                            message: e.to_string(),
                            location: None,
                            source: Some(Box::new(e)),
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
                    serde_json::to_value(
                        toml::from_str::<serde_json::Value>(&content)
                            .map_err(|e| crate::error::ConfigError::ParseError {
                                format: "toml".to_string(),
                                message: e.to_string(),
                                location: None,
                                source: Some(Box::new(e)),
                            })?
                    ).map_err(|e| crate::error::ConfigError::ParseError {
                        format: "toml".to_string(),
                        message: e.to_string(),
                        location: None,
                        source: Some(Box::new(e)),
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
                    serde_json::to_value(
                        serde_yaml_ng::from_str::<serde_json::Value>(&content)
                            .map_err(|e| crate::error::ConfigError::ParseError {
                                format: "yaml".to_string(),
                                message: e.to_string(),
                                location: None,
                                source: Some(Box::new(e)),
                            })?
                    ).map_err(|e| crate::error::ConfigError::ParseError {
                        format: "yaml".to_string(),
                        message: e.to_string(),
                        location: None,
                        source: Some(Box::new(e)),
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

        let annotated: AnnotatedValue = serde_json::from_value(clean_value)
            .map_err(|e| crate::error::ConfigError::ParseError {
                format: "json".to_string(),
                message: e.to_string(),
                location: None,
                source: Some(Box::new(e)),
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
}
