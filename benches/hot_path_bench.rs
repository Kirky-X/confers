// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

use confers::{new_in_memory, AnnotatedValue, ConfigReader, ConfigValue, ConfigWriter, SourceId};
use criterion::{criterion_group, criterion_main, Criterion};
use tokio::runtime::Runtime;

fn bench_hot_path_get(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let config = new_in_memory();

    rt.block_on(async {
        for i in 0..100 {
            let value = AnnotatedValue::new(
                ConfigValue::string(format!("value_{i}")),
                SourceId::default(),
                format!("key_{i}"),
            );
            config.set(&format!("key_{i}"), value).await.unwrap();
        }
    });

    c.bench_function("hot_path_get_100_keys", |b| {
        b.to_async(&rt).iter(|| async {
            for i in 0..100 {
                let _ = config.get_string(&format!("key_{i}")).await;
            }
        })
    });
}

fn bench_dynamic_field_access(c: &mut Criterion) {
    use confers::dynamic::DynamicField;
    let field = DynamicField::new(String::from("initial"));

    c.bench_function("dynamic_field_get_ref", |b| {
        b.iter(|| {
            let _arc = field.get_ref();
            std::hint::black_box(&_arc);
        })
    });

    c.bench_function("dynamic_field_get_clone", |b| {
        b.iter(|| {
            let _val = field.get();
            std::hint::black_box(&_val);
        })
    });
}

criterion_group!(benches, bench_hot_path_get, bench_dynamic_field_access);
criterion_main!(benches);
