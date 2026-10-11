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

use confers::i18n::{tr, tr_args};
use confers::loader::Format;
use confers::remote::{EtcdSourceBuilder, EtcdTlsConfig, PolledSource};
use std::time::Duration;

/// 演示基础 etcd 连接和配置轮询
async fn demo_basic_poll(endpoint: &str) -> anyhow::Result<()> {
    println!("\n{}\n", tr("example-remote-etcd-demo1"));

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
            println!("{}", tr("example-remote-etcd-built"));
            println!(
                "  {}",
                tr_args(
                    "example-remote-etcd-source-id",
                    &[("id", s.source_id().as_str().to_string())]
                )
            );
            println!(
                "  {}",
                tr_args(
                    "example-remote-etcd-poll-interval",
                    &[("interval", format!("{:?}", s.poll_interval()))]
                )
            );
            s
        }
        Err(e) => {
            // gRPC 连接可能惰性建立，build 失败说明端点不可达
            println!("{}", tr("example-remote-etcd-build-failed-normal"));
            println!(
                "  {}",
                tr_args("example-remote-etcd-error", &[("error", e.to_string())])
            );
            println!("\n  {}", tr("example-remote-etcd-hint"));
            println!("    docker run -d --name etcd -p 2379:2379 \\");
            println!("      quay.io/coreos/etcd:v3.5 /usr/local/bin/etcd \\");
            println!("      --name s1 --data-dir /etcd-data \\");
            println!("      --listen-client-urls http://0.0.0.0:2379 \\");
            println!("      --advertise-client-urls http://0.0.0.0:2379");
            return Ok(());
        }
    };

    // 尝试轮询配置 —— gRPC 连接是惰性的，build 成功不代表 poll 也会成功
    println!("\n{}", tr("example-remote-etcd-polling"));
    match source.poll().await {
        Ok(value) => {
            println!("{}", tr("example-remote-etcd-loaded"));
            if value.is_map() {
                println!("  {}", tr("example-remote-etcd-value-map"));
            } else if value.is_null() {
                println!("  {}", tr("example-remote-etcd-value-null"));
            } else {
                println!("  {}", tr("example-remote-etcd-value-scalar"));
            }
        }
        Err(e) => {
            println!("{}", tr("example-remote-etcd-poll-failed-normal"));
            println!(
                "  {}",
                tr_args("example-remote-etcd-error", &[("error", e.to_string())])
            );
        }
    }

    Ok(())
}

/// 演示带用户名/密码认证的 etcd 连接
async fn demo_with_auth(endpoint: &str) -> anyhow::Result<()> {
    println!("\n{}\n", tr("example-remote-etcd-demo2"));

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
            println!("{}", tr("example-remote-etcd-with-auth-built"));
            println!(
                "  {}",
                tr_args(
                    "example-remote-etcd-source-id",
                    &[("id", s.source_id().as_str().to_string())]
                )
            );
            match s.poll().await {
                Ok(_) => println!("{}", tr("example-remote-etcd-loaded")),
                Err(e) => println!(
                    "{}",
                    tr_args(
                        "example-remote-etcd-poll-failed",
                        &[("error", e.to_string())]
                    )
                ),
            }
        }
        Err(e) => {
            println!(
                "{}",
                tr_args(
                    "example-remote-etcd-build-failed",
                    &[("error", e.to_string())]
                )
            );
        }
    }

    println!("\n  {}", tr("example-remote-etcd-credentials-note"));

    Ok(())
}

/// 演示 TLS 配置与多端点集群
async fn demo_tls_and_endpoints() -> anyhow::Result<()> {
    println!("\n{}\n", tr("example-remote-etcd-demo3"));

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
            println!("{}", tr("example-remote-etcd-with-tls-built"));
            println!(
                "  {}",
                tr_args(
                    "example-remote-etcd-source-id",
                    &[("id", s.source_id().as_str().to_string())]
                )
            );
            println!(
                "  {}",
                tr_args(
                    "example-remote-etcd-poll-interval",
                    &[("interval", format!("{:?}", s.poll_interval()))]
                )
            );
        }
        Err(e) => {
            println!("{}", tr("example-remote-etcd-build-failed-unreachable"));
            println!(
                "  {}",
                tr_args("example-remote-etcd-error", &[("error", e.to_string())])
            );
        }
    }

    println!("\n{}", tr("example-remote-etcd-tls-steps"));
    println!("  {}", tr("example-remote-etcd-tls-step-1"));
    println!("  {}", tr("example-remote-etcd-tls-step-2"));
    println!("  {}", tr("example-remote-etcd-tls-step-3"));
    println!("  {}", tr("example-remote-etcd-tls-step-4"));
    println!("  {}", tr("example-remote-etcd-tls-step-5"));

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("========================================");
    println!("  {}", tr("example-remote-etcd-title"));
    println!("========================================");

    let endpoint = std::env::var("ETCD_ENDPOINT").unwrap_or_else(|_| "127.0.0.1:2379".to_string());

    println!(
        "{}",
        tr_args(
            "example-remote-etcd-endpoint",
            &[("endpoint", endpoint.clone())]
        )
    );

    demo_basic_poll(&endpoint).await?;
    demo_with_auth(&endpoint).await?;
    demo_tls_and_endpoints().await?;

    println!("\n========================================");
    println!("  {}", tr("example-remote-etcd-completed"));
    println!("========================================");
    Ok(())
}
