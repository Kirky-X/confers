# 任务列表：基础设施要素

## 阶段 1：基准测试

### Task 1: 添加 benches/Cargo.toml 和 load_bench.rs

**Files:**
- Create: `benches/Cargo.toml`
- Create: `benches/load_bench.rs`

**Step 1: 创建 Cargo.toml**

```toml
[package]
name = "confers-benches"
version = "0.3.0"
edition = "2021"

[dependencies]
confers = { path = "..", features = ["full"] }
criterion = { version = "0.7", features = ["async_tokio"] }
tokio = { version = "1", features = ["full"] }

[[bench]]
name = "load_bench"
harness = false
```

**Step 2: 验证 Cargo.toml 语法**

```bash
cargo check -p confers-benches 2>&1 | head -20
```

**Step 3: 实现 load_bench.rs**

```rust
use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};
use confers::{Config, LoaderConfig};
use std::sync::Arc;

#[derive(Config, Debug)]
#[config(env_prefix = "BENCH_")]
struct BenchConfig {
    #[config(default = 1000)]
    pub timeout_ms: u32,
    pub host: String,
    pub port: u16,
}

fn bench_load(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("load");
    group.bench_function("50_fields", |b| {
        b.to_async(&rt).iter(|| async {
            BenchConfig::load().await
        });
    });
}

criterion_group!(benches, bench_load);
criterion_main!(benches);
```

**Step 4: 运行基准测试**

```bash
cargo bench --bench load_bench 2>&1 | tail -30
```

**Step 5: 提交**

```bash
git add benches/Cargo.toml benches/load_bench.rs
git commit -m "feat(bench): add load benchmark"
```

---

### Task 2: 添加 merge_bench.rs

**Files:**
- Create: `benches/merge_bench.rs`

**Step 1: 编写测试**

```rust
use criterion::{criterion_group, criterion_main, Criterion};
use confers::{AnnotatedValue, ConfigValue, MergeStrategy};

fn bench_merge(c: &mut Criterion) {
    let large_config = generate_nested_config(8, 50);
    let overlay = generate_nested_config(8, 10);

    c.bench_function("merge_8x50", |b| {
        b.iter(|| {
            AnnotatedValue::merge(&large_config, &overlay, MergeStrategy::DeepMerge)
        });
    });
}

criterion_group!(benches, bench_merge);
criterion_main!(benches);
```

**Step 2: 确认编译失败**

```bash
cargo check -p confers-benches 2>&1 | grep merge_bench
```

**Step 3: 实现 generate_nested_config 辅助函数**

```rust
fn generate_nested_config(depth: usize, fields_per_level: usize) -> AnnotatedValue {
    // 生成嵌套配置
}

fn criterion_group!(benches, bench_merge);
fn criterion_main!(benches);
```

**Step 4: 运行测试**

```bash
cargo bench --bench merge_bench
```

**Step 5: 提交**

```bash
git add benches/merge_bench.rs
git commit -m "feat(bench): add merge benchmark"
```

---

### Task 3: 添加 dynamic_field_bench.rs

**Files:**
- Create: `benches/dynamic_field_bench.rs`

**Step 1: 编写测试**

```rust
use criterion::{criterion_group, criterion_main, Criterion};
use confers::dynamic::DynamicField;

fn bench_dynamic_field_get(c: &mut Criterion) {
    let field = DynamicField::new(42u32);
    c.bench_function("get", |b| {
        b.iter(|| field.get());
    });
}
```

**Step 2: 确认编译失败（DynamicField 未导出）**

**Step 3: 在 lib.rs 中确保 DynamicField 导出**

```rust
#[cfg(feature = "dynamic")]
pub use dynamic::{CallbackGuard, DynamicField, DynamicFieldBuilder};
```

**Step 4: 运行测试**

```bash
cargo bench --bench dynamic_field_bench
```

**Step 5: 提交**

```bash
git add benches/dynamic_field_bench.rs
git commit -m "feat(bench): add DynamicField benchmark"
```

---

### Task 4: 添加 incremental_merge_bench.rs

**Files:**
- Create: `benches/incremental_merge_bench.rs`

**Step 1-5: 类似前序任务**

---

## 阶段 2：模糊测试

### Task 5: 添加 fuzz/Cargo.toml

**Files:**
- Create: `fuzz/Cargo.toml`

**Step 1: 创建 Cargo.toml**

```toml
[package]
name = "confers-fuzz"
version = "0.0.0"
edition = "2021"

[dependencies]
confers = { path = "..", features = ["full"] }
libfuzzer-sys = "0.4"

[[bin]]
name = "merger"
path = "fuzz_targets/merger.rs"

[[bin]]
name = "parser"
path = "fuzz_targets/parser.rs"

[[bin]]
name = "interpolation"
path = "fuzz_targets/interpolation.rs"
```

**Step 2: 验证 Cargo.toml**

```bash
cargo check -p confers-fuzz 2>&1 | head -10
```

**Step 3: 无需实现（仅配置）**

**Step 4: 验证项目编译**

```bash
cargo check --all-features
```

**Step 5: 提交**

```bash
git add fuzz/Cargo.toml
git commit -m "feat(fuzz): add fuzzing crate"
```

