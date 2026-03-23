//! ConfigBus 多实例广播示例
//!
//! 本示例展示如何使用 confers 的 ConfigBus 功能实现多实例配置同步：
//! - NATS 消息总线集成
//! - Redis Pub/Sub 集成
//! - 配置变更事件广播
//! - 多实例配置一致性保证
//!
//! 运行方式：
//!   # 启动 NATS 服务器
//!   docker run -d --name nats -p 4222:4222 nats:latest
//!
//!   # 运行示例
//!   cargo run --bin config_bus
//!
//! 设计依据：ADR-035（ConfigBus）
//! 对标框架：Spring Cloud Bus

use futures::StreamExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{info, warn};

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
    /// 监听地址
    #[serde(default = "default_host")]
    pub host: String,

    /// 监听端口
    #[serde(default = "default_port")]
    pub port: u16,

    /// 工作线程数
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

/// 功能开关配置
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct FeatureFlags {
    /// 启用新 UI
    #[serde(default)]
    pub new_ui: bool,

    /// 启用 Beta API
    #[serde(default)]
    pub beta_api: bool,

    /// 启用实时通知
    #[serde(default)]
    pub realtime_notification: bool,
}

// =============================================================================
// ConfigBus 核心类型定义
// =============================================================================

/// 配置变更事件
///
/// 当配置发生变化时，通过 ConfigBus 广播此事件到所有实例
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigChangeEvent {
    /// 实例 ID（标识配置变更的来源）
    pub instance_id: String,

    /// 事件时间戳
    pub timestamp: chrono::DateTime<chrono::Utc>,

    /// 触发变更的源名称
    pub source: String,

    /// 变更的顶层键（不含敏感字段）
    pub changed_keys: Vec<String>,

    /// 新配置的 SHA-256 校验和（用于幂等性检查）
    pub checksum: String,

    /// 配置版本号
    pub version: u64,
}

impl ConfigChangeEvent {
    /// 创建新的配置变更事件
    pub fn new(instance_id: &str, source: &str, changed_keys: Vec<String>, checksum: &str) -> Self {
        Self {
            instance_id: instance_id.to_string(),
            timestamp: chrono::Utc::now(),
            source: source.to_string(),
            changed_keys,
            checksum: checksum.to_string(),
            version: 0,
        }
    }

    /// 从配置计算 SHA-256 校验和
    pub fn calculate_checksum<T: Serialize>(config: &T) -> String {
        let json = serde_json::to_vec(config).unwrap_or_default();
        let mut hasher = Sha256::new();
        hasher.update(&json);
        format!("{:x}", hasher.finalize())
    }

    /// 设置版本号
    pub fn with_version(mut self, version: u64) -> Self {
        self.version = version;
        self
    }
}

/// 配置事件流类型别名
pub type ConfigEventStream = Pin<Box<dyn futures::Stream<Item = ConfigChangeEvent> + Send>>;

/// ConfigBus 枚举 - 支持多种后端实现
///
/// 设计依据：ADR-035
/// 对标框架：Spring Cloud Bus
#[derive(Clone)]
pub enum ConfigBus {
    Nats(NatsConfigBus),
    Redis(RedisConfigBus),
    InMemory(InMemoryConfigBus),
}

impl ConfigBus {
    /// 发布配置变更事件
    pub async fn publish(&self, event: ConfigChangeEvent) -> confers::ConfigResult<()> {
        match self {
            ConfigBus::Nats(bus) => bus.publish(event).await,
            ConfigBus::Redis(bus) => bus.publish(event).await,
            ConfigBus::InMemory(bus) => bus.publish(event).await,
        }
    }

    /// 订阅配置变更事件
    pub async fn subscribe(&self) -> confers::ConfigResult<ConfigEventStream> {
        match self {
            ConfigBus::Nats(bus) => bus.subscribe().await,
            ConfigBus::Redis(bus) => bus.subscribe().await,
            ConfigBus::InMemory(bus) => bus.subscribe().await,
        }
    }
}

// =============================================================================
// NATS ConfigBus 实现
// =============================================================================

/// NATS ConfigBus 实现
///
/// 推荐用于容器化环境，轻量级且高性能
#[derive(Clone)]
#[allow(dead_code)]
pub struct NatsConfigBus {
    /// NATS 客户端
    client: Option<Arc<async_nats::Client>>,

    /// 订阅主题
    subject: String,

    /// 实例 ID
    instance_id: String,

