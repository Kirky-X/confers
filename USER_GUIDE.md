# Confers 用户指南

**版本**: v1.0.0
 **最后更新**: 2025-12-12

------

## 📋 目录

1. [简介](#1-简介)
2. [安装与设置](#2-安装与设置)
3. [基础教程](#3-基础教程)
4. [进阶特性](#4-进阶特性)
5. [最佳实践](#5-最佳实践)
6. [配置参考](#6-配置参考)
7. [故障排查](#7-故障排查)
8. [迁移指南](#8-迁移指南)

------

## 1. 简介

### 1.1 什么是 Confers？

Confers 是一个现代化的 Rust 配置管理库，旨在简化应用程序的配置管理。通过过程宏驱动的方式，Confers 让配置定义和加载变得极其简单。

### 1.2 为什么选择 Confers？

| 特性     | Confers          | 传统方案       |
| -------- | ---------------- | -------------- |
| 代码量   | 1 行宏           | 50+ 行样板代码 |
| 类型安全 | ✅ 编译时检查     | ❌ 运行时错误   |
| 多源合并 | ✅ 自动按优先级   | ❌ 手动处理     |
| 热重载   | ✅ 开箱即用       | ❌ 需自己实现   |
| 配置验证 | ✅ 集成 validator | ❌ 手动验证     |
| 审计日志 | ✅ 自动生成       | ❌ 手动记录     |

### 1.3 核心概念

#### 配置源（Config Source）

配置源是配置数据的来源，Confers 支持以下配置源：

```
1. 文件配置      - TOML, JSON, YAML, INI
2. 环境变量      - 系统环境变量
3. 命令行参数    - CLI 参数
4. 远程配置中心  - Etcd, Consul, HTTP
5. 代码默认值    - 结构体字段默认值
```

#### 优先级合并（Priority Merge）

当多个配置源提供相同字段时，Confers 按以下优先级合并：

```
默认值 < 系统文件 < 用户文件 < 远程配置 < 指定文件 < 环境变量 < 命令行
```

**重要**: Confers 使用**部分覆盖**策略，即高优先级配置源只覆盖显式指定的字段。

------

## 2. 安装与设置

### 2.1 添加依赖

在 `Cargo.toml` 中添加：

```toml
[dependencies]
confers = "0.1"
serde = { version = "1.0", features = ["derive"] }

# 可选特性
[dependencies.confers]
version = "0.1"
features = [
    "watch",    # 配置热重载
    "remote",   # 远程配置支持
    "schema",   # Schema 导出
    "audit",    # 审计日志（默认启用）
]
```

### 2.2 特性标志详解

| 特性     | 用途             | 额外依赖                     |
| -------- | ---------------- | ---------------------------- |
| `watch`  | 配置文件变更监听 | notify, tokio                |
| `remote` | 远程配置中心支持 | etcd-client, consul, reqwest |
| `schema` | JSON Schema 生成 | schemars                     |
| `audit`  | 审计日志（默认） | -                            |
| `cli`    | CLI 工具功能     | clap                         |

### 2.3 安装 CLI 工具（可选）

```bash
cargo install confers-cli

# 验证安装
confers --version
```

------

## 3. 基础教程

### 3.1 第一个配置文件

#### 步骤 1: 定义配置结构

```rust
// src/config.rs
use confers::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Config, Serialize, Deserialize, Debug)]
#[config(env_prefix = "MYAPP_")]
pub struct AppConfig {
    #[cfg_attr(
        description = "服务器主机地址",
        default = "\"0.0.0.0\".to_string()"
    )]
    pub host: String,
    
    #[cfg_attr(
        description = "服务器端口",
        default = "8080"
    )]
    pub port: u16,
    
    #[cfg_attr(
        description = "启用调试模式",
        default = "false"
    )]
    pub debug: bool,
}
```

#### 步骤 2: 创建配置文件

创建 `config.toml`:

```toml
# 服务器主机地址
host = "localhost"

# 服务器端口
port = 8080

# 启用调试模式
debug = true
```

#### 步骤 3: 加载配置

```rust
// src/main.rs
mod config;
use config::AppConfig;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 加载配置
    let config = AppConfig::load()?;
    
    println!("服务器配置:");
    println!("  主机: {}", config.host);
    println!("  端口: {}", config.port);
    println!("  调试: {}", config.debug);
    
    Ok(())
}
```

#### 步骤 4: 运行应用

```bash
cargo run

# 输出:
# 服务器配置:
#   主机: localhost
#   端口: 8080
#   调试: true
```

### 3.2 使用环境变量

环境变量优先级高于配置文件：

```bash
# 覆盖端口配置
export MYAPP_PORT=9000

# 覆盖调试模式
export MYAPP_DEBUG=false

cargo run

# 输出:
# 服务器配置:
#   主机: localhost      ← 来自配置文件
#   端口: 9000          ← 来自环境变量
#   调试: false         ← 来自环境变量
```

### 3.3 使用命令行参数

命令行参数优先级最高：

```bash
cargo run -- --port 3000 --host 127.0.0.1

# 输出:
# 服务器配置:
#   主机: 127.0.0.1     ← 来自命令行
#   端口: 3000          ← 来自命令行
#   调试: false         ← 来自环境变量
```

### 3.4 生成配置模板

使用 CLI 工具生成配置模板：

```bash
confers generate --output config.toml --level full

# 生成的 config.toml:
# # 服务器主机地址
# host = "0.0.0.0"
#
# # 服务器端口
# port = 8080
#
# # 启用调试模式
# debug = false
```

------

## 4. 进阶特性

### 4.1 嵌套配置

#### 定义嵌套结构

```rust
#[derive(Config, Serialize, Deserialize, Debug)]
#[config(env_prefix = "MYAPP_")]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub logging: LoggingConfig,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ServerConfig {
    #[serde(default = "default_host")]
    pub host: String,
    
    #[serde(default = "default_port")]
    pub port: u16,
    
    pub workers: Option<usize>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DatabaseConfig {
    pub url: String,
    
    #[serde(default = "default_pool_size")]
    pub pool_size: u32,
    
    pub max_connections: Option<u32>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LoggingConfig {
    #[serde(default = "default_log_level")]
    pub level: String,
    
    pub file: Option<String>,
}

// 默认值函数
fn default_host() -> String { "0.0.0.0".to_string() }
fn default_port() -> u16 { 8080 }
fn default_pool_size() -> u32 { 10 }
fn default_log_level() -> String { "info".to_string() }
```

#### 配置文件（TOML）

```toml
[server]
host = "localhost"
port = 8080
workers = 4

[database]
url = "postgresql://localhost/mydb"
pool_size = 20
max_connections = 100

[logging]
level = "debug"
file = "/var/log/myapp.log"
```

#### 配置文件（JSON）

```json
{
  "server": {
    "host": "localhost",
    "port": 8080,
    "workers": 4
  },
  "database": {
    "url": "postgresql://localhost/mydb",
    "pool_size": 20,
    "max_connections": 100
  },
  "logging": {
    "level": "debug",
    "file": "/var/log/myapp.log"
  }
}
```

#### 环境变量映射

```bash
# 服务器配置
export MYAPP_SERVER_HOST=0.0.0.0
export MYAPP_SERVER_PORT=9000
export MYAPP_SERVER_WORKERS=8

# 数据库配置
export MYAPP_DATABASE_URL=postgresql://prod/db
export MYAPP_DATABASE_POOL_SIZE=50

# 日志配置
export MYAPP_LOGGING_LEVEL=info
export MYAPP_LOGGING_FILE=/var/log/prod.log
```

### 4.2 配置验证

#### 基础验证规则

```rust
use validator::Validate;

#[derive(Config, Serialize, Deserialize, Debug, Validate)]
pub struct AppConfig {
    // 端口范围验证
    #[cfg_attr(
        validate = "range(min = 1, max = 65535)",
        error_msg = "端口必须在 1-65535 之间"
    )]
    pub port: u16,
    
    // 邮箱验证
    #[cfg_attr(
        validate = "email",
        error_msg = "无效的邮箱地址"
    )]
    pub admin_email: String,
    
    // URL 验证
    #[cfg_attr(
        validate = "url",
        error_msg = "无效的 URL"
    )]
    pub webhook_url: String,
    
    // 长度验证
    #[cfg_attr(
        validate = "length(min = 8, max = 32)",
        error_msg = "密码长度必须在 8-32 字符之间"
    )]
    pub password: String,
    
    // 正则验证
    #[cfg_attr(
        validate = "regex(pattern = r'^[a-zA-Z0-9_]+$')",
        error_msg = "用户名只能包含字母、数字和下划线"
    )]
    pub username: String,
}
```

#### 自定义验证函数

```rust
use validator::ValidationError;

fn validate_password_strength(password: &str) -> Result<(), ValidationError> {
    let has_lowercase = password.chars().any(|c| c.is_lowercase());
    let has_uppercase = password.chars().any(|c| c.is_uppercase());
    let has_digit = password.chars().any(|c| c.is_numeric());
    let has_special = password.chars().any(|c| "!@#$%^&*".contains(c));
    
    if !(has_lowercase && has_uppercase && has_digit && has_special) {
        return Err(ValidationError::new("weak_password"));
    }
    
    Ok(())
}

#[derive(Config, Serialize, Deserialize, Debug)]
pub struct AppConfig {
    #[cfg_attr(
        custom_validate = "validate_password_strength",
        error_msg = "密码强度不足，需要包含大小写字母、数字和特殊字符"
    )]
    pub admin_password: String,
}
```

#### 处理验证错误

```rust
fn main() {
    match AppConfig::load() {
        Ok(config) => {
            println!("配置加载成功: {:?}", config);
        }
        Err(confers::ConfigError::ValidationError(errors)) => {
            eprintln!("配置验证失败:");
            for (field, error_list) in errors.field_errors() {
                for error in error_list {
                    eprintln!("  - {}: {}", field, error.message.as_ref().unwrap_or(&"验证失败".into()));
                }
            }
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("配置加载失败: {}", e);
            std::process::exit(1);
        }
    }
}
```

### 4.3 配置热重载

#### Channel 模式（推荐）

适用于多订阅者场景：

```rust
use confers::prelude::*;
use tokio;

#[derive(Config, Serialize, Deserialize, Debug, Clone)]
#[config(watch = true)]
pub struct AppConfig {
    pub port: u16,
    pub workers: usize,
    pub debug: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 启动配置监听
    let watcher = AppConfig::watch()?;
    
    // 订阅者 1: HTTP 服务器
    let mut rx1 = watcher.subscribe();
    tokio::spawn(async move {
        while rx1.changed().await.is_ok() {
            let config = rx1.borrow().clone();
            println!("[HTTP Server] 端口已更新为: {}", config.port);
            // 重新绑定端口...
        }
    });
    
    // 订阅者 2: Worker 池
    let mut rx2 = watcher.subscribe();
    tokio::spawn(async move {
        while rx2.changed().await.is_ok() {
            let config = rx2.borrow().clone();
            println!("[Worker Pool] Worker 数量已更新为: {}", config.workers);
            // 调整 worker 数量...
        }
    });
    
    // 主循环
    loop {
        let config = watcher.current();
        println!("当前配置: {:?}", config);
        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
    }
}
```

#### Callback 模式

适用于单订阅者、简单场景：

```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let watcher = AppConfig::watch()?;
    
    // 注册回调函数
    watcher.on_change(|new_config| {
        println!("配置已更新: {:?}", new_config);
        // 执行重载逻辑...
    });
    
    // 保持应用运行
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }
}
```

#### 防止重载失败影响服务

```rust
use std::sync::Arc;
use tokio::sync::RwLock;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let watcher = AppConfig::watch()?;
    let config = Arc::new(RwLock::new(watcher.current()));
    
    // 配置变更处理
    let config_clone = config.clone();
    let mut rx = watcher.subscribe();
    tokio::spawn(async move {
        while rx.changed().await.is_ok() {
            let new_config = rx.borrow().clone();
            
            // 验证新配置
            if let Err(e) = new_config.validate() {
                eprintln!("新配置验证失败，保留旧配置: {}", e);
                continue;
            }
            
            // 更新配置
            *config_clone.write().await = new_config;
            println!("配置已成功更新");
        }
    });
    
    // 使用配置
    loop {
        let current_config = config.read().await;
        println!("当前端口: {}", current_config.port);
        drop(current_config);
        
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    }
}
```

### 4.4 远程配置中心

#### Etcd 配置

```rust
#[derive(Config, Serialize, Deserialize, Debug)]
#[config(
    remote = "etcd://localhost:2379/myapp/config",
    remote_timeout = "5s",
    remote_fallback = true  // 连接失败时降级到本地配置
)]
pub struct AppConfig {
    pub port: u16,
    pub database_url: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 自动从 Etcd 加载配置
    let config = AppConfig::load().await?;
    println!("{:?}", config);
    Ok(())
}
```

#### 向 Etcd 写入配置

```bash
# 安装 etcdctl
brew install etcd  # macOS
# 或
apt install etcd-client  # Ubuntu

# 写入配置
etcdctl put /myapp/config '{"port": 8080, "database_url": "postgresql://localhost/db"}'

# 查看配置
etcdctl get /myapp/config
```

#### Consul 配置

```rust
#[derive(Config, Serialize, Deserialize, Debug)]
#[config(
    remote = "consul://localhost:8500/myapp/config",
    remote_fallback = true
)]
pub struct AppConfig {
    pub port: u16,
    pub api_key: String,
}
```

#### HTTP 配置源

```rust
#[derive(Config, Serialize, Deserialize, Debug)]
#[config(
    remote = "https://api.example.com/config/myapp",
    remote_auth = "bearer:your_token_here"
)]
pub struct AppConfig {
    pub port: u16,
    pub features: Vec<String>,
}
```

#### 远程配置监听（自动更新）

```rust
#[derive(Config, Serialize, Deserialize, Debug, Clone)]
#[config(
    remote = "etcd://localhost:2379/myapp/config",
    watch = true  // 同时启用热重载
)]
pub struct AppConfig {
    pub port: u16,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let watcher = AppConfig::watch().await?;
    
    let mut rx = watcher.subscribe();
    tokio::spawn(async move {
        while rx.changed().await.is_ok() {
            let new_config = rx.borrow().clone();
            println!("远程配置已更新: {:?}", new_config);
        }
    });
    
    // 主逻辑...
    Ok(())
}
```

### 4.5 敏感信息处理

#### 标记敏感字段

```rust
#[derive(Config, Serialize, Deserialize, Debug)]
pub struct AppConfig {
    pub host: String,
    pub port: u16,
    
    #[cfg_attr(
        sensitive = true,
        description = "数据库密码"
    )]
    pub db_password: String,
    
    #[cfg_attr(
        sensitive = true,
        description = "API 密钥"
    )]
    pub api_key: String,
    
    #[cfg_attr(
        sensitive = true,
        description = "JWT 签名密钥"
    )]
    pub jwt_secret: String,
}
```

#### 审计日志自动脱敏

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = AppConfig::load()?;
    
    // 导出审计日志
    config.export_audit_log()?;
    
    // 查看审计日志 (config.audit.toml)
    // db_password = "******"  ← 自动脱敏
    // api_key = "******"      ← 自动脱敏
    // jwt_secret = "******"   ← 自动脱敏
    
    Ok(())
}
```

#### 配置加密（v0.4.0+）

**生成加密密钥**:

```bash
confers keygen --output ~/.confers/encryption.key
```

**加密配置值**:

```bash
confers encrypt --value "my_secret_password"
# 输出: enc:AES256:Zm9vYmFyLi4u...
```

**使用加密字段**:

```rust
#[derive(Config, Serialize, Deserialize, Debug)]
pub struct AppConfig {
    #[cfg_attr(
        encrypted = true,
        sensitive = true,
        description = "数据库密码"
    )]
    pub db_password: String,
}
```

**配置文件**:

```toml
# 使用加密后的值
db_password = "enc:AES256:Zm9vYmFyLi4u..."
```

**设置解密密钥**:

```bash
# 方式1: 环境变量
export CONFERS_ENCRYPTION_KEY="your_base64_key"

# 方式2: 密钥文件（自动读取）
# ~/.confers/encryption.key
```

### 4.6 配置 Schema 导出

#### 生成 JSON Schema

```rust
use confers::prelude::*;

#[derive(Config, Serialize, Deserialize)]
pub struct AppConfig {
    #[cfg_attr(description = "服务器端口")]
    pub port: u16,
    
    #[cfg_attr(description = "数据库配置")]
    pub database: DatabaseConfig,
}

fn main() {
    // 生成 JSON Schema
    let schema = AppConfig::json_schema();
    println!("{}", serde_json::to_string_pretty(&schema).unwrap());
    
    // 导出到文件
    AppConfig::export_schema("schema.json").unwrap();
}
```

#### 使用 CLI 生成 Schema

```bash
# JSON Schema
confers schema --format json --output schema.json

# TypeScript 类型定义
confers schema --format typescript --output config.d.ts
```

**生成的 TypeScript 类型**:

```typescript
export interface AppConfig {
  /** 服务器端口 */
  port: number;
  
  /** 数据库配置 */
  database: DatabaseConfig;
}

export interface DatabaseConfig {
  url: string;
  pool_size: number;
}
```

---

## 5. 最佳实践

### 5.1 项目结构

推荐的项目结构：

```
myapp/
├── src/
│   ├── main.rs
│   ├── config/
│   │   ├── mod.rs           # 配置模块入口
│   │   ├── app.rs           # 应用配置
│   │   ├── database.rs      # 数据库配置
│   │   └── server.rs        # 服务器配置
│   └── ...
├── config/
│   ├── default.toml         # 默认配置
│   ├── development.toml     # 开发环境
│   ├── production.toml      # 生产环境
│   └── test.toml            # 测试环境
├── Cargo.toml
└── README.md
```

### 5.2 配置模块化

**src/config/mod.rs**:

```rust
mod app;
mod database;
mod server;

pub use app::AppConfig;
pub use database::DatabaseConfig;
pub use server::ServerConfig;

use confers::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Config, Serialize, Deserialize, Debug)]
#[config(env_prefix = "MYAPP_")]
pub struct Config {
    pub app: AppConfig,
    pub database: DatabaseConfig,
    pub server: ServerConfig,
}

impl Config {
    /// 根据环境加载配置
    pub fn load_for_env(env: &str) -> Result<Self, confers::ConfigError> {
        std::env::set_var("CONFIG_FILE", format!("config/{}.toml", env));
        Self::load()
    }
}
```

**src/config/database.rs**:

```rust
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Serialize, Deserialize, Debug, Validate)]
pub struct DatabaseConfig {
    #[validate(url)]
    pub url: String,
    
    #[validate(range(min = 1, max = 1000))]
    pub pool_size: u32,
    
    pub max_connections: Option<u32>,
    
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
}

fn default_timeout() -> u64 { 30 }
```

### 5.3 环境特定配置

```rust
// src/main.rs
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 从环境变量读取运行环境
    let env_name = env::var("APP_ENV").unwrap_or_else(|_| "development".to_string());
    
    // 加载对应环境的配置
    let config = Config::load_for_env(&env_name)?;
    
    println!("当前环境: {}", env_name);
    println!("配置: {:?}", config);
    
    Ok(())
}
```

```bash
# 开发环境
APP_ENV=development cargo run

