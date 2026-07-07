// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! 渐进式重载示例
//!
//! 本示例展示如何使用 confers 的 ProgressiveReloader 实现配置热更新：
//! - 使用 Builder 构建 ProgressiveReloader
//! - 演示 Immediate / Canary / Linear 三种重载策略
//! - 查看重载结果与当前配置

use confers::interface::ConfigProvider;
use confers::watcher::{ProgressiveReloader, ReloadOutcome, ReloadStrategy};
use confers::AnnotatedValue;
use std::sync::Arc;
use std::time::Duration;

/// 示例配置类型
#[derive(Debug, Clone, PartialEq)]
struct AppConfig {
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
    println!("  渐进式重载示例");
    println!("========================================\n");

    let initial = Arc::new(AppConfig {
        port: 8080,
        host: "0.0.0.0".to_string(),
    });
    let provider = Arc::new(EmptyProvider) as Arc<dyn ConfigProvider>;

    // --- 1. Immediate 策略：立即切换 ---
    println!("--- 策略 1: Immediate（立即重载）---");
    let reloader = ProgressiveReloader::new(initial.clone(), ReloadStrategy::Immediate);
    println!("初始配置: port={}, host={}", initial.port, initial.host);

    let new_config = Arc::new(AppConfig {
        port: 9090,
        host: "127.0.0.1".to_string(),
    });
    let outcome = reloader
        .begin_reload(new_config.clone(), provider.clone())
        .await?;
    print_outcome(&outcome);
    println!(
        "当前配置: port={}, host={}",
        reloader.current().port,
        reloader.current().host
    );

    // --- 2. Canary 策略：金丝雀发布 ---
    println!("\n--- 策略 2: Canary（金丝雀发布）---");
    let canary_reloader = ProgressiveReloader::builder()
        .initial(initial.clone())
        .strategy(ReloadStrategy::Canary {
            trial_duration: Duration::from_millis(100),
            poll_interval: Duration::from_millis(20),
        })
        .build();
    println!("初始配置: port={}", canary_reloader.current().port);

    let canary_config = Arc::new(AppConfig {
        port: 8443,
        host: "0.0.0.0".to_string(),
    });
    let outcome = canary_reloader
        .begin_reload(canary_config.clone(), provider.clone())
        .await?;
    print_outcome(&outcome);
    println!("当前配置: port={}", canary_reloader.current().port);

    // --- 3. Linear 策略：线性推出 ---
    println!("\n--- 策略 3: Linear（线性推出）---");
    let linear_reloader = ProgressiveReloader::builder()
        .initial(initial.clone())
        .strategy(ReloadStrategy::Linear {
            steps: 3,
            interval: Duration::from_millis(20),
        })
        .build();
    println!("初始配置: port={}", linear_reloader.current().port);

    let linear_config = Arc::new(AppConfig {
        port: 3000,
        host: "127.0.0.1".to_string(),
    });
    let outcome = linear_reloader
        .begin_reload(linear_config.clone(), provider.clone())
        .await?;
    print_outcome(&outcome);
    println!("当前配置: port={}", linear_reloader.current().port);

    println!("\n========================================");
    println!("  示例运行完成");
    println!("========================================");
    Ok(())
}

/// 打印重载结果
fn print_outcome(outcome: &ReloadOutcome) {
    match outcome {
        ReloadOutcome::Committed => println!("✓ 重载结果: Committed（已提交）"),
        ReloadOutcome::RolledBack { reason } => {
            println!("✗ 重载结果: RolledBack（已回滚）— {}", reason)
        }
    }
}
