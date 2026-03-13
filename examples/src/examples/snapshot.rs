//! 配置快照持久化示例
//!
//! 本示例展示如何使用 confers 的配置快照功能：
//! - 配置快照自动保存
//! - 快照时间戳命名
//! - 敏感字段脱敏
//! - 快照历史管理
//! - 快照对比和回溯
//!
//! 设计依据：ADR-033（配置快照）
//! 对标框架：Hydra 工作目录隔离 / Azure App Configuration 快照

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

// =============================================================================
// 配置结构定义
// =============================================================================

/// 应用配置结构
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct AppConfig {
    /// 应用名称
    #[serde(default = "default_name")]
    pub name: String,

    /// 应用版本
    #[serde(default = "default_version")]
    pub version: String,

    /// 环境
    #[serde(default = "default_environment")]
    pub environment: String,

    /// 服务器配置
    #[serde(default)]
    pub server: ServerConfig,

    /// 数据库配置
    #[serde(default)]
    pub database: DatabaseConfig,

    /// 功能开关
    #[serde(default)]
    pub features: FeatureFlags,
}

fn default_name() -> String {
    "myapp".to_string()
}

fn default_version() -> String {
    "1.0.0".to_string()
}

fn default_environment() -> String {
    "development".to_string()
}

/// 服务器配置
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct ServerConfig {
    #[serde(default = "default_host")]
    pub host: String,

    #[serde(default = "default_port")]
    pub port: u16,

    #[serde(default = "default_workers")]
    pub workers: usize,
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}

fn default_port() -> u16 {
    8080
}

fn default_workers() -> usize {
    4
}

/// 数据库配置
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct DatabaseConfig {
    #[serde(default)]
    pub url: String,

    #[serde(default = "default_max_conn")]
    pub max_connections: u32,

    #[serde(default)]
    pub timeout_seconds: u32,
}

fn default_max_conn() -> u32 {
    10
}

/// 功能开关配置
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct FeatureFlags {
    #[serde(default)]
    pub new_ui: bool,

    #[serde(default)]
    pub beta_api: bool,

    #[serde(default)]
    pub realtime_notification: bool,
}

// =============================================================================
// 快照配置
// =============================================================================

/// 快照格式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotFormat {
    Toml,
    Json,
    Yaml,
}

impl SnapshotFormat {
    /// 获取文件扩展名
    pub fn ext(&self) -> &'static str {
        match self {
            SnapshotFormat::Toml => "toml",
            SnapshotFormat::Json => "json",
            SnapshotFormat::Yaml => "yaml",
        }
    }

    /// 从扩展名解析
    pub fn from_ext(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "toml" => Some(SnapshotFormat::Toml),
            "json" => Some(SnapshotFormat::Json),
            "yaml" | "yml" => Some(SnapshotFormat::Yaml),
            _ => None,
        }
    }
}

/// 快照配置
#[derive(Debug, Clone)]
pub struct SnapshotConfig {
    /// 快照目录
    pub dir: PathBuf,

    /// 最大保留快照数，超出时删除最旧的
    pub max_snapshots: usize,

    /// 快照格式
    pub format: SnapshotFormat,

    /// 是否在快照中记录每个值的来源（source + location）
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
    /// 创建新的快照配置
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self {
            dir: dir.into(),
            ..Default::default()
        }
    }

    /// 设置最大快照数
    pub fn with_max_snapshots(mut self, max: usize) -> Self {
        self.max_snapshots = max;
        self
    }

    /// 设置快照格式
    pub fn with_format(mut self, format: SnapshotFormat) -> Self {
        self.format = format;
        self
    }

    /// 设置是否包含来源信息
    pub fn with_provenance(mut self, include: bool) -> Self {
        self.include_provenance = include;
        self
    }
}

// =============================================================================
// 快照条目
// =============================================================================

/// 快照条目信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotEntry {
    /// 快照文件名
    pub filename: String,

    /// 快照时间戳
    pub timestamp: DateTime<Utc>,

    /// 快照大小（字节）
    pub size: u64,

    /// 配置字段数量
    pub field_count: usize,

    /// 配置校验和（SHA-256）
    pub checksum: String,

    /// 配置版本
    pub version: u64,
}

