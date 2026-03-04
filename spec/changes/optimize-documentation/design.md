# Design: 文档内容优化

## 概述

根据实际代码情况，对 `docs/*.md` 文档进行全面优化，确保文档与 v0.3.0 版本的代码实现保持一致。

**优先级：** D → B → A → C

---

## 问题分析

### 发现的主要问题

| # | 问题 | 影响 | 位置 |
|---|------|------|------|
| 1 | **CHANGELOG 缺少 0.3.0 条目** | 用户不知道新版本变化 | docs/CHANGELOG.md |
| 2 | **特性列表与 Cargo.toml 不一致** | `monitoring` 不存在，缺少 `typescript-schema`, `security`, `key`, `tracing` | docs/API_REFERENCE.md |
| 3 | **新模块无 API 文档** | 6855 行新代码无文档记录 | docs/API_REFERENCE.md |
| 4 | **derive 特性描述不准确** | 不是独立 feature，由宏提供 | docs/API_REFERENCE.md |
| 5 | **预设特性列表过时** | `dev`, `production`, `full` 与实际不匹配 | docs/API_REFERENCE.md |
| 6 | **代码文档警告未修复** | 9 个文档警告 | src/merger/strategy.rs, src/dynamic.rs |

### 版本差异

```
当前代码版本: 0.3.0
文档记录版本: 0.2.2 (2026-01-25)
```

### 新增代码统计

从 `migrate-from-target-project` 分支合并后新增：

```
17 files changed, 6855 insertions(+)
- src/key/          - 密钥管理系统 (~2,000 行)
- src/security/     - 安全模块 (~3,600 行)
- src/schema/       - TypeScript Schema 生成 (~300 行)
- confers-cli/      - CLI 增强命令 (~800 行)
```

---

## 架构设计

### 四阶段优化流程

```
┌─────────────────────────────────────────────────────┐
│              D. 整体一致性检查                        │
├─────────────────────────────────────────────────────┤
│  建立 Feature 映射表 → 生成一致性报告 → 创建修复清单  │
└─────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────┐
│              B. 版本同步更新 (0.2.2 → 0.3.0)          │
├─────────────────────────────────────────────────────┤
│  创建 CHANGELOG 0.3.0 条目 → 记录新增功能 → 更新指南  │
└─────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────┐
│           A. 补充新功能文档                           │
├─────────────────────────────────────────────────────┤
│  API_REFERENCE.md 新增章节 → USER_GUIDE.md 新增示例  │
└─────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────┐
│          C. API 文档准确性修正                        │
├─────────────────────────────────────────────────────┤
│  修正特性表格 → 更新预设列表 → 验证示例代码           │
└─────────────────────────────────────────────────────┘
```

---

## 阶段 D: 整体一致性检查

### 目标
建立 Cargo.toml features 与文档的完整映射，生成一致性报告。

### 任务清单

#### D.1: 提取实际 Features
- 读取 `Cargo.toml` 中所有 `[features]` 定义
- 整理单独特性、特性预设、依赖关系
- 生成 Feature 矩阵表格

#### D.2: 文档对比分析
- 扫描 `docs/API_REFERENCE.md` 中的特性列表
- 扫描 `docs/USER_GUIDE.md` 中的特性描述
- 标记差异点：
  - 文档有但代码没有的 feature
  - 代码有但文档缺失的 feature
  - 描述不准确的 feature

#### D.3: 代码覆盖检查
- 检查所有 `#[cfg(feature = "...")]` 代码
- 确认每个 feature 的文档覆盖情况
- 识别无文档的公共 API

#### D.4: 文档警告修复
修复 `cargo doc` 生成的 9 个警告：
```rust
// src/merger/strategy.rs
// 修复未解析的文档链接: low, high

// src/dynamic.rs
// 修复未闭合的 HTML 标签: Vec, T, RwLock
```

### 输出物
- `FEATURE_MATRIX.md` - 特性对照表
- `CONSISTENCY_REPORT.md` - 一致性检查报告
- 修复后的代码（无文档警告）

### 验收标准
- [ ] `cargo doc --all-features` 无警告
- [ ] Feature 矩阵与 Cargo.toml 100% 一致
- [ ] 所有公共 API 有文档覆盖

---

## 阶段 B: 版本同步更新

### 目标
创建 CHANGELOG 0.3.0 条目，记录版本变更。

### CHANGELOG 结构

