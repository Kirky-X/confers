# Tasks: 文档内容优化

**执行顺序：** D → B → A → C

**总时间估计：** 9-14 小时

---

## 阶段 D: 整体一致性检查

**目标：** 建立 Cargo.toml features 与文档的完整映射，生成一致性报告。

### Task D.1: 提取 Cargo.toml Features

**Files:**
- Create: `spec/changes/optimize-documentation/FEATURE_MATRIX.md`

**Step 1: 编写提取脚本**

```bash
cat > extract_features.sh <<'EOF'
#!/bin/bash
echo "=== Confers Feature Matrix ==="
echo ""
echo "## Single Features"
grep -A 50 "^# Format support" ../../Cargo.toml | grep "^.*= \[" | grep -v "^#" | grep -v "^default"
echo ""
echo "## Feature Presets"
grep -A 10 "^# Feature presets" ../../Cargo.toml | grep "^.*= \[" | grep -v "^#"
EOF
```

**Step 2: 运行提取**

```bash
cd spec/changes/optimize-documentation
bash extract_features.sh > FEATURE_MATRIX.raw
```

**Expected:** 生成包含所有 features 的原始列表

**Step 3: 整理成表格**

创建 `FEATURE_MATRIX.md`，包含：
- 单独特性列表
- 特性预设定义
- 依赖关系

**Step 4: 验证完整性**

```bash
# 确保所有 features 都被记录
grep -c "^\w\+=" FEATURE_MATRIX.raw
# 预期: 40+ features
```

**Step 5: 提交**

```bash
git add -f spec/changes/optimize-documentation/FEATURE_MATRIX.md
git commit -m "docs(extract): add Cargo.toml feature matrix"
```

---

### Task D.2: 对比文档与代码

**Files:**
- Create: `spec/changes/optimize-documentation/CONSISTENCY_REPORT.md`
- Modify: `docs/API_REFERENCE.md` (标记差异位置)

**Step 1: 扫描文档特性列表**

```bash
# 提取文档中提到的 features
grep -E "\`(minimal|recommended|dev|production|full|derive|validation|watch|encryption|cli|schema|monitoring|remote|typescript|security|key|tracing)\`" docs/API_REFERENCE.md | sort -u
```

**Step 2: 生成差异报告**

创建 `CONSISTENCY_REPORT.md`，包含：

| 类型 | 名称 | 位置 | 状态 |
|------|------|------|------|
| 文档有，代码没有 | `monitoring` | API_REFERENCE.md:99 | ❌ 需删除 |
| 文档有，代码没有 | `hocon` | API_REFERENCE.md:102 | ❌ 需删除 |
| 文档有，代码没有 | `derive` | API_REFERENCE.md:92 | ❌ 需修正 |
| 代码有，文档没有 | `typescript-schema` | Cargo.toml:109 | ❌ 需添加 |
| 代码有，文档没有 | `security` | Cargo.toml:110 | ❌ 需添加 |
| 代码有，文档没有 | `key` | Cargo.toml:111 | ❌ 需添加 |
| 代码有，文档没有 | `tracing` | Cargo.toml:112 | ❌ 需添加 |

**Step 3: 验证报告**

确保：
- 每个 Cargo.toml 中的 feature 都在报告中
- 差异分类准确

**Step 4: 提交**

```bash
git add -f spec/changes/optimize-documentation/CONSISTENCY_REPORT.md
git commit -m "docs(analyze): add feature consistency report"
```

---

### Task D.3: 修复代码文档警告

**Files:**
- Modify: `src/merger/strategy.rs:11`
- Modify: `src/dynamic.rs:10,31,41`

**Step 1: 验证警告存在**

```bash
cargo doc --all-features --no-deps 2>&1 | grep -E "(warning|error)"
```

**Expected:** 看到 9 个警告

**Step 2: 修复 merger/strategy.rs 文档链接**

在 `src/merger/strategy.rs:11-13` 修复：
```rust
// Before:
/// See [`low`](low) and [`high`](high) for details.

// After:
/// See the `low` and `high` modules for details.
```

**Step 3: 修复 dynamic.rs HTML 标签**

在 `src/dynamic.rs` 修复未闭合的 HTML 标签：
```rust
// Before:
/// A vector of items, e.g., `Vec<String>`

// After:
/// A vector of items, e.g., `Vec<String>` (no HTML tags)
```

**Step 4: 验证修复**

