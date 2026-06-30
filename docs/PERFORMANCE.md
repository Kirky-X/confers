# Performance Optimization Guide

<span id="top"></span>

<div align="center">

<img src="image/confers.png" alt="Confers Logo" width="150" style="margin-bottom: 16px">

### Performance Tuning for Confers

[🏠 Home](../README.md) • [📖 User Guide](USER_GUIDE.md) • [🔒 Security](SECURITY.md)

---

</div>

## Table of Contents

- [Overview](#overview)
- [Configuration Loading](#configuration-loading)
- [Validation Performance](#validation-performance)
- [Memory Optimization](#memory-optimization)
- [Concurrency](#concurrency)
- [Caching](#caching)
- [Benchmarking](#benchmarking)

---

## <span id="overview">Overview</span>

Confers is designed for high-performance configuration management. This guide covers techniques to optimize performance for production workloads.

### Performance Targets

| Operation | Target | Notes |
|:----------|:------:|:------|
| Config load (small) | < 1ms | 1-50 fields |
| Config load (large) | < 10ms | 500+ fields |
| Validation (per field) | < 100ns | Simple rules |
| Merge (per key) | < 1us | Deep merge |
| Dynamic field read | < 10ns | Lock-free |

---

## <span id="configuration-loading">Configuration Loading</span>

### Async vs Sync Loading

```rust
use confers::Config;

// Prefer async for production workloads
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Async loading allows concurrent I/O
    let config = AppConfig::load().await?;
    Ok(())
}

// Sync loading for simple scripts
fn main() -> anyhow::Result<()> {
    let config = AppConfig::load_sync()?;
    Ok(())
}
```

### Lazy Loading Pattern

Only load what's needed:

```rust
// Instead of loading everything
let config = ConfigBuilder::<serde_json::Value>::new()
    .file("config.toml")
    .build()?;

// Load specific sections on demand
let server_config = config.get_section("server")?;
let db_config = config.get_section("database")?;
```

### File Format Performance

| Format | Load Speed | Memory | Recommendation |
|:-------|:----------:|:------:|:---------------|
| TOML | Fast | Medium | Default choice |
| JSON | Very Fast | Low | Best performance |
| YAML | Medium | High | Use only if needed |
| INI | Fast | Low | Simple configs |

**Tip:** For maximum performance, use JSON format with `features = ["json"]`.

---

## <span id="validation-performance">Validation Performance</span>

### Enable Parallel Validation

```toml
# Cargo.toml
[dependencies]
confers = { features = ["parallel", "validation"] }
```

```rust
#[derive(Config)]
#[config(validate)]
#[config(parallel_validation = true)]  // Enable parallel validation
pub struct LargeConfig {
    pub field_1: String,
    pub field_2: String,
    // ... 100+ fields
}
```

### Validation Cache

Validation caching is not currently exposed as a public API. Wrap `Validate`
in your own cache (e.g. `moka::Cache`) if repeated validation of identical
configurations becomes a bottleneck.

### Skip Expensive Rules When Possible

```rust
#[derive(Config)]
pub struct Config {
    #[config(skip_in_test)]
    pub expensive_rule: ExpensiveValidator,
}
```

---

## <span id="memory-optimization">Memory Optimization</span>

### Reduce Memory Footprint

```rust
use confers::Config;

// Use compact string types
#[derive(Config)]
pub struct Config {
    #[config(compact_string)]
    pub short_string: String,  // Interned, low memory

    pub normal_string: String,
}

// Enable memory limits for large configs
let loader = ConfigLoader::builder()
    .max_memory_mb(256)  // Limit to 256MB
    .build()?;
```

### Zero-Copy Reads

```rust
use confers::types::ConfigValue;

let value: &ConfigValue = config.get("key")?;
// Zero-copy access to config values
println!("{:?}", value.as_str());
```

### Streaming for Large Files

`confers` does not currently ship a streaming loader API. For files larger
than 10 MB, prefer loading the file into a `String` once and passing it to
`confers::parse_content` so the parser can operate on the in-memory buffer
without re-reading from disk.

---

## <span id="concurrency">Concurrency</span>

### Lock-Free Dynamic Fields

```rust
use confers::dynamic::DynamicField;
use std::sync::Arc;

// Lock-free reads, atomic writes
let field = Arc::new(DynamicField::new(config.clone()));

// Multiple readers with no contention
let snapshot1 = field.get();  // No lock needed
let snapshot2 = field.get();  // Concurrent read
```

### Shared Config Across Threads

```rust
use std::sync::Arc;
use confers::{ConfigBuilder, ConfigProviderExt};

let config = Arc::new(
    ConfigBuilder::<AppConfig>::new()
        .file("config.toml")
        .env()
        .build()?
);

// Share across multiple async tasks
tokio::spawn({
    let config = Arc::clone(&config);
    async move {
        let value = config.get_string("key").unwrap();
        println!("{}", value);
    }
});
```

### Parallel Source Loading

```rust
let config = ConfigBuilder::new()
    .file_async("config.toml")      // Load in parallel
    .env_async()                     // Load in parallel
    .build()?;

// Waits for all sources concurrently
```

---

## <span id="caching">Caching</span>

### Built-in Cache

```rust
use confers::loader::LoaderConfig;

// LoaderConfig exposes the in-memory cache policy used by the loader.
let loader_config = LoaderConfig::default();
```

### ETags and Remote Config

```rust
use confers::remote::HttpPolledSourceBuilder;

let source = HttpPolledSourceBuilder::new()
    .url("https://config-server.example.com/app-config")
    .interval(std::time::Duration::from_secs(30))
    .build()?;
```

### Snapshot Caching

```rust
// Cache expensive validation results
let snapshot = config.snapshot()?;
let cached = snapshot.restore()?;  // Instant for cached values
```

---

## <span id="benchmarking">Benchmarking</span>

### Run Benchmarks

```bash
# Run all benchmarks
cargo bench

# Run specific benchmark
cargo bench --bench load_bench

# Benchmark output
test bench_config_load  ... bench: 1,000 ns/iter (+/- 50)
test bench_merge       ... bench: 2,500 ns/iter (+/- 100)
test bench_validate    ... bench:   500 ns/iter (+/- 25)
```

### Benchmark Suite

Pre-defined benchmarks in `benches/`:

| Benchmark | File | Description |
|:----------|:-----|:------------|
| `load_bench` | `benches/load_bench.rs` | Config loading performance |
| `merge_bench` | `benches/merge_bench.rs` | Merge operation performance |
| `dynamic_field_bench` | `benches/dynamic_field_bench.rs` | Dynamic field read/write |
| `incremental_merge_bench` | `benches/incremental_merge_bench.rs` | Incremental merge performance |
| `concurrent_access_bench` | `benches/concurrent_access_bench.rs` | Concurrent access patterns |

### Profile Your Application

```bash
# Add profiling to your Cargo.toml
[dependencies]
perf-monitor = "0.2"

# Use in code
use perf_monitor::cpu_monitor::CpuMonitor;

let mut monitor = CpuMonitor::start();
let config = AppConfig::load()?;
println!("CPU time: {:?}", monitor.elapsed());
```

---

## Performance Checklist

Before production deployment:

- [ ] Run `cargo bench` to verify performance meets targets
- [ ] Enable `parallel` feature for large configs
- [ ] Use `json` format for best load performance
- [ ] Enable `dynamic` feature for lock-free reads
- [ ] Configure appropriate cache sizes
- [ ] Set memory limits for untrusted configs
- [ ] Profile with real workloads

---

<div align="center">

**[⬆ Back to Top](#top)**

Built with performance in mind by Kirky.X

</div>
