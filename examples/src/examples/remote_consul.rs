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
    println!("\n=== 演示 1: 基础 Consul 配置轮询 ===\n");

    // 使用 ConsulSourceBuilder 构建 ConsulSource
    // build() 是同步方法，返回 ConfigResult<ConsulSource>
    let source = ConsulSourceBuilder::new()
        .address(address)
        .prefix("myapp/config")
        .format(Format::Json)
        .interval(Duration::from_secs(30))
        .build()?;

    println!("ConsulSource 构建成功");
    println!("  源 ID: {}", source.source_id().as_str());
    println!("  轮询间隔: {:?}", source.poll_interval());

    // 尝试轮询配置 —— 如果没有 Consul 运行，预期会失败
    println!("\n正在轮询 Consul KV (prefix: myapp/config)...");
    match source.poll().await {
        Ok(value) => {
            println!("✓ 配置加载成功");
            if value.is_map() {
                println!("  值类型: Map (包含多个键值对)");
            } else if value.is_null() {
                println!("  值类型: Null (前缀下无配置)");
            } else {
                println!("  值类型: 标量值");
            }
        }
        Err(e) => {
            // 连接失败是预期行为（无 Consul 运行时）
            println!("✗ 轮询失败（如果没有运行 Consul 这是正常的）");
            println!("  错误: {}", e);
            println!("\n  提示: 运行以下命令启动 Consul:");
            println!("    docker run -d --name consul -p 8500:8500 consul:latest");
        }
    }

    Ok(())
}

/// 演示带 ACL token 的 Consul 连接
async fn demo_with_token(address: &str) -> anyhow::Result<()> {
    println!("\n=== 演示 2: 带 ACL Token 的连接 ===\n");

    let source = ConsulSourceBuilder::new()
        .address(address)
        .prefix("secure/config")
        .token("your-acl-token-here")
        .interval(Duration::from_secs(60))
        .build()?;

    println!("带 Token 的 ConsulSource 构建成功");
    println!("  源 ID: {}", source.source_id().as_str());

    match source.poll().await {
        Ok(_) => println!("✓ 配置加载成功（Token 验证通过）"),
        Err(e) => {
            println!("✗ 轮询失败: {}", e);
            println!("  注意: 生产环境应始终使用 HTTPS + ACL Token");
        }
    }

    Ok(())
}

/// 演示构建参数与限制配置
async fn demo_builder_options() -> anyhow::Result<()> {
    println!("\n=== 演示 3: 构建器参数与安全限制 ===\n");

    // 展示响应大小限制和条目数限制（防止 DoS）
    let source = ConsulSourceBuilder::new()
        .address("127.0.0.1:8500")
        .prefix("config")
        .max_response_bytes(4 * 1024 * 1024) // 4 MB 上限
        .max_kv_entries(5_000) // 最多 5000 条 KV
        .build()?;

    println!("ConsulSource 安全限制配置:");
    println!("  源 ID: {}", source.source_id().as_str());
    println!("  最大响应体: 4 MB");
    println!("  最大 KV 条目数: 5000");
    println!("\n这些限制防止超大响应导致的 DoS/OOM (CWE-400/CWE-502)");

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("========================================");
    println!("  Remote Consul 配置源示例");
    println!("========================================");

    // 从环境变量读取 Consul 地址，默认本地
    let address = std::env::var("CONSUL_ADDRESS").unwrap_or_else(|_| "127.0.0.1:8500".to_string());

    println!("Consul 地址: {}", address);

    demo_basic_poll(&address).await?;
    demo_with_token(&address).await?;
    demo_builder_options().await?;

    println!("\n========================================");
    println!("  示例运行完成");
    println!("========================================");
    Ok(())
}
