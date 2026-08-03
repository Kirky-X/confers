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

criterion_group!(benches, bench_hot_path_get);
criterion_main!(benches);
