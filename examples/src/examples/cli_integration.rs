//! CLI Integration Example - ConfigClap Derive Macro
//!
//! 本示例展示如何使用 confers 的 CLI 集成功能：
//! - `#[derive(ConfigClap)]` 派生宏自动生成 CLI 参数
//! - CLI 参数覆盖配置文件值
//! - 点号表示法的嵌套配置 CLI 参数
//! - 自动生成帮助文本
//! - CLI 参数与配置合并
//!
//! 设计依据：ADR-011（CLI 参数解析集成）
//!
//! 运行方式：
//!   cargo run --bin cli_integration
//!   cargo run --bin cli_integration -- --help
//!   cargo run --bin cli_integration -- --host 0.0.0.0 --port 9000
//!   cargo run --bin cli_integration -- --server.host localhost

use confers::Config;
use serde::Deserialize;

// =============================================================================
// 配置结构定义
// =============================================================================

/// 应用配置结构
///
/// 使用 `#[config]` 属性定义配置字段。
/// 派生 `ConfigClap` 后自动生成 CLI 参数支持。
#[derive(Config, Deserialize, Debug, Clone)]
#[config(env_prefix = "APP_")]
pub struct AppConfig {
    /// 服务器监听地址
    #[config(default = "127.0.0.1".to_string(), description = "服务器监听地址")]
    pub host: String,

    /// 服务器监听端口
    #[config(default = 8080u16, description = "服务器监听端口")]
    pub port: u16,

    /// 工作线程数
    #[config(default = 4usize, description = "工作线程数")]
    pub workers: usize,

    /// 日志级别
    #[config(default = "info".to_string(), description = "日志级别 (trace|debug|info|warn|error)")]
    pub log_level: String,

    /// 是否启用调试模式
    #[config(default = false, description = "启用调试模式")]
    pub debug: bool,
}

/// 服务器配置
#[derive(Config, Deserialize, Debug, Clone)]
#[config(env_prefix = "SERVER_")]
pub struct ServerConfig {
    /// 主机地址
    #[config(default = "0.0.0.0".to_string(), name = "host", description = "服务器主机")]
    pub host: String,

    /// 端口
    #[config(default = 8443u16, name = "port", description = "服务器端口")]
    pub port: u16,

    /// 连接超时（秒）
    #[config(default = 30u64, name = "timeout", description = "连接超时时间")]
    pub timeout: u64,

    /// 最大连接数
    #[config(default = 1000u32, name = "max_conn", description = "最大连接数")]
    pub max_connections: u32,
}

/// 数据库配置
#[derive(Config, Deserialize, Debug, Clone)]
#[config(env_prefix = "DB_")]
pub struct DatabaseConfig {
    /// 数据库主机
    #[config(default = "localhost".to_string(), description = "数据库主机")]
    pub host: String,

    /// 数据库端口
    #[config(default = 5432u16, description = "数据库端口")]
    pub port: u16,

    /// 数据库名称
    #[config(default = "appdb".to_string(), description = "数据库名称")]
    pub database: String,

    /// 最大连接数
    #[config(default = 20u32, description = "数据库最大连接数")]
    pub max_connections: u32,
}

/// 缓存配置
#[derive(Config, Deserialize, Debug, Clone)]
#[config(env_prefix = "CACHE_")]
pub struct CacheConfig {
    /// 启用缓存
    #[config(default = true, description = "是否启用缓存")]
    pub enabled: bool,

    /// TTL（秒）
    #[config(default = 300u64, description = "缓存 TTL")]
    pub ttl_seconds: u64,

    /// 最大条目数
    #[config(default = 10000u64, description = "缓存最大条目数")]
    pub max_entries: u64,
}

/// 完整应用配置（包含多个子配置）
#[derive(Config, Deserialize, Debug, Clone)]
#[config(env_prefix = "APP_", app_name = "myapp")]
pub struct FullAppConfig {
    /// 应用名称
    #[config(default = "myapp".to_string())]
    pub name: String,

    /// 服务器配置
    #[config(flatten)]
    pub server: ServerConfig,

    /// 数据库配置
    #[config(flatten)]
    pub database: DatabaseConfig,

    /// 缓存配置
    #[config(flatten)]
    pub cache: CacheConfig,
}

// =============================================================================
// CLI 参数处理
// =============================================================================

/// CLI 参数到配置的映射
#[derive(Debug, Clone)]
pub struct CliOverride {
    /// 参数名称（点号表示法）
    pub key: String,

    /// 参数值（字符串）
    pub value: String,
}

