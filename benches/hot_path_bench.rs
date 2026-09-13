// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

use confers::{
    AnnotatedValue, ConfigReader, ConfigValue, ConfigWriter, SharedValueReader, SourceId,
    new_in_memory,
};
use criterion::{Criterion, criterion_group, criterion_main};
use tokio::runtime::Runtime;

fn bench_hot_path_get(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let config = new_in_memory();

    rt.block_on(async {
        for i in 0..100 {
            let value = AnnotatedValue::new(
                ConfigValue::string(format!("value_{i}")),
                SourceId::default(),
                format!("key_{i}"),
            );
            config.set(&format!("key_{i}"), value).await.unwrap();
        }
    });

    // Keys are built outside the measured loop: the benchmark targets the
    // lookup cost, not per-iteration string formatting.
    let keys: Vec<String> = (0..100).map(|i| format!("key_{i}")).collect();

    c.bench_function("hot_path_get_100_keys", |b| {
        b.to_async(&rt).iter(|| async {
            for key in &keys {
                let _ = config.get_string(key).await;
            }
        })
    });
}

/// Zero-copy comparison: deep-clone `get_raw` vs the shared `Arc`
/// handle on a large (10 KiB) string value.
fn bench_zero_copy_large_value(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let config = new_in_memory();
    let large_value = "x".repeat(10 * 1024);

    rt.block_on(async {
        config
            .set(
                "large.value",
                AnnotatedValue::new(
                    ConfigValue::string(large_value.clone()),
                    SourceId::default(),
                    "large.value",
                ),
            )
            .await
            .unwrap();
    });

    let mut group = c.benchmark_group("hot_path_zero_copy");

    group.bench_function("deep_clone_get_raw_10kib", |b| {
        b.to_async(&rt).iter(|| async {
            let raw = config.get_raw("large.value").await.unwrap().unwrap();
            std::hint::black_box(raw.as_str().map(str::len))
        })
    });

    group.bench_function("shared_arc_get_shared_10kib", |b| {
        b.to_async(&rt).iter(|| async {
            let shared = config.get_shared("large.value").await.unwrap().unwrap();
            std::hint::black_box(shared.as_str().map(str::len))
        })
    });

    group.finish();
}

criterion_group!(benches, bench_hot_path_get, bench_zero_copy_large_value);
criterion_main!(benches);
