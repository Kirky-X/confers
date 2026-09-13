# ⚡ Confers 性能指南

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
- [性能基线](#-性能基线)
- [相关文档](#-相关文档)

</details>

---

## 🎯 概述

以下目标是面向生产工作负载的性能验收参考。

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

### 惰性分段解析

对超大 TOML 文档（`lazy` 特性），`LazySegmentedConfig` 按顶层表头把文档切分为段：切分是一次廉价的行扫描，段在首次访问时才真正解析并缓存，从未被访问的段永远不会被解析。

```rust
use confers::lazy::LazySegmentedConfig;

let config = LazySegmentedConfig::from_toml_document(&huge_toml_text);

// 只解析被访问的段
let server = config.get_segment("server")?;

// 观察解析进度
println!("已解析 {}/{} 段", config.parsed_count(), config.segment_keys().len());
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
confers = { version = "0.6.0-rc.3", features = ["validation"] }
```

### 校验缓存

校验缓存目前没有作为公开 API 暴露。如果对相同配置的重复校验成为瓶颈，可以自行用缓存（例如 `moka::Cache`）包裹 `Validate`。

### 减少参与加载的字段

无需从配置加载的重计算字段使用 `#[config(skip)]` 标记，加载器会完全跳过它们：

```rust
use confers::Config;

#[derive(Config)]
pub struct Config {
    pub port: u16,

    /// 不参与配置加载，使用结构体默认值
    #[config(skip)]
    pub expensive_rule: ExpensiveValidator,
}
```

---

## 🧠 内存优化

### 降低内存占用

Confers 内部已对短字符串做驻留优化（`compact_str`），并使用 `IndexMap` 保持键序，通常无需额外处理。对不可信来源的配置，建议显式设置资源上限：

```rust
use confers::{ConfigBuilder, ConfigLimits};

let config = ConfigBuilder::<AppConfig>::new()
    .file("config.toml")
    .limits(ConfigLimits {
        max_file_size_bytes: 64 * 1024 * 1024, // 单文件上限 64 MB
        max_nesting_depth: 32,
        ..Default::default()
    })
    .build()?;
```

### 零拷贝读取

内置内存配置（`new_in_memory()`，需 `remote`/`config-bus`/`encryption`/`watch` 任一特性）实现了 `SharedValueReader`，`get_shared()` 返回 `Arc` 句柄，读取大 value 时避免深拷贝：

```rust
use confers::{new_in_memory, SharedValueReader};

let config = new_in_memory();
let shared = config.get_shared("large_key").await?;
// shared: Arc<AnnotatedValue>，克隆只增加引用计数
```

### 大文件的处理

`confers` 目前没有内置流式加载 API。对于超过 10 MB 的文件，建议先将文件一次性读入 `String`，再传给 `confers::parse_content`，让解析器直接在内存缓冲区上工作，避免反复读盘；TOML 超大文档可进一步考虑 `lazy` 特性的惰性分段解析（见[惰性分段解析](#惰性分段解析)一节）。

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

### 远程轮询间隔

远程配置（`remote` 特性）按固定间隔轮询，合理放大间隔可显著降低网络与解析开销：

```rust
use confers::remote::HttpPolledSourceBuilder;

let source = HttpPolledSourceBuilder::new()
    .url("https://config-server.example.com/app-config")
    .interval(std::time::Duration::from_secs(30))
    .build()?;
```

### 快照与恢复

`snapshot` 特性提供配置快照落盘与恢复（`SnapshotManager`），恢复走已解析文件的加载路径，可用于快速回滚：

```rust
use confers::snapshot::{SnapshotManager, SnapshotConfig};

let manager = SnapshotManager::new(SnapshotConfig::default());
let path = manager.save(&annotated_value, &["database.password"]).await?;
let restored = manager.load_snapshot(&path).await?;
```

---

## 🧪 基准测试

### 运行基准测试

```bash
# 运行全部基准测试
cargo bench

# 运行指定基准测试
cargo bench --bench load_bench

# 保存基线，供后续版本对比
cargo bench --save-baseline <name>
```

Criterion 会在 `target/criterion/` 下生成含分布图与回归检测的 HTML 报告，用浏览器打开即可查看各用例的中位数与置信区间。

### 基准测试套件

`benches/` 中预定义的基准测试：

| 基准测试 | 文件 | 说明 |
|:---------|:-----|:-----|
| `load_bench` | `benches/load_bench.rs` | 配置加载性能 |
| `merge_bench` | `benches/merge_bench.rs` | 合并操作性能 |
| `interpolation_bench` | `benches/interpolation_bench.rs` | 变量插值性能 |
| `value_path_bench` | `benches/value_path_bench.rs` | 值路径访问性能 |
| `dynamic_field_bench` | `benches/dynamic_field_bench.rs` | 动态字段读写 |
| `hot_path_bench` | `benches/hot_path_bench.rs` | 热路径（零拷贝读取） |
| `watch_callback_bench` | `benches/watch_callback_bench.rs` | 变更流（ChangeStream）往返与扇出 |
| `concurrent_rw_bench` | `benches/concurrent_rw_bench.rs` | 并发读写模式 |
| `concurrent_access_bench` | `benches/concurrent_access_bench.rs` | 并发访问模式 |

### 对应用进行性能剖析

基准测试覆盖库内热路径；应用级剖析建议使用标准工具链：

```bash
# 火焰图（cargo-flamegraph，Linux 下基于 perf）
cargo install flamegraph
cargo flamegraph --bench load_bench

# 或直接对运行中的示例采样
perf record -g cargo run --example basic_usage -p confers-examples
perf report
```

---

## 📊 性能基线

> 基线在开发机（WSL2, linux 6.6, 16 线程）本地采集，criterion 默认参数之外的运行使用
> `--warm-up-time 1 --measurement-time 2 --sample-size 20`。
> 数值为 `[lower bound, estimate, upper bound]` 区间的 estimate（中位数口径）。
>
> **CI 门禁说明**：阈值（如 P99 +15% 阻断）待基线在 CI 环境稳定后启用；当前以
> `cargo bench --save-baseline rc.4` 归档，供后续版本对比。

### 加载路径（load_bench，default features）

| 用例 | 耗时（estimate） |
| --- | --- |
| load/50_fields | ~691 ns |
| load/100_fields | ~692 ns |
| load/200_fields | ~713 ns |

### 合并路径（merge_bench，interpolation feature）

| 用例 | 耗时（estimate） |
| --- | --- |
| merge_shallow/size_10 | ~263 ns |
| merge_shallow/size_100 | ~1.91 µs |
| merge_shallow/size_1000 | ~20.9 µs |
| merge_deep/depth_2 | ~3.93 µs |
| merge_deep/depth_4 | ~830 µs |
| merge_deep/depth_6 | ~244 ms（指数级用例，仅作上限参考） |
| merge_strategies/join | ~46.7 ms |
| replace_strategy_1000 | ~66.3 µs |

### watch 回调路径（watch_callback_bench，change-stream feature）

统一变更流（`ChangeStream`）publish → 订阅端送达的端到端成本：

| 用例 | 耗时（estimate） |
| --- | --- |
| change_stream_roundtrip_1_sub | ~1.98 µs |
| change_stream_roundtrip_8_sub | ~3.72 µs |
| change_stream_publish_ack（16 连发 + pending 记账） | ~5.89 µs |

结论：单订阅者一次变更通告约 2 µs，8 订阅者扇出 < 4 µs，通知路径不构成
热重载瓶颈（对比一次典型 load 的 ~0.7 µs 量级一致）。

### 零拷贝热路径（hot_path_bench）

`InMemoryConfig` 内部存储改为 `Cache<String, Arc<AnnotatedValue>>`，并新增
`SharedValueReader::get_shared()`（返回 `Arc` 句柄，避免深拷贝大 value）。
10 KiB 字符串 value 的读取对比（同一运行内）：

| 读取路径 | 耗时（median） |
| --- | --- |
| `get_raw`（深拷贝，变更前路径） | ~337 ns |
| `get_shared`（Arc 共享句柄，变更后路径） | ~161 ns |

大 value 读取零拷贝路径约 **2.1x** 提升，且不随 value 体积增长。
附带记录：`hot_path_get_100_keys`（100 键 get_string 扫描）≈ 24.9 µs。
公共 API 零破坏：`get_raw`/`get_string` 语义不变，`get_shared` 为纯新增。

---

## 🔧 性能检查清单

上线生产环境之前：

- [ ] 运行 `cargo bench` 验证性能是否达标
- [ ] 使用 `json` 格式获得最佳加载性能
- [ ] 启用 `dynamic` 特性获得无锁读取
- [ ] 配置合理的缓存大小
- [ ] 为不可信配置设置内存上限
- [ ] 用真实负载做性能剖析

---

## 📚 相关文档

| 文档 | 说明 |
|:-----|:-----|
| [🏗️ 架构文档](ARCHITECTURE.md) | 性能设计背后的模块与数据流 |
| [📖 用户指南](USER_GUIDE.md) | 配置加载与特性启用 |
| [📘 API 参考](API_REFERENCE.md) | 涉及的 API 完整签名 |
| [🧪 测试场景矩阵](TEST_SCENARIOS.md) | 基准与并发场景的验收口径 |
