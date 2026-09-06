# 🔌 Confers 库集成指南

本指南介绍如何通过公开的库 API 将 `confers` 嵌入到其他 Rust 项目中。CLI 可执行文件（`confers` 命令）只是同一套库 API 之上的薄封装，因此下文的模式同样描述了 CLI 内部的实现方式。

> **注意**：`confers::cli` 由 `cli` 特性门控，其暴露的类型（`Cli`、`Commands`）是 **crate 内私有的**。不存在公开的 `ConfersCli` 结构体，也没有 `confers::commands` 模块。若要以编程方式驱动 confers，请使用下文记载的库 API。

## 📋 目录

<details open>
<summary>📑 目录（点击展开）</summary>

- [快速开始](#-快速开始)
- [加载配置](#-加载配置)
- [校验](#-校验)
- [加密](#-加密)
- [远程来源](#️-远程来源)
- [特性标志](#-特性标志)
- [错误处理](#-错误处理)
- [从旧版 ConfersCli 快照迁移](#-从旧版-conferscli-快照迁移)
- [故障排查](#-故障排查)

</details>

---

## 🚀 快速开始

### 1. 添加依赖

```toml
[dependencies]
confers = { version = "0.6.0-rc.2", features = ["toml", "json", "env"] }
```

### 2. 基本用法

```rust
use confers::{ConfigBuilder, ConfigConnector, ConfigReader};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct AppConfig {
    pub name: String,
    pub port: u16,
}

fn main() -> confers::BuildResult<()> {
    let config = ConfigBuilder::<AppConfig>::new()
        .file("config.toml")
        .env()
        .build()?;

    // 访问带类型的配置
    println!("name = {}", config.name);
    println!("port = {}", config.port);
    Ok(())
}
```

## 📥 加载配置

`ConfigBuilder` 是从多个来源组装配置的入口。每个来源贡献一个配置层，后加入的来源按合并策略覆盖先加入的来源。

```rust
use confers::ConfigBuilder;

let value = ConfigBuilder::<serde_json::Value>::new()
    .file("base.toml")
    .file("override.toml")
    .env()
    .build_annotated()?; // 返回带有来源信息的 AnnotatedValue
```

### 来源链

如需更精细的优先级控制，请使用 `SourceChainBuilder`：

```rust
use confers::{SourceChainBuilder, FileSource, EnvSource};

let chain = SourceChainBuilder::new()
    .add_source(FileSource::new("base.toml"))
    .add_source(EnvSource::new())
    .build();
```

## ✅ 校验

启用 `validation` 特性，并在配置结构体上派生 `Validate`（来自 `garde`）：

```rust
use confers::Config;
use garde::Validate;

#[derive(Config, Validate)]
#[config(validate)]
struct ServerConfig {
    #[garde(email)]
    pub admin_email: String,
    #[garde(range(min = 1, max = 65535))]
    pub port: u16,
}
```

## 🔐 加密

对于敏感字段，启用 `encryption` 特性并使用 `XChaCha20Crypto`：

```rust
use confers::XChaCha20Crypto;

let crypto = XChaCha20Crypto::new();
let ciphertext = crypto.encrypt(b"secret value", &key)?;
```

## ☁️ 远程来源

`remote` 特性提供基于 HTTP 轮询的远程配置来源：

```rust
use confers::remote::HttpPolledSourceBuilder;

let source = HttpPolledSourceBuilder::new()
    .url("https://config-server.example.com/app-config")
    .interval(std::time::Duration::from_secs(30))
    .build()?;
```

如需 etcd 或 Consul 后端，请启用 `etcd` 或 `consul` 特性，并分别使用 `EtcdSourceBuilder` / `ConsulSourceBuilder`。

## 🎨 特性标志

本库按特性门控。完整的特性预设列表（`default`、`recommended`、`dev`、`production`、`full`）见 `Cargo.toml`。常用特性：

| 特性 | 说明 |
| -------------- | ---------------------------------------- |
| `toml`         | TOML 格式支持                            |
| `json`         | JSON 格式支持                            |
| `yaml`         | YAML 格式支持                            |
| `env`          | 环境变量来源                             |
| `validation`   | 基于 `garde` 的模式校验                  |
| `watch`        | 文件监听与热重载                         |
| `encryption`   | 字段级加密（XChaCha20）                  |
| `audit`        | 审计日志                                 |
| `cli`          | CLI 可执行文件（不暴露公开 API）         |

## 🚨 错误处理

本库区分**配置阶段**错误与**运行时**错误：

- `ConfigConfigError` —— 初始化阶段失败（缺少字段、解析错误、校验失败）。
- `ConfersError` —— 运行时失败（超时、远程不可用、解密失败）。

```rust
use confers::{ConfigConfigError, ConfersError};

match result {
    Err(ConfigConfigError::MissingField { field }) => {
        eprintln!("Missing field: {}", field);
    }
    Err(ConfersError::Timeout { .. }) => {
        eprintln!("Operation timed out");
    }
    _ => {}
}
```

## 🔄 从旧版 ConfersCli 快照迁移

如果您以前依赖过引用 `confers::ConfersCli`、`confers::commands::key::KeySubcommand` 或 `confers::commands::validate::{ValidateCommand, ValidateLevel}` 的代码片段，请迁移到上文的公开 API。这些类型从来不属于公开导出面，现已从文档中移除；CLI 可执行文件内部使用 `clap`，相关类型保持 crate 内私有。

## 🔧 故障排查

### 特性未启用

如果找不到某个符号，请确认对应特性已启用：

```toml
[dependencies]
confers = { version = "0.6.0-rc.2", features = ["validation", "encryption"] }
```

### 加密密钥问题

`derive_field_key` 要求 32 字节主密钥。请使用 `KeyManager`（位于 `key` 特性下）安全管理密钥材料：

```rust
use confers::key::KeyManager;
use std::path::PathBuf;

let mut km = KeyManager::new(PathBuf::from("./secure_keys"))?;
```

### 校验失败

请检查 `ValidationResult`，其中包含未通过的规则列表与出错的字段路径。每个 `ValidationRule` 会报告字段路径、规则名称以及人类可读的错误消息。
