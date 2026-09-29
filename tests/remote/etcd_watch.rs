// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! etcd watch 公开接口的兼容性回归（接口冻结契约网）。
//!
//! 从 crate 外部消费者视角验证冻结契约：[`WatchEventSource`] 可被外部
//! 类型实现并驱动 [`EtcdWatcher`] 完整生命周期；[`EtcdWatchEvent`] /
//! [`EtcdWatchRetry`] 的公开形状与默认退避语义稳定。运行时行为的细粒度
//! 覆盖在 `src/remote/etcd_watch.rs` 内部单元测试中，此处不重复。
//!
//! 纯 mock 联测，不依赖真实 etcd 实例。
//!
//! 单线程不变式：本文件依赖 `#[tokio::test]` 默认的 current_thread
//! runtime，`try_lock().unwrap()` 因此不会遇到 `WouldBlock`（锁竞争
//! 只可能来自阻塞线程）；若改为 multi_thread flavor 或引入后台线程，
//! 须先将 `try_lock` 换成容错获取。

#![cfg(feature = "etcd-watch")]

use std::pin::Pin;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::Duration;

use confers::async_trait;
use confers::remote::etcd_watch::{
    EtcdWatchCallback, EtcdWatchEvent, EtcdWatchRetry, EtcdWatcher, WatchEventSource, WatchItem,
};
use futures_util::Stream;

/// 外部消费者的 mock 传输：按连接尝试重放事件序列，末条序列无限重复；
/// 序列消费完后保持 pending（模拟健康的空闲连接）。
struct ConsumerSideMock {
    attempts: Mutex<Vec<Vec<WatchItem>>>,
    connections: AtomicI64,
}

#[async_trait]
impl WatchEventSource for ConsumerSideMock {
    async fn watch(
        &self,
    ) -> confers::error::ConfigResult<Pin<Box<dyn Stream<Item = WatchItem> + Send>>> {
        use futures_util::stream;
        self.connections.fetch_add(1, Ordering::SeqCst);

        let items: Vec<WatchItem> = {
            let mut q = self.attempts.try_lock().unwrap();
            if q.len() > 1 {
                q.remove(0)
            } else {
                q.first().cloned().unwrap_or_default()
            }
        };
        let mut iter = items.into_iter();
        Ok(Box::pin(stream::poll_fn(
            move |_cx: &mut Context<'_>| match iter.next() {
                Some(item) => Poll::Ready(Some(item)),
                None => Poll::Pending,
            },
        )))
    }
}

fn put(key: &str, value: &str, rev: i64) -> WatchItem {
    Ok(EtcdWatchEvent {
        key: key.to_string(),
        value: Some(value.to_string()),
        mod_revision: rev,
    })
}

fn deleted(key: &str, rev: i64) -> WatchItem {
    Ok(EtcdWatchEvent {
        key: key.to_string(),
        value: None,
        mod_revision: rev,
    })
}

type EventSink = Arc<Mutex<Vec<EtcdWatchEvent>>>;

fn sink_callback(
    sink: &EventSink,
    notify: tokio::sync::mpsc::UnboundedSender<EtcdWatchEvent>,
) -> EtcdWatchCallback {
    let sink = Arc::clone(sink);
    Arc::new(move |event: &EtcdWatchEvent| {
        sink.try_lock().unwrap().push(event.clone());
        let _ = notify.send(event.clone());
    })
}

/// 等待通道送达 `min_len` 个事件（事件到达即通知，无轮询延迟）；每事件
/// 独立超时兜底，超时 panic（暴露冻结契约回退）。
async fn wait_for(rx: &mut tokio::sync::mpsc::UnboundedReceiver<EtcdWatchEvent>, min_len: usize) {
    for _ in 0..min_len {
        tokio::time::timeout(Duration::from_secs(5), rx.recv())
            .await
            .expect("timed out waiting for a watch event")
            .expect("watch event channel closed unexpectedly");
    }
}

