//! DynamicField benchmark for confers configuration library.

use criterion::{criterion_group, criterion_main, Criterion};

#[cfg(feature = "dynamic")]
fn bench_dynamic_field_get(c: &mut Criterion) {
    use confers::dynamic::DynamicField;

    let field = DynamicField::new(42u32);

    c.bench_function("dynamic_field_get", |b| {
        b.iter(|| field.get());
    });
}

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

#[cfg(not(feature = "dynamic"))]
fn bench_dynamic_field_disabled(_c: &mut Criterion) {}

#[cfg(feature = "dynamic")]
criterion_group!(benches, bench_dynamic_field_get, bench_dynamic_field_register);

#[cfg(not(feature = "dynamic"))]
criterion_group!(benches, bench_dynamic_field_disabled);

criterion_main!(benches);