# 生产环境
APP_ENV=production cargo run

# 测试环境
APP_ENV=test cargo test
```

### 5.4 配置单例模式

使用 `once_cell` 实现全局配置单例：

```rust
use once_cell::sync::Lazy;
use std::sync::RwLock;

pub static CONFIG: Lazy<RwLock<AppConfig>> = Lazy::new(|| {
    let config = AppConfig::load().expect("配置加载失败");
    RwLock::new(config)
});

// 读取配置
pub fn get_config() -> impl std::ops::Deref<Target = AppConfig> {
    CONFIG.read().unwrap()
}

// 更新配置（热重载时使用）
pub fn update_config(new_config: AppConfig) {
    *CONFIG.write().unwrap() = new_config;
}

// 使用示例
fn main() {
    let config = get_config();
    println!("端口: {}", config.port);
}
```

### 5.5 配置测试

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    
    #[test]
    fn test_load_from_file() {
        let config = AppConfig::load_from_file("config/test.toml").unwrap();
        assert_eq!(config.port, 8080);
    }
    
    #[test]
    fn test_load_from_env() {
        env::set_var("MYAPP_PORT", "9000");
        let config = AppConfig::load().unwrap();
        assert_eq!(config.port, 9000);
        env::remove_var("MYAPP_PORT");
    }
    
    #[test]
    fn test_validation_failure() {
        env::set_var("MYAPP_PORT", "99999");  // 超出范围
        let result = AppConfig::load();
        assert!(result.is_err());
        env::remove_var("MYAPP_PORT");
    }
    
    #[test]
    fn test_default_values() {
        let config = AppConfig::new();  // 使用默认值
        assert_eq!(config.host, "0.0.0.0");
        assert_eq!(config.port, 8080);
    }
}
```