```bash
cargo doc --all-features --no-deps 2>&1 | grep -c "warning"
# Expected: 0 warnings
```

**Step 5: 提交**

```bash
git add src/merger/strategy.rs src/dynamic.rs
git commit -m "fix(docs): resolve all cargo doc warnings"
```

---

## 阶段 B: 版本同步更新

**目标：** 创建 CHANGELOG 0.3.0 条目，记录版本变更。

### Task B.1: 创建 CHANGELOG 0.3.0 条目

**Files:**
- Modify: `docs/CHANGELOG.md` (在文件开头添加)

**Step 1: 备份当前 CHANGELOG**

```bash
cp docs/CHANGELOG.md docs/CHANGELOG.md.backup
```

**Step 2: 添加 0.3.0 条目**

在 `docs/CHANGELOG.md` 第 7 行（`## [0.2.2]` 之前）添加：

```markdown
## [0.3.0] - 2026-03-04

### 🎉 新增功能

#### TypeScript Schema 生成
- 添加 `typescript-schema` feature
- 支持从 Rust 类型生成 TypeScript 类型定义
- API: `confers::schema::generate_typescript::<T>()`

#### 密钥管理系统 (key feature)
- `KeyManager`: 密钥生命周期管理
- `KeyStorage`: 加密密钥存储（XChaCha20）
- `KeyRotationService`: 自动密钥轮换
- 密钥元数据和版本管理

#### 安全模块 (security feature)
- `EnvSecurityValidator`: 环境变量安全验证
- `ErrorSanitizer`: 错误信息敏感数据脱敏
- `ConfigInjector`: 安全配置注入
- `SecureString`: 自动清零的安全字符串

#### CLI 增强命令
- `confers generate`: 配置模板生成
- `confers wizard`: 交互式配置向导
- `confers completions`: Shell 补全生成

#### 并行验证
- 添加 `parallel` feature (rayon)
- 大型配置验证性能提升

### 🔧 改进
- 更新特性预设定义 (minimal, recommended, dev, production, full, distributed)
- 优化 XChaCha20 加密性能
- 改进错误信息可读性

### 🐛 修复
- 修复 `progressive-reload` feature gating (缺少 `async_trait`)
- 修复 `encryption` feature 测试 panic

### 🔒 安全
- 密钥管理使用 HKDF 密钥派生
- 环境变量注入防护增强
- 添加路径遍历保护

### 📊 统计
- 新增文件: 17
- 新增代码: 6,855 行
- 新增 features: 4 (typescript-schema, security, key, parallel)

```

**Step 3: 验证格式**

```bash
# 检查 Markdown 格式
grep -E "^## \[0\." docs/CHANGELOG.md | head -5
# Expected:
# ## [0.3.0] - 2026-03-04
# ## [0.2.2] - 2026-01-25
```

**Step 4: 验证内容完整性**

确保记录了所有 merge commit 的变更：
```bash
git show 6c88ad8 --stat | grep "confers-cli/src/commands"
git show 6c88ad8 --stat | grep "src/key"
git show 6c88ad8 --stat | grep "src/security"
```

**Step 5: 提交**

```bash
git add docs/CHANGELOG.md
git commit -m "docs(changelog): add v0.3.0 release notes"
```

---

## 阶段 A: 补充新功能文档

**目标：** 为新迁移的功能模块补充完整的 API 文档和使用指南。

### Task A.1: API_REFERENCE.md - 密钥管理章节

**Files:**
- Modify: `docs/API_REFERENCE.md` (在适当位置添加新章节)

**Step 1: 验证 API 存在**

```bash
# 确认 KeyManager 等类型已导出
grep -r "pub use.*key::" src/lib.rs
# Expected: KeyManager, KeyRotationService, KeyStorage, etc.
```

**Step 2: 添加密钥管理章节**

在 `docs/API_REFERENCE.md` 中添加（在"加密功能"章节之后）：

```markdown
### 密钥管理

`key` feature 提供完整的密钥生命周期管理。

#### KeyManager

密钥生命周期管理的核心组件。

```rust
use confers::key::KeyManager;

// 创建密钥管理器
let master_key = [0u8; 32];
let mut manager = KeyManager::new(master_key);

// 创建新密钥
let key = manager.create_key("database", Some("DB encryption key".to_string()))?;

// 获取密钥
let key_data = manager.get_key("database")?;

