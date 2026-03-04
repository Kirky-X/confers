# Confers Config 宏完整使用指南

## 概述

`#[derive(Config)]` 是 Confers 库的核心宏，它为 Rust 结构体自动生成完整的配置管理功能。该宏位于 `macros/src/lib.rs`，结合 `codegen.rs` 和 `parse.rs` 实现代码生成。

---

## 一、结构体级别属性

### 1.1 启用验证

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
#[config(validate)]  // 启用配置验证
pub struct AppConfig {
    pub name: String,
    pub port: u16,
}
```

**效果**：
- 自动实现 `validator::Validate` trait
- 调用 `config.validate()` 时会验证所有字段

---

### 1.2 环境变量前缀

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
#[config(env_prefix = "APP_")]  // 读取 APP_NAME, APP_PORT 等
pub struct AppConfig {
    pub name: String,
    pub port: u16,
}
```

**效果**：
- 环境变量读取时添加前缀
- 例如：`APP_NAME=myapp` 会映射到 `name` 字段

---

### 1.3 应用名称

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
#[config(app_name = "myapp")]  // 配置目录名称
pub struct AppConfig {
    pub name: String,
}
```

**效果**：
- 指定搜索配置文件时的目录名称
- 会搜索 `~/.config/myapp/`、`/etc/myapp/` 等路径

---

### 1.4 严格模式

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
#[config(strict = true)]  // CLI 参数解析错误时退出
pub struct AppConfig {
    pub name: String,
}
```

**效果**：
- CLI 参数解析失败时返回错误
- 非严格模式下会忽略错误

---

### 1.5 文件监控（热重载）

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
#[config(watch = true)]  // 启用文件监控
pub struct AppConfig {
    #[config(default = 8080)]
    pub port: u16,
}
```

**效果**：
- 需要启用 `watch` 特性
- 可使用 `load_with_watcher()` 方法获取 watcher

---

### 1.6 格式检测模式

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
#[config(format_detection = "ByContent")]  // 按内容检测
pub struct AppConfig {
    pub name: String,
}
```

**可选值**：
- `ByContent`：根据文件内容检测
- `ByExtension`：根据扩展名检测

---

### 1.7 审计日志

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
#[config(audit_log = true)]  // 启用审计日志
#[config(audit_log_path = "/var/log/config.log")]  // 审计日志路径
pub struct AppConfig {
    pub name: String,
}
```

**效果**：
- 需要启用 `audit` 特性
- 记录配置加载过程到指定文件

---

## 二、字段级别属性

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

**方式二：字符串类型旧语法**
```rust
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
pub struct AppConfig {
    #[config(default = "\"default_value\".to_string()")]
    pub name: String,
}
```

**效果**：
- 当配置文件中没有该字段时使用默认值
- 自动实现 `Default` trait

---

### 2.2 字段描述

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
pub struct AppConfig {
    #[config(description = "服务器端口号")]
    pub port: u16,
    
    #[config(description = "数据库连接URL")]
    pub database_url: String,
}
```

**效果**：
- 生成 CLI 帮助信息
- 用于 JSON Schema 生成

---

### 2.3 配置名称映射

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

### 2.4 环境变量名称映射

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

### 2.5 CLI 参数名称

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

### 2.6 验证规则

**范围验证**
```rust
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
#[config(validate)]
pub struct AppConfig {
    #[config(validate = "range(min = 1, max = 65535)")]
    pub port: u16,
    
    #[config(validate = "range(min = 0, max = 100)")]
    pub rate: i32,
}
```

**长度验证**
```rust
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
#[config(validate)]
pub struct AppConfig {
    #[config(validate = "length(min = 3, max = 50)")]
    pub username: String,
}
```

**内置验证器**
```rust
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
#[config(validate)]
pub struct AppConfig {
    #[config(validate = "email")]
    pub email: String,
    
    #[config(validate = "url")]
    pub website: String,
}
```