    /// 连接状态
    connected: Arc<RwLock<bool>>,
}

impl NatsConfigBus {
    /// 创建新的 NATS ConfigBus
    pub fn new(instance_id: &str, subject: &str) -> Self {
        Self {
            client: None,
            subject: subject.to_string(),
            instance_id: instance_id.to_string(),
            connected: Arc::new(RwLock::new(false)),
        }
    }

    /// 连接到 NATS 服务器
    pub async fn connect(&mut self, url: &str) -> confers::ConfigResult<()> {
        info!("正在连接 NATS 服务器: {}", url);

        match async_nats::connect(url).await {
            Ok(client) => {
                self.client = Some(Arc::new(client));
                *self.connected.write().await = true;
                info!("NATS 连接成功");
                Ok(())
            }
            Err(e) => {
                warn!("NATS 连接失败: {}", e);
                Err(confers::ConfigError::RemoteUnavailable {
                    error_type: format!("nats_connect: {}", e),
                    retryable: true,
                })
            }
        }
    }

    /// 检查连接状态
    pub async fn is_connected(&self) -> bool {
        *self.connected.read().await
    }

    /// 发布事件
    async fn publish(&self, event: ConfigChangeEvent) -> confers::ConfigResult<()> {
        let client =
            self.client
                .as_ref()
                .ok_or_else(|| confers::ConfigError::RemoteUnavailable {
                    error_type: "NATS client not connected".to_string(),
                    retryable: false,
                })?;

        let payload =
            serde_json::to_vec(&event).map_err(|e| confers::ConfigError::RemoteUnavailable {
                error_type: format!("serialize: {}", e),
                retryable: false,
            })?;

        client
            .publish(self.subject.clone(), payload.into())
            .await
            .map_err(|e| confers::ConfigError::RemoteUnavailable {
                error_type: format!("nats_publish: {}", e),
                retryable: true,
            })?;

        info!(
            "[NATS] 配置变更事件已发布: instance={}, keys={:?}",
            event.instance_id, event.changed_keys
        );

        Ok(())
    }

    /// 订阅事件
    async fn subscribe(&self) -> confers::ConfigResult<ConfigEventStream> {
        let client =
            self.client
                .as_ref()
                .ok_or_else(|| confers::ConfigError::RemoteUnavailable {
                    error_type: "NATS client not connected".to_string(),
                    retryable: false,
                })?;

        let subscriber = client.subscribe(self.subject.clone()).await.map_err(|e| {
            confers::ConfigError::RemoteUnavailable {
                error_type: format!("nats_subscribe: {}", e),
                retryable: true,
            }
        })?;

        info!("[NATS] 已订阅主题: {}", self.subject);

        let stream = subscriber
            .then(|msg| async move { serde_json::from_slice(&msg.payload).ok() })
            .filter_map(|x| async move { x });

        Ok(Box::pin(stream))
    }
}

// =============================================================================
// Redis ConfigBus 实现
// =============================================================================

/// Redis ConfigBus 实现
///
/// 适用于已有 Redis 基础设施的环境
#[derive(Clone)]
#[allow(dead_code)]
pub struct RedisConfigBus {
    /// Redis 连接管理器
    connection_manager: Option<Arc<redis::aio::ConnectionManager>>,

    /// 发布/订阅频道
    channel: String,

    /// 实例 ID
    instance_id: String,
}

impl RedisConfigBus {
    /// 创建新的 Redis ConfigBus
    pub fn new(instance_id: &str, channel: &str) -> Self {
        Self {
            connection_manager: None,
            channel: channel.to_string(),
            instance_id: instance_id.to_string(),
        }
    }

    /// 连接到 Redis 服务器
    pub async fn connect(&mut self, url: &str) -> confers::ConfigResult<()> {
        info!("正在连接 Redis 服务器: {}", url);

        let client =
            redis::Client::open(url).map_err(|e| confers::ConfigError::RemoteUnavailable {
                error_type: format!("redis_client: {}", e),
                retryable: false,
            })?;

        let manager = redis::aio::ConnectionManager::new(client)
            .await
            .map_err(|e| confers::ConfigError::RemoteUnavailable {
                error_type: format!("redis_connect: {}", e),
                retryable: true,
            })?;

        self.connection_manager = Some(Arc::new(manager));
        info!("Redis 连接成功");

        Ok(())
    }

