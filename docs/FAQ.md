<span id="top"></span>
<div align="center">

<img src="../resource/confers.png" alt="Confers Logo" width="150" style="margin-bottom: 16px">

# ❓ 常见问题解答 (FAQ)


[🏠 首页](../README.md) • [📖 用户指南](USER_GUIDE.md) • [🔧 API 参考](API_REFERENCE.md)

---

</div>

## 📋 目录

<details open style="padding:16px">
<summary style="cursor:pointer; font-weight:600; color:#1E293B">📑 目录（点击展开）</summary>

- [一般问题](#一般问题)
- [安装与配置](#安装与配置)
- [使用与功能](#使用与功能)
- [性能](#性能)
- [安全](#安全)
- [故障排除](#故障排除)
- [贡献](#贡献)
- [许可](#许可)

</details>

---

## 一般问题

<div align="center" style="margin: 24px 0">

### 🤔 关于项目

</div>

<details style="padding:16px; margin: 8px 0">
<summary style="cursor:pointer; font-weight:600; color:#1E293B">❓ 什么是 Confers？</summary>

**Confers** 是一个现代化、类型安全的 Rust 配置管理库。它提供：

| ✨ 特性 | 描述 |
|:------:|------|
| **零样板代码** | 只需一个 `#[derive(Config)]` 即可定义配置 |
| **类型安全** | 配置结构的编译时类型检查 |
| **多源支持** | 自动合并文件、环境变量和远程源 |

它专为需要稳健、生产级配置管理方式的 **Rust 开发者** 设计。

**了解更多：** [用户指南](USER_GUIDE.md)

</details>

<details style="padding:16px; margin: 8px 0">
<summary style="cursor:pointer; font-weight:600; color:#1E293B">❓ 为什么应该使用 Confers？</summary>

<div style="padding:16px">

| 功能 | Confers | Figment | Config-rs |
|:-----|:-------:|:-------:|:---------:|
| 类型安全 | ✅ **强** | ✅ 良好 | ⚠️ 手动 |
| 热重载 | ✅ **内置** | ⚠️ 手动 | ⚠️ 手动 |
| 验证 | ✅ **集成** | ⚠️ 手动 | ⚠️ 手动 |
| 审计日志 | ✅ **包含** | ❌ 否 | ❌ 否 |

</div>

**主要优势：**

- 🚀 **零样板代码**：用最少的代码加载复杂配置
- 🔄 **智能合并**：自动处理多个来源之间的优先级
- 🛡️ **安全性**：内置敏感字段加密和屏蔽支持
- 📊 **可观测性**：详细的审计日志，记录每个配置值的来源

</details>

<details style="padding:16px; margin: 8px 0">
<summary style="cursor:pointer; font-weight:600; color:#1E293B">❓ Confers 可以用于生产环境了吗？</summary>

<div style="padding:16px; margin: 16px 0">

**当前状态：** ✅ **生产就绪！**

</div>

<table style="width:100%; border-collapse: collapse">
<tr>
<td width="50%" style="padding: 16px">

**已就绪功能：**

- ✅ 核心加载逻辑稳定
- ✅ 支持主要格式（TOML、JSON、YAML）
- ✅ 环境变量覆盖
- ✅ 验证框架
- ✅ 远程源（Etcd、Consul）

</td>
<td width="50%" style="padding: 16px">

**成熟度指标：**

- 📊 广泛的测试套件
- 🔄 定期维护
- 🛡️ 安全导向设计
- 📖 不断增长的文档

</td>
</tr>
</table>> **注意：** 在升级版本之前，请务必查看 [CHANGELOG](../CHANGELOG.md)。

</details>

<details style="padding:16px; margin: 8px 0">
<summary style="cursor:pointer; font-weight:600; color:#1E293B">❓ 支持哪些平台？</summary>

<div style="padding:16px">

| 平台 | 架构 | 状态 | 备注 |
|:-----|:-----|:----:|:-----|
| **Linux** | x86_64 | ✅ 完全支持 | 主要平台 |
| | ARM64 | ✅ 完全支持 | 在 ARM 服务器上测试 |
| **macOS** | x86_64 | ✅ 完全支持 | Intel Mac |
| | ARM64 | ✅ 完全支持 | Apple Silicon (M1/M2/M3) |
| **Windows** | x86_64 | ✅ 完全支持 | Windows 10+ |

</div>

</details>

---

## 安装与配置

<div align="center" style="margin: 24px 0">

### 🚀 快速开始

</div>

<details style="padding:16px; margin: 8px 0">
<summary style="cursor:pointer; font-weight:600; color:#1E293B">❓ 如何安装？</summary>

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

<details style="padding:16px; margin: 8px 0">
<summary style="cursor:pointer; font-weight:600; color:#1E293B">❓ 如何选择合适的特性组合？</summary>

<div style="padding:16px">

**特性预设（推荐）：**

| 预设 | 说明 | 适用场景 |
|:----:|------|:---------|
| <span style="color:#166534; padding:4px 8px">minimal</span> | 仅配置加载 | 仅需基本配置加载功能 |
| <span style="color:#1E40AF; padding:4px 8px">recommended</span> | 配置加载 + 验证 | 大多数应用（推荐） |
| <span style="color:#92400E; padding:4px 8px">dev</span> | 开发配置 | 开发和调试 |
| <span style="color:#991B1B; padding:4px 8px">production</span> | 生产配置 | 生产环境 |
| <span style="color:#5B21B6; padding:4px 8px">full</span> | 所有功能 | 需要完整功能 |

**使用示例：**

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

</div>> 💡 **提示**: 默认特性为 `derive`（仅配置加载）。如需验证功能，请使用 `recommended` 预设或显式启用 `validation` 特性。

</details>

<details style="padding:16px; margin: 8px 0">
<summary style="cursor:pointer; font-weight:600; color:#1E293B">❓ 不同特性组合的依赖数量有何差异？</summary>

<div style="padding:16px">

| 特性组合 | 依赖数量 | 编译时间 | 二进制大小 |
|:---------|:--------:|:--------:|:----------:|
| `minimal` | ~15 个 | 最短 | 最小 |
| `recommended` | ~20 个 | 短 | 小 |
| `dev` | ~30 个 | 中 | 中 |
| `production` | ~35 个 | 中 | 中 |
| `full` | ~50+ 个 | 长 | 大 |

</div>

选择合适的特性组合可以显著减少编译时间和二进制大小。

</details>

<details style="padding:16px; margin: 8px 0">
<summary style="cursor:pointer; font-weight:600; color:#1E293B">❓ 系统要求是什么？</summary>

**最低要求：**

| 组件 | 要求 | 推荐 |
|:-----|:----:|:-----:|
| Rust 版本 | 1.75+ | 最新稳定版 |
| 内存 | 最小 | - |
| 磁盘空间 | 最小 | - |

**可选：**

- 🔧 `watch` 功能需要操作系统级别的文件通知支持
- ☁️ `remote` 功能需要访问配置中心的网络访问

</details>

---

## 使用与功能

<div align="center" style="margin: 24px 0">

### 💡 使用 API

</div>

<details style="padding:16px; margin: 8px 0">
<summary style="cursor:pointer; font-weight:600; color:#1E293B">❓ 如何开始基本使用？</summary>

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
    let config = AppConfig::load()?;
    
    println!("主机: {}, 端口: {}", config.host, config.port);
    Ok(())
}
```

</details>

<details style="padding:16px; margin: 8px 0">
<summary style="cursor:pointer; font-weight:600; color:#1E293B">❓ 支持哪些格式和来源？</summary>

**支持的格式：**

| ✅ 格式 | 描述 |
|:-------:|:-----|
| TOML | 首选格式 |
| JSON | 通用格式 |
| YAML | 人类可读 |
| INI | 简单格式 |

**支持的来源：**

| ✅ 来源 | 描述 |
|:-------:|:-----|
| 文件 | 自动检测 `config.{toml,json,yaml,ini}` |
| 环境变量 | 支持自定义前缀 |
| CLI 参数 | 与 `clap` 集成 |
| 远程 | Etcd、Consul、HTTP |
| 默认值 | 在结构体定义中指定 |

</details>

<details style="padding:16px; margin: 8px 0">
<summary style="cursor:pointer; font-weight:600; color:#1E293B">❓ 可以验证配置吗？</summary>

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

</details>

---

## 性能

<div align="center" style="margin: 24px 0">

### ⚡ 速度和优化

</div>

<details style="padding:16px; margin: 8px 0">
<summary style="cursor:pointer; font-weight:600; color:#1E293B">❓ Confers 有多快？</summary>

**基准测试结果（加载 100+ 个键）：**

| 来源 | 格式 | 延迟（平均） |
|:-----|:-----|:------------:|
| 本地文件 | TOML | ~0.5 毫秒 |
| 环境变量 | - | ~0.1 毫秒 |
| 远程（Etcd） | JSON | ~5-20 毫秒 |

**自行运行基准测试：**

```bash
cargo bench
```

</details>

<details style="padding:16px; margin: 8px 0">
<summary style="cursor:pointer; font-weight:600; color:#1E293B">❓ 内存使用情况如何？</summary>

**典型内存使用：**

Confers 使用极少的内存，标准应用程序配置通常 **小于 1MB**。它在可能的情况下使用 `serde` 进行零拷贝反序列化。

**内存安全：**

- ✅ 无内存泄漏（通过持续测试验证）
- ✅ 敏感数据使用后可清零
- ✅ 利用 Rust 的所有权模型保证安全

</details>

---

## 安全

<div align="center" style="margin: 24px 0">

### 🔒 安全功能

</div>

<details style="padding:16px; margin: 8px 0">
<summary style="cursor:pointer; font-weight:600; color:#1E293B">❓ Confers 安全吗？</summary>

**是的！** 安全是 Confers 的核心关注点。

<div style="padding:16px; margin: 16px 0">

**安全功能：**

| 实现 | 保护 |
|:-----|:-----|
| ✅ 内存安全（Rust） | ✅ 缓冲区溢出保护 |
| ✅ 敏感字段屏蔽 | ✅ 抗侧信道攻击 |
| ✅ 恒定时间加密 | ✅ 内存擦除（zeroize） |
| ✅ 安全路径验证 | ✅ 静态加密（v0.4.0+） |

</div>

</details>

<details style="padding:16px; margin: 8px 0">
<summary style="cursor:pointer; font-weight:600; color:#1E293B">❓ 如何报告安全漏洞？</summary>

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

---

## 故障排除

<div align="center" style="margin: 24px 0">

### 🔧 常见问题

</div>

<details style="padding:16px; margin: 8px 0">
<summary style="cursor:pointer; font-weight:600; color:#991B1B">❓ 我遇到 "FileNotFound" 错误</summary>

**问题：**

```
Error: 配置文件未找到: config.toml
```

**解决方案：**

1. 确保文件位于根目录或 `config/` 目录中
2. 检查文件名（支持：`config.toml`、`config.json`、`config.yaml`、`config.ini`）
3. 如果使用自定义路径，请确保路径正确

</details>

<details style="padding:16px; margin: 8px 0">
<summary style="cursor:pointer; font-weight:600; color:#92400E">❓ 我遇到 "ValidationError"</summary>

**问题：**

```
Error: 验证失败: ...
```

**解决方案：**

1. 查看错误消息，了解哪个字段失败以及失败原因
2. 确保配置文件或环境变量符合预期格式和约束

</details>

---

## 贡献

<div align="center" style="margin: 24px 0">

### 🤝 加入社区

</div>

<details style="padding:16px; margin: 8px 0">
<summary style="cursor:pointer; font-weight:600; color:#1E293B">❓ 如何贡献？</summary>

**贡献方式：**

| 代码贡献 | 非代码贡献 |
|:---------|:----------|
| 🐛 修复 bug | 📖 编写教程 |
| ✨ 添加功能 | 🎨 设计资源 |
| 📝 改进文档 | 🌍 翻译文档 |
| ✅ 编写测试 | 💬 回答问题 |

**入门指南：**

1. 🍴 Fork 仓库
2. 🌱 创建分支
3. ✏️ 进行更改
4. ✅ 添加测试
5. 📤 提交 PR

**指南：** [CONTRIBUTING.md](CONTRIBUTING.md)

</details>

<details style="padding:16px; margin: 8px 0">
<summary style="cursor:pointer; font-weight:600; color:#1E293B">❓ 在哪里可以获得帮助？</summary>

**支持渠道：**

| 渠道 | 描述 | 响应时间 |
|:-----|:-----|:--------:|
| 🐛 [GitHub Issues](https://github.com/Kirky-X/confers/issues) | Bug 报告和功能请求 | 关键 bug：24 小时 |
| 💬 [GitHub Discussions](https://github.com/Kirky-X/confers/discussions) | 问答和想法 | 2-3 天 |
| 💡 [Discord](https://discord.gg/project) | 实时聊天 | 即时 |

</details>

---

## 许可

<div align="center" style="margin: 24px 0">

### 📄 许可信息

</div>

<details style="padding:16px; margin: 8px 0">
<summary style="cursor:pointer; font-weight:600; color:#1E293B">❓ 这是什么许可证？</summary>

**双重许可：**

<table style="width:100%; border-collapse: collapse">
<tr>
<td width="50%" style="padding: 16px; text-align:center">

**MIT 许可证**

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](../LICENSE-MIT)

**权限：**
- ✅ 商业使用
- ✅ 修改
- ✅ 分发
- ✅ 私人使用

</td>
<td width="50%" style="padding: 16px; text-align:center">

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

---

<div align="center" style="margin: 32px 0; padding: 24px">

### 🎯 仍然有问题？

| 创建 Issue | 开始讨论 | 发送邮件 |
|:----------:|:--------:|:--------:|
| [🐛 报告问题](https://github.com/Kirky-X/confers/issues) | [💬 社区讨论](https://github.com/Kirky-X/confers/discussions) | [📧 联系支持](mailto:support@example.com) |

---

**[📖 用户指南](USER_GUIDE.md)** • **[🔧 API 文档](https://docs.rs/confers)** • **[🏠 首页](../README.md)**

由 Kirky.X 用 ❤️ 制作

**[⬆ 返回顶部](#top)**

</div>