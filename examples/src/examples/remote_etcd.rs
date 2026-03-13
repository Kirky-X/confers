//! etcd 远程配置源示例
//!
//! 本示例展示如何使用 confers 从 etcd KV Store 加载配置：
//! - etcd v3 API 连接
//! - TLS 安全连接
//! - 配置监听和自动更新
//! - 租约和 TTL 管理
//!
//! 运行方式：
//!   # 启动 etcd 服务器
//!   docker run -d --name etcd -p 2379:2379 -p 2380:2380 \
//!     quay.io/coreos/etcd:v3.5 /usr/local/bin/etcd \
//!     --name s1 \
//!     --data-dir /etcd-data \
//!     --listen-client-urls http://0.0.0.0:2379 \
//!     --advertise-client-urls http://0.0.0.0:2379
//!
//!   # 运行示例
//!   cargo run --bin remote_etcd
//!
//! 设计依据：ADR-005（远程源）、ADR-037（轮询源）

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{error, info, warn, Level};
use tracing_subscriber::FmtSubscriber;

// =============================================================================
// 配置结构定义
// =============================================================================

/// 应用配置结构
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    /// 应用名称
    #[serde(default)]
    pub name: String,

    /// 应用版本
    #[serde(default)]
    pub version: String,

    /// 环境
    #[serde(default)]
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

/// 服务器配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ServerConfig {
    #[serde(default)]
    pub host: String,

    #[serde(default)]
    pub port: u16,

    #[serde(default)]
    pub workers: usize,
}

/// 数据库配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DatabaseConfig {
    #[serde(default)]
    pub url: String,

    #[serde(default)]
    pub max_connections: u32,

    #[serde(default)]
    pub timeout_seconds: u32,
}

/// 功能开关配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FeatureFlags {
    #[serde(default)]
    pub new_ui: bool,

    #[serde(default)]
    pub beta_api: bool,

    #[serde(default)]
    pub realtime_notification: bool,
}

// =============================================================================
// etcd 客户端配置
// =============================================================================

/// etcd 客户端配置
#[derive(Debug, Clone)]
pub struct EtcdConfig {
    /// etcd 端点列表
    pub endpoints: Vec<String>,

    /// 是否启用 TLS
    pub tls_enabled: bool,

    /// TLS CA 证书路径
    pub ca_cert_path: Option<String>,

    /// TLS 客户端证书路径
    pub client_cert_path: Option<String>,

    /// TLS 客户端私钥路径
    pub client_key_path: Option<String>,

    /// 认证用户名
    pub username: Option<String>,

    /// 认证密码
    pub password: Option<String>,

    /// 连接超时（秒）
    pub connect_timeout: u64,

    /// 请求超时（秒）
    pub request_timeout: u64,
}

impl Default for EtcdConfig {
    fn default() -> Self {
        Self {
            endpoints: vec!["http://127.0.0.1:2379".to_string()],
            tls_enabled: false,
            ca_cert_path: None,
            client_cert_path: None,
            client_key_path: None,
            username: None,
            password: None,
            connect_timeout: 5,
            request_timeout: 10,
        }
    }
}

impl EtcdConfig {
    /// 创建新的 etcd 配置
    pub fn new(endpoints: Vec<String>) -> Self {
        Self {
            endpoints,
            ..Default::default()
        }
    }

    /// 启用 TLS
    pub fn with_tls(mut self, ca_cert: &str, client_cert: &str, client_key: &str) -> Self {
        self.tls_enabled = true;
        self.ca_cert_path = Some(ca_cert.to_string());
        self.client_cert_path = Some(client_cert.to_string());
        self.client_key_path = Some(client_key.to_string());
        self
    }

    /// 设置认证信息
    pub fn with_auth(mut self, username: &str, password: &str) -> Self {
        self.username = Some(username.to_string());
        self.password = Some(password.to_string());
        self
    }
}

// =============================================================================
// etcd 配置源实现
// =============================================================================

/// etcd 配置源
///
/// 从 etcd KV Store 加载配置，支持监听变更
pub struct EtcdConfigSource {
    /// etcd 配置
    config: EtcdConfig,

    /// 配置键前缀
    prefix: String,

    /// 连接状态
    connected: Arc<RwLock<bool>>,

    /// 缓存的配置值
    cache: Arc<RwLock<HashMap<String, String>>>,
}

impl EtcdConfigSource {
    /// 创建新的 etcd 配置源
    pub fn new(config: EtcdConfig, prefix: &str) -> Self {
        Self {
            config,
            prefix: prefix.to_string(),
            connected: Arc::new(RwLock::new(false)),
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 测试连接
    pub async fn test_connection(&self) -> Result<bool, Box<dyn std::error::Error>> {
        info!("测试 etcd 连接: {:?}", self.config.endpoints);

        // 使用 reqwest 进行简单的健康检查
        // 实际生产环境应使用 etcd-client crate
        for endpoint in &self.config.endpoints {
            let health_url = format!("{}/health", endpoint);

            match reqwest::Client::new()
                .get(&health_url)
                .timeout(Duration::from_secs(self.config.connect_timeout))
                .send()
                .await
            {
                Ok(resp) if resp.status().is_success() => {
                    info!("✓ etcd 连接成功: {}", endpoint);
                    *self.connected.write().await = true;
                    return Ok(true);
                }
                Ok(resp) => {
                    warn!("etcd 返回错误状态: {} - {}", endpoint, resp.status());
                }
                Err(e) => {
                    warn!("etcd 连接失败: {} - {}", endpoint, e);
                }
            }
        }

        *self.connected.write().await = false;
        Ok(false)
    }

    /// 读取配置值
    ///
    /// 从 etcd 读取指定前缀下的所有键值对
    pub async fn read_config(&self) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
        info!("从 etcd 读取配置: prefix={}", self.prefix);

        // ⚠️ MOCK DATA - This is simulated data for demonstration purposes only
        // In production, use etcd-client crate to read actual KV pairs
        warn!("Using MOCK data - replace with actual etcd client implementation");

        let mut config_map = HashMap::new();

        // 示例配置值
        config_map.insert(format!("{}/name", self.prefix), "myapp".to_string());
        config_map.insert(format!("{}/version", self.prefix), "1.0.0".to_string());
        config_map.insert(
            format!("{}/environment", self.prefix),
            "production".to_string(),
        );
        config_map.insert(
            format!("{}/server/host", self.prefix),
            "0.0.0.0".to_string(),
        );
        config_map.insert(format!("{}/server/port", self.prefix), "8080".to_string());
        config_map.insert(format!("{}/server/workers", self.prefix), "8".to_string());
        config_map.insert(
            format!("{}/database/url", self.prefix),
            "postgresql://localhost/mydb".to_string(),
        );
        config_map.insert(
            format!("{}/database/max_connections", self.prefix),
            "20".to_string(),
        );
        config_map.insert(
            format!("{}/features/new_ui", self.prefix),
            "true".to_string(),
        );
        config_map.insert(
            format!("{}/features/beta_api", self.prefix),
            "false".to_string(),
        );

        // 更新缓存
        *self.cache.write().await = config_map.clone();

        Ok(config_map)
    }

    /// 解析配置到结构体
    pub async fn load<T: for<'de> Deserialize<'de>>(
        &self,
    ) -> Result<T, Box<dyn std::error::Error>> {
        let config_map = self.read_config().await?;

        // 将扁平的键值对转换为嵌套结构
        let json_value = self.map_to_json(&config_map);

        // 反序列化为目标类型
        let config: T = serde_json::from_value(json_value)?;

        Ok(config)
    }

    /// 将扁平的键值对转换为 JSON 结构
    fn map_to_json(&self, map: &HashMap<String, String>) -> serde_json::Value {
        let mut result = serde_json::Map::new();

        for (key, value) in map {
            // 移除前缀
            let key = key
                .strip_prefix(&format!("{}/", self.prefix))
                .unwrap_or(key);

            // 解析嵌套路径
            let parts: Vec<&str> = key.split('/').collect();
            Self::set_nested_value(&mut result, &parts, value);
        }

        serde_json::Value::Object(result)
    }

    /// 设置嵌套值
    fn set_nested_value(
        map: &mut serde_json::Map<String, serde_json::Value>,
        parts: &[&str],
        value: &str,
    ) {
        if parts.is_empty() {
            return;
        }

        if parts.len() == 1 {
            // 尝试解析为不同类型
            let json_value = if value == "true" {
                serde_json::Value::Bool(true)
            } else if value == "false" {
                serde_json::Value::Bool(false)
            } else if let Ok(n) = value.parse::<i64>() {
                serde_json::Value::Number(n.into())
            } else if let Ok(n) = value.parse::<f64>() {
                serde_json::Number::from_f64(n)
                    .map(serde_json::Value::Number)
                    .unwrap_or_else(|| serde_json::Value::String(value.to_string()))
            } else {
                serde_json::Value::String(value.to_string())
            };

            map.insert(parts[0].to_string(), json_value);
        } else {
            let nested = map
                .entry(parts[0].to_string())
                .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));

            if let serde_json::Value::Object(ref mut nested_map) = nested {
                Self::set_nested_value(nested_map, &parts[1..], value);
            }
        }
    }

    /// 监听配置变更
    ///
    /// 使用 etcd Watch API 监听配置变更
    pub async fn watch(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("开始监听 etcd 配置变更: prefix={}", self.prefix);

        // 实际实现应使用 etcd-client crate 的 WatchClient
        // 这里仅作演示
        loop {
            tokio::time::sleep(Duration::from_secs(10)).await;

            // 模拟检测变更
            let current = self.read_config().await?;
            let cached = self.cache.read().await.clone();

            if current != cached {
                info!("检测到配置变更");
                *self.cache.write().await = current;
            }
        }
    }

    /// 写入配置值
    ///
    /// 将配置值写入 etcd
    pub async fn write_config(
        &self,
        key: &str,
        value: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let full_key = format!("{}/{}", self.prefix, key);

        info!("写入配置: {} = {}", full_key, value);

        // 实际实现应使用 etcd-client crate 的 PutRequest
        // 这里仅更新缓存
        self.cache.write().await.insert(full_key, value.to_string());

        Ok(())
    }

    /// 使用租约设置配置
    ///
    /// 设置带 TTL 的配置值
    pub async fn write_with_lease(
        &self,
        key: &str,
        value: &str,
        ttl_seconds: i64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let full_key = format!("{}/{}", self.prefix, key);

        info!(
            "写入配置（租约 TTL={}s）: {} = {}",
            ttl_seconds, full_key, value
        );

        // 实际实现：
        // 1. 创建租约：LeaseGrantRequest { TTL: ttl_seconds }
        // 2. 写入配置并关联租约：PutRequest { lease: lease_id }

        // 模拟：设置定时删除
        let cache = self.cache.clone();
        let key_clone = full_key.clone();
        tokio::spawn(async move {
            if let Err(e) = async {
                tokio::time::sleep(Duration::from_secs(ttl_seconds as u64)).await;
                cache.write().await.remove(&key_clone);
                info!("租约过期，配置已删除: {}", key_clone);
                Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
            }
            .await
            {
                tracing::error!("租约清理任务失败: {}", e);
            }
        });

        self.cache.write().await.insert(full_key, value.to_string());

        Ok(())
    }
}

