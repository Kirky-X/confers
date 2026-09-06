# 📘 Confers API 参考

本参考文档完整收录 Confers 的公开 API，包括配置构建器、核心 trait、错误类型以及各特性门控模块的接口。文档假设启用了 `full` 特性；使用其他特性组合时，部分 API 可能不可用。

## 📋 目录

<details open>
<summary>📑 目录（点击展开）</summary>

- [概述](#-概述)
- [核心 API](#-核心-api)
- [配置/扩展 API](#-配置扩展-api)
- [错误类型](#-错误类型)
- [特性门控 API](#-特性门控-api)
- [使用示例](#-使用示例)
- [最佳实践](#-最佳实践)

</details>

---

## 🎯 概述

### API 设计原则

| 原则 | 说明 |
|:-----|:-----|
| **简单** | 直观易用 |
| **安全** | 默认类型安全 |
| **可组合** | 轻松构建复杂工作流 |
| **文档完善** | 提供全面的文档支持 |

### 📦 特性说明

confers 提供灵活的特性配置，用户可按需选择所需功能：

**特性预设：**

| 预设 | 特性 | 适用场景 |
|------|------|----------|
| `minimal` | `env` + `json` | 最小依赖（环境变量 + JSON） |
| `recommended` | `toml` + `json` + `env` + `validation` | 大多数应用的推荐配置 |
| `dev` | `toml` + `json` + `yaml` + `env` + `cli` + `validation` + `schema` + `audit` + `watch` + `migration` + `snapshot` + `dynamic` | 开发配置 |
| `production` | `toml` + `env` + `watch` + `encryption` + `validation` + `audit` + `schema` + `cli` + `migration` + `dynamic` + `progressive-reload` + `snapshot` | 生产配置 |
| `distributed` | `toml` + `json` + `env` + `watch` + `validation` + `config-bus` + `progressive-reload` + `audit` | 分布式系统 |
| `full` | 全部特性 | 完整功能集 |

**单项特性：**

| 特性 | 说明 | 默认启用 |
|------|------|----------|
| **格式支持** |||
| `toml` | TOML 格式支持 | ✅ |
| `json` | JSON 格式支持 | ✅ |
| `yaml` | YAML 格式支持 | ❌ |
| `ini` | INI 格式支持 | ❌ |
| `env` | 环境变量支持 | ✅ |
| `dotenv` | `.env` 文件支持（`env` 的别名） | ❌ |
| **核心特性** |||
| `validation` | 配置校验（garde） | ❌ |
| `watch` | 文件监听与热重载 | ❌ |
| `encryption` | XChaCha20 加密 | ❌ |
| `cli` | 命令行集成 | ❌ |
| `schema` | JSON Schema 生成 | ❌ |
| `typescript-schema` | TypeScript 类型生成 | ❌ |
| **安全** |||
| `security` | 安全模块 | ❌ |
| `security-rules` | 安全校验规则 | ❌ |
| `key` | 密钥管理系统 | ❌ |
| **进阶特性** |||
| `feature-toggle` | 运行时特性开关 | ❌ |
| `audit` | 审计日志 | ❌ |
| `dynamic` | 动态字段 | ❌ |
| `progressive-reload` | 渐进式发布 | ❌ |
| `migration` | 配置迁移 | ❌ |
| `snapshot` | 快照回滚 | ❌ |
| `interpolation` | 变量插值 | ❌ |
| **远程来源** |||
| `remote` | HTTP 轮询 | ❌ |
| `etcd` | Etcd 集成 | ❌ |
| `consul` | Consul 集成 | ❌ |
| **消息总线** |||
| `config-bus` | 配置事件总线 | ❌ |
| `nats-bus` | NATS 消息总线 | ❌ |
| `redis-bus` | Redis 消息总线 | ❌ |
| **其他** |||
| `context-aware` | 上下文感知配置 | ❌ |
| `modules` | 模块化配置 | ❌ |

---

## 🧱 核心 API

### 配置构建器（ConfigBuilder）

`ConfigBuilder<T>` 是从多个来源加载并合并配置的核心组件，支持文件、环境变量、远程来源等配置源的智能合并。

```mermaid
graph TB
    subgraph Sources ["配置来源"]
        A["配置文件"]
        B["环境变量"]
        C["CLI 参数"]
        D["远程来源"]
    end

    subgraph Loader ["ConfigBuilder"]
        E["智能合并"]
        F["校验"]
        G["热重载"]
    end

    subgraph Output ["输出"]
        H["类型安全的配置"]
    end

    Sources --> Loader
    Loader --> Output
```

#### 创建与配置

##### `ConfigBuilder::new()`

创建新的配置构建器实例。

```rust
pub fn new() -> Self
```

**示例：**

```rust
let builder = ConfigBuilder::<AppConfig>::new();
```

**说明**：`ConfigBuilder` 实现了 `Default` trait。`new()` 方法返回一个带合理默认值的实例。

##### `defaults(defaults: HashMap<String, ConfigValue>)`

设置默认配置值，当其他来源未提供对应值时使用。

```rust
pub fn defaults(mut self, defaults: HashMap<String, ConfigValue>) -> Self
```

**示例：**

```rust
use std::collections::HashMap;
use confers::ConfigValue;

let mut defaults = HashMap::new();
defaults.insert("port".to_string(), ConfigValue::uint(8080));
defaults.insert("host".to_string(), ConfigValue::string("localhost"));

let builder = ConfigBuilder::<AppConfig>::new()
    .defaults(defaults);
```

**说明**：默认值优先级最低，会被其他配置来源覆盖。

##### `file(path: impl Into<PathBuf>)`

添加显式配置文件。支持多个配置文件，优先级按添加顺序递增。

```rust
pub fn file(mut self, path: impl Into<PathBuf>) -> Self
```

**示例：**

```rust
let builder = ConfigBuilder::<AppConfig>::new()
    .file("config/base.toml")
    .file("config/development.toml");
```

**说明**：文件按添加顺序加载，后加入的文件优先级更高。

##### `file_optional(path: impl Into<PathBuf>)`

添加可选配置文件。文件不存在时会被静默跳过。

```rust
pub fn file_optional(mut self, path: impl Into<PathBuf>) -> Self
```

**示例：**

```rust
let builder = ConfigBuilder::<AppConfig>::new()
    .file("config.toml")
    .file_optional("config.local.toml"); // 可能不存在
```

##### `env()`

添加环境变量来源。环境变量将被加载并映射到配置字段。

```rust
pub fn env(mut self) -> Self
```

**示例：**

```rust
let builder = ConfigBuilder::<AppConfig>::new()
    .file("config.toml")
    .env();
```

##### `env_prefix(prefix: impl Into<String>)`

添加带前缀的环境变量来源。只加载以该前缀开头的环境变量。

```rust
pub fn env_prefix(mut self, prefix: impl Into<String>) -> Self
```

**示例：**

```rust
let builder = ConfigBuilder::<AppConfig>::new()
    .env_prefix("APP");
// 加载 APP_PORT、APP_HOST 等
```

**示例：**

```rust
let builder = ConfigBuilder::<AppConfig>::new()
    .env()
    .env_prefix("APP");
```

**说明**：环境变量优先级高于配置文件，但低于内存来源。

##### `with_snapshot(config: SnapshotConfig)`（需要 `snapshot` 特性）

启用快照配置以支持回滚。

```rust
#[cfg(feature = "snapshot")]
pub fn with_snapshot(mut self, config: SnapshotConfig) -> Self
```

##### `fail_fast(enabled: bool)`

启用或禁用快速失败模式。快速失败模式下，任何配置错误都会立即中止构建过程。

```rust
pub fn fail_fast(mut self, fail_fast: bool) -> Self
```

> **注意**：`watch()`、`validate()` 与 `with_audit*()` 方法已在早期版本移除。文件监听现在通过 `FsWatcher`/`MultiFsWatcher` 直接处理；校验由 `validation` 特性与 `#[config(validate)]` 属性控制；审计日志通过 `AuditWriter` builder 配置。

##### `limits(limits: ConfigLimits)`

设置用于资源管理的配置上限。

```rust
pub fn limits(mut self, limits: ConfigLimits) -> Self
```

##### `strategy(strategy: MergeStrategy)`

设置配置来源之间的合并策略。

```rust
pub fn strategy(mut self, strategy: MergeStrategy) -> Self
```

#### 远程配置

> ⚠️ **注意**：远程配置需要启用 `remote` 特性。请使用 `source()` 方法添加远程配置来源。

##### `source(source: Box<dyn Source>)`

添加自定义配置来源。这是添加远程来源的统一方法。

```rust
pub fn source(mut self, source: Box<dyn Source>) -> Self
```

**示例 - HTTP 远程来源：**

```rust
use confers::remote::HttpPolledSourceBuilder;

let http_source = HttpPolledSourceBuilder::new()
    .url("https://config-server.example.com/app-config")
    .timeout(Duration::from_secs(30))
    .build()?;

let config = ConfigBuilder::<AppConfig>::new()
    .source(Box::new(http_source))
    .build()?;
```

**示例 - Etcd 来源：**

```rust
use confers::remote::EtcdSourceBuilder;

// 注意：EtcdSourceBuilder 的 build() 是异步方法（内部建立 gRPC 连接）
let etcd_source = EtcdSourceBuilder::new()
    .endpoints(vec!["localhost:2379"])
    .prefix("/myapp/config")
    .build()
    .await?;

let config = ConfigBuilder::<AppConfig>::new()
    .source(Box::new(etcd_source))
    .build()?;
```

**示例 - Consul 来源：**

```rust
use confers::remote::ConsulSourceBuilder;

let consul_source = ConsulSourceBuilder::new()
    .address("localhost:8500")
    .prefix("myapp/config")
    .build()?;

let config = ConfigBuilder::<AppConfig>::new()
    .source(Box::new(consul_source))
    .build()?;
```

#### 构建方法

##### `build()`

同步构建配置，合并所有已配置的来源。

```rust
pub fn build(self) -> ConfigResult<T>
```

**示例：**

```rust
let config = builder.build()?;
```

##### `build_with_fallback(fallback: T)`

带回退配置地构建。构建失败时返回回退配置。

```rust
pub fn build_with_fallback(self, fallback: T) -> BuildResult<T>
```

**示例：**

```rust
let result = builder.build_with_fallback(AppConfig::default());
if result.degraded {
    println!("Using fallback: {:?}", result.degraded_reason);
}
```

##### `build_resilient()`

弹性构建，收集警告而不是直接失败。

```rust
pub fn build_resilient(self) -> ConfigResult<BuildResult<T>>
```

##### `build_with_watcher()`（异步，已弃用）

> **已弃用**（自 0.3.0 起）：该方法在文件变更时**不会**执行热重载。完整的热重载支持请直接使用 `FsWatcher`/`MultiFsWatcher`。

构建并附带监听器支持。返回配置更新的接收端与监听器守卫。需要 `watch` 特性。

```rust
#[deprecated(since = "0.3.0", note = "Does not reload on file changes. Use FsWatcher/MultiFsWatcher directly.")]
#[cfg(feature = "watch")]
pub async fn build_with_watcher(
    self,
) -> ConfigResult<(
    tokio::sync::watch::Receiver<Arc<T>>,
    WatcherGuard,
)>
```

#### 格式探测

##### `detect_format_from_content(content: &str) -> Option<Format>`

根据文件内容智能探测配置格式。

```rust
pub fn detect_format_from_content(content: &str) -> Option<Format>
```

**支持的探测格式**：JSON、YAML、TOML、INI

##### `detect_format_from_path(path: &Path) -> Option<Format>`

根据文件扩展名探测配置格式。

```rust
pub fn detect_format_from_path(path: &Path) -> Option<Format>
```

---

## 🔌 配置/扩展 API

### ConfigProvider Trait

`ConfigProvider` 是配置访问的核心 trait，为访问配置值提供基础接口。所有配置提供者都必须实现该 trait。

#### Trait 定义

```rust
pub trait ConfigProvider: Send + Sync {
    fn get_raw(&self, key: &str) -> Option<&AnnotatedValue>;
    fn keys(&self) -> Vec<String>;
    fn has(&self, key: &str) -> bool { ... }
}
```

#### 方法说明

##### `get_raw(&self, key: &str) -> Option<&AnnotatedValue>`

按键获取原始的带注解值。键不存在时返回 `None`。

```rust
fn get_raw(&self, key: &str) -> Option<&AnnotatedValue>
```

##### `keys(&self) -> Vec<String>`

以点号格式（如 `"database.host"`）获取所有配置键。

```rust
fn keys(&self) -> Vec<String>
```

##### `has(&self, key: &str) -> bool`

检查键是否存在。

```rust
fn has(&self, key: &str) -> bool
```

---

### ConfigProviderExt Trait

`ConfigProviderExt` 是扩展 trait，提供类型安全的便捷访问方法。它为所有 `ConfigProvider` 类型提供默认实现。

#### Trait 定义

```rust
pub trait ConfigProviderExt: ConfigProvider {
    fn get_string(&self, key: &str) -> Option<String>;
    fn get_int(&self, key: &str) -> Option<i64>;
    fn get_uint(&self, key: &str) -> Option<u64>;
    fn get_float(&self, key: &str) -> Option<f64>;
    fn get_bool(&self, key: &str) -> Option<bool>;
    fn get_typed<T>(&self, key: &str) -> ConfigResult<T>;
    fn get_many<'a>(&self, keys: &[&'a str]) -> HashMap<&'a str, Option<&AnnotatedValue>>;
    fn get_by_path(&self, path: &[&str]) -> Option<&AnnotatedValue>;
}
```

#### 方法说明

##### `get_string(&self, key: &str) -> Option<String>`

获取字符串类型的配置值。

**示例：**

```rust
let name = config.get_string("app.name");
assert_eq!(name, Some("my-app".to_string()));
```

##### `get_int(&self, key: &str) -> Option<i64>`

获取整数类型的配置值。

**示例：**

```rust
let port = config.get_int("server.port");
assert_eq!(port, Some(8080));
```

##### `get_uint(&self, key: &str) -> Option<u64>`

获取无符号整数类型的配置值。

##### `get_bool(&self, key: &str) -> Option<bool>`

获取布尔类型的配置值。

**示例：**

```rust
let debug = config.get_bool("app.debug");
assert_eq!(debug, Some(true));
```

##### `get_float(&self, key: &str) -> Option<f64>`

获取浮点数类型的配置值。

##### `get_typed<T>(&self, key: &str) -> ConfigResult<T>`

按键获取指定类型的值。值无法转换时返回错误。

```rust
fn get_typed<T>(&self, key: &str) -> ConfigResult<T>
where
    T: std::str::FromStr + Default,
    T::Err: std::fmt::Display
```

##### `get_many<'a>(&self, keys: &[&'a str]) -> HashMap<&'a str, Option<&AnnotatedValue>>`

高效地一次获取多个值。缺失的键对应 `None` 值。

---

### KeyProvider Trait

`KeyProvider` 是同步加密密钥提供者 trait。实现方为敏感字段加密提供密钥。

```rust
pub trait KeyProvider: Send + Sync {
    fn get_key(&self) -> ConfigResult<ZeroizingBytes>;
    fn provider_type(&self) -> &'static str;
    fn cache_policy(&self) -> KeyCachePolicy { ... }
}
```

#### KeyCachePolicy

```rust
pub enum KeyCachePolicy {
    Ttl,      // 带存活时间的缓存（默认）
    Forever,  // 永久缓存
    Never,    // 永不缓存
}
```

---

### TypedConfigKey

类型安全的配置键，将配置路径绑定到特定类型，提供编译期安全。

```rust
pub struct TypedConfigKey<T> {
    path: &'static str,
    description: Option<&'static str>,
}

impl<T> TypedConfigKey<T> {
    pub const fn new(path: &'static str) -> Self;
    pub const fn with_description(mut self, description: &'static str) -> Self;
    pub fn path(&self) -> &'static str;
    pub fn description(&self) -> Option<&'static str>;
}
```

**示例：**

```rust
use confers::TypedConfigKey;

static DB_HOST: TypedConfigKey<String> =
    TypedConfigKey::new("database.host")
        .with_description("Database hostname");

static DB_PORT: TypedConfigKey<u16> =
    TypedConfigKey::new("database.port");
```

---

## 🚨 错误类型

### `ConfigConfigError`（配置阶段）

初始化时遇到的配置阶段错误。这类错误表示阻止配置加载的根本性问题。

| 变体 | 说明 | 处理建议 |
|------|------|----------|
| `MissingField { field: String }` | 缺少必需的配置字段 | 在配置来源中补充缺失字段 |
| `InvalidValue { field: String, expected_type: String, message: String }` | 配置值非法 | 检查值的类型与格式 |
| `FileNotFound { filename: PathBuf, source: Option<std::io::Error> }` | 找不到配置文件 | 检查文件路径是否正确 |
| `ParseError { format: String, message: String, location: Option<ParseLocation>, source: Option<Box<dyn Error + Send + Sync>> }` | 配置来源解析错误 | 检查配置文件语法 |
| `SizeLimitExceeded { actual: usize, limit: usize }` | 配置超出大小上限 | 调大上限或精简配置 |
| `ValidationFailed { field: String, rule: String, message: String }` | 校验约束未通过 | 修正配置值以满足约束 |
| `VersionMismatch { found: u32, expected: u32 }` | 配置版本不匹配 | 更新配置版本 |
| `SourceChainError { message: String, source_index: usize }` | 来源链配置错误 | 检查来源配置 |
| `InterpolationError { variable: String, message: String }` | 插值配置错误 | 检查插值变量语法 |
| `CircularReference { path: String }` | 检测到循环引用 | 修正变量引用链 |

### `ConfigError` / `ConfersError`（运行时阶段）

运行期间遇到的配置错误。`ConfersError` 是 `ConfigError` 的类型别名。

| 变体 | 说明 | 处理建议 |
|------|------|----------|
| `FileNotFound { filename: PathBuf, source: Option<std::io::Error> }` | 找不到配置文件 | 检查文件路径是否正确 |
| `ParseError { format: String, message: String, location: Option<ParseLocation>, source: Option<Box<dyn Error>> }` | 配置解析出错 | 检查配置文件语法 |
| `ValidationFailed { field: String, rule: String, message: String }` | 字段校验失败 | 检查字段值的约束 |
| `SchemaValidationFailed { count: usize }` | Schema 校验失败 | 检查配置是否符合 schema |
| `DecryptionFailed { message: String }` | 解密失败 | 检查加密密钥与数据 |
| `RemoteUnavailable { error_type: String, retryable: bool }` | 远程来源不可用 | 可重试则重试，并检查网络 |
| `VersionMismatch { found: u32, expected: u32 }` | 版本不匹配 | 更新配置版本 |
| `MigrationFailed { from: u32, to: u32, reason: String, source: Option<Box<dyn Error>> }` | 迁移失败 | 检查迁移函数 |
| `ModuleNotFound { group: String, module: String }` | 模块或 profile 未找到 | 检查模块/profile 名称 |
| `ReloadRolledBack { reason: String }` | 重载已回滚 | 检查健康检查校验器 |
| `IoError(std::io::Error)` | IO 操作错误 | 检查文件权限与磁盘空间 |
| `InvalidValue { key: String, expected_type: String, message: String }` | 键的值非法 | 检查值的类型与格式 |
| `SourceChainError { message: String, source_index: usize }` | 来源链错误 | 检查来源配置 |
| `Timeout { duration_ms: u64 }` | 操作超时 | 增大超时时间或检查网络 |
| `SizeLimitExceeded { actual: usize, limit: usize }` | 文件超出大小上限 | 调大上限或精简文件 |
| `InterpolationError { variable: String, message: String }` | 插值错误 | 检查变量语法 |
| `KeyError { message: String }` | 加密密钥错误 | 检查密钥配置 |
| `CircularReference { path: String }` | 检测到循环引用 | 修正变量引用链 |
| `LockPoisoned { resource: String }` | Mutex/RwLock 中毒 | 重试操作或重启服务 |
| `MultiSource { source: MultiSourceError }` | 多个来源失败 | 检查各来源的错误 |
| `ConcurrencyConflict { key: String, message: String, expected_type: Option<String> }` | 并发冲突 | 重试该操作 |
| `KeyRotationFailed { from_version: String, to_version: String, reason: String }` | 密钥轮换失败 | 检查密钥版本与权限 |
| `WatcherError { message: String, path: Option<PathBuf>, recoverable: bool }` | 监听器错误 | 检查文件路径与权限 |
| `OverrideBlocked { key: String, reason: String, override_source: Option<String> }` | 覆盖被阻止 | 检查保护规则 |
| `HealthCheckFailed { reason: String }` | 健康检查失败 | 检查健康检查配置 |

### 相关类型定义

#### `KeyVersion`

```rust
pub struct KeyVersion {
    pub id: String,           // 密钥版本唯一标识
    pub version: u32,         // 版本号
    pub created_at: u64,      // 创建时间戳
    pub status: KeyStatus,    // 密钥状态
    pub algorithm: String,    // 加密算法
}
```

#### `KeyStatus`

```rust
pub enum KeyStatus {
    Active,       // 活跃，可用于加解密
    Deprecated,   // 已弃用，仅用于解密历史数据
    Compromised,  // 已泄露，应立即轮换
}
```

#### `KeyInfo`

```rust
pub struct KeyInfo {
    pub key_id: String,           // 密钥环 ID
    pub current_version: u32,     // 当前活跃版本
    pub total_versions: usize,    // 总版本数
    pub active_versions: usize,   // 活跃版本数
    pub deprecated_versions: usize, // 已弃用版本数
    pub created_at: u64,          // 创建时间戳
    pub last_rotated_at: Option<u64>, // 上次轮换时间
}
```

#### `RotationResult`

```rust
pub struct RotationResult {
    pub key_id: String,           // 密钥环 ID
    pub previous_version: u32,    // 轮换前版本
    pub new_version: u32,         // 轮换后版本
    pub rotated_at: u64,          // 轮换时间戳
    pub reencryption_required: bool, // 是否需要重新加密
}
```

---

## 🚪 特性门控 API

以下 API 由 Cargo 特性门控，启用对应特性后才可用。

### 密钥管理（`key` 特性）

`KeyManager` 提供加密密钥的全面管理，包括轮换、版本控制与密钥存储。需要启用 `encryption` 特性。

```mermaid
graph TB
    subgraph Storage ["密钥存储"]
        A["密钥环"]
        B["版本历史"]
        C["元数据"]
    end

    subgraph Manager ["KeyManager"]
        D["轮换管理"]
        E["版本控制"]
        F["安全存储"]
    end

    subgraph Operations ["操作"]
        G["创建"]
        H["轮换"]
        I["获取"]
        J["删除"]
    end

    Storage --> Manager
    Manager --> Operations
```

#### 创建与管理

##### `KeyManager::new()`

创建新的密钥管理器。密钥数据经主密钥加密后由调用方负责持久化。

```rust
pub fn new() -> Result<Self, ConfigError>
```

**示例：**

```rust
let km = KeyManager::new()?;
```

##### `initialize(master_key: &[u8; 32], key_id: String, created_by: String)`

使用主密钥初始化新的密钥环。

```rust
pub fn initialize(
    &mut self,
    master_key: &[u8; 32],
    key_id: String,
    created_by: String,
) -> Result<KeyVersion, ConfigError>
```

**参数说明：**

| 参数 | 说明 |
|------|------|
| `master_key` | 用于加密密钥存储的 32 字节主密钥 |
| `key_id` | 密钥环的唯一标识 |
| `created_by` | 用于审计追踪的创建者标识 |

**示例：**

```rust
use confers::key::KeyManager;

let mut km = KeyManager::new()?;
let master_key = [0u8; 32]; // 从安全位置获取
let version = km.initialize(
    &master_key,
    "production".to_string(),
    "security-team".to_string()
)?;
```

##### `rotate_key(master_key: &[u8; 32], key_id: Option<String>, created_by: String, description: Option<String>)`

将密钥轮换到新版本，支持满足安全合规要求的密钥轮换。

```rust
pub fn rotate_key(
    &mut self,
    master_key: &[u8; 32],
    key_id: Option<String>,
    created_by: String,
    description: Option<String>,
) -> Result<RotationResult, ConfigError>
```

**返回值**：`RotationResult` 包含轮换前后的版本信息，以及是否需要重新加密。

**示例：**

```rust
let result = km.rotate_key(
    &master_key,
    Some("production".to_string()),
    "security-team".to_string(),
    Some("Scheduled key rotation".to_string())
)?;

println!("Key rotated from version {} to {}", result.previous_version, result.new_version);
```

##### `get_key_info(key_id: &str)`

获取指定密钥的元数据与版本信息。

```rust
pub fn get_key_info(&self, key_id: &str) -> Result<KeyInfo, ConfigError>
```

##### `get_key_by_version(key_id: &str, version: u32) -> Result<Option<&KeyBundle>, ConfigError>`

获取指定密钥版本的密钥数据；版本不存在时返回 `None`。

```rust
pub fn get_key_by_version(&self, key_id: &str, version: u32) -> Result<Option<&KeyBundle>, ConfigError>
```

##### `generate_key() -> Result<[u8; 32], ConfigError>`

使用加密安全随机数生成新的 256 位密钥。

```rust
pub fn generate_key(&mut self) -> Result<[u8; 32], ConfigError>
```

##### `list_keys() -> Vec<KeyInfo>`

列出所有密钥环的元数据。

```rust
pub fn list_keys(&self) -> Vec<KeyInfo>
```

#### 密钥生命周期管理

`key` 特性提供完整的密钥生命周期管理，包括密钥创建、存储、轮换与吊销。

```rust
use confers::key::KeyManager;

// 创建密钥管理器
let master_key = [0u8; 32]; // 从安全位置获取
let mut manager = KeyManager::new()?;

// 初始化新密钥环
let version = manager.initialize(
    &master_key,
    "production".to_string(),
    "security-team".to_string()
)?;

// 生成新密钥
let key = manager.generate_key()?;

// 列出所有密钥
let keys = manager.list_keys();

// 轮换密钥
let result = manager.rotate_key(
    &master_key,
    Some("production".to_string()),
    "security-team".to_string(),
    Some("Scheduled rotation".to_string())
)?;
```

#### 方法一览

| 方法 | 参数 | 返回值 | 说明 |
|------|------|--------|------|
| `new()` | - | `Result<Self>` | 创建密钥管理器 |
| `initialize(master_key, key_id, created_by)` | `&[u8; 32], String, String` | `Result<KeyVersion>` | 初始化新密钥环 |
| `generate_key()` | - | `Result<[u8; 32]>` | 生成新的随机密钥 |
| `rotate_key(master_key, key_id, created_by, description)` | 见上文 | `Result<RotationResult>` | 轮换密钥到新版本 |
| `list_keys()` | - | `Vec<KeyInfo>` | 列出所有密钥环 |
| `get_key_info(key_id)` | `&str` | `Result<KeyInfo>` | 获取密钥环元数据 |

#### KeyRotationService

自动密钥轮换服务。

```rust
use confers::key::{KeyRotationPolicy, KeyRotationService};

// 轮换策略（字段全部公开，可按需调整）
let policy = KeyRotationPolicy {
    max_versions: 5,
    rotation_interval_days: 90, // 建议每 90 天轮换
    grace_period_days: 14,
    auto_rotate: true,
    notify_before_expiry_days: 30,
};

let mut service = KeyRotationService::new(policy);

// 生成、校验并执行轮换（关联/实例方法，需要已初始化的密钥环）
// let plan = KeyRotationService::create_rotation_plan(&key_ring, 2)?;
// KeyRotationService::validate_rotation(&key_ring, &plan, &policy)?;
// let result = service.execute_rotation(
//     &mut key_ring,
//     &master_key,
//     "security-team".to_string(),
//     Some("Scheduled rotation".to_string()),
// )?;
```

#### KeyStorage

加密的密钥持久化存储。

```rust
use confers::key::KeyStorage;
use std::path::PathBuf;

let mut storage = KeyStorage::new(PathBuf::from("/path/to/keys"))?;
storage.set_master_key(&master_key); // 设置主密钥，用于加解密密钥数据
storage.save()?;                     // 将密钥持久化到存储路径
storage.load()?;                     // 从存储路径加载密钥
```

---

### 加密函数（`encryption` 特性）

`XChaCha20Crypto` 实现 XChaCha20-Poly1305 加密以保护敏感配置值，提供带关联数据的认证加密（AEAD）。需要启用 `encryption` 特性。

```mermaid
graph LR
    A["明文"] --> B["XChaCha20-Poly1305 加密"]
    B --> C["输出<br/>nonce + ciphertext"]
    C --> D["存储或传输"]
    D --> E["解密"]
    E --> F["恢复明文"]
```

#### 创建

##### `XChaCha20Crypto::new()`

创建新的加密器实例。

```rust
pub fn new() -> Self
```

**示例：**

```rust
use confers::XChaCha20Crypto;

let crypto = XChaCha20Crypto::new();
```

#### 加密/解密操作

##### `encrypt(plaintext: &[u8], key: &[u8]) -> Result<(Vec<u8>, Vec<u8>), CryptoError>`

使用 32 字节密钥加密字节序列。返回 `(nonce, ciphertext)` 元组。

**特性：**

- 使用 XChaCha20-Poly1305 算法（ChaCha20 的扩展 nonce 变体）
- 每次加密生成随机 96 位 nonce
- 提供带完整性校验的认证加密

```rust
pub fn encrypt(&self, plaintext: &[u8], key: &[u8]) -> Result<(Vec<u8>, Vec<u8>), CryptoError>
```

**示例：**

```rust
let key = [0u8; 32]; // 应使用安全的随机密钥
let (nonce, ciphertext) = crypto.encrypt(b"my-secret-api-key", &key)?;
```

##### `decrypt(nonce: &[u8], ciphertext: &[u8], key: &[u8]) -> Result<Vec<u8>, CryptoError>`

使用 nonce、密文和 32 字节密钥解密字节序列。

**特性：**

- 需要加密时使用的同一 nonce
- 校验 Poly1305 认证标签，检测到篡改会返回错误

```rust
pub fn decrypt(&self, nonce: &[u8], ciphertext: &[u8], key: &[u8]) -> Result<Vec<u8>, CryptoError>
```

**示例：**

```rust
let decrypted = crypto.decrypt(&nonce, &ciphertext, &key)?;
assert_eq!(decrypted, b"my-secret-api-key");
```

#### 密钥派生

##### `derive_field_key(master_key: &[u8], field_path: &str, key_version: &str) -> Result<[u8; 32], CryptoError>`

使用 HKDF-SHA256 从主密钥派生字段级加密密钥。

```rust
pub fn derive_field_key(
    master_key: &[u8],
    field_path: &str,
    key_version: &str,
) -> Result<[u8; 32], CryptoError>
```

**示例：**

```rust
use confers::derive_field_key;

let master_key = [0u8; 32];
let field_key = derive_field_key(&master_key, "database.password", "v1")?;
```

---

### 安全模块（`security` 特性）

`security` 特性提供环境变量校验、错误脱敏与安全注入能力。

#### EnvSecurityValidator

环境变量安全校验器，防止注入攻击。

```rust
use confers::security::{EnvSecurityValidator, EnvironmentValidationConfig};
use std::collections::HashMap;

// 通过 EnvironmentValidationConfig 自定义校验规则
let config = EnvironmentValidationConfig::new()
    .with_custom_blocked_patterns(vec![
        r".*_SECRET$".to_string(),
        r".*_PASSWORD$".to_string(),
    ]);
let validator = EnvSecurityValidator::with_config(config);

// 校验单个环境变量名；第二个参数为值上下文，
// 传入以 "enc:" 开头的加密值时会跳过敏感名封锁规则
validator.validate_env_name("APP_NAME", None)?;
validator.validate_env_name("DB_PASSWORD", None)?; // Err: 命中 *_PASSWORD$ 封锁规则

// 校验环境变量值（长度与内容检查）
validator.validate_env_value("production")?;

// 批量校验「配置字段名 -> 环境变量名」映射
let mut mapping = HashMap::new();
mapping.insert("app_name".to_string(), "APP_NAME".to_string());
validator.validate_env_mapping(&mapping)?;
```

也可使用 `EnvSecurityValidator::strict()` / `EnvSecurityValidator::lenient()` 直接采用预设的严格/宽松规则。

#### ErrorSanitizer

错误信息中的敏感数据脱敏。

```rust
use confers::security::ErrorSanitizer;

let sanitizer = ErrorSanitizer::default();
let clean_msg = sanitizer.sanitize(&error_msg);
```

#### ConfigInjector

安全的配置注入器。

> **注意**：`ConfigInjector` 目前位于 `src/security/config_injector.rs`，但**未被重导出**为公开 API（`src/security/mod.rs` 中的 `mod config_injector` 声明为 `pub(crate)`）。它被视为内部基础设施；如需将配置注入集成到您自己的流水线中，请提交 issue 让维护者暴露稳定接口。

#### SecurityValidatorRegistry（`security-rules` 特性）

`security-rules` 特性提供一套标准化的安全校验规则库，内置校验器与注册表，可在启动时自动执行。

```rust
use confers::security::rules::{SecurityValidatorRegistry, SecurityValidator};

// 使用内置校验器（JWT、CORS、SSRF、TLS）
let registry = SecurityValidatorRegistry::with_defaults();
let report = registry.validate_all(&config);

if !report.is_ok(false) {
    for v in &report.violations {
        eprintln!("[{}] {}: {}", v.severity, v.validator, v.message);
    }
}
```

**内置校验器：**

| 校验器 | 类别 | 检查内容 |
|--------|------|----------|
| `JwtSecretValidator` | `jwt` | 密钥长度 ≥ 32 字节、弱口令检测 |
| `CorsValidator` | `cors` | 通配符 `*` 检测、methods 非空、max_age ≤ 86400s |
| `SsrfValidator` | `ssrf` | 18 个封锁 CIDR 网段（IPv4 + IPv6）、白名单支持 |
| `TlsConfigValidator` | `tls` | min_version ≥ 1.2、12 个弱加密套件 |

**自定义校验器：**

```rust
use confers::security::rules::{SecurityValidator, SecurityViolation, ViolationSeverity};
use confers::interface::ConfigProvider;

struct MyValidator;

impl SecurityValidator for MyValidator {
    fn validate(&self, config: &dyn ConfigProvider) -> Result<(), Vec<SecurityViolation>> {
        // 自定义校验逻辑
        Ok(())
    }
    fn name(&self) -> &'static str { "my_validator" }
    fn category(&self) -> &'static str { "custom" }
    fn description(&self) -> &'static str { "My custom validator" }
}

let mut registry = SecurityValidatorRegistry::new();
registry.register(Box::new(MyValidator));
```

---

### 运行时特性开关（`feature-toggle` 特性）

`feature-toggle` 特性提供运行时特性开关，与编译期 `cfg` 特性互补。

```rust
use confers::toggle::FeatureToggleRegistry;

let registry = FeatureToggleRegistry::new();

// 注册并控制特性
registry.register("new_ui", "New UI Design", false);
registry.enable("new_ui");
assert!(registry.is_enabled("new_ui"));

// 从配置切换
registry.load_from_config(&config, "features");

// 列出全部开关
for info in registry.list() {
    println!("{}: enabled={}, desc={}", info.name, info.enabled, info.description);
}
```

> **注意**：运行时开关与编译期 `cfg(feature = ...)` 标志互补。编译期标志控制代码的裁剪与包含；运行时开关控制行为切换，无需重新编译。

---

### 模块化配置（`modules` 特性）

`modules` 特性提供可组合的配置分组支持，允许在运行时选择配置模块组合。适用于管理多套环境配置（如数据库：mysql/postgresql，缓存：redis/memory）。

#### ModuleConfig

包含 profile 路径与活跃 profile 的配置模块。

```rust
use confers::modules::ModuleConfig;
use std::path::PathBuf;

let config = ModuleConfig::new(
    "database",
    vec![
        ("mysql", PathBuf::from("conf/db/mysql.toml")),
        ("postgresql", PathBuf::from("conf/db/postgresql.toml")),
    ],
    Some("mysql"),
);
```

#### ModuleConfig 方法

| 方法 | 参数 | 返回值 | 说明 |
|------|------|--------|------|
| `new(name, paths, default)` | `&str, Vec<(&str, PathBuf)>, Option<&str>` | `Self` | 创建新的模块配置 |
| `name()` | - | `&str` | 获取模块名 |
| `active_profile()` | - | `&str` | 获取活跃 profile 名 |
| `profiles()` | - | `Vec<Arc<str>>` | 获取可用 profile 列表 |
| `profile_count()` | - | `usize` | 获取 profile 数量 |
| `get_profile(profile)` | `&str` | `Option<&PathBuf>` | 按名称获取 profile 路径 |
| `has_profile(profile)` | `&str` | `bool` | 检查 profile 是否存在 |
| `set_active_profile(profile)` | `&str` | `Result<(), ConfigError>` | 设置活跃 profile（带校验） |

**示例：**

```rust
use confers::modules::ModuleConfig;
use std::path::PathBuf;

let mut config = ModuleConfig::new(
    "database",
    vec![
        ("mysql", PathBuf::from("conf/db/mysql.toml")),
        ("postgresql", PathBuf::from("conf/db/postgresql.toml")),
    ],
    Some("mysql"),
);

// 访问模块信息
assert_eq!(config.name(), "database");
assert_eq!(config.active_profile(), "mysql");
assert_eq!(config.profile_count(), 2);

// 切换 profile
config.set_active_profile("postgresql")?;
assert_eq!(config.active_profile(), "postgresql");
```

#### ModuleRegistry

管理配置分组（模块）的注册表。

```rust
use confers::modules::ModuleRegistry;
use std::path::PathBuf;

// ModuleRegistry 未提供 new()，使用 Default 或 with_capacity 创建
let mut registry = ModuleRegistry::default();
```

#### ModuleRegistry 方法

| 方法 | 参数 | 返回值 | 说明 |
|------|------|--------|------|
| `default()`（`Default` trait） | - | `Self` | 创建新的空模块注册表 |
| `with_capacity(capacity)` | `usize` | `Self` | 以预分配容量创建 |
| `register_group(name, profiles, default)` | `&str, Vec<(&str, PathBuf)>, Option<&str>` | - | 注册新的配置分组 |
| `get(name)` | `&str` | `Option<&ModuleConfig>` | 按名称获取模块配置 |
| `set_active_profile(name, profile)` | `&str, &str` | `Result<(), ConfigError>` | 设置分组的活跃 profile |
| `get_active_profile(name)` | `&str` | `Option<Arc<str>>` | 获取分组的活跃 profile 名 |
| `active_profiles()` | - | `HashMap<Arc<str>, Arc<str>>` | 获取全部活跃 profile |
| `load_module(name, profile, config)` | `&str, &str, &LoaderConfig` | `Result<AnnotatedValue, ConfigError>` | 以指定 profile 加载模块 |
| `load_active(name, config)` | `&str, &LoaderConfig` | `Result<AnnotatedValue, ConfigError>` | 以活跃 profile 加载 |
| `list_groups()` | - | `Vec<Arc<str>>` | 列出全部已注册分组名 |
| `resolve_from_env(prefix)` | `Option<&str>` | `&mut Self` | 从环境变量解析 profile |
| `validate_active_profiles()` | - | `Result<(), ConfigError>` | 校验全部活跃 profile 的路径有效 |

**示例：**

```rust
use confers::modules::ModuleRegistry;
use confers::loader::LoaderConfig;
use std::path::PathBuf;

let mut registry = ModuleRegistry::default();

// 注册配置分组
registry.register_group(
    "database",
    vec![
        ("mysql", PathBuf::from("conf/db/mysql.toml")),
        ("postgresql", PathBuf::from("conf/db/postgresql.toml")),
    ],
    Some("mysql"),
);

registry.register_group(
    "cache",
    vec![
        ("redis", PathBuf::from("conf/cache/redis.toml")),
        ("memory", PathBuf::from("conf/cache/memory.toml")),
    ],
    Some("redis"),
);

// 以指定 profile 加载模块
let config = registry.load_module("database", "postgresql", &LoaderConfig::default())?;

// 或以活跃 profile 加载
let config = registry.load_active("database", &LoaderConfig::default())?;

// 切换 profile
registry.set_active_profile("database", "mysql")?;

// 从环境变量解析（如 DATABASE_PROFILE=postgresql）
registry.resolve_from_env(Some(""));
```

---

### TypeScript Schema 生成（`typescript-schema` 特性）

`typescript-schema` 特性支持从 Rust 类型生成 TypeScript 类型定义。

#### generate_typescript

生成 TypeScript 类型定义。

```rust
use confers::schema::TypeScriptGenerator;
use schemars::JsonSchema;

// generate 要求类型实现 schemars::JsonSchema
#[derive(confers::Config, JsonSchema)]
pub struct AppConfig {
    pub name: String,
    pub port: u16,
    pub debug: bool,
}

let ts = TypeScriptGenerator::generate::<AppConfig>()?;
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

---

### Schema 生成（`schema` 特性）

配置结构可通过 `schemars` crate 生成 JSON Schema。需要启用 `schema` 特性。

要生成 Schema，配置结构体需要派生 `JsonSchema` trait：

```rust
use serde::{Deserialize, Serialize};
#[cfg(feature = "schema")]
use schemars::JsonSchema;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct AppConfig {
    pub name: String,
    pub port: u16,
}
```

生成的 Schema 可用于校验配置格式或生成文档。

---

## 💡 使用示例

### 基础配置加载

```rust
use confers::ConfigBuilder;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Default, Clone)]
struct AppConfig {
    database_url: String,
    port: u16,
    debug: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ConfigBuilder::<AppConfig>::new()
        .file("config.toml")
        .env()
        .env_prefix("MYAPP")
        .build()?;

    println!("Database: {}", config.database_url);
    println!("Port: {}", config.port);
    Ok(())
}
```

### 密钥轮换

```rust
use confers::key::KeyManager;

fn rotate_keys() -> Result<(), Box<dyn std::error::Error>> {
    let mut km = KeyManager::new()?;
    let master_key = load_master_key()?; // 从安全存储加载主密钥

    let result = km.rotate_key(
        &master_key,
        Some("production".to_string()),
        "security-team".to_string(),
        Some("Scheduled rotation".to_string())
    )?;

    println!("Key version rotated from {} to {}", result.previous_version, result.new_version);
    Ok(())
}
```

### 多来源配置合并

```rust
use confers::ConfigBuilder;
use confers::ConfigValue;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize)]
struct ServerConfig {
    host: String,
    port: i32,
    workers: usize,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut defaults = HashMap::new();
    defaults.insert("host".to_string(), ConfigValue::string("127.0.0.1"));
    defaults.insert("port".to_string(), ConfigValue::integer(8080));
    defaults.insert("workers".to_string(), ConfigValue::uint(4));

    let config = ConfigBuilder::<ServerConfig>::new()
        .defaults(defaults)
        .file("server.toml")
        .env()
        .build()?;

    println!("Server running at {}:{}", config.host, config.port);
    Ok(())
}
```

### 配置加密

```rust
use confers::XChaCha20Crypto;

fn encrypt_sensitive_data() -> Result<(), Box<dyn std::error::Error>> {
    let crypto = XChaCha20Crypto::new();
    let key = load_encryption_key()?; // 32 字节密钥

    let secret = b"my-super-secret-api-key";
    let (nonce, ciphertext) = crypto.encrypt(secret, &key)?;

    println!("Encrypted {} bytes", ciphertext.len());

    let decrypted = crypto.decrypt(&nonce, &ciphertext, &key)?;
    assert_eq!(decrypted, secret);

    Ok(())
}
```

### 配置校验

```rust
use confers::ConfigBuilder;
use garde::Validate;

#[derive(Debug, Deserialize, Validate)]
struct ServerConfig {
    #[garde(length(min = 1))]
    host: String,

    #[garde(range(min = 1, max = 65535))]
    port: u16,
}

fn validate_config() -> Result<(), Box<dyn std::error::Error>> {
    let config = ConfigBuilder::<ServerConfig>::new()
        .file("server.toml")
        .build()?;

    config.validate()?; // 使用 garde 校验
    println!("Configuration is valid");
    Ok(())
}
```

### 自定义格式解析器

对于标准库不支持的配置格式，可以实现自定义解析器：

```rust
use confers::{ConfigBuilder, ConfigError};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize)]
struct CustomConfig {
    settings: HashMap<String, String>,
}

fn load_custom_config() -> Result<CustomConfig, ConfigError> {
    let content = std::fs::read_to_string("config.custom")?;
    let config: CustomConfig = toml::from_str(&content)
        .map_err(|e| ConfigError::ParseError {
            format: "custom".into(),
            message: e.to_string(),
            location: None,
            source: Some(Box::new(e)),
        })?;
    Ok(config)
}
```

### 配置回滚

利用版本历史实现配置回滚：

```rust
use confers::ConfigBuilder;
use std::path::PathBuf;

fn rollback_to_previous_version() -> Result<(), Box<dyn std::error::Error>> {
    let config_dir = PathBuf::from("/etc/myapp");

    let versions = std::fs::read_dir(config_dir.join("history"))?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|p| p.extension().map(|e| e == "toml").unwrap_or(false))
        .collect::<Vec<_>>();

    if versions.len() >= 2 {
        let previous_version = &versions[versions.len() - 2];

        let config = ConfigBuilder::<AppConfig>::new()
            .file(previous_version)
            .build()?;

        println!("Rolled back to previous configuration version");
        return Ok(());
    }

    Err("Not enough version history for rollback".into())
}
```

---

## 🌟 最佳实践

### 配置校验

始终在加载时校验配置：启用 `validation` 特性并使用 `#[config(validate)]`，配置构建阶段即执行 garde 校验：

