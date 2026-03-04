//! DynamicField benchmark for confers configuration library.

use criterion::{criterion_group, criterion_main, Criterion};

/// Benchmark: DynamicField get operation
#[cfg(feature = "dynamic")]
fn bench_dynamic_field_get(c: &mut Criterion) {
    use confers::dynamic::DynamicField;

    let field = DynamicField::new(42u32);

    c.bench_function("dynamic_field_get", |b| {
        b.iter(|| field.get());
    });
}

/// Benchmark: DynamicField on_change callback registration
#[cfg(feature = "dynamic")]
fn bench_dynamic_field_register(c: &mut Criterion) {
    use confers::dynamic::DynamicField;

    let field = DynamicField::new(42u32);

    c.bench_function("dynamic_field_register", |b| {
        b.iter(|| {
            field.on_change(|_: &u32| {});
        });
    });
}

/// Placeholder benchmarks when dynamic feature is not enabled
#[cfg(not(feature = "dynamic"))]
fn bench_dynamic_field_disabled(_c: &mut Criterion) {}

criterion_group!(
    benches,
    #[cfg(feature = "dynamic")]
    bench_dynamic_field_get,
    #[cfg(feature = "dynamic")]
    bench_dynamic_field_register,
    #[cfg(not(feature = "dynamic"))]
    bench_dynamic_field_disabled
);
criterion_main!(benches);