    /// 发布事件
    async fn publish(&self, event: ConfigChangeEvent) -> confers::ConfigResult<()> {
        let manager = self.connection_manager.as_ref().ok_or_else(|| {
            confers::ConfigError::RemoteUnavailable {
                error_type: "Redis client not connected".to_string(),
                retryable: false,
            }
        })?;

        let payload =
            serde_json::to_string(&event).map_err(|e| confers::ConfigError::RemoteUnavailable {
                error_type: format!("serialize: {}", e),
                retryable: false,
            })?;

        let mut conn = manager.as_ref().clone();
        redis::cmd("PUBLISH")
            .arg(&self.channel)
            .arg(&payload)
            .query_async::<()>(&mut conn)
            .await
            .map_err(|e| confers::ConfigError::RemoteUnavailable {
                error_type: format!("redis_publish: {}", e),
                retryable: true,
            })?;

        info!(
            "[Redis] 配置变更事件已发布: instance={}, keys={:?}",
            event.instance_id, event.changed_keys
        );

        Ok(())
    }

    /// 订阅事件
    async fn subscribe(&self) -> confers::ConfigResult<ConfigEventStream> {
        Err(confers::ConfigError::RemoteUnavailable {
            error_type: "Redis Pub/Sub 订阅需要单独连接，请参考 redis crate 文档".to_string(),
            retryable: false,
        })
    }
}

// =============================================================================
// 内存 ConfigBus 实现（用于测试）
// =============================================================================

/// 内存 ConfigBus 实现
///
/// 用于单机测试和开发环境
#[derive(Clone)]
#[allow(dead_code)]
pub struct InMemoryConfigBus {
    /// 事件发送通道
    sender: Arc<tokio::sync::broadcast::Sender<ConfigChangeEvent>>,

    /// 实例 ID
    instance_id: String,
}

impl InMemoryConfigBus {
    /// 创建新的内存 ConfigBus
    pub fn new(instance_id: &str) -> Self {
        let (sender, _) = tokio::sync::broadcast::channel(100);
        Self {
            sender: Arc::new(sender),
            instance_id: instance_id.to_string(),
        }
    }
}

impl InMemoryConfigBus {
    /// 发布事件
    async fn publish(&self, event: ConfigChangeEvent) -> confers::ConfigResult<()> {
        self.sender
            .send(event)
            .map_err(|e| confers::ConfigError::RemoteUnavailable {
                error_type: format!("broadcast: {}", e),
                retryable: false,
            })?;

        Ok(())
    }

    /// 订阅事件
    async fn subscribe(&self) -> confers::ConfigResult<ConfigEventStream> {
        let receiver = self.sender.subscribe();
        let stream = tokio_stream::wrappers::BroadcastStream::new(receiver)
            .then(|result| async move { result.ok() })
            .filter_map(|x| async move { x });
        Ok(Box::pin(stream))
    }
}

// =============================================================================
// 配置管理器
// =============================================================================

/// 配置管理器
///
/// 负责配置的加载、更新和广播
pub struct ConfigManager<T: Clone + Send + Sync + 'static> {
    /// 当前配置
    config: Arc<RwLock<T>>,

    /// ConfigBus
    bus: ConfigBus,

    /// 实例 ID
    instance_id: String,
}

impl<T: Clone + Send + Sync + 'static> ConfigManager<T> {
    /// 创建新的配置管理器
    pub fn new(initial_config: T, bus: ConfigBus, instance_id: &str) -> Self {
        Self {
            config: Arc::new(RwLock::new(initial_config)),
            bus,
            instance_id: instance_id.to_string(),
        }
    }

    /// 获取当前配置
    pub async fn get_config(&self) -> T {
        self.config.read().await.clone()
    }

    /// 更新配置并广播变更
    pub async fn update_config(
        &self,
        new_config: T,
        changed_keys: Vec<String>,
    ) -> confers::ConfigResult<()> {
        *self.config.write().await = new_config.clone();

        // 使用简单的校验和：基于时间戳和变更键
        let checksum_input = format!(
            "{}:{:?}:{}",
            chrono::Utc::now().timestamp(),
            changed_keys,
            self.instance_id
        );
        let mut hasher = Sha256::new();
        hasher.update(checksum_input.as_bytes());
        let checksum = format!("{:x}", hasher.finalize());

        let event =
            ConfigChangeEvent::new(&self.instance_id, "manual_update", changed_keys, &checksum);

        self.bus.publish(event).await?;

        info!("[ConfigManager] 配置已更新并广播");
        Ok(())
    }

    /// 启动配置变更监听
    pub async fn start_listener(&self) -> confers::ConfigResult<()> {
        let mut stream = self.bus.subscribe().await?;

        info!("[ConfigManager] 开始监听配置变更事件");

        while let Some(event) = stream.next().await {
            if event.instance_id == self.instance_id {
                continue;
            }

            info!(
                "[ConfigManager] 收到配置变更: from={}, keys={:?}",
                event.instance_id, event.changed_keys
            );
        }

        Ok(())
    }
}