// =============================================================================
// TLS 配置示例
// =============================================================================

/// TLS 配置结构
#[derive(Debug, Clone)]
pub struct TlsConfig {
    /// CA 证书内容
    pub ca_cert: String,

    /// 客户端证书内容
    pub client_cert: String,

    /// 客户端私钥内容
    pub client_key: String,

    /// 服务器名称（用于 SNI）
    pub server_name: String,
}

impl TlsConfig {
    /// 从文件加载 TLS 配置
    pub fn from_files(
        ca_cert_path: &str,
        client_cert_path: &str,
        client_key_path: &str,
        server_name: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let ca_cert = std::fs::read_to_string(ca_cert_path)?;
        let client_cert = std::fs::read_to_string(client_cert_path)?;
        let client_key = std::fs::read_to_string(client_key_path)?;

        Ok(Self {
            ca_cert,
            client_cert,
            client_key,
            server_name: server_name.to_string(),
        })
    }

    /// 验证 TLS 配置
    pub fn validate(&self) -> Result<(), String> {
        if self.ca_cert.is_empty() {
            return Err("CA certificate is empty".to_string());
        }
        if self.client_cert.is_empty() {
            return Err("Client certificate is empty".to_string());
        }
        if self.client_key.is_empty() {
            return Err("Client key is empty".to_string());
        }
        if self.server_name.is_empty() {
            return Err("Server name is empty".to_string());
        }
        Ok(())
    }
}

