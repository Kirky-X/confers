//! Incremental merge benchmark.
//!
//! Measures performance of incremental configuration merging.

use confers::merger::{MergeEngine, MergeStrategy};
use confers::value::{AnnotatedValue, ConfigValue};
use confers::SourceId;
use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;
use std::sync::Arc;

fn create_nested_config(depth: usize, width: usize, path: &str) -> AnnotatedValue {
    if depth == 0 {
        return AnnotatedValue::new(
            ConfigValue::String("value".to_string()),
            SourceId::new("bench"),
            path,
        );
    }

    let mut map = indexmap::IndexMap::new();
    for i in 0..width {
        let key = format!("key_{}", i);
        let child_path = format!("{}.{}", path, key);
        let value = create_nested_config(depth - 1, width, &child_path);
        map.insert(Arc::from(key), value);
    }
    AnnotatedValue::new(
        ConfigValue::Map(Arc::new(map)),
        SourceId::new("bench"),
        path,
    )
}

fn bench_merge_shallow(c: &mut Criterion) {
    let mut group = c.benchmark_group("merge_shallow");
    group.warm_up_time(std::time::Duration::from_secs(2));

    for size in [10, 100, 1000].iter() {
        group.bench_with_input(format!("size_{}", size), size, |b, &size| {
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
    group.warm_up_time(std::time::Duration::from_secs(2));

    for depth in [2, 4, 6].iter() {
        group.bench_with_input(format!("depth_{}", depth), depth, |b, &depth| {
            let base = create_nested_config(depth, 10, "base");
            let override_val = create_nested_config(depth, 10, "override");
            let engine = MergeEngine::new().with_default_strategy(MergeStrategy::DeepMerge);

            b.iter(|| engine.merge(black_box(&base), black_box(&override_val)));
        });
    }

    group.finish();
}

fn bench_merge_strategies(c: &mut Criterion) {
    let mut group = c.benchmark_group("merge_strategies");
    group.warm_up_time(std::time::Duration::from_secs(2));

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

fn bench_incremental_merge(c: &mut Criterion) {
    let mut group = c.benchmark_group("incremental_merge");
    group.warm_up_time(std::time::Duration::from_secs(2));

    group.bench_function("single_key_update", |b| {
        let base = create_nested_config(4, 20, "base");
        let mut override_map = indexmap::IndexMap::new();
        override_map.insert(
            Arc::from("key_0"),
            AnnotatedValue::new(
                ConfigValue::String("updated".to_string()),
                SourceId::new("bench"),
                "override.key_0",
            ),
        );
        let override_val = AnnotatedValue::new(
            ConfigValue::Map(Arc::new(override_map)),
            SourceId::new("bench"),
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
                    SourceId::new("bench"),
                    format!("override.key_{}", i),
                ),
            );
        }
        let override_val = AnnotatedValue::new(
            ConfigValue::Map(Arc::new(override_map)),
            SourceId::new("bench"),
            "override",
        );
        let engine = MergeEngine::new().with_default_strategy(MergeStrategy::DeepMerge);

        b.iter(|| engine.merge(black_box(&base), black_box(&override_val)));
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_merge_shallow,
    bench_merge_deep,
    bench_merge_strategies,
    bench_incremental_merge,
);

criterion_main!(benches);