// 列出所有密钥
let keys = manager.list_keys();

// 撤销密钥
manager.revoke_key("database")?;
```

#### 方法

| 方法 | 参数 | 返回值 | 描述 |
|------|------|--------|------|
| `new(master_key)` | `&[u8; 32]` | `Self` | 创建密钥管理器 |
| `create_key(id, desc)` | `&str, Option<String>` | `Result<KeyBundle>` | 创建新密钥 |
| `get_key(id)` | `&str` | `Option<&KeyBundle>` | 获取密钥 |
| `list_keys()` | - | `Vec<KeyInfo>` | 列出所有密钥 |
| `revoke_key(id)` | `&str` | `Result<()>` | 撤销密钥 |

#### KeyRotationService

自动密钥轮换服务。

```rust
use confers::key::{KeyRotationService, KeyRotationPolicy};

let policy = KeyRotationPolicy::default()
    .with_max_age(Duration::from_secs(86400 * 90)); // 90天

let service = KeyRotationService::new(manager, policy);
service.check_and_rotate()?;
```

#### KeyStorage

加密密钥持久化存储。

```rust
use confers::key::KeyStorage;

let storage = KeyStorage::new("/path/to/keys")?;
storage.save(&key_bundle)?;
let loaded = storage.load("key_id")?;
```
```

**Step 3: 验证代码示例**

```bash
# 提取代码块并测试
cargo test --doc key:: 2>&1 | grep -E "(test result|running)"
```

**Step 4: 提交**

```bash
git add docs/API_REFERENCE.md
git commit -m "docs(api): add key management API documentation"
```

---

### Task A.2: API_REFERENCE.md - 安全模块章节

**Files:**
- Modify: `docs/API_REFERENCE.md`

**Step 1: 验证 API 存在**

```bash
grep -r "pub use.*security::" src/lib.rs
```

**Step 2: 添加安全模块章节**

```markdown
### 安全模块

`security` feature 提供环境变量验证、错误脱敏和安全注入功能。

#### EnvSecurityValidator

环境变量安全验证器，防止注入攻击。

```rust
use confers::security::EnvSecurityValidator;

let validator = EnvSecurityValidator::builder()
    .allow_pattern(r"^[A-Z][A-Z0-9_]*$")
    .block_pattern(r".*_SECRET$")
    .block_pattern(r".*_PASSWORD$")
    .build()?;

// 验证单个变量
validator.validate_var("APP_NAME")?;
validator.validate_var("DB_PASSWORD")?; // Err: 包含 _SECRET

// 验证所有环境变量
validator.validate_all(std::env::vars())?;
```

#### ErrorSanitizer

错误信息敏感数据脱敏。

```rust
use confers::security::ErrorSanitizer;

let sanitizer = ErrorSanitizer::default();
let clean_msg = sanitizer.sanitize(&error_msg);
```

#### ConfigInjector

安全配置注入器。

```rust
use confers::security::ConfigInjector;

let injector = ConfigInjector::new()
    .with_validator(validator)
    .inject(&config)?;
```
```

**Step 3: 提交**

```bash
git add docs/API_REFERENCE.md
git commit -m "docs(api): add security module API documentation"
```

---

### Task A.3: API_REFERENCE.md - TypeScript Schema 章节

**Files:**
- Modify: `docs/API_REFERENCE.md`

**Step 1: 验证 API 存在**

```bash
grep -r "pub use.*schema::" src/lib.rs
```

**Step 2: 添加 TypeScript Schema 章节**

```markdown
### TypeScript Schema 生成

`typescript-schema` feature 支持从 Rust 类型生成 TypeScript 定义。

#### generate_typescript

生成 TypeScript 类型定义。

```rust
use confers::schema::generate_typescript;

#[derive(confers::Config)]
pub struct AppConfig {
    pub name: String,
    pub port: u16,
    pub debug: bool,
}

let ts = generate_typescript::<AppConfig>();
println!("{}", ts);
```

**输出：**

```typescript
// Auto-generated from Rust
export interface AppConfig {
  name: string;
  port: number;
  debug: boolean;
}
```

#### 使用场景

- 前后端类型共享
- API 契约生成
- 配置文件 IDE 支持
```

**Step 3: 提交**

```bash
git add docs/API_REFERENCE.md
git commit -m "docs(api): add TypeScript schema generation documentation"
```

---

