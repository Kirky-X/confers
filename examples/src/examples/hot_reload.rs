// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! 热重载示例 - 配置文件变更监听
//!
//! 本示例展示如何使用 confers 的热重载功能：
//! - 文件系统监听（`FsWatcher` + `WatcherConfig` 防抖与失败暂停）
//! - 配置变更检测与自动重载
//! - 重载失败的重试计数
//!
//! 运行方式（对当前工作目录无要求，无需手工准备配置文件）：
//!   cargo run -p confers-examples --bin hot_reload
//!
//! 流程：把 examples/config/hot_reload.toml 模板在编译期嵌入二进制（`include_str!`），
//! 启动时拷贝到系统临时目录并对该副本建立监听；约 2 秒后示例自身向同一文件
//! 写入端口已变更的新内容（原地写入，触发 Modify 事件），完整演示
//! 「修改 → 事件 → 重载 → 新值生效」闭环后自动退出。
//! 在真实应用中，只需把 `FsWatcher::new` 指向自己的配置文件路径即可。

use std::time::Duration;

use confers::i18n::{tr, tr_args};
use confers::watcher::{FsWatcher, WatcherConfig};
use serde::Deserialize;
use tracing::{error, info, warn};

/// 配置模板：编译期嵌入，修改 examples/config/hot_reload.toml 后重新运行即生效。
const CONFIG_TEMPLATE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/config/hot_reload.toml"
));

/// 演示修改的字段：模拟运维变更服务端口。
const PORT_OLD: &str = "port = 8080";
const PORT_NEW: &str = "port = 9090";

/// 写入演示修改前的延迟：留出监听建立（inotify watch 就绪）的时间。
const DEMO_MODIFY_DELAY: Duration = Duration::from_secs(2);

/// 等待变更事件的总上限：超时说明当前环境不支持文件系统事件通知
/// （网络文件系统、部分容器挂载等），显式报错而非无限等待。
const DEMO_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Debug, Clone, Deserialize)]
struct ConfersConfig {
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

impl ConfersConfig {
    fn load(path: impl AsRef<std::path::Path>) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path.as_ref())?;
        let config: ConfersConfig = toml::from_str(&content)?;
        Ok(config)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            // 尊重 RUST_LOG；未设置时才落到 info 基线（直接 add_directive 会
            // 用无 target 的指令覆盖 RUST_LOG，级别控制失效）。
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_target(false)
        .with_thread_ids(false)
        .with_file(true)
        .with_line_number(true)
        .init();

    info!("{}", tr("example-hot-reload-started"));

    // 模板拷贝到以进程号命名的临时目录，监听与修改都发生在副本上，
    // 多实例并行互不干扰，也不会污染仓库工作区。
    let demo_dir =
        std::env::temp_dir().join(format!("confers-hot-reload-demo-{}", std::process::id()));
    std::fs::create_dir_all(&demo_dir)?;
    let config_path = demo_dir.join("config.toml");
    std::fs::write(&config_path, CONFIG_TEMPLATE)?;
    info!(
        "{}",
        tr_args(
            "example-hot-reload-watched-file",
            &[("path", format!("{config_path:?}"))]
        )
    );

    let watcher_config = WatcherConfig::builder()
        .debounce_ms(300)
        .min_reload_interval_ms(1000)
        .max_consecutive_failures(3)
        .failure_pause_ms(5000)
        .rollback_on_validation_failure(true)
        .build();

    info!(
        "{}",
        tr_args(
            "example-hot-reload-watcher-settings",
            &[
                ("debounce", watcher_config.debounce_ms.to_string()),
                (
                    "min_interval",
                    watcher_config.min_reload_interval_ms.to_string()
                ),
                (
                    "max_failures",
                    watcher_config.max_consecutive_failures.to_string()
                ),
            ]
        )
    );

    // FsWatcher 内部监听文件的父目录（对编辑器式原子替换健壮），
    // 事件以绝对路径转发。
    let mut watcher = FsWatcher::new(&config_path, watcher_config.debounce_ms).await?;

    // 初始加载：真实应用启动时读取一次配置文件。
    let config = ConfersConfig::load(&config_path)?;

    print_config(&config);

    // 演示写入任务：延迟后把端口改为 9090 写回同一文件（原地写入，
    // 同一 inode，走 Modify 事件的最快通知路径）。
    let modify_path = config_path.clone();
    tokio::spawn(async move {
        tokio::time::sleep(DEMO_MODIFY_DELAY).await;
        let modified = CONFIG_TEMPLATE.replace(PORT_OLD, PORT_NEW);
        if modified == CONFIG_TEMPLATE {
            error!(
                "{}",
                tr_args(
                    "example-hot-reload-port-not-found",
                    &[("port", PORT_OLD.to_string())]
                )
            );
            return;
        }
        match std::fs::write(&modify_path, modified) {
            Ok(()) => info!(
                "{}",
                tr_args(
                    "example-hot-reload-demo-write-completed",
                    &[("old", PORT_OLD.to_string()), ("new", PORT_NEW.to_string()),]
                )
            ),
            Err(e) => error!(
                "{}",
                tr_args(
                    "example-hot-reload-demo-write-failed",
                    &[("error", e.to_string())]
                )
            ),
        }
    });

