// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Value path traversal benchmark for confers configuration library.
//!
//! Measures `all_paths()` performance for various tree shapes and sizes.

use confers::types::{AnnotatedValue, ConfigValue};
use confers::SourceId;
use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;

mod common;

/// Benchmark: Shallow wide tree - 1 level, 1000 keys
fn bench_path_shallow_wide_1000(c: &mut Criterion) {
    let value = AnnotatedValue::new(
        common::create_flat_map(1000),
        SourceId::new("bench"),
        "root",
    );

    c.bench_function("path_shallow_wide_1000", |b| {
        b.iter(|| value.all_paths());
    });
}

/// Benchmark: Shallow wide tree - 1 level, 5000 keys
fn bench_path_shallow_wide_5000(c: &mut Criterion) {
    let value = AnnotatedValue::new(
        common::create_flat_map(5000),
        SourceId::new("bench"),
        "root",
    );

    c.bench_function("path_shallow_wide_5000", |b| {
        b.iter(|| value.all_paths());
    });
}

/// Benchmark: Deep narrow tree
fn bench_path_deep_narrow(c: &mut Criterion) {
    let mut group = c.benchmark_group("path_deep_narrow");
    group.warm_up_time(std::time::Duration::from_secs(1));

    for depth in [5, 10, 20, 50] {
        let value = common::create_nested_config(depth, 1, "root");

        group.bench_with_input(format!("depth_{}", depth), &depth, |b, _| {
            b.iter(|| black_box(&value).all_paths());
        });
    }

    group.finish();
}

/// Benchmark: Balanced tree (depth x width)
fn bench_path_balanced(c: &mut Criterion) {
    let mut group = c.benchmark_group("path_balanced");
    group.warm_up_time(std::time::Duration::from_secs(1));

    for (depth, width) in [(2, 10), (3, 10), (4, 5), (5, 4)] {
        let value = common::create_nested_config(depth, width, "root");

        group.bench_with_input(format!("d{}_w{}", depth, width), &(depth, width), |b, _| {
            b.iter(|| black_box(&value).all_paths())
        });
    }

    group.finish();
}

/// Benchmark: Empty/null value
fn bench_path_empty(c: &mut Criterion) {
    let value = AnnotatedValue::new(ConfigValue::Null, SourceId::new("bench"), "empty");

    c.bench_function("path_empty", |b| {
        b.iter(|| value.all_paths());
    });
}

/// Benchmark: Primitive leaf value
fn bench_path_primitive(c: &mut Criterion) {
    let value = AnnotatedValue::new(
        ConfigValue::String("just a string".to_string()),
        SourceId::new("bench"),
        "primitive",
    );

    c.bench_function("path_primitive", |b| {
        b.iter(|| value.all_paths());
    });
}

/// Benchmark: Array path traversal
fn bench_path_array(c: &mut Criterion) {
    let mut arr = Vec::new();
    for i in 0..100 {
        arr.push(AnnotatedValue::new(
            ConfigValue::map(vec![(
                "nested",
                AnnotatedValue::new(
                    ConfigValue::String(format!("v{}", i)),
                    SourceId::new("bench"),
                    format!("arr.{}.nested", i),
                ),
            )]),
            SourceId::new("bench"),
            format!("arr.{}", i),
        ));
    }

    let value = AnnotatedValue::new(
        ConfigValue::Array(arr.into()),
        SourceId::new("bench"),
        "arr",
    );

    c.bench_function("path_array_100", |b| {
        b.iter(|| value.all_paths());
    });
}

criterion_group!(
    benches,
    bench_path_shallow_wide_1000,
    bench_path_shallow_wide_5000,
    bench_path_deep_narrow,
    bench_path_balanced,
    bench_path_empty,
    bench_path_primitive,
    bench_path_array,
);
criterion_main!(benches);
