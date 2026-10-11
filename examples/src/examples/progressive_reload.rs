// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! 渐进式重载示例
//!
//! 本示例展示如何使用 confers 的 ProgressiveReloader 实现配置热更新：
//! - 使用 Builder 构建 ProgressiveReloader
//! - 演示 Immediate / Canary / Linear 三种重载策略
//! - 查看重载结果与当前配置

use confers::AnnotatedValue;
use confers::i18n::{tr, tr_args};
use confers::interface::ConfigProvider;
use confers::watcher::{ProgressiveReloader, ReloadOutcome, ReloadStrategy};
use std::sync::Arc;
use std::time::Duration;

/// 示例配置类型
#[derive(Debug, Clone, PartialEq)]
struct ConfersConfig {
    port: u16,
    host: String,
}

/// 简单的 ConfigProvider 实现（供 begin_reload 使用）
struct EmptyProvider;

impl ConfigProvider for EmptyProvider {
    fn get_raw(&self, _key: &str) -> Option<&AnnotatedValue> {
        None
    }
    fn keys(&self) -> Vec<String> {
        vec![]
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("========================================");
    println!("  {}", tr("example-progressive-reload-title"));
    println!("========================================\n");

    let initial = Arc::new(ConfersConfig {
        port: 8080,
        host: "0.0.0.0".to_string(),
    });
    let provider = Arc::new(EmptyProvider) as Arc<dyn ConfigProvider>;

    println!("{}", tr("example-progressive-reload-strategy-immediate"));
    let reloader = ProgressiveReloader::new(initial.clone(), ReloadStrategy::Immediate);
    println!(
        "{}",
        tr_args(
            "example-progressive-reload-initial-config",
            &[
                ("port", initial.port.to_string()),
                ("host", initial.host.clone()),
            ]
        )
    );

    let new_config = Arc::new(ConfersConfig {
        port: 9090,
        host: "127.0.0.1".to_string(),
    });
    let outcome = reloader
        .begin_reload(new_config.clone(), provider.clone())
        .await?;
    print_outcome(&outcome);
    println!(
        "{}",
        tr_args(
            "example-progressive-reload-current-config",
            &[
                ("port", reloader.current().port.to_string()),
                ("host", reloader.current().host.clone()),
            ]
        )
    );

    println!("\n{}", tr("example-progressive-reload-strategy-canary"));
    let canary_reloader = ProgressiveReloader::builder()
        .initial(initial.clone())
        .strategy(ReloadStrategy::Canary {
            trial_duration: Duration::from_millis(100),
            poll_interval: Duration::from_millis(20),
        })
        .build();
    println!(
        "{}",
        tr_args(
            "example-progressive-reload-initial-port",
            &[("port", canary_reloader.current().port.to_string())]
        )
    );

    let canary_config = Arc::new(ConfersConfig {
        port: 8443,
        host: "0.0.0.0".to_string(),
    });
    let outcome = canary_reloader
        .begin_reload(canary_config.clone(), provider.clone())
        .await?;
    print_outcome(&outcome);
    println!(
        "{}",
        tr_args(
            "example-progressive-reload-current-port",
            &[("port", canary_reloader.current().port.to_string())]
        )
    );

    println!("\n{}", tr("example-progressive-reload-strategy-linear"));
    let linear_reloader = ProgressiveReloader::builder()
        .initial(initial.clone())
        .strategy(ReloadStrategy::Linear {
            steps: 3,
            interval: Duration::from_millis(20),
        })
        .build();
    println!(
        "{}",
        tr_args(
            "example-progressive-reload-initial-port",
            &[("port", linear_reloader.current().port.to_string())]
        )
    );

    let linear_config = Arc::new(ConfersConfig {
        port: 3000,
        host: "127.0.0.1".to_string(),
    });
    let outcome = linear_reloader
        .begin_reload(linear_config.clone(), provider.clone())
        .await?;
    print_outcome(&outcome);
    println!(
        "{}",
        tr_args(
            "example-progressive-reload-current-port",
            &[("port", linear_reloader.current().port.to_string())]
        )
    );

    println!("\n========================================");
    println!("  {}", tr("example-progressive-reload-completed"));
    println!("========================================");
    Ok(())
}

/// 打印重载结果
fn print_outcome(outcome: &ReloadOutcome) {
    match outcome {
        ReloadOutcome::Committed => {
            println!("{}", tr("example-progressive-reload-outcome-committed"))
        }
        ReloadOutcome::RolledBack { reason } => {
            println!(
                "{}",
                tr_args(
                    "example-progressive-reload-outcome-rolled-back",
                    &[("reason", reason.clone())]
                )
            )
        }
    }
}
