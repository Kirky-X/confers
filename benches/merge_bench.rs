// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Merge benchmark for confers configuration library.

use criterion::{criterion_group, criterion_main, Criterion};
use confers::merger::MergeEngine;
use confers::types::{AnnotatedValue, ConfigValue};
use confers::SourceId;
use indexmap::IndexMap;
use std::sync::Arc;

/// Benchmark: ConfigValue String construction
fn bench_config_value_string(c: &mut Criterion) {
    c.bench_function("config_value_string", |b| {
        b.iter(|| confers::ConfigValue::String("test_value".to_string()));
    });
}

/// Benchmark: ConfigValue I64 construction
fn bench_config_value_i64(c: &mut Criterion) {
    c.bench_function("config_value_i64", |b| {
        b.iter(|| confers::ConfigValue::I64(42));
    });
}

/// Benchmark: ConfigValue Bool construction
fn bench_config_value_bool(c: &mut Criterion) {
    c.bench_function("config_value_bool", |b| {
        b.iter(|| confers::ConfigValue::Bool(true));
    });
}

/// Benchmark: AnnotatedValue construction
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

criterion_group!(
    benches,
    bench_config_value_string,
    bench_config_value_i64,
    bench_config_value_bool,
    bench_annotated_value,
    bench_merge_large_map,
    bench_merge_deep_nested,
    bench_merge_cow_hit,
    bench_merge_cow_miss
);
criterion_main!(benches);

// =============================================================================
// Real merge benchmarks using MergeEngine::merge()
// =============================================================================

/// Helper: create an AnnotatedValue wrapping a ConfigValue at a given path.
fn annotated(value: ConfigValue, path: &str) -> AnnotatedValue {
    AnnotatedValue::new(value, SourceId::new("bench"), path)
}

/// Helper: build a ConfigValue::Map from an iterator of (key, ConfigValue) pairs.
fn make_map(entries: Vec<(String, ConfigValue)>) -> ConfigValue {
    let map: IndexMap<Arc<str>, AnnotatedValue> = entries
        .into_iter()
        .map(|(k, v)| {
            let arc_key: Arc<str> = Arc::from(k.as_str());
            (arc_key, annotated(v, &k))
        })
        .collect();
    ConfigValue::Map(Arc::new(map))
}

/// Benchmark: merge two 100-field flat maps (Replace strategy).
fn bench_merge_large_map(c: &mut Criterion) {
    let engine = MergeEngine::new();

    let low_entries: Vec<(String, ConfigValue)> = (0..100)
        .map(|i| (format!("field_{:03}", i), ConfigValue::String(format!("low_{}", i))))
        .collect();
    let high_entries: Vec<(String, ConfigValue)> = (0..100)
        .map(|i| (format!("field_{:03}", i), ConfigValue::String(format!("high_{}", i))))
        .collect();

    let low = annotated(make_map(low_entries), "root");
    let high = annotated(make_map(high_entries), "root");

    c.bench_function("merge_large_map_100_fields", |b| {
        b.iter(|| engine.merge(&low, &high).unwrap())
    });
}

/// Benchmark: merge two 5-level deeply nested maps (1 key per level).
fn bench_merge_deep_nested(c: &mut Criterion) {
    let engine = MergeEngine::new();

    // Build nested maps from inside out: level5 -> level4 -> ... -> level1
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

/// Benchmark: COW fast path — both maps share the same Arc (identical pointers).
fn bench_merge_cow_hit(c: &mut Criterion) {
    let engine = MergeEngine::new();

    let shared_map: Arc<IndexMap<Arc<str>, AnnotatedValue>> = Arc::new(
        (0..50)
            .map(|i| {
                let key_str = format!("key_{:03}", i);
                let key: Arc<str> = Arc::from(key_str.as_str());
                let val = annotated(ConfigValue::I64(i as i64), &key_str);
                (key, val)
            })
            .collect(),
    );

    let low = annotated(ConfigValue::Map(Arc::clone(&shared_map)), "root");
    let high = annotated(ConfigValue::Map(Arc::clone(&shared_map)), "root");

    c.bench_function("merge_cow_hit_same_arc", |b| {
        b.iter(|| engine.merge(&low, &high).unwrap())
    });
}

/// Benchmark: COW slow path — different Arc with only 2 fields different out of 50.
fn bench_merge_cow_miss(c: &mut Criterion) {
    let engine = MergeEngine::new();

    let build_map = |prefix: &str| -> ConfigValue {
        let map: IndexMap<Arc<str>, AnnotatedValue> = (0..50)
            .map(|i| {
                let key_str = format!("key_{:03}", i);
                let key: Arc<str> = Arc::from(key_str.as_str());
                let val = annotated(ConfigValue::String(format!("{}_{}", prefix, i)), &key_str);
                (key, val)
            })
            .collect();
        ConfigValue::Map(Arc::new(map))
    };

    let low = annotated(build_map("low"), "root");
    let high = annotated(build_map("high"), "root");

    c.bench_function("merge_cow_miss_different_arc", |b| {
        b.iter(|| engine.merge(&low, &high).unwrap())
    });
}