impl SnapshotEntry {
    /// 从文件路径创建快照条目
    pub fn from_path(path: &Path) -> std::io::Result<Self> {
        let metadata = std::fs::metadata(path)?;
        let filename = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let timestamp = parse_timestamp_from_filename(&filename).unwrap_or_else(Utc::now);

        Ok(Self {
            filename,
            timestamp,
            size: metadata.len(),
            field_count: 0,
            checksum: String::new(),
            version: 0,
        })
    }
}

/// 从文件名解析时间戳
///
/// 文件名格式: config-20260225T143000Z.toml
fn parse_timestamp_from_filename(filename: &str) -> Option<DateTime<Utc>> {
    let re = regex::Regex::new(r"config-(\d{8}T\d{6}Z)").ok()?;
    let caps = re.captures(filename)?;
    let ts_str = caps.get(1)?.as_str();
    DateTime::parse_from_str(&format!("{}+00:00", ts_str), "%Y%m%dT%H%M%SZ%z")
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

// =============================================================================
// 快照管理器
// =============================================================================

/// 快照管理器
///
/// 负责配置快照的保存、加载和管理
pub struct SnapshotManager {
    /// 快照配置
    config: SnapshotConfig,

    /// 敏感字段路径集合
    sensitive_paths: Arc<RwLock<Vec<String>>>,
}

impl SnapshotManager {
    /// 创建新的快照管理器
    pub fn new(config: SnapshotConfig) -> Self {
        Self {
            config,
            sensitive_paths: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// 添加敏感字段路径
    pub async fn add_sensitive_path(&self, path: String) {
        self.sensitive_paths.write().await.push(path);
    }

    /// 保存快照（脱敏，sensitive 字段替换为 [REDACTED]）
    pub async fn save<T: Serialize>(&self, config: &T) -> std::io::Result<PathBuf> {
        tokio::fs::create_dir_all(&self.config.dir).await?;

        let timestamp = Utc::now().format("%Y%m%dT%H%M%SZ");
        let filename = format!("config-{}.{}", timestamp, self.config.format.ext());
        let path = self.config.dir.join(&filename);

        let content = self.serialize_with_redaction(config).await?;

        tokio::fs::write(&path, content).await?;

        info!("[SnapshotManager] 快照已保存: {:?}", path);

        self.prune_old_snapshots().await?;

        Ok(path)
    }

    /// 序列化配置并脱敏敏感字段
    async fn serialize_with_redaction<T: Serialize>(&self, config: &T) -> std::io::Result<String> {
        let sensitive = self.sensitive_paths.read().await;

        match self.config.format {
            SnapshotFormat::Toml => {
                let mut value = serde_json::to_value(config).map_err(|e| {
                    std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())
                })?;
                self.redact_sensitive_fields(&mut value, &sensitive).await;
                toml::to_string_pretty(&value).map_err(|e| {
                    std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())
                })
            }
            SnapshotFormat::Json => {
                let mut value = serde_json::to_value(config).map_err(|e| {
                    std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())
                })?;
                self.redact_sensitive_fields(&mut value, &sensitive).await;
                serde_json::to_string_pretty(&value).map_err(|e| {
                    std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())
                })
            }
            SnapshotFormat::Yaml => {
                let mut value = serde_json::to_value(config).map_err(|e| {
                    std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())
                })?;
                self.redact_sensitive_fields(&mut value, &sensitive).await;
                serde_yaml_ng::to_string(&value).map_err(|e| {
                    std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())
                })
            }
        }
    }

    /// 脱敏敏感字段
    async fn redact_sensitive_fields(&self, value: &mut serde_json::Value, sensitive_paths: &[String]) {
        for path in sensitive_paths {
            if let Some(field) = Self::get_nested_value_mut(value, path) {
                *field = serde_json::Value::String("[REDACTED]".to_string());
            }
        }
    }

    /// 获取嵌套值的可变引用
    fn get_nested_value_mut<'a>(value: &'a mut serde_json::Value, path: &str) -> Option<&'a mut serde_json::Value> {
        let parts: Vec<&str> = path.split('.').collect();
        let mut current = value;

        for part in parts {
            if let serde_json::Value::Object(map) = current {
                current = map.get_mut(part)?;
            } else {
                return None;
            }
        }

        Some(current)
    }

    /// 列出所有快照（最新在前）
    pub async fn list(&self) -> std::io::Result<Vec<SnapshotEntry>> {
        let mut entries = Vec::new();
        let mut dir = tokio::fs::read_dir(&self.config.dir).await?;

        while let Some(entry) = dir.next_entry().await? {
            let path = entry.path();
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if SnapshotFormat::from_ext(ext).is_some() {
                    if let Ok(snapshot_entry) = SnapshotEntry::from_path(&path) {
                        entries.push(snapshot_entry);
                    }
                }
            }
        }

        entries.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        Ok(entries)
    }

    /// 加载历史快照（用于回溯和对比）
    pub async fn load<T: for<'de> Deserialize<'de>>(&self, filename: &str) -> std::io::Result<T> {
        let path = self.config.dir.join(filename);

        let content = tokio::fs::read_to_string(&path).await?;

        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("toml");

        match SnapshotFormat::from_ext(ext) {
            Some(SnapshotFormat::Toml) => {
                toml::from_str(&content).map_err(|e| {
                    std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())
                })
            }
            Some(SnapshotFormat::Json) => {
                serde_json::from_str(&content).map_err(|e| {
                    std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())
                })
            }
            Some(SnapshotFormat::Yaml) => {
                serde_yaml_ng::from_str(&content).map_err(|e| {
                    std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())
                })
            }
            None => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Unsupported snapshot format: {}", ext),
            )),
        }
    }

    /// 清理超出限制的旧快照
    async fn prune_old_snapshots(&self) -> std::io::Result<()> {
        let entries = self.list().await?;

        if entries.len() > self.config.max_snapshots {
            let to_delete = &entries[self.config.max_snapshots..];

            for entry in to_delete {
                let path = self.config.dir.join(&entry.filename);
                tokio::fs::remove_file(&path).await?;
                info!("[SnapshotManager] 已删除旧快照: {}", entry.filename);
            }
        }

        Ok(())
    }

    /// 对比两个快照的差异
    pub async fn diff<T: Serialize + for<'de> Deserialize<'de> + std::fmt::Debug>(
        &self,
        filename1: &str,
        filename2: &str,
    ) -> std::io::Result<ConfigDiff> {
        let config1: T = self.load(filename1).await?;
        let config2: T = self.load(filename2).await?;

        let json1 = serde_json::to_value(&config1).map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())
        })?;

        let json2 = serde_json::to_value(&config2).map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())
        })?;

        Ok(ConfigDiff::compare(&json1, &json2))
    }

    /// 删除指定快照
    pub async fn delete(&self, filename: &str) -> std::io::Result<()> {
        let path = self.config.dir.join(filename);
        tokio::fs::remove_file(&path).await?;
        info!("[SnapshotManager] 已删除快照: {}", filename);
        Ok(())
    }

    /// 清理指定天数之前的快照
    pub async fn prune_older_than(&self, days: u32) -> std::io::Result<usize> {
        let entries = self.list().await?;
        let cutoff = Utc::now() - chrono::Duration::days(days as i64);
        let mut deleted_count = 0;

        for entry in entries {
            if entry.timestamp < cutoff {
                self.delete(&entry.filename).await?;
                deleted_count += 1;
            }
        }

        Ok(deleted_count)
    }
}

