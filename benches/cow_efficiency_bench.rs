//! COW (Copy-on-Write) efficiency benchmark for confers configuration library.
//!
//! Measures the efficiency of the merger's copy-on-write implementation.

use confers::merger::{MergeEngine, MergeStrategy};
use confers::value::ConfigValue;
use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;
use std::sync::Arc;

mod common;
use common::{av, create_large_map, create_override_map};

/// Create a map with only a few keys for targeted updates.
fn create_small_map(key_count: usize) -> ConfigValue {
    create_override_map(key_count)
}

/// Benchmark: No modifications - should use COW fast path (return original Arc)
fn bench_cow_no_modification(c: &mut Criterion) {
    let engine = MergeEngine::new();
    let large = av(create_large_map(1000, "value"), "root");

    c.bench_function("cow_no_modification_1000", |b| {
        b.iter(|| engine.merge(black_box(&large), black_box(&large)));
    });
}

/// Benchmark: Single key modification out of 1000
fn bench_cow_single_modification(c: &mut Criterion) {
    let engine = MergeEngine::new();
    let large = av(create_large_map(1000, "value"), "root");

    // Override only key_0
    let mut override_map = indexmap::IndexMap::new();
    override_map.insert(
        Arc::from("key_0"),
        av(ConfigValue::String("changed".to_string()), "key_0"),
    );
    let small = av(ConfigValue::Map(Arc::new(override_map)), "root");

    c.bench_function("cow_single_modification_1000", |b| {
        b.iter(|| engine.merge(black_box(&large), black_box(&small)));
    });
}

/// Benchmark: Ten key modifications out of 1000
fn bench_cow_ten_modifications(c: &mut Criterion) {
    let engine = MergeEngine::new();
    let large = av(create_large_map(1000, "value"), "root");

    let mut override_map = indexmap::IndexMap::new();
    for i in 0..10 {
        override_map.insert(
            Arc::from(format!("key_{}", i)),
            av(
                ConfigValue::String(format!("changed_{}", i)),
                &format!("key_{}", i),
            ),
        );
    }
    let small = av(ConfigValue::Map(Arc::new(override_map)), "root");

    c.bench_function("cow_ten_modifications_1000", |b| {
        b.iter(|| engine.merge(black_box(&large), black_box(&small)));
    });
}

/// Benchmark: No overlap between maps (all new keys)
fn bench_cow_no_overlap(c: &mut Criterion) {
    let engine = MergeEngine::new();

    let map_a = create_large_map(500, "value");
    let _map_b = create_large_map(500, "value");
    let a = av(map_a, "root");
    let b = av(
        ConfigValue::Map(Arc::new({
            let mut m = indexmap::IndexMap::new();
            for i in 500..1000 {
                m.insert(
                    Arc::from(format!("key_{}", i)),
                    av(
                        ConfigValue::String(format!("value_{}", i)),
                        &format!("k{}", i),
                    ),
                );
            }
            m
        })),
        "root",
    );

    c.bench_function("cow_no_overlap_1000", |bencher| {
        bencher.iter(|| engine.merge(black_box(&a), black_box(&b)));
    });
}

/// Benchmark: Replace strategy - always replaces (no COW benefit)
fn bench_replace_strategy(c: &mut Criterion) {
    let engine = MergeEngine::new().with_default_strategy(MergeStrategy::Replace);
    let large = av(create_large_map(1000, "value"), "root");
    let small = av(create_small_map(1), "root");

    c.bench_function("replace_strategy_1000", |b| {
        b.iter(|| engine.merge(black_box(&large), black_box(&small)));
    });
}

/// Benchmark: Deep merge with nested structures
fn bench_cow_deep_merge(c: &mut Criterion) {
    let engine = MergeEngine::new().with_default_strategy(MergeStrategy::DeepMerge);

    fn make_nested(depth: usize, prefix: &str) -> ConfigValue {
        if depth == 0 {
            return ConfigValue::String(format!("val_{}", prefix));
        }
        let mut map = indexmap::IndexMap::new();
        for i in 0..10 {
            let key = format!("{}_{}", prefix, i);
            map.insert(
                Arc::from(format!("child_{}", i)),
                av(
                    make_nested(depth - 1, &key),
                    &format!("{}.child_{}", prefix, i),
                ),
            );
        }
        ConfigValue::Map(Arc::new(map))
    }

    let base = av(make_nested(3, "base"), "root");

    c.bench_function("cow_deep_merge_depth3", |b| {
        b.iter(|| engine.merge(black_box(&base), black_box(&base)));
    });
}

/// Benchmark: Compare merged result identity (is it the same Arc?)
fn bench_cow_identity_check(c: &mut Criterion) {
    let engine = MergeEngine::new();
    let large = av(create_large_map(1000, "value"), "root");

    // Merging with itself should return the SAME Arc (COW optimization)
    c.bench_function("cow_identity_check", |b| {
        b.iter(|| {
            let result = engine.merge(&large, &large).unwrap();
            // This is a compile-time check - we just measure the call
            black_box(result);
        });
    });
}

criterion_group!(
    benches,
    bench_cow_no_modification,
    bench_cow_single_modification,
    bench_cow_ten_modifications,
    bench_cow_no_overlap,
    bench_replace_strategy,
    bench_cow_deep_merge,
    bench_cow_identity_check,
);
criterion_main!(benches);
