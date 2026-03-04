use std::path::PathBuf;
use std::time::Duration;

use confers::watcher::{FsWatcher, WatcherConfig};
use serde::Deserialize;
use tracing::{error, info, warn, Level};
use tracing_subscriber::FmtSubscriber;

#[derive(Debug, Clone, Deserialize)]
struct AppConfig {
    application: ApplicationConfig,
    server: ServerConfig,
    logging: LoggingConfig,
    database: DatabaseConfig,
    cache: CacheConfig,
}

#[derive(Debug, Clone, Deserialize)]
struct ApplicationConfig {
    name: String,
    version: String,
    environment: String,
}

#[derive(Debug, Clone, Deserialize)]
struct ServerConfig {
    host: String,
    port: u16,
    max_connections: u32,
}

#[derive(Debug, Clone, Deserialize)]
struct LoggingConfig {
    level: String,
    format: String,
    output: String,
}

#[derive(Debug, Clone, Deserialize)]
struct DatabaseConfig {
    url: String,
    max_connections: u32,
    timeout_seconds: u32,
}

#[derive(Debug, Clone, Deserialize)]
struct CacheConfig {
    enabled: bool,
    ttl_seconds: u32,
    max_size_mb: u32,
}

impl AppConfig {
    fn load(path: impl AsRef<std::path::Path>) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path.as_ref())?;
        let config: AppConfig = toml::from_str(&content)?;
        Ok(config)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .with_thread_ids(false)
        .with_file(true)
        .with_line_number(true)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("设置日志追踪器失败");

    info!("=== 热重载示例程序启动 ===");

    let config_path = PathBuf::from("config/config.toml");

    if !config_path.exists() {
        error!("配置文件不存在: {:?}", config_path);
        return Ok(());
    }

    let watcher_config = WatcherConfig::builder()
        .debounce_ms(300)
        .min_reload_interval_ms(1000)
        .max_consecutive_failures(3)
        .failure_pause_ms(5000)
        .rollback_on_validation_failure(true)
        .build();

    info!(
        "WatcherConfig 配置: debounce={}ms, min_reload_interval={}ms, max_failures={}",
        watcher_config.debounce_ms,
        watcher_config.min_reload_interval_ms,
        watcher_config.max_consecutive_failures
    );

    let mut watcher = FsWatcher::new(&config_path, watcher_config.debounce_ms).await?;

    let config = AppConfig::load(&config_path)?;

    print_config(&config);

    let mut consecutive_failures = 0u32;
    let mut last_reload_time = std::time::Instant::now();

    info!("等待配置文件变化... (修改 config/config.toml 触发热重载)");
    info!("按 Ctrl+C 退出程序");

    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            info!("收到退出信号，正在关闭...");
        }
        _ = async {
            while let Some(changed_path) = watcher.recv().await {
                let now = std::time::Instant::now();
                let elapsed = now.duration_since(last_reload_time).as_millis() as u64;

                if elapsed < watcher_config.min_reload_interval_ms {
                    info!(
                        "跳过重载: 距离上次重载仅 {}ms (最小间隔: {}ms)",
                        elapsed,
                        watcher_config.min_reload_interval_ms
                    );
                    continue;
                }

                info!("检测到配置文件变化: {:?}", changed_path);

                match AppConfig::load(&changed_path) {
                    Ok(new_config) => {
                        consecutive_failures = 0;
                        last_reload_time = now;

                        info!("配置重载成功!");
                        print_config(&new_config);
                    }
                    Err(e) => {
                        consecutive_failures += 1;
                        error!("配置重载失败: {} (连续失败: {}/{})",
                               e,
                               consecutive_failures,
                               watcher_config.max_consecutive_failures);

                        if consecutive_failures >= watcher_config.max_consecutive_failures {
                            warn!("连续失败次数达到上限，暂停监听 {}ms",
                                  watcher_config.failure_pause_ms);

                            tokio::time::sleep(Duration::from_millis(watcher_config.failure_pause_ms)).await;
                            consecutive_failures = 0;
                        }
                    }
                }
            }
        } => {}
    }

    watcher.stop();

    info!("=== 程序退出 ===");
    Ok(())
}

fn print_config(config: &AppConfig) {
    println!("\n{}", "=".repeat(60));
    println!("当前配置 (已重载)");
    println!("{}", "=".repeat(60));
    println!(
        "应用: {} v{} (环境: {})",
        config.application.name, config.application.version, config.application.environment
    );
    println!(
        "服务器: {}:{} (最大连接数: {})",
        config.server.host, config.server.port, config.server.max_connections
    );
    println!(
        "日志: 级别={}, 格式={}, 输出={}",
        config.logging.level, config.logging.format, config.logging.output
    );
    println!(
        "数据库: {} (最大连接: {}, 超时: {}s)",
        config.database.url, config.database.max_connections, config.database.timeout_seconds
    );
    println!(
        "缓存: 启用={}, TTL={}s, 最大={}MB",
        config.cache.enabled, config.cache.ttl_seconds, config.cache.max_size_mb
    );
    println!("{}\n", "=".repeat(60));
}