### 5.6 安全建议

#### ✅ 推荐做法

```rust
// 1. 敏感字段标记
#[cfg_attr(sensitive = true)]
pub api_key: String,

// 2. 使用环境变量传递敏感信息
// export MYAPP_API_KEY=secret_key

// 3. 不要将敏感配置提交到代码仓库
// .gitignore:
// config/production.toml
// config/secrets.toml

// 4. 使用配置加密
#[cfg_attr(encrypted = true, sensitive = true)]
pub password: String,

// 5. 限制配置文件权限
// chmod 600 config/production.toml
```

#### ❌ 不推荐做法

```rust
// 1. 硬编码敏感信息
pub const API_KEY: &str = "my_secret_key";  // ❌

// 2. 明文存储密码
password = "plaintext_password"  // ❌ 使用加密

// 3. 敏感配置未标记
pub api_key: String,  // ❌ 缺少 sensitive = true

// 4. 将生产配置提交到 Git
git add config/production.toml  // ❌
```

------

## 6. 配置参考

### 6.1 宏属性完整列表

#### 结构体级别 `#[config(...)]`

| 属性               | 类型   | 默认值                  | 说明                                        |
| ------------------ | ------ | ----------------------- | ------------------------------------------- |
| `env_prefix`       | String | `""`                    | 环境变量前缀                                |
| `strict`           | bool   | `false`                 | 严格模式（配置源失败时是否报错）            |
| `watch`            | bool   | `false`                 | 启用配置热重载                              |
| `format_detection` | String | `"ByContent"`           | 格式检测方式（`ByContent` / `ByExtension`） |
| `audit_log`        | bool   | `true`                  | 启用审计日志                                |
| `audit_log_path`   | String | -                       | 审计日志输出路径                            |
| `remote`           | String | -                       | 远程配置地址                                |
| `remote_timeout`   | String | `"5s"`                  | 远程连接超时时间                            |
| `remote_fallback`  | bool   | `false`                 | 远程失败时是否降级到本地配置                |

