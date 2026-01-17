// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Benchmark tests for configuration parsing performance

use confers::providers::file_provider::FileConfigProvider;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::path::PathBuf;

static SAMPLE_TOML: &str = r#"
name = "benchmark-test"
version = "1.0.0"

[database]
url = "postgresql://localhost:5432/benchmark"
pool_size = 10
timeout = 30

[server]
host = "0.0.0.0"
port = 8080
workers = 4

[cache]
enabled = true
ttl = 3600
max_size = 1000

[features]
logging = true
metrics = true
caching = true
audit = false
"#;

static SAMPLE_JSON: &str = r#"{
    "name": "benchmark-test",
    "version": "1.0.0",
    "database": {
        "url": "postgresql://localhost:5432/benchmark",
        "pool_size": 10,
        "timeout": 30
    },
    "server": {
        "host": "0.0.0.0",
        "port": 8080,
        "workers": 4
    },
    "cache": {
        "enabled": true,
        "ttl": 3600,
        "max_size": 1000
    },
    "features": {
        "logging": true,
        "metrics": true,
        "caching": true,
        "audit": false
    }
}"#;

fn generate_large_config(size_kb: usize) -> String {
    let mut config = String::from(
        "[database]\nurl = \"postgresql://localhost:5432/large\"\npool_size = 10\ntimeout = 30\n\n[server]\nhost = \"0.0.0.0\"\nport = 8080\nworkers = 4\n\n",
    );

    let section = r#"[[services]]
name = "service-0"
enabled = true
port = 8080
timeout = 30
retry_count = 3

[service.endpoints]
health = "/health"
metrics = "/metrics"

"#;

    let target_size = size_kb * 1024;
    while config.len() < target_size {
        config.push_str(section);
    }

    config
}

fn bench_toml_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("config_parsing_toml");
    group.throughput(Throughput::Bytes(SAMPLE_TOML.len() as u64));

    group.bench_function("parse_toml_small", |b| {
        b.iter(|| {
            let _ = SAMPLE_TOML.parse::<toml::Value>();
        });
    });

    group.finish();
}

fn bench_json_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("config_parsing_json");
    group.throughput(Throughput::Bytes(SAMPLE_JSON.len() as u64));

    group.bench_function("parse_json_small", |b: &mut criterion::Bencher| {
        b.iter(|| {
            let _: Result<serde_json::Value, _> = serde_json::from_str(SAMPLE_JSON);
        });
    });

    group.finish();
}

fn bench_config_scaling(c: &mut Criterion) {
    let sizes = vec![1, 10, 50];
    let mut group = c.benchmark_group("config_parsing_scaling");

    for &size_kb in &sizes {
        let config_str = generate_large_config(size_kb);
        group.throughput(Throughput::Bytes(config_str.len() as u64));

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}KB", size_kb)),
            &config_str,
            |b: &mut criterion::Bencher, config: &String| {
                b.iter(|| {
                    let _: Result<toml::Value, _> = config.parse();
                });
            },
        );
    }

    group.finish();
}

fn bench_provider_loading(c: &mut Criterion) {
    let mut group = c.benchmark_group("provider_loading");

    group.bench_function("file_provider_creation", |b: &mut criterion::Bencher| {
        b.iter(|| {
            let provider = FileConfigProvider::new(vec![PathBuf::from("/tmp/config.toml")]);
            black_box(provider);
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_toml_parsing,
    bench_json_parsing,
    bench_config_scaling,
    bench_provider_loading
);
criterion_main!(benches);