// =============================================================================
// 主程序
// =============================================================================

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .with_target(false)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .init();

    info!("========================================");
    info!("  ConfigBus 多实例广播示例");
    info!("========================================");

    demo_in_memory_bus().await?;
    demo_nats_bus().await?;
    demo_config_manager().await?;

    info!("");
    info!("========================================");
    info!("  示例运行完成");
    info!("========================================");

    Ok(())
}

/// 演示内存 ConfigBus
async fn demo_in_memory_bus() -> Result<(), Box<dyn std::error::Error>> {
    info!("\n=== 演示 1: 内存 ConfigBus ===\n");

    let bus = ConfigBus::InMemory(InMemoryConfigBus::new("instance-1"));

    let mut stream = bus.subscribe().await?;

    let event = ConfigChangeEvent::new(
        "instance-1",
        "file_watcher",
        vec!["server.port".to_string(), "features.new_ui".to_string()],
        "abc123",
    );

    bus.publish(event.clone()).await?;
    info!("✓ 事件已发布");

    let bus_clone = bus.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(100)).await;
        let event2 = ConfigChangeEvent::new(
            "instance-2",
            "env_override",
            vec!["server.workers".to_string()],
            "def456",
        );
        bus_clone.publish(event2).await.unwrap();
    });

    if let Some(received) = tokio::time::timeout(Duration::from_secs(1), stream.next()).await? {
        info!(
            "✓ 收到事件: instance={}, source={}",
            received.instance_id, received.source
        );
    }

    Ok(())
}

/// 演示 NATS ConfigBus
async fn demo_nats_bus() -> Result<(), Box<dyn std::error::Error>> {
    info!("\n=== 演示 2: NATS ConfigBus ===\n");

    let nats_url =
        std::env::var("NATS_URL").unwrap_or_else(|_| "nats://127.0.0.1:4222".to_string());

    let mut nats_bus = NatsConfigBus::new("instance-1", "config.updates");

    match nats_bus.connect(&nats_url).await {
        Ok(_) => {
            info!("✓ NATS 连接成功");

            let bus = ConfigBus::Nats(nats_bus);
            let event = ConfigChangeEvent::new(
                "instance-1",
                "config_reload",
                vec!["database.url".to_string()],
                "xyz789",
            );

            bus.publish(event).await?;
            info!("✓ 事件已发布到 NATS");
        }
        Err(e) => {
            warn!(
                "NATS 连接失败（这是正常的，如果没有运行 NATS 服务器）: {}",
                e
            );
            info!("提示: 运行 'docker run -d --name nats -p 4222:4222 nats:latest' 启动 NATS");
        }
    }

    Ok(())
}

/// 演示配置管理器
async fn demo_config_manager() -> Result<(), Box<dyn std::error::Error>> {
    info!("\n=== 演示 3: 配置管理器 ===\n");

    let initial_config = AppConfig::default();

    let bus = ConfigBus::InMemory(InMemoryConfigBus::new("manager-instance"));

    let manager = ConfigManager::new(initial_config, bus.clone(), "manager-instance");

    let mut stream = bus.subscribe().await?;

    let config = manager.get_config().await;
    info!("初始配置: name={}, env={}", config.name, config.environment);

    let mut new_config = config.clone();
    new_config.environment = "production".to_string();
    new_config.features.new_ui = true;

    manager
        .update_config(
            new_config,
            vec!["environment".to_string(), "features.new_ui".to_string()],
        )
        .await?;

    if let Some(event) = tokio::time::timeout(Duration::from_secs(1), stream.next()).await? {
        info!(
            "✓ 收到配置变更广播: instance={}, changed_keys={:?}",
            event.instance_id, event.changed_keys
        );
    }

    let updated = manager.get_config().await;
    info!(
        "更新后配置: env={}, new_ui={}",
        updated.environment, updated.features.new_ui
    );

    info!("✓ 配置管理器演示完成");

    Ok(())
}
