// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Unified merge benchmark for confers configuration library.
//!
//! Covers value construction, merge strategies, COW efficiency,
//! deep/nested merges, and incremental merge scenarios.
//! Consolidates the former merge_bench, cow_efficiency_bench, and
//! incremental_merge_bench into a single file.

use confers::merger::{MergeEngine, MergeStrategy};
use confers::types::{AnnotatedValue, ConfigValue};
use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;
use std::sync::Arc;

mod common;
use common::{
    annotated, av, create_large_map, create_nested_config, create_override_map, make_map,
};

// =============================================================================
// Value construction benchmarks
// =============================================================================

fn bench_config_value_string(c: &mut Criterion) {
    c.bench_function("config_value_string", |b| {
        b.iter(|| confers::ConfigValue::String("test_value".to_string()));
    });
}

fn bench_config_value_i64(c: &mut Criterion) {
    c.bench_function("config_value_i64", |b| {
        b.iter(|| confers::ConfigValue::I64(42));
    });
}

fn bench_config_value_bool(c: &mut Criterion) {
    c.bench_function("config_value_bool", |b| {
        b.iter(|| confers::ConfigValue::Bool(true));
    });
}

fn bench_annotated_value(c: &mut Criterion) {
    c.bench_function("annotated_value_construction", |b| {
        b.iter(|| {
            confers::AnnotatedValue::new(
                confers::ConfigValue::String("test".to_string()),
                confers::SourceId::new("default"),
                "test_path",
            )
        });
    });
}

// =============================================================================
// Basic merge benchmarks
// =============================================================================

/// Merge two 100-field flat maps (Replace strategy).
fn bench_merge_large_map(c: &mut Criterion) {
    let engine = MergeEngine::new();

    let low_entries: Vec<(String, ConfigValue)> = (0..100)
        .map(|i| {
            (
                format!("field_{:03}", i),
                ConfigValue::String(format!("low_{}", i)),
            )
        })
        .collect();
    let high_entries: Vec<(String, ConfigValue)> = (0..100)
        .map(|i| {
            (
                format!("field_{:03}", i),
                ConfigValue::String(format!("high_{}", i)),
            )
        })
        .collect();

    let low = annotated(make_map(low_entries), "root");
    let high = annotated(make_map(high_entries), "root");

    c.bench_function("merge_large_map_100_fields", |b| {
        b.iter(|| engine.merge(&low, &high).unwrap())
    });
}

/// Merge two 5-level deeply nested maps (1 key per level).
fn bench_merge_deep_nested(c: &mut Criterion) {
    let engine = MergeEngine::new();

    let mut low_val = ConfigValue::String("low_leaf".to_string());
    let mut high_val = ConfigValue::String("high_leaf".to_string());
    for depth in 0..5 {
        let key = format!("level_{}", depth);
        low_val = make_map(vec![(key.clone(), low_val)]);
        high_val = make_map(vec![(key, high_val)]);
    }

    let low = annotated(low_val, "root");
    let high = annotated(high_val, "root");

    c.bench_function("merge_deep_nested_5_levels", |b| {
        b.iter(|| engine.merge(&low, &high).unwrap())
    });
}

// =============================================================================
// COW efficiency benchmarks
// =============================================================================

/// COW fast path — merging identical values (no modification).
fn bench_cow_no_modification(c: &mut Criterion) {
    let engine = MergeEngine::new();
    let large = av(create_large_map(1000, "value"), "root");

    c.bench_function("cow_no_modification_1000", |b| {
        b.iter(|| engine.merge(black_box(&large), black_box(&large)));
    });
}