```markdown
## [0.3.0] - 2026-03-04

### 🎉 新增功能

#### TypeScript Schema 生成
- 添加 `typescript-schema` feature
- 支持从 Rust 类型生成 TypeScript 类型定义
- API: `confers::schema::generate_typescript()`

#### 密钥管理系统 (key feature)
- KeyManager: 密钥生命周期管理
- KeyStorage: 加密密钥存储
- KeyRotationService: 自动密钥轮换
- 支持 XChaCha20 加密

#### 安全模块 (security feature)
- EnvSecurityValidator: 环境变量安全验证
- ErrorSanitizer: 错误信息敏感数据脱敏
- ConfigInjector: 安全配置注入
- SecureString: 自动清零的安全字符串

#### CLI 增强命令
- `confers generate`: 配置模板生成
- `confers wizard`: 交互式配置向导
- `confers completions`: Shell 补全生成

#### 并行验证
- 添加 `parallel` feature (rayon)
- 大型配置验证性能提升

### 🔧 改进
- 更新特性预设定义
- 优化 XChaCha20 加密性能
- 改进错误信息可读性

### 🐛 修复
- 修复 progressive-reload feature gating
- 修复 encryption feature 测试

### 🔒 安全
- 密钥管理使用 HKDF 密钥派生
- 环境变量注入防护增强

### 📊 统计
- 新增文件: 17
- 新增代码: 6,855 行
- 新增 features: 4 (typescript-schema, security, key, parallel)
```

### 任务清单
- [ ] 创建 CHANGELOG 0.3.0 条目
- [ ] 分类所有变更（新增/改进/修复/安全）
- [ ] 添加迁移指南（如有破坏性变更）
- [ ] 更新版本对比表

### 验收标准
- [ ] CHANGELOG 符合 Keep a Changelog 格式
- [ ] 所有 merge 提交的变更都被记录
- [ ] 无破坏性变更遗漏

---

## 阶段 A: 补充新功能文档

### 目标
为新迁移的功能模块补充完整的 API 文档和使用指南。

### A.1: API_REFERENCE.md 新增章节

#### 密钥管理 API 章节
```markdown
## 密钥管理 (key feature)

### KeyManager
密钥生命周期管理的核心组件。

#### 方法
- `new(master_key: &[u8; 32]) -> Self`
- `create_key(&mut self, key_id: &str, description: Option<String>) -> Result<KeyBundle>`
- `get_key(&self, key_id: &str) -> Option<&KeyBundle>`
- `list_keys(&self) -> Vec<KeyInfo>`
- `revoke_key(&mut self, key_id: &str) -> Result<()>`

#### 示例
\`\`\`rust
use confers::key::KeyManager;

let master_key = [0u8; 32];
let mut manager = KeyManager::new(master_key);

let key = manager.create_key("database", Some("DB encryption key".to_string()))?;
\`\`\`

### KeyRotationService
自动密钥轮换服务。

### KeyStorage
加密密钥持久化存储。
```

#### 安全模块 API 章节
```markdown
## 安全模块 (security feature)

### EnvSecurityValidator
环境变量安全验证器。

### ErrorSanitizer
错误信息敏感数据脱敏。

### ConfigInjector
安全配置注入器。

### SecureString
自动清零的安全字符串。
```

#### TypeScript Schema 生成章节
```markdown
## TypeScript Schema 生成 (typescript-schema feature)

从 Rust 类型生成 TypeScript 定义。

### 方法
- `generate_typescript<T>() -> String`

### 示例
\`\`\`rust
use confers::schema;

#[derive(Config)]
pub struct Config {
    pub name: String,
    pub port: u16,
}

let ts = schema::generate_typescript::<Config>();
\`\`\`
```

#### CLI 命令章节
```markdown
## CLI 命令

### generate
生成配置模板。

### wizard
交互式配置向导。

### completions
生成 Shell 补全脚本。
```

### A.2: USER_GUIDE.md 新增示例

#### 密钥管理使用示例
```markdown
### 密钥管理

confers 提供完整的密钥生命周期管理：

\`\`\`rust
use confers::key::KeyManager;
use confers::secret::XChaCha20Crypto;

// 创建密钥管理器
let master_key = [0u8; 32];
let mut manager = KeyManager::new(master_key);

// 创建新密钥
let key = manager.create_key("api_key", Some("API 加密密钥".to_string()))?;

// 获取密钥用于加密
let key_data = manager.get_key("api_key")?;
\`\`\`
```

#### 安全配置示例
```markdown
### 安全配置

使用环境变量验证：

\`\`\`rust
use confers::security::EnvSecurityValidator;

let validator = EnvSecurityValidator::builder()
    .allow_pattern(r"^[A-Z][A-Z0-9_]*$")
    .block_pattern(r".*_SECRET$")
    .build()?;

validator.validate_var("APP_NAME")?;
validator.validate_var("DB_PASSWORD")?; // Err: 包含 _SECRET
\`\`\`
```

#### TypeScript 集成示例
```markdown
### TypeScript 前端集成

生成前端类型定义：

\`\`\`typescript
// 自动生成
export interface AppConfig {
  name: string;
  port: number;
  debug: boolean;
}
\`\`\`
```

### A.3: 可选的新文档

- `KEY_MANAGEMENT.md` - 密钥管理详细指南
- `SECURITY_GUIDE.md` - 安全配置完整指南
- `TYPESCRIPT_INTEGRATION.md` - TypeScript 集成指南

### 任务清单
- [ ] API_REFERENCE.md 新增 4 个章节
- [ ] USER_GUIDE.md 新增 3 个示例
- [ ] 验证所有代码示例可编译
- [ ] 添加 cross-reference 链接