#### 字段级别 `#[cfg_attr(...)]`

| 属性              | 类型   | 说明                           |
| ----------------- | ------ | ------------------------------ |
| `description`     | String | 字段描述（用于生成文档和模板） |
| `default`         | Expr   | 默认值表达式                   |
| `name_config`     | String | 配置文件中的键名（覆盖默认）   |
| `name_env`        | String | 环境变量名（覆盖默认）         |
| `name_clap_long`  | String | CLI 长选项名                   |
| `name_clap_short` | char   | CLI 短选项                     |
| `validate`        | String | 验证规则（validator 语法）     |
| `custom_validate` | String | 自定义验证函数名               |
| `error_msg`       | String | 验证失败时的错误提示           |
| `sensitive`       | bool   | 敏感字段标记（审计日志脱敏）   |
| `encrypted`       | bool   | 加密存储（v0.4.0+）            |
| `flatten`         | Flag   | 展平嵌套结构                   |
| `skip`            | Flag   | 跳过此字段                     |

### 6.2 验证规则参考

#### 数值范围

```rust
#[cfg_attr(validate = "range(min = 0, max = 100)")]
pub percentage: u8,

#[cfg_attr(validate = "range(min = 1)")]
pub positive_number: i32,
```

#### 字符串长度

```rust
#[cfg_attr(validate = "length(min = 1, max = 100)")]
pub username: String,

#[cfg_attr(validate = "length(equal = 10)")]
pub phone: String,
```

