// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! etcd watch 事件分发基准：`EtcdWatcher::run` 的每事件热路径。
//!
//! 覆盖转正冻结契约的关键成本面：mock 传输满速交付一批事件时，watcher 侧
//! 的 `fetch_max` revision 追踪 + `Arc<dyn Fn>` 同步回调分发的单位事件开销
//! （不含真实 gRPC 传输；真实路径另有 protobuf 解码与每事件一次的
//! key/value 借用→lossy 转换分配，mock 直接构造 `EtcdWatchEvent`，该部分
//! 不在基线口径内）。基线数字记录在 `docs/PERFORMANCE.md`。
//!
//! 口径说明：事件数据放在 `Arc<Vec<_>>` 中跨迭代共享，计时区间内不含批次
//! 构建成本；每事件交付时的 String clone 属于交付本身的成本，保留在计时
//! 区间内。watcher/source/spawn 的固定构建开销按整批 1000 事件分摊。

use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Poll};
use std::time::Duration;

use confers::async_trait;
use confers::remote::etcd_watch::{
    EtcdWatchCallback, EtcdWatchEvent, EtcdWatcher, WatchEventSource, WatchItem,
};
use criterion::{Criterion, criterion_group, criterion_main};
use futures_util::Stream;

const BATCH: usize = 1000;

/// 单流交付一批事件后转入空闲（Pending）：批内满速分发，批后挂起——
/// watcher 由此获得真实的 await 让出点，abort 能立即生效。
struct BatchSource {
    /// 事件批次跨迭代共享；流内逐事件 clone 属于交付成本而非构建成本。
    events: Arc<Vec<EtcdWatchEvent>>,
}

#[async_trait]
impl WatchEventSource for BatchSource {
    async fn watch(
        &self,
    ) -> confers::error::ConfigResult<Pin<Box<dyn Stream<Item = WatchItem> + Send>>> {
        use futures_util::stream;
        let events = Arc::clone(&self.events);
        let mut index = 0usize;
        Ok(Box::pin(stream::poll_fn(move |_cx: &mut Context<'_>| {
            if index < events.len() {
                let item = Ok(events[index].clone());
                index += 1;
                Poll::Ready(Some(item))
            } else {
                Poll::Pending
            }
        })))
    }
}

fn bench_etcd_watch_dispatch(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    let mut group = c.benchmark_group("etcd_watch");

    let events = Arc::new(
        (1..=BATCH as i64)
            .map(|rev| EtcdWatchEvent {
                key: format!("bench/key-{rev}"),
                value: Some(format!("value-{rev}")),
                mod_revision: rev,
            })
            .collect::<Vec<_>>(),
    );

    // 每次迭代交付完整一批（1000 个事件）：空回调 + revision 追踪，
    // 即消费者看到的最小每事件成本。
    group.bench_function("dispatch_1000_events_empty_callback", |b| {
        b.iter(|| {
            rt.block_on(async {
                let source = Arc::new(BatchSource {
                    events: Arc::clone(&events),
                });
                let watcher = Arc::new(EtcdWatcher::new(source));
                let delivered = Arc::new(AtomicUsize::new(0));
                let callback: EtcdWatchCallback = {
                    let delivered = Arc::clone(&delivered);
                    Arc::new(move |_event| {
                        delivered.fetch_add(1, Ordering::Relaxed);
                    })
                };
                let handle = tokio::spawn(async move { watcher.run(callback).await });
                let wait = async {
                    while delivered.load(Ordering::Relaxed) < BATCH {
                        tokio::task::yield_now().await;
                    }
                };
                // 超时兜底：分发路径若回归到吞事件，benchmark 失败于计数
                // 断言而非挂死整个 bench 运行。
                let reached = tokio::time::timeout(Duration::from_secs(30), wait).await;
                assert!(
                    reached.is_ok(),
                    "watcher failed to deliver {BATCH} events within 30s"
                );
                handle.abort();
            })
        })
    });

    group.finish();
}

criterion_group!(benches, bench_etcd_watch_dispatch);
criterion_main!(benches);