impl CliOverride {
    /// 从命令行参数创建覆盖列表
    pub fn from_args<I>(args: I) -> Vec<Self>
    where
        I: Iterator<Item = String>,
    {
        let mut overrides = Vec::new();
        let mut current_key: Option<String> = None;

        for arg in args {
            if arg.starts_with("--") {
                // 开始新的参数
                if let Some(key) = current_key.take() {
                    // 前一个参数没有值
                    overrides.push(Self {
                        key,
                        value: "true".to_string(),
                    });
                }
                current_key = Some(arg.trim_start_matches("--").replace("-", "_"));
            } else if let Some(key) = current_key.take() {
                // 前一个参数的值
                overrides.push(Self { key, value: arg });
            }
        }

        // 处理最后一个参数（如果它是键）
        if let Some(key) = current_key {
            overrides.push(Self {
                key,
                value: "true".to_string(),
            });
        }

        overrides
    }

    /// 转换为配置值的哈希映射
    pub fn to_config_map(
        overrides: &[Self],
    ) -> std::collections::HashMap<String, confers::ConfigValue> {
        let mut map = std::collections::HashMap::new();

        for override_ in overrides {
            let value = parse_value(&override_.value);
            map.insert(override_.key.clone(), value);
        }

        map
    }
}

/// 解析字符串值为 ConfigValue
fn parse_value(s: &str) -> confers::ConfigValue {
    // 尝试解析为布尔值
    match s.to_lowercase().as_str() {
        "true" | "yes" | "1" => return confers::ConfigValue::Bool(true),
        "false" | "no" | "0" => return confers::ConfigValue::Bool(false),
        _ => {}
    }

    // 尝试解析为数字
    if let Ok(n) = s.parse::<i64>() {
        return confers::ConfigValue::I64(n);
    }

    if let Ok(n) = s.parse::<f64>() {
        return confers::ConfigValue::F64(n);
    }

    // 默认为字符串
    confers::ConfigValue::String(s.to_string())
}

// =============================================================================
// 帮助文本生成
// =============================================================================

/// 生成配置参数的帮助文本
pub struct HelpGenerator;

impl HelpGenerator {
    /// 为配置结构生成帮助文本
    pub fn generate_for_config<T: 'static>() -> String {
        let mut lines = Vec::new();

        lines.push("Configuration Parameters:".to_string());
        lines.push(String::new());

        // 使用反射获取字段信息
        // 注意：这里使用硬编码的字段列表作为演示
        // 实际实现会通过 derive 宏自动生成

        lines.push("  --host <ADDR>        服务器监听地址 (default: 127.0.0.1)".to_string());
        lines.push("  --port <PORT>        服务器监听端口 (default: 8080)".to_string());
        lines.push("  --workers <N>         工作线程数 (default: 4)".to_string());
        lines.push("  --log-level <LEVEL>  日志级别 (default: info)".to_string());
        lines.push("  --debug <BOOL>       启用调试模式 (default: false)".to_string());

        lines.join("\n")
    }

    /// 生成完整的帮助文本
    pub fn generate_full_help(name: &str) -> String {
        format!(
            r#"
{name} - Configuration CLI Integration Example

USAGE:
    {name} [OPTIONS]

OPTIONS:
    --host <ADDR>           服务器监听地址 (default: 127.0.0.1)
    --port <PORT>           服务器监听端口 (default: 8080)
    --workers <N>           工作线程数 (default: 4)
    --log-level <LEVEL>     日志级别: trace, debug, info, warn, error (default: info)
    --debug                 启用调试模式 (default: false)
    -h, --help              显示帮助信息
    -V, --version           显示版本信息

ENVIRONMENT VARIABLES:
    APP_HOST                服务器监听地址
    APP_PORT                服务器监听端口
    APP_WORKERS             工作线程数
    APP_LOG_LEVEL           日志级别
    APP_DEBUG                启用调试模式

EXAMPLES:
    # 使用默认配置运行
    {name}

    # 覆盖服务器端口
    {name} --port 9000

    # 启用调试模式
    {name} --debug true --log-level debug

    # 从环境变量加载
    export APP_PORT=9000
    export APP_LOG_LEVEL=debug
    {name}

CONFIG MERGE ORDER (later sources override earlier):
    1. Default values (compiled into binary)
    2. Environment variables (APP_* prefix)
    3. Configuration file (config.toml)
    4. CLI arguments (highest priority)
"#,
            name = name
        )
    }
}

// =============================================================================
// 配置与 CLI 合并演示
// =============================================================================

