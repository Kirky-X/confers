// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Load benchmark for confers configuration library.
//!
//! Measures cold load performance for configurations with varying field count.

use confers::Source;
use criterion::{criterion_group, criterion_main, Criterion};
use std::path::PathBuf;

/// Generate a config with a specified number of fields for benchmarking.
fn generate_config_string(field_count: usize) -> String {
    let mut toml = String::from("[app]\n");
    for i in 0..field_count {
        toml.push_str(&format!("field_{} = \"value_{}\"\n", i, i));
    }
    toml
}

/// Create a temporary config file and return its path.
fn create_temp_config(field_count: usize) -> PathBuf {
    let config_str = generate_config_string(field_count);
    let temp_dir = std::env::temp_dir();
    let config_path = temp_dir.join(format!("bench_config_{}.toml", field_count));
    std::fs::write(&config_path, &config_str).unwrap();
    config_path
}

fn bench_load(c: &mut Criterion) {
    let mut group = c.benchmark_group("load");

    for field_count in [50, 100, 200] {
        let config_path = create_temp_config(field_count);

        group.bench_function(format!("{}_fields", field_count), |b| {
            b.iter(|| {
                let source = confers::FileSource::new(&config_path);
                source.collect()
            });
        });

        std::fs::remove_file(config_path).ok();
    }

    group.finish();
}

criterion_group!(benches, bench_load);
criterion_main!(benches);