// =============================================================================
// 配置差异
// =============================================================================

/// 配置差异
#[derive(Debug, Clone)]
pub struct ConfigDiff {
    /// 新增的字段
    pub added: Vec<String>,

    /// 删除的字段
    pub removed: Vec<String>,

    /// 修改的字段
    pub changed: Vec<ChangedField>,
}

/// 修改的字段
#[derive(Debug, Clone)]
pub struct ChangedField {
    /// 字段路径
    pub path: String,

    /// 旧值
    pub old_value: serde_json::Value,

    /// 新值
    pub new_value: serde_json::Value,
}

impl ConfigDiff {
    /// 比较两个配置值
    pub fn compare(old: &serde_json::Value, new: &serde_json::Value) -> Self {
        let mut diff = Self {
            added: Vec::new(),
            removed: Vec::new(),
            changed: Vec::new(),
        };

        diff.compare_values("", old, new);
        diff
    }

    fn compare_values(&mut self, path: &str, old: &serde_json::Value, new: &serde_json::Value) {
        match (old, new) {
            (serde_json::Value::Object(old_map), serde_json::Value::Object(new_map)) => {
                for (key, new_val) in new_map {
                    let full_path = if path.is_empty() {
                        key.clone()
                    } else {
                        format!("{}.{}", path, key)
                    };

                    if let Some(old_val) = old_map.get(key) {
                        self.compare_values(&full_path, old_val, new_val);
                    } else {
                        self.added.push(full_path);
                    }
                }

                for key in old_map.keys() {
                    if !new_map.contains_key(key) {
                        let full_path = if path.is_empty() {
                            key.clone()
                        } else {
                            format!("{}.{}", path, key)
                        };
                        self.removed.push(full_path);
                    }
                }
            }
            _ => {
                if old != new {
                    self.changed.push(ChangedField {
                        path: path.to_string(),
                        old_value: old.clone(),
                        new_value: new.clone(),
                    });
                }
            }
        }
    }