/// 演示配置与 CLI 参数合并
fn demo_merge_config_and_cli() {
    println!("\n=== 演示 1: 配置与 CLI 参数合并 ===\n");

    // 模拟配置文件加载后的配置
    let config = AppConfig::load_sync().expect("配置加载失败");

    println!("原始配置 (来自配置文件/环境变量):");
    println!("  host: {}", config.host);
    println!("  port: {}", config.port);
    println!("  workers: {}", config.workers);
    println!("  log_level: {}", config.log_level);
    println!("  debug: {}", config.debug);

    // 模拟 CLI 参数
    let cli_args = vec![
        "--port".to_string(),
        "9000".to_string(),
        "--log-level".to_string(),
        "debug".to_string(),
    ];

    println!("\nCLI 参数:");
    for chunk in cli_args.chunks(2) {
        if chunk.len() == 2 {
            println!("  {} = {}", chunk[0], chunk[1]);
        }
    }

    // 解析 CLI 参数为覆盖
    let overrides = CliOverride::from_args(cli_args.into_iter());
    println!("\n解析后的 CLI 覆盖:");
    for override_ in &overrides {
        println!("  {} = {}", override_.key, override_.value);
    }

    // 合并配置
    let merged = merge_config(config, &overrides);
    println!("\n合并后的配置:");
    println!("  host: {} (不变)", merged.host);
    println!("  port: {} (被 CLI 覆盖)", merged.port);
    println!("  workers: {} (不变)", merged.workers);
    println!("  log_level: {} (被 CLI 覆盖)", merged.log_level);
    println!("  debug: {} (不变)", merged.debug);
}

/// 合并配置和 CLI 覆盖
fn merge_config(mut config: AppConfig, overrides: &[CliOverride]) -> AppConfig {
    for override_ in overrides {
        match override_.key.as_str() {
            "host" => config.host = override_.value.clone(),
            "port" => {
                if let Ok(port) = override_.value.parse() {
                    config.port = port;
                }
            }
            "workers" => {
                if let Ok(workers) = override_.value.parse() {
                    config.workers = workers;
                }
            }
            "log_level" => config.log_level = override_.value.clone(),
            "debug" => {
                if let Ok(debug) = override_.value.parse() {
                    config.debug = debug;
                }
            }
            _ => {}
        }
    }
    config
}

/// 演示嵌套配置的 CLI 参数
fn demo_nested_config_cli() {
    println!("\n=== 演示 2: 嵌套配置的 CLI 参数 ===\n");

    println!("使用点号表示法指定嵌套配置值:");
    println!();
    println!("  --server.host localhost");
    println!("  --server.port 9000");
    println!("  --database.host db.example.com");
    println!("  --cache.enabled true");
    println!();

    // 模拟嵌套 CLI 参数
    let nested_cli = vec![
        "--server.host".to_string(),
        "0.0.0.0".to_string(),
        "--server.port".to_string(),
        "443".to_string(),
        "--database.max_connections".to_string(),
        "100".to_string(),
    ];

    let overrides = CliOverride::from_args(nested_cli.into_iter());
    println!("解析后的嵌套覆盖:");
    for override_ in &overrides {
        println!("  {} = {}", override_.key, override_.value);
    }

    // 将覆盖应用到嵌套结构
    let config_map = CliOverride::to_config_map(&overrides);
    println!("\n配置值映射:");
    for (key, value) in &config_map {
        println!("  {} = {:?}", key, value);
    }
}

/// 演示帮助文本生成
fn demo_help_text() {
    println!("\n=== 演示 3: 帮助文本生成 ===\n");

    let help = HelpGenerator::generate_full_help("cli_integration");
    println!("{}", help);
}

/// 演示 ConfigClap 派生宏用法
fn demo_config_clap_derive() {
    println!("\n=== 演示 4: ConfigClap 派生宏 ===\n");

    println!("使用 #[derive(ConfigClap)] 自动生成 CLI 参数支持:");
    println!();
    println!("  #[derive(Config, Deserialize, ConfigClap)]");
    println!("  #[config(env_prefix = \"APP_\")]");
    println!("  pub struct AppConfig {{");
    println!("      #[config(default = \"127.0.0.1\".to_string(), description = \"服务器地址\")]");
    println!("      pub host: String,");
    println!();
    println!("      #[config(default = 8080u16, description = \"服务器端口\",");
    println!("               name_clap_short = 'p')]");
    println!("      pub port: u16,");
    println!("  }}");
    println!();
    println!("  // 自动生成:");
    println!("  // - AppConfigCliArgs 结构体 (implements clap::Args)");
    println!("  // - AppConfig::clap_args() 方法");
    println!("  // - AppConfig::clap_app() 方法");
    println!();
    println!("  let cli_args = AppConfig::clap_args();");
    println!("  let config = AppConfig::load_sync()");
    println!("      .with_overrides(cli_args.to_config_map());");
}