```rust
use confers::Config;
use garde::Validate;
use serde::{Deserialize, Serialize};

#[derive(Config, Validate, Serialize, Deserialize, Debug)]
#[config(validate)]
struct DatabaseConfig {
    #[serde(default = "default_url")]
    #[garde(length(min = 1))]
    url: String,

    #[serde(default = "default_pool_size")]
    #[garde(range(min = 1, max = 100))]
    pool_size: usize,
}

fn default_url() -> String {
    "postgres://localhost:5432/app".to_string()
}

fn default_pool_size() -> usize {
    10
}
```

### 密钥管理安全

⚠️ 在生产环境中，务必安全管理密钥：

```rust
use confers::key::KeyManager;

fn setup_secure_key_management() -> Result<(), Box<dyn std::error::Error>> {
    // 从环境变量或安全存储获取主密钥
    let master_key = std::env::var("MASTER_KEY")
        .map(|s| {
            let mut key = [0u8; 32];
            let key_bytes = s.as_bytes();
            key.copy_from_slice(&key_bytes[..32.min(key_bytes.len())]);
            key
        })?;

    let mut km = KeyManager::new()?;

    // 初始化密钥环
    km.initialize(
        &master_key,
        "production".to_string(),
        "security-team".to_string(),
    )?;

    // 定期轮换密钥（建议每 90 天）
    let rotation_result = km.rotate_key(
        &master_key,
        Some("production".to_string()),
        "security-team".to_string(),
        Some("Scheduled rotation".to_string()),
    )?;

    println!("Key rotated from version {} to {}",
        rotation_result.previous_version,
        rotation_result.new_version);

    Ok(())
}
```