**自定义验证器**
```rust
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
#[config(validate)]
pub struct AppConfig {
    #[config(validate = "custom:my_validator")]
    pub field: String,
}

// 需要实现对应的验证函数
```

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
- 审计日志中自动掩码处理
- 不会明文输出敏感信息

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
- 嵌套结构的字段会被提升到顶层
- 支持 `database.host` 和 `database_host` 两种访问方式

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
    pub temp_field: String,  // 不会从配置加载
}
```

**效果**：
- 该字段不会从配置文件加载
- 使用结构体默认值

---

## 三、综合示例

```rust
use confers::Config;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Config)]
#[config(
    validate,                                    // 启用验证
    env_prefix = "APP_",                         // 环境变量前缀
    app_name = "myapp",                         // 应用名称
    strict = false,                              // 非严格模式
    watch = false,                               // 不监控文件变化
    format_detection = "ByExtension",          // 按扩展名检测格式
    audit_log = false,                           // 不启用审计
)]
pub struct AppConfig {
    // ============ 基础类型 ============
    #[config(description = "应用名称")]
    pub name: String,
    
    #[config(default = 8080, description = "服务端口")]
    pub port: u16,
    
    #[config(default = false, description = "调试模式")]
    pub debug: bool,
    
    // ============ 字符串类型 ============
    #[config(default = "\"localhost\".to_string()", description = "服务器主机")]
    pub host: String,
    
    // ============ 验证规则 ============
    #[config(
        validate = "range(min = 1, max = 65535)",
        description = "端口范围"
    )]
    pub admin_port: u16,
    
    #[config(
        validate = "length(min = 3, max = 100)",
        description = "用户名长度"
    )]
    pub username: String,
    
    #[config(validate = "email", description = "邮箱地址")]
    pub email: String,
    
    #[config(validate = "url", description = "网站URL")]
    pub website: String,
    
    // ============ 敏感字段 ============
    #[config(sensitive = true, description = "数据库密码")]
    pub db_password: String,
    
    #[config(sensitive = true, description = "API密钥")]
    pub api_key: String,
    
    // ============ 自定义映射 ============
    #[config(name_env = "CUSTOM_DATABASE_URL", description = "数据库连接")]
    pub database_url: String,
    
    // ============ 嵌套配置 ============
    #[config(flatten, description = "数据库配置")]
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

## 四、自动生成的方法

使用 `#[derive(Config)]` 宏后，结构体会自动获得以下方法：

### 4.1 加载方法

```rust
// 异步加载
let config = AppConfig::load()?;

// 同步加载
let config = AppConfig::load_sync()?;

// 带观察者的加载
let (config, watcher) = AppConfig::load_with_watcher()?;

// 从指定文件加载
let loader = AppConfig::load_file("config.toml");
let config = loader.load()?;

// 使用自定义 CLI 参数
let config = AppConfig::load_from_args(vec!["--name", "test"])?;
```

### 4.2 配置加载器

```rust
let loader = AppConfig::new_loader();
let loader = AppConfig::load_with_strict(true);
```

### 4.3 Schema 生成

```rust
// 生成 JSON Schema
let schema = AppConfig::json_schema();

// 生成 TypeScript 类型
let ts_schema = AppConfig::typescript_schema();

// 导出到文件
AppConfig::export_schema("schema.json")?;
```

### 4.4 其他方法

```rust
// 转换为 Map
let map = config.to_map();

// 获取默认值
let default = AppConfig::default();

// 配置映射
let env_map = AppConfig::env_mapping();
```

---

## 五、完整使用示例