/// 演示 CLI 短参数和长参数
fn demo_short_and_long_args() {
    println!("\n=== 演示 5: CLI 参数命名 ===\n");

    println!("字段属性控制 CLI 参数名称:");
    println!();
    println!("  #[config(name_clap_long = \"server-host\")]");
    println!("  // 生成: --server-host");
    println!();
    println!("  #[config(name_clap_short = 'h')]");
    println!("  // 生成: -h");
    println!();
    println!("  #[config(name = \"db_host\", name_clap_long = \"database-host\")]");
    println!("  // 配置键: db_host");
    println!("  // CLI 参数: --database-host");
    println!();

    // 演示如何处理下划线和连字符转换
    println!("自动转换规则:");
    println!("  - 字段名中的 '_' 转换为 '-' (适用于长参数)");
    println!("  - 示例: server_host -> --server-host");
    println!("  - CLI 值解析为正确的类型 (u16, bool, String 等)");
}

/// 演示 CLI 与环境变量的优先级
fn demo_priority_order() {
    println!("\n=== 演示 6: 配置优先级 ===\n");

    println!("Confers 配置合并优先级（从低到高）:");
    println!();
    println!("  1. 默认值 (#[config(default = ...)] )");
    println!("     代码中硬编码的默认值");
    println!();
    println!("  2. 配置文件 (config.toml, config.yaml 等)");
    println!("     从文件系统加载的配置文件");
    println!();
    println!("  3. 环境变量 (APP_* 前缀)");
    println!("     系统环境变量，自动转换为配置键");
    println!();
    println!("  4. CLI 参数 (--flag value)");
    println!("     命令行参数，最高优先级");
    println!();
    println!("  适用场景:");
    println!("  - 开发环境: 使用配置文件");
    println!("  - 生产环境: 使用环境变量覆盖敏感配置");
    println!("  - 调试时: 使用 CLI 参数临时覆盖");
    println!();

    println!("示例优先级:");
    println!("  配置: port = 8080");
    println!("  ENV:  APP_PORT=9000");
    println!("  CLI:  --port=10000");
    println!("  最终: port = 10000  (CLI 优先)");
}

/// 演示完整的 CLI 工作流
fn demo_full_workflow() {
    println!("\n=== 演示 7: 完整 CLI 工作流 ===\n");

    println!("典型的 CLI + 配置工作流:");
    println!();
    println!("  Step 1: 定义配置结构");
    println!("  ```");
    println!("  #[derive(Config, Deserialize, ConfigClap)]");
    println!("  #[config(env_prefix = \"APP_\")]");
    println!("  pub struct AppConfig {{ ... }}");
    println!("  ```");
    println!();
    println!("  Step 2: 解析 CLI 参数");
    println!("  ```");
    println!("  let cli_args = AppConfig::clap_args();");
    println!("  ```");
    println!();
    println!("  Step 3: 加载配置");
    println!("  ```");
    println!("  let config = AppConfig::load_sync()?;");
    println!("  ```");
    println!();
    println!("  Step 4: 合并 CLI 覆盖");
    println!("  ```");
    println!("  let config = merge_with_overrides(config, &cli_args);");
    println!("  ```");
    println!();
    println!("  Step 5: 验证配置");
    println!("  ```");
    println!("  if let Err(e) = validate_config(&config) {{");
    println!("      eprintln!(\"配置无效: {{}}\", e);");
    println!("      std::process::exit(1);");
    println!("  }}");
    println!("  ```");
    println!();
    println!("  Step 6: 使用配置启动应用");
    println!("  ```");
    println!("  start_server(config).await;");
    println!("  ```");
}

// =============================================================================
// 主程序
// =============================================================================

fn main() {
    println!("========================================");
    println!("  CLI Integration Example");
    println!("  ConfigClap Derive Macro Demo");
    println!("========================================");

    // 检查是否请求帮助
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("{}", HelpGenerator::generate_full_help("cli_integration"));
        return;
    }

    if args.iter().any(|a| a == "--version" || a == "-V") {
        println!("cli_integration v{}", env!("CARGO_PKG_VERSION"));
        return;
    }

    demo_merge_config_and_cli();
    demo_nested_config_cli();
    demo_help_text();
    demo_config_clap_derive();
    demo_short_and_long_args();
    demo_priority_order();
    demo_full_workflow();

    println!("\n========================================");
    println!("  示例运行完成");
    println!("========================================");
}