### 热重载配置

使用 `FsWatcher` 监听配置文件变更并重载（需要 `watch` 特性）：

```rust
use confers::watcher::{FsWatcher, WatcherConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let watcher_config = WatcherConfig::builder()
        .with_debounce(300) // 防抖间隔（毫秒）
        .build();

    // 以指定防抖间隔监听配置文件
    let mut watcher = FsWatcher::new("config.toml", watcher_config.debounce_ms).await?;

    // 每次文件变更都重新加载配置（load_sync 为 Config 派生宏生成的方法）
    while let Some(changed_path) = watcher.recv().await {
        println!("检测到配置变更: {:?}", changed_path);
        let config = AppConfig::load_sync()?;
        println!("配置已重载: {:?}", config);
    }

    Ok(())
}
```

**注意**：热重载功能需要启用 `watch` 特性。`ConfigBuilder::build_with_watcher()` 已弃用（不会在文件变更时重载），请直接使用 `FsWatcher`/`MultiFsWatcher`。

### 敏感数据加密

加密敏感配置值：

```rust
use confers::XChaCha20Crypto;
use serde::Deserialize;

#[derive(Deserialize)]
struct Secrets {
    // encrypt 属性需要显式指定算法
    #[config(encrypt = "xchacha20")]
    api_key: String,

    #[config(encrypt = "xchacha20")]
    database_password: String,
}

fn decrypt_secrets() -> Result<(), Box<dyn std::error::Error>> {
    let crypto = XChaCha20Crypto::new();
    let key = load_encryption_key()?; // 32 字节密钥

    let (nonce, ciphertext) = crypto.encrypt(b"my-secret-api-key", &key)?;
    let decrypted = crypto.decrypt(&nonce, &ciphertext, &key)?;

    Ok(())
}
```

