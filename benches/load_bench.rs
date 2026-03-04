//! Load benchmark for confers configuration library.
//!
//! Measures cold load performance for configurations with varying field counts.

use criterion::{criterion_group, criterion_main, Criterion};
use confers::Source;
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

/// Benchmark: 50 fields cold load
fn bench_load_50_fields(c: &mut Criterion) {
    let config_path = create_temp_config(50);

    c.bench_function("load_50_fields", |b| {
        b.iter(|| {
            let source = confers::FileSource::new(&config_path);
            source.collect()
        });
    });

    std::fs::remove_file(config_path).ok();
}

/// Benchmark: 100 fields cold load
fn bench_load_100_fields(c: &mut Criterion) {
    let config_path = create_temp_config(100);

    c.bench_function("load_100_fields", |b| {
        b.iter(|| {
            let source = confers::FileSource::new(&config_path);
            source.collect()
        });
    });

    std::fs::remove_file(config_path).ok();
}

/// Benchmark: 200 fields cold load
fn bench_load_200_fields(c: &mut Criterion) {
    let config_path = create_temp_config(200);

    c.bench_function("load_200_fields", |b| {
        b.iter(|| {
            let source = confers::FileSource::new(&config_path);
            source.collect()
        });
    });

    std::fs::remove_file(config_path).ok();
}

criterion_group!(
    benches,
    bench_load_50_fields,
    bench_load_100_fields,
    bench_load_200_fields
);
criterion_main!(benches);
