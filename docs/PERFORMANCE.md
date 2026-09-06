# 📊 Confers 性能指南

Confers 为高性能配置管理而生。本指南介绍针对生产负载优化性能的各项技术。

## 📋 目录

<details open>
<summary>📑 目录（点击展开）</summary>

- [概述](#-概述)
- [配置加载](#-配置加载)
- [校验性能](#-校验性能)
- [内存优化](#-内存优化)
- [并发](#-并发)
- [缓存](#-缓存)
- [基准测试](#-基准测试)

</details>

---

## 🎯 概述

Confers 的设计目标就是高性能配置管理。本指南涵盖针对生产工作负载进行性能优化的技术。

### 性能目标

| 操作 | 目标 | 说明 |
|:-----|:----:|:-----|
| 配置加载（小配置） | < 1ms | 1-50 个字段 |
| 配置加载（大配置） | < 10ms | 500+ 个字段 |
| 校验（每字段） | < 100ns | 简单规则 |
| 合并（每键） | < 1us | 深度合并 |
| 动态字段读取 | < 10ns | 无锁 |

---

## 📥 配置加载

### 异步 vs 同步加载

```rust
use confers::Config;

// 生产负载优先使用异步
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 异步加载允许并发 I/O
    let config = AppConfig::load().await?;
    Ok(())
}

// 简单脚本可使用同步加载
fn main() -> anyhow::Result<()> {
    let config = AppConfig::load_sync()?;
    Ok(())
}
```

### 惰性加载模式

只加载需要的内容：

```rust
// 与其一次性加载全部
let config = ConfigBuilder::<serde_json::Value>::new()
    .file("config.toml")
    .build()?;

// 按需加载特定分区
let server_config = config.get_section("server")?;
let db_config = config.get_section("database")?;
```

### 文件格式与性能

| 格式 | 加载速度 | 内存 | 建议 |
|:-----|:--------:|:----:|:-----|
| TOML | 快 | 中 | 默认之选 |
| JSON | 非常快 | 低 | 性能最佳 |
| YAML | 中 | 高 | 仅在需要时使用 |
| INI | 快 | 低 | 简单配置 |

**提示**：如追求极致性能，可使用 JSON 格式并启用 `features = ["json"]`。

---

## ✅ 校验性能

### 启用校验

```toml
# Cargo.toml
[dependencies]
confers = { features = ["validation"] }
```

### 校验缓存

校验缓存目前没有作为公开 API 暴露。如果对相同配置的重复校验成为瓶颈，可以自行用缓存（例如 `moka::Cache`）包裹 `Validate`。

### 尽量跳过开销大的规则

```rust
#[derive(Config)]
pub struct Config {
    #[config(skip_in_test)]
    pub expensive_rule: ExpensiveValidator,
}
```

---

## 🧠 内存优化

### 降低内存占用

```rust
use confers::Config;

// 使用紧凑字符串类型
#[derive(Config)]
pub struct Config {
    #[config(compact_string)]
    pub short_string: String,  // 内部驻留，内存占用低

    pub normal_string: String,
}

// 为大型配置启用内存上限
let loader = ConfigLoader::builder()
    .max_memory_mb(256)  // 限制为 256MB
    .build()?;
```

### 零拷贝读取

```rust
use confers::types::ConfigValue;

let value: &ConfigValue = config.get("key")?;
// 对配置值的零拷贝访问
println!("{:?}", value.as_str());
```

### 大文件的处理

`confers` 目前没有内置流式加载 API。对于超过 10 MB 的文件，建议先将文件一次性读入 `String`，再传给 `confers::parse_content`，让解析器直接在内存缓冲区上工作，避免反复读盘。

---

## 🔀 并发

### 无锁动态字段

```rust
use confers::dynamic::DynamicField;
use std::sync::Arc;

// 无锁读取，原子写入
let field = Arc::new(DynamicField::new(config.clone()));

// 多读者零争用
let snapshot1 = field.get();  // 无需加锁
let snapshot2 = field.get();  // 并发读取
```

### 跨线程共享配置

```rust
use std::sync::Arc;
use confers::{ConfigBuilder, ConfigProviderExt};

let config = Arc::new(
    ConfigBuilder::<AppConfig>::new()
        .file("config.toml")
        .env()
        .build()?
);

// 在多个异步任务之间共享
tokio::spawn({
    let config = Arc::clone(&config);
    async move {
        let value = config.get_string("key").unwrap();
        println!("{}", value);
    }
});
```

---

## 💾 缓存

### 内置缓存

```rust
use confers::loader::LoaderConfig;

// LoaderConfig 暴露了 loader 使用的内存缓存策略
let loader_config = LoaderConfig::default();
```

### ETag 与远程配置

```rust
use confers::remote::HttpPolledSourceBuilder;

let source = HttpPolledSourceBuilder::new()
    .url("https://config-server.example.com/app-config")
    .interval(std::time::Duration::from_secs(30))
    .build()?;
```

### 快照缓存

```rust
// 缓存开销大的校验结果
let snapshot = config.snapshot()?;
let cached = snapshot.restore()?;  // 命中缓存时几乎瞬时完成
```

---

## 🧪 基准测试

### 运行基准测试

```bash
# 运行全部基准测试
cargo bench

# 运行指定基准测试
cargo bench --bench load_bench

# 基准测试输出示例
test bench_config_load  ... bench: 1,000 ns/iter (+/- 50)
test bench_merge       ... bench: 2,500 ns/iter (+/- 100)
test bench_validate    ... bench:   500 ns/iter (+/- 25)
```

### 基准测试套件

`benches/` 中预定义的基准测试：

| 基准测试 | 文件 | 说明 |
|:---------|:-----|:-----|
| `load_bench` | `benches/load_bench.rs` | 配置加载性能 |
| `merge_bench` | `benches/merge_bench.rs` | 合并操作性能 |
| `interpolation_bench` | `benches/interpolation_bench.rs` | 变量插值性能 |
| `value_path_bench` | `benches/value_path_bench.rs` | 值路径访问性能 |
| `dynamic_field_bench` | `benches/dynamic_field_bench.rs` | 动态字段读写 |
| `hot_path_bench` | `benches/hot_path_bench.rs` | 热路径（动态 + watch） |
| `concurrent_rw_bench` | `benches/concurrent_rw_bench.rs` | 并发读写模式 |
| `concurrent_access_bench` | `benches/concurrent_access_bench.rs` | 并发访问模式 |

### 对应用进行性能剖析

```bash
# 在 Cargo.toml 中添加剖析依赖
[dependencies]
perf-monitor = "0.2"

# 在代码中使用
use perf_monitor::cpu_monitor::CpuMonitor;

let mut monitor = CpuMonitor::start();
let config = AppConfig::load()?;
println!("CPU time: {:?}", monitor.elapsed());
```

---

## 性能检查清单

上线生产环境之前：

- [ ] 运行 `cargo bench` 验证性能是否达标
- [ ] 使用 `json` 格式获得最佳加载性能
- [ ] 启用 `dynamic` 特性获得无锁读取
- [ ] 配置合理的缓存大小
- [ ] 为不可信配置设置内存上限
- [ ] 用真实负载做性能剖析