### 性能优化

#### 异步加载

> 💡 **提示**：对于大型配置或远程配置来源，请始终使用异步加载：

```rust
use confers::ConfigBuilder;

fn load_config_efficiently() -> Result<(), Box<dyn std::error::Error>> {
    let start = std::time::Instant::now();

    let config = ConfigBuilder::<AppConfig>::new()
        .file("config.toml")
        .env()
        .build()?;

    let elapsed = start.elapsed();
    println!("Configuration loading time: {:?}", elapsed);

    Ok(())
}
```

#### 配置缓存

对频繁访问的配置使用内存缓存：

```rust
use std::sync::Arc;
use tokio::sync::RwLock;
use confers::ConfigBuilder;

struct CachedConfig {
    cache: Arc<RwLock<Option<AppConfig>>>,
}

impl CachedConfig {
    fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(None)),
        }
    }

    async fn get(&self) -> Result<AppConfig, Box<dyn std::error::Error>> {
        {
            let cached = self.cache.read().await;
            if let Some(config) = &*cached {
                return Ok(config.clone());
            }
        }

        let config = ConfigBuilder::<AppConfig>::new()
            .file("config.toml")
            .env()
            .build()?;

        {
            let mut writer = self.cache.write().await;
            *writer = Some(config.clone());
        }

        Ok(config)
    }
}
```