---

### Task 6: 添加 fuzz/fuzz_targets/merger.rs

**Files:**
- Create: `fuzz/fuzz_targets/merger.rs`

**Step 1: 编写测试**

```rust
#![no_main]

use libfuzzer_sys::fuzz_target;
use confers::{AnnotatedValue, MergeStrategy};

fuzz_target!(|data: &[u8]| {
    // 解析数据为两个 AnnotatedValue
    if let (Some(low), Some(high)) = parse_values(data) {
        let _ = AnnotatedValue::merge(&low, &high, MergeStrategy::Replace);
        let _ = AnnotatedValue::merge(&low, &high, MergeStrategy::DeepMerge);
    }
});
```

**Step 2: 确认编译失败（libfuzzer 未安装）**

**Step 3: 安装 nightly + cargo-fuzz**

```bash
rustup +nightly install
cargo install cargo-fuzz
```

**Step 4: 运行 fuzz**

```bash
cargo +nightly fuzz run merger 2>&1 | head -20
```

**Step 5: 提交**

```bash
git add fuzz/fuzz_targets/merger.rs
git commit -m "feat(fuzz): add merger fuzzer"
```

---

### Task 7: 添加 fuzz/fuzz_targets/parser.rs

**Files:**
- Create: `fuzz/fuzz_targets/parser.rs`

**Step 1-5: 类似 Task 6**

---

### Task 8: 添加 fuzz/fuzz_targets/interpolation.rs

**Files:**
- Create: `fuzz/fuzz_targets/interpolation.rs`

**Step 1-5: 类似 Task 6**

---

## 阶段 3：CI/CD

### Task 9: 添加 .github/workflows/ci.yml

**Files:**
- Create: `.github/workflows/ci.yml`

**Step 1: 创建工作流文件**

```yaml
name: CI

on:
  push:
    branches: [main, master]
  pull_request:

jobs:
  fmt:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo fmt --check

  clippy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo clippy --all-features -- -D warnings

  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo test

  # ... more jobs
```

**Step 2: 验证 YAML 语法**

```bash
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/ci.yml'))"
```

**Step 3: 无需实现（CI 在远程运行）**

**Step 4: 验证 workflow 格式**

```bash
gh run list --limit 1 2>&1 || echo "Not in repo context"
```

**Step 5: 提交**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: add main CI workflow"
```

---

### Task 10: 添加 .github/workflows/release.yml

**Files:**
- Create: `.github/workflows/release.yml`

**Step 1-5: 类似 Task 9**

---

### Task 11: 添加 .github/workflows/fuzz.yml

**Files:**
- Create: `.github/workflows/fuzz.yml`

**Step 1-5: 类似 Task 9**

---

### Task 12: 添加 .github/workflows/adr-check.yml

**Files:**
- Create: `.github/workflows/adr-check.yml`

**Step 1-5: 类似 Task 9**

---

## 阶段 4：ADR 文档

### Task 13: 创建核心 ADR (ADR-001 ~ ADR-010)

**Files:**
- Create: `docs/adr/ADR-001-async-runtime.md`
- ...
- Create: `docs/adr/ADR-010-observability.md`

**Step 1: 创建 ADR-001**

```markdown
# ADR-001: 异步运行时选择

## 状态
Accepted

## 背景
需要为异步操作选择运行时。

## 决策
使用 tokio 作为异步运行时，同步使用不依赖 tokio。

## 后果
- 正面：成熟稳定，生态丰富
- 负面：二进制体积增加
```

**Step 2: 验证文件创建**

```bash
ls docs/adr/ADR-001*.md
```

**Step 3-5: 批量创建剩余 ADR**

---

### Task 14: 创建安全和测试 ADR (ADR-011 ~ ADR-020)

**Files:**
- Create: `docs/adr/ADR-011-test-strategy.md`
- ...
- Create: `docs/adr/ADR-020-security-best-practices.md`

---

### Task 15: 创建 API 和错误处理 ADR (ADR-021 ~ ADR-030)

**Files:**
- Create: `docs/adr/ADR-021-error-messages.md`
- ...
- Create: `docs/adr/ADR-030-override-policy.md`

---

### Task 16: 创建新增功能 ADR (ADR-031 ~ ADR-040)

**Files:**
- Create: `docs/adr/ADR-031-dynamic-field.md`
- ...
- Create: `docs/adr/ADR-040-value-provenance.md`

---

## 阶段 5：验证

### Task 17: 最终验证

**Step 1: 编译检查**

```bash
cargo check --all-features 2>&1 | tail -5
```

**Step 2: 测试检查**

```bash
cargo test --all-features 2>&1 | tail -10
```

**Step 3: 基准测试**

```bash
cargo bench 2>&1 | grep -E "loading|merging" | head -5
```

**Step 4: 目录检查**

```bash
echo "=== Benches ===" && ls benches/
echo "=== Fuzz ===" && ls fuzz/
echo "=== Workflows ===" && ls .github/workflows/
echo "=== ADR ===" && ls docs/adr/ | wc -l
```

**Step 5: 提交**

```bash
git add -A
git commit -m "chore: add infrastructure essentials (benchmarks, fuzzing, CI/CD, ADR)"
```
