<div align="center">

# 📘 API 参考文档

### 完整的 API 文档

[🏠 首页](../README.md) • [📖 用户指南](USER_GUIDE.md) • [🏗️ 架构设计](ARCHITECTURE.md)

---

</div>

## 📋 目录

- [概述](#概述)
- [核心 API](#核心-api)
    - [配置加载器](#配置加载器)
    - [密钥管理](#密钥管理)
    - [加密功能](#加密功能)
- [错误处理](#错误处理)
- [类型定义](#类型定义)
- [示例](#示例)

---

## 概述

<div align="center">

### 🎯 API 设计原则

</div>

<table>
<tr>
<td width="25%" align="center">
<img src="https://img.icons8.com/fluency/96/000000/easy.png" width="64"><br>
<b>简洁</b><br>
直观易用
</td>
<td width="25%" align="center">
<img src="https://img.icons8.com/fluency/96/000000/security-checked.png" width="64"><br>
<b>安全</b><br>
默认类型安全
</td>
<td width="25%" align="center">
<img src="https://img.icons8.com/fluency/96/000000/module.png" width="64"><br>
<b>可组合</b><br>
轻松构建复杂工作流
</td>
<td width="25%" align="center">
<img src="https://img.icons8.com/fluency/96/000000/documentation.png" width="64"><br>
<b>完善文档</b><br>
全面的文档支持
</td>
</tr>
</table>

---

## 核心 API

### 配置加载器

`ConfigLoader<T>` 是从多个来源加载和合并配置的核心组件。

#### `ConfigLoader::new()`

创建新的配置加载器实例。

```rust
pub fn new() -> Self
```

#### `with_defaults(defaults: T)`

设置默认配置值。

```rust
pub fn with_defaults(mut self, defaults: T) -> Self
```

#### `with_file(path: impl AsRef<Path>)`

添加显式配置文件。

```rust
pub fn with_file(mut self, path: impl AsRef<Path>) -> Self
```

#### `with_app_name(name: impl Into<String>)`

设置应用程序名称，用于标准配置文件位置（例如 `/etc/<app_name>/config.toml`）。

```rust
pub fn with_app_name(mut self, name: impl Into<String>) -> Self
```

#### `with_env(enabled: bool)`

启用或禁用从环境变量加载。

```rust
pub fn with_env(mut self, enabled: bool) -> Self
```

#### `with_env_prefix(prefix: impl Into<String>)`

设置环境变量前缀（例如 `APP_PORT`）。

```rust
pub fn with_env_prefix(mut self, prefix: impl Into<String>) -> Self
```

#### `with_watch(enabled: bool)`

启用或禁用文件监视以实现自动配置重新加载。

```rust
pub fn with_watch(mut self, watch: bool) -> Self
```

#### `with_audit(enabled: bool)`

启用或禁用配置加载的审计日志记录。

```rust
pub fn with_audit(mut self, enabled: bool) -> Self
```

#### `load()`

异步加载配置。

```rust
pub async fn load(&self) -> Result<T, ConfigError>
```

#### `load_sync_with_audit()`

同步加载配置并支持审计（需要 `audit` 特性）。

```rust
pub fn load_sync_with_audit(&self) -> Result<T, ConfigError>
```

---

### 密钥管理

`KeyManager` 提供加密密钥的综合管理，包括轮换和版本控制。

#### `KeyManager::new(storage_path: PathBuf)`

使用指定存储路径创建新的密钥管理器。

```rust
pub fn new(storage_path: PathBuf) -> Result<Self, ConfigError>
```

#### `initialize(master_key: &[u8; 32], key_id: String, created_by: String)`

使用主密钥初始化新的密钥环。

```rust
pub fn initialize(
    &mut self,
    master_key: &[u8; 32],
    key_id: String,
    created_by: String,
) -> Result<KeyVersion, ConfigError>
```

#### `rotate_key(master_key: &[u8; 32], key_id: Option<String>, created_by: String, description: Option<String>)`

将密钥环轮换到新版本。

```rust
pub fn rotate_key(
    &mut self,
    master_key: &[u8; 32],
    key_id: Option<String>,
    created_by: String,
    description: Option<String>,
) -> Result<RotationResult, ConfigError>
```

#### `get_key_info(key_id: &str)`

获取特定密钥的元数据和版本信息。

```rust
pub fn get_key_info(&self, key_id: &str) -> Result<KeyInfo, ConfigError>
```

---

### 加密功能

`ConfigEncryption` 实现 AES-256-GCM 加密以保护敏感配置值。

#### `ConfigEncryption::new(key_bytes: [u8; 32])`

使用 32 字节密钥创建新的加密器。

```rust
pub fn new(key_bytes: [u8; 32]) -> Self
```

#### `ConfigEncryption::from_env()`

使用 `CONFERS_ENCRYPTION_KEY` 环境变量创建加密器。

```rust
pub fn from_env() -> Result<Self, ConfigError>
```

#### `encrypt(plaintext: &str)`

加密字符串值。返回格式化字符串：`enc:AES256GCM:<nonce>:<ciphertext>`。

```rust
pub fn encrypt(&self, plaintext: &str) -> Result<String, ConfigError>
```

#### `decrypt(encrypted_value: &str)`

解密格式化的加密字符串。

```rust
pub fn decrypt(&self, encrypted_value: &str) -> Result<String, ConfigError>
```

---

## 错误处理

### `ConfigError`

操作过程中遇到的常见错误变体。

| 变体 | 描述 |
|-------------------------|--------------------------------------------------------------|
| `FileNotFound` | 在指定路径未找到配置文件 |
| `FormatDetectionFailed` | 检测文件格式失败（TOML、JSON、YAML）|
| `ParseError` | 解析配置内容时出错 |
| `ValidationError` | 配置未通过验证检查 |
| `KeyNotFound` | 未找到请求的密钥 ID |
| `KeyRotationFailed` | 密钥轮换过程中发生错误 |
| `MemoryLimitExceeded` | 当前内存使用量超过配置的限制 |
| `RemoteError` | 从远程源加载配置时出错（etcd、http）|

---

## 类型定义

### `KeyVersion`

```rust
pub struct KeyVersion {
    pub id: String,
    pub version: u32,
    pub created_at: u64,
    pub status: KeyStatus,
    pub algorithm: String,
}
```

### `KeyInfo`

```rust
pub struct KeyInfo {
    pub key_id: String,
    pub current_version: u32,
    pub total_versions: usize,
    pub active_versions: usize,
    pub deprecated_versions: usize,
    pub created_at: u64,
    pub last_rotated_at: Option<u64>,
}
```

### `RotationResult`

```rust
pub struct RotationResult {
    pub key_id: String,
    pub previous_version: u32,
    pub new_version: u32,
    pub rotated_at: u64,
    pub reencryption_required: bool,
}
```

---

## 示例

### 基本配置加载

```rust
use confers::ConfigLoader;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Default, Clone)]
struct AppConfig {
    database_url: String,
    port: u16,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let loader = ConfigLoader::<AppConfig>::new()
        .with_app_name("myapp")
        .with_file("config.toml")
        .with_env(true)
        .with_env_prefix("MYAPP");

    let config = loader.load().await?;
    println!("数据库: {}", config.database_url);
    Ok(())
}
```

### 密钥轮换

```rust
use confers::key::manager::KeyManager;
use std::path::PathBuf;

fn rotate_my_keys() -> Result<(), Box<dyn std::error::Error>> {
    let mut km = KeyManager::new(PathBuf::from("./keys"))?;
    let master_key = [0u8; 32]; // 在生产环境中，请安全地加载此密钥
    
    let result = km.rotate_key(
        &master_key,
        Some("default".to_string()),
        "admin".to_string(),
        Some("计划轮换".to_string())
    )?;
    
    println!("轮换后的密钥版本: {}", result.new_version);
    Ok(())
}
```

### 多源配置合并

```rust
use confers::ConfigLoader;
use serde::Deserialize;

#[derive(Deserialize)]
struct ServerConfig {
    host: String,
    port: i32,
    workers: usize,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ConfigLoader::<ServerConfig>::new()
        .with_defaults(ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 8080,
            workers: 4,
        })
        .with_file("server.toml")     // 优先级最高
        .with_env(true)               // 允许环境变量覆盖
        .load()
        .await?;

    println!("服务器在 {}:{} 运行", config.host, config.port);
    Ok(())
}
```

### 配置加密

```rust
use confers::encryption::ConfigEncryption;

fn encrypt_sensitive_data() -> Result<(), Box<dyn std::error::Error>> {
    let encryption = ConfigEncryption::from_env()?;
    
    let secret = "my-super-secret-api-key";
    let encrypted = encryption.encrypt(secret)?;
    
    println!("加密后的值: {}", encrypted);
    
    let decrypted = encryption.decrypt(&encrypted)?;
    assert_eq!(decrypted, secret);
    
    Ok(())
}
```

### 配置差异比较

```rust
use confers::commands::{DiffCommand, DiffOptions};

fn compare_configs() -> Result<(), Box<dyn std::error::Error>> {
    let result = DiffCommand::execute(
        "config_development.json",
        "config_production.json",
        DiffOptions::default(),
    )?;

    if result.has_diff() {
        println!("发现配置差异:");
        for diff in result.get_diffs() {
            println!("- {}", diff);
        }
    } else {
        println!("配置完全一致");
    }

    Ok(())
}
```

### 环境变量配置

```rust
use confers::ConfigLoader;
use serde::Deserialize;

#[derive(Deserialize)]
struct AppConfig {
    debug_mode: bool,
    api_endpoint: String,
    timeout: u64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ConfigLoader::<AppConfig>::new()
        .with_file("config.toml")
        .with_env(true)
        .with_env_prefix("APP")
        .load()
        .await?;

    println!("调试模式: {}", config.debug_mode);
    Ok(())
}
```

在 `config.toml` 中：
```toml
debug_mode = false
api_endpoint = "https://api.example.com"
timeout = 30
```

使用环境变量覆盖：
```bash
export APP_DEBUG_MODE=true
export APP_API_ENDPOINT="https://staging.api.example.com"
```

---

## 最佳实践

### 配置验证

始终使用 serde 的验证特性来确保配置的有效性：

```rust
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use chrono::Duration;

#[serde_as]
#[derive(Deserialize, Serialize)]
struct DatabaseConfig {
    #[serde(default = "default_url")]
    url: String,
    
    #[serde(default = "default_pool_size")]
    #[serde(validate(range(min = 1, max = 100)))]
    pool_size: usize,
    
    #[serde_as(as = "serde_with::DurationSeconds<u64>")]
    #[serde(default = "default_timeout")]
    timeout: Duration,
}

fn default_url() -> String {
    "postgres://localhost:5432/app".to_string()
}

fn default_pool_size() -> usize {
    10
}

fn default_timeout() -> Duration {
    Duration::seconds(30)
}
```

### 密钥管理安全

生产环境中务必安全地管理密钥：

```rust
use confers::key::manager::KeyManager;
use std::path::PathBuf;

fn setup_secure_key_management() -> Result<(), Box<dyn std::error::Error>> {
    let master_key = std::env::var("MASTER_KEY")
        .map(|s| {
            let mut key = [0u8; 32];
            let key_bytes = s.as_bytes();
            key.copy_from_slice(&key_bytes[..32.min(key_bytes.len())]);
            key
        })?;

    let mut km = KeyManager::new(PathBuf::from("/etc/confers/keys"))?;
    
    km.initialize(
        &master_key,
        "production".to_string(),
        "security-team".to_string(),
    )?;

    Ok(())
}
```

### 热重载配置

使用文件监视实现配置热重载：

```rust
use confers::ConfigLoader;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = ConfigLoader::<AppConfig>::new()
        .with_file("config.toml")
        .with_watch(true)
        .load()
        .await?;

    println!("初始配置已加载: {:?}", config);

    // 配置文件更改时自动重新加载
    loop {
        tokio::time::sleep(Duration::from_secs(60)).await;
        println!("配置仍在运行，最新版本: {:?}", config);
    }
}
```

---

## 高级功能

### 自定义格式解析器

对于标准库不支持的配置格式，可以实现自定义解析器：

```rust
use confers::{ConfigLoader, FormatParser, ConfigError};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize)]
struct CustomFormat {
    settings: HashMap<String, String>,
}

struct CustomParser;

impl FormatParser for CustomParser {
    fn format_name(&self) -> &str {
        "custom"
    }

    fn parse(&self, content: &str) -> Result<HashMap<String, serde_json::Value>, ConfigError> {
        let config: CustomFormat = toml::from_str(content)
            .map_err(ConfigError::ParseError)?;
        
        let mut map = HashMap::new();
        for (key, value) in config.settings {
            map.insert(key, serde_json::json!(value));
        }
        Ok(map)
    }
}
```

### 配置回滚

使用版本历史实现配置回滚：

```rust
use confers::ConfigLoader;
use std::path::PathBuf;

async fn rollback_to_previous_version() -> Result<(), Box<dyn std::error::Error>> {
    let config_dir = PathBuf::from("/etc/myapp");
    
    let versions = std::fs::read_dir(config_dir.join("history"))?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|p| p.extension().map(|e| e == "toml").unwrap_or(false))
        .collect::<Vec<_>>();

    if versions.len() >= 2 {
        let previous_version = &versions[versions.len() - 2];
        
        let config = ConfigLoader::<AppConfig>::new()
            .with_file(previous_version)
            .load()
            .await?;

        println!("已回滚到之前的配置版本");
        return Ok(());
    }

    Err("没有足够的版本历史用于回滚".into())
}
```

---

## 性能优化

### 异步加载

对于大型配置或远程配置源，始终使用异步加载：

```rust
use confers::ConfigLoader;

async fn load_remote_config() -> Result<(), Box<dyn std::error::Error>> {
    let start = std::time::Instant::now();
    
    let config = ConfigLoader::<AppConfig>::new()
        .with_file("config.toml")
        .with_env(true)
        .load()
        .await?;
    
    let elapsed = start.elapsed();
    println!("配置加载耗时: {:?}", elapsed);
    
    Ok(())
}
```

### 配置缓存

对于频繁访问的配置，使用内存缓存：

```rust
use std::sync::Arc;
use tokio::sync::RwLock;
use std::future::Future;

struct CachedConfig {
    cache: Arc<RwLock<Option<AppConfig>>>,
    loader: ConfigLoader<AppConfig>,
}

impl CachedConfig {
    fn new(loader: ConfigLoader<AppConfig>) -> Self {
        Self {
            cache: Arc::new(RwLock::new(None)),
            loader,
        }
    }

    async fn get(&self) -> Result<AppConfig, Box<dyn std::error::Error>> {
        {
            let cached = self.cache.read().await;
            if let Some(config) = &*cached {
                return Ok(config.clone());
            }
        }

        let config = self.loader.load().await?;
        
        {
            let mut writer = self.cache.write().await;
            *writer = Some(config.clone());
        }

        Ok(config)
    }
}
```

---

## 故障排除

### 常见问题

**Q: 配置文件未找到？**
A: 检查文件路径是否正确，确保使用绝对路径或相对于工作目录的路径。

**Q: 环境变量未生效？**
A: 确认已调用 `with_env(true)`，并检查环境变量名称是否使用正确的前缀。

**Q: 加密解密失败？**
A: 确保使用相同的密钥进行加密和解密，检查 `CONFERS_ENCRYPTION_KEY` 环境变量。

**Q: 配置验证失败？**
A: 查看详细的验证错误信息，确保配置值满足所有验证约束。

### 日志调试

启用详细日志以进行调试：

```rust
use env_logger;

fn setup_logging() {
    env_logger::Builder::from_env(env_logger::Env::default()
        .default_filter_or("confers=debug"))
        .init();
}
```

---

<div align="center">

### 感谢使用 Confers！

如有问题或建议，请访问 [GitHub 仓库](https://github.com/Kirky-X/confers)。

</div>
