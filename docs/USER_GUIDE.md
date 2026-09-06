# 📖 Confers 用户指南

**confers** 是一个功能强大的 Rust 配置管理库，旨在简化应用的配置加载、校验与管理。它支持从文件（JSON、TOML、YAML）、环境变量、命令行参数以及远程来源（Etcd、HTTP）加载配置。本指南将带您从安装入门一路走到进阶用法与最佳实践。

## 📋 目录

<details open>
<summary>📑 目录（点击展开）</summary>

- [简介](#-简介)
- [快速开始](#-快速开始)
  - [前置条件](#-前置条件)
  - [安装](#-安装)
  - [第一步](#-第一步)
- [核心概念](#-核心概念)
  - [Config 派生宏](#config-派生宏)
  - [分层加载](#分层加载)
  - [灵活的数据来源](#灵活的数据来源)
  - [配置文件搜索路径](#配置文件搜索路径)
- [配置](#️-配置)
  - [定义配置结构体](#-定义配置结构体)
  - [加载配置](#-加载配置)
  - [默认值与环境变量](#-默认值与环境变量)
  - [命令行工具](#-命令行工具)
- [进阶用法](#-进阶用法)
  - [校验与清洗](#-校验与清洗)
  - [远程配置（Etcd/Consul/HTTP）](#️-远程配置etcdconsulhttp)
  - [审计日志与安全](#-审计日志与安全)
  - [文件监听与热重载](#-文件监听与热重载)
  - [敏感数据加密](#-敏感数据加密)
- [最佳实践](#-最佳实践)
  - [推荐的设计模式](#-推荐的设计模式)
  - [安全配置实践](#-安全配置实践)
- [故障排查](#-故障排查)

</details>

---

## 🎯 简介

本指南将带您掌握：

| 内容 | 说明 |
|:-----|:-----|
| **快速开始** | 5 分钟完成环境搭建 |
| **灵活配置** | 支持多种来源与格式 |
| **最佳实践** | 学习规范的配置管理方式 |
| **进阶特性** | 掌握热重载与远程配置 |

**confers** 是一个功能强大的 Rust 配置管理库，旨在简化应用的配置加载、校验与管理。它支持从文件（JSON、TOML、YAML）、环境变量、命令行参数以及远程来源（Etcd、HTTP）加载配置。

> 💡 **提示**：本指南假设您具备基础的 Rust 知识。如果您是 Rust 新手，建议先阅读 [Rust 官方教程](https://doc.rust-lang.org/book/)。

---

## 🚀 快速开始

### 📌 前置条件

开始之前，请确认已安装以下工具：

**必装**

- ✅ Rust 1.97.1+（stable）
- ✅ Cargo（随 Rust 一起安装）
- ✅ Git

**可选**

- 🔧 支持 Rust 的 IDE（如 VS Code + rust-analyzer）
- 🔧 Docker（用于容器化部署）
- 🔧 Etcd（用于远程配置测试）

<details>
<summary>🔍 验证安装</summary>

```bash
# 检查 Rust 版本
rustc --version
# 期望输出：rustc 1.97.1（或更高）

# 检查 Cargo 版本
cargo --version
# 期望输出：cargo 1.88.0（或更高）
```

</details>

### 📦 安装

将 `confers` 加入您的 `Cargo.toml`：

| 安装方式 | 配置 | 适用场景 |
|----------|------|----------|
| **默认** | `confers = "0.6.0-rc.2"` | 包含 toml、json、env |
| **最小化** | `confers = { version = "0.6.0-rc.2", default-features = false, features = ["minimal"] }` | 仅环境变量 |
| **推荐** | `confers = { version = "0.6.0-rc.2", default-features = false, features = ["recommended"] }` | TOML + JSON + Env + 校验 |
| **全量** | `confers = { version = "0.6.0-rc.2", features = ["full"] }` | 全部特性 |

**可用的特性预设：**

| 预设 | 特性 | 适用场景 |
|------|------|----------|
| `minimal` | `env`、`json` | 环境变量 + JSON |
| `recommended` | `toml`、`json`、`env`、`validation` | 配置加载 + 校验 |
| `dev` | `toml`、`json`、`yaml`、`env`、`cli`、`validation`、`schema`、`audit`、`watch`、`migration`、`snapshot`、`dynamic` | 全工具开发环境 |
| `production` | `toml`、`env`、`watch`、`encryption`、`validation`、`audit`、`schema`、`cli`、`migration`、`dynamic`、`progressive-reload`、`snapshot` | 生产就绪配置 |
| `distributed` | `toml`、`env`、`watch`、`validation`、`config-bus`、`progressive-reload`、`audit` | 分布式系统 |
| `full` | 全部特性 | 完整功能集 |

**单项特性：**

| 特性 | 说明 | 默认启用 |
|------|------|----------|
| `toml` | TOML 格式支持 | ✅ |
| `json` | JSON 格式支持 | ✅ |
| `env` | 环境变量支持 | ✅ |
| `yaml` | YAML 格式支持 | ❌ |
| `validation` | 配置校验（garde） | ❌ |
| `cli` | 命令行工具 | ❌ |
| `watch` | 文件监听与热重载 | ❌ |
| `audit` | 审计日志 | ❌ |
| `schema` | JSON Schema 生成 | ❌ |
| `remote` | 远程配置（etcd、consul、http） | ❌ |
| `encryption` | 配置加密 | ❌ |

如果需要异步/远程支持，请添加 tokio：

```toml
[dependencies]
tokio = { version = "1.0", features = ["full"] }
```

### 💡 第一步

让我们用一个简单示例验证安装。我们将定义一个带默认值和环境变量映射的配置结构体：

```rust
use confers::{Config, ConfigBuilder};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Config)]
#[config(env_prefix = "APP")]
struct AppConfig {
    #[config(default = 8080)]
    port: u16,

    #[config(default = "\"localhost\".to_string()")]
    host: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 使用 ConfigBuilder 加载配置
    let config = ConfigBuilder::<AppConfig>::new()
        .file("config.toml")
        .env_prefix("APP_")
        .build()?;

    println!("🚀 Server running at: {}:{}", config.host, config.port);
    Ok(())
}

// 或者使用 FsWatcher 实现真正的热重载（推荐）
#[tokio::main]
async fn async_main() -> Result<(), Box<dyn std::error::Error>> {
    use confers::watcher::FsWatcher;
    let config = ConfigBuilder::<AppConfig>::new()
        .file("config.toml")
        .build()?;

    let mut watcher = FsWatcher::new("config.toml", 200).await?;
    while watcher.recv().await.is_some() {
        // 配置文件变更 —— 重建并应用
        let _new_config = ConfigBuilder::<AppConfig>::new()
            .file("config.toml")
            .build()?;
    }

    println!("🚀 Server running at: {}:{}", config.host, config.port);
    Ok(())
}
```

**说明**：`Config` 派生宏提供类型安全的配置。使用 `ConfigBuilder` 进行加载：
- `ConfigBuilder::<T>::new().build()` —— 同步加载
- `ConfigBuilder::<T>::new().build_with_fallback(fallback)` —— 带回退配置
- `ConfigBuilder::<T>::new().build_with_watcher().await` —— **⚠️ 自 0.3.0 起已弃用**（不再随文件变更重新加载；热重载请直接使用 `FsWatcher`/`MultiFsWatcher`）

---

## 🔑 核心概念

理解以下核心概念能帮助您更高效地使用 `confers`。

```mermaid
graph TB
    subgraph Sources ["配置来源"]
        A["配置文件<br/>JSON, TOML, YAML"]
        B["环境变量"]
        C["CLI 参数"]
        D["远程来源<br/>Etcd, Consul, HTTP"]
    end

    subgraph Priority ["优先级（从高到低）"]
        P1["CLI 参数<br/>最高优先级"]
        P2["环境变量"]
        P3["配置文件"]
        P4["默认值<br/>最低优先级"]
    end

    subgraph Result ["结果"]
        R["类型安全的配置"]
    end

    Sources --> Priority
    Priority --> R
```

### Config 派生宏

`confers` 的核心是 `Config` 派生宏。它会为您的结构体自动实现配置加载逻辑，包括处理默认值、环境变量前缀和校验规则。

### 分层加载

`confers` 遵循"最后定义者生效"原则，按以下优先级顺序合并配置：

1. **命令行参数**（最高优先级）
2. **环境变量**
3. **配置文件**（如 `config.toml`）
4. **默认值**（最低优先级）

### 灵活的数据来源

您可以轻松组合来自不同来源的配置：

- **文件**：支持自动探测 JSON、TOML、YAML 格式。
- **环境变量**：通过 `env_prefix` 自动映射环境变量。
- **远程**：支持 Etcd、Consul 以及 HTTP 轮询/监听。

### 配置文件搜索路径

`confers` 支持灵活的文件搜索策略，可按需在不同位置查找配置文件。

#### 默认搜索路径

使用 `Config::load_sync()` 或 `Config::create_loader()` 时，`confers` 按以下优先级搜索配置文件：

| 优先级 | 搜索路径 | 条件 | 文件格式 |
|--------|----------|------|----------|
| 1 | `./` | 总是 | `config.{toml,json,yaml,yml}` |
| 2 | `~/.config/<app_name>/` | 设置了 `app_name` | `config.{toml,json,yaml,yml}` |
| 3 | `~/.config/` | 总是 | `config.{toml,json,yaml,yml}` |
| 4 | `~/` | 总是 | `config.{toml,json,yaml,yml}` |
| 5 | `/etc/<app_name>/` | Unix 且设置了 `app_name` | `config.{toml,json,yaml,yml}` |

#### app_name 的作用

`app_name` 是可选的应用标识符，用于在系统标准目录中组织配置文件：

```rust
#[derive(Debug, Serialize, Deserialize, Config)]
#[config(app_name = "myapp")]  // ✅ 显式设置 app_name
pub struct AppConfig {
    pub host: String,
    pub port: u16,
}
```

**设置了 app_name 时的搜索路径**：

```
./myapp/config.toml              ✅
~/.config/myapp/config.toml      ✅
~/.config/config.toml            ✅
~/config.toml                    ✅
/etc/myapp/config.toml           ✅（Unix）
./config.toml                    ❌（不再搜索）
```

**未设置 app_name 时的搜索路径**：

```
./config.toml                    ✅
~/.config/config.toml            ✅
~/config.toml                    ✅
```

#### 配置文件命名规则

`confers` 支持以下配置文件命名模式：

```bash
# 标准配置文件
config.toml
config.json
config.yaml
config.yml

# 环境专属配置文件（设置了 RUN_ENV 环境变量时）
<app_name>.<env>.toml
# 示例：myapp.production.toml、myapp.development.json
```

#### 使用场景示例

**场景 1：使用系统标准目录的应用**

```rust
#[derive(Config)]
#[config(app_name = "my-awesome-app")]
pub struct ProductionConfig {
    pub database_url: String,
    pub max_connections: u32,
}
// 配置文件位于：~/.config/my-awesome-app/config.toml
```

**场景 2：使用当前目录的简单应用**

```rust
#[derive(Config)]
pub struct SimpleConfig {
    pub debug: bool,
    pub workers: usize,
}
// 配置文件位于：./config.toml（简单应用推荐）
```

**场景 3：指定确切路径**

```rust
#[derive(Config)]
pub struct AppConfig {
    pub name: String,
}

// 使用 ConfigBuilder 指定确切路径
let config = ConfigBuilder::<AppConfig>::new()
    .file("/etc/myapp/production.toml")
    .build()?;
```

**场景 4：环境专属配置**

```bash
# 设置运行环境
export RUN_ENV=production

# confers 会自动搜索：
# ./myapp.production.toml
# ~/.config/myapp.production.toml
# /etc/myapp.production.toml（Unix）
```

#### 最佳实践建议

1. **应用程序**：建议设置 `app_name` 以使用系统标准目录

   ```rust
   #[config(app_name = "your-app-name")]
   ```

2. **库/工具**：使用默认行为，在当前目录查找 `config.toml`

3. **测试/特殊需求**：使用 `load_file()` 指定确切路径

4. **跨平台应用**：设置 `app_name` 以获得最佳的跨平台兼容性

> 💡 **提示**：如果找不到配置文件，`confers` 会使用默认值继续加载（除非启用了严格模式）。如需精确控制配置文件路径，请使用 `Config::load_file()`。

---

## ⚙️ 配置

本节介绍如何定义配置结构体、加载配置以及使用配套的命令行工具。

### 🧱 定义配置结构体

使用 `#[derive(Config)]` 与 `#[config(...)]` 属性来配置您的结构体。也支持嵌套结构体：

```rust
use serde::Deserialize;
use confers::Config;

#[derive(Config, Deserialize)]
struct DatabaseConfig {
    #[config(default = "\"localhost\".to_string()")]
    host: String,
    #[config(default = "5432")]
    port: u16,
}

#[derive(Config, Deserialize)]
#[config(env_prefix = "MYAPP", strict = true)]
struct MyConfig {
    #[config(default = "100")]
    timeout_ms: u64,

    // 嵌套结构体
    db: DatabaseConfig,

    #[config(sensitive = true)] // 审计日志中会被脱敏
    api_key: String,
}
```

### 📥 加载配置

`confers` 提供 `ConfigBuilder` 实现灵活的配置加载：

```rust
use confers::ConfigBuilder;

// 基础同步加载
let config = ConfigBuilder::<MyConfig>::new()
    .file("config.toml")
    .build()?;

// 带环境变量与前缀
let config = ConfigBuilder::<MyConfig>::new()
    .file("config.toml")
    .env_prefix("MYAPP_")
    .build()?;

// 启用校验
let config = ConfigBuilder::<MyConfig>::new()
    .file("config.toml")
    .validate(true)
    .build()?;

// 支持热重载（异步）- 使用 FsWatcher（推荐）
#[cfg(feature = "watch")]
use confers::watcher::FsWatcher;
#[cfg(feature = "watch")]
let initial = ConfigBuilder::<MyConfig>::new()
    .file("config.toml")
    .validate(true)
    .build()?;
#[cfg(feature = "watch")]
let mut watcher = FsWatcher::new("config.toml", 200).await?;
```

### 🔢 默认值与环境变量

- **默认值**：使用 `#[config(default = ...)]` 属性。数值类型直接写数值；字符串使用表达式语法。
- **环境变量**：默认映射规则为 `PREFIX_FIELD_NAME`。例如 `MYAPP_TIMEOUT_MS` 映射到 `timeout_ms`。

### 💻 命令行工具

confers 提供功能齐全的命令行工具，支持配置文件生成、校验、加密、差异对比等能力。

#### 安装 CLI

```bash
# 从源码安装
cargo install confers

# 或从 crates.io 安装
cargo install confers-cli

# 检查版本
confers --version

# 查看帮助
confers --help
```

#### 命令参考

```bash
confers 0.6.0-rc.2
A powerful Rust configuration management library

USAGE:
    confers [OPTIONS] <SUBCOMMAND>

OPTIONS:
    -h, --help         Print help information
    -V, --version      Print version information
    -v, --verbose      Enable verbose output (-vv for more detail)

SUBCOMMANDS:
    diff       Compare differences between two configuration files
    generate   Generate configuration template
    validate   Validate configuration file
    encrypt    Encrypt sensitive configuration
    wizard     Interactive configuration generation wizard
    key        Generate and manage encryption keys
    help       Print help information
```

#### diff - 配置差异对比

对比两个配置文件之间的差异，支持多种输出格式：

```bash
# 基础用法 - 对比两个配置文件
confers diff config1.toml config2.toml

# 指定输出格式
confers diff config1.toml config2.toml --format unified    # unified diff 格式
confers diff config1.toml config2.toml --format context    # context diff 格式
confers diff config1.toml config2.toml --format normal     # 标准 diff 格式
confers diff config1.toml config2.toml --format side-by-side  # 并排对比格式
confers diff config1.toml config2.toml --format strict     # 严格模式

# 生成报告
confers diff config1.toml config2.toml -o diff_report.md

# 查看详细帮助
confers diff --help
```

**输出格式说明：**

| 格式 | 说明 | 适用场景 |
|------|------|----------|
| `unified` | 带行号与上下文的 unified diff 格式 | 代码评审、版本对比 |
| `context` | context diff 格式 | 查看变更上下文 |
| `normal` | 标准 diff 格式 | 简单差异对比 |
| `side-by-side` | 并排对比格式 | 可视化对比 |
| `strict` | 严格模式，仅显示真实差异 | 精确差异分析 |

#### generate - 模板生成

```bash
# 基础用法
confers generate --struct "AppConfig" --output config_template.toml

# 指定输出格式
confers generate --struct "AppConfig" --format toml --output config.toml
confers generate --struct "AppConfig" --format yaml --output config.yaml
confers generate --struct "AppConfig" --format json --output config.json

# 指定输出级别
confers generate --struct "AppConfig" --level minimal    # 最小输出
confers generate --struct "AppConfig" --level full       # 完整输出
confers generate --struct "AppConfig" --level doc        # 带文档输出

# 查看详细帮助
confers generate --help
```

**输出级别说明：**

| 级别 | 说明 | 适用场景 |
|------|------|----------|
| `minimal` | 仅必填字段与注释 | 快速上手 |
| `full` | 全部字段、默认值与注释 | 完整配置 |
| `doc` | 包含字段描述 | 文档生成 |

#### validate - 配置校验

```bash
# 基础用法 - 校验配置文件
confers validate config.toml

# 指定输出级别
confers validate config.toml --level minimal    # 最小输出
confers validate config.toml --level full       # 完整输出
confers validate config.toml --level doc        # 带文档输出

# 跳过严格模式
confers validate config.toml --no-strict

# 校验并生成报告
confers validate config.toml -o validation_report.md

# 查看详细帮助
confers validate --help
```

#### encrypt - 配置加密

```bash
# 加密配置文件
confers encrypt input.toml --key-file secret.key --output encrypted.toml

# 加密单个值
confers encrypt "sensitive_value" --key-file secret.key

# 解密配置文件
confers encrypt encrypted.toml --key-file secret.key --decrypt --output decrypted.toml

# 查看详细帮助
confers encrypt --help
```

**使用示例：**

```bash
# 生成密钥并加密
confers key -o secret.key
confers encrypt config.toml --key-file secret.key -o config.encrypted.toml

# 解密以使用
confers encrypt config.encrypted.toml --key-file secret.key --decrypt -o config.toml
```

#### wizard - 交互式向导

```bash
# 启动交互式向导
confers wizard

# 指定配置文件类型
confers wizard --format toml
confers wizard --format yaml
confers wizard --format json

# 查看详细帮助
confers wizard --help
```

**向导流程：**

1. 输入配置名称
2. 设置服务器参数（host、port）
3. 配置数据库连接（url、pool）
4. 配置日志级别
5. 生成配置文件

#### key - 密钥管理

```bash
# 生成新密钥
confers key -o encryption.key

# 生成 256 位密钥
confers key --length 256 -o encryption.key

# 从口令派生密钥
confers key --derive --password "your_password" -o derived.key

# 查看密钥信息
confers key --info encryption.key

# 查看详细帮助
confers key --help
```

---

## 🚧 进阶用法

### ✅ 校验与清洗

`confers` 与 `garde` 校验库集成：

```rust
use garde::Validate;

#[derive(Config, Deserialize, Validate)]
#[config(validate)] // 启用自动校验
struct MyConfig {
    #[garde(range(min = 1, max = 65535))]
    port: u16,

    #[garde(email)]
    admin_email: String,
}
```

**注意**：请在依赖中添加 `garde = { version = "0.22", features = ["derive"] }`。

### ☁️ 远程配置（Etcd/Consul/HTTP）

> ⚠️ **注意**：以下功能需要启用 `remote` 特性。

启用 `remote` 特性后，可以从远程来源加载配置：

```rust
// 注意：远程配置也可以实现自定义 Source
// 完整实现参见 examples 目录

// 使用 HTTP 轮询来源（内置）
#[cfg(feature = "remote")]
use confers::remote::HttpPolledSourceBuilder;

#[cfg(feature = "remote")]
let http_source = HttpPolledSourceBuilder::new()
    .url("https://api.example.com/config")
    .bearer_token("your-token")
    .build()?;

#[cfg(feature = "remote")]
let config = ConfigBuilder::<MyConfig>::new()
    .source(Box::new(http_source))
    .build()?;
    .await?;
```

### 📝 审计日志与安全

> 📝 **提示**：以下功能需要启用 `audit` 特性。

启用 `audit` 特性后，`confers` 可以记录配置加载历史并自动脱敏敏感字段：

```rust
#[derive(Config, Deserialize)]
struct SecureConfig {
    #[config(sensitive = true)]
    db_password: String,
}

// 敏感字段在日志与 debug 输出中自动脱敏
```

### 👀 文件监听与热重载

> ✨ **提示**：以下功能需要启用 `watch` 特性。

`confers` 支持基于文件监听的热重载：

```rust
use confers::ConfigBuilder;
use confers::watcher::FsWatcher;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 同步构建初始配置
    let config = ConfigBuilder::<MyConfig>::new()
        .file("config.toml")
        .build()?;

    // 设置 FsWatcher 实现热重载（0.3.0+ 推荐）
    let mut watcher = FsWatcher::new("config.toml", 200).await?;

    // 监听文件变更并重建配置
    while let Some(changed_path) = watcher.recv().await {
        println!("Config file changed: {:?}", changed_path);
        let new_config = ConfigBuilder::<MyConfig>::new()
            .file("config.toml")
            .build()?;
        // 将 new_config 应用到你的应用状态
    }

    Ok(())
}
```

> **注意**：旧版 `ConfigBuilder::build_with_watcher()` 方法自 v0.3.0 起已弃用。它只构建初始配置，并会静默丢弃文件变更事件。如需真正的热重载支持，请如上所示直接使用 `FsWatcher` / `MultiFsWatcher`。

### 🔐 敏感数据加密

`confers` 使用 XChaCha20-Poly1305 加密算法保护敏感配置信息：

```rust
use confers::XChaCha20Crypto;

// 创建加密器实例
let crypto = XChaCha20Crypto::new();

// 生成 32 字节密钥（务必妥善保存！）
let key = [0u8; 32]; // 生产环境请使用安全的随机密钥

// 加密敏感数据 - 返回 (nonce, ciphertext)
let (nonce, ciphertext) = crypto.encrypt(b"super_secret_password", &key)?;

// 解密配置
let decrypted = crypto.decrypt(&nonce, &ciphertext, &key)?;
```

**在配置结构体中使用：**

```rust
#[derive(Config, Deserialize)]
struct SecureConfig {
    #[config(encrypt = "xchacha20")]
    db_password: String,
}
```

---

## 🌟 最佳实践

### ✅ 推荐的设计模式

**推荐做法**

- **分层配置**：将配置拆分为多个小结构体（如 `DatabaseConfig`、`ServerConfig`），再组合成 `AppConfig`。
- **环境隔离**：为不同环境使用不同的 `env_prefix`（如 `DEV_`、`PROD_`）。
- **防御式加载**：可选字段始终使用 `Option<T>`，关键字段提供 `default` 默认值。
- **校验与清洗**：始终启用 `validate` 属性，并使用 `with_sanitizer` 清洗输入（如去除字符串首尾空白）。
- **安全**：用 `sensitive = true` 标记敏感字段，防止审计日志泄露。

**应避免的做法**

- **全局静态变量**：避免用全局 `static` 存储配置。建议通过依赖注入或 `Arc` 传递配置。
- **忽略错误**：生产环境应严格检查 `ConfigError`，尤其是 `MemoryLimitExceeded` 与 `ValidationError`。
- **硬编码**：任何可能随环境变化的参数都应通过配置管理，而不是硬编码。
- **明文存储敏感信息**：敏感配置应使用加密特性保护。

### 🔒 安全配置实践

在生产环境中，正确配置安全选项至关重要。本节介绍如何安全地使用 `confers` 的各项安全特性。

#### 1. 敏感数据处理

> ⚠️ **重要**：切勿在配置文件中明文存储敏感信息（如密码、API 密钥、令牌等）。

```rust
use confers::Config;
use serde::Deserialize;

#[derive(Config, Deserialize)]
#[config(env_prefix = "APP")]
struct SecureConfig {
    // 标记敏感字段，审计日志会自动脱敏
    #[config(sensitive = true)]
    database_password: String,

    #[config(sensitive = true)]
    api_key: String,

    // 非敏感字段
    server_name: String,
}
```

**推荐做法：**

- 使用环境变量存储敏感信息
- 使用 `confers encrypt` 命令加密敏感配置
- 将密钥存放在密钥管理系统（如 AWS Secrets Manager、HashiCorp Vault）

#### 2. 配置加密

使用 XChaCha20-Poly1305 加密算法保护敏感配置信息。

```rust
use confers::XChaCha20Crypto;

// 创建加密器实例
let crypto = XChaCha20Crypto::new();

// 生成 32 字节密钥（务必妥善保存！）
let key = [0u8; 32]; // 生产环境请使用安全的随机密钥

// 加密敏感值 - 返回 (nonce, ciphertext)
let (nonce, encrypted_password) = crypto.encrypt(b"my_secret_password", &key)?;

// 解密配置
let decrypted_password = crypto.decrypt(&nonce, &encrypted_password, &key)?;
```

#### 3. 密钥管理

> ⚠️ **重要**：密钥必须安全存放，绝不提交到版本控制系统。

```rust
use confers::key::KeyManager;
use std::path::PathBuf;

// 创建密钥管理器
let mut key_manager = KeyManager::new(PathBuf::from("./secure_keys"))?;

// 初始化密钥环（仅首次需要）
let master_key = [0u8; 32]; // 从安全位置获取
let version = key_manager.initialize(
    &master_key,
    "production".to_string(),
    "security-team".to_string()
)?;

// 定期轮换密钥（建议每 90 天）
let rotation_result = key_manager.rotate_key(
    &master_key,
    Some("production".to_string()),
    "security-team".to_string(),
    Some("Scheduled rotation".to_string())
)?;

println!("Key rotated from version {} to {}",
    rotation_result.previous_version,
    rotation_result.new_version);
```

**密钥管理最佳实践：**

- ✅ 使用硬件安全模块（HSM）或密钥管理服务
- ✅ 定期轮换密钥（建议每 90 天）
- ✅ 为不同环境使用不同密钥
- ✅ 使用强随机数生成器创建密钥
- ❌ 不要在代码中硬编码密钥
- ❌ 不要将密钥提交到版本控制系统
- ❌ 不要在日志中输出密钥

#### 4. 审计日志配置

配置审计日志以追踪所有配置的加载与修改操作。

```rust
use confers::audit::{AuditWriter, AuditConfig};
use std::path::PathBuf;

// 以 builder 模式创建审计写入器
let audit_writer = AuditWriter::builder()
    .log_dir(PathBuf::from("/var/log/confers"))
    .enabled(true)
    .build();

// 记录配置加载事件
audit_writer.log_load("config.toml");

// 记录敏感操作
audit_writer.log_key_access("database_password");
audit_writer.log_decrypt("api_key", true);
```

**审计日志最佳实践：**

- ✅ 将审计日志存储在安全位置（如 `/var/log/confers/`）
- ✅ 配置日志轮转，防止磁盘空间耗尽
- ✅ 限制审计日志文件的访问权限（仅 root/管理员）
- ✅ 监控审计日志以发现可疑活动
- ✅ 实施满足合规要求的日志保留策略

#### 5. 远程配置安全

从远程来源加载配置时，必须确保连接安全。

```rust
use confers::ConfigBuilder;

// 使用 TLS 加密连接（需要 remote 特性）
let config = ConfigBuilder::<MyConfig>::new()
    .file("config.toml")
    .env()
    .build()?;

// 远程配置请使用 remote 特性与 HttpPolledSource
// 完整示例参见 examples/remote_consul.rs
```

**远程配置安全最佳实践：**

- ✅ 始终使用 HTTPS/TLS 加密连接
- ✅ 使用强口令与安全的认证令牌
- ✅ 定期轮换认证凭据
- ✅ 使用证书验证服务器身份
- ✅ 配置超时以避免长时间挂起
- ❌ 不要在 URL 中传递敏感信息
- ❌ 不要使用不安全的 HTTP 连接

#### 6. 配置校验

使用校验器确保配置值处于预期范围内。

```rust
use confers::{Config, Validate};
use serde::Deserialize;
use garde::Validate;

// 使用 garde 派生宏定义校验规则
#[derive(Config, Deserialize, Validate)]
#[config(validate)]  // 配置加载时启用自动校验
struct ValidatedConfig {
    #[garde(range(min = 1, max = 65535))]
    port: u16,

    #[garde(length(min = 1, max = 253))]
    host: String,

    #[garde(email)]
    admin_email: Option<String>,
}

// 配置加载过程中会自动执行校验
// 校验失败时返回 ConfigError::ValidationError
let config = ConfigBuilder::<ValidatedConfig>::new()
    .file("config.toml")
    .validate(true)
    .build()?;
```

**注意**：请在依赖中添加 `garde = { version = "0.22", features = ["derive", "email", "url", "regex"] }`。

**配置校验最佳实践：**

- ✅ 校验所有用户输入
- ✅ 确保数值处于预期范围内
- ✅ 校验字符串格式（如 URL、邮箱）
- ✅ 记录所有校验失败事件
- ✅ 将校验失败视为潜在安全事件
- ❌ 不要为图方便绕过校验

#### 7. 安全校验规则

`security-rules` 特性提供一套标准化的安全校验规则库。内置校验器覆盖 JWT 密钥强度、CORS 配置、SSRF 防护与 TLS 设置 —— 全部在启动时自动执行。

```toml
# Cargo.toml
[dependencies]
confers = { version = "0.6.0-rc.2", features = ["security-rules"] }
```

```rust
use confers::security::rules::SecurityValidatorRegistry;

// 创建包含全部内置校验器的注册表
let registry = SecurityValidatorRegistry::with_defaults();

// 对配置执行全部校验器
let report = registry.validate_all(&config);

// 检查结果
if !report.is_ok(false) {
    for v in &report.violations {
        eprintln!("[{}] {}: {}", v.severity, v.validator, v.message);
    }
    // 出现严重问题时快速失败
    if report.critical_count() > 0 {
        std::process::exit(1);
    }
}
```

**内置校验器：**

| 校验器 | 检查内容 | 严重级别 |
|--------|----------|----------|
| JWT Secret | 长度 ≥ 32 字节、弱口令检测 | Critical |
| CORS | 通配符 `*`、空 methods、max_age > 86400s | Critical/Warning |
| SSRF | 18 个封锁 CIDR 网段、白名单支持 | Critical |
| TLS | min_version ≥ 1.2、弱加密套件 | Critical/Warning |

**自定义校验器**：实现 `SecurityValidator` trait 并通过 `SecurityValidatorRegistry::register()` 注册。

#### 8. 运行时特性开关

`feature-toggle` 特性提供运行时特性开关，与编译期 `cfg(feature = ...)` 标志互补，支持灰度发布与无需重新编译的热切换。

```toml
# Cargo.toml
[dependencies]
confers = { version = "0.6.0-rc.2", features = ["feature-toggle"] }
```

```rust
use confers::toggle::FeatureToggleRegistry;

let registry = FeatureToggleRegistry::new();

// 以默认状态注册特性
registry.register("new_dashboard", "New Dashboard UI", false);
registry.register("beta_api", "Beta API Endpoint", false);

// 从配置加载覆盖值
registry.load_from_config(&config, "features");

// 在应用代码中检查
if registry.is_enabled("new_dashboard") {
    // 使用新面板逻辑
} else {
    // 使用旧逻辑
}
```

配置文件（`config.toml`）：

```toml
[features]
new_dashboard = true
beta_api = false
```

> **提示**：代码裁剪/包含请使用编译期特性（如 `encryption`），行为切换（如 A/B 测试、灰度发布）请使用运行时开关。

#### 9. 生产环境安全检查清单

部署到生产环境前，请逐项检查以下安全条目：

| 安全条目 | 状态 | 说明 |
|----------|------|------|
| 敏感数据加密 | ☐ | 所有敏感信息已加密 |
| 密钥管理 | ☐ | 密钥安全存放并定期轮换 |
| 审计日志 | ☐ | 已启用审计日志并安全存储 |
| 配置校验 | ☐ | 所有配置均经过校验 |
| TLS 加密 | ☐ | 远程连接使用 TLS |
| 访问控制 | ☐ | 配置文件访问权限已收紧 |
| 错误处理 | ☐ | 错误信息不泄露敏感数据 |
| 日志脱敏 | ☐ | 敏感字段已标记为 sensitive |
| 安全校验规则 | ☐ | 启动时安全规则校验通过 |
| 特性开关 | ☐ | 运行时特性开关已审查并配置 |

---

## 🔧 故障排查

| 问题 | 解决方案 |
|------|----------|
| **❓ 环境变量不生效** | 1. 检查 `#[config(env_prefix = "APP")]` 是否设置正确。<br>2. 环境变量名应为 `PREFIX_FIELD_NAME`（全大写）。<br>3. 嵌套结构体使用双下划线，例如 `APP_DB__HOST` 映射到 `db.host`。 |
| **❓ 加载时报 MemoryLimitExceeded 错误** | 1. 检查配置文件是否过大或存在循环引用。<br>2. 调高 `with_memory_limit(mb)` 阈值（默认不限制）。 |
| **❓ 校验失败 ValidationError** | 1. 检查 `validator` 约束逻辑。`confers` 在加载完成后立即执行校验。<br>2. 查看错误输出，其中会指出哪个字段未通过哪条约束。 |
| **❓ 远程配置加载失败 RemoteError** | 1. 检查网络连接与 URL 正确性。<br>2. 若启用了 TLS，确保证书路径正确且有效。<br>3. 检查认证令牌或用户名/口令是否过期。 |

**💬 还需要帮助？** [提交 Issue](https://github.com/Kirky-X/confers/issues) 或访问 [在线 API 文档](https://docs.rs/confers)。

---

## 🎯 延伸阅读

| 文档 | 说明 |
|:-----|:-----|
| [📚 API 参考](API_REFERENCE.md) | 详细的接口文档 |
| [🏗️ 架构文档](ARCHITECTURE.md) | 了解内部机制 |
| [💻 示例代码](../examples/) | 真实场景代码示例 |
| [❓ FAQ](FAQ.md) | 常见问题解答 |
