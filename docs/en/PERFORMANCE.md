# ⚡ Confers Performance Guide

Confers is built for high-performance configuration management. This guide covers the techniques for optimizing performance under production workloads.

## 📋 Table of Contents

- [Overview](#-overview)
- [Config Loading](#-config-loading)
- [Validation Performance](#-validation-performance)
- [Memory Optimization](#-memory-optimization)
- [Concurrency](#-concurrency)
- [Caching](#-caching)
- [Benchmarks](#-benchmarks)
- [Performance Baselines](#-performance-baselines)
- [Related Documents](#-related-documents)

---

## 🎯 Overview

The following goals are the performance acceptance reference for production workloads.

### Performance Targets

| Operation | Target | Notes |
|:-----|:----:|:-----|
| Config loading (small config) | < 1ms | 1-50 fields |
| Config loading (large config) | < 10ms | 500+ fields |
| Validation (per field) | < 100ns | simple rules |
| Merge (per key) | < 1us | deep merge |
| Dynamic field read | < 10ns | lock-free |

---

## 📥 Config Loading

### Async vs Sync Loading

```rust
use confers::Config;

// Prefer async for production workloads
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Async loading allows concurrent I/O
    let config = ConfersConfig::load().await?;
    Ok(())
}

// Simple scripts can use synchronous loading
fn main() -> anyhow::Result<()> {
    let config = ConfersConfig::load_sync()?;
    Ok(())
}
```

### Lazy Segmented Parsing

For very large TOML documents (the `lazy-parse` feature), `LazySegmentedConfig` splits the document into segments by top-level table headers: splitting is a cheap line scan, and a segment is truly parsed and cached only on first access — segments never visited are never parsed.

```rust
use confers::lazy::LazySegmentedConfig;

let config = LazySegmentedConfig::from_toml_document(&huge_toml_text);

// Parse only the segments being accessed
let server = config.get_segment("server")?;

// Observe parsing progress
println!("已解析 {}/{} 段", config.parsed_count(), config.segment_keys().len());
```

### File Formats and Performance

| Format | Load Speed | Memory | Advice |
|:-----|:--------:|:----:|:-----|
| TOML | fast | medium | the default choice |
| JSON | very fast | low | best performance |
| YAML | medium | high | use only when needed |
| INI | fast | low | simple configs |

**Tip**: for maximum performance, use the JSON format with `features = ["json"]`.

---

## ✅ Validation Performance

### Enabling Validation

```toml
# Cargo.toml
[dependencies]
confers = { version = "0.6.0", features = ["validation"] }
```

### Validation Caching

A validation cache is not currently exposed as a public API. If repeated validation of the same config becomes a bottleneck, wrap `Validate` with your own cache (e.g. `moka::Cache`).

### Reducing the Fields Participating in Loading

Mark recompute-heavy fields that need not load from config with `#[config(skip)]`; the loader skips them entirely:

```rust
use confers::Config;

#[derive(Config)]
pub struct Config {
    pub port: u16,

    /// Not loaded from configuration; uses the struct default
    #[config(skip)]
    pub expensive_rule: ExpensiveValidator,
}
```

---

## 🧠 Memory Optimization

### Reducing Memory Footprint

Confers uses `IndexMap` internally to keep key order, usually requiring no extra handling. For config from untrusted sources, set resource limits explicitly:

```rust
use confers::{ConfigBuilder, ConfigLimits};

let config = ConfigBuilder::<ConfersConfig>::new()
    .file("config.toml")
    .limits(ConfigLimits {
        max_file_size_bytes: 64 * 1024 * 1024, // per-file cap: 64 MB
        max_nesting_depth: 32,
        ..Default::default()
    })
    .build()?;
```

### Zero-Copy Reads

The built-in in-memory config (`new_in_memory()`, requires the `async-core` feature; enabling any of `remote`/`config-bus`/`encryption`/`watch` automatically brings in that aggregate feature) implements `SharedValueReader`; `get_shared()` returns an `Arc` handle, avoiding deep copies when reading large values:

```rust
use confers::{new_in_memory, SharedValueReader};

let config = new_in_memory();
let shared = config.get_shared("large_key").await?;
// shared: Option<Arc<AnnotatedValue>> (the `?` already unwrapped the ConfersResult outer layer); cloning only bumps the refcount
```

### Handling Large Files

`confers` currently has no built-in streaming load API. For files over 10 MB, read the file into a `String` in one pass and hand it to `confers::parse_content`, letting the parser work directly on the in-memory buffer and avoiding repeated disk reads; for very large TOML documents, further consider the lazy segmented parsing of the `lazy-parse` feature (see the [Lazy Segmented Parsing](#lazy-segmented-parsing) section).
`confers` currently has no built-in streaming load API. For files over 10 MB, read the file into a `String` in one pass and hand it to `confers::parse_content`, letting the parser work directly on the in-memory buffer and avoiding repeated disk reads; for very large TOML documents, further consider the lazy segmented parsing of the `lazy-parse` feature (see the [Lazy Segmented Parsing](#lazy-segmented-parsing) section).

---

## 🔀 Concurrency

### Lock-Free Dynamic Fields

```rust
use confers::dynamic::DynamicField;
use std::sync::Arc;

// Lock-free reads, atomic writes
let field = Arc::new(DynamicField::new(config.clone()));

// Multiple readers, zero contention
let snapshot1 = field.get();  // no locking needed
let snapshot2 = field.get();  // concurrent reads
```

### Sharing Config Across Threads

```rust
use std::sync::Arc;
use confers::ConfigBuilder;

let config = Arc::new(
    ConfigBuilder::<ConfersConfig>::new()
        .file("config.toml")
        .env()
        .build()?
);

// Share across multiple async tasks
tokio::spawn({
    let config = Arc::clone(&config);
    async move {
        // The built artifact is a plain typed struct; access fields directly
        let host = config.host.clone();
        println!("{host}");
    }
});
```

---

## 💾 Caching

### Built-in Cache

The in-memory cache of parsed config values is an internal implementation of `InMemoryConfig` (`moka` provides future/sync dual-mode caching); `LoaderConfig` exposes no cache configuration items — it only controls loading behavior itself:

```rust
use confers::loader::LoaderConfig;

// Load behavior config only: size caps, allowed directories, symlink checks, format detection, error sanitization (no cache fields)
let loader_config = LoaderConfig::default();
```

### Remote Polling Interval

Remote config (the `remote` feature) polls at a fixed interval; enlarging the interval sensibly significantly reduces network and parsing overhead:

```rust
use confers::remote::HttpPolledSourceBuilder;

let source = HttpPolledSourceBuilder::new()
    .url("https://config-server.example.com/app-config")
    .interval(std::time::Duration::from_secs(30))
    .build()?;
```

### Snapshots and Restore

The `snapshot` feature provides config snapshot persistence and restore (`SnapshotManager`); restore goes through the parsed-file loading path and suits fast rollback:

```rust
use confers::snapshot::{SnapshotManager, SnapshotConfig};

let manager = SnapshotManager::new(SnapshotConfig::default());
let path = manager.save(&annotated_value, &["database.password"]).await?;
let restored = manager.load_snapshot(&path).await?;
```

---

## 🧪 Benchmarks

### Running Benchmarks

```bash
# Run all benchmarks
cargo bench

# Run a specific benchmark
cargo bench --bench load_bench

# Save a baseline for comparison across versions
cargo bench --save-baseline <name>
```

Criterion generates an HTML report with distribution plots and regression detection under `target/criterion/`; open it in a browser to view each case's median and confidence interval.

### Benchmark Suite

The benchmarks predefined in `benches/`:

| Benchmark | File | Description |
|:---------|:-----|:-----|
| `load_bench` | `benches/load_bench.rs` | config loading performance |
| `merge_bench` | `benches/merge_bench.rs` | merge operation performance |
| `interpolation_bench` | `benches/interpolation_bench.rs` | variable interpolation performance |
| `value_path_bench` | `benches/value_path_bench.rs` | value path access performance |
| `dynamic_field_bench` | `benches/dynamic_field_bench.rs` | dynamic field reads/writes |
| `hot_path_bench` | `benches/hot_path_bench.rs` | hot path (zero-copy reads) |
| `watch_callback_bench` | `benches/watch_callback_bench.rs` | change stream (ChangeStream) roundtrip and fanout |
| `etcd_watch_bench` | `benches/etcd_watch_bench.rs` | etcd watch event dispatch (revision tracking + callback dispatch) |
| `concurrent_rw_bench` | `benches/concurrent_rw_bench.rs` | concurrent read/write patterns |
| `concurrent_access_bench` | `benches/concurrent_access_bench.rs` | concurrent access patterns |

### Profiling Your Application

Benchmarks cover the library's internal hot paths; for application-level profiling use the standard toolchain:

```bash
# Flame graph (cargo-flamegraph, perf-based on Linux)
cargo install flamegraph
cargo flamegraph --bench load_bench

# Or sample a running example directly (examples are [[bin]] targets; use --bin, not --example)
perf record -g cargo run --bin basic_usage -p confers-examples
perf report
```

---

## 📊 Performance Baselines

> Baselines were collected locally on a development machine (WSL2, linux 6.6, 16 threads); runs beyond criterion defaults used
> `--warm-up-time 1 --measurement-time 2 --sample-size 20`.
> Values are the estimate (median scope) of the `[lower bound, estimate, upper bound]` interval.
>
> **CI gate note**: the CI benchmarks job re-measures on every run and compares against the main baseline, but is currently an
> **informational gate** — regressions only emit a WARNING and do not block merging (blocking thresholds such as P99 +15% await a stable
> baseline in the CI environment before being enabled); the first baseline was archived with `cargo bench --save-baseline rc.4`.

### Loading path (load_bench, default features)

| Case | Time (estimate) |
| --- | --- |
| load/50_fields | ~691 ns |
| load/100_fields | ~692 ns |
| load/200_fields | ~713 ns |

### Merge path (merge_bench, interpolation feature)

| Case | Time (estimate) |
| --- | --- |
| merge_shallow/size_10 | ~263 ns |
| merge_shallow/size_100 | ~1.91 µs |
| merge_shallow/size_1000 | ~20.9 µs |
| merge_deep/depth_2 | ~3.93 µs |
| merge_deep/depth_4 | ~830 µs |
| merge_deep/depth_6 | ~244 ms (exponential case; upper-bound reference only) |
| merge_strategies/join | ~46.7 ms |
| replace_strategy_1000 | ~66.3 µs |

### watch callback path (watch_callback_bench, change-stream feature)

End-to-end cost of the unified change stream (`ChangeStream`) publish → subscriber delivery:

| Case | Time (estimate) |
| --- | --- |
| change_stream_roundtrip_1_sub | ~1.98 µs |
| change_stream_roundtrip_8_sub | ~3.72 µs |
| change_stream_publish_ack (16-shot burst + pending accounting) | ~5.89 µs |

Conclusion: one change notification costs about 2 µs for a single subscriber and the 8-subscriber
fan-out stays < 4 µs — the notification path is not a hot-reload bottleneck (consistent with a
typical load at the ~0.7 µs order of magnitude).

### etcd watch dispatch path (etcd_watch_bench, etcd-watch feature)

The per-event hot path of `EtcdWatcher::run`: the mock transport delivers a batch of events at full speed, with an empty callback +
`fetch_max` revision tracking (excluding the real gRPC transport and user callbacks; the real path additionally has
protobuf decoding and a per-event key/value borrow→lossy conversion allocation — the mock constructs
`EtcdWatchEvent` directly, so that part is outside the baseline scope). Event batches are shared across iterations via
`Arc<Vec<_>>`, and the timed region excludes batch construction cost; the per-delivery String clone and the
watcher/spawn fixed overheads are counted in (amortized over the whole 1000-event batch):

| Case | Time (estimate) |
| --- | --- |
| dispatch_1000_events_empty_callback (whole batch) | [83.8 µs, 84.8 µs, 86.0 µs] |

Conclusion: per-event dispatch costs about 0.085 µs (the 0.47 µs first-batch figure is dominated by
setup clones, since separated via Arc sharing) — negligible against watch poll intervals (seconds).
This path is part of the stability-frozen contract; the baseline serves regression comparison across
versions. The CI benchmarks job runs with `--features dev,etcd-watch,change-stream` and re-measures
this baseline on every run.

### Zero-copy hot path (hot_path_bench)

`InMemoryConfig`'s internal storage switched to `Cache<String, Arc<AnnotatedValue>>`, and
`SharedValueReader::get_shared()` was added (returning an `Arc` handle to avoid deep-copying large values).
Comparison of reading a 10 KiB string value (within the same run):

| Read Path | Time (median) |
| --- | --- |
| `get_raw` (deep copy, pre-change path) | ~337 ns |
| `get_shared` (Arc shared handle, post-change path) | ~161 ns |

The zero-copy large-value read path improves throughput by about **2.1x**, independent of value size.
Side note: `hot_path_get_100_keys` (100-key get_string scan) ≈ 24.9 µs.
Zero public-API breakage: `get_raw`/`get_string` semantics unchanged; `get_shared` is a pure addition.

---

## 🔧 Performance Checklist

Before going to production:

- [ ] Run `cargo bench` to verify performance meets targets
- [ ] Use the `json` format for the best loading performance
- [ ] Enable the `dynamic` feature for lock-free reads
- [ ] Configure sensible cache sizes
- [ ] Set memory caps for untrusted config
- [ ] Profile with real workloads

---

## 📚 Related Documents

| Document | Description |
|:-----|:-----|
| [🏗️ Architecture](ARCHITECTURE.md) | the modules and data flows behind the performance design |
| [📖 User Guide](USER_GUIDE.md) | config loading and feature enabling |
| [📘 API Reference](API_REFERENCE.md) | complete signatures of the involved APIs |
| [🧪 Test Scenario Matrix](TEST_SCENARIOS.md) | acceptance scope for benchmark and concurrency scenarios |
