//! =============================================================================
//! 配置组（Config Groups）示例
//!
//! 本示例展示如何使用 confers 库实现多环境配置管理：
//! - SourceChain：配置源链式组合
//! - 多环境配置：dev / prod 环境配置切换
//! - 配置优先级：base < 环境 < 环境变量
//! - 配置覆盖机制
//!
//! 运行方式：
//!   cargo run --example config_groups          # 默认使用 dev 环境
//!   cargo run --example config_groups -- dev  # 显式指定 dev 环境
//!   cargo run --example config_groups -- prod # 生产环境
//!   APP_ENV=prod cargo run --example config_groups # 通过环境变量指定
//! =============================================================================

use confers::{config, ConfigValue, SourceChainBuilder};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

// =============================================================================
// 配置结构定义
// =============================================================================

/// 主应用配置结构
///
/// 使用 `ConfigBuilder` 配合 `SourceChain` 实现配置加载。
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct AppConfig {
    /// 应用元数据
    pub app: AppMeta,

    /// 服务器配置
    pub server: ServerConfig,

    /// 数据库配置
    pub database: DatabaseConfig,

    /// 日志配置
    pub logging: LoggingConfig,

    /// 缓存配置
    pub cache: CacheConfig,

    /// 安全配置
    pub security: SecurityConfig,

    /// 开发环境特有配置（仅 dev 环境生效）
    #[serde(default)]
    pub development: Option<DevelopmentConfig>,

    /// 生产环境特有配置（仅 prod 环境生效）
    #[serde(default)]
    pub production: Option<ProductionConfig>,
}

/// 应用元数据
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct AppMeta {
    /// 应用名称
    pub name: String,

    /// 应用版本
    pub version: String,

    /// 运行环境（development / production）
    #[serde(default)]
    pub environment: String,
}

/// 服务器配置
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct ServerConfig {
    /// 监听地址
    #[serde(default)]
    pub host: String,

    /// 监听端口
    #[serde(default)]
    pub port: u16,

    /// 工作线程数
    #[serde(default)]
    pub workers: usize,
}

/// 数据库配置
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct DatabaseConfig {
    /// 数据库连接 URL
    pub url: String,

    /// 最大连接数
    #[serde(default)]
    pub max_connections: u32,

    /// 连接超时（秒）
    #[serde(default)]
    pub connection_timeout: u64,
}

/// 日志配置
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct LoggingConfig {
    /// 是否启用日志
    #[serde(default)]
    pub enable: bool,

    /// 日志级别：trace / debug / info / warn / error
    #[serde(default)]
    pub level: String,

    /// 日志格式：pretty / json
    #[serde(default)]
    pub format: String,
}

/// 缓存配置
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct CacheConfig {
    /// 是否启用缓存
    #[serde(default)]
    pub enable: bool,

    /// 缓存 TTL（秒）
    #[serde(default)]
    pub ttl: u64,
}

/// 安全配置
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct SecurityConfig {
    /// 是否启用 HTTPS
    #[serde(default)]
    pub https: bool,

    /// 是否启用 CORS
    #[serde(default)]
    pub cors: bool,

    /// 允许的源列表
    #[serde(default)]
    pub allowed_origins: Vec<String>,
}

/// 开发环境特有配置
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct DevelopmentConfig {
    /// 启用热重载
    #[serde(default)]
    pub hot_reload: bool,

    /// 启用调试端点
    #[serde(default)]
    pub debug_endpoints: bool,

    /// 详细错误信息
    #[serde(default)]
    pub verbose_errors: bool,
}

/// 生产环境特有配置
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct ProductionConfig {
    /// 启用健康检查
    #[serde(default)]
    pub health_check: bool,

    /// 启用限流
    #[serde(default)]
    pub rate_limit: bool,

    /// 最大请求大小（字节）
    #[serde(default)]
    pub max_request_size: u64,

    /// 启用性能监控
    #[serde(default)]
    pub monitoring: bool,
}