/// 外部实现的传输可驱动完整 watch 生命周期：事件按交付序穿透回调、
/// 断流后自动重连续流、`last_revision` 跨连接按 `fetch_max` 单调
/// （乱序 revision 不回退）。
#[tokio::test]
async fn external_mock_implementation_drives_full_lifecycle() {
    let mock = Arc::new(ConsumerSideMock {
        attempts: Mutex::new(vec![
            vec![
                put("cfg/a", "v1", 5),
                put("cfg/a", "v0", 3),
                Err("connection reset".to_string()),
            ],
            vec![put("cfg/b", "v2", 9)],
        ]),
        connections: AtomicI64::new(0),
    });
    let watcher = Arc::new(EtcdWatcher::new(
        Arc::clone(&mock) as Arc<dyn WatchEventSource>
    ));
    assert_eq!(watcher.last_revision(), 0, "首个事件前 revision 为 0");

    let sink: EventSink = Arc::new(Mutex::new(Vec::new()));
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let run_handle = tokio::spawn({
        let watcher = Arc::clone(&watcher);
        let sink = Arc::clone(&sink);
        async move { watcher.run(sink_callback(&sink, tx)).await }
    });
    wait_for(&mut rx, 3).await;
    run_handle.abort();

    let events = sink.try_lock().unwrap();
    assert_eq!(events.len(), 3, "两个连接交付的事件全部到达回调");
    assert_eq!(events[0].mod_revision, 5, "交付顺序保持 etcd commit order");
    assert_eq!(events[1].mod_revision, 3);
    assert_eq!(events[2].mod_revision, 9);
    assert_eq!(
        watcher.last_revision(),
        9,
        "last_revision 取已见最高 revision，乱序与重连均不回退"
    );
    assert!(
        mock.connections.load(Ordering::SeqCst) >= 2,
        "断流后必须重连"
    );
}

/// 删除事件以 `value: None` 穿透到消费者回调，且推进 revision——
/// 外部消费者可据此区分删除与更新。
#[tokio::test]
async fn deletion_events_surface_with_none_value() {
    let mock = Arc::new(ConsumerSideMock {
        attempts: Mutex::new(vec![vec![put("cfg/x", "v", 4), deleted("cfg/x", 5)]]),
        connections: AtomicI64::new(0),
    });
    let watcher = Arc::new(EtcdWatcher::new(mock));
    let sink: EventSink = Arc::new(Mutex::new(Vec::new()));
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

    let run_handle = tokio::spawn({
        let watcher = Arc::clone(&watcher);
        let sink = Arc::clone(&sink);
        async move { watcher.run(sink_callback(&sink, tx)).await }
    });
    wait_for(&mut rx, 2).await;
    run_handle.abort();

    let events = sink.try_lock().unwrap();
    assert_eq!(events.len(), 2);
    assert_eq!(events[1].key, "cfg/x", "删除事件携带完整 key");
    assert_eq!(events[1].value, None, "删除以 value=None 交付");
    assert_eq!(events[1].mod_revision, 5);
    assert_eq!(watcher.last_revision(), 5, "删除事件同样推进 revision");
}

/// 冻结的类型形状与默认语义：`EtcdWatchEvent` 公开可构造且 `PartialEq`
/// 按字段全等；`EtcdWatchRetry::default()` 冻结为 500ms 基础退避、30s
/// 封顶，且默认策略按 2 倍指数增长（消费者依赖默认退避节奏）。
///
/// 毒事件解码契约（key/value 无效 UTF-8 → `U+FFFD` lossy 交付、revision
/// 照常消费）不在本文件断言：gRPC mapper 无法在 mock 层驱动，该语义由
/// `src/remote/etcd_watch.rs` 内部测试 `poison_bytes_are_delivered_lossily`
/// 以纯函数级单测钉住。
#[test]
fn frozen_public_shapes_and_default_retry_semantics() {
    let event = EtcdWatchEvent {
        key: "k".to_string(),
        value: Some("v".to_string()),
        mod_revision: 1,
    };
    assert_eq!(
        event,
        EtcdWatchEvent {
            key: "k".to_string(),
            value: Some("v".to_string()),
            mod_revision: 1,
        },
        "同字段事件相等（PartialEq 契约）"
    );
    assert_ne!(
        event,
        EtcdWatchEvent {
            key: "k".to_string(),
            value: None,
            mod_revision: 1,
        },
        "value 差异必须可区分（更新 vs 删除）"
    );

    let default_retry = EtcdWatchRetry::default();
    assert_eq!(default_retry.base, Duration::from_millis(500));
    assert_eq!(default_retry.max, Duration::from_secs(30));
    assert_eq!(
        default_retry.delay_for(1),
        Duration::from_millis(500),
        "首次重连延迟 = base"
    );
    assert_eq!(
        default_retry.delay_for(2),
        Duration::from_secs(1),
        "默认策略按 2 倍指数增长"
    );
    assert_eq!(
        default_retry.delay_for(17),
        Duration::from_secs(30),
        "封顶于 max"
    );
}
