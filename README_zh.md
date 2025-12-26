# Confers - 现代化的 Rust 配置管理库

<div align="center">
[Show Image](https://crates.io/crates/confers) [Show Image](https://docs.rs/confers) [Show Image](LICENSE) [Show Image](https://github.com/yourusername/confers/actions)
</div>
<div align="center">
**零样板代码 · 类型安全 · 生产就绪**
</div>
<div align="center">
[快速开始](#快速开始) · [文档](https://docs.rs/confers) · [示例](#示例) · [贡献指南](#贡献)
</div>



------

## ✨ 特性

- 🎯 **零样板代码** - 通过 `#[derive(Config)]` 一行搞定配置定义
- 🔄 **智能合并** - 自动按优先级合并多种配置源
- 🛡️ **类型安全** - 编译时类型检查，告别运行时配置错误
- 🔥 **热重载** - 配置文件修改后自动生效，无需重启
- ✅ **配置验证** - 集成 validator，丰富的验证规则
- 📊 **审计日志** - 完整记录配置加载过程，敏感字段自动脱敏
- 🌐 **多格式支持** - TOML / JSON / YAML / INI
- ☁️ **远程配置** - 支持 Etcd / Consul / HTTP 配置中心
- 🔒 **加密支持** - 敏感字段加密存储（v0.4.0+）
- 🛠️ **CLI 工具** - 模板生成、验证、差异对比

------

## 📦 安装

将以下内容添加到 `Cargo.toml`:

```toml
[dependencies]
confers = "0.1"
serde = { version = "1.0", features = ["derive"] }

# 可选特性
confers = { version = "0.1", features = ["watch", "remote", "cli"] }
```

**特性标志**:

- `watch` - 启用配置热重载
- `remote` - 启用远程配置中心支持
- `audit` - 启用审计日志（默认启用）
- `schema` - 启用 Schema 导出
- `cli` - 包含 CLI 工具

------

## 🚀 快速开始

### 基础用法

```rust
use confers::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Config, Serialize, Deserialize, Debug)]
#[config(env_prefix = "MYAPP_")]
struct AppConfig {
    #[cfg_attr(description = "服务器主机地址", default = "\"localhost\".to_string()")]
    host: String,
    
    #[cfg_attr(description = "服务器端口", default = "8080")]
    port: u16,
    
    #[cfg_attr(description = "启用调试模式")]
    debug: Option<bool>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 自动从多种来源加载配置
    let config = AppConfig::load()?;
    
    println!("服务器将在 {}:{} 启动", config.host, config.port);
    
    Ok(())
}
```

### 配置文件 (config.toml)

```toml
# 服务器主机地址
host = "0.0.0.0"

# 服务器端口
port = 8080

# 启用调试模式
debug = true
```

### 环境变量覆盖

```bash
# 环境变量优先级高于配置文件
export MYAPP_PORT=9000
export MYAPP_DEBUG=false

# 运行应用
cargo run
```

### 命令行参数（最高优先级）

```bash
# 命令行参数优先级最高
cargo run -- --port 3000 --host 127.0.0.1
```

---

## 📖 核心概念

### 配置源优先级

Confers 按以下优先级自动合并配置（从低到高）：

```
1. 系统配置文件      /etc/{app_name}/config.*
2. 用户配置文件      ~/.config/{app_name}/config.*
3. 远程配置中心      etcd://... / consul://... / http://...
4. 指定配置文件      --config path/to/config.toml
5. 环境变量          {PREFIX}_KEY=value
6. 命令行参数        --key value (最高优先级)
```

**部分覆盖策略**：高优先级配置源只覆盖显式指定的字段，其他字段从低优先级获取。

### 嵌套配置

```rust
#[derive(Config, Serialize, Deserialize, Debug)]
struct AppConfig {
    server: ServerConfig,
    database: DatabaseConfig,
}

#[derive(Serialize, Deserialize, Debug)]
struct ServerConfig {
    host: String,
    port: u16,
}

#[derive(Serialize, Deserialize, Debug)]
struct DatabaseConfig {
    #[cfg_attr(description = "数据库连接URL")]
    url: String,
    
    #[cfg_attr(description = "连接池大小", default = "10")]
    pool_size: u32,
}
```

**配置文件**:

```toml
[server]
host = "0.0.0.0"
port = 8080

[database]
url = "postgresql://localhost/mydb"
pool_size = 20
```

**环境变量映射**:

```bash
export MYAPP_SERVER_HOST=localhost
export MYAPP_SERVER_PORT=9000
export MYAPP_DATABASE_URL=postgresql://prod/db
export MYAPP_DATABASE_POOL_SIZE=50
```

------

## 🎨 宏属性详解

### 结构体级别属性

```rust
#[derive(Config)]
#[config(
    env_prefix = "MYAPP_",              // 环境变量前缀（默认: 空）
    strict = false,                      // 严格模式（默认: false）
    watch = true,                        // 启用热重载（默认: false）
    format_detection = "ByContent",      // 格式检测方式（默认: ByContent）
    audit_log = true,                    // 启用审计日志（默认: true）
    audit_log_path = "./config.log",     // 审计日志路径（默认: ./config.audit.toml）
    remote = "etcd://localhost:2379/app" // 远程配置地址（可选）
)]
struct AppConfig { }
```

### 字段级别属性

```rust
#[cfg_attr(
    // 基础属性
    description = "字段描述",           // 用于生成文档和模板
    default = "默认值表达式",            // 默认值（Rust 表达式）
    
    // 命名配置
    name_config = "配置文件中的键名",    // 覆盖默认键名
    name_env = "环境变量名",            // 覆盖默认环境变量名
    name_clap_long = "长选项",          // CLI 长选项名
    name_clap_short = 'c',             // CLI 短选项
    
    // 验证规则
    validate = "range(min = 1, max = 65535)", // validator 语法
    custom_validate = "my_validator",         // 自定义验证函数
    
    // 安全配置
    sensitive = true,                   // 敏感字段（审计日志脱敏）
    encrypted = true,                   // 加密存储（v0.4.0+）
    
    // 特殊标记
    flatten,                            // 展平嵌套结构
    skip                                // 跳过此字段
)]
```

------

## 💡 示例

### 1. 基础配置

```rust
use confers::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Config, Serialize, Deserialize)]
#[config(env_prefix = "APP_")]
struct Config {
    #[cfg_attr(default = "\"localhost\".to_string()")]
    host: String,
    
    #[cfg_attr(default = "8080")]
    port: u16,
}

fn main() {
    let config = Config::load().unwrap();
    println!("{:?}", config);
}
```

### 2. 配置验证

```rust
#[derive(Config, Serialize, Deserialize)]
struct Config {
    #[cfg_attr(
        validate = "range(min = 1, max = 65535)",
        error_msg = "端口必须在 1-65535 之间"
    )]
    port: u16,
    
    #[cfg_attr(
        validate = "email",
        error_msg = "无效的邮箱地址"
    )]
    email: String,
    
    #[cfg_attr(
        validate = "url",
        error_msg = "无效的 URL"
    )]
    website: String,
}

fn main() {
    match Config::load() {
        Ok(config) => println!("配置加载成功: {:?}", config),
        Err(e) => eprintln!("配置验证失败: {}", e),
    }
}
```

### 3. 热重载

```rust
use confers::prelude::*;
use tokio;

#[derive(Config, Serialize, Deserialize, Clone)]
#[config(watch = true)]
struct Config {
    port: u16,
    debug: bool,
}

#[tokio::main]
async fn main() {
    // 启动配置监听
    let watcher = Config::watch().unwrap();
    
    // 方式1: Channel 模式（推荐）
    let mut rx = watcher.subscribe();
    tokio::spawn(async move {
        while rx.changed().await.is_ok() {
            let new_config = rx.borrow().clone();
            println!("配置已更新: {:?}", new_config);
            // 在这里重新加载资源、更新状态等
        }
    });
    
    // 方式2: Callback 模式
    watcher.on_change(|config| {
        println!("配置变更: {:?}", config);
    });
    
    // 主应用逻辑
    loop {
        let config = watcher.current();
        println!("当前端口: {}", config.port);
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    }
}
```

### 4. 远程配置

```rust
#[derive(Config, Serialize, Deserialize)]
#[config(
    remote = "etcd://localhost:2379/myapp/config",
    remote_fallback = true  // 远程失败时降级到本地配置
)]
struct Config {
    port: u16,
    database_url: String,
}

#[tokio::main]
async fn main() {
    // 自动从 Etcd 加载配置
    let config = Config::load().await.unwrap();
    println!("{:?}", config);
}
```

支持的远程配置中心：

- **Etcd**: `etcd://host:port/key`
- **Consul**: `consul://host:port/key`
- **HTTP**: `http://api.example.com/config` 或 `https://...`

### 5. 敏感字段处理

```rust
#[derive(Config, Serialize, Deserialize)]
struct Config {
    #[cfg_attr(
        sensitive = true,
        description = "数据库密码"
    )]
    db_password: String,
    
    #[cfg_attr(
        sensitive = true,
        encrypted = true,  // v0.4.0+
        description = "API 密钥"
    )]
    api_key: String,
}

fn main() {
    let config = Config::load().unwrap();
    
    // 导出审计日志（敏感字段自动脱敏）
    config.export_audit_log().unwrap();
    // 审计日志中显示为:
    // db_password = "******"
    // api_key = "******"
}
```

### 6. 自定义验证

```rust
use validator::ValidationError;

fn validate_password_strength(password: &str) -> Result<(), ValidationError> {
    if password.len() < 8 {
        return Err(ValidationError::new("password_too_short"));
    }
    if !password.chars().any(|c| c.is_numeric()) {
        return Err(ValidationError::new("password_needs_number"));
    }
    Ok(())
}

#[derive(Config, Serialize, Deserialize)]
struct Config {
    #[cfg_attr(
        custom_validate = "validate_password_strength",
        error_msg = "密码强度不足"
    )]
    password: String,
}
```

### 7. 生成配置模板

```rust
#[derive(Config, Serialize, Deserialize)]
#[config(env_prefix = "MYAPP_")]
struct Config {
    #[cfg_attr(description = "服务器端口", default = "8080")]
    port: u16,
    
    #[cfg_attr(description = "启用调试模式", default = "false")]
    debug: bool,
}

fn main() {
    // 生成完整模板（包含所有字段和注释）
    let template = Config::generate_template(TemplateLevel::Full);
    println!("{}", template);
    
    // 输出:
    // # 服务器端口
    // port = 8080
    //
    // # 启用调试模式
    // debug = false
}
```

------

## 🛠️ CLI 工具

### 安装

```bash
cargo install confers-cli
```

### 命令

#### 1. 生成配置模板

```bash
# 生成完整模板
confers generate --output config.toml --level full

# 生成最小模板（仅必填字段）
confers generate --output config.toml --level minimal
```

#### 2. 验证配置文件

```bash
confers validate --config config.toml

# 输出:
# ✅ 配置验证通过
# 或
# ❌ 验证失败:
#   - port: 端口必须在 1-65535 之间
#   - email: 无效的邮箱地址
```

#### 3. 配置差异对比

```bash
confers diff production.toml staging.toml

# 输出:
# - port: 8080
# + port: 9000
#   host: "0.0.0.0"
# - debug: true
# + debug: false
```

#### 4. 导出 Schema

```bash
# 生成 JSON Schema
confers schema --format json --output schema.json

# 生成 TypeScript 类型定义
confers schema --format typescript --output config.d.ts
```

#### 5. Shell 自动补全

```bash
# Bash
confers completions bash > /usr/share/bash-completion/completions/myapp

# Zsh
confers completions zsh > ~/.zsh/completion/_myapp

# Fish
confers completions fish > ~/.config/fish/completions/myapp.fish
```

#### 6. 加密配置（v0.4.0+）

```bash
# 生成加密密钥
confers keygen --output ~/.confers/encryption.key

# 加密单个值
confers encrypt --value "my_secret_password"
# 输出: enc:AES256:Zm9vYmFy...

# 批量加密配置文件
confers encrypt-file --input config.plain.toml --output config.encrypted.toml
```

---

## 📚 完整使用指南

### 配置加载流程

```
1. 初始化应用元数据
   ├─ 获取应用名称（从 Cargo.toml 或环境变量）
   ├─ 获取环境变量前缀
   └─ 确定配置文件搜索路径

2. 按优先级加载配置源
   ├─ 系统配置文件 (/etc/{app}/config.*)
   ├─ 用户配置文件 (~/.config/{app}/config.*)
   ├─ 远程配置中心 (etcd/consul/http)
   ├─ 指定配置文件 (--config)
   ├─ 环境变量 ({PREFIX}_*)
   └─ 命令行参数

3. 配置合并与验证
   ├─ 使用 Figment 按优先级合并
   ├─ 部分覆盖策略
   ├─ 类型转换与反序列化
   └─ 执行验证规则

4. 生成审计日志
   ├─ 记录所有配置源状态
   ├─ 输出最终配置（脱敏）
   └─ 记录验证结果

5. 返回配置对象
```

### 错误处理

#### 严格模式 vs 宽松模式

```rust
// 严格模式：任何配置源失败都返回错误
#[derive(Config)]
#[config(strict = true)]
struct Config { }

// 宽松模式（默认）：允许部分配置源失败
#[derive(Config)]
#[config(strict = false)]
struct Config { }
```

**宽松模式行为**：

- ✅ 系统配置文件未找到 → 跳过（常见情况）
- ✅ 用户配置文件未找到 → 跳过（常见情况）
- ❌ 指定配置文件未找到 → **报错**（用户明确指定）
- ⚠️ 环境变量格式错误 → 跳过该变量，记录警告
- ❌ 命令行参数错误 → **报错**（Clap 自动处理）

#### 错误类型

```rust
use confers::ConfigError;

match Config::load() {
    Ok(config) => { /* ... */ }
    Err(ConfigError::FileNotFound { path }) => {
        eprintln!("配置文件未找到: {:?}", path);
    }
    Err(ConfigError::ParseError { source }) => {
        eprintln!("配置解析失败: {}", source);
    }
    Err(ConfigError::ValidationError(errors)) => {
        eprintln!("配置验证失败:");
        for (field, error) in errors.field_errors() {
            eprintln!("  - {}: {}", field, error);
        }
    }
    Err(e) => {
        eprintln!("未知错误: {}", e);
    }
}
```

### 跨平台路径处理

Confers 自动处理 Windows 和 Unix 路径差异：

```rust
// Windows 用户配置文件
C:\Users\foo\config.toml

// 自动转换为 Unix 风格（内部处理）
/c/Users/foo/config.toml

// 路径展开
~/.config/app/config.toml  →  /home/user/.config/app/config.toml
$HOME/config.toml          →  /home/user/config.toml

// 混合分隔符（自动归一化）
C:/Users\foo/config.toml   →  /c/Users/foo/config.toml
```

### 多格式配置文件

#### 格式优先级

当同一目录存在多个格式的配置文件时：

```
config.toml  ← 最高优先级
config.json
config.yaml
config.ini   ← 最低优先级
```

#### 格式检测模式

```rust
#[derive(Config)]
#[config(format_detection = "ByContent")]  // 默认
struct Config { }

#[derive(Config)]
#[config(format_detection = "ByExtension")]  // 仅看扩展名
struct Config { }
```

**ByContent 模式**（推荐）：

- 读取文件内容判断格式
- 防止格式不匹配（如 JSON 内容保存为 .toml）
- 提供清晰的错误提示

**ByExtension 模式**：

- 仅根据文件扩展名判断
- 性能更好（无需读取文件）
- 适合确定格式正确的场景

### 审计日志

#### 审计日志格式

```toml
# Confers 配置审计日志
# 生成时间: 2025-12-12 10:30:45 UTC

[metadata]
loaded_at = "2025-12-12T10:30:45Z"
app_name = "myapp"
version = "1.0.0"
hostname = "prod-server-01"
load_duration_ms = 125

[sources]
system_config = { status = "loaded", path = "/etc/myapp/config.toml" }
user_config = { status = "not_found", path = "~/.config/myapp/config.toml" }
remote_config = { status = "loaded", url = "etcd://localhost:2379/myapp" }
env_vars = { status = "loaded", count = 3 }
cli_args = { status = "loaded", count = 2 }

[warnings]
# 多格式配置文件检测
multiple_formats_detected = [
    "/etc/myapp/config.toml",
    "/etc/myapp/config.json"  # 已忽略
]

[config]
# 合并后的最终配置（敏感字段已脱敏）
host = "0.0.0.0"
port = 8080
debug = false

[config.database]
host = "localhost"
port = 5432
username = "admin"
password = "******"  # 敏感字段已脱敏

[validation]
status = "passed"
errors = []
```

------

## 🔒 安全最佳实践

### 1. 敏感信息保护

```rust
#[derive(Config)]
struct Config {
    // ✅ 正确：标记为敏感字段
    #[cfg_attr(sensitive = true)]
    db_password: String,
    
    #[cfg_attr(sensitive = true)]
    api_key: String,
    
    // ❌ 错误：未标记，可能泄露到日志
    secret_token: String,
}
```

### 2. 路径安全

Confers 自动防护路径遍历攻击：

```rust
// ❌ 恶意路径会被拒绝
../../../etc/passwd
../../.ssh/id_rsa
/etc/shadow

// ✅ 正常路径允许
/etc/myapp/config.toml
~/.config/myapp/config.toml
./config.toml
```

### 3. 环境变量验证

```rust
// Confers 自动验证环境变量：
// - 键名长度 ≤ 256 字节
// - 值长度 ≤ 4KB
// - 键名只允许字母数字和下划线
```

### 4. 配置加密（v0.4.0+）

```rust
#[derive(Config)]
struct Config {
    #[cfg_attr(encrypted = true, sensitive = true)]
    db_password: String,
}
```

**配置文件**:

```toml
# 使用 confers encrypt 命令加密
db_password = "enc:AES256:Zm9vYmFyLi4u"
```

**密钥管理**:

```bash
# 方式1: 环境变量
export CONFERS_ENCRYPTION_KEY="base64_encoded_key"

# 方式2: 密钥文件
echo "base64_encoded_key" > ~/.confers/encryption.key
```

------

## ⚡ 性能优化

### 配置缓存

```rust
use once_cell::sync::OnceCell;

static CONFIG: OnceCell<AppConfig> = OnceCell::new();

fn get_config() -> &'static AppConfig {
    CONFIG.get_or_init(|| {
        AppConfig::load().expect("配置加载失败")
    })
}

fn main() {
    // 首次调用加载配置
    let config = get_config();
    
    // 后续调用直接返回缓存
    let config2 = get_config();  // 零开销
}
```

### 延迟加载

```rust
#[derive(Config)]
struct Config {
    // 基础配置立即加载
    port: u16,
    
    // 复杂配置延迟加载
    #[cfg_attr(skip)]
    database: Option<DatabaseConfig>,
}

impl Config {
    fn database(&mut self) -> &DatabaseConfig {
        self.database.get_or_insert_with(|| {
            DatabaseConfig::load_from_file("database.toml").unwrap()
        })
    }
}
```

---

## 🐛 故障排查

### 常见问题

#### 1. 配置文件未找到

```
错误: 配置文件未找到: /etc/myapp/config.toml
```

**解决方案**:

- 检查文件路径是否正确
- 使用 `--config` 明确指定配置文件
- 启用宽松模式（`strict = false`）跳过缺失的配置文件

#### 2. 环境变量未生效

```
# 环境变量设置了但未生效
export PORT=9000  # ❌ 缺少前缀
export MYAPP_PORT=9000  # ✅ 正确
```

**检查清单**:

- ✅ 环境变量是否包含正确的前缀？
- ✅ 变量名是否全大写？
- ✅ 嵌套字段是否使用下划线分隔？

#### 3. 验证失败

```
错误: 配置验证失败
  - port: 端口必须在 1-65535 之间
```

**解决方案**:

- 检查配置值是否符合验证规则
- 查看 `error_msg` 获取详细提示
- 使用 `confers validate` 命令检查配置

#### 4. 热重载不工作

**检查清单**:

- ✅ 是否启用了 `watch = true`？
- ✅ 是否启用了 `watch` 特性？ `confers = { features = ["watch"] }`
- ✅ 文件路径是否正确？
- ✅ 是否有文件写入权限？

### 调试模式

```bash
# 启用调试日志
RUST_LOG=confers=debug cargo run

# 查看配置加载顺序
confers debug --show-sources

# 导出完整配置（包含来源信息）
confers debug --dump-config
```

------

## 🤝 贡献

欢迎贡献！请查看 [CONTRIBUTING.md](CONTRIBUTING.md) 了解详情。

### 开发环境设置

```bash
# 克隆仓库
git clone https://github.com/yourusername/confers.git
cd confers

# 安装依赖
cargo build

# 运行测试
cargo test --all-features

# 运行示例
cargo run --example basic
```

### 提交规范

```
feat: 新功能
fix: 修复 bug
docs: 文档更新
test: 测试相关
refactor: 重构
perf: 性能优化
```

------

## 📄 许可证

本项目采用 MIT 或 Apache-2.0 双许可证。详见 [LICENSE-MIT](LICENSE-MIT) 和 [LICENSE-APACHE](LICENSE-APACHE)。

------

## 🙏 致谢

Confers 基于以下优秀的开源项目构建：

- [figment](https://github.com/SergioBenitez/Figment) - 配置合并
- [serde](https://github.com/serde-rs/serde) - 序列化框架
- [clap](https://github.com/clap-rs/clap) - 命令行解析
- [validator](https://github.com/Keats/validator) - 数据验证
- [notify](https://github.com/notify-rs/notify) - 文件监听

------

## 📞 联系方式

- **问题反馈**: [GitHub Issues](https://github.com/yourusername/confers/issues)
- **讨论区**: [GitHub Discussions](https://github.com/yourusername/confers/discussions)
- **文档**: [docs.rs/confers](https://docs.rs/confers)

------

<div align="center">
**如果 Confers 对你有帮助，请给个 ⭐️ Star！**
</div>