# 任务列表：修复剩余5个 Clippy 警告

## Task 1: 修复 `if_same_then_else` 逻辑错误

**Files:**
- Modify: `src/value.rs:721-725`
- Test: `src/value.rs` (现有测试)

**Step 1: 理解当前代码**

```bash
# 查看当前实现
cargo clippy --all-features 2>&1 | grep "if_same_then_else"
```

**Step 2: 修复代码**

将 `src/value.rs` 第 721-725 行：
```rust
let start_path = if include_self {
    self.path.clone()
} else {
    self.path.clone()
};
```

修改为：
```rust
let start_path = self.path.clone();
```

**Step 3: 运行 clippy 验证**

```bash
cargo clippy --all-features
```

期望：`if_same_then_else` 警告消失

**Step 4: 运行测试验证**

```bash
cargo test --all-features
```

期望：所有测试通过

**Step 5: 提交**

```bash
git add src/value.rs
git commit -m "fix(value): remove redundant if-else in all_paths_internal"
```

---

## Task 2: 修复 `type_complexity` 添加类型别名

**Files:**
- Modify: `src/dynamic.rs`
- Test: `src/dynamic.rs` (现有测试)

**Step 1: 添加类型别名**

在 `src/dynamic.rs` 文件顶部（约第 22 行后）添加：

```rust
/// Callback storage for dynamic field change notifications.
type CallbackStorage<T> = Arc<RwLock<HashMap<CallbackId, Box<dyn Fn(&T) + Send + Sync>>>>;
```

**Step 2: 更新 DynamicField 结构体**

将第 43 行：
```rust
callbacks: Arc<RwLock<HashMap<CallbackId, Box<dyn Fn(&T) + Send + Sync>>>>,
```

修改为：
```rust
callbacks: CallbackStorage<T>,
```

**Step 3: 更新 CallbackGuard 结构体**

将第 137 行：
```rust
callbacks: Arc<RwLock<HashMap<CallbackId, Box<dyn Fn(&T) + Send + Sync>>>>,
```

修改为：
```rust
callbacks: CallbackStorage<T>,
```

**Step 4: 运行 clippy 验证**

```bash
cargo clippy --all-features
```

期望：`type_complexity` 警告消失

**Step 5: 运行测试验证**

```bash
cargo test --all-features
```

期望：所有测试通过

**Step 6: 提交**

```bash
git add src/dynamic.rs
git commit -m "refactor(dynamic): add CallbackStorage type alias to reduce complexity"
```

---

## Task 3: 重命名 `SourceChain::add` 为 `push`

**Files:**
- Modify: `src/config/chain.rs:53` (方法定义)
- Modify: `src/config/chain.rs` (测试代码，约7处)

**Step 1: 重命名方法定义**

将 `src/config/chain.rs` 第 53 行：
```rust
pub fn add(mut self, source: Box<dyn Source>) -> Self {
```

修改为：
```rust
pub fn push(mut self, source: Box<dyn Source>) -> Self {
```

**Step 2: 更新测试代码**

在 `src/config/chain.rs` 测试模块中，将所有 `.add(` 调用替换为 `.push(`：

```bash
# 使用编辑工具替换测试代码中的调用
# 约 307, 318, 322, 351, 352, 362, 365 行
```

**Step 3: 更新 SourceChainBuilder**

将第 207 行：
```rust
self.chain = self.chain.add(source);
```

修改为：
```rust
self.chain = self.chain.push(source);
```

**Step 4: 运行 clippy 验证**

```bash
cargo clippy --all-features
```

期望：`should_implement_trait` 警告消失

**Step 5: 运行测试验证**

```bash
cargo test --all-features
```

期望：所有测试通过

**Step 6: 提交**

```bash
git add src/config/chain.rs
git commit -m "refactor(chain): rename add to push for better clarity"
```

---

## Task 4: 为 `ConflictReport` 添加 Builder

**Files:**
- Modify: `src/value.rs`
- Test: `src/value.rs` (更新现有测试)

**Step 1: 添加 ConflictReportBuilder 结构体**

在 `ConflictReport` impl 块之后（约第 892 行后）添加：

```rust
/// Builder for creating ConflictReport instances.
#[derive(Debug, Clone, Default)]
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
    /// Set the path of the conflicting value.
    pub fn path(mut self, path: impl Into<Arc<str>>) -> Self {
        self.path = Some(path.into());
        self
    }

    /// Set the low priority value and its source.
    pub fn low_value(mut self, value: String, source: SourceId) -> Self {
        self.low_value = Some(value);
        self.low_source = Some(source);
        self
    }

    /// Set the low priority value location.
    pub fn low_location(mut self, loc: SourceLocation) -> Self {
        self.low_location = Some(loc);
        self
    }

    /// Set the high priority value and its source.
    pub fn high_value(mut self, value: String, source: SourceId) -> Self {
        self.high_value = Some(value);
        self.high_source = Some(source);
        self
    }

    /// Set the high priority value location.
    pub fn high_location(mut self, loc: SourceLocation) -> Self {
        self.high_location = Some(loc);
        self
    }

    /// Set which value won.
    pub fn winner(mut self, winner: ConflictWinner) -> Self {
        self.winner = Some(winner);
        self
    }

    /// Build the ConflictReport.
    ///
    /// # Panics
    ///
    /// Panics if required fields (path, values, sources, winner) are not set.
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

impl ConflictReport {
    /// Create a builder for ConflictReport.
    pub fn builder() -> ConflictReportBuilder {
        ConflictReportBuilder::default()
    }
}
```

**Step 2: 更新测试代码**

在 `src/value.rs` 测试模块中（约第 1001 行），将：

```rust
let report = ConflictReport::new(
    "database.host",
    "localhost".to_string(),
    SourceId::new("file1"),
    None,
    "127.0.0.1".to_string(),
    SourceId::new("file2"),
    None,
    ConflictWinner::High,
);
```

修改为：

```rust
let report = ConflictReport::builder()
    .path("database.host")
    .low_value("localhost".to_string(), SourceId::new("file1"))
    .high_value("127.0.0.1".to_string(), SourceId::new("file2"))
    .winner(ConflictWinner::High)
    .build();
```

**Step 3: 运行 clippy 验证**

```bash
cargo clippy --all-features
```

期望：`too_many_arguments` 警告消失

**Step 4: 运行测试验证**

```bash
cargo test --all-features
```

期望：所有测试通过

**Step 5: 提交**

```bash
git add src/value.rs
git commit -m "refactor(value): add ConflictReportBuilder pattern"
```

---

## Task 5: 最终验证

**Step 1: 运行完整 clippy 检查**

```bash
cargo clippy --all-features
```

期望：**0 warnings**

**Step 2: 运行所有测试**

```bash
cargo test --all-features
```

期望：所有测试通过

**Step 3: 检查代码格式**

```bash
cargo fmt --all -- --check
```

期望：无格式问题

**Step 4: 编译检查**

```bash
cargo check --all-features
```

期望：编译成功，无警告

**Step 5: 提交总结**

```bash
git add -A
git commit -m "chore: resolve all remaining clippy warnings

- Fixed if_same_then_else logic error in value.rs
- Added CallbackStorage type alias in dynamic.rs
- Renamed SourceChain::add to push
- Added ConflictReportBuilder pattern

All clippy warnings resolved. All tests passing."
```

---

## 任务状态

- [x] Task 1: 修复 `if_same_then_else`
- [x] Task 2: 修复 `type_complexity`
- [x] Task 3: 重命名 `add` 为 `push`
- [x] Task 4: 添加 `ConflictReportBuilder`
- [x] Task 5: 最终验证