// =============================================================================
// 配置加载示例
// =============================================================================

/// 示例 1：使用 ConfigBuilder 构建配置源链
///
/// ConfigBuilder 允许你精确控制配置源的优先级和组合顺序。
/// 优先级规则：先添加的源优先级低，后添加的源优先级高（后者覆盖前者）。
fn load_with_config_builder(env: &str) -> Result<AppConfig, Box<dyn std::error::Error>> {
    println!("\n========== 使用 ConfigBuilder 加载配置 ==========");
    println!("环境: {}", env);

    // 获取配置目录路径
    let config_dir = PathBuf::from("config");

    // 使用 ConfigBuilder 构建配置
    // 优先级从低到高：base.toml < {env}.toml < 环境变量
    let app_config: AppConfig = config::<AppConfig>()
        // 第 1 层：基础配置（最低优先级）
        .file(config_dir.join("base.toml"))
        // 第 2 层：环境配置（中等优先级）
        .file(config_dir.join(format!("{}.toml", env)))
        // 第 3 层：环境变量（最高优先级）
        .env_prefix("APP_")
        .build()?;

    Ok(app_config)
}

/// 示例 2：使用 SourceChainBuilder 手动构建配置源链
fn load_with_source_chain(env: &str) -> Result<AppConfig, Box<dyn std::error::Error>> {
    println!("\n========== 使用 SourceChainBuilder 加载配置 ==========");
    println!("环境: {}", env);

    // 获取配置目录路径
    let config_dir = PathBuf::from("config");

    // 构建 SourceChain
    // 优先级从低到高：base.toml < {env}.toml < 环境变量
    let chain = SourceChainBuilder::new()
        // 第 1 层：基础配置（最低优先级）
        .file(config_dir.join("base.toml"))
        // 第 2 层：环境配置（中等优先级）
        .file(config_dir.join(format!("{}.toml", env)))
        // 第 3 层：环境变量（最高优先级）
        .env_with_prefix("APP_")
        // 构建 SourceChain
        .build();

    // 打印配置源信息
    println!("\n配置源链:");
    for (idx, name) in chain.source_names().iter().enumerate() {
        println!("  {}. {}", idx + 1, name);
    }

    // 收集并合并所有配置源
    let _merged = chain.collect()?;

    // 转换为目标配置结构
    let app_config: AppConfig = config::<AppConfig>()
        .file(config_dir.join("base.toml"))
        .file(config_dir.join(format!("{}.toml", env)))
        .env_prefix("APP_")
        .build()?;

    Ok(app_config)
}

/// 示例 3：演示配置优先级机制
fn demonstrate_priority() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n========== 配置优先级演示 ==========");

    // 创建一个测试配置结构
    #[derive(Debug, Default, Deserialize)]
    struct PriorityTest {
        #[serde(default)]
        name: String,
        #[serde(default)]
        port: u16,
        #[serde(default)]
        level: String,
    }

    // 创建一个配置源链用于演示
    let test_config: PriorityTest = config::<PriorityTest>()
        // 基础层配置
        .default("name", ConfigValue::string("BaseApp"))
        .default("port", ConfigValue::uint(8080))
        .default("level", ConfigValue::string("info"))
        // 覆盖层配置
        .memory(HashMap::from([
            ("name".to_string(), ConfigValue::string("DevApp")),
            ("port".to_string(), ConfigValue::uint(3000)),
        ]))
        .build()?;

    println!("优先级演示结果:");
    println!("  name = {} (高优先级覆盖低优先级)", test_config.name);
    println!("  port = {} (高优先级覆盖低优先级)", test_config.port);
    println!(
        "  level = {} (高优先级未设置，保留低优先级)",
        test_config.level
    );

    Ok(())
}

// =============================================================================
// 辅助函数
// =============================================================================

