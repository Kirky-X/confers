// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Watch-callback benchmark: the unified change-stream round trip.
//!
//! Measures the end-to-end cost of the hot-reload notification path that
//! upper layers subscribe to: `ChangeStream::publish` (file / remote /
//! canary producers) through the `ConfigBus` transport to a delivered event
//! on the subscriber side. Baseline numbers live in
//! `docs/PERFORMANCE.md` (baseline gate, workspace-rc4-completion T110).

use confers::ChangeStream;
use confers::stream::{ChangeEvent, ChangeSource, InMemoryChangeStream};
use confers::types::ConfigValue;
use criterion::{Criterion, criterion_group, criterion_main};

fn bench_watch_callback(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    let mut group = c.benchmark_group("watch_callback");

    // Per-publish cost incl. subscriber delivery (single subscriber).
    group.bench_function("change_stream_roundtrip_1_sub", |b| {
        b.iter(|| {
            rt.block_on(async {
                let stream = InMemoryChangeStream::new();
                let mut rx = stream.subscribe().await.expect("subscribe");
                stream
                    .publish(ChangeEvent::new(
                        "bench.key",
                        None,
                        Some(ConfigValue::string("value")),
                        ChangeSource::File,
                    ))
                    .await
                    .expect("publish");
                let event = futures_util::StreamExt::next(&mut rx)
                    .await
                    .expect("delivered");
                assert_eq!(event.key, "bench.key");
                event.version
            })
        })
    });

    // Delivery fan-out cost with 8 concurrent subscribers.
    group.bench_function("change_stream_roundtrip_8_sub", |b| {
        b.iter(|| {
            rt.block_on(async {
                let stream = InMemoryChangeStream::new();
                let mut receivers = Vec::new();
                for _ in 0..8 {
                    receivers.push(stream.subscribe().await.expect("subscribe"));
                }
                stream
                    .publish(ChangeEvent::new(
                        "bench.key",
                        None,
                        Some(ConfigValue::string("value")),
                        ChangeSource::Remote("bench".to_string()),
                    ))
                    .await
                    .expect("publish");
                for rx in receivers.iter_mut() {
                    futures_util::StreamExt::next(rx)
                        .await
                        .expect("delivered");
                }
            })
        })
    });

    // Payload retention + ack bookkeeping (pending store churn).
    group.bench_function("change_stream_publish_ack", |b| {
        b.iter(|| {
            rt.block_on(async {
                let stream = InMemoryChangeStream::new();
                for _ in 0..16 {
                    stream
                        .publish(ChangeEvent::new(
                            "bench.ack",
                            None,
                            Some(ConfigValue::string("value")),
                            ChangeSource::Bus,
                        ))
                        .await
                        .expect("publish");
                }
                stream.pending_count()
            })
        })
    });

    group.finish();
}

criterion_group!(benches, bench_watch_callback);
criterion_main!(benches);