    /// 是否有差异
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.removed.is_empty() && self.changed.is_empty()
    }

    /// 打印差异
    pub fn print(&self) {
        if self.is_empty() {
            println!("配置无差异");
            return;
        }

        if !self.added.is_empty() {
            println!("新增字段:");
            for path in &self.added {
                println!("  + {}", path);
            }
        }

        if !self.removed.is_empty() {
            println!("删除字段:");
            for path in &self.removed {
                println!("  - {}", path);
            }
        }

        if !self.changed.is_empty() {
            println!("修改字段:");
            for field in &self.changed {
                println!("  ~ {}: {:?} -> {:?}", field.path, field.old_value, field.new_value);
            }
        }
    }
}

// =============================================================================
// 主程序
// =============================================================================

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("设置日志失败");

    info!("========================================");
    info!("  配置快照持久化示例");
    info!("========================================");

    demo_basic_snapshot().await?;
    demo_list_and_load().await?;
    demo_snapshot_diff().await?;
    demo_sensitive_redaction().await?;
    demo_snapshot_cleanup().await?;

    info!("");
    info!("========================================");
    info!("  示例运行完成");
    info!("========================================");

    Ok(())
}

/// 演示基础快照保存
async fn demo_basic_snapshot() -> Result<(), Box<dyn std::error::Error>> {
    info!("\n=== 演示 1: 基础快照保存 ===\n");

    let config = AppConfig {
        name: "myapp".to_string(),
        version: "1.0.0".to_string(),
        environment: "production".to_string(),
        server: ServerConfig {
            host: "0.0.0.0".to_string(),
            port: 8080,
            workers: 8,
        },
        database: DatabaseConfig {
            url: "postgresql://localhost/mydb".to_string(),
            max_connections: 20,
            timeout_seconds: 30,
        },
        features: FeatureFlags {
            new_ui: true,
            beta_api: false,
            realtime_notification: true,
        },
    };

    let snapshot_config = SnapshotConfig::new("config-snapshots")
        .with_max_snapshots(10)
        .with_format(SnapshotFormat::Toml);

    let manager = SnapshotManager::new(snapshot_config);

    let path = manager.save(&config).await?;
    info!("✓ 快照已保存: {:?}", path);

    let content = tokio::fs::read_to_string(&path).await?;
    info!("快照内容预览:\n{}", content.lines().take(10).collect::<Vec<_>>().join("\n"));

    Ok(())
}

/// 演示快照列表和加载
async fn demo_list_and_load() -> Result<(), Box<dyn std::error::Error>> {
    info!("\n=== 演示 2: 快照列表和加载 ===\n");

    let manager = SnapshotManager::new(SnapshotConfig::new("config-snapshots"));

    let entries = manager.list().await?;
    info!("快照列表 (共 {} 个):", entries.len());

    for entry in entries.iter().take(5) {
        info!(
            "  {} - {} bytes, {}",
            entry.filename,
            entry.size,
            entry.timestamp.format("%Y-%m-%d %H:%M:%S")
        );
    }

    if let Some(latest) = entries.first() {
        let config: AppConfig = manager.load(&latest.filename).await?;
        info!("\n最新快照内容:");
        info!("  应用: {} v{}", config.name, config.version);
        info!("  环境: {}", config.environment);
        info!("  服务器: {}:{}", config.server.host, config.server.port);
    }

    Ok(())
}

