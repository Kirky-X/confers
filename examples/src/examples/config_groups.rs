// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! 配置组（Config Groups）示例
//!
//! 本示例展示如何使用 confers 库实现多环境配置管理：
//! - SourceChain：配置源链式组合
//! - 多环境配置：dev / prod 环境配置切换
//! - 配置优先级：base < 环境 < 环境变量
//! - 配置覆盖机制
//!
//! 运行方式（环境参数可省略，默认 dev）：
//!   cargo run -p confers-examples --bin config_groups             # 默认 dev 环境
//!   cargo run -p confers-examples --bin config_groups -- dev      # 显式指定 dev 环境
//!   cargo run -p confers-examples --bin config_groups -- prod     # 生产环境
//!   APP_ENV=prod cargo run -p confers-examples --bin config_groups # 环境变量指定
//!
//! 本示例依赖 examples/config/ 下的 base.toml 与 {env}.toml。

use confers::i18n::{tr, tr_args};
use confers::{ConfigValue, SourceChainBuilder, config};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

// 配置结构定义

/// 主应用配置结构
///
/// 使用 `ConfigBuilder` 配合 `SourceChain` 实现配置加载。
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct ConfersConfig {
    /// 应用元数据
    pub app: ConfersMeta,

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
pub struct ConfersMeta {
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

// 配置加载示例

/// 示例 1：使用 ConfigBuilder 构建配置源链
///
/// ConfigBuilder 允许你精确控制配置源的优先级和组合顺序。
/// 优先级规则：先添加的源优先级低，后添加的源优先级高（后者覆盖前者）。
fn load_with_config_builder(env: &str) -> Result<ConfersConfig, Box<dyn std::error::Error>> {
    println!("\n{}", tr("example-config-groups-builder-header"));
    println!(
        "{}",
        tr_args(
            "example-config-groups-environment",
            &[("env", env.to_string())]
        )
    );

    let config_dir = config_dir();

    // 优先级从低到高：base.toml < {env}.toml < 环境变量
    let app_config: ConfersConfig = config::<ConfersConfig>()
        .file(config_dir.join("base.toml"))
        .file(config_dir.join(format!("{}.toml", env)))
        .env_prefix("APP_")
        .build()?;

    Ok(app_config)
}

/// 示例 2：使用 SourceChainBuilder 手动构建配置源链
fn load_with_source_chain(env: &str) -> Result<ConfersConfig, Box<dyn std::error::Error>> {
    println!("\n{}", tr("example-config-groups-chain-header"));
    println!(
        "{}",
        tr_args(
            "example-config-groups-environment",
            &[("env", env.to_string())]
        )
    );

    let config_dir = config_dir();

    // 优先级从低到高：base.toml < {env}.toml < 环境变量
    let chain = SourceChainBuilder::new()
        .file(config_dir.join("base.toml"))
        .file(config_dir.join(format!("{}.toml", env)))
        .env_with_prefix("APP_")
        .build();

    println!("\n{}", tr("example-config-groups-source-chain"));
    for (idx, name) in chain.source_names().iter().enumerate() {
        println!("  {}. {}", idx + 1, name);
    }

    let merged = chain.collect()?;
    let all_paths = merged.all_paths();
    let top_sections: Vec<&str> = all_paths
        .iter()
        .map(|p| p.as_ref())
        .filter(|p| !p.is_empty() && !p.contains('.'))
        .collect();
    println!(
        "\n{}",
        tr_args(
            "example-config-groups-merged-sections",
            &[("sections", format!("{top_sections:?}"))]
        )
    );

    let app_config: ConfersConfig = config::<ConfersConfig>()
        .file(config_dir.join("base.toml"))
        .file(config_dir.join(format!("{}.toml", env)))
        .env_prefix("APP_")
        .build()?;

    Ok(app_config)
}

/// 示例 3：演示配置优先级机制
fn demonstrate_priority() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n{}", tr("example-config-groups-priority-header"));

    #[derive(Debug, Default, Deserialize)]
    struct PriorityTest {
        #[serde(default)]
        name: String,
        #[serde(default)]
        port: u16,
        #[serde(default)]
        level: String,
    }

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

    println!("{}", tr("example-config-groups-priority-result"));
    println!(
        "  {}",
        tr_args(
            "example-config-groups-priority-name",
            &[("name", test_config.name.clone())]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-config-groups-priority-port",
            &[("port", test_config.port.to_string())]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-config-groups-priority-level",
            &[("level", test_config.level.clone())]
        )
    );

    Ok(())
}

// 辅助函数

/// 定位示例配置目录。
///
/// 文件源路径按惯例相对当前工作目录解析（真实应用通常在其运行目录下放 config/）。
/// 本示例有两种运行入口：仓库根（`cargo run -p confers-examples --bin ...`，此时
/// 工作目录是仓库根）或 examples/ 目录（`cargo run --bin ...`），因此按顺序探测两者。
/// 都找不到时返回默认的 "config"，让文件源报出含路径的 FileNotFound 错误。
fn config_dir() -> PathBuf {
    ["config", "examples/config"]
        .into_iter()
        .map(PathBuf::from)
        .find(|dir| dir.join("base.toml").is_file())
        .unwrap_or_else(|| PathBuf::from("config"))
}

/// 打印完整配置信息
fn print_config(config: &ConfersConfig) {
    println!("\n{}", tr("example-config-groups-final-header"));

    println!("\n{}", tr("example-config-groups-app-section"));
    println!(
        "  {}",
        tr_args(
            "example-config-groups-app-name",
            &[("name", config.app.name.clone())]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-config-groups-app-version",
            &[("version", config.app.version.clone())]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-config-groups-app-environment",
            &[("environment", config.app.environment.clone())]
        )
    );

    println!("\n{}", tr("example-config-groups-server-section"));
    println!(
        "  {}",
        tr_args(
            "example-config-groups-server-host",
            &[("host", config.server.host.clone())]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-config-groups-server-port",
            &[("port", config.server.port.to_string())]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-config-groups-server-workers",
            &[("workers", config.server.workers.to_string())]
        )
    );

    println!("\n{}", tr("example-config-groups-database-section"));
    println!("  URL: {}", config.database.url);
    println!(
        "  {}",
        tr_args(
            "example-config-groups-database-max-connections",
            &[("count", config.database.max_connections.to_string())]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-config-groups-database-timeout",
            &[("seconds", config.database.connection_timeout.to_string())]
        )
    );

    println!("\n{}", tr("example-config-groups-logging-section"));
    println!(
        "  {}",
        tr_args(
            "example-config-groups-logging-enabled",
            &[("enabled", config.logging.enable.to_string())]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-config-groups-logging-level",
            &[("level", config.logging.level.clone())]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-config-groups-logging-format",
            &[("format", config.logging.format.clone())]
        )
    );

    println!("\n{}", tr("example-config-groups-cache-section"));
    println!(
        "  {}",
        tr_args(
            "example-config-groups-cache-enabled",
            &[("enabled", config.cache.enable.to_string())]
        )
    );
    println!("  TTL: {}s", config.cache.ttl);

    println!("\n{}", tr("example-config-groups-security-section"));
    println!("  HTTPS: {}", config.security.https);
    println!("  CORS: {}", config.security.cors);
    println!(
        "  {}",
        tr_args(
            "example-config-groups-security-origins",
            &[("origins", format!("{:?}", config.security.allowed_origins))]
        )
    );

    if let Some(ref dev) = config.development {
        println!("\n{}", tr("example-config-groups-dev-section"));
        println!(
            "  {}",
            tr_args(
                "example-config-groups-dev-hot-reload",
                &[("enabled", dev.hot_reload.to_string())]
            )
        );
        println!(
            "  {}",
            tr_args(
                "example-config-groups-dev-debug-endpoints",
                &[("enabled", dev.debug_endpoints.to_string())]
            )
        );
        println!(
            "  {}",
            tr_args(
                "example-config-groups-dev-verbose-errors",
                &[("enabled", dev.verbose_errors.to_string())]
            )
        );
    }

    if let Some(ref prod) = config.production {
        println!("\n{}", tr("example-config-groups-prod-section"));
        println!(
            "  {}",
            tr_args(
                "example-config-groups-prod-health-check",
                &[("enabled", prod.health_check.to_string())]
            )
        );
        println!(
            "  {}",
            tr_args(
                "example-config-groups-prod-rate-limit",
                &[("enabled", prod.rate_limit.to_string())]
            )
        );
        println!(
            "  {}",
            tr_args(
                "example-config-groups-prod-max-request-size",
                &[("bytes", prod.max_request_size.to_string())]
            )
        );
        println!(
            "  {}",
            tr_args(
                "example-config-groups-prod-monitoring",
                &[("enabled", prod.monitoring.to_string())]
            )
        );
    }

    println!("\n============================\n");
}

// 主函数

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    tracing::info!("{}", tr("example-config-groups-started"));

    // 确定运行环境
    // 优先级：命令行参数 > 环境变量 > 默认值
    // cargo 会把 `--` 之后的参数原样传给二进制进程（不包含 cargo 自己的参数），
    // 因此 args[0] 是二进制名，args[1] 即第一个位置参数。
    // 例如 `cargo run -p confers-examples --bin config_groups -- prod`
    // 的进程参数为 ["config_groups", "prod"]。
    let env = std::env::args()
        .nth(1)
        .or_else(|| std::env::var("APP_ENV").ok())
        .unwrap_or_else(|| "dev".to_string());

    println!("========================================");
    println!("    {}", tr("example-config-groups-title"));
    println!("========================================");
    println!(
        "{}",
        tr_args("example-config-groups-current-env", &[("env", env.clone())])
    );

    demonstrate_priority()?;

    match load_with_config_builder(&env) {
        Ok(config) => {
            println!("\n{}", tr("example-config-groups-builder-loaded"));
            print_config(&config);
        }
        Err(e) => {
            println!(
                "\n{}",
                tr_args(
                    "example-config-groups-builder-failed",
                    &[("error", e.to_string())]
                )
            );
        }
    }

    match load_with_source_chain(&env) {
        Ok(config) => {
            println!("{}", tr("example-config-groups-chain-loaded"));
            print_config(&config);
        }
        Err(e) => {
            println!(
                "{}",
                tr_args(
                    "example-config-groups-chain-failed",
                    &[("error", e.to_string())]
                )
            );
        }
    }

    println!("\n{}", tr("example-config-groups-comparison-header"));
    println!("{}", tr("example-config-groups-dev-traits"));
    println!("{}", tr("example-config-groups-prod-traits"));

    println!("\n{}", tr("example-config-groups-override-header"));
    println!("{}", tr("example-config-groups-override-hint"));
    println!("{}", tr("example-config-groups-override-port"));
    println!("{}", tr("example-config-groups-override-database-url"));
    println!("{}", tr("example-config-groups-override-log-level"));
    println!("\n{}", tr("example-config-groups-override-example"));

    tracing::info!("{}", tr("example-config-groups-completed"));
    Ok(())
}