#### 格式验证

```rust
#[cfg_attr(validate = "email")]
pub email: String,

#[cfg_attr(validate = "url")]
pub website: String,

#[cfg_attr(validate = "ip")]
pub server_ip: String,
```

#### 正则表达式

```rust
#[cfg_attr(validate = "regex(pattern = r'^[a-zA-Z0-9_]+$')")]
pub identifier: String,

#[cfg_attr(validate = "regex(pattern = r'^\d{3}-\d{4}$')")]
pub phone_number: String,
```

#### 自定义验证

```rust
#[cfg_attr(custom_validate = "validate_custom_rule")]
pub custom_field: String,
```

### 6.3 环境变量命名规则

| 配置结构                    | 环境变量名                  |
| --------------------------- | --------------------------- |
| `config.port`               | `PREFIX_PORT`               |
| `config.server.host`        | `PREFIX_SERVER_HOST`        |
| `config.database.pool_size` | `PREFIX_DATABASE_POOL_SIZE` |
| `config.logging.level`      | `PREFIX_LOGGING_LEVEL`      |

**规则**:

- 使用前缀（`env_prefix`）
- 嵌套字段用单下划线分隔
- 全部大写
- 字段名中的下划线保留

---

## 7. 故障排查

### 7.1 常见错误

#### 错误 1: 配置文件未找到