### Task A.4: USER_GUIDE.md - 使用示例

**Files:**
- Modify: `docs/USER_GUIDE.md`

**Step 1: 添加密钥管理示例**

在"高级用法"章节添加：

```markdown
### 密钥管理

confers 提供完整的密钥生命周期管理：

```rust
use confers::key::KeyManager;
use confers::secret::XChaCha20Crypto;

// 1. 创建密钥管理器
let master_key = std::env::var("MASTER_KEY")?
    .as_bytes()
    .try_into()
    .map_err(|_| ConfigError::InvalidKeyLength)?;

let mut manager = KeyManager::new(master_key);

// 2. 创建用于数据库加密的密钥
let db_key = manager.create_key(
    "database",
    Some("Production database encryption key".to_string())
)?;

// 3. 获取密钥进行加密
let key_data = manager.get_key("database")?;
let crypto = XChaCha20Crypto::new();
let (nonce, ciphertext) = crypto.encrypt(
    b"sensitive data",
    &key_data.key_bytes()
)?;
```
```

**Step 2: 添加安全配置示例**

```markdown
### 安全配置

使用环境变量验证防止注入攻击：

```rust
use confers::security::EnvSecurityValidator;

// 配置验证器
let validator = EnvSecurityValidator::builder()
    .allow_pattern(r"^[A-Z][A-Z0-9_]*$")      // 只允许大写字母、数字、下划线
    .block_pattern(r".*_SECRET$")             // 禁止 _SECRET 结尾
    .block_pattern(r".*_PASSWORD$")           // 禁止 _PASSWORD 结尾
    .block_pattern(r".*[;<>&|`$].*")          // 禁止 shell 元字符
    .build()?;

// 加载配置前验证
validator.validate_all(std::env::vars())?;

let config = AppConfig::load()?;
```
```

**Step 3: 提交**

```bash
git add docs/USER_GUIDE.md
git commit -m "docs(guide): add security and key management examples"
```

---

## 阶段 C: API 文档准确性修正

**目标：** 修正特性列表和预设，确保与 Cargo.toml 完全一致。

### Task C.1: 修正特性表格

**Files:**
- Modify: `docs/API_REFERENCE.md` (特性列表部分，约第 88-103 行)

**Step 1: 备份当前表格**

```bash
# 定位特性表格
grep -n "单独特性" docs/API_REFERENCE.md
```

**Step 2: 重写特性表格**

替换为准确的特性列表：

```markdown
**单独特性：**

| 特性 | 描述 | 默认 |
|------|------|------|
| **格式支持** |||
| `toml` | TOML 格式支持 | ✅ |
| `json` | JSON 格式支持 | ✅ |
| `yaml` | YAML 格式支持 | ❌ |
| `ini` | INI 格式支持 | ❌ |
| `env` | 环境变量支持 | ✅ |
| **核心功能** |||
| `validation` | 基于 garde 的配置验证 | ✅ |
| `watch` | 文件监控与热重载 | ❌ |
| `encryption` | XChaCha20 加密 | ❌ |
| `cli` | 命令行集成 | ❌ |
| `schema` | JSON Schema 生成 | ❌ |
| `typescript-schema` | TypeScript 类型生成 | ❌ |
| `parallel` | 并行验证 (rayon) | ❌ |
| **安全** |||
| `security` | 安全模块 (验证、脱敏、注入) | ❌ |
| `key` | 密钥管理系统 | ❌ |
| **高级功能** |||
| `audit` | 审计日志 | ❌ |
| `metrics` | 指标收集 | ❌ |
| `dynamic` | 动态字段 | ❌ |
| `progressive-reload` | 渐进式部署 | ❌ |
| `migration` | 配置迁移 | ❌ |
| `snapshot` | 快照回滚 | ❌ |
| `profile` | 环境配置 | ❌ |
| `interpolation` | 变量插值 | ❌ |
| `tracing` | 分布式追踪 | ❌ |
| **远程源** |||
| `remote` | HTTP 轮询 | ❌ |
| `etcd` | Etcd 集成 | ❌ |
| `consul` | Consul 集成 | ❌ |
| `cache-redis` | Redis 缓存 | ❌ |
| **其他** |||
| `config-bus` | 配置事件总线 | ❌ |
| `context-aware` | 上下文感知配置 | ❌ |
| `modules` | 模块化配置 | ❌ |
```

**Step 3: 验证与 Cargo.toml 一致**

