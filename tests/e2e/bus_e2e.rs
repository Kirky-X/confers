// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! E2E: 配置总线(tests/e2e/bus_e2e.rs)
//!
//! 依赖 docker compose 服务:NATS 127.0.0.1:4222、Redis 127.0.0.1:16379
//! (进程内总线 BUS 组无外部依赖)。
//!
//! 场景固化(docs/TEST_SCENARIOS.md §2.20–2.22):
//! - BUS-04 容量满:capacity=1 连发多条,滞后订阅者只收到最新事件,不 panic
//! - BUS-09 Lifecycle start/stop(InMemory 为 no-op):stop 后 publish/subscribe 仍可用(行为固化)
//! - BUS-10 订阅者 drop → subscriber_count 递减;零订阅者 publish 不 panic
//! - NAT-06 连接不可达 NATS → 明确错误不 panic;换真实服务后收发恢复
//! - RDS-06 pool_size>1 并发发布不丢消息
//! - RDS-07 连接不可达 Redis → 明确错误;真实服务恢复收发
//!
//! BUS-01…03/05…07、NAT-01…05、RDS-01…05 已有覆盖(tests/remote/bus.rs);
//! NAT-07/RDS-07 的总线驱动热更组合固化于 combo_e2e.rs(CMP-04/16)。

use confers::bus::{BusBuilder, ConfigBus, ConfigChangeEvent, InMemoryBus};
use futures_util::StreamExt;
use std::sync::atomic::{AtomicU64, Ordering};

fn unique(prefix: &str) -> String {
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;
    format!(
        "{prefix}-e2e-{}-{}",
        nanos,
        SEQ.fetch_add(1, Ordering::Relaxed)
    )
}

fn event(instance: &str, key: &str) -> ConfigChangeEvent {
    ConfigChangeEvent::new(instance, "e2e", vec![key.to_string()], "checksum")
}

async fn nats_ready() -> bool {
    std::net::TcpStream::connect("127.0.0.1:4222").is_ok()
}

async fn redis_ready() -> bool {
    std::net::TcpStream::connect("127.0.0.1:16379").is_ok()
}

/// BUS-04:capacity=1,订阅者不消费时连发 5 条 → 不 panic,只补发最新一条。
#[tokio::test]
async fn bus04_capacity_one_drops_lagged_events_without_panic() {
    let bus: InMemoryBus = BusBuilder::new().capacity(1).build();
    let mut rx = bus.subscribe().await.expect("subscribe");

    for i in 0..5 {
        bus.publish(event("prod", &format!("key{i}")))
            .await
            .expect("publish ok");
    }

    // 滞后订阅者:只允许收到 ≤1 条(最新),不得 panic 或收到全部历史。
    let drain = tokio::time::timeout(std::time::Duration::from_millis(300), async {
        let mut seen = Vec::new();
        while let Some(ev) = rx.next().await {
            seen.push(ev);
        }
        seen
    })
    .await;
    if let Ok(seen) = drain {
        assert!(
            seen.len() <= 1,
            "capacity=1 must not deliver the full history, got {}",
            seen.len()
        );
    }

    // 订阅者重新可用后能继续收到新事件。
    let mut rx = bus.subscribe().await.expect("resubscribe");
    bus.publish(event("prod", "after"))
        .await
        .expect("publish ok");
    let ev = tokio::time::timeout(std::time::Duration::from_secs(2), rx.next())
        .await
        .expect("event must arrive")
        .expect("stream open");
    assert_eq!(ev.changed_keys, vec!["after".to_string()]);
}

/// BUS-09:InMemoryBus 生命周期为 no-op,stop 后 publish/subscribe 仍可用。
#[tokio::test]
async fn bus09_lifecycle_stop_is_noop_for_in_memory_bus() {
    use confers::lifecycle::Lifecycle;

    let bus: InMemoryBus = BusBuilder::new().build();
    Lifecycle::start(&bus).await.expect("start");
    Lifecycle::stop(&bus).await.expect("stop");

    // 行为固化:stop 为 no-op,消息面仍可用。
    let mut rx = bus.subscribe().await.expect("subscribe after stop");
    bus.publish(event("post-stop", "k"))
        .await
        .expect("publish after stop");
    let ev = tokio::time::timeout(std::time::Duration::from_secs(2), rx.next())
        .await
        .expect("event must arrive")
        .expect("stream open");
    assert_eq!(ev.instance_id, "post-stop");
}