```
Error: 配置文件未找到: /etc/myapp/config.toml
```

**原因**:

- 配置文件路径不正确
- 配置文件不存在
- 没有读取权限

**解决方案**:

```bash
# 1. 检查文件是否存在
ls -la /etc/myapp/config.toml

# 2. 检查文件权限
chmod 644 /etc/myapp/config.toml

# 3. 使用 --config 明确指定路径
cargo run -- --config ./config.toml

# 4. 启用宽松模式（跳过缺失的配置文件）
#[config(strict = false)]
```

#### 错误 2: 环境变量未生效

```bash
export PORT=9000  # ❌ 不生效
```

**原因**: 缺少环境变量前缀

**解决方案**:

```bash
# 检查配置的前缀
#[config(env_prefix = "MYAPP_")]

# 正确的环境变量名
export MYAPP_PORT=9000  # ✅ 生效
```

#### 错误 3: 类型转换失败

```
Error: invalid type: string "abc", expected u16
```

**原因**: 配置值类型与字段类型不匹配

**解决方案**:

```toml
# ❌ 错误
port = "8080"  # 字符串

# ✅ 正确
port = 8080    # 数字
```

#### 错误 4: 验证失败

```
Error: 配置验证失败
  - port: 端口必须在 1-65535 之间
```

