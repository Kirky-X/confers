//! Merge benchmark for confers configuration library.

use criterion::{criterion_group, criterion_main, Criterion};
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
    bench_annotated_value
);
criterion_main!(benches);