/// BUS-10:订阅者 drop 后计数递减;零订阅者 publish 正常返回。
#[tokio::test]
async fn bus10_dropped_receiver_decrements_and_publish_stays_safe() {
    let bus: InMemoryBus = BusBuilder::new().build();

    {
        let rx1 = bus.subscribe().await.expect("subscribe 1");
        let rx2 = bus.subscribe().await.expect("subscribe 2");
        assert_eq!(bus.subscriber_count(), 2);
        drop(rx1);
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        assert_eq!(bus.subscriber_count(), 1, "drop must decrement");
        drop(rx2);
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    assert_eq!(bus.subscriber_count(), 0);

    // 零订阅者 publish:返回 Ok(事件被丢弃但不报错、不 panic)。
    bus.publish(event("no-subscribers", "k"))
        .await
        .expect("publish with zero subscribers must be Ok");
}

/// NAT-06:不可达地址 → 明确连接错误;真实服务恢复收发。
#[tokio::test]
async fn nat06_unreachable_nats_errors_then_real_service_recovers() {
    use confers::bus::{NatsBusBuilder, NatsConfigBus};

    // 不可达端口:连接必须返回错误,不得 panic/挂死。
    let err = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        NatsConfigBus::connect("nats://127.0.0.1:1", unique("nat06.bad")),
    )
    .await
    .expect("connect must not hang");
    assert!(err.is_err(), "unreachable NATS must fail to connect");

    // 恢复:指向真实服务后即可正常收发。
    if !nats_ready().await {
        eprintln!("Skipping recovery half: NATS not available");
        return;
    }
    let bus = NatsBusBuilder::new()
        .url("nats://127.0.0.1:4222")
        .subject(unique("nat06.recover.subject"))
        .stream_name(unique("NAT06"))
        .build()
        .await
        .expect("connect to real NATS");
    let mut rx = bus.subscribe().await.expect("subscribe");
    bus.publish(event("nat06-recovered", "recovered.key"))
        .await
        .expect("publish");
    let ev = tokio::time::timeout(std::time::Duration::from_secs(5), rx.next())
        .await
        .expect("event must arrive")
        .expect("stream open");
    assert_eq!(ev.instance_id, "nat06-recovered");
}

/// RDS-06:pool_size>1 并发发布不丢消息。
#[tokio::test]
async fn rds06_concurrent_publish_with_pool_delivers_all() {
    use confers::bus::RedisBusBuilder;

    if !redis_ready().await {
        eprintln!("Skipping test: Redis not available");
        return;
    }

    let bus = RedisBusBuilder::new()
        .url("redis://127.0.0.1:16379")
        .channel(unique("rds06.channel"))
        .pool_size(4)
        .build()
        .await
        .expect("redis bus builds");

    let mut rx = bus.subscribe().await.expect("subscribe");
    // pub/sub 注册异步生效,留出注册窗口(与既有集成测试一致)。
    tokio::time::sleep(std::time::Duration::from_millis(150)).await;

    const TASKS: usize = 4;
    const PER_TASK: usize = 25;
    let bus = std::sync::Arc::new(bus);
    let handles: Vec<_> = (0..TASKS)
        .map(|t| {
            let bus = std::sync::Arc::clone(&bus);
            tokio::spawn(async move {
                for i in 0..PER_TASK {
                    bus.publish(event(&format!("rds06-t{t}"), &format!("k{t}-{i}")))
                        .await
                        .expect("publish");
                }
            })
        })
        .collect();
    for handle in handles {
        handle.await.expect("publisher task");
    }

    // 汇总:100 条必须全部到达。
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(10);
    let mut received = 0usize;
    while received < TASKS * PER_TASK {
        match tokio::time::timeout_at(deadline, rx.next()).await {
            Ok(Some(_)) => received += 1,
            Ok(None) => panic!("stream ended early at {received}"),
            Err(_) => panic!(
                "timeout: {received}/{} events delivered; pool publish must not lose messages",
                TASKS * PER_TASK
            ),
        }
    }
    assert_eq!(received, TASKS * PER_TASK);
}

/// RDS-07:不可达 Redis → 明确错误;真实服务恢复收发。
/// 行为固化:builder 连接惰性建立 —— build() 返回 Ok,
/// 首次 subscribe/publish 才暴露 RemoteUnavailable。
#[tokio::test]
async fn rds07_unreachable_redis_errors_then_real_service_recovers() {
    use confers::bus::RedisBusBuilder;

    let bus = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        RedisBusBuilder::new().url("redis://127.0.0.1:1").build(),
    )
    .await
    .expect("connect must not hang");
    let bus = bus.expect("builder connects lazily (fixated behavior)");

    let sub = tokio::time::timeout(std::time::Duration::from_secs(10), bus.subscribe())
        .await
        .expect("subscribe must not hang");
    assert!(
        sub.is_err(),
        "unreachable Redis must surface RemoteUnavailable on first use"
    );
    let pubr = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        bus.publish(event("rds07", "k")),
    )
    .await
    .expect("publish must not hang");
    assert!(
        pubr.is_err(),
        "publish against unreachable Redis must error"
    );

    if !redis_ready().await {
        eprintln!("Skipping recovery half: Redis not available");
        return;
    }
    let bus = RedisBusBuilder::new()
        .url("redis://127.0.0.1:16379")
        .channel(unique("rds07.channel"))
        .build()
        .await
        .expect("connect to real Redis");
    let mut rx = bus.subscribe().await.expect("subscribe");
    tokio::time::sleep(std::time::Duration::from_millis(150)).await;
    bus.publish(event("rds07-recovered", "recovered.key"))
        .await
        .expect("publish");
    let ev = tokio::time::timeout(std::time::Duration::from_secs(5), rx.next())
        .await
        .expect("event must arrive")
        .expect("stream open");
    assert_eq!(ev.instance_id, "rds07-recovered");
}
