//! CLI 集成示例 - ConfigClap 派生宏
//!
//! 本示例展示如何使用 confers 的 CLI 集成功能：
//! - `#[derive(ConfigClap)]` 自动生成 CLI 参数（基于 clap）
//! - `Option<T>` 字段映射为可选 CLI 参数，`bool` 字段映射为布尔标志
//! - `#[config(name = "server.host")]` 点号表示法：CLI 参数 `--server-host`
//! - CLI 参数覆盖配置默认值的完整工作流
//!
//! 运行方式：
//!   cargo run --bin cli_integration
//!   cargo run --bin cli_integration -- --help
//!   cargo run --bin cli_integration -- --server-host 0.0.0.0 --server-port 9000 --debug

use confers::Config;
use confers::ConfigClap;
use serde::Deserialize;
use std::collections::HashMap;

/// 应用配置结构
///
/// `ConfigClap` 派生宏自动生成 `AppConfigCliArgs` 结构体（实现 `clap::Parser`），
/// 以及 `AppConfig::clap_args()` / `clap_app()` 方法。
///
/// 字段类型与 CLI 参数的对应关系：
/// - `Option<T>` → 可选参数 `--flag <VALUE>`（未提供时为 None）
/// - `bool` → 布尔标志 `--flag`（存在为 true，不存在为 false）
///
/// `#[config(name = "server.host")]` 点号表示法：
/// - 配置键为 `server.host`
/// - CLI 参数自动生成为 `--server-host`（点号替换为连字符）
#[derive(Config, Deserialize, ConfigClap, Debug, Clone)]
#[config(env_prefix = "APP_")]
pub struct AppConfig {
    /// 服务器地址（配置键 server.host，CLI 参数 --server-host）
    #[config(
        name = "server.host",
        default = Some("127.0.0.1".to_string()),
        description = "服务器监听地址"
    )]
    pub host: Option<String>,

    /// 服务器端口（配置键 server.port，CLI 参数 --server-port）
    #[config(
        name = "server.port",
        default = Some(8080u16),
        description = "服务器监听端口"
    )]
    pub port: Option<u16>,

    /// 工作线程数
    #[config(default = Some(4usize), description = "工作线程数")]
    pub workers: Option<usize>,

    /// 日志级别
    #[config(
        default = Some("info".to_string()),
        description = "日志级别 (trace|debug|info|warn|error)"
    )]
    pub log_level: Option<String>,

    /// 调试模式（bool → CLI 标志 --debug）
    #[config(default = false, description = "启用调试模式")]
    pub debug: bool,
}

/// 打印配置
fn print_config(label: &str, config: &AppConfig) {
    println!("\n[{}]", label);
    println!("  host (server.host): {:?}", config.host);
    println!("  port (server.port): {:?}", config.port);
    println!("  workers:            {:?}", config.workers);
    println!("  log_level:          {:?}", config.log_level);
    println!("  debug:              {}", config.debug);
}

/// 打印 CLI 覆盖映射
fn print_overrides(map: &HashMap<String, confers::ConfigValue>) {
    println!("\n[CLI 覆盖映射 (to_config_map)]");
    if map.is_empty() {
        println!("  (空)");
    } else {
        for (key, value) in map {
            println!("  {} = {:?}", key, value);
        }
    }
}

/// 合并：CLI 覆盖中的非 Null 值覆盖默认配置
///
/// 注意：`to_config_map()` 的键是 Rust 字段名（如 "host"），
/// 不是 `#[config(name = ...)]` 的值（如 "server.host"）。
fn merge_config(
    config: &AppConfig,
    overrides: &HashMap<String, confers::ConfigValue>,
) -> AppConfig {
    let mut result = config.clone();

    if let Some(confers::ConfigValue::String(v)) = overrides.get("host") {
        result.host = Some(v.clone());
    }
    if let Some(v) = overrides.get("port").and_then(|v| v.as_u64()) {
        result.port = Some(v as u16);
    }
    if let Some(v) = overrides.get("workers").and_then(|v| v.as_u64()) {
        result.workers = Some(v as usize);
    }
    if let Some(confers::ConfigValue::String(v)) = overrides.get("log_level") {
        result.log_level = Some(v.clone());
    }
    if let Some(confers::ConfigValue::Bool(v)) = overrides.get("debug") {
        result.debug = *v;
    }

    result
}

fn main() -> anyhow::Result<()> {
    println!("========================================");
    println!("  CLI 集成示例 - ConfigClap 派生宏");
    println!("========================================");
    println!("\n运行方式:");
    println!("  cargo run --bin cli_integration -- --help");
    println!("  cargo run --bin cli_integration -- --server-host 0.0.0.0 --debug");

    // 步骤 1: 加载默认配置（来自 #[config(default = ...)]）
    println!("\n=== 步骤 1: 默认配置 ===");
    let config = AppConfig::default();
    print_config("默认配置 (Default)", &config);

    // 步骤 2: 解析 CLI 参数
    // clap_args() 调用 clap::Parser::parse()，解析 std::env::args()
    // 传入 --help 时 clap 会打印帮助并退出
    println!("\n=== 步骤 2: 解析 CLI 参数 (clap_args) ===");
    let cli_args = AppConfig::clap_args();
    println!("\n[CLI 解析结果]");
    println!("  host:      {:?}", cli_args.host);
    println!("  port:      {:?}", cli_args.port);
    println!("  workers:   {:?}", cli_args.workers);
    println!("  log_level: {:?}", cli_args.log_level);
    println!("  debug:     {}", cli_args.debug);

    // 步骤 3: 转换为覆盖映射
    println!("\n=== 步骤 3: 转换为覆盖映射 (to_config_map) ===");
    let overrides = cli_args.to_config_map();
    print_overrides(&overrides);

    // 步骤 4: 合并配置（CLI 非 Null 值覆盖默认值）
    println!("\n=== 步骤 4: 合并配置 (CLI 覆盖默认值) ===");
    let merged = merge_config(&config, &overrides);
    print_config("合并后配置", &merged);

    println!("\n========================================");
    println!("  示例运行完成");
    println!("========================================");
    Ok(())
}