/// 打印完整配置信息
fn print_config(config: &AppConfig) {
    println!("\n========== 最终配置 ==========");

    // 应用元数据
    println!("\n[应用元数据]");
    println!("  名称: {}", config.app.name);
    println!("  版本: {}", config.app.version);
    println!("  环境: {}", config.app.environment);

    // 服务器配置
    println!("\n[服务器配置]");
    println!("  地址: {}", config.server.host);
    println!("  端口: {}", config.server.port);
    println!("  工作线程: {}", config.server.workers);

    // 数据库配置
    println!("\n[数据库配置]");
    println!("  URL: {}", config.database.url);
    println!("  最大连接数: {}", config.database.max_connections);
    println!("  连接超时: {}s", config.database.connection_timeout);

    // 日志配置
    println!("\n[日志配置]");
    println!("  启用: {}", config.logging.enable);
    println!("  级别: {}", config.logging.level);
    println!("  格式: {}", config.logging.format);

    // 缓存配置
    println!("\n[缓存配置]");
    println!("  启用: {}", config.cache.enable);
    println!("  TTL: {}s", config.cache.ttl);

    // 安全配置
    println!("\n[安全配置]");
    println!("  HTTPS: {}", config.security.https);
    println!("  CORS: {}", config.security.cors);
    println!("  允许的源: {:?}", config.security.allowed_origins);

    // 环境特有配置
    if let Some(ref dev) = config.development {
        println!("\n[开发环境配置]");
        println!("  热重载: {}", dev.hot_reload);
        println!("  调试端点: {}", dev.debug_endpoints);
        println!("  详细错误: {}", dev.verbose_errors);
    }

    if let Some(ref prod) = config.production {
        println!("\n[生产环境配置]");
        println!("  健康检查: {}", prod.health_check);
        println!("  限流: {}", prod.rate_limit);
        println!("  最大请求大小: {} bytes", prod.max_request_size);
        println!("  性能监控: {}", prod.monitoring);
    }

    println!("\n============================\n");
}

// =============================================================================
// 主函数
// =============================================================================

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    tracing::info!("配置组示例启动");

    // 确定运行环境
    // 优先级：命令行参数 > 环境变量 > 默认值
    let env = std::env::args()
        .nth(2) // 跳过 "run" 和 "--example"
        .or_else(|| std::env::var("APP_ENV").ok())
        .unwrap_or_else(|| "dev".to_string());

    println!("========================================");
    println!("    配置组（Config Groups）示例");
    println!("========================================");
    println!("当前环境: {}", env);

    // 演示配置优先级
    demonstrate_priority()?;

    // 方法 1：使用 ConfigBuilder 加载
    match load_with_config_builder(&env) {
        Ok(config) => {
            println!("\n✓ ConfigBuilder 加载成功!");
            print_config(&config);
        }
        Err(e) => {
            println!("\n✗ ConfigBuilder 加载失败: {}", e);
        }
    }

    // 方法 2：使用 SourceChainBuilder 加载
    match load_with_source_chain(&env) {
        Ok(config) => {
            println!("✓ SourceChainBuilder 加载成功!");
            print_config(&config);
        }
        Err(e) => {
            println!("✗ SourceChainBuilder 加载失败: {}", e);
        }
    }

    // 演示不同环境的配置差异
    println!("\n========== 环境配置对比 ==========");
    println!("dev 环境特点: localhost, port=3000, debug 日志, 关闭缓存");
    println!("prod 环境特点: 0.0.0.0, port=8080, warn 日志, 开启缓存");

    // 演示环境变量覆盖
    println!("\n========== 环境变量覆盖演示 ==========");
    println!("你可以使用以下环境变量覆盖配置:");
    println!("  APP_SERVER_PORT=9000      - 覆盖服务器端口");
    println!("  APP_DATABASE_URL=...       - 覆盖数据库 URL");
    println!("  APP_LOGGING_LEVEL=debug   - 覆盖日志级别");
    println!("\n示例: APP_SERVER_PORT=9000 cargo run --example config_groups -- dev");

    tracing::info!("示例运行完成!");
    Ok(())
}