**解决方案**:

```bash
# 使用 CLI 工具验证配置
confers validate --config config.toml

# 检查配置值是否符合验证规则
port = 70000  # ❌ 超出范围
port = 8080   # ✅ 正确
```

#### 错误 5: 热重载不工作

**检查清单**:

```rust
// 1. 是否启用 watch 特性？
confers = { version = "0.1", features = ["watch"] }

// 2. 是否设置 watch = true？
#[config(watch = true)]

// 3. 是否使用异步运行时？
#[tokio::main]
async fn main() { }

// 4. 文件路径是否正确？
// 使用绝对路径或检查工作目录
```

### 7.2 调试技巧

#### 启用调试日志

```bash
# 设置日志级别
export RUST_LOG=confers=debug

cargo run
```

#### 查看配置加载顺序

```bash
confers debug --show-sources

# 输出:
# 配置源加载顺序:
# 1. 系统配置: /etc/myapp/config.toml (已加载)
# 2. 用户配置: ~/.config/myapp/config.toml (未找到)
# 3. 环境变量: 3 个变量已加载
# 4. 命令行参数: 2 个参数已解析
```

#### 导出最终配置

```bash
confers debug --dump-config

# 输出完整的合并后配置
```

#### 验证配置来源

```rust
// 查看审计日志
cat config.audit.toml

# [sources]
# system_config = { status = "loaded", path = "/etc/myapp/config.toml" }
# env_vars = { status = "loaded", count = 3 }
# cli_args = { status = "loaded", count = 2 }
```

