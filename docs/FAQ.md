# ❓ Confers FAQ

本页汇总 Confers 的常见问题与解答，按主题分组。没有找到答案？欢迎前往 [GitHub Issues](https://github.com/Kirky-X/confers/issues) 提问。

## 📋 目录

<details open>
<summary>📑 目录（点击展开）</summary>

- [通用问题](#-通用问题)
- [安装与配置](#-安装与配置)
- [使用与特性](#️-使用与特性)
- [性能](#-性能)
- [故障排查](#-故障排查)

</details>

---

## 🧭 通用问题

### ❓ 什么是 Confers？

**Confers** 是一个现代化的类型安全 Rust 配置管理库。它提供：

| ✨ 特性 | 说明 |
|:-------:|:-----|
| **零样板代码** | 只需 `#[derive(Config)]` 即可定义配置 |
| **类型安全** | 在编译期对配置结构进行类型检查 |
| **多来源支持** | 自动合并文件、环境变量与远程来源 |

它面向需要健壮、生产级配置管理方案的 **Rust 开发者**。

**了解更多**：[用户指南](USER_GUIDE.md)

### ❓ 为什么要使用 Confers？

| 特性 | Confers | Figment | Config-rs |
|:-----|:-------:|:-------:|:---------:|
| 类型安全 | ✅ **强** | ✅ 良好 | ⚠️ 手动 |
| 热重载 | ✅ **内置** | ⚠️ 手动 | ⚠️ 手动 |
| 校验 | ✅ **集成** | ⚠️ 手动 | ⚠️ 手动 |
| 审计日志 | ✅ **包含** | ❌ 无 | ❌ 无 |

**核心优势：**

- 🚀 **零样板代码**：用最少的代码加载复杂配置
- 🔄 **智能合并**：自动处理多来源之间的优先级
- 🛡️ **安全**：内置敏感字段加密与脱敏支持
- 📊 **可观测**：详细的审计日志，追踪每个配置值的来源

### ❓ Confers 可以用于生产环境吗？

**当前状态**：✅ **可用于生产！**

**已就绪的特性：**

- ✅ 核心加载逻辑稳定
- ✅ 支持主流格式（TOML、JSON、YAML）
- ✅ 环境变量覆盖
- ✅ 校验框架
- ✅ 远程来源（Etcd、Consul）

**成熟度指标：**

- 📊 覆盖广泛的测试套件
- 🔄 持续维护
- 🛡️ 以安全为中心的设计
- 📖 文档持续完善

> **注意**：升级版本前请务必查阅[更新日志](CHANGELOG.md)。

### ❓ 支持哪些平台？

| 平台 | 架构 | 状态 | 说明 |
|:-----|:-----|:----:|:-----|
| **Linux** | x86_64 | ✅ 完整支持 | 主力平台 |
| | ARM64 | ✅ 完整支持 | 已在 ARM 服务器上测试 |
| **macOS** | x86_64 | ✅ 完整支持 | Intel Mac |
| | ARM64 | ✅ 完整支持 | Apple Silicon（M1/M2/M3） |
| **Windows** | x86_64 | ✅ 完整支持 | Windows 10+ |

### ❓ 如何参与贡献？

**参与方式：**

| 代码贡献 | 非代码贡献 |
|:---------|:-----------|
| 🐛 修复缺陷 | 📖 编写教程 |
| ✨ 添加特性 | 🎨 设计素材 |
| 📝 改进文档 | 🌍 翻译文档 |
| ✅ 编写测试 | 💬 解答问题 |

**上手步骤：**

1. 🍴 Fork 仓库
2. 🌱 创建分支
3. ✏️ 修改代码
4. ✅ 补充测试
5. 📤 提交 PR

**指南**：[贡献指南](CONTRIBUTING.md)

### ❓ 在哪里可以获得帮助？

**支持渠道：**

| 渠道 | 说明 | 响应时间 |
|:-----|:-----|:--------:|
| 🐛 [GitHub Issues](https://github.com/Kirky-X/confers/issues) | 缺陷报告与特性请求 | 关键缺陷：48 小时内确认 |
| 💬 [GitHub Discussions](https://github.com/Kirky-X/confers/discussions) | 问答与想法交流 | 2-3 天 |

### ❓ 项目采用什么许可证？

本项目基于 [MIT 许可证](../LICENSE) 发布。

**您获得的权利：**

- ✅ 商业使用
- ✅ 修改
- ✅ 分发
- ✅ 私有使用

---

## 📦 安装与配置

### ❓ 如何安装？

**Rust 项目：**

在 `Cargo.toml` 中添加：

```toml
[dependencies]
confers = "0.6.0-rc.2"
serde = { version = "1.0", features = ["derive"] }
```

或使用 cargo：

```bash
cargo add confers serde --features serde/derive
```

**可选特性：**

```toml
confers = { version = "0.6.0-rc.2", features = ["watch", "remote", "cli"] }
```

**安装验证：**

```rust
use confers::Config;
use serde::{Deserialize, Serialize};

#[derive(Config, Serialize, Deserialize, Debug)]
struct TestConfig {
    name: String,
}

fn main() {
    let _ = TestConfig::load_sync();
    println!("✅ Installation successful!");
}
```

### ❓ 如何选择合适的特性组合？

**特性预设（推荐）：**

| 预设 | 说明 | 适用场景 |
|:----:|:-----|:---------|
| `minimal` | 环境变量 + JSON | 只需要基础配置加载 |
| `recommended` | TOML + JSON + Env + 校验 | 大多数应用（推荐） |
| `dev` | 开发配置（含 watch、snapshot） | 开发与调试 |
| `production` | 生产配置（含加密） | 生产环境 |
| `distributed` | 分布式系统配置 | 微服务与分布式系统 |
| `full` | 全部特性 | 需要完整功能 |

**用法示例：**

```toml
# 最小化使用
[dependencies]
confers = { version = "0.6.0-rc.2", default-features = false, features = ["minimal"] }

# 推荐配置
[dependencies]
confers = { version = "0.6.0-rc.2", default-features = false, features = ["recommended"] }

# 生产配置
[dependencies]
confers = { version = "0.6.0-rc.2", default-features = false, features = ["production"] }

# 分布式系统配置
[dependencies]
confers = { version = "0.6.0-rc.2", default-features = false, features = ["distributed"] }

# 全量特性配置
[dependencies]
confers = { version = "0.6.0-rc.2", features = ["full"] }
```

> 💡 **提示**：默认特性为 `toml`、`json`、`env`。如需校验功能，请使用 `recommended` 预设或显式启用 `validation` 特性。

### ❓ 不同特性组合的依赖数量差别有多大？

| 特性组合 | 依赖数量 | 编译时间 | 二进制体积 |
|:--------|:--------:|:--------:|:----------:|
| `minimal` | 约 15 | 最短 | 最小 |
| `recommended` | 约 20 | 短 | 小 |
| `dev` | 约 30 | 中 | 中 |
| `production` | 约 35 | 中 | 中 |
| `cli` | 约 25 | 中 | 小 |
| `full` | 50+ | 长 | 大 |

选择合适的特性组合可以显著降低编译时间与二进制体积。

### ❓ 系统要求是什么？

**最低要求：**

| 组件 | 要求 | 推荐配置 |
|:-----|:----:|:--------:|
| Rust 版本 | 1.97.1+ | 最新稳定版 |
| 内存 | 极低 | - |
| 磁盘空间 | 极低 | - |

**可选条件：**

- 🔧 `watch` 特性需要操作系统级文件通知支持
- ☁️ `remote` 特性需要能够访问配置中心的网络

---

## 🛠️ 使用与特性

### ❓ 如何快速上手基本用法？

**5 分钟快速开始：**

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
    let config = AppConfig::load_sync()?;

    println!("Host: {}, Port: {}", config.host, config.port);
    Ok(())
}
```

### ❓ 支持哪些格式和来源？

**支持的格式：**

| ✅ 格式 | 说明 |
|:-------:|:-----|
| TOML | 首选格式 |
| JSON | 通用格式 |
| YAML | 人类可读 |
| INI | 简单格式 |

**支持的来源：**

| ✅ 来源 | 说明 |
|:-------:|:-----|
| 文件 | 自动探测 `config.{toml,json,yaml,ini}` |
| 环境变量 | 支持自定义前缀 |
| CLI 参数 | 与 `clap` 集成 |
| 远程 | Etcd、Consul、HTTP |
| 默认值 | 在结构体定义中指定 |
| 内存 | 通过编程方式设置 |

**支持的远程配置：**

| ✅ 远程来源 | 说明 |
|:----------------:|:------------|
| Etcd | 分布式键值存储 |
| Consul | 服务发现与配置 |
| HTTP | 通过 HTTP(S) 拉取配置 |
| Redis | 配置变更广播（`redis-bus` 特性） |

### ❓ 可以校验配置吗？

**可以！** Confers 与 `garde` 校验库集成。

```rust
use confers::Config;
use garde::Validate;
use serde::{Deserialize, Serialize};

#[derive(Config, Serialize, Deserialize, Validate, Debug)]
#[config(validate)]
struct AppConfig {
    #[garde(length(min = 1))]
    host: String,

    #[garde(range(min = 1024, max = 65535))]
    port: u16,
}
```

**注意**：请在依赖中添加 `garde = { version = "0.22", features = ["derive"] }`。

### ❓ Confers 安全吗？

**安全！** 安全是 Confers 的核心关注点。

**安全特性：**

| 实现 | 防护效果 |
|:-----|:---------|
| ✅ 内存安全（Rust） | ✅ 防缓冲区溢出 |
| ✅ 敏感字段脱敏 | ✅ 抗侧信道攻击 |
| ✅ 常量时间加密 | ✅ 内存清零（zeroize） |
| ✅ 安全路径校验 | ✅ 静态加密（v0.5.0+） |

### ❓ 如何报告安全漏洞？

**请负责任地报告安全问题：**

1. **不要**创建公开的 GitHub Issue
2. **邮件**：Kirky-X@outlook.com
3. **请包含：**
    - 漏洞描述
    - 复现步骤
    - 潜在影响

**响应时间线：**

- 📧 首次响应：48 小时内确认
- 🔍 初步评估：7 天内
- 📢 公开披露：修复发布之后

---

## ⚡ 性能

### ❓ Confers 有多快？

**基准测试结果（加载 100+ 个键）：**

| 来源 | 格式 | 延迟（平均） |
|:-----|:-----|:------------:|
| 本地文件 | TOML | 约 0.5 ms |
| 环境变量 | - | 约 0.1 ms |
| 远程（Etcd） | JSON | 约 5-20 ms |

**自行运行基准测试：**

```bash
cargo bench
```

### ❓ 内存占用如何？

**典型内存占用：**

Confers 的内存占用非常低，标准应用配置通常**小于 1MB**。它尽可能使用 `serde` 进行零拷贝反序列化。

**内存安全：**

- ✅ 无内存泄漏（经持续测试验证）
- ✅ 敏感数据可在使用后清零
- ✅ 充分利用 Rust 的所有权模型保证安全

---

## 🔧 故障排查

### ❓ 出现 "FileNotFound" 错误

**问题：**

```
Error: Configuration file not found: config.toml
```

**解决方案：**

1. 确认文件位于根目录或 `config/` 目录
2. 检查文件名（支持：`config.toml`、`config.json`、`config.yaml`、`config.ini`）
3. 如果使用自定义路径，请确认路径正确

### ❓ 出现 "ValidationError"

**问题：**

```
Error: Validation failed: ...
```

**解决方案：**

1. 查看错误信息，确认是哪个字段未通过以及原因
2. 确保配置文件或环境变量符合预期的格式与约束

---

## 🎯 还有其他问题？

| 提交 Issue | 发起讨论 | 发送邮件 |
|:---------------:|:------------------:|:----------:|
| [🐛 报告问题](https://github.com/Kirky-X/confers/issues) | [💬 社区讨论](https://github.com/Kirky-X/confers/discussions) | [📧 联系支持](mailto:Kirky-X@outlook.com) |

**[📖 用户指南](USER_GUIDE.md)** • **[🔧 在线 API 文档](https://docs.rs/confers)** • **[🏠 返回首页](../README.md)**