### 5.1 基础使用

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
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ServerConfig::load()?;
    
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
    
    #[config(sensitive = true)]
    pub api_secret: String,
}
```

**加密值格式**：
```
password = "enc:AES256GCM:base64nonce:base64ciphertext"
```

### 5.3 热重载

```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[derive(Debug, Clone, Serialize, Deserialize, Config)]
    #[config(watch = true)]
    pub struct HotReloadConfig {
        #[config(default = 8080)]
        pub port: u16,
    }
    
    let (config, watcher) = HotReloadConfig::load_with_watcher().await?;
    
    if let Some(mut w) = watcher {
        tokio::spawn(async move {
            loop {
                if let Ok(_) = w.rx.recv() {
                    println!("配置文件已变更，重新加载配置...");
                }
            }
        });
    }
    
    // 应用运行...
    
    Ok(())
}
```

---

## 六、属性汇总表

| 属性 | 位置 | 作用 |
|------|------|------|
| `validate` | 结构体 | 启用配置验证 |
| `env_prefix` | 结构体 | 环境变量前缀 |
| `app_name` | 结构体 | 应用名称（配置目录） |
| `strict` | 结构体 | 严格模式 |
| `watch` | 结构体 | 启用文件监控 |
| `format_detection` | 结构体 | 格式检测模式 |
| `audit_log` | 结构体 | 启用审计日志 |
| `audit_log_path` | 结构体 | 审计日志路径 |
| `default` | 字段 | 默认值 |
| `description` | 字段 | 字段描述 |
| `name` | 字段 | 配置键名 |
| `name_env` | 字段 | 环境变量名 |
| `name_clap_long` | 字段 | CLI 长参数名 |
| `name_clap_short` | 字段 | CLI 短参数名 |
| `validate` | 字段 | 验证规则 |
| `custom_validate` | 字段 | 自定义验证 |
| `sensitive` | 字段 | 敏感字段标记 |
| `flatten` | 字段 | 扁平化嵌套配置 |
| `skip` | 字段 | 跳过该字段 |

---

## 七、验证规则详解

### 7.1 range 验证

```rust
#[config(validate = "range(min = 1, max = 65535)")]
pub port: u16,
```

支持的数据类型：
- u8, u16, u32, u64, u128, usize
- i8, i16, i32, i64, i128, isize
- f32, f64

### 7.2 length 验证

```rust
#[config(validate = "length(min = 0, max = 100)")]
pub username: String,
```

支持：
- 字符串长度
- 数组长度

### 7.3 内置验证器

**email 验证**
```rust
#[config(validate = "email")]
pub email: String,
```

**url 验证**
```rust
#[config(validate = "url")]
pub website: String,
```

### 7.4 自定义验证

```rust
#[config(validate = "custom:my_function")]
pub field: String,
```

需要在代码中实现对应的验证函数。

---

## 八、特性依赖

| 属性/方法 | 需要特性 |
|-----------|----------|
| `#[config(validate)]` | `validation` |
| `#[config(watch = true)]` | `watch` |
| `#[config(audit_log = true)]` | `audit` |
| `AppConfig::json_schema()` | `schema` |
| `AppConfig::typescript_schema()` | `schema` |
| CLI 参数支持 | `cli` |
| 加密支持 | `encryption` |
| 远程配置 | `remote` |

---

## 九、最佳实践

### 9.1 推荐配置

```toml
# Cargo.toml
[dependencies]
confers = { version = "0.2", features = ["recommended"] }
```

`recommended` 特性包含：`derive`, `validation`

### 9.2 开发环境配置

```toml
# Cargo.toml
[dependencies]
confers = { version = "0.2", features = ["dev"] }
```

`dev` 特性包含：`derive`, `validation`, `cli`, `schema`, `audit`, `monitoring`, `tracing`

### 9.3 生产环境配置

```toml
# Cargo.toml
[dependencies]
confers = { version = "0.2", features = ["production"] }
```

`production` 特性包含：`derive`, `validation`, `watch`, `encryption`, `remote`, `monitoring`, `tracing`

---

## 十、故障排除

### 10.1 常见问题

**Q: 配置值没有正确加载？**
A: 检查环境变量前缀是否正确，确认配置文件格式是否匹配。

**Q: 验证失败但不知道原因？**
A: 使用 `strict = true` 模式查看详细错误信息。

**Q: 敏感字段在日志中泄露？**
A: 确保使用 `sensitive = true` 属性标记敏感字段。

**Q: 热重载不生效？**
A: 确保启用了 `watch` 特性，并且使用了 `load_with_watcher()` 方法。

---

*本文档基于 Confers v0.2.2 版本编写*
