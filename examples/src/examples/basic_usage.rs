// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! Basic Usage - 基础配置加载示例
//!
//! 本示例展示如何使用 confers 加载和管理配置：
//! - 使用默认配置值
//! - 从环境变量覆盖配置
//! - 使用 derive 宏定义配置结构
//! - 访问和验证配置值

use confers::Config;
use confers::i18n::{tr, tr_args};
use serde::Deserialize;

/// 应用配置结构
///
/// 使用 `#[derive(Config)]` 宏自动实现配置加载功能。
/// 使用 `#[config(default = ...)]` 为每个字段指定默认值。
#[derive(Config, Deserialize, Debug, Clone)]
pub struct ConfersConfig {
    /// 服务器监听地址
    #[config(default = "127.0.0.1".to_string())]
    pub host: String,

    /// 服务器监听端口
    #[config(default = 8080u16)]
    pub port: u16,

    /// 工作线程数
    #[config(default = 4usize)]
    pub workers: usize,

    /// 数据库连接 URL
    #[config(default = "postgresql://localhost:5432/app".to_string())]
    pub database_url: String,

    /// 最大连接数
    #[config(default = 10u32)]
    pub max_connections: u32,

    /// 是否启用日志
    #[config(default = true)]
    pub enable_logging: bool,

    /// 是否启用指标
    #[config(default = false)]
    pub enable_metrics: bool,

    /// 日志级别
    #[config(default = "info".to_string())]
    pub log_level: String,

    /// 应用名称
    #[config(default = "myapp".to_string())]
    pub app_name: String,

    /// 应用版本
    #[config(default = "1.0.0".to_string())]
    pub app_version: String,
}

// 工具函数

/// 打印配置信息（用于演示）
fn print_config(config: &ConfersConfig) {
    println!("\n{}", tr("example-basic-usage-config-info"));

    println!("\n{}", tr("example-basic-usage-server-section"));
    println!(
        "  {}",
        tr_args("example-basic-usage-host", &[("host", config.host.clone())])
    );
    println!(
        "  {}",
        tr_args(
            "example-basic-usage-port",
            &[("port", config.port.to_string())]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-basic-usage-workers",
            &[("workers", config.workers.to_string())]
        )
    );

    println!("\n{}", tr("example-basic-usage-database-section"));
    println!("  URL: {}", config.database_url);
    println!(
        "  {}",
        tr_args(
            "example-basic-usage-max-connections",
            &[("count", config.max_connections.to_string())]
        )
    );

    println!("\n{}", tr("example-basic-usage-feature-section"));
    println!(
        "  {}",
        tr_args(
            "example-basic-usage-logging-enabled",
            &[("enabled", config.enable_logging.to_string())]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-basic-usage-metrics-enabled",
            &[("enabled", config.enable_metrics.to_string())]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-basic-usage-log-level",
            &[("level", config.log_level.clone())]
        )
    );

    println!("\n{}", tr("example-basic-usage-app-metadata"));
    println!(
        "  {}",
        tr_args(
            "example-basic-usage-app-name",
            &[("name", config.app_name.clone())]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-basic-usage-app-version",
            &[("version", config.app_version.clone())]
        )
    );

    println!("\n==============================\n");
}

/// 验证配置的有效性
fn validate_config(config: &ConfersConfig) -> Result<(), String> {
    if config.port == 0 {
        return Err(tr("example-basic-usage-port-zero"));
    }

    if config.workers == 0 {
        return Err(tr("example-basic-usage-workers-zero"));
    }

    if config.max_connections == 0 {
        return Err(tr("example-basic-usage-max-connections-zero"));
    }

    let valid_log_levels = vec!["trace", "debug", "info", "warn", "error"];
    if !valid_log_levels.contains(&config.log_level.as_str()) {
        return Err(tr_args(
            "example-basic-usage-invalid-log-level",
            &[
                ("level", config.log_level.clone()),
                ("valid", format!("{valid_log_levels:?}")),
            ],
        ));
    }

    Ok(())
}

// 主函数

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("========================================");
    println!("  {}", tr("example-basic-usage-title"));
    println!("========================================");
    println!(
        "\n{}",
        tr_args(
            "example-basic-usage-cwd",
            &[("cwd", format!("{:?}", std::env::current_dir()?))]
        )
    );

    println!("\n{}", tr("example-basic-usage-env-section"));
    println!("{}", tr("example-basic-usage-env-hint"));
    println!("  export HOST=0.0.0.0");
    println!("  export PORT=9000");
    println!("  export DATABASE_URL='postgresql://prod-db/app'");
    println!("  export LOG_LEVEL=debug");

    println!("\n{}", tr("example-basic-usage-loading"));

    let config = ConfersConfig::load_sync().map_err(|e| {
        tr_args(
            "example-basic-usage-load-failed",
            &[("error", format!("{e:?}"))],
        )
    })?;

    print_config(&config);

    println!("{}", tr("example-basic-usage-validating"));
    if let Err(e) = validate_config(&config) {
        eprintln!(
            "{}",
            tr_args(
                "example-basic-usage-validation-failed",
                &[("error", e.clone())],
            )
        );
        return Err(e.into());
    }
    println!("{}", tr("example-basic-usage-validation-passed"));

    println!("\n{}", tr("example-basic-usage-ready"));
    println!(
        "{}",
        tr_args(
            "example-basic-usage-server-address",
            &[
                ("host", config.host.clone()),
                ("port", config.port.to_string())
            ]
        )
    );

    if config.enable_logging {
        println!(
            "{}",
            tr_args(
                "example-basic-usage-logging-level",
                &[("level", config.log_level.clone())]
            )
        );
    }

    if config.enable_metrics {
        println!("{}", tr("example-basic-usage-metrics-on"));
    }

    println!("\n========================================");
    println!("  {}", tr("example-basic-usage-completed"));
    println!("========================================");
    Ok(())
}
