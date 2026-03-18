use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

fn bench_concurrent_reads(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let field = Arc::new(confers::DynamicField::new(42u32));

    c.bench_function("concurrent_reads_100", |b| {
        b.to_async(&rt).iter(|| {
            let f = field.clone();
            async move { f.get() }
        });
    });
}

fn bench_concurrent_updates(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let counter = Arc::new(AtomicUsize::new(0));
    let field = Arc::new(confers::DynamicField::new(0u32));

    c.bench_function("concurrent_updates_100", |b| {
        b.to_async(&rt).iter(|| {
            let counter = counter.clone();
            let f = field.clone();
            async move {
                let val = counter.fetch_add(1, Ordering::Relaxed) as u32;
                f.update(val)
            }
        });
    });
}

fn bench_callback_registration(c: &mut Criterion) {
    let field = confers::DynamicField::new(0u32);

    c.bench_function("callback_registration", |b| {
        b.iter(|| {
            let _guard = field.on_change(|_v| {});
        });
    });
}

fn bench_callback_many_fields(c: &mut Criterion) {
    c.bench_function("callback_many_fields", |b| {
        b.iter(|| {
            let field = confers::DynamicField::new(0u32);
            let _g1 = field.on_change(|_v| {});
            let _g2 = field.on_change(|_v| {});
            let _g3 = field.on_change(|_v| {});
            let _g4 = field.on_change(|_v| {});
            let _g5 = field.on_change(|_v| {});
        });
    });
}

fn bench_dynamic_get_many(c: &mut Criterion) {
    let field = confers::DynamicField::new(42u32);
    c.bench_function("dynamic_get_many", |b| {
        b.iter(|| {
            for _ in 0..1000 {
                black_box(field.get());
            }
        });
    });
}

criterion_group!(
    benches,
    bench_concurrent_reads,
    bench_concurrent_updates,
    bench_callback_registration,
    bench_callback_many_fields,
    bench_dynamic_get_many
);
criterion_main!(benches);