/// 演示快照对比
async fn demo_snapshot_diff() -> Result<(), Box<dyn std::error::Error>> {
    info!("\n=== 演示 3: 快照对比 ===\n");

    let manager = SnapshotManager::new(SnapshotConfig::new("config-snapshots"));

    let config1 = AppConfig {
        name: "myapp".to_string(),
        version: "1.0.0".to_string(),
        environment: "development".to_string(),
        server: ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 3000,
            workers: 4,
        },
        database: DatabaseConfig::default(),
        features: FeatureFlags::default(),
    };

    let config2 = AppConfig {
        name: "myapp".to_string(),
        version: "1.1.0".to_string(),
        environment: "production".to_string(),
        server: ServerConfig {
            host: "0.0.0.0".to_string(),
            port: 8080,
            workers: 8,
        },
        database: DatabaseConfig {
            url: "postgresql://prod-db/mydb".to_string(),
            max_connections: 50,
            timeout_seconds: 60,
        },
        features: FeatureFlags {
            new_ui: true,
            beta_api: false,
            realtime_notification: true,
        },
    };

    let path1 = manager.save(&config1).await?;
    let filename1 = path1.file_name().unwrap().to_string_lossy().to_string();

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let path2 = manager.save(&config2).await?;
    let filename2 = path2.file_name().unwrap().to_string_lossy().to_string();

    let diff = manager.diff::<AppConfig>(&filename1, &filename2).await?;

    info!("配置差异对比:");
    info!("  旧快照: {}", filename1);
    info!("  新快照: {}", filename2);
    info!("");
    diff.print();

    Ok(())
}

/// 演示敏感字段脱敏
async fn demo_sensitive_redaction() -> Result<(), Box<dyn std::error::Error>> {
    info!("\n=== 演示 4: 敏感字段脱敏 ===\n");

    #[derive(Serialize, Deserialize, Debug, Clone)]
    struct SensitiveConfig {
        app_name: String,
        database: DatabaseSensitive,
        api_keys: ApiKeys,
    }

    #[derive(Serialize, Deserialize, Debug, Clone, Default)]
    struct DatabaseSensitive {
        url: String,
        password: String,
        username: String,
    }

    #[derive(Serialize, Deserialize, Debug, Clone, Default)]
    struct ApiKeys {
        public_key: String,
        secret_key: String,
    }

    let config = SensitiveConfig {
        app_name: "secure-app".to_string(),
        database: DatabaseSensitive {
            url: "postgresql://localhost/mydb".to_string(),
            password: "super-secret-password".to_string(),
            username: "admin".to_string(),
        },
        api_keys: ApiKeys {
            public_key: "pk_live_xxx".to_string(),
            secret_key: "sk_live_yyy".to_string(),
        },
    };

    let manager = SnapshotManager::new(SnapshotConfig::new("config-snapshots"));

    manager.add_sensitive_path("database.password".to_string()).await;
    manager.add_sensitive_path("api_keys.secret_key".to_string()).await;

    let path = manager.save(&config).await?;
    let content = tokio::fs::read_to_string(&path).await?;

    info!("脱敏后的快照内容:");
    info!("{}", content);

    assert!(content.contains("[REDACTED]"));
    assert!(!content.contains("super-secret-password"));
    assert!(!content.contains("sk_live_yyy"));

    info!("\n✓ 敏感字段已成功脱敏");

    Ok(())
}

/// 演示快照清理
async fn demo_snapshot_cleanup() -> Result<(), Box<dyn std::error::Error>> {
    info!("\n=== 演示 5: 快照清理 ===\n");

    let manager = SnapshotManager::new(
        SnapshotConfig::new("config-snapshots")
            .with_max_snapshots(5),
    );

    let config = AppConfig::default();

    for i in 0..8 {
        let mut cfg = config.clone();
        cfg.version = format!("1.0.{}", i);
        manager.save(&cfg).await?;
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }

    let entries = manager.list().await?;
    info!("创建 8 个快照后，保留 {} 个（max_snapshots=5）", entries.len());

    for entry in &entries {
        info!("  {}", entry.filename);
    }

    let deleted = manager.prune_older_than(1).await?;
    info!("\n清理 1 天前的快照: 删除 {} 个", deleted);

    Ok(())
}