### 安全注意事项

#### 敏感数据处理

**如何识别敏感字段：**

- 密码、API 密钥、访问令牌
- 数据库连接字符串
- 私钥、证书
- 个人身份信息（PII）

**如何加密敏感数据：**

```rust
use confers::XChaCha20Crypto;

// 创建加密器
let crypto = XChaCha20Crypto::new();

// 生成或加载 32 字节密钥
let key = load_encryption_key()?;

// 加密敏感值
let (nonce, ciphertext) = crypto.encrypt(b"my-secret-api-key", &key)?;
println!("Encrypted {} bytes", ciphertext.len());

// 解密敏感值
let decrypted = crypto.decrypt(&nonce, &ciphertext, &key)?;
assert_eq!(decrypted, b"my-secret-api-key");
```

**⚠️ 安全提示：**

- 🔴 **密钥管理**：加密密钥必须安全存放，绝不提交到版本控制系统
- 🔴 **密钥轮换**：生产环境建议定期轮换密钥
- 🔴 **密钥长度**：XChaCha20-Poly1305 的密钥必须恰好 32 字节（256 位）
- 🔴 **nonce 存储**：nonce 须与密文一同存储以供解密

#### 密钥管理

**如何生成安全密钥：**

```rust
use rand::Rng;

// 生成安全的随机密钥
let mut key = [0u8; 32];
let mut rng = rand::thread_rng();
rng.fill(&mut key);

// 配合 XChaCha20Crypto 使用
let crypto = XChaCha20Crypto::new();
let (nonce, ciphertext) = crypto.encrypt(b"secret", &key)?;
```