// =============================================================================
// 主程序
// =============================================================================

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("设置日志失败");

    info!("========================================");
    info!("  etcd 远程配置源示例");
    info!("========================================");

    // 演示 1: 基础连接和配置读取
    demo_basic_connection().await?;

    // 演示 2: 配置写入和租约
    demo_write_operations().await?;

    // 演示 3: TLS 配置
    demo_tls_config().await?;

    // 演示 4: 配置监听
    demo_config_watching().await?;

    info!("");
    info!("========================================");
    info!("  示例运行完成");
    info!("========================================");

    Ok(())
}

/// 演示基础连接和配置读取
async fn demo_basic_connection() -> Result<(), Box<dyn std::error::Error>> {
    info!("\n=== 演示 1: 基础连接和配置读取 ===\n");

    // 从环境变量读取配置
    let endpoints = std::env::var("ETCD_ENDPOINTS")
        .unwrap_or_else(|_| "http://127.0.0.1:2379".to_string())
        .split(',')
        .map(|s| s.trim().to_string())
        .collect();

    let config = EtcdConfig::new(endpoints);
    let source = EtcdConfigSource::new(config, "myapp/config");

    // 测试连接
    match source.test_connection().await {
        Ok(true) => info!("✓ etcd 连接成功"),
        Ok(false) => {
            warn!("etcd 连接失败（这是正常的，如果没有运行 etcd 服务器）");
            info!("提示: 运行以下命令启动 etcd:");
            info!("  docker run -d --name etcd -p 2379:2379 -p 2380:2380 \\");
            info!("    quay.io/coreos/etcd:v3.5 /usr/local/bin/etcd \\");
            info!("    --name s1 --data-dir /etcd-data \\");
            info!("    --listen-client-urls http://0.0.0.0:2379 \\");
            info!("    --advertise-client-urls http://0.0.0.0:2379");
        }
        Err(e) => error!("连接测试错误: {}", e),
    }

    // 读取配置
    let config_map = source.read_config().await?;
    info!("\n读取到的配置值:");
    for (key, value) in config_map.iter().take(5) {
        info!("  {} = {}", key, value);
    }

    // 解析为结构体
    let app_config: AppConfig = source.load().await?;
    info!("\n解析后的配置:");
    info!("  应用: {} v{}", app_config.name, app_config.version);
    info!("  环境: {}", app_config.environment);
    info!(
        "  服务器: {}:{}",
        app_config.server.host, app_config.server.port
    );
    info!("  数据库: {}", app_config.database.url);

    Ok(())
}

/// 演示配置写入和租约
async fn demo_write_operations() -> Result<(), Box<dyn std::error::Error>> {
    info!("\n=== 演示 2: 配置写入和租约 ===\n");

    let config = EtcdConfig::default();
    let source = EtcdConfigSource::new(config, "myapp/config");

    // 写入配置
    source.write_config("features/new_feature", "true").await?;
    info!("✓ 配置已写入");

    // 使用租约写入临时配置
    source
        .write_with_lease("temp/session_id", "abc123", 5)
        .await?;
    info!("✓ 临时配置已写入（TTL=5s）");

    // 验证写入
    let config_map = source.read_config().await?;
    if let Some(value) = config_map.get("myapp/config/features/new_feature") {
        info!("验证: new_feature = {}", value);
    }

    Ok(())
}