/// Single key modification out of 1000.
fn bench_cow_single_modification(c: &mut Criterion) {
    let engine = MergeEngine::new();
    let large = av(create_large_map(1000, "value"), "root");

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

/// Ten key modifications out of 1000.
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

/// No overlap between maps (all new keys).
fn bench_cow_no_overlap(c: &mut Criterion) {
    let engine = MergeEngine::new();

    let map_a = create_large_map(500, "value");
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

/// COW identity — merging with itself returns the same Arc.
fn bench_cow_identity_check(c: &mut Criterion) {
    let engine = MergeEngine::new();
    let large = av(create_large_map(1000, "value"), "root");

    c.bench_function("cow_identity_check", |b| {
        b.iter(|| {
            let result = engine.merge(&large, &large).unwrap();
            black_box(result);
        });
    });
}

/// Deep merge with nested structures (depth 3, 10 children per level).
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

// =============================================================================
// Strategy comparison benchmarks
// =============================================================================

fn bench_merge_strategies(c: &mut Criterion) {
    let mut group = c.benchmark_group("merge_strategies");

    let base = create_nested_config(3, 50, "base");
    let override_val = create_nested_config(3, 50, "override");

    group.bench_function("replace", |b| {
        let engine = MergeEngine::new().with_default_strategy(MergeStrategy::Replace);
        b.iter(|| engine.merge(black_box(&base), black_box(&override_val)));
    });

    group.bench_function("deep_merge", |b| {
        let engine = MergeEngine::new().with_default_strategy(MergeStrategy::DeepMerge);
        b.iter(|| engine.merge(black_box(&base), black_box(&override_val)));
    });

    group.bench_function("join", |b| {
        let engine =
            MergeEngine::new().with_default_strategy(MergeStrategy::Join { separator: "," });
        b.iter(|| engine.merge(black_box(&base), black_box(&override_val)));
    });

    group.finish();
}

/// Replace strategy with a small override (no COW benefit).
fn bench_replace_strategy(c: &mut Criterion) {
    let engine = MergeEngine::new().with_default_strategy(MergeStrategy::Replace);
    let large = av(create_large_map(1000, "value"), "root");
    let small = av(create_override_map(1), "root");

    c.bench_function("replace_strategy_1000", |b| {
        b.iter(|| engine.merge(black_box(&large), black_box(&small)));
    });
}

// =============================================================================
// Parameterized merge scale benchmarks
// =============================================================================

fn bench_merge_shallow(c: &mut Criterion) {
    let mut group = c.benchmark_group("merge_shallow");

    for size in [10, 100, 1000] {
        group.bench_with_input(format!("size_{}", size), &size, |b, &size| {
            let base = create_nested_config(1, size, "base");
            let override_val = create_nested_config(1, size, "override");
            let engine = MergeEngine::new().with_default_strategy(MergeStrategy::DeepMerge);

            b.iter(|| engine.merge(black_box(&base), black_box(&override_val)));
        });
    }

    group.finish();
}

fn bench_merge_deep(c: &mut Criterion) {
    let mut group = c.benchmark_group("merge_deep");

    for depth in [2, 4, 6] {
        group.bench_with_input(format!("depth_{}", depth), &depth, |b, &depth| {
            let base = create_nested_config(depth, 10, "base");
            let override_val = create_nested_config(depth, 10, "override");
            let engine = MergeEngine::new().with_default_strategy(MergeStrategy::DeepMerge);

            b.iter(|| engine.merge(black_box(&base), black_box(&override_val)));
        });
    }

    group.finish();
}

// =============================================================================
// Incremental merge benchmarks
// =============================================================================

fn bench_incremental_merge(c: &mut Criterion) {
    let mut group = c.benchmark_group("incremental_merge");

    group.bench_function("single_key_update", |b| {
        let base = create_nested_config(4, 20, "base");
        let mut override_map = indexmap::IndexMap::new();
        override_map.insert(
            Arc::from("key_0"),
            AnnotatedValue::new(
                ConfigValue::String("updated".to_string()),
                confers::SourceId::new("bench"),
                "override.key_0",
            ),
        );
        let override_val = AnnotatedValue::new(
            ConfigValue::Map(Arc::new(override_map)),
            confers::SourceId::new("bench"),
            "override",
        );
        let engine = MergeEngine::new().with_default_strategy(MergeStrategy::DeepMerge);

        b.iter(|| engine.merge(black_box(&base), black_box(&override_val)));
    });

    group.bench_function("batch_update_10", |b| {
        let base = create_nested_config(4, 20, "base");
        let mut override_map = indexmap::IndexMap::new();
        for i in 0..10 {
            override_map.insert(
                Arc::from(format!("key_{}", i)),
                AnnotatedValue::new(
                    ConfigValue::String("updated".to_string()),
                    confers::SourceId::new("bench"),
                    format!("override.key_{}", i),
                ),
            );
        }
        let override_val = AnnotatedValue::new(
            ConfigValue::Map(Arc::new(override_map)),
            confers::SourceId::new("bench"),
            "override",
        );
        let engine = MergeEngine::new().with_default_strategy(MergeStrategy::DeepMerge);

        b.iter(|| engine.merge(black_box(&base), black_box(&override_val)));
    });

    group.finish();
}

criterion_group!(
    benches,
    // Value construction
    bench_config_value_string,
    bench_config_value_i64,
    bench_config_value_bool,
    bench_annotated_value,
    // Basic merge
    bench_merge_large_map,
    bench_merge_deep_nested,
    // COW efficiency
    bench_cow_no_modification,
    bench_cow_single_modification,
    bench_cow_ten_modifications,
    bench_cow_no_overlap,
    bench_cow_identity_check,
    bench_cow_deep_merge,
    // Strategy comparison
    bench_merge_strategies,
    bench_replace_strategy,
    // Parameterized scale
    bench_merge_shallow,
    bench_merge_deep,
    // Incremental
    bench_incremental_merge,
);
criterion_main!(benches);