**如何轮换密钥：**

```rust
use confers::key::KeyManager;

let mut km = KeyManager::new()?;
let master_key = load_master_key()?; // 从安全存储加载主密钥

// 轮换密钥
let result = km.rotate_key(
    &master_key,
    Some("production".to_string()),
    "security-team".to_string(),
    Some("Scheduled key rotation".to_string())
)?;

println!("Key version rotated from {} to {}", result.previous_version, result.new_version);
```

**⚠️ 安全提示：**

- 🔴 **密钥存储**：使用硬件安全模块（HSM）或密钥管理服务
- 🔴 **密钥轮换**：建议每 90 天轮换一次密钥
- 🔴 **密钥备份**：安全备份密钥，确保可以恢复
- 🔴 **密钥泄露应急处置**：密钥泄露后立即轮换并通知相关团队

#### 审计日志配置

**如何启用审计日志：**

```rust
use confers::audit::{AuditWriter, AuditConfig};
use std::path::PathBuf;

// 创建审计配置
let audit_config = AuditConfig::builder()
    .log_dir(PathBuf::from("/var/log/confers"))
    .enabled(true)
    .build();

// 创建审计写入器
let writer = AuditWriter::builder()
    .log_dir(PathBuf::from("/var/log/confers"))
    .enabled(true)
    .build();

// 记录审计事件
writer.log_load("config.toml");
writer.log_key_access("database_password");
writer.log_decrypt("api_key", true);
```

