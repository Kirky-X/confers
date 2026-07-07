// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! ConfigBus 配置变更广播示例
//!
//! 本示例展示如何使用 confers 的 ConfigBus 实现配置变更事件广播：
//! - 使用 BusBuilder 创建 InMemoryBus
//! - 订阅配置变更事件流
//! - 发布配置变更事件
//! - 多订阅者接收事件

use confers::bus::{BusBuilder, ConfigBus, ConfigChangeEvent, InMemoryBus};
use futures::StreamExt;
use std::time::Duration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("========================================");
    println!("  ConfigBus 配置变更广播示例");
    println!("========================================\n");

    // 1. 使用 BusBuilder 创建 InMemoryBus
    let bus: InMemoryBus = BusBuilder::new().capacity(256).build();
    println!("✓ InMemoryBus 已创建（容量 256）");

    // 2. 订阅配置变更事件（先订阅再发布才能收到事件）
    let mut subscriber_a = bus.subscribe().await?;
    println!("✓ 订阅者 A 已订阅");

    // 3. 发布第一个配置变更事件
    let event1 = ConfigChangeEvent::new(
        "instance-1",
        "file_watcher",
        vec!["server.port".to_string(), "server.host".to_string()],
        "checksum-abc",
    );
    bus.publish(event1).await?;
    println!("✓ 事件已发布: source=file_watcher, keys=[server.port, server.host]");

    // 4. 订阅者 A 接收事件
    if let Some(received) =
        tokio::time::timeout(Duration::from_secs(1), subscriber_a.next()).await?
    {
        println!(
            "✓ 订阅者 A 收到事件: instance={}, source={}, checksum={}",
            received.instance_id, received.source, received.checksum
        );
    }

    // 5. 演示多订阅者广播
    println!("\n--- 演示多订阅者广播 ---");
    let mut subscriber_b = bus.subscribe().await?;
    let mut subscriber_c = bus.subscribe().await?;
    println!(
        "✓ 订阅者 B、C 已订阅（当前订阅者数: {}）",
        bus.subscriber_count()
    );

    let event2 = ConfigChangeEvent::new(
        "instance-2",
        "env_override",
        vec!["database.url".to_string()],
        "checksum-def",
    );
    bus.publish(event2).await?;
    println!("✓ 事件已发布: source=env_override");

    // B 和 C 都应收到事件（广播）
    if let Some(r) = tokio::time::timeout(Duration::from_secs(1), subscriber_b.next()).await? {
        println!(
            "✓ 订阅者 B 收到: source={}, keys={:?}",
            r.source, r.changed_keys
        );
    }
    if let Some(r) = tokio::time::timeout(Duration::from_secs(1), subscriber_c.next()).await? {
        println!(
            "✓ 订阅者 C 收到: source={}, keys={:?}",
            r.source, r.changed_keys
        );
    }

    // 6. 演示直接创建 InMemoryBus 并使用 ConfigBus trait 方法
    println!("\n--- 演示直接创建 InMemoryBus ---");
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
            "✓ 通过 InMemoryBus 收到: source={}, checksum={}",
            r.source, r.checksum
        );
    }

    println!("\n========================================");
    println!("  示例运行完成");
    println!("========================================");
    Ok(())
}
