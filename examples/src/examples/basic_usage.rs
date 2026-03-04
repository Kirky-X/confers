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
pub struct AppConfig {
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
fn print_config(config: &AppConfig) {
    println!("\n========== 配置信息 ==========");

    // 服务器配置
    println!("\n[服务器配置]");
    println!("  地址: {}", config.host);
    println!("  端口: {}", config.port);
    println!("  工作线程: {}", config.workers);

    // 数据库配置
    println!("\n[数据库配置]");
    println!("  URL: {}", config.database_url);
    println!("  最大连接数: {}", config.max_connections);

    // 功能配置
    println!("\n[功能配置]");
    println!("  启用日志: {}", config.enable_logging);
    println!("  启用指标: {}", config.enable_metrics);
    println!("  日志级别: {}", config.log_level);

    // 应用元数据
    println!("\n[应用元数据]");
    println!("  名称: {}", config.app_name);
    println!("  版本: {}", config.app_version);

    println!("\n==============================\n");
}

/// 验证配置的有效性
fn validate_config(config: &AppConfig) -> Result<(), String> {
    // 验证端口范围
    if config.port == 0 {
        return Err("服务器端口不能为 0".to_string());
    }

    // 验证工作线程数
    if config.workers == 0 {
        return Err("工作线程数不能为 0".to_string());
    }

    // 验证数据库连接数
    if config.max_connections == 0 {
        return Err("最大连接数不能为 0".to_string());
    }

    // 验证日志级别
    let valid_log_levels = vec!["trace", "debug", "info", "warn", "error"];
    if !valid_log_levels.contains(&config.log_level.as_str()) {
        return Err(format!(
            "无效的日志级别: {}（有效值: {:?}",
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
    println!("  Basic Usage - 基础配置加载示例");
    println!("========================================");
    println!("\n当前工作目录: {:?}", std::env::current_dir()?);

    // 演示环境变量覆盖说明
    println!("\n[环境变量覆盖]");
    println!("可以通过设置以下环境变量来覆盖配置值：");
    println!("  export HOST=0.0.0.0");
    println!("  export PORT=9000");
    println!("  export DATABASE_URL='postgresql://prod-db/app'");
    println!("  export LOG_LEVEL=debug");

    // 加载配置 - 使用 load_sync() 自动从环境变量加载
    println!("\n正在加载配置...");

    let config = AppConfig::load_sync().map_err(|e| format!("配置加载失败: {:?}", e))?;

    // 打印配置信息
    print_config(&config);

    // 验证配置
    println!("正在验证配置...");
    if let Err(e) = validate_config(&config) {
        eprintln!("配置验证失败: {}", e);
        return Err(e.into());
    }
    println!("✓ 配置验证通过!");

    // 模拟应用运行
    println!("\n应用配置已加载并验证，可以开始运行应用...");
    println!("服务器地址: {}:{}", config.host, config.port);

    // 演示配置使用
    if config.enable_logging {
        println!("日志功能已启用，级别: {}", config.log_level);
    }

    if config.enable_metrics {
        println!("指标功能已启用");
    }

    println!("\n========================================");
    println!("  示例运行完成!");
    println!("========================================");
    Ok(())
}
