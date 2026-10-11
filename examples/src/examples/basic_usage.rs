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

// ============================================================================
// 工具函数
// ============================================================================

/// 打印配置信息（用于演示）
fn print_config(config: &ConfersConfig) {
    println!("\n========== Configuration info ==========");

    // 服务器配置
    println!("\n[Server configuration]");
    println!("  host: {}", config.host);
    println!("  port: {}", config.port);
    println!("  workers: {}", config.workers);

    // 数据库配置
    println!("\n[Database configuration]");
    println!("  URL: {}", config.database_url);
    println!("  Max connections: {}", config.max_connections);

    // 功能配置
    println!("\n[Feature configuration]");
    println!("  logging enabled: {}", config.enable_logging);
    println!("  metrics enabled: {}", config.enable_metrics);
    println!("  log level: {}", config.log_level);

    // 应用元数据
    println!("\n[App metadata]");
    println!("  name: {}", config.app_name);
    println!("  version: {}", config.app_version);

    println!("\n==============================\n");
}

/// 验证配置的有效性
fn validate_config(config: &ConfersConfig) -> Result<(), String> {
    // 验证端口范围
    if config.port == 0 {
        return Err("server port must not be 0".to_string());
    }

    // 验证工作线程数
    if config.workers == 0 {
        return Err("worker count must not be 0".to_string());
    }

    // 验证数据库连接数
    if config.max_connections == 0 {
        return Err("max connections must not be 0".to_string());
    }

    // 验证日志级别
    let valid_log_levels = vec!["trace", "debug", "info", "warn", "error"];
    if !valid_log_levels.contains(&config.log_level.as_str()) {
        return Err(format!(
            "invalid log level: {} (valid values: {:?}",
            config.log_level, valid_log_levels
        ));
    }

    Ok(())
}

// ============================================================================
// 主函数
// ============================================================================

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("========================================");
    println!("  Basic Usage - basic config loading demo");
    println!("========================================");
    println!(
        "\ncurrent working directory: {:?}",
        std::env::current_dir()?
    );

    println!("\n[Environment variable override]");
    println!("Override configuration values with these environment variables:");
    println!("  export HOST=0.0.0.0");
    println!("  export PORT=9000");
    println!("  export DATABASE_URL='postgresql://prod-db/app'");
    println!("  export LOG_LEVEL=debug");

    println!("\nLoading configuration...");

    let config =
        ConfersConfig::load_sync().map_err(|e| format!("configuration load failed: {:?}", e))?;

    print_config(&config);

    println!("Validating configuration...");
    if let Err(e) = validate_config(&config) {
        eprintln!("Configuration validation failed: {}", e);
        return Err(e.into());
    }
    println!("✓ Configuration validation passed!");

    println!("\nConfiguration loaded and validated; the application can start...");
    println!("server address: {}:{}", config.host, config.port);

    if config.enable_logging {
        println!("logging enabled, level: {}", config.log_level);
    }

    if config.enable_metrics {
        println!("metrics enabled");
    }

    println!("\n========================================");
    println!("  Example completed!");
    println!("========================================");
    Ok(())
}
