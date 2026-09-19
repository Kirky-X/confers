<div align="center">

<img src="docs/assets/confers.png" alt="Confers Logo" width="180">

[![CI Status](https://github.com/Kirky-X/confers/actions/workflows/ci.yml/badge.svg)](https://github.com/Kirky-X/confers/actions/workflows/ci.yml) [![Version](https://img.shields.io/crates/v/confers.svg)](https://crates.io/crates/confers) [![Docs.rs](https://docs.rs/confers/badge.svg)](https://docs.rs/confers) [![Downloads](https://img.shields.io/crates/d/confers.svg)](https://crates.io/crates/confers) [![License](https://img.shields.io/crates/l/confers.svg)](LICENSE) [![Rust](https://img.shields.io/badge/rust-1.97.1%2B-orange.svg)](https://www.rust-lang.org/) [![Coverage](https://codecov.io/gh/Kirky-X/confers/branch/main/graph/badge.svg)](https://codecov.io/gh/Kirky-X/confers)

**中文** | [English](README_EN.md)

**生产级 Rust 配置库，零样板代码**

[✨ 功能特性](#-功能特性) • [🚀 快速开始](#-快速开始) • [📚 文档](#-文档) • [💻 示例](#-示例) • [🤝 参与贡献](#-参与贡献)

</div>

---

<div align="center">

### 🎯 一处声明，多来源汇入

给结构体标注 `#[derive(Config)]`，按优先级合并各来源，编译期产出强类型字段：

<table style="width:100%; border-collapse: collapse">
<tr>
<td align="center" width="25%">🧩<br><b>派生宏驱动</b><br><span style="color:#64748B">加载 · 默认值 · 环境覆盖</span></td>
<td align="center" width="25%">🛡️<br><b>类型安全</b><br><span style="color:#64748B">合并即定型 · 可选校验</span></td>
<td align="center" width="25%">🔄<br><b>热重载</b><br><span style="color:#64748B">渐进发布 · 异常自动回滚</span></td>
<td align="center" width="25%">🔐<br><b>端到端加密</b><br><span style="color:#64748B">认证加密 · 字段级子密钥</span></td>
</tr>
</table>

</div>

---

## 📋 目录

- [✨ 功能特性](#-功能特性)
- [🚀 快速开始](#-快速开始)
- [🎨 特性标志](#-特性标志)
- [📚 文档](#-文档)
- [💻 示例](#-示例)
- [🏗️ 架构](#️-架构)
- [🧪 测试](#-测试)
- [📊 性能](#-性能)
- [🔒 安全](#-安全)
- [🗺️ 开发路线图](#️-开发路线图)
- [🤝 参与贡献](#-参与贡献)
- [📋 更新日志](#-更新日志)
- [📄 许可证](#-许可证)
- [🙏 致谢](#-致谢)
- [📞 联系与支持](#-联系与支持)
- [⭐ Star 历史](#-star-历史)

---

## ✨ 功能特性

<table style="width:100%; border-collapse: collapse">
<tr>
<td width="50%" style="vertical-align:top; padding: 12px">🧩 <b>派生宏驱动</b><br><span style="color:#64748B"><code>#[derive(Config)]</code> 与 <code>#[config(...)]</code> 属性在编译期生成加载、默认值与环境变量覆盖代码</span></td>
<td width="50%" style="vertical-align:top; padding: 12px">🗂️ <b>多格式支持</b><br><span style="color:#64748B">TOML、JSON、YAML、INI 与 <code>.env</code>，支持按内容自动探测格式</span></td>
</tr>
<tr>
<td width="50%" style="vertical-align:top; padding: 12px">🔗 <b>多来源优先级链</b><br><span style="color:#64748B">文件、环境变量、内存与远程来源按声明顺序合并，每个值携带来源与位置元数据</span></td>
<td width="50%" style="vertical-align:top; padding: 12px">🛡️ <b>类型安全与校验</b><br><span style="color:#64748B">合并结果反序列化为强类型结构体，可选 garde 规则校验</span></td>
</tr>
<tr>
<td width="50%" style="vertical-align:top; padding: 12px">🔄 <b>热重载</b><br><span style="color:#64748B">文件监听与自适应去抖，支持渐进发布与健康检查自动回滚</span></td>
<td width="50%" style="vertical-align:top; padding: 12px">⚡ <b>动态字段</b><br><span style="color:#64748B">基于 arc-swap 的无锁运行时更新，支持回调与字段监听</span></td>
</tr>
<tr>
<td width="50%" style="vertical-align:top; padding: 12px">🔐 <b>配置加密</b><br><span style="color:#64748B">XChaCha20-Poly1305 认证加密，HKDF-SHA256 按字段派生子密钥</span></td>
<td width="50%" style="vertical-align:top; padding: 12px">🌐 <b>远程配置</b><br><span style="color:#64748B">HTTP 轮询、etcd v3、Consul、Nacos、Kubernetes，内置熔断器与 SSRF 防护</span></td>
</tr>
<tr>
<td width="50%" style="vertical-align:top; padding: 12px">📢 <b>变更广播</b><br><span style="color:#64748B">NATS / Redis Pub-Sub 消息总线，多实例配置同步</span></td>
<td width="50%" style="vertical-align:top; padding: 12px">📊 <b>Schema 生成</b><br><span style="color:#64748B">自动生成 JSON Schema 与 TypeScript 类型定义</span></td>
</tr>
<tr>
<td width="50%" style="vertical-align:top; padding: 12px">🧾 <b>审计日志</b><br><span style="color:#64748B">HMAC 签名保护完整性，敏感字段自动脱敏</span></td>
<td width="50%" style="vertical-align:top; padding: 12px">🧰 <b>CLI 诊断工具</b><br><span style="color:#64748B">inspect、validate、diff、export、snapshot、schema、doctor 等子命令</span></td>
</tr>
</table>

除上述核心能力外，配置迁移、快照与回滚、变量插值、模块化配置、上下文感知、运行时特性开关、OpenFeature 风格评估、安全规则校验器、密钥管理与云 KMS、惰性分段解析、统一变更流等能力也均以独立特性标志提供；完整的功能矩阵（逐项对应 `Cargo.toml` 的 `[features]` 定义）见 [🎨 特性标志](#-特性标志) 一节。

---

## 🚀 快速开始

### 📦 安装

```bash
cargo add confers
```

要求 Rust 1.97.1 及以上（MSRV，与仓库 `rust-toolchain.toml` 一致）。默认特性包含 `toml`、`json`、`env`；各功能预设（`minimal` / `recommended` / `dev` / `production` / `distributed` / `full`）的安装方式与特性清单统一见下方 [🎨 特性标志](#-特性标志) 一节。

### 💡 最小示例

以下示例改编自 [`examples/src/examples/basic_usage.rs`](examples/src/examples/basic_usage.rs)：

```rust
use confers::Config;
use serde::Deserialize;

#[derive(Config, Deserialize, Debug, Clone)]
pub struct ConfersConfig {
    /// 服务器监听地址
    #[config(default = "127.0.0.1".to_string())]
    pub host: String,

    /// 服务器监听端口
    #[config(default = 8080u16)]
    pub port: u16,

    /// 日志级别
    #[config(default = "info".to_string())]
    pub log_level: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 读取字段默认值，环境变量 HOST、PORT、LOG_LEVEL 可覆盖
    let config = ConfersConfig::load_sync()?;

    println!("监听地址: {}:{}", config.host, config.port);
    Ok(())
}
```

```bash
# 环境变量覆盖默认值
export PORT=9000
cargo run    # 输出: 监听地址: 127.0.0.1:9000
```

### 🧭 核心概念

- **来源链**：通过 `ConfigBuilder` / `SourceChainBuilder` 声明 `FileSource`、`EnvSource`、`MemorySource` 与远程来源，声明顺序即优先级，后加入者覆盖先加入者。
- **注解值**：每个配置值包装为 `AnnotatedValue`，携带 `SourceId` 与 `SourceLocation`（精确到行列），冲突可溯源。
- **双阶段错误**：初始化期失败返回 `ConfigConfigError`，运行期失败返回 `ConfersError`，两类问题分开处理。
- **特性门控**：全部可选能力均为独立 feature，编译产物只包含启用的部分，最小可只用 `env` + `json`。

---

## 🎨 特性标志

### 📦 功能预设

| 预设 | 安装方式 | 包含特性 | 适用场景 |
|------|----------|----------|----------|
| 默认 | `cargo add confers` | `toml`、`json`、`env` | 开箱即用 |
| `minimal` | `cargo add confers --no-default-features --features minimal` | `env`、`json` | 最小化加载 |
| `recommended` | `cargo add confers --no-default-features --features recommended` | `toml`、`env`、`validation`、`json`、`security-rules` | 大多数应用 |
| `dev` | `cargo add confers --features dev` | `toml`、`json`、`yaml`、`env`、`cli`、`validation`、`schema`、`audit`、`watch`、`migration`、`snapshot`、`dynamic` | 开发环境全套工具 |
| `production` | `cargo add confers --features production` | `toml`、`env`、`watch`、`encryption`、`validation`、`audit`、`schema`、`cli`、`migration`、`dynamic`、`progressive-reload`、`snapshot`、`security-rules`、`feature-toggle` | 生产环境 |
| `distributed` | `cargo add confers --features distributed` | `toml`、`json`、`env`、`watch`、`validation`、`config-bus`、`progressive-reload`、`audit` | 分布式系统 |
| `full` | `cargo add confers --features full` | 全部特性 | 完整能力集 |

### 📋 功能矩阵

下表逐项对应 `Cargo.toml` 的 `[features]` 定义，`default = ["toml", "json", "env"]`。

<table style="width:100%; border-collapse: collapse">
<tr><th style="text-align:left">特性</th><th style="text-align:center">默认</th><th style="text-align:left">说明</th></tr>
<tr><td colspan="3" style="background:#F8FAFC"><b>格式支持</b></td></tr>
<tr><td><code>toml</code></td><td align="center">✅</td><td>TOML 配置文件</td></tr>
<tr><td><code>json</code></td><td align="center">✅</td><td>JSON 配置文件</td></tr>
<tr><td><code>yaml</code></td><td align="center">❌</td><td>YAML 配置文件</td></tr>
<tr><td><code>ini</code></td><td align="center">❌</td><td>INI 配置文件</td></tr>
<tr><td><code>env</code></td><td align="center">✅</td><td>环境变量加载与 <code>.env</code> 文件</td></tr>
<tr><td><code>dotenv</code></td><td align="center">❌</td><td><code>env</code> 的别名</td></tr>
<tr><td colspan="3" style="background:#F8FAFC"><b>核心能力</b></td></tr>
<tr><td><code>validation</code></td><td align="center">❌</td><td>基于 garde 的配置校验</td></tr>
<tr><td><code>watch</code></td><td align="center">❌</td><td>文件监听与热重载，自适应去抖</td></tr>
<tr><td><code>encryption</code></td><td align="center">❌</td><td>XChaCha20-Poly1305 加密与 HKDF 字段密钥派生</td></tr>
<tr><td><code>cli</code></td><td align="center">❌</td><td>confers 命令行诊断工具</td></tr>
<tr><td><code>schema</code></td><td align="center">❌</td><td>JSON Schema 生成</td></tr>
<tr><td><code>typescript-schema</code></td><td align="center">❌</td><td>TypeScript 类型生成（<code>schema</code> 的别名）</td></tr>
<tr><td><code>dynamic</code></td><td align="center">❌</td><td>动态字段，arc-swap 无锁读取</td></tr>
<tr><td><code>progressive-reload</code></td><td align="center">❌</td><td>渐进式重载，金丝雀发布与健康检查回滚（含 <code>watch</code>）</td></tr>
<tr><td><code>audit</code></td><td align="center">❌</td><td>审计日志，HMAC 完整性与敏感字段脱敏</td></tr>
<tr><td><code>migration</code></td><td align="center">❌</td><td>配置版本迁移</td></tr>
<tr><td><code>snapshot</code></td><td align="center">❌</td><td>快照与回滚</td></tr>
<tr><td><code>interpolation</code></td><td align="center">❌</td><td><code>${VAR}</code> 变量插值，支持嵌套默认值</td></tr>
<tr><td><code>tracing</code></td><td align="center">❌</td><td>启用内部 tracing 门面（未启用时为 no-op）</td></tr>
<tr><td colspan="3" style="background:#F8FAFC"><b>安全</b></td></tr>
<tr><td><code>security</code></td><td align="center">❌</td><td>安全模块：加密集成、错误脱敏、环境变量校验（含 <code>encryption</code>）</td></tr>
<tr><td><code>security-rules</code></td><td align="center">❌</td><td>内置 JWT、CORS、SSRF、TLS 校验器与注册表</td></tr>
<tr><td><code>key</code></td><td align="center">❌</td><td>密钥生命周期管理与轮换（含 <code>encryption</code>）</td></tr>
<tr><td><code>keyring</code></td><td align="center">❌</td><td>密钥存储后端（文件、MasterKey、secret-tool，含 <code>encryption</code>）</td></tr>
<tr><td><code>cloud-kms</code></td><td align="center">❌</td><td>云 KMS 密钥提供方，含 Vault Transit（含 <code>encryption</code>，依赖 <code>remote</code>）</td></tr>
<tr><td colspan="3" style="background:#F8FAFC"><b>远程来源</b></td></tr>
<tr><td><code>remote</code></td><td align="center">❌</td><td>HTTP 轮询来源，含 SSRF 防护与熔断器</td></tr>
<tr><td><code>etcd</code></td><td align="center">❌</td><td>etcd v3 集成（含 <code>remote</code>）</td></tr>
<tr><td><code>etcd-watch</code></td><td align="center">❌</td><td>etcd watch 监听（含 <code>etcd</code>）</td></tr>
<tr><td><code>consul</code></td><td align="center">❌</td><td>HashiCorp Consul 集成（含 <code>remote</code>）</td></tr>
<tr><td><code>nacos</code></td><td align="center">❌</td><td>Nacos 配置中心集成（含 <code>remote</code>）</td></tr>
<tr><td><code>k8s</code></td><td align="center">❌</td><td>Kubernetes ConfigMap / Secret 来源，挂载卷与 API 两种方式（含 <code>remote</code>）</td></tr>
<tr><td colspan="3" style="background:#F8FAFC"><b>消息总线与变更流</b></td></tr>
<tr><td><code>config-bus</code></td><td align="center">❌</td><td>配置变更事件总线，基于 tokio broadcast</td></tr>
<tr><td><code>nats-bus</code></td><td align="center">❌</td><td>NATS 总线后端（含 <code>config-bus</code>）</td></tr>
<tr><td><code>redis-bus</code></td><td align="center">❌</td><td>Redis Pub/Sub 总线后端（含 <code>config-bus</code>）</td></tr>
<tr><td><code>change-stream</code></td><td align="center">❌</td><td>统一变更流端口，复用 config-bus 传输（含 <code>watch</code>）</td></tr>
<tr><td colspan="3" style="background:#F8FAFC"><b>组织与扩展</b></td></tr>
<tr><td><code>modules</code></td><td align="center">❌</td><td>模块化配置分组与注册表</td></tr>
<tr><td><code>context-aware</code></td><td align="center">❌</td><td>上下文感知（租户维度）配置</td></tr>
<tr><td><code>feature-toggle</code></td><td align="center">❌</td><td>运行时特性开关注册表</td></tr>
<tr><td><code>openfeature</code></td><td align="center">❌</td><td>OpenFeature 风格特性评估（含 <code>feature-toggle</code>、<code>context-aware</code>）</td></tr>
<tr><td><code>lazy</code></td><td align="center">❌</td><td>超大文档惰性分段解析</td></tr>
</table>

---

## 📚 文档

| 文档 | 说明 |
|------|------|
| [📖 用户指南](docs/USER_GUIDE.md) | 从安装到进阶的完整使用教程 |
| [📘 API 参考](docs/API_REFERENCE.md) | 全部公开 API 的详细说明 |
| [🏗️ 架构文档](docs/ARCHITECTURE.md) | 设计原则、模块划分与数据流 |
| [🧭 宏配置指南](docs/CONFIG_MACRO_GUIDE.md) | `Config` 派生宏与 `#[config(...)]` 属性的完整用法 |
| [📈 性能优化指南](docs/PERFORMANCE.md) | 基准数据、性能口径与优化建议 |
| [🔒 安全文档](docs/SECURITY.md) | 安全设计、最佳实践与漏洞处理记录 |
| [❓ FAQ](docs/FAQ.md) | 常见问题解答 |
| [📚 库集成指南](docs/LIBRARY_INTEGRATION.md) | 如何将 confers CLI 集成到您的项目 |
| [🧪 测试场景矩阵](docs/TEST_SCENARIOS.md) | E2E 验收场景穷举矩阵 |
| [📋 更新日志](docs/CHANGELOG.md) | 每个版本的变更记录 |
| [🤝 贡献指南](docs/CONTRIBUTING.md) | 如何参与项目开发 |
| [📦 在线 API 文档](https://docs.rs/confers) | docs.rs 自动生成的最新文档 |
| [📦 crates.io](https://crates.io/crates/confers) | 发布页面 |

---

## 💻 示例

全部 21 个可运行示例位于 [`examples/`](examples/) 目录，每个示例对应一个 `cargo run --bin` 目标，覆盖基础加载、热重载、加密与密钥轮换、校验、插值、审计、快照、迁移、动态字段、远程来源（HTTP / etcd / Consul）、总线、渐进发布、Schema 生成等全部特性域；逐示例的文件、依赖服务与验收标准见 [🧪 测试场景文档 · examples 运行清单](docs/TEST_SCENARIOS.md)。

```bash
# 运行单个示例（在 examples/ 目录下）
cd examples && cargo run --bin basic_usage
cd examples && cargo run --bin encryption

# 验证全部示例可编译
cd examples && ./verify_examples.sh
```

### 🤖 CLI 工具

除运行示例外，也可以使用随库附带的 CLI 诊断工具（`cli` 特性，入口 `src/cli/main.rs`）排查真实项目中的配置：

```bash
cargo install confers --features cli
```

提供 `inspect`、`validate`、`export`、`diff`、`snapshot`、`schema`、`get`、`doctor`、`docs --agent` 子命令，退出码约定：0 成功、1 配置错误、2 I/O 错误。全部命令的参数、输出与用法示例见 [📖 用户指南 · 命令行工具](docs/USER_GUIDE.md#-命令行工具)；将 CLI 集成到您自己的项目见 [📚 库集成指南](docs/LIBRARY_INTEGRATION.md)。

---

## 🏗️ 架构

Confers 采用门面与内部实现分离的分层设计：`src/` 公开模块只做转发动机，真正实现位于 `src/impl_/`，可选能力按 feature 独立门控；核心数据通路为来源链注册 → loader 格式探测与解析（错误精确到行列）→ `MergeEngine` 沿来源链深度合并 → serde 反序列化为强类型结构体（可选 garde 校验与敏感字段解密），派生宏由 workspace 内的 `confers-macros` 过程宏 crate 在编译期生成加载代码。

架构图、公开核心与特性门控模块表、加载 / 热重载 / 多实例变更广播数据流，以及接口隔离（`ConfigReader` / `ConfigWriter` / `ConfigConnector` / `ConfigProvider` 等 trait）与安全、性能设计，详见 [🏗️ 架构文档](docs/ARCHITECTURE.md)。

---

## 🧪 测试

### 🎯 测试策略

测试金字塔覆盖七层：`src/` 内联单元测试、按功能域组织的集成测试（`tests/core`、`tests/security`、`tests/remote`、`tests/watcher`、`tests/cli`）、24 个经 `[[test]]` 显式注册的 E2E 套件（`tests/e2e`）、宏测试（`macros/tests`，trybuild 编译失败用例）、模糊测试（`fuzz/`，3 个 cargo-fuzz 目标）、Criterion 基准（`benches/`，9 组）与公开 API 文档测试。各层命令、357 条场景矩阵与 E2E 文件映射见 [🧪 测试场景文档](docs/TEST_SCENARIOS.md)。

### ▶️ 运行命令（与 CI 一致）

```bash
# 全量测试（CI 矩阵按 default / recommended / full 三档运行）
cargo test --workspace --features full

# Lint 与格式门禁
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo fmt --all -- --check

# 覆盖率门禁：行覆盖率不低于 80%
cargo llvm-cov --workspace --all-features --fail-under-lines 80

# 基准测试
cargo bench --features dev --benches

# 模糊测试（在 fuzz/ 目录下）
cargo fuzz run parser
```

总线相关的集成测试需要本地 NATS 服务，CI 使用 `nats:2.10 -js` 容器（见 `docker-compose.test.yml`）。

### 📊 测试规模

截至 v0.6.0-rc.3：单元测试约 2100+（`src/` 内联）、集成与 E2E 共 600 个（`tests/`，53 个文件）、模糊测试目标 3 个、Criterion 基准 9 组；覆盖率门禁为行覆盖率不低于 80%，CI 与 pre-push 钩子双重执行。逐项统计见 [🧪 测试场景文档 · 统计汇总](docs/TEST_SCENARIOS.md#6-统计汇总)。

---

## 📊 性能

基线在开发机本地采集（criterion 中位数口径）：50 字段加载约 691 ns、200 字段约 713 ns、千级浅合并约 20.9 µs、变更流单订阅者往返约 1.98 µs、零拷贝读取（`get_shared`）较大值深拷贝（`get_raw`）约 2.1 倍提升；实际性能取决于配置复杂度与硬件，可运行 `cargo bench` 复现。完整基准数据表与测量口径见 [📈 性能优化指南 · 性能基线](docs/PERFORMANCE.md#-性能基线)；无锁动态读取、`IndexMap` 键序、`compact_str` 驻留、自适应去抖与特性门控裁剪等设计要点见 [🏗️ 架构文档 · 性能设计](docs/ARCHITECTURE.md#-性能设计)。

---

## 🔒 安全

### 🛡️ 安全设计

安全设计围绕敏感数据全生命周期防护展开：XChaCha20-Poly1305 认证加密与 HKDF-SHA256 字段级密钥派生、丢弃即清零的内存安全、密钥版本与轮换治理、内置 JWT/CORS/SSRF/TLS 校验规则、错误脱敏与 HMAC 签名审计日志、远程来源 SSRF 校验与熔断器。逐项机制的代码级细节见 [🏗️ 架构文档 · 安全设计](docs/ARCHITECTURE.md#-安全设计)，安全配置最佳实践与漏洞处理流程见 [🔒 安全文档](docs/SECURITY.md)。

### ⛓️ 供应链与门禁

`cargo deny check`、`cargo audit` 与 lefthook 私钥扫描、覆盖率门禁等供应链门禁在 CI 与本地 Git 钩子双重执行，完整清单见 [🔒 安全文档 · 供应链与门禁](docs/SECURITY.md#供应链与门禁)。

### 🚨 报告安全漏洞

请勿通过公开 issue 报告安全漏洞。请使用 GitHub [Security Advisories](https://github.com/Kirky-X/confers/security/advisories/new) 私密披露通道提交报告。项目承诺 48 小时内确认、7 天内给出初步评估。完整政策见 [SECURITY.md](docs/SECURITY.md)。

---

## 🗺️ 开发路线图

<table style="width:100%; border-collapse: collapse">
<tr><th style="text-align:center">状态</th><th style="text-align:left">方向</th><th style="text-align:left">条目</th></tr>
<tr><td align="center">✅</td><td>核心引擎</td><td>派生宏、多格式支持、来源链合并、环境变量与 CLI 覆盖</td></tr>
<tr><td align="center">✅</td><td>校验与 Schema</td><td>garde 校验、JSON Schema 生成、TypeScript 类型生成</td></tr>
<tr><td align="center">✅</td><td>热更新</td><td>文件监听热重载、渐进式发布、动态字段、快照回滚、配置迁移、变量插值</td></tr>
<tr><td align="center">✅</td><td>安全与审计</td><td>XChaCha20-Poly1305 加密、密钥管理与轮换、审计日志、安全规则校验器</td></tr>
<tr><td align="center">✅</td><td>远程与总线</td><td>HTTP 轮询、etcd、Consul、Nacos、Kubernetes ConfigMap / Secret、NATS 与 Redis 总线</td></tr>
<tr><td align="center">🚧</td><td>远程来源成熟度</td><td><code>remote</code>、<code>etcd</code>、<code>consul</code> 处于测试期（Beta），接口可能调整</td></tr>
<tr><td align="center">📋</td><td>性能优化</td><td>基准套件完善（criterion 基线）、大型配置内存占用优化、高频读取零拷贝热路径</td></tr>
<tr><td align="center">📋</td><td>云原生集成</td><td>服务网格支持、分布式追踪集成</td></tr>
</table>

---

## 🤝 参与贡献

详细的贡献流程与代码规范请参阅 [🤝 贡献指南](docs/CONTRIBUTING.md)。

### 🛠️ 开发环境

工具链为 Rust 1.97.1（`rust-toolchain.toml` 锁定）；提交前运行 `cargo fmt --all -- --check` 与 `cargo clippy --all-targets --all-features -- -D warnings`；[lefthook](https://github.com/evilmartians/lefthook) Git 钩子在 pre-commit 执行 rustfmt、clippy、`cargo deny check` 与私钥扫描，commit-msg 校验 Conventional Commits，pre-push 执行 `cargo audit` 与覆盖率门禁。完整环境搭建步骤见 [🤝 贡献指南 · 环境准备](docs/CONTRIBUTING.md#-环境准备)。

### 💖 贡献方式

<table style="width:100%; border-collapse: collapse">
<tr>
<td width="33%" align="center" style="padding: 16px">

### 🐛 报告 Bug

发现问题？<br>
<a href="https://github.com/Kirky-X/confers/issues/new">创建 Issue</a>

</td>
<td width="33%" align="center" style="padding: 16px">

### 💡 功能建议

有好想法？<br>
<a href="https://github.com/Kirky-X/confers/discussions">开始讨论</a>

</td>
<td width="33%" align="center" style="padding: 16px">

### 🔧 提交 PR

想贡献代码？<br>
<a href="https://github.com/Kirky-X/confers/pulls">Fork 并提交 PR</a>

</td>
</tr>
</table>

---

## 📋 更新日志

完整版本历史见 [📋 更新日志](docs/CHANGELOG.md)（遵循 [Keep a Changelog](https://keepachangelog.com/en/1.0.0/) 格式，语义化版本）。

| 版本 | 日期 | 要点 |
|------|------|------|
| 0.6.0-rc.3 | 2026-09-10 | 统一变更流端口（`change-stream`）；新增 Kubernetes / Nacos 配置源与 etcd 原生 watch；CLI 新增 `doctor`、`schema`、`get` 子命令；零拷贝热路径、审计 HMAC 链式签名与惰性分段解析 |
| 0.6.0-rc.2 | 2026-09-07 | 12 项以上 0.x 依赖刷新（async-nats 0.50、chacha20poly1305 0.11、garde 0.23 等）；测试金字塔固化，补齐 7 个 E2E 套件并注册 |
| 0.5.1 | 2026-08-06 | 新增 `SecurityValidator` 安全规则与 `FeatureToggleRegistry` 运行时开关；修复 SSRF 白名单绕过与 TLS 版本比较等问题 |

---

## 📄 许可证

本项目采用 MIT 许可证，附加 [Commons Clause](LICENSE) v1.0 条件：未经单独授权不得销售本软件。详见 [LICENSE](LICENSE)。

---

## 🙏 致谢

### 🌟 核心依赖

Confers 站在以下优秀开源项目的肩膀上：

| 依赖 | 用途 |
|------|------|
| [serde](https://github.com/serde-rs/serde) | 序列化与反序列化框架 |
| [tokio](https://github.com/tokio-rs/tokio) | 异步运行时 |
| [garde](https://github.com/jprochazk/garde) | 配置校验 |
| [arc-swap](https://github.com/vorner/arc-swap) | 无锁并发快照 |
| [chacha20poly1305](https://github.com/RustCrypto/AEADs) | 认证加密 |
| [notify-debouncer-full](https://github.com/notify-rs/notify) | 文件监听与去抖 |
| [clap](https://github.com/clap-rs/clap) | CLI 框架 |
| [schemars](https://github.com/GREsau/schemars) | JSON Schema 生成 |
| [criterion](https://github.com/bheisler/criterion.rs) | 基准测试 |

### 💝 特别感谢

感谢 Rust 社区与所有 [贡献者](https://github.com/Kirky-X/confers/graphs/contributors)。

---

## 📞 联系与支持

<table style="width:100%; max-width: 600px">
<tr>
<td align="center" width="33%">
<a href="https://github.com/Kirky-X/confers/issues"><b style="color:#991B1B">Issues</b></a><br>
<span style="color:#64748B">报告问题和 Bug</span>
</td>
<td align="center" width="33%">
<a href="https://github.com/Kirky-X/confers/discussions"><b style="color:#1E40AF">讨论区</b></a><br>
<span style="color:#64748B">提问和分享想法</span>
</td>
<td align="center" width="33%">
<a href="https://github.com/Kirky-X/confers"><b style="color:#1E293B">GitHub</b></a><br>
<span style="color:#64748B">查看源代码</span>
</td>
</tr>
</table>

---

## ⭐ Star 历史

[![Star History Chart](https://api.star-history.com/svg?repos=Kirky-X/confers&type=Date)](https://star-history.com/#Kirky-X/confers&Date)

如果这个项目对您有帮助，请考虑给它一个 ⭐️！

**由 Kirky.X 构建**

---

<sub>© 2026 Kirky.X. 保留所有权利。</sub>