### 7.3 性能问题

#### 问题: 配置加载缓慢

**原因**:

- 远程配置网络延迟
- 大量环境变量解析
- 复杂的验证规则

**解决方案**:

```rust
// 1. 设置远程连接超时
#[config(remote_timeout = "2s")]

// 2. 使用配置缓存
static CONFIG: Lazy<AppConfig> = Lazy::new(|| {
    AppConfig::load().unwrap()
});

// 3. 减少不必要的验证
// 只在必要字段上使用验证规则
```

#### 问题: 热重载占用资源

**解决方案**:

```rust
// 1. 调整防抖动时间
// 默认 500ms，可以增加到 1000ms

// 2. 限制监听的文件数量
// 只监听实际使用的配置文件

// 3. 使用条件编译
#[cfg(not(feature = "watch"))]
let config = AppConfig::load()?;

#[cfg(feature = "watch")]
let config = AppConfig::watch()?;
```

------

## 8. 迁移指南

### 8.1 从 config-rs 迁移

**之前 (config-rs)**:

```rust
use config::{Config, File};

let settings = Config::builder()
    .add_source(File::with_name("config"))
    .add_source(config::Environment::with_prefix("APP"))
    .build()?;

let port: u16 = settings.get("port")?;
let host: String = settings.get("host")?;
```

**之后 (Confers)**:

```rust
use confers::prelude::*;

#[derive(Config, Serialize, Deserialize)]
#[config(env_prefix = "APP_")]
struct Settings {
    port: u16,
    host: String,
}

let settings = Settings::load()?;
// 直接访问字段，类型安全
println!("{}", settings.port);
```

### 8.2 从 figment 迁移

**之前 (figment)**:

```rust
use figment::{Figment, providers::{Toml, Env}};

#[derive(Deserialize)]
struct Config {
    port: u16,
}

let config: Config = Figment::new()
    .merge(Toml::file("config.toml"))
    .merge(Env::prefixed("APP_"))
    .extract()?;
```

**之后 (Confers)**:

```rust
use confers::prelude::*;

#[derive(Config, Serialize, Deserialize)]
#[config(env_prefix = "APP_")]
struct Config {
    port: u16,
}

let config = Config::load()?;
```

### 8.3 从环境变量迁移

**之前 (dotenv + env::var)**:

```rust
use dotenv::dotenv;
use std::env;

dotenv().ok();
let port: u16 = env::var("PORT")
    .unwrap_or("8080".to_string())
    .parse()
    .expect("PORT must be a number");
```

**之后 (Confers)**:

```rust
use confers::prelude::*;

#[derive(Config, Serialize, Deserialize)]
struct Config {
    #[cfg_attr(default = "8080")]
    port: u16,
}

let config = Config::load()?;
```

------

## 附录

### A. 完整示例项目

参见 [examples/](https://github.com/yourusername/confers/tree/main/examples) 目录：

- `basic.rs` - 基础配置加载
- `nested.rs` - 嵌套配置结构
- `validation.rs` - 配置验证
- `hot_reload.rs` - 热重载示例
- `remote_config.rs` - 远程配置中心
- `web_server.rs` - 完整 Web 服务器示例

### B. API 文档

完整 API 文档: https://docs.rs/confers

### C. 社区资源

- **GitHub 仓库**: https://github.com/yourusername/confers
- **问题反馈**: https://github.com/yourusername/confers/issues
- **讨论区**: https://github.com/yourusername/confers/discussions
- **Crates.io**: https://crates.io/crates/confers