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
fn bench_dynamic_field_trigger(c: &mut Criterion) {
    use confers::dynamic::DynamicField;

    let field = DynamicField::new(42u32);
    
    // Pre-register a callback to simulate real-world usage
    field.on_change(|_: &u32| {});

    c.bench_function("dynamic_field_trigger", |b| {
        b.iter(|| {
            field.set(43);
        });
    });
}

#[cfg(feature = "dynamic")]
fn bench_dynamic_field_trigger_multiple(c: &mut Criterion) {
    use confers::dynamic::DynamicField;

    let field = DynamicField::new(42u32);
    
    // Pre-register multiple callbacks to simulate real-world usage with multiple listeners
    field.on_change(|_: &u32| {});
    field.on_change(|_: &u32| {});
    field.on_change(|_: &u32| {});

    c.bench_function("dynamic_field_trigger_multiple", |b| {
        b.iter(|| {
            field.set(43);
        });
    });
}

#[cfg(not(feature = "dynamic"))]
fn bench_dynamic_field_disabled(_c: &mut Criterion) {}

#[cfg(feature = "dynamic")]
criterion_group!(
    benches,
    bench_dynamic_field_get,
    bench_dynamic_field_trigger,
    bench_dynamic_field_trigger_multiple
);

#[cfg(not(feature = "dynamic"))]
criterion_group!(benches, bench_dynamic_field_disabled);

criterion_main!(benches);