```bash
# 提取文档中的特性
grep -E "^\`\w+\`" docs/API_REFERENCE.md | wc -l
# 预期: ~30 个特性

# 对比 Cargo.toml
grep -E "^\w+\s*=" Cargo.toml | grep -v "^default" | wc -l
# 预期: 数量匹配
```

**Step 4: 提交**

```bash
git add docs/API_REFERENCE.md
git commit -m "docs(api): correct feature list to match Cargo.toml"
```

---

### Task C.2: 修正预设特性列表

**Files:**
- Modify: `docs/API_REFERENCE.md` (预设特性部分，约第 80-86 行)

**Step 1: 提取实际预设**

```bash
# 从 Cargo.toml 提取预设定义
grep -A 2 "^minimal\|^recommended\|^dev\|^production\|^full\|^distributed" Cargo.toml | grep "="
```

**Step 2: 更新预设表格**

替换为：

```markdown
**特性预设：**

| 预设 | 特性 | 使用场景 |
|------|------|----------|
| <span style="color:#166534">minimal</span> | `env` | 最小依赖（仅环境变量） |
| <span style="color:#1E40AF">recommended</span> | `toml` + `env` + `validation` | 推荐大多数应用 |
| <span style="color:#92400E">dev</span> | `toml` + `json` + `yaml` + `env` + `cli` + `validation` + `schema` + `audit` + `profile` + `watch` + `migration` + `snapshot` + `dynamic` | 开发配置 |
| <span style="color:#991B1B">production</span> | `toml` + `env` + `watch` + `encryption` + `validation` + `audit` + `profile` + `metrics` + `schema` + `cli` + `migration` + `dynamic` + `progressive-reload` + `snapshot` | 生产配置 |
| <span style="color:#5B21B6">full</span> | 所有特性 | 完整功能集 |
| <span style="color:#0891b2">distributed</span> | `toml` + `env` + `watch` + `validation` + `config-bus` + `progressive-reload` + `metrics` + `audit` | 分布式系统 |
```

**Step 3: 验证**

```bash
# 检查 Markdown 格式
grep -A 1 "^\| <span" docs/API_REFERENCE.md | grep "^\|"
```

**Step 4: 提交**

```bash
git add docs/API_REFERENCE.md
git commit -m "docs(api): update feature presets to match Cargo.toml"
```

---

### Task C.3: 最终验证

**Files:**
- All modified files

**Step 1: 运行文档测试**

```bash
cargo test --doc --all-features
```

**Expected:** 所有测试通过

**Step 2: 生成文档并检查警告**

```bash
cargo doc --all-features --no-deps 2>&1 | grep -E "(warning|error)"
```

**Expected:** 0 warnings, 0 errors

**Step 3: 验证链接**

```bash
# 检查文档内部链接
grep -r "](.*.md)" docs/ | while read line; do
    link=$(echo "$line" | sed 's/.*](\(.*\.md\)).*/\1/')
    if [ ! -z "$link" ] && [ ! -f "docs/$link" ] && [ ! -f "$link" ]; then
        echo "Broken link: $link in $line"
    fi
done
```

**Expected:** 无损坏链接

**Step 4: 构建验证**

```bash
cargo build --all-features
```

**Expected:** 编译成功

**Step 5: 最终提交**

```bash
git add docs/
git commit -m "docs: complete documentation optimization for v0.3.0

All documentation now aligned with v0.3.0 code implementation:

- CHANGELOG updated with 0.3.0 release notes
- API_REFERENCE includes all new features (key, security, typescript-schema)
- Feature lists match Cargo.toml exactly
- All code examples verified
- Zero cargo doc warnings"
```

---

## 完成状态

- [ ] Task D.1: 提取 Cargo.toml Features
- [ ] Task D.2: 对比文档与代码
- [ ] Task D.3: 修复代码文档警告
- [ ] Task B.1: 创建 CHANGELOG 0.3.0 条目
- [ ] Task A.1: API_REFERENCE.md - 密钥管理章节
- [ ] Task A.2: API_REFERENCE.md - 安全模块章节
- [ ] Task A.3: API_REFERENCE.md - TypeScript Schema 章节
- [ ] Task A.4: USER_GUIDE.md - 使用示例
- [ ] Task C.1: 修正特性表格
- [ ] Task C.2: 修正预设特性列表
- [ ] Task C.3: 最终验证