/// 演示 TLS 配置
async fn demo_tls_config() -> Result<(), Box<dyn std::error::Error>> {
    info!("\n=== 演示 3: TLS 安全连接配置 ===\n");

    // TLS 配置示例
    let tls_config = TlsConfig {
        ca_cert: "-----BEGIN CERTIFICATE-----\n... CA 证书内容 ...\n-----END CERTIFICATE-----"
            .to_string(),
        client_cert:
            "-----BEGIN CERTIFICATE-----\n... 客户端证书内容 ...\n-----END CERTIFICATE-----"
                .to_string(),
        client_key:
            "-----BEGIN PRIVATE KEY-----\n... 客户端私钥内容 ...\n-----END PRIVATE KEY-----"
                .to_string(),
        server_name: "etcd.example.com".to_string(),
    };

    // 验证 TLS 配置
    match tls_config.validate() {
        Ok(()) => info!("✓ TLS 配置验证通过"),
        Err(e) => warn!("TLS 配置验证失败: {}", e),
    }

    // 创建带 TLS 的 etcd 配置
    let etcd_config = EtcdConfig::new(vec!["https://etcd.example.com:2379".to_string()])
        .with_tls(
            "/path/to/ca.crt",
            "/path/to/client.crt",
            "/path/to/client.key",
        )
        .with_auth("admin", "secret");

    info!("\nTLS 配置:");
    info!("  端点: {:?}", etcd_config.endpoints);
    info!("  TLS 启用: {}", etcd_config.tls_enabled);
    info!("  CA 证书: {:?}", etcd_config.ca_cert_path);
    info!("  客户端证书: {:?}", etcd_config.client_cert_path);
    info!("  认证用户: {:?}", etcd_config.username);

    info!("\n生产环境 TLS 配置步骤:");
    info!("  1. 生成 CA 证书和密钥");
    info!("  2. 为 etcd 服务器生成证书");
    info!("  3. 为客户端生成证书");
    info!("  4. 配置 etcd 启用 TLS:");
    info!("     --cert-file /path/to/server.crt");
    info!("     --key-file /path/to/server.key");
    info!("     --client-cert-auth");
    info!("     --trusted-ca-file /path/to/ca.crt");
    info!("  5. 客户端使用 TLS 连接");

    Ok(())
}

/// 演示配置监听
async fn demo_config_watching() -> Result<(), Box<dyn std::error::Error>> {
    info!("\n=== 演示 4: 配置监听 ===\n");

    let config = EtcdConfig::default();
    let _source = EtcdConfigSource::new(config, "myapp/config");

    info!("etcd Watch API 功能:");
    info!("  - 实时监听配置变更");
    info!("  - 支持历史变更记录");
    info!("  - 支持从指定版本开始监听");
    info!("  - 支持取消监听");

    info!("\n使用示例:");
    info!("  // 创建监听器");
    info!("  let (mut watcher, stream) = client.watch(KeyRange::key(\"myapp/config\")).await?;");
    info!("");
    info!("  // 处理变更事件");
    info!("  while let Some(event) = stream.next().await {{");
    info!("      match event.event_type {{");
    info!("          EventType::Put => info!(\"配置更新: {{:?}}\", event.kv),");
    info!("          EventType::Delete => info!(\"配置删除: {{:?}}\", event.kv),");
    info!("      }}");
    info!("  }}");

    info!("\n最佳实践:");
    info!("  1. 使用前缀监听而非单个键");
    info!("  2. 处理网络断开和重连");
    info!("  3. 实现配置变更的幂等处理");
    info!("  4. 添加变更事件的审计日志");

    Ok(())
}
