use criterion::{criterion_group, criterion_main, Criterion};
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
        let counter = counter.clone();
        let f = field.clone();
        b.to_async(&rt).iter(|| {
            let val = counter.fetch_add(1, Ordering::Relaxed) as u32;
            async move { f.update(val) }
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

criterion_group!(
    benches,
    bench_concurrent_reads,
    bench_concurrent_updates,
    bench_callback_registration
);
criterion_main!(benches);