**⚠️ 安全提示：**

- 🔴 **日志完整性**：审计日志使用 HMAC 签名保护完整性
- 🔴 **日志访问控制**：限制审计日志文件的访问权限（仅 root/管理员）
- 🔴 **日志归档**：定期归档审计日志，防止日志文件过大
- 🔴 **日志监控**：监控审计日志的访问记录，发现异常访问

#### 生产环境安全配置

**环境变量安全配置：**

```bash
# 使用环境变量存储敏感信息
export APP_DATABASE_URL="postgres://user:password@localhost/db"  # pragma: allowlist secret
export APP_API_KEY="your-api-key"  # pragma: allowlist secret
export CONFERS_ENCRYPTION_KEY="base64-encoded-key"  # pragma: allowlist secret
```

**远程配置安全配置：**

```rust
// 远程配置需要直接使用远程来源
use confers::remote::HttpPolledSourceBuilder;

let remote_source = HttpPolledSourceBuilder::new()
    .url("https://config.example.com")
    .timeout(std::time::Duration::from_secs(30))
    .build()?;

let config = ConfigBuilder::<AppConfig>::new()
    .source(Box::new(remote_source))
    .build()?;
```

**⚠️ 安全提示：**

- 🔴 **TLS 配置**：远程配置传输始终使用 TLS 加密
- 🔴 **访问控制**：限制远程配置服务的访问权限
- 🔴 **最小权限原则**：只授予必要的权限
- 🔴 **安全审计**：定期审计生产环境配置

