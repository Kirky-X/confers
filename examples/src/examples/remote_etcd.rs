// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! Remote etcd 配置源示例
//!
//! 本示例展示如何使用 confers 的 etcd 集成从 etcd KV Store 加载配置：
//! - 使用 `EtcdSourceBuilder` 构建配置源（异步 build）
//! - 配置 etcd 端点、认证凭证、KV 前缀
//! - 通过 `PolledSource` trait 轮询配置
//! - 优雅处理连接失败（无 etcd 运行时的预期行为）
//!
//! 运行方式：
//!   # 启动 etcd（可选）
//!   docker run -d --name etcd -p 2379:2379 \
//!     quay.io/coreos/etcd:v3.5 /usr/local/bin/etcd \
//!     --name s1 --data-dir /etcd-data \
//!     --listen-client-urls http://0.0.0.0:2379 \
//!     --advertise-client-urls http://0.0.0.0:2379
//!   # 运行示例
//!   cargo run --bin remote_etcd

use confers::loader::Format;
use confers::remote::{EtcdSourceBuilder, EtcdTlsConfig, PolledSource};
use std::time::Duration;

/// 演示基础 etcd 连接和配置轮询
async fn demo_basic_poll(endpoint: &str) -> anyhow::Result<()> {
    println!("\n=== Demo 1: Basic etcd configuration polling ===\n");

    // EtcdSourceBuilder 的 build() 是异步方法（与 Consul 的同步 build 不同）
    // 内部通过 etcd-client SDK 建立 gRPC 连接
    // 注意：endpoints() 替换默认端点，endpoint() 则追加到默认列表
    let build_result = EtcdSourceBuilder::new()
        .endpoints(vec![endpoint.to_string()])
        .prefix("myapp/config")
        .format(Format::Json)
        .interval(Duration::from_secs(30))
        .build()
        .await;

    let source = match build_result {
        Ok(s) => {
            println!("EtcdSource built successfully");
            println!("  source ID: {}", s.source_id().as_str());
            println!("  poll interval: {:?}", s.poll_interval());
            s
        }
        Err(e) => {
            // gRPC 连接可能惰性建立，build 失败说明端点不可达
            println!("✗ EtcdSource build failed (it is normal if etcd is not running)");
            println!("  error: {}", e);
            println!("\n  Hint: run the following command to start etcd:");
            println!("    docker run -d --name etcd -p 2379:2379 \\");
            println!("      quay.io/coreos/etcd:v3.5 /usr/local/bin/etcd \\");
            println!("      --name s1 --data-dir /etcd-data \\");
            println!("      --listen-client-urls http://0.0.0.0:2379 \\");
            println!("      --advertise-client-urls http://0.0.0.0:2379");
            return Ok(());
        }
    };

    // 尝试轮询配置 —— gRPC 连接是惰性的，build 成功不代表 poll 也会成功
    println!("\nPolling etcd KV (prefix: myapp/config)...");
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
            println!("✗ Poll failed (it is normal if etcd is not running)");
            println!("  error: {}", e);
        }
    }

    Ok(())
}

/// 演示带用户名/密码认证的 etcd 连接
async fn demo_with_auth(endpoint: &str) -> anyhow::Result<()> {
    println!("\n=== Demo 2: Authenticated etcd connection ===\n");

    let build_result = EtcdSourceBuilder::new()
        .endpoints(vec![endpoint.to_string()])
        .prefix("secure/config")
        .username("root")
        .password("secret")
        .interval(Duration::from_secs(60))
        .build()
        .await;

    match build_result {
        Ok(s) => {
            println!("EtcdSource with authentication built successfully");
            println!("  source ID: {}", s.source_id().as_str());
            match s.poll().await {
                Ok(_) => println!("✓ config loaded successfully"),
                Err(e) => println!("✗ Poll failed: {}", e),
            }
        }
        Err(e) => {
            println!("✗ build failed: {}", e);
        }
    }

    println!(
        "\n  Note: In production, read credentials from environment variables instead of hard-coding them"
    );

    Ok(())
}

/// 演示 TLS 配置与多端点集群
async fn demo_tls_and_endpoints() -> anyhow::Result<()> {
    println!("\n=== Demo 3: TLS configuration and multi-endpoint cluster ===\n");

    // EtcdTlsConfig 指定 CA 证书、客户端证书和私钥的文件路径
    let tls = EtcdTlsConfig {
        ca_file: "/etc/etcd/ca.pem".to_string(),
        cert_file: "/etc/etcd/client.pem".to_string(),
        key_file: "/etc/etcd/client-key.pem".to_string(),
    };

    // 多端点集群配置 + TLS
    let build_result = EtcdSourceBuilder::new()
        .endpoints(vec![
            "etcd1.example.com:2379".to_string(),
            "etcd2.example.com:2379".to_string(),
            "etcd3.example.com:2379".to_string(),
        ])
        .prefix("config")
        .tls(tls)
        .interval(Duration::from_secs(15))
        .build()
        .await;

    match build_result {
        Ok(s) => {
            println!("EtcdSource with TLS built successfully");
            println!("  source ID: {}", s.source_id().as_str());
            println!("  poll interval: {:?}", s.poll_interval());
        }
        Err(e) => {
            println!("✗ build failed (unreachable remote endpoints are normal)");
            println!("  error: {}", e);
        }
    }

    println!("\nProduction TLS configuration steps:");
    println!("  1. Generate the CA certificate and key");
    println!("  2. Generate certificates for the etcd server");
    println!("  3. Generate certificates for the client");
    println!("  4. Enable TLS when starting etcd (--cert-file, --key-file, --client-cert-auth)");
    println!("  5. Connect the client through the TLS endpoint");

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("========================================");
    println!("  Remote etcd config source demo");
    println!("========================================");

    // 从环境变量读取 etcd 端点，默认本地
    let endpoint = std::env::var("ETCD_ENDPOINT").unwrap_or_else(|_| "127.0.0.1:2379".to_string());

    println!("etcd endpoint: {}", endpoint);

    demo_basic_poll(&endpoint).await?;
    demo_with_auth(&endpoint).await?;
    demo_tls_and_endpoints().await?;

    println!("\n========================================");
    println!("  Example completed");
    println!("========================================");
    Ok(())
}
