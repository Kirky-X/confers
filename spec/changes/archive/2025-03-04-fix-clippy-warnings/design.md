# Design: 修复剩余5个 Clippy 警告

## 架构概述

本变更不涉及架构修改，仅修复代码质量问题。变更集中在三个模块：

```
confers/
├── src/
│   ├── value.rs          # if_same_then_else, too_many_arguments
│   ├── config/
│   │   └── chain.rs      # should_implement_trait
│   └── dynamic.rs        # type_complexity
```

## 组件设计

### 1. 修复 `if_same_then_else` (value.rs)

**问题**：
```rust
let start_path = if include_self {
    self.path.clone()
} else {
    self.path.clone()
};
```

**解决方案**：
```rust
let start_path = self.path.clone();
```

**理由**：
- 两个分支完全相同，条件判断无意义
- `include_self` 参数虽然在 `paths` 变量初始化时有意义，但对 `start_path` 无影响
- 这是代码复制粘贴引入的逻辑错误

### 2. 修复 `type_complexity` (dynamic.rs)

**问题**：
```rust
callbacks: Arc<RwLock<HashMap<CallbackId, Box<dyn Fn(&T) + Send + Sync>>>>
```

**解决方案**：添加类型别名
```rust
/// Callback storage for dynamic field change notifications.
type CallbackStorage<T> = Arc<RwLock<HashMap<CallbackId, Box<dyn Fn(&T) + Send + Sync>>>>;

pub struct DynamicField<T: Clone + Send + Sync + 'static> {
    value: ArcSwap<T>,
    callbacks: CallbackStorage<T>,
    next_id: AtomicU64,
}
```

**优势**：
- 提高类型签名的可读性
- 集中定义类型，便于维护
- 添加文档说明用途

### 3. 修复 `should_implement_trait` (chain.rs)

**问题**：方法名 `add` 触发 `should_implement_trait` 警告

**解决方案**：重命名为 `push`
```rust
// Before:
pub fn add(mut self, source: Box<dyn Source>) -> Self

// After:
pub fn push(mut self, source: Box<dyn Source>) -> Self
```

**理由**：
- `push` 符合 Rust `Vec` 的惯用法
- 避免实现 `std::ops::Add` trait 导致的语义混淆
- builder 模式不需要操作符重载

**影响范围**：7 处调用（均在 `chain.rs` 测试代码中）

### 4. 修复 `too_many_arguments` (value.rs)

**问题**：`ConflictReport::new` 有 8 个参数

**解决方案**：添加 Builder 模式
```rust
impl ConflictReport {
    pub fn builder() -> ConflictReportBuilder {
        ConflictReportBuilder::default()
    }
}

pub struct ConflictReportBuilder {
    path: Option<Arc<str>>,
    low_value: Option<String>,
    low_source: Option<SourceId>,
    low_location: Option<SourceLocation>,
    high_value: Option<String>,
    high_source: Option<SourceId>,
    high_location: Option<SourceLocation>,
    winner: Option<ConflictWinner>,
}

impl ConflictReportBuilder {
    pub fn path(mut self, path: impl Into<Arc<str>>) -> Self {
        self.path = Some(path.into());
        self
    }

    pub fn low_value(mut self, value: String, source: SourceId) -> Self {
        self.low_value = Some(value);
        self.low_source = Some(source);
        self
    }

    pub fn low_location(mut self, loc: SourceLocation) -> Self {
        self.low_location = Some(loc);
        self
    }

    pub fn high_value(mut self, value: String, source: SourceId) -> Self {
        self.high_value = Some(value);
        self.high_source = Some(source);
        self
    }

    pub fn high_location(mut self, loc: SourceLocation) -> Self {
        self.high_location = Some(loc);
        self
    }

    pub fn winner(mut self, winner: ConflictWinner) -> Self {
        self.winner = Some(winner);
        self
    }

    pub fn build(self) -> ConflictReport {
        ConflictReport::new(
            self.path.expect("path required"),
            self.low_value.expect("low_value required"),
            self.low_source.expect("low_source required"),
            self.low_location,
            self.high_value.expect("high_value required"),
            self.high_source.expect("high_source required"),
            self.high_location,
            self.winner.expect("winner required"),
        )
    }
}
```

**使用示例**：
```rust
let report = ConflictReport::builder()
    .path("database.host")
    .low_value("localhost".to_string(), low_source)
    .low_location(low_loc)
    .high_value("127.0.0.1".to_string(), high_source)
    .high_location(high_loc)
    .winner(ConflictWinner::High)
    .build();
```

**优势**：
- 可扩展性好
- 更符合 Rust 生态惯用法
- 显著提高可读性

**影响范围**：1 处调用（在 `value.rs` 测试代码中）

## 数据模型

无数据模型变更。

## API 设计

### 公共 API

无公共 API 变更。所有变更均为内部实现。

### 内部 API

| API | 变更 | 破坏性 |
|-----|------|--------|
| `SourceChain::add` | 重命名为 `push` | 是 |
| `ConflictReport::new` | 保留，添加 `builder()` | 否 |
| `DynamicField.callbacks` | 类型改为 `CallbackStorage<T>` | 否（仅类型别名） |

## 错误处理

无错误处理变更。

## 迁移计划

### 测试代码更新

由于所有变更影响的都是测试代码，迁移步骤：

1. 更新 `chain.rs` 测试中的 `add` 调用 → `push`
2. 更新 `value.rs` 测试中的 `ConflictReport::new` 调用 → 使用 builder
3. 运行 `cargo test --all-features` 验证

### 兼容性

- **向后兼容**：外部 API 无变更
- **内部破坏性变更**：可接受（仅测试代码受影响）

## 测试策略

### 单元测试

所有现有测试应继续通过：
- `chain.rs` 测试：验证 `push` 方法功能
- `value.rs` 测试：验证 `ConflictReport` 功能
- `dynamic.rs` 测试：验证 callback 功能

### Clippy 检查

```bash
cargo clippy --all-features
# 期望：无警告
```

### 覆盖率

无需添加新测试，现有测试已覆盖所有变更。