#### API 方法安全注解

**加密 API：**

```rust
/// Encrypt sensitive configuration value
///
/// # Security Notes
///
/// - ⚠️ **Key Management**: The encryption key must be stored securely and never committed to version control
/// - ⚠️ **Key Rotation**: Regular key rotation is recommended for production environments
/// - ⚠️ **Key Length**: The key must be exactly 32 bytes (256 bits) for XChaCha20-Poly1305
/// - ⚠️ **Nonce Storage**: Store the nonce alongside the ciphertext for decryption
///
/// # Example
///
/// ```rust
/// let crypto = XChaCha20Crypto::new();
/// let (nonce, ciphertext) = crypto.encrypt(b"sensitive-data", &key)?;
/// ```
pub fn encrypt(&self, plaintext: &[u8], key: &[u8]) -> Result<(Vec<u8>, Vec<u8>), CryptoError>
```

**密钥管理 API：**

```rust
/// Initialize new keyring
///
/// # Security Notes
///
/// - ⚠️ **Master Key**: The master key must be stored securely and never shared
/// - ⚠️ **Key ID**: Use descriptive key IDs (e.g., "production", "staging")
/// - ⚠️ **Created By**: Include creator information for audit trail
/// - ⚠️ **Key Backup**: Ensure you have a secure backup of the master key
///
/// # Example
///
/// ```rust
/// let version = km.initialize(
///     &master_key,
///     "production".to_string(),
///     "security-team".to_string()
/// )?;
/// ```
pub fn initialize(
    &mut self,
    master_key: &[u8; 32],
    key_id: String,
    created_by: String,
) -> Result<KeyVersion, ConfigError>
```

**审计日志 API：**

```rust
/// Log configuration loading event
///
/// # Security Notes
///
/// - ⚠️ **Log Path**: Store audit logs in a secure location with restricted access
/// - ⚠️ **Log Rotation**: Configure log rotation to prevent disk space exhaustion
/// - ⚠️ **Log Integrity**: Audit logs are signed to prevent tampering
/// - ⚠️ **Log Monitoring**: Monitor audit logs for suspicious activity
///
/// # Example
///
/// ```rust
/// use confers::audit::{AuditWriter, AuditConfig};
/// use std::path::PathBuf;
///
/// let writer = AuditWriter::builder()
///     .log_dir(PathBuf::from("/var/log/confers"))
///     .enabled(true)
///     .build();
///
/// writer.log_load("config.toml");
/// ```
pub fn log_load(&self, source: &str) -> ConfigResult<()>
```

**配置校验 API：**

校验通过 `validation` 特性与 `#[config(validate)]` 属性启用，在 `ConfigBuilder::build()` 阶段自动执行；失败时返回 `ConfigError::ValidationFailed`。

```rust
/// Configuration validation using garde derive macro
///
/// # Security Notes
///
/// - ⚠️ **Input Validation**: Always validate user input before use
/// - ⚠️ **Range Checking**: Ensure numeric values are within expected ranges
/// - ⚠️ **Error Messages**: Avoid exposing sensitive information in error messages
/// - ⚠️ **Validation Failures**: Treat validation failures as potential security incidents
///
/// # Example
///
/// ```rust
/// use confers::Config;
/// use garde::Validate;
/// use serde::Deserialize;
///
/// #[derive(Config, Validate, Deserialize)]
/// #[config(validate)]
/// struct ServerConfig {
///     #[garde(range(min = 1, max = 65535))]
///     port: u16,
/// }
///
/// // 校验在 build() 阶段自动执行
/// let config = ConfigBuilder::<ServerConfig>::new()
///     .file("config.toml")
///     .build()?;
/// ```
```

### 常见问题排查

#### 常见问题

| 问题 | 解决方案 |
|------|----------|
| **问：找不到配置文件？** | 检查文件路径是否正确；使用 `.file("config.toml")` 显式指定路径，可选文件用 `.file_optional()`（不存在时静默跳过），或使用 `.source()` 接入自定义来源。 |
| **问：环境变量不生效？** | 确认已调用 `.env()` 或 `.env_prefix("PREFIX")`，并检查环境变量名是否使用了正确的前缀。例如配置字段 `port` 对应的环境变量名为 `<PREFIX>_PORT`。 |
| **问：加密/解密失败？** | 确保加密与解密使用同一 32 字节密钥；使用 `derive_field_key` 派生字段密钥时，`field_path` 与 `key_version` 必须与加密时完全一致。 |
| **问：配置校验失败？** | 查看详细的校验错误信息，确保配置值满足全部校验约束。检查字段类型是否匹配。 |
| **问：远程配置加载超时？** | 检查网络连接与远程服务可用性，在构建来源时配置超时：`HttpPolledSourceBuilder::new().url(url).timeout(Duration::from_secs(60)).build()?`。 |
| **问：内存占用过高？** | 使用 `.limits(ConfigLimits { .. })` 设置文件大小、嵌套深度、键数量等上限，优化配置文件体积，避免在配置中存储大型二进制数据。 |

#### 调试日志

confers 核心不绑定特定日志门面，错误统一通过 `ConfigResult` / `ConfersError` 返回，便于精确处理。

仓库的示例程序使用 `tracing` + `tracing-subscriber` 输出运行状态，可配合 `RUST_LOG` 控制日志级别：

```bash
# 以 debug 级别运行某个示例
RUST_LOG=debug cargo run -p confers-examples --bin basic_usage
```

---

### 💝 感谢使用 Confers！

如有疑问或建议，请访问 [GitHub 仓库](https://github.com/Kirky-X/confers)。

**[🏠 返回首页](../README.md)** • **[📖 用户指南](USER_GUIDE.md)**