### 验收标准
- [ ] 所有新增公共 API 有文档
- [ ] 每个功能至少有 1 个完整示例
- [ ] 文档内部链接正确

---

## 阶段 C: API 文档准确性修正

### 目标
修正特性列表和预设，确保与 Cargo.toml 完全一致。

### C.1: 特性表格修正

#### 删除的条目
```markdown
- derive (不是独立 feature，由宏提供)
- monitoring (不存在)
- hocon (未实现)
```

#### 新增的条目
```markdown
+ typescript-schema - TypeScript 类型生成
+ security - 安全功能
+ key - 密钥管理
+ tracing - 分布式追踪
+ progressive-reload - 渐进式部署
+ config-bus - 配置事件总线
+ context-aware - 上下文感知
+ modules - 模块化配置
+ interpolation - 变量插值
```

### C.2: 预设特性修正

```markdown
| 预设 | 实际 Cargo.toml 定义 |
|------|---------------------|
| minimal | env |
| recommended | toml + env + validation |
| dev | toml + json + yaml + env + cli + validation + schema + audit + profile + watch + migration + snapshot + dynamic |
| production | toml + env + watch + encryption + validation + audit + profile + metrics + schema + cli + migration + dynamic + progressive-reload + snapshot |
| full | toml + json + yaml + env + cli + validation + watch + encryption + schema + metrics + dynamic + progressive-reload + audit + migration + snapshot + profile + interpolation + hot-reload + remote + config-bus + context-aware + modules + etcd + consul + cache-redis |
| distributed | toml + env + watch + validation + config-bus + progressive-reload + metrics + audit |
```

### C.3: API 示例验证

```bash
# 验证所有文档中的代码示例
cargo test --doc

# 验证文档编译
cargo doc --all-features --no-deps
```

### 任务清单
- [ ] 修正 API_REFERENCE.md 特性表格
- [ ] 更新预设特性列表
- [ ] 验证所有代码示例
- [ ] 修复文档交叉引用

### 验收标准
- [ ] 特性列表与 Cargo.toml 100% 一致
- [ ] 所有代码示例通过 `cargo test --doc`
- [ ] 无文档警告

---

## 数据流

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│ Cargo.toml  │────▶│ Feature 矩阵│────▶│ 文档更新    │
│ (实际定义)  │     │ (映射表)    │     │ (最终输出)  │
└─────────────┘     └─────────────┘     └─────────────┘
       │                                      ▲
       │                                      │
       ▼                                      │
┌─────────────┐     ┌─────────────┐           │
│ 代码检查    │────▶│ 一致性报告  │───────────┘
│ cfg(feature)│     │ (差异清单)  │
└─────────────┘     └─────────────┘
```

---

## 错误处理

### 潜在风险与应对

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| Feature 名称歧义 | 文档误导 | 使用 Cargo.toml 作为唯一真实来源 |
| 代码示例无法编译 | 用户体验差 | 每阶段运行 `cargo test --doc` |
| 文档格式不一致 | 可读性差 | 使用统一的 markdown 模板 |
| 遗漏新 API | 文档不完整 | 自动扫描 `pub use` 语句 |

### 回滚策略
- 每个阶段独立 git commit
- 保留原始文档备份
- 使用 git revert 可回滚

---

## 测试策略

### 每阶段验证
```bash
# 1. 构建验证
cargo build --all-features

# 2. 文档测试
cargo test --doc --all-features

# 3. 文档生成
cargo doc --all-features --no-deps --open

# 4. 链接检查
grep -r "](.*.md)" docs/ | while read link; do
    [ -f "${link#*[%(}" ] || echo "Broken: $link"
done
```

### 最终验收
- [ ] 无文档警告
- [ ] 所有链接有效
- [ ] 代码示例可编译
- [ ] CHANGELOG 符合格式
- [ ] Feature 列表一致

---

## 时间估计

| 阶段 | 任务 | 时间 |
|------|------|------|
| D | 整体一致性检查 | 2-3 小时 |
| B | 版本同步更新 | 1-2 小时 |
| A | 补充新功能文档 | 4-6 小时 |
| C | API 文档修正 | 2-3 小时 |
| **总计** | | **9-14 小时** |

---

## 成功标准

### 必须达成
- [x] CHANGELOG 包含 0.3.0 条目
- [ ] 文档与代码 100% 一致
- [ ] 所有新功能有文档
- [ ] `cargo doc` 无警告

### 可选达成
- [ ] 新增独立指南文档
- [ ] 文档翻译更新
- [ ] 示例代码补充

---

## 参考

- [Keep a Changelog](https://keepachangelog.com/en/1.0.0/)
- [Semantic Versioning](https://semver.org/spec/v2.0.0.html)
- [Cargo Features](https://doc.rust-lang.org/cargo/reference/features.html)
- 项目: `/home/dev/projects/confers`
- 目标项目: `/home/project/confers` (v0.2.2)
- Merge commit: `6c88ad8`
