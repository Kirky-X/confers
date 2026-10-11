// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! ConfigBus 配置变更广播示例
//!
//! 本示例展示如何使用 confers 的 ConfigBus 实现配置变更事件广播：
//! - 使用 BusBuilder 创建 InMemoryBus
//! - 订阅配置变更事件流
//! - 发布配置变更事件
//! - 多订阅者接收事件

use confers::bus::{BusBuilder, ConfigBus, ConfigChangeEvent, InMemoryBus};
use confers::i18n::{tr, tr_args};
use futures::StreamExt;
use std::time::Duration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("========================================");
    println!("  {}", tr("example-config-bus-title"));
    println!("========================================\n");

    let bus: InMemoryBus = BusBuilder::new().capacity(256).build();
    println!("{}", tr("example-config-bus-bus-created"));

    // 先订阅再发布才能收到事件。
    let mut subscriber_a = bus.subscribe().await?;
    println!("{}", tr("example-config-bus-subscriber-a-subscribed"));

    let event1 = ConfigChangeEvent::new(
        "instance-1",
        "file_watcher",
        vec!["server.port".to_string(), "server.host".to_string()],
        "checksum-abc",
    );
    bus.publish(event1).await?;
    println!("{}", tr("example-config-bus-event-published-file-watcher"));

    if let Some(received) =
        tokio::time::timeout(Duration::from_secs(1), subscriber_a.next()).await?
    {
        println!(
            "{}",
            tr_args(
                "example-config-bus-subscriber-a-received",
                &[
                    ("instance", received.instance_id.clone()),
                    ("source", received.source.clone()),
                    ("checksum", received.checksum.clone()),
                ]
            )
        );
    }

    println!("\n{}", tr("example-config-bus-multi-subscriber"));
    let mut subscriber_b = bus.subscribe().await?;
    let mut subscriber_c = bus.subscribe().await?;
    println!(
        "{}",
        tr_args(
            "example-config-bus-subscribers-bc-subscribed",
            &[("count", bus.subscriber_count().to_string())]
        )
    );

    let event2 = ConfigChangeEvent::new(
        "instance-2",
        "env_override",
        vec!["database.url".to_string()],
        "checksum-def",
    );
    bus.publish(event2).await?;
    println!("{}", tr("example-config-bus-event-published-env-override"));

    // B 和 C 都应收到事件（广播）
    if let Some(r) = tokio::time::timeout(Duration::from_secs(1), subscriber_b.next()).await? {
        println!(
            "{}",
            tr_args(
                "example-config-bus-subscriber-b-received",
                &[
                    ("source", r.source.clone()),
                    ("keys", format!("{:?}", r.changed_keys)),
                ]
            )
        );
    }
    if let Some(r) = tokio::time::timeout(Duration::from_secs(1), subscriber_c.next()).await? {
        println!(
            "{}",
            tr_args(
                "example-config-bus-subscriber-c-received",
                &[
                    ("source", r.source.clone()),
                    ("keys", format!("{:?}", r.changed_keys)),
                ]
            )
        );
    }

    println!("\n{}", tr("example-config-bus-create-directly"));
    let bus2 = InMemoryBus::with_capacity(128);
    let mut sub = bus2.subscribe().await?;

    let event3 = ConfigChangeEvent::new(
        "instance-3",
        "api_push",
        vec!["feature.new_ui".to_string()],
        "checksum-ghi",
    );
    bus2.publish(event3).await?;

    if let Some(r) = tokio::time::timeout(Duration::from_secs(1), sub.next()).await? {
        println!(
            "{}",
            tr_args(
                "example-config-bus-received-via-in-memory-bus",
                &[
                    ("source", r.source.clone()),
                    ("checksum", r.checksum.clone()),
                ]
            )
        );
    }

    println!("\n========================================");
    println!("  {}", tr("example-config-bus-completed"));
    println!("========================================");
    Ok(())
}
