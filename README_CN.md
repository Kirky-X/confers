[English](README.md) | 中文

<div align="center">

<img src="docs/assets/confers.png" alt="Confers Logo" width="240">

[![Rust 2024](https://img.shields.io/badge/Rust-2024-edded?logo=rust&style=for-the-badge)](https://www.rust-lang.org/) [![Rustc 1.97.1+](https://img.shields.io/badge/Rustc-1.97.1+-orange.svg?style=for-the-badge)](https://www.rust-lang.org/) [![crates.io](https://img.shields.io/crates/v/confers?style=for-the-badge)](https://crates.io/crates/confers) [![docs.rs](https://img.shields.io/docsrs/confers?style=for-the-badge)](https://docs.rs/confers) [![downloads](https://img.shields.io/crates/d/confers?style=for-the-badge)](https://crates.io/crates/confers) [![CI](https://img.shields.io/github/actions/workflow/status/Kirky-X/confers/ci.yml?branch=main&label=CI&style=for-the-badge)](https://github.com/Kirky-X/confers/actions/workflows/ci.yml) [![coverage](https://img.shields.io/codecov/c/github/Kirky-X/confers?style=for-the-badge)](https://codecov.io/gh/Kirky-X/confers) [![license](https://img.shields.io/crates/l/confers?style=for-the-badge)](LICENSE)

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

<details open>
<summary>📑 目录</summary>

- [✨ 功能特性](#-功能特性)
- [🚀 快速开始](#-快速开始)
- [🎨 特性标志](#-特性标志)
- [📚 文档](#-文档)
- [💻 示例](#-示例)
- [🔒 安全](#-安全)
- [🗺️ 开发路线图](#️-开发路线图)
- [🤝 参与贡献](#-参与贡献)
- [📋 更新日志](#-更新日志)
- [📄 许可证](#-许可证)
- [🙏 致谢](#-致谢)
- [📞 联系与支持](#-联系与支持)
- [⭐ Star 历史](#-star-历史)

</details>

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
<td width="50%" style="vertical-align:top; padding: 12px">📢 <b>变更广播</b><br><span style="color:#64748B">NATS JetStream（至少一次）/ Redis Pub-Sub（至多一次，断线自动重连）消息总线，多实例配置同步</span></td>
<td width="50%" style="vertical-align:top; padding: 12px">📊 <b>Schema 生成</b><br><span style="color:#64748B">自动生成 JSON Schema 与 TypeScript 类型定义</span></td>
</tr>
<tr>
<td width="50%" style="vertical-align:top; padding: 12px">🧾 <b>审计日志</b><br><span style="color:#64748B">HMAC 签名保护完整性，敏感字段自动脱敏</span></td>
<td width="50%" style="vertical-align:top; padding: 12px">🧰 <b>CLI 诊断工具</b><br><span style="color:#64748B">inspect、validate、export、diff、snapshot、schema、get、docs、doctor 子命令</span></td>
</tr>
</table>

除上述核心能力外，配置迁移、快照与回滚、变量插值、模块化配置、上下文感知、运行时特性开关、OpenFeature 风格评估、安全规则校验器、密钥管理与云 KMS、惰性分段解析、统一变更流等能力也均以独立特性标志提供；完整的功能矩阵（逐项对应 `Cargo.toml` 的 `[features]` 单特性定义）见 [🎨 特性标志](#-特性标志) 一节。

---

## 🚀 快速开始

### 📦 安装

<details>
<summary>📄 代码 · bash</summary>

```bash
cargo add confers
```

</details>

要求 Rust 1.97.1 及以上（MSRV，与仓库 `rust-toolchain.toml` 一致）。默认特性包含 `toml`、`json`、`dotenv`；各功能预设（`minimal` / `recommended` / `dev` / `production` / `distributed` / `full`）的安装方式与特性清单统一见下方 [🎨 特性标志](#-特性标志) 一节。

### 💡 最小示例

以下示例改编自 [`examples/src/examples/basic_usage.rs`](examples/src/examples/basic_usage.rs)：

<details>
<summary>📄 代码 · rust</summary>

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

</details>

<details>
<summary>📄 代码 · bash</summary>

```bash
# 环境变量覆盖默认值
export PORT=9000
cargo run    # 输出: 监听地址: 127.0.0.1:9000
```

</details>

---

## 🎨 特性标志

### 📦 功能预设

<details>
<summary>📋 表格</summary>

| 预设 | 安装方式 | 包含特性 | 适用场景 |
|------|----------|----------|----------|
| 默认 | `cargo add confers` | `toml`、`json`、`dotenv` | 开箱即用 |
| `minimal` | `cargo add confers --no-default-features --features minimal` | `dotenv`、`json` | 最小化加载 |
| `recommended` | `cargo add confers --no-default-features --features recommended` | `toml`、`dotenv`、`validation`、`json`、`security-rules` | 大多数应用 |
| `dev` | `cargo add confers --features dev` | `toml`、`json`、`yaml`、`dotenv`、`cli`、`validation`、`json-schema`、`audit`、`watch`、`migration`、`snapshot`、`dynamic` | 开发环境全套工具 |
| `production` | `cargo add confers --features production` | `toml`、`dotenv`、`watch`、`encryption`、`validation`、`audit`、`json-schema`、`cli`、`migration`、`dynamic`、`progressive-reload`、`snapshot`、`security-rules`、`feature-toggle` | 生产环境 |
| `distributed` | `cargo add confers --features distributed` | `toml`、`json`、`dotenv`、`watch`、`validation`、`config-bus`、`progressive-reload`、`audit` | 分布式系统 |
| `full` | `cargo add confers --features full` | 全部 39 项正式特性（不含 `hot-reload-kit` 与待移除兼容别名 `env`/`schema`/`key`） | 完整能力集 |

</details>

### 📋 功能矩阵

下表逐项对应 `Cargo.toml` 的 `[features]` 单特性定义（预设见上表），`default = ["toml", "json", "dotenv"]`。

<details>
<summary>📋 表格</summary>

<table style="width:100%; border-collapse: collapse">
<tr><th style="text-align:left">特性</th><th style="text-align:center">默认</th><th style="text-align:left">说明</th></tr>
<tr><td colspan="3" style="background:#F8FAFC"><b>格式支持</b></td></tr>
<tr><td><code>toml</code></td><td align="center">✅</td><td>TOML 配置文件</td></tr>
<tr><td><code>json</code></td><td align="center">✅</td><td>JSON 配置文件</td></tr>
<tr><td><code>yaml</code></td><td align="center">❌</td><td>YAML 配置文件</td></tr>
<tr><td><code>ini</code></td><td align="center">❌</td><td>INI 配置文件</td></tr>
<tr><td><code>dotenv</code></td><td align="center">✅</td><td>环境变量加载与 <code>.env</code> 文件</td></tr>
<tr><td><code>env</code></td><td align="center">❌</td><td><code>dotenv</code> 的兼容别名（启用时编译告警提示，下一版移除）</td></tr>
<tr><td colspan="3" style="background:#F8FAFC"><b>核心能力</b></td></tr>
<tr><td><code>validation</code></td><td align="center">❌</td><td>基于 garde 的配置校验</td></tr>
<tr><td><code>watch</code></td><td align="center">❌</td><td>文件监听与热重载，自适应去抖</td></tr>
<tr><td><code>encryption</code></td><td align="center">❌</td><td>XChaCha20-Poly1305 加密与 HKDF 字段密钥派生</td></tr>
<tr><td><code>cli</code></td><td align="center">❌</td><td>confers 命令行诊断工具</td></tr>
<tr><td><code>json-schema</code></td><td align="center">❌</td><td>JSON Schema 生成</td></tr>
<tr><td><code>typescript-schema</code></td><td align="center">❌</td><td>TypeScript 类型生成（含 <code>json-schema</code>）</td></tr>
<tr><td><code>schema</code></td><td align="center">❌</td><td><code>json-schema</code> 的兼容别名（启用时编译告警提示，下一版移除）</td></tr>
<tr><td><code>dynamic</code></td><td align="center">❌</td><td>动态字段，arc-swap 无锁读取</td></tr>
<tr><td><code>progressive-reload</code></td><td align="center">❌</td><td>渐进式重载，金丝雀发布与健康检查回滚（含 <code>watch</code>）</td></tr>
<tr><td><code>hot-reload-kit</code></td><td align="center">❌</td><td>热重载工具集 <code>HotReloader</code> 门面（<code>watch</code> + <code>progressive-reload</code> 组合；不入 <code>full</code> 预设）</td></tr>
<tr><td><code>async-core</code></td><td align="center">❌</td><td>内部异步聚合特性：<code>remote</code> / <code>config-bus</code> / <code>encryption</code> / <code>watch</code> 任一启用时自动生效（实现侧统一以它门控异步代码）</td></tr>
<tr><td><code>audit</code></td><td align="center">❌</td><td>审计日志，HMAC 完整性与敏感字段脱敏；<code>AuditSink</code> 端口由同工作区 inklog 的 <code>confers-audit</code> 特性实现（可编译实跑示例见 inklog 仓库 <code>examples/src/bin/config/confers_audit.rs</code>）</td></tr>
<tr><td><code>migration</code></td><td align="center">❌</td><td>配置版本迁移</td></tr>
<tr><td><code>snapshot</code></td><td align="center">❌</td><td>快照与回滚</td></tr>
<tr><td><code>interpolation</code></td><td align="center">❌</td><td><code>${VAR}</code> 变量插值，支持嵌套默认值</td></tr>
<tr><td><code>tracing</code></td><td align="center">❌</td><td>内部 tracing 门面：load/reload/decrypt/remote_fetch 关键路径 span 与结构化事件（未启用时为 no-op）</td></tr>
<tr><td colspan="3" style="background:#F8FAFC"><b>安全</b></td></tr>
<tr><td><code>security</code></td><td align="center">❌</td><td>安全模块：<code>SecureString</code> 安全字符串、错误脱敏 <code>ErrorSanitizer</code>（与 <code>encryption</code> 解耦，仅依赖 zeroize/sha2/hex 轻量原语）</td></tr>
<tr><td><code>security-rules</code></td><td align="center">❌</td><td>内置 JWT、CORS、SSRF、TLS 校验器与注册表</td></tr>
<tr><td><code>key-management</code></td><td align="center">❌</td><td>密钥生命周期管理与轮换（含 <code>encryption</code>）</td></tr>
<tr><td><code>key</code></td><td align="center">❌</td><td><code>key-management</code> 的兼容别名（启用时编译告警提示，下一版移除）</td></tr>
<tr><td><code>keyring</code></td><td align="center">❌</td><td>密钥存储后端（文件、MasterKey、secret-tool，含 <code>encryption</code>）</td></tr>
<tr><td><code>cloud-kms</code></td><td align="center">❌</td><td>云 KMS 密钥提供方：Vault Transit / AWS KMS（SigV4）/ GCP KMS（metadata token）（含 <code>encryption</code>，依赖 <code>remote</code>）</td></tr>
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
<tr><td><code>canary</code></td><td align="center">❌</td><td>多实例金丝雀发布编排：消费变更流分批推进/回滚 + 服务网格权重适配（含 <code>change-stream</code>、<code>progressive-reload</code>）</td></tr>
<tr><td colspan="3" style="background:#F8FAFC"><b>组织与扩展</b></td></tr>
<tr><td><code>modules</code></td><td align="center">❌</td><td>模块化配置分组与注册表</td></tr>
<tr><td><code>context-aware</code></td><td align="center">❌</td><td>上下文感知（租户维度）配置</td></tr>
<tr><td><code>feature-toggle</code></td><td align="center">❌</td><td>运行时特性开关注册表</td></tr>
<tr><td><code>openfeature</code></td><td align="center">❌</td><td>OpenFeature 风格特性评估（含 <code>feature-toggle</code>、<code>context-aware</code>）</td></tr>
<tr><td><code>lazy-parse</code></td><td align="center">❌</td><td>超大文档惰性分段解析</td></tr>
</table>

</details>

---

## 📚 文档

<details>
<summary>📋 表格</summary>

| 文档 | 说明 |
|------|------|
| [📖 用户指南](docs/cn/USER_GUIDE.md) | 从安装到进阶的完整使用教程 |
| [📘 API 参考](docs/cn/API_REFERENCE.md) | 全部公开 API 的详细说明 |
| [🏗️ 架构文档](docs/cn/ARCHITECTURE.md) | 设计原则、模块划分与数据流 |
| [🧭 宏配置指南](docs/cn/CONFIG_MACRO_GUIDE.md) | `Config` 派生宏与 `#[config(...)]` 属性的完整用法 |
| [📈 性能优化指南](docs/cn/PERFORMANCE.md) | 基准数据、性能口径与优化建议 |
| [🔒 安全文档](docs/cn/SECURITY.md) | 安全设计、最佳实践与漏洞处理记录 |
| [❓ FAQ](docs/cn/FAQ.md) | 常见问题解答 |
| [📚 库集成指南](docs/cn/LIBRARY_INTEGRATION.md) | 如何将 confers CLI 集成到您的项目 |
| [🧪 测试场景矩阵](docs/cn/TEST_SCENARIOS.md) | E2E 验收场景穷举矩阵 |
| [🕊️ 金丝雀发布编排](docs/cn/CANARY_ORCHESTRATION.md) | 跨实例分批 rollout 编排与服务网格权重下发 |
| [📋 更新日志](docs/cn/CHANGELOG.md) | 每个版本的变更记录 |
| [🤝 贡献指南](docs/cn/CONTRIBUTING.md) | 如何参与项目开发 |
| [🤝 行为准则](docs/cn/CODE_OF_CONDUCT.md) | 社区行为准则与执行方针 |
| [📦 在线 API 文档](https://docs.rs/confers) | docs.rs 自动生成的最新文档 |
| [📦 crates.io](https://crates.io/crates/confers) | 发布页面 |

</details>

---

## 💻 示例

全部 22 个可运行示例位于 [`examples/`](examples/) 目录，每个示例对应一个 `cargo run --bin` 目标，覆盖基础加载、热重载、加密与密钥轮换、校验、插值、审计、快照、迁移、动态字段、远程来源（etcd / Consul）、总线、渐进发布、Schema 生成等全部特性域；逐示例的文件、依赖服务与验收标准见 [🧪 测试场景文档 · examples 运行清单](docs/cn/TEST_SCENARIOS.md)。

---

## 🔒 安全

请勿通过公开 issue 报告安全漏洞：请使用 GitHub [Security Advisories](https://github.com/Kirky-X/confers/security/policy) 私密披露通道提交报告。安全设计、供应链门禁与漏洞处理流程见 [🔒 安全文档](docs/cn/SECURITY.md)。

---

## 🗺️ 开发路线图

<details>
<summary>📋 表格</summary>

<table style="width:100%; border-collapse: collapse">
<tr><th style="text-align:center">状态</th><th style="text-align:left">方向</th><th style="text-align:left">条目</th></tr>
<tr><td align="center">✅</td><td>核心引擎</td><td>派生宏、多格式支持、来源链合并、环境变量与 CLI 覆盖</td></tr>
<tr><td align="center">✅</td><td>校验与 Schema</td><td>garde 校验、JSON Schema 生成、TypeScript 类型生成</td></tr>
<tr><td align="center">✅</td><td>热更新</td><td>文件监听热重载、渐进式发布、动态字段、快照回滚、配置迁移、变量插值</td></tr>
<tr><td align="center">✅</td><td>安全与审计</td><td>XChaCha20-Poly1305 加密、密钥管理与轮换、审计日志、安全规则校验器</td></tr>
<tr><td align="center">✅</td><td>远程与总线</td><td>HTTP 轮询、etcd、Consul、Nacos、Kubernetes ConfigMap / Secret、NATS 与 Redis 总线</td></tr>
<tr><td align="center">✅</td><td>远程来源稳定</td><td><code>remote</code>、<code>etcd</code>、<code>consul</code>、<code>etcd-watch</code> 转正：接口已冻结，1.0 前不再破坏性变更（watch 兼容性回归测试守护）</td></tr>
<tr><td align="center">✅</td><td>性能优化</td><td>criterion 基线（<a href="docs/cn/PERFORMANCE.md#-性能基线">性能基线</a>：加载 / 合并 / watch 分发）、内存上限与惰性分段解析、高频读取零拷贝热路径（<code>get_shared</code> 约 2.1x）</td></tr>
<tr><td align="center">✅</td><td>云原生集成</td><td>服务网格流量切分（<code>canary</code> 编排器 + <a href="docs/cn/CANARY_ORCHESTRATION.md">Envoy/Istio 样例</a>）、分布式追踪集成（<code>tracing</code> 门面：load/reload/decrypt/remote_fetch span）</td></tr>
</table>

</details>

---

## 🤝 参与贡献

详细的贡献流程与代码规范请参阅 [🤝 贡献指南](docs/cn/CONTRIBUTING.md)。

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

完整版本历史见 [📋 更新日志](docs/cn/CHANGELOG.md)（遵循 [Keep a Changelog](https://keepachangelog.com/en/1.0.0/) 格式，语义化版本）。

---

## 📄 许可证

本项目基于 [MIT](LICENSE) 许可证发布。

Copyright (c) 2026 Kirky.X🌠

---

## 🙏 致谢

### 🌟 核心依赖

Confers 站在以下优秀开源项目的肩膀上：

<details>
<summary>📋 表格</summary>

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

</details>

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
