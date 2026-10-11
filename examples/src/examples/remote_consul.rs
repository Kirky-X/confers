// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! Remote Consul 配置源示例
//!
//! 本示例展示如何使用 confers 的 Consul 集成从 Consul KV Store 加载配置：
//! - 使用 `ConsulSourceBuilder` 构建配置源
//! - 配置 Consul 地址、ACL token、KV 前缀
//! - 通过 `PolledSource` trait 轮询配置
//! - 优雅处理连接失败（无 Consul 运行时的预期行为）
//!
//! 运行方式：
//!   # 启动 Consul（可选）
//!   docker run -d --name consul -p 8500:8500 consul:latest
//!   # 运行示例
//!   cargo run --bin remote_consul

use confers::i18n::{tr, tr_args};
use confers::loader::Format;
use confers::remote::{ConsulSourceBuilder, PolledSource};
use std::time::Duration;

/// 演示基础 Consul 连接和配置轮询
async fn demo_basic_poll(address: &str) -> anyhow::Result<()> {
    println!("\n{}\n", tr("example-remote-consul-demo1"));

    // build() 是同步方法，返回 ConfigResult<ConsulSource>
    let source = ConsulSourceBuilder::new()
        .address(address)
        .prefix("myapp/config")
        .format(Format::Json)
        .interval(Duration::from_secs(30))
        .build()?;

    println!("{}", tr("example-remote-consul-built"));
    println!(
        "  {}",
        tr_args(
            "example-remote-consul-source-id",
            &[("id", source.source_id().as_str().to_string())]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-remote-consul-poll-interval",
            &[("interval", format!("{:?}", source.poll_interval()))]
        )
    );

    // 尝试轮询配置 —— 如果没有 Consul 运行，预期会失败
    println!("\n{}", tr("example-remote-consul-polling"));
    match source.poll().await {
        Ok(value) => {
            println!("{}", tr("example-remote-consul-loaded"));
            if value.is_map() {
                println!("  {}", tr("example-remote-consul-value-map"));
            } else if value.is_null() {
                println!("  {}", tr("example-remote-consul-value-null"));
            } else {
                println!("  {}", tr("example-remote-consul-value-scalar"));
            }
        }
        Err(e) => {
            // 连接失败是预期行为（无 Consul 运行时）
            println!("{}", tr("example-remote-consul-poll-failed-normal"));
            println!(
                "  {}",
                tr_args("example-remote-consul-error", &[("error", e.to_string())])
            );
            println!("\n  {}", tr("example-remote-consul-hint"));
            println!("    docker run -d --name consul -p 8500:8500 consul:latest");
        }
    }

    Ok(())
}

/// 演示带 ACL token 的 Consul 连接
async fn demo_with_token(address: &str) -> anyhow::Result<()> {
    println!("\n{}\n", tr("example-remote-consul-demo2"));

    let source = ConsulSourceBuilder::new()
        .address(address)
        .prefix("secure/config")
        .token("your-acl-token-here")
        .interval(Duration::from_secs(60))
        .build()?;

    println!("{}", tr("example-remote-consul-with-token-built"));
    println!(
        "  {}",
        tr_args(
            "example-remote-consul-source-id",
            &[("id", source.source_id().as_str().to_string())]
        )
    );

    match source.poll().await {
        Ok(_) => println!("{}", tr("example-remote-consul-loaded-token-verified")),
        Err(e) => {
            println!(
                "{}",
                tr_args(
                    "example-remote-consul-poll-failed",
                    &[("error", e.to_string())]
                )
            );
            println!("  {}", tr("example-remote-consul-token-note"));
        }
    }

    Ok(())
}

/// 演示构建参数与限制配置
async fn demo_builder_options() -> anyhow::Result<()> {
    println!("\n{}\n", tr("example-remote-consul-demo3"));

    // 展示响应大小限制和条目数限制（防止 DoS）
    let source = ConsulSourceBuilder::new()
        .address("127.0.0.1:8500")
        .prefix("config")
        .max_response_bytes(4 * 1024 * 1024) // 4 MB 上限
        .max_kv_entries(5_000) // 最多 5000 条 KV
        .build()?;

    println!("{}", tr("example-remote-consul-security-limits"));
    println!(
        "  {}",
        tr_args(
            "example-remote-consul-source-id",
            &[("id", source.source_id().as_str().to_string())]
        )
    );
    println!("  {}", tr("example-remote-consul-max-response-body"));
    println!("  {}", tr("example-remote-consul-max-kv-entries"));
    println!("\n{}", tr("example-remote-consul-dos-limits-note"));

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("========================================");
    println!("  {}", tr("example-remote-consul-title"));
    println!("========================================");

    let address = std::env::var("CONSUL_ADDRESS").unwrap_or_else(|_| "127.0.0.1:8500".to_string());

    println!(
        "{}",
        tr_args(
            "example-remote-consul-address",
            &[("address", address.clone())]
        )
    );

    demo_basic_poll(&address).await?;
    demo_with_token(&address).await?;
    demo_builder_options().await?;

    println!("\n========================================");
    println!("  {}", tr("example-remote-consul-completed"));
    println!("========================================");
    Ok(())
}
