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

use confers::loader::Format;
use confers::remote::{ConsulSourceBuilder, PolledSource};
use std::time::Duration;

/// 演示基础 Consul 连接和配置轮询
async fn demo_basic_poll(address: &str) -> anyhow::Result<()> {
    println!("\n=== Demo 1: Basic Consul configuration polling ===\n");

    // build() 是同步方法，返回 ConfigResult<ConsulSource>
    let source = ConsulSourceBuilder::new()
        .address(address)
        .prefix("myapp/config")
        .format(Format::Json)
        .interval(Duration::from_secs(30))
        .build()?;

    println!("ConsulSource built successfully");
    println!("  source ID: {}", source.source_id().as_str());
    println!("  poll interval: {:?}", source.poll_interval());

    // 尝试轮询配置 —— 如果没有 Consul 运行，预期会失败
    println!("\nPolling Consul KV (prefix: myapp/config)...");
    match source.poll().await {
        Ok(value) => {
            println!("✓ config loaded successfully");
            if value.is_map() {
                println!("  value type: Map (multiple key-value pairs)");
            } else if value.is_null() {
                println!("  value type: Null (no configuration under the prefix)");
            } else {
                println!("  value type: scalar value");
            }
        }
        Err(e) => {
            // 连接失败是预期行为（无 Consul 运行时）
            println!("✗ Poll failed (it is normal if Consul is not running)");
            println!("  error: {}", e);
            println!("\n  Hint: run the following command to start Consul:");
            println!("    docker run -d --name consul -p 8500:8500 consul:latest");
        }
    }

    Ok(())
}

/// 演示带 ACL token 的 Consul 连接
async fn demo_with_token(address: &str) -> anyhow::Result<()> {
    println!("\n=== Demo 2: Connection with ACL token ===\n");

    let source = ConsulSourceBuilder::new()
        .address(address)
        .prefix("secure/config")
        .token("your-acl-token-here")
        .interval(Duration::from_secs(60))
        .build()?;

    println!("ConsulSource with token built successfully");
    println!("  source ID: {}", source.source_id().as_str());

    match source.poll().await {
        Ok(_) => println!("✓ Config loaded successfully (token verified)"),
        Err(e) => {
            println!("✗ Poll failed: {}", e);
            println!("  Note: always use HTTPS + ACL token in production");
        }
    }

    Ok(())
}

/// 演示构建参数与限制配置
async fn demo_builder_options() -> anyhow::Result<()> {
    println!("\n=== Demo 3: Builder parameters and security limits ===\n");

    // 展示响应大小限制和条目数限制（防止 DoS）
    let source = ConsulSourceBuilder::new()
        .address("127.0.0.1:8500")
        .prefix("config")
        .max_response_bytes(4 * 1024 * 1024) // 4 MB 上限
        .max_kv_entries(5_000) // 最多 5000 条 KV
        .build()?;

    println!("ConsulSource security limits configuration:");
    println!("  source ID: {}", source.source_id().as_str());
    println!("  Max response body: 4 MB");
    println!("  Max KV entries: 5000");
    println!("\nThese limits prevent DoS/OOM from oversized responses (CWE-400/CWE-502)");

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("========================================");
    println!("  Remote Consul config source demo");
    println!("========================================");

    // 从环境变量读取 Consul 地址，默认本地
    let address = std::env::var("CONSUL_ADDRESS").unwrap_or_else(|_| "127.0.0.1:8500".to_string());

    println!("Consul address: {}", address);

    demo_basic_poll(&address).await?;
    demo_with_token(&address).await?;
    demo_builder_options().await?;

    println!("\n========================================");
    println!("  Example completed");
    println!("========================================");
    Ok(())
}
