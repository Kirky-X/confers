// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

use confers::{new_in_memory, AnnotatedValue, ConfigReader, ConfigValue, ConfigWriter, SourceId};
use criterion::{criterion_group, criterion_main, Criterion};
use std::sync::Arc;
use tokio::runtime::Runtime;

fn bench_concurrent_rw(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let config = Arc::new(new_in_memory());

    c.bench_function("concurrent_rw_10_tasks", |b| {
        b.to_async(&rt).iter(|| {
            let config = Arc::clone(&config);
            async move {
                let mut handles = vec![];

                for _ in 0..5 {
                    let c = Arc::clone(&config);
                    handles.push(tokio::spawn(async move {
                        for i in 0..100 {
                            let _ = c.get_string(&format!("key_{i}")).await;
                        }
                    }));
                }

                for t in 0..5 {
                    let c = Arc::clone(&config);
                    handles.push(tokio::spawn(async move {
                        for i in 0..20 {
                            let value = AnnotatedValue::new(
                                ConfigValue::string(format!("val_{t}_{i}")),
                                SourceId::default(),
                                format!("key_{t}_{i}"),
                            );
                            c.set(&format!("key_{t}_{i}"), value).await.unwrap();
                        }
                    }));
                }

                for h in handles {
                    h.await.unwrap();
                }
            }
        })
    });
}

criterion_group!(benches, bench_concurrent_rw);
criterion_main!(benches);
