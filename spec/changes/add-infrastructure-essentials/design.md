# 设计：基础设施要素实施方案

## 架构概览

```
confers/
├── benches/                    # 基准测试
│   ├── load_bench.rs          # 加载性能
│   ├── merge_bench.rs         # 合并性能
│   ├── dynamic_field_bench.rs  # 动态字段性能
│   └── incremental_merge_bench.rs  # 增量合并性能
├── fuzz/                      # 模糊测试
│   ├── Cargo.toml
│   └── fuzz_targets/
│       ├── merger.rs
│       ├── parser.rs
│       └── interpolation.rs
├── .github/workflows/         # CI/CD
│   ├── ci.yml
│   ├── release.yml
│   ├── adr-check.yml
│   └── fuzz.yml
└── docs/adr/                 # ADR 文档
    ├── ADR-001.md
    └── ...
```

## 组件设计

### 1. 基准测试 (benches/)

使用 criterion 作为基准测试框架：

```toml
# benches/Cargo.toml
[package]
name = "confers-benches"
version = "0.3.0"
edition = "2021"

[dependencies]
confers = { path = ".." }
criterion = { version = "0.7", features = ["async_tokio"] }

[[bench]]
name = "load_bench"
harness = false
```

**目标指标**：
- 50 字段冷加载 < 5ms p99
- 8 层 × 50 键合并 < 1ms p99
- DynamicField::get 吞吐 > 100M ops/s
- 增量合并 < 0.1ms p99

### 2. 模糊测试 (fuzz/)

使用 libfuzzer-sys：

```toml
# fuzz/Cargo.toml
[package]
name = "confers-fuzz"
version = "0.0.0"
edition = "2021"

[dependencies]
confers = { path = ".." }
libfuzzer-sys = "0.4"

[[bin]]
name = "merger"
path = "fuzz_targets/merger.rs"
```

**覆盖场景**：
- 深层嵌套 Map 合并不 panic
- 循环引用检测返回正确错误
- 畸形 TOML/JSON/YAML 返回 ParseError
- 无限递归插值返回 CircularReference

### 3. CI/CD 工作流 (.github/workflows/)

#### ci.yml - 主 CI 流程

```yaml
jobs:
  - fmt (cargo fmt --check)
  - clippy (cargo clippy --all-features)
  - test (cargo test)
  - doc (cargo doc --no-deps)
  - deny (cargo deny check)
  - audit (cargo audit)
  - coverage (cargo llvm-cov)
  - msrv (cargo check --all-features)
  - integration (docker compose up; cargo test)
  - examples (cargo build --examples)
```

#### release.yml - 发布流程

```yaml
on:
  push:
    tags:
      - 'v*'
jobs:
  - build-release
  - publish-crates
  - create-github-release
```

#### fuzz.yml - 模糊测试定时任务

```yaml
on:
  schedule:
    - cron: '0 2 * * 0'  # 每周日凌晨
jobs:
  fuzz:
    runs: cargo fuzz run merger -- -max_total_time=300
```

#### adr-check.yml - ADR 一致性检查

```yaml
on:
  pull_request:
    paths: ['src/**/*.rs', 'docs/adr/*.md']
jobs:
  check:
    run: grep -r "ADR-" src/ --include="*.rs" | validate
```

### 4. ADR 文档 (docs/adr/)

基于 dev-v2.md 第 17 节的 ADR 摘要，创建 40 个文档：

| ADR | 主题 |
|-----|------|
| ADR-001 ~ ADR-010 | 核心架构决策 |
| ADR-011 ~ ADR-020 | 测试、CI、安全 |
| ADR-021 ~ ADR-030 | 错误处理、API 设计 |
| ADR-031 ~ ADR-040 | 新增功能 (DynamicField, Context-Aware 等) |

**ADR 模板**：

```markdown
# ADR-XXX: <标题>

## 状态
[ Proposed | Accepted | Deprecated ]

## 背景
<问题描述>

## 决策
<决定的内容>

## 后果
<正面影响 | 负面影响>
```

## 实施顺序

1. **基准测试** → 验证现有代码性能基线
2. **模糊测试** → 发现并修复潜在问题
3. **CI/CD** → 自动化验证
4. **ADR 文档** → 记录设计决策

## 错误处理

- 基准测试失败：警告但不阻塞合并（可后续优化）
- 模糊测试崩溃：记录为 bug，需修复
- CI 失败：阻止合并
- ADR 引用缺失：adr-check.yml 阻止合并