    info!(
        "{}",
        tr_args(
            "example-hot-reload-waiting",
            &[("seconds", format!("{:.1}", DEMO_MODIFY_DELAY.as_secs_f32()))]
        )
    );
    info!("{}", tr("example-hot-reload-ctrl-c-hint"));

    let mut consecutive_failures = 0u32;
    let mut last_reload_time = std::time::Instant::now();
    // 演示目标：至少完成一次「检测变化 → 重载成功 → 新值生效」。
    let mut reloaded = false;
    let mut failed = false;

    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            info!("{}", tr("example-hot-reload-exit-signal"));
        }
        _ = async {
            while let Some(changed_path) = watcher.recv().await {
                let now = std::time::Instant::now();
                let elapsed = now.duration_since(last_reload_time).as_millis() as u64;

                if elapsed < watcher_config.min_reload_interval_ms {
                    info!(
                        "{}",
                        tr_args(
                            "example-hot-reload-reload-skipped",
                            &[
                                ("elapsed", elapsed.to_string()),
                                (
                                    "min_interval",
                                    watcher_config.min_reload_interval_ms.to_string()
                                ),
                            ]
                        )
                    );
                    continue;
                }

                info!(
                    "{}",
                    tr_args(
                        "example-hot-reload-change-detected",
                        &[("path", format!("{changed_path:?}"))]
                    )
                );

                match ConfersConfig::load(&changed_path) {
                    Ok(new_config) => {
                        consecutive_failures = 0;
                        last_reload_time = now;

                        info!("{}", tr("example-hot-reload-reload-succeeded"));
                        print_config(&new_config);
                        reloaded = true;
                        break;
                    }
                    Err(e) => {
                        consecutive_failures += 1;
                        error!(
                            "{}",
                            tr_args(
                                "example-hot-reload-reload-failed",
                                &[
                                    ("error", e.to_string()),
                                    ("failures", consecutive_failures.to_string()),
                                    (
                                        "max_failures",
                                        watcher_config.max_consecutive_failures.to_string()
                                    ),
                                ]
                            )
                        );

                        if consecutive_failures >= watcher_config.max_consecutive_failures {
                            warn!(
                                "{}",
                                tr_args(
                                    "example-hot-reload-failure-limit",
                                    &[(
                                        "pause",
                                        watcher_config.failure_pause_ms.to_string()
                                    )]
                                )
                            );

                            tokio::time::sleep(Duration::from_millis(watcher_config.failure_pause_ms)).await;
                            consecutive_failures = 0;
                        }
                    }
                }
            }
            // 循环结束而未重载成功：watcher 已停止或失败（recv 返回 None），
            // 例如监听建立失败或监听目录被移除。显式上报而非当作正常退出。
            if !reloaded {
                error!("{}", tr("example-hot-reload-watcher-stopped"));
                failed = true;
            }
        } => {}
        _ = tokio::time::sleep(DEMO_TIMEOUT) => {
            error!(
                "{}",
                tr_args(
                    "example-hot-reload-timeout",
                    &[("seconds", DEMO_TIMEOUT.as_secs().to_string())]
                )
            );
            error!("{}", tr("example-hot-reload-no-fs-events"));
            failed = true;
        }
    }

    watcher.stop();

    // 尽力清理演示目录；失败仅残留临时文件，不影响结果。
    let _ = std::fs::remove_dir_all(&demo_dir);

    if reloaded {
        info!("{}", tr("example-hot-reload-demo-completed"));
        Ok(())
    } else if failed {
        Err(tr("example-hot-reload-demo-not-completed").into())
    } else {
        info!("{}", tr("example-hot-reload-exiting"));
        Ok(())
    }
}

fn print_config(config: &ConfersConfig) {
    println!("\n{}", "=".repeat(60));
    println!("{}", tr("example-hot-reload-current-config"));
    println!("{}", "=".repeat(60));
    println!(
        "{}",
        tr_args(
            "example-hot-reload-app-line",
            &[
                ("name", config.application.name.clone()),
                ("version", config.application.version.clone()),
                ("environment", config.application.environment.clone()),
            ]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-hot-reload-server-line",
            &[
                ("host", config.server.host.clone()),
                ("port", config.server.port.to_string()),
                ("max_connections", config.server.max_connections.to_string()),
            ]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-hot-reload-logs-line",
            &[
                ("level", config.logging.level.clone()),
                ("format", config.logging.format.clone()),
                ("output", config.logging.output.clone()),
            ]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-hot-reload-database-line",
            &[
                ("url", config.database.url.clone()),
                (
                    "max_connections",
                    config.database.max_connections.to_string()
                ),
                ("timeout", config.database.timeout_seconds.to_string()),
            ]
        )
    );
    println!(
        "{}",
        tr_args(
            "example-hot-reload-cache-line",
            &[
                ("enabled", config.cache.enabled.to_string()),
                ("ttl", config.cache.ttl_seconds.to_string()),
                ("max", config.cache.max_size_mb.to_string()),
            ]
        )
    );
    println!("{}\n", "=".repeat(60));
}
