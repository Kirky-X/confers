# 🧩 Confers 宏指南

本文档完整介绍 `#[derive(Config)]` 宏的用法。该宏是 Confers 库的核心，能为 Rust 结构体自动生成完整的配置管理功能。宏实现位于 `macros/src/lib.rs`，通过 `codegen.rs` 与 `parse.rs` 完成代码生成。

## 📋 目录

<details open>
<summary>📑 目录（点击展开）</summary>

- [概述](#-概述)
- [结构体级属性](#️-结构体级属性)
- [字段级属性](#️-字段级属性)
- [综合示例](#-综合示例)
- [自动生成的方法](#️-自动生成的方法)
- [完整使用示例](#-完整使用示例)
- [属性速查表](#-属性速查表)
- [使用 Garde 进行校验](#-使用-garde-进行校验)
- [特性依赖](#-特性依赖)
- [最佳实践](#-最佳实践)
- [故障排查](#-故障排查)

</details>

---

## 🎯 概述

`#[derive(Config)]` 是 Confers 库的核心宏。它为 Rust 结构体自动生成完整的配置管理功能。该宏位于 `macros/src/lib.rs`，通过 `codegen.rs` 与 `parse.rs` 实现代码生成。

---

## 🏗️ 结构体级属性

### 1.1 启用校验

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
#[config(validate)]  // 启用配置校验
pub struct AppConfig {
    pub name: String,
    pub port: u16,
}
```

**效果**：
- 自动实现 `validator::Validate` trait
- 调用 `config.validate()` 时对所有字段进行校验

---

### 1.2 环境变量前缀

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
#[config(env_prefix = "APP_")]  // 读取 APP_NAME、APP_PORT 等
pub struct AppConfig {
    pub name: String,
    pub port: u16,
}
```

**效果**：
- 读取环境变量时附加前缀
- 例如：`APP_NAME=myapp` 映射到 `name` 字段

---

### 1.3 应用名称

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
#[config(app_name = "myapp")]  // 配置目录名
pub struct AppConfig {
    pub name: String,
}
```

**效果**：
- 指定搜索配置文件时使用的目录名
- 会在 `~/.config/myapp/`、`/etc/myapp/` 等路径中搜索

---

### 1.4 严格模式

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
#[config(strict = true)]  // CLI 参数解析出错时直接报错退出
pub struct AppConfig {
    pub name: String,
}
```

**效果**：
- CLI 参数解析失败时返回错误
- 非严格模式会忽略错误

---

### 1.5 文件监听（热重载）

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
#[config(watch = true)]  // 启用文件监听
pub struct AppConfig {
    #[config(default = 8080)]
    pub port: u16,
}
```

**效果**：
- 需要启用 `watch` 特性
- 使用 `ConfigBuilder::build_with_watcher()` 获取监听器

---

### 1.6 配置版本

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
#[config(version = 2)]  // 用于迁移的配置版本号
pub struct AppConfig {
    pub name: String,
}
```

**效果**：
- 与配置迁移（migration）配合使用
- 支持模式演进的版本追踪

---

## 🏷️ 字段级属性

### 2.1 默认值

**方式一：新语法（推荐）**
```rust
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
pub struct AppConfig {
    #[config(default = "default_value")]
    pub name: String,

    #[config(default = 8080)]
    pub port: u32,

    #[config(default = 3.14)]
    pub rate: f64,

    #[config(default = true)]
    pub debug: bool,
}
```

**方式二：字符串类型的旧语法**
```rust
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
pub struct AppConfig {
    #[config(default = "\"default_value\".to_string()")]
    pub name: String,
}
```

**效果**：
- 配置文件中缺少该字段时使用默认值
- 自动实现 `Default` trait

---

### 2.2 字段描述

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
pub struct AppConfig {
    #[config(description = "Server port number")]
    pub port: u16,

    #[config(description = "Database connection URL")]
    pub database_url: String,
}
```

**效果**：
- 生成 CLI 帮助信息
- 用于 JSON Schema 生成

---

### 2.3 配置键名映射

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
pub struct AppConfig {
    #[config(name = "app_name")]  // 配置文件中使用 app_name
    pub name: String,
}
```

**效果**：
- 字段名为 `name`，但配置键为 `app_name`

---

### 2.4 环境变量名映射

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
#[config(env_prefix = "APP")]
pub struct AppConfig {
    #[config(name_env = "CUSTOM_PORT")]  // 读取 APP_CUSTOM_PORT
    pub port: u16,
}
```

**优先级**：`name_env` > 自动推导

---

### 2.5 CLI 参数名

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
pub struct AppConfig {
    #[config(name_clap_long = "server-port")]
    pub port: u16,

    #[config(name_clap_short = 'p')]
    pub port2: u16,
}
```

**效果**：
- CLI 参数：`--server-port` 或 `-p`

---

### 2.6 校验规则

Confers 使用 `garde` 校验库。要启用校验，请派生 `garde::Validate` 并在字段上添加校验属性：

**范围校验**
```rust
use confers::Config;
use garde::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, Config, Validate)]
#[config(validate)]
pub struct AppConfig {
    #[garde(range(min = 1, max = 65535))]
    pub port: u16,

    #[garde(range(min = 0, max = 100))]
    pub rate: i32,
}
```

**长度校验**
```rust
#[derive(Debug, Clone, Serialize, Deserialize, Config, Validate)]
#[config(validate)]
pub struct AppConfig {
    #[garde(length(min = 3, max = 50))]
    pub username: String,
}
```

**内置校验器**
```rust
#[derive(Debug, Clone, Serialize, Deserialize, Config, Validate)]
#[config(validate)]
pub struct AppConfig {
    #[garde(email)]
    pub email: String,

    #[garde(url)]
    pub website: String,
}
```

**注意**：`#[config(validate)]` 属性用于在构建时启用校验，而具体的校验规则通过 `garde` crate 的 `#[garde(...)]` 属性指定。

---

### 2.7 敏感字段

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
pub struct AppConfig {
    #[config(sensitive = true)]
    pub password: String,

    #[config(sensitive = true)]
    pub api_key: String,
}
```

**效果**：
- 在审计日志中自动脱敏
- 敏感信息不会以明文输出

---

### 2.8 扁平化字段

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
pub struct DatabaseConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize, Config)]
pub struct AppConfig {
    #[config(flatten)]
    pub database: DatabaseConfig,

    pub app_name: String,
}
```

**效果**：
- 嵌套结构的字段被提升到顶层
- 同时支持 `database.host` 与 `database_host` 两种访问方式

**与 serde 集成**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NestedConfig {
    #[serde(flatten)]
    pub inner: InnerConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Config)]
pub struct InnerConfig {
    pub value: String,
}
```

---

### 2.9 跳过字段

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
pub struct AppConfig {
    pub name: String,

    #[config(skip)]
    pub temp_field: String,  // 不会从配置中加载
}
```

**效果**：
- 该字段不会从配置文件加载
- 使用结构体的默认值

---

### 2.10 加密字段

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
pub struct AppConfig {
    #[config(encrypt = "xchacha20")]
    pub database_password: String,

    #[config(encrypt = "xchacha20")]
    pub api_key: String,
}
```

**效果**：
- 加载时自动解密字段值
- 需要启用 `encryption` 特性
- 使用 XChaCha20-Poly1305 加密算法

---

### 2.11 变量插值

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
pub struct AppConfig {
    #[config(interpolate)]
    pub database_url: String,  // 支持 ${VAR} 语法
}
```

**效果**：
- 为该字段启用变量插值
- 支持 `${VAR}` 与 `${VAR:-default}` 语法
- 需要 `interpolation` 特性

---

### 2.12 合并策略

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
pub struct AppConfig {
    #[config(merge_strategy = "append")]
    pub hosts: Vec<String>,

    #[config(merge_strategy = "deep_merge")]
    pub settings: HashMap<String, String>,
}
```

**可用策略**：
- `replace`：替换已有值（默认）
- `append`：追加到数组
- `prepend`：前插到数组
- `join`：拼接数组值
- `deep_merge`：深度合并映射

---

### 2.13 动态字段

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
pub struct AppConfig {
    #[config(dynamic)]
    pub feature_flags: HashMap<String, bool>,
}
```

**效果**：
- 生成用于运行时更新的 `DynamicField` 句柄
- 需要 `dynamic` 特性
- 支持可热更新的配置分区

---

### 2.14 模块分组

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
pub struct AppConfig {
    #[config(module_group = "database")]
    pub db_host: String,

    #[config(module_group = "database")]
    pub db_port: u16,
}
```

**效果**：
- 将相关字段分组，实现模块化配置
- 支持模块级重载与校验
- 需要 `modules` 特性

---

## 📦 综合示例

```rust
use confers::Config;
use garde::Validate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Config, Validate)]
#[config(
    validate,                                    // 启用校验
    env_prefix = "APP_",                         // 环境变量前缀
    app_name = "myapp",                         // 应用名称
    strict = false,                              // 非严格模式
    watch = false,                               // 不监听文件变更
    version = 1,                                 // 配置版本
)]
pub struct AppConfig {
    // ============ 基础类型 ============
    #[config(description = "Application name")]
    pub name: String,

    #[config(default = 8080, description = "Server port")]
    pub port: u16,

    #[config(default = false, description = "Debug mode")]
    pub debug: bool,

    // ============ 字符串类型 ============
    #[config(default = "\"localhost\".to_string()", description = "Server host")]
    pub host: String,

    // ============ 校验规则（使用 garde） ============
    #[garde(range(min = 1, max = 65535))]
    #[config(description = "Admin port")]
    pub admin_port: u16,

    #[garde(length(min = 3, max = 100))]
    #[config(description = "Username")]
    pub username: String,

    #[garde(email)]
    #[config(description = "Email address")]
    pub email: String,

    #[garde(url)]
    #[config(description = "Website URL")]
    pub website: String,

    // ============ 敏感字段 ============
    #[config(sensitive = true, description = "Database password")]
    pub db_password: String,

    #[config(sensitive = true, description = "API key")]
    pub api_key: String,

    // ============ 加密字段 ============
    #[config(encrypt = "xchacha20", description = "Secret token")]
    pub secret_token: String,

    // ============ 变量插值 ============
    #[config(interpolate, description = "Database URL")]
    pub database_url: String,

    // ============ 自定义映射 ============
    #[config(name_env = "CUSTOM_DATABASE_URL", description = "Custom database URL")]
    pub custom_db_url: String,

    // ============ 嵌套配置 ============
    #[config(flatten, description = "Database configuration")]
    pub database: DatabaseConfig,

    // ============ 跳过字段 ============
    #[config(skip)]
    pub runtime_data: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DatabaseConfig {
    pub host: String,
    pub port: u16,
    pub name: String,
}
```

---

## ⚙️ 自动生成的方法

使用 `#[derive(Config)]` 宏之后，结构体会自动获得以下方法：

### 4.1 配置构建器

```rust
use confers::ConfigBuilder;

// 使用 ConfigBuilder 进行基础加载
let config = ConfigBuilder::<AppConfig>::new()
    .file("config.toml")
    .env()
    .build()?;

// 带环境变量前缀
let config = ConfigBuilder::<AppConfig>::new()
    .file("config.toml")
    .env_prefix("APP_")
    .build()?;

// 支持热重载（需要 watch 特性）
let (rx, guard) = ConfigBuilder::<AppConfig>::new()
    .file("config.toml")
    .watch(true)
    .build_with_watcher().await?;
```

### 4.2 辅助函数

```rust
// 便捷的 config() 函数
let config = confers::config::<AppConfig>()
    .file("config.toml")
    .env()
    .build()?;
```

### 4.3 Schema 生成

```rust
// 生成 JSON Schema（需要 schema 特性）
// 注意：需要派生 ConfigSchema
let schema = AppConfig::json_schema();

// 生成 TypeScript 类型（需要 typescript-schema 特性）
let ts_type = AppConfig::typescript_type();
```

### 4.4 其他方法

```rust
// 获取默认值
let default = AppConfig::default();

// 访问配置值
let value = config.some_field;
```

---

## 💡 完整使用示例

### 5.1 基础用法

**定义配置结构体**
```rust
use confers::Config;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Config)]
#[config(validate)]
#[config(env_prefix = "APP_")]
pub struct ServerConfig {
    pub host: String,

    #[config(default = 8080)]
    pub port: u16,

    #[config(default = true)]
    pub enabled: bool,
}
```

**创建配置文件 `config.toml`**
```toml
host = "0.0.0.0"
port = 9000
enabled = false
```

**使用配置**
```rust
use confers::ConfigBuilder;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ConfigBuilder::<ServerConfig>::new()
        .file("config.toml")
        .env_prefix("APP_")
        .build()?;

    println!("Host: {}", config.host);
    println!("Port: {}", config.port);
    println!("Enabled: {}", config.enabled);

    Ok(())
}
```

**环境变量覆盖**
```bash
export APP_PORT=3000
export APP_ENABLED=true
cargo run
```

### 5.2 敏感配置加密

```rust
use confers::Config;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Config)]
pub struct SecureConfig {
    #[config(sensitive = true)]
    pub password: String,

    #[config(encrypt = "xchacha20")]
    pub api_secret: String,
}
```

**加密使用 XChaCha20-Poly1305 算法。nonce 与密文一同存储。**

### 5.3 热重载

```rust
use confers::ConfigBuilder;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[derive(Debug, Clone, Serialize, Deserialize, Config)]
    #[config(watch = true)]
    pub struct HotReloadConfig {
        #[config(default = 8080)]
        pub port: u16,
    }

    let (rx, guard) = ConfigBuilder::<HotReloadConfig>::new()
        .file("config.toml")
        .watch(true)
        .build_with_watcher().await?;

    let config = rx.borrow().clone();
    println!("Initial port: {}", config.port);

    // 应用持续运行……
    // 配置文件变更时，rx 会收到更新

    Ok(())
}
```

---

## 📑 属性速查表

### 结构体级属性

| 属性 | 用途 |
|------|------|
| `validate` | 启用配置校验（需要派生 garde::Validate） |
| `env_prefix` | 环境变量前缀 |
| `app_name` | 应用名称（配置目录） |
| `strict` | CLI 解析严格模式 |
| `watch` | 启用文件监听 |
| `version` | 用于迁移的配置版本号 |

### 字段级属性

| 属性 | 用途 |
|------|------|
| `default` | 默认值表达式 |
| `description` | 用于文档的字段描述 |
| `name` | 覆盖配置键名 |
| `name_env` | 覆盖环境变量名 |
| `name_clap_long` | CLI 长参数名 |
| `name_clap_short` | CLI 短参数字符 |
| `sensitive` | 标记为敏感字段（日志中隐藏） |
| `encrypt` | 加密算法（如 "xchacha20"） |
| `flatten` | 扁平化嵌套配置 |
| `skip` | 加载时跳过该字段 |
| `interpolate` | 启用变量插值 |
| `merge_strategy` | 多来源合并策略 |
| `dynamic` | 生成 DynamicField 句柄 |
| `module_group` | 模块化配置的分组 |

---

## ✅ 使用 Garde 进行校验

校验由 `garde` crate 负责。请派生 `garde::Validate` 并使用 `#[garde(...)]` 属性：

### 7.1 范围校验

```rust
#[garde(range(min = 1, max = 65535))]
pub port: u16,
```

支持的数据类型：
- u8, u16, u32, u64, u128, usize
- i8, i16, i32, i64, i128, isize
- f32, f64

### 7.2 长度校验

```rust
#[garde(length(min = 0, max = 100))]
pub username: String,
```

支持：
- 字符串长度
- 数组长度

### 7.3 内置校验器

**邮箱校验**
```rust
#[garde(email)]
pub email: String,
```

**URL 校验**
```rust
#[garde(url)]
pub website: String,
```

**模式校验**
```rust
#[garde(pattern(r"^[A-Z]{2}\d{6}$"))]
pub id_code: String,
```

### 7.4 自定义校验

```rust
#[garde(custom(my_validator))]
pub field: String,

fn my_validator(value: &str, _: &garde::ValidateContext) -> garde::Result {
    if value.contains("invalid") {
        return Err(garde::Error::new("value contains invalid content"));
    }
    Ok(())
}
```

---

## 🎨 特性依赖

| 属性/方法 | 所需特性 |
|-----------|----------|
| `#[config(validate)]` | `validation` |
| `#[config(watch = true)]` | `watch` |
| `json_schema()` | `schema` |
| `typescript_type()` | `typescript-schema` |
| CLI 参数支持 | `cli` |
| 加密支持 | `encryption` |
| 远程配置 | `remote` |
| 变量插值 | `interpolation` |
| 动态字段 | `dynamic` |
| 模块分组 | `modules` |

---

## 🌟 最佳实践

### 9.1 推荐配置

```toml
# Cargo.toml
[dependencies]
confers = { version = "0.6.0-rc.2", features = ["recommended"] }
garde = { version = "0.22", features = ["derive"] }
```

`recommended` 特性包含：`toml`、`json`、`env`、`validation`

### 9.2 开发环境配置

```toml
# Cargo.toml
[dependencies]
confers = { version = "0.6.0-rc.2", features = ["dev"] }
garde = { version = "0.22", features = ["derive"] }
```

`dev` 特性包含面向开发便利的大多数特性。

### 9.3 生产环境配置

```toml
# Cargo.toml
[dependencies]
confers = { version = "0.6.0-rc.2", features = ["production"] }
garde = { version = "0.22", features = ["derive"] }
```

`production` 特性包含：`toml`、`env`、`watch`、`encryption`、`validation`、`audit`、`schema`、`cli`、`migration`、`dynamic`、`progressive-reload`、`snapshot`

---

## 🔧 故障排查

### 10.1 常见问题

**问：配置值加载不正确？**
答：检查环境变量前缀是否正确，并确认配置文件格式匹配。

**问：校验失败但不知道原因？**
答：使用 `strict = true` 模式查看详细的错误信息。

**问：敏感字段在日志中泄露？**
答：确保使用 `sensitive = true` 属性标记敏感字段。

**问：热重载不生效？**
答：确保已启用 `watch` 特性，并且使用的是 `load_with_watcher()` 方法。

---

*本文档基于 Confers v0.5.0 编写。*
