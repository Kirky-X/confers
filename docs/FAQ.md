<div align="center">

# ❓ 常见问题解答 (FAQ)

### 常见问题的快速解答

[🏠 首页](../README.md) • [📖 用户指南](USER_GUIDE.md) • [🔧 故障排除](TROUBLESHOOTING.md)

---

</div>

## 📋 目录

- [一般问题](#一般问题)
- [安装与配置](#安装与配置)
- [使用与功能](#使用与功能)
- [性能](#性能)
- [安全](#安全)
- [故障排除](#故障排除)
- [贡献](#贡献)
- [许可](#许可)

---

## 一般问题

<div align="center">

### 🤔 关于项目

</div>

<details>
<summary><b>❓ 什么是 Confers？</b></summary>

<br>

**Confers** 是一个现代化、类型安全的 Rust 配置管理库。它提供：

- ✅ **零样板代码** - 只需一个 `#[derive(Config)]` 即可定义配置
- ✅ **类型安全** - 配置结构的编译时类型检查
- ✅ **多源支持** - 自动合并文件、环境变量和远程源

它专为需要稳健、生产级配置管理方式的 **Rust 开发者** 设计。

**了解更多：** [用户指南](USER_GUIDE.md)

</details>

<details>
<summary><b>❓ 为什么应该使用这个而不是其他替代品？</b></summary>

<br>

<table>
<tr>
<th>功能</th>
<th>Confers</th>
<th>Figment</th>
<th>Config-rs</th>
</tr>
<tr>
<td>类型安全</td>
<td>✅ 强</td>
<td>✅ 良好</td>
<td>⚠️ 手动</td>
</tr>
<tr>
<td>热重载</td>
<td>✅ 内置</td>
<td>⚠️ 手动</td>
<td>⚠️ 手动</td>
</tr>
<tr>
<td>验证</td>
<td>✅ 集成</td>
<td>⚠️ 手动</td>
<td>⚠️ 手动</td>
</tr>
<tr>
<td>审计日志</td>
<td>✅ 包含</td>
<td>❌ 否</td>
<td>❌ 否</td>
</tr>
</table>

**主要优势：**

- 🚀 **零样板代码**：用最少的代码加载复杂配置
- 🔄 **智能合并**：自动处理多个来源之间的优先级
- 🛡️ **安全性**：内置敏感字段加密和屏蔽支持
- 📊 **可观测性**：详细的审计日志，记录每个配置值的来源

</details>

<details>
<summary><b>❓ 这个产品已经可以用于生产环境了吗？</b></summary>

<br>

**当前状态：** ✅ **生产就绪！**

<table>
<tr>
<td width="50%">

**已就绪功能：**

- ✅ 核心加载逻辑稳定
- ✅ 支持主要格式（TOML、JSON、YAML）
- ✅ 环境变量覆盖
- ✅ 验证框架
- ✅ 远程源（Etcd、Consul）

</td>
<td width="50%">

**成熟度指标：**

- 📊 广泛的测试套件
- 🔄 定期维护
- 🛡️ 安全导向设计
- 📖 不断增长的文档

</td>
</tr>
</table>

> **注意：** 在升级版本之前，请务必查看 [CHANGELOG](../CHANGELOG.md)。

</details>

<details>
<summary><b>❓ 支持哪些平台？</b></summary>

<br>

<table>
<tr>
<th>平台</th>
<th>架构</th>
<th>状态</th>
<th>备注</th>
</tr>
<tr>
<td rowspan="2"><b>Linux</b></td>
<td>x86_64</td>
<td>✅ 完全支持</td>
<td>主要平台</td>
</tr>
<tr>
<td>ARM64</td>
<td>✅ 完全支持</td>
<td>在 ARM 服务器上测试</td>
</tr>
<tr>
<td rowspan="2"><b>macOS</b></td>
<td>x86_64</td>
<td>✅ 完全支持</td>
<td>Intel Mac</td>
</tr>
<tr>
<td>ARM64</td>
<td>✅ 完全支持</td>
<td>Apple Silicon (M1/M2/M3)</td>
</tr>
<tr>
<td><b>Windows</b></td>
<td>x86_64</td>
<td>✅ 完全支持</td>
<td>Windows 10+</td>
</tr>
</table>

</details>

<details>
<summary><b>❓ 支持哪些编程语言？</b></summary>

<br>

**Confers** 是一个原生 **Rust** 库。虽然目前没有为其他语言提供官方绑定，但其设计专注于为 Rust 生态系统提供最佳体验。

**文档：**

- [Rust API 文档](https://docs.rs/confers)

</details>

---

## 安装与配置

<div align="center">

### 🚀 快速开始
</div>

<details>
<summary><b>❓ 如何安装？</b></summary>

<br>

**对于 Rust 项目：**

在 `Cargo.toml` 中添加以下内容：

```toml
[dependencies]
confers = "0.1"
serde = { version = "1.0", features = ["derive"] }
```

或使用 cargo：

```bash
cargo add confers serde --features serde/derive
```

**可选功能：**

```toml
confers = { version = "0.1", features = ["watch", "remote", "cli"] }
```

**验证：**

```rust
use confers::Config;
use serde::{Deserialize, Serialize};

#[derive(Config, Serialize, Deserialize, Debug)]
struct TestConfig {
    name: String,
}

fn main() {
    let _ = TestConfig::load();
    println!("✅ 安装成功！");
}
```

</details>

<details>
<summary><b>❓ 如何选择合适的特性组合？</b></summary>

<br>

confers 提供了多种特性预设和单独特性，你可以根据项目需求选择：

**特性预设（推荐）**：

| 预设 | 说明 | 适用场景 |
|------|------|----------|
| `minimal` | 仅配置加载（最小依赖） | 仅需基本配置加载功能 |
| `recommended` | 配置加载 + 验证 | 大多数应用（推荐） |
| `dev` | 开发配置（包含 CLI、schema、audit、monitoring） | 开发和调试 |
| `production` | 生产配置（包含 watch、encryption、remote、monitoring） | 生产环境 |
| `full` | 所有功能 | 需要完整功能 |

**使用示例**：

```toml
# 最小化使用
[dependencies]
confers = { version = "0.1", default-features = false, features = ["minimal"] }

# 推荐配置
[dependencies]
confers = { version = "0.1", default-features = false, features = ["recommended"] }

# 生产配置
[dependencies]
confers = { version = "0.1", default-features = false, features = ["production"] }
```

**单独特性**：

- `derive` - 配置结构体的 derive 宏
- `validation` - 配置验证支持
- `cli` - 命令行工具
- `watch` - 文件监控和热重载
- `audit` - 审计日志
- `schema` - JSON Schema 生成
- `parallel` - 并行验证
- `monitoring` - 系统监控
- `remote` - 远程配置（etcd、consul、http）
- `encryption` - 配置加密

> 💡 **提示**: 默认特性为 `derive`（仅配置加载）。如需验证功能，请使用 `recommended` 预设或显式启用 `validation` 特性。

</details>

<details>
<summary><b>❓ 不同特性组合的依赖数量有何差异？</b></summary>

<br>

| 特性组合 | 依赖数量 | 编译时间 | 二进制大小 |
|----------|----------|----------|------------|
| `minimal` | ~15 个 | 最短 | 最小 |
| `recommended` | ~20 个 | 短 | 小 |
| `dev` | ~30 个 | 中 | 中 |
| `production` | ~35 个 | 中 | 中 |
| `full` | ~50+ 个 | 长 | 大 |

选择合适的特性组合可以显著减少编译时间和二进制大小。
```

**另请参阅：** [安装指南](USER_GUIDE.md#安装)

</details>

<details>
<summary><b>❓ 系统要求是什么？</b></summary>

<br>

**最低要求：**

<table>
<tr>
<th>组件</th>
<th>要求</th>
<th>推荐</th>
</tr>
<tr>
<td>Rust 版本</td>
<td>1.75+</td>
<td>最新稳定版</td>
</tr>
<tr>
<td>内存</td>
<td>最小</td>
<td>-</td>
</tr>
<tr>
<td>磁盘空间</td>
<td>最小</td>
<td>-</td>
</tr>
</table>

**可选：**

- 🔧 `watch` 功能需要操作系统级别的文件通知支持（通过 `notify` crate）
- ☁️ `remote` 功能需要访问配置中心（Etcd、Consul）的网络访问

</details>

<details>
<summary><b>❓ 我遇到编译错误，应该怎么办？</b></summary>

<br>

**常见解决方案：**

1. **检查 Rust 版本：**
   ```bash
   rustc --version
   # 应为 1.75.0 或更高版本
   ```

2. **确保启用了 `serde` 派生：**
   确保在 `Cargo.toml` 中为 `serde` 配置了 `features = ["derive"]`。

3. **清理构建产物：**
   ```bash
   cargo clean
   cargo build
   ```

**仍然有问题？**

- 📝 查看 [故障排除指南](TROUBLESHOOTING.md)
- 🐛 [创建 issue](../../issues) 并附上错误详情

</details>

<details>
<summary><b>❓ 可以在 Docker 中使用吗？</b></summary>

<br>

**可以！** Confers 在容器化环境中完美工作。它可以从环境变量加载配置，这是 Docker 的标准方式。

**多阶段 Dockerfile 示例：**

```dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/my_app /usr/local/bin/
CMD ["my_app"]
```

**Docker Compose 中的环境变量：**

```yaml
services:
  app:
    image: my_app:latest
    environment:
      - APP_PORT=8080
      - APP_DATABASE_URL=postgres://user:pass@db/dbname
```

</details>

---

## 使用与功能

<div align="center">

### 💡 使用 API

</div>

<details>
<summary><b>❓ 如何开始基本使用？</b></summary>

<br>

**5 分钟快速入门：**

```rust
use confers::Config;
use serde::{Deserialize, Serialize};

// 1. 定义配置结构
#[derive(Config, Serialize, Deserialize, Debug)]
#[config(env_prefix = "APP_")]
struct AppConfig {
    host: String,
    port: u16,
    debug: bool,
}

fn main() -> anyhow::Result<()> {
    // 2. 从默认来源加载配置
    // (config.toml, .env, 环境变量)
    let config = AppConfig::load()?;
    
    println!("主机: {}, 端口: {}", config.host, config.port);
    Ok(())
}
```

**下一步：**

- 📖 [用户指南](USER_GUIDE.md)
- 💻 [更多示例](../examples/)

</details>

<details>
<summary><b>❓ 支持哪些格式和来源？</b></summary>

<br>

**支持的格式：**

- ✅ TOML
- ✅ JSON
- ✅ YAML
- ✅ INI

**支持的来源：**

- ✅ **文件**：自动检测 `config.{toml,json,yaml,ini}`
- ✅ **环境变量**：支持自定义前缀
- ✅ **CLI 参数**：与 `clap` 集成
- ✅ **远程**：Etcd、Consul、HTTP（通过 `remote` 功能）
- ✅ **默认值**：在结构体定义中指定

</details>

<details>
<summary><b>❓ 可以验证配置吗？</b></summary>

<br>

**可以！** Confers 与 `validator` crate 集成。

```rust
use confers::Config;
use serde::{Deserialize, Serialize};

#[derive(Config, Serialize, Deserialize, Debug)]
struct AppConfig {
    #[config(validate = "length(min = 1)")]
    host: String,
    
    #[config(validate = "range(min = 1024, max = 65535)")]
    port: u16,
}
```

**好处：**

- 🛡️ 在启动时捕获配置错误
- 🎯 精确的错误消息
- ✅ 支持嵌套验证

</details>

<details>
<summary><b>❓ 如何正确处理错误？</b></summary>

<br>

**推荐模式：**

```rust
use confers::ConfigError;

fn main() {
    if let Err(e) = run() {
        match e {
            ConfigError::FileNotFound { path } => {
                eprintln!("未找到配置文件: {:?}", path);
            }
            ConfigError::ValidationError(msg) => {
                eprintln!("验证失败: {}", msg);
            }
            _ => eprintln!("加载配置时出错: {}", e),
        }
    }
}
```

</details>

<details>
<summary><b>❓ 支持异步/await 吗？</b></summary>

<br>

**支持！** Confers 通过 `ConfigLoader` 支持异步加载，这对远程配置源特别有用。

```rust
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = AppConfig::new_loader()
        .with_file("config.toml")
        .load()
        .await?;
    Ok(())
}
```

</details>

---

## 性能

<div align="center">

### ⚡ 速度和优化

</div>

<details>
<summary><b>❓ 有多快？</b></summary>

<br>

Confers 设计高效，在应用程序启动时开销极小。

**基准测试结果（加载 100+ 个键）：**

<table>
<tr>
<th>来源</th>
<th>格式</th>
<th>延迟（平均）</th>
</tr>
<tr>
<td>本地文件</td>
<td>TOML</td>
<td>~0.5 毫秒</td>
</tr>
<tr>
<td>环境变量</td>
<td>-</td>
<td>~0.1 毫秒</td>
</tr>
<tr>
<td>远程（Etcd）</td>
<td>JSON</td>
<td>~5-20 毫秒</td>
</tr>
</table>

**自行运行基准测试：**

```bash
cargo bench
```

</details>

<details>
<summary><b>❓ 如何提高性能？</b></summary>

<br>

**优化技巧：**

1. **启用发布模式：**
   ```bash
   cargo build --release
   ```

2. **使用 `parallel` 特性预分配：**
   如果配置文件非常大，启用 `parallel` 功能以加快验证速度。

3. **使用 `prelude` 以获得宏的最佳编译时间：**
   确保在 `src/lib.rs` 中使用推荐的模式以获得最快的编译时间。

</details>

<details>
<summary><b>❓ 内存使用情况如何？</b></summary>

<br>

**典型内存使用：**

Confers 使用极少的内存，标准应用程序配置通常 **小于 1MB**。它在可能的情况下使用 `serde` 进行零拷贝反序列化。

**内存安全：**

- ✅ 无内存泄漏（通过持续测试验证）
- ✅ 敏感数据使用后可清零
- ✅ 利用 Rust 的所有权模型保证安全

</details>

---

## 安全

<div align="center">

### 🔒 安全功能

</div>

<details>
<summary><b>❓ 这是安全的吗？</b></summary>

<br>

**是的！** 安全是 Confers 的核心关注点。

**安全功能：**

<table>
<tr>
<td width="50%">

**实现**

- ✅ 内存安全（Rust）
- ✅ 敏感字段屏蔽
- ✅ 恒定时间加密
- ✅ 安全路径验证

</td>
<td width="50%">

**保护**

- ✅ 缓冲区溢出保护
- ✅ 抗侧信道攻击
- ✅ 内存擦除（zeroize）
- ✅ 静态加密（v0.4.0+）

</td>
</tr>
</table>

**合规性：**

- 🏅 遵循配置管理行业最佳实践
- 🏅 支持中国标准（通过加密模块支持 SM4-GCM）

**更多详情：** [安全指南](SECURITY.md)

</details>

<details>
<summary><b>❓ 如何报告安全漏洞？</b></summary>

<br>

**请负责任地报告安全问题：**

1. **请勿** 创建公开的 GitHub issue
2. **邮件：** security@confers.io
3. **包括：**
    - 漏洞描述
    - 重现步骤
    - 潜在影响

**响应时间表：**

- 📧 初始响应：24 小时
- 🔍 评估：72 小时
- 📢 公开披露：修复发布后

</details>

<details>
<summary><b>❓ 敏感数据怎么办？</b></summary>

<br>

Confers 提供了几种处理敏感数据的方法：

1. **日志中的屏蔽**：可以标记字段以便在审计日志中屏蔽。
2. **加密**：内置支持配置值的 AES-256-GCM 加密。
3. **环境变量**：推荐在生产环境中用于密钥。

**最佳实践：**

```rust
#[derive(Config, Serialize, Deserialize)]
struct Secrets {
    #[config(sensitive = true)] // 在日志中将值标记为已屏蔽
    api_key: String,
}
```

</details>

---

## 故障排除

<div align="center">

### 🔧 常见问题

</div>

<details>
<summary><b>❓ 我遇到 "FileNotFound" 错误</b></summary>

<br>

**问题：**

```
Error: 配置文件未找到: config.toml
```

**原因：** Confers 在预期位置找不到配置文件。

**解决方案：**

1. 确保文件位于根目录或 `config/` 目录中。
2. 检查文件名（支持：`config.toml`、`config.json`、`config.yaml`、`config.ini`）。
3. 如果使用自定义路径，请确保路径正确。

</details>

<details>
<summary><b>❓ 我遇到 "ValidationError"</b></summary>

<br>

**问题：**

```
Error: 验证失败: ...
```

**原因：** 加载的配置不满足结构体中定义的验证规则。

**解决方案：**

1. 查看错误消息，了解哪个字段失败以及失败原因。
2. 确保配置文件或环境变量符合预期格式和约束。

</details>

<details>
<summary><b>❓ 如何调试配置加载？</b></summary>

<br>

**解决方案：**
启用审计日志以查看每个值的具体来源。

```rust
fn main() {
    tracing_subscriber::fmt::init();
    // Confers 使用 tracing 记录加载过程
    let config = AppConfig::load().unwrap();
}
```

设置 `RUST_LOG=confers=debug` 以查看详细日志。

</details>

**更多问题？** 查看 [故障排除指南](TROUBLESHOOTING.md)

---

## 贡献

<div align="center">

### 🤝 加入社区

</div>

<details>
<summary><b>❓ 如何贡献？</b></summary>

<br>

**贡献方式：**

<table>
<tr>
<td width="50%">

**代码贡献**

- 🐛 修复 bug
- ✨ 添加功能
- 📝 改进文档
- ✅ 编写测试

</td>
<td width="50%">

**非代码贡献**

- 📖 编写教程
- 🎨 设计资源
- 🌍 翻译文档
- 💬 回答问题

</td>
</tr>
</table>

**入门指南：**

1. 🍴 Fork 仓库
2. 🌱 创建分支
3. ✏️ 进行更改
4. ✅ 添加测试
5. 📤 提交 PR

**指南：** [CONTRIBUTING.md](../CONTRIBUTING.md)

</details>

<details>
<summary><b>❓ 发现 bug 应该怎么办？</b></summary>

<br>

**报告前：**

1. ✅ 查看 [现有 issue](../../issues)
2. ✅ 尝试最新版本
3. ✅ 查看 [故障排除指南](TROUBLESHOOTING.md)

**创建良好的 Bug 报告：**

```markdown
### 描述
清晰的 bug 描述

### 重现步骤
1. 第一步
2. 第二步
3. 查看错误

### 预期行为
应该发生什么

### 实际行为
实际发生了什么

### 环境
- 操作系统: Ubuntu 22.04
- Rust 版本: 1.75.0
- 项目版本: 1.0.0

### 其他上下文
任何其他相关信息
```

**提交：** [创建 Issue](../../issues/new)

</details>

<details>
<summary><b>❓ 在哪里可以获得帮助？</b></summary>

<br>

<div align="center">

### 💬 支持渠道

</div>

<table>
<tr>
<td width="33%" align="center">

**🐛 Issues**

[GitHub Issues](../../issues)

Bug 报告和功能请求

</td>
<td width="33%" align="center">

**💬 Discussions**

[GitHub Discussions](../../discussions)

问答和想法

</td>
<td width="33%" align="center">

**💡 Discord**

[加入服务器](https://discord.gg/project)

实时聊天

</td>
</tr>
</table>

**响应时间：**

- 🐛 关键 bug：24 小时
- 🔧 功能请求：1 周
- 💬 问题：2-3 天

</details>

---

## 许可

<div align="center">

### 📄 许可信息

</div>

<details>
<summary><b>❓这是什么许可证？</b></summary>

<br>

**双重许可：**

<table>
<tr>
<td width="50%" align="center">

**MIT 许可证**

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](../LICENSE-MIT)

**权限：**

- ✅ 商业使用
- ✅ 修改
- ✅ 分发
- ✅ 私人使用

</td>
<td width="50%" align="center">

**Apache 许可证 2.0**

[![License: Apache 2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](../LICENSE-APACHE)

**权限：**

- ✅ 商业使用
- ✅ 修改
- ✅ 分发
- ✅ 专利授权

</td>
</tr>
</table>

**您可以选择任一许可证使用。**

</details>

<details>
<summary><b>❓ 可以在商业项目中使用吗？</b></summary>

<br>

**可以！** MIT 和 Apache 2.0 许可证都允许商业使用。

**您需要做的：**

1. ✅ 包含许可证文本
2. ✅ 包含版权声明
3. ✅ 说明任何修改

**您不需要做的：**

- ❌ 分享您的源代码
- ❌ 开源您的项目
- ❌ 支付版税

**问题？** 联系：legal@example.com

</details>

---

<div align="center">

### 🎯 仍然有问题？

<table>
<tr>
<td width="33%" align="center">
<a href="../../issues">
<img src="https://img.icons8.com/fluency/96/000000/bug.png" width="48"><br>
<b>创建 Issue</b>
</a>
</td>
<td width="33%" align="center">
<a href="../../discussions">
<img src="https://img.icons8.com/fluency/96/000000/chat.png" width="48"><br>
<b>开始讨论</b>
</a>
</td>
<td width="33%" align="center">
<a href="mailto:support@example.com">
<img src="https://img.icons8.com/fluency/96/000000/email.png" width="48"><br>
<b>发送邮件</b>
</a>
</td>
</tr>
</table>

---

**[📖 用户指南](USER_GUIDE.md)** • **[🔧 API 文档](https://docs.rs/confers)** • **[🏠 首页](../README.md)**

由文档团队用 ❤️ 制作

[⬆ 返回顶部](#-常见问题解答-faq)
