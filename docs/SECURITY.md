# 🔒 Confers 安全文档

Confers 将安全性作为核心设计目标。本文档介绍 Confers 的安全策略、漏洞报告流程、内置安全机制与安全最佳实践。

## 📋 目录

- [支持版本](#-支持版本)
- [漏洞报告流程](#-漏洞报告流程)
- [安全设计概览](#️-安全设计概览)
- [安全最佳实践](#-安全最佳实践)
- [相关文档](#-相关文档)

---

## 📌 支持版本

我们建议所有用户始终使用最新发布版本，以获得完整的安全修复。各版本的安全修复情况请查阅 [CHANGELOG](CHANGELOG.md)。

### 最低支持 Rust 版本（MSRV）

Confers 要求 Rust 1.97.1+，以确保：

- Rust 标准库包含最新安全修复
- 稳定的 async trait 支持
- 完整的内存安全保证

### 依赖安全

我们使用 `cargo-audit` 监控依赖中的已知漏洞：

```bash
# 运行安全审计
cargo audit

# 更新漏洞通告数据库
cargo audit --fetch-index
```

### 依赖审批流程

所有新增依赖必须满足：

1. **活跃维护**：近期有提交（6 个月内）
2. **安全历史**：无已知未修复漏洞
3. **最小依赖**：优先选择小而专注的 crate
4. **许可证兼容**：优先 MIT 或 Apache-2.0

### 已知且可接受的风险

| 依赖 | 风险 | 缓解措施 |
|:-----|:-----|:---------|
| `serde` | 复杂序列化逻辑 | 经过广泛审计，必不可少 |
| `tokio` | 二进制体积较大 | 仅在启用异步相关特性时引入 |
| `reqwest` | HTTP 客户端攻击面 | 显式启用 TLS |

### 供应链与门禁

仓库通过以下自动化门禁保障供应链安全，CI 与本地 Git 钩子（lefthook）双重执行：

| 门禁 | 工具 | 执行时机 | 说明 |
|:-----|:-----|:---------|:-----|
| 依赖漏洞 / 许可证 / 禁用依赖 / 来源校验 | `cargo deny check`（配置见 `deny.toml`） | CI 与 pre-commit | 阻断含已知漏洞、许可证不合规或被禁用的依赖 |
| RustSec 安全公告扫描 | `cargo audit` | CI 与 pre-push | 扫描 `Cargo.lock` 中的 RustSec 已知漏洞 |
| 私钥泄露扫描 | lefthook `no-private-key` | pre-commit | 阻止 PEM 私钥与 `sk-` 形式令牌进入提交 |
| 覆盖率门禁 | `cargo llvm-cov --fail-under-lines 80` | CI 与 pre-push | 行覆盖率不低于 80% |

---

## 🐛 漏洞报告流程

我们非常重视安全漏洞。如果您发现安全问题，请负责任地进行报告。

### 如何报告

**请勿针对安全漏洞创建公开的 GitHub Issue。**

| 方式 | 联系渠道 | 响应时间 |
|:-----|:---------|:---------|
| **邮件** | Kirky-X@outlook.com | 48 小时内 |
| **GitHub 安全通告** | [通过 GH Advisory 报告](https://github.com/Kirky-X/confers/security/advisories/new) | 48 小时内 |

### 报告内容

报告时请包含：

1. **描述**：清晰描述漏洞本身
2. **复现步骤**：详细的问题复现步骤
3. **影响评估**：该漏洞可能如何被利用
4. **受影响版本**：哪些版本受到影响
5. **修复建议**（可选）：如果您已想到潜在修复方案

### 我们的承诺

| 阶段 | 时间线 | 动作 |
|:-----|:-------|:-----|
| **确认收到** | 48 小时内 | 确认收到报告 |
| **初步评估** | 7 天内 | 完成严重程度定级 |
| **修复开发** | 视严重程度而定 | 按优先级实施修复 |
| **协同披露** | 修复可用之后 | 发布公开公告 |

### 严重程度分级

| 严重程度 | 示例 | 响应要求 |
|:---------|:-----|:---------|
| **Critical（严重）** | 远程代码执行、数据外泄 | 72 小时内修复 |
| **High（高危）** | 权限提升、拒绝服务 | 7 天内修复 |
| **Medium（中危）** | 信息泄露、安全绕过 | 30 天内修复 |
| **Low（低危）** | 一般性安全改进 | 在下个版本修复 |

### 漏洞披露时间线

```text
Day 0：发现漏洞
Day 1-2：确认收到报告
Day 3-10：完成定级，开发修复
Day 11-30：发布修复（视严重程度允许的情况）
Day 31+：公开披露（若仍未修复）
```

### 安全相关提交流程

所有安全修复遵循以下流程：

1. **私有分支**：修复在私有分支中开发
2. **CVE 申报**：如适用，向 MITRE 申报 CVE
3. **协同发布**：修复与披露同步发布
4. **复盘**：内部复盘漏洞产生的原因

---

## 🛡️ 安全设计概览

Confers 通过多层安全机制保护您的配置数据。

### 加密（XChaCha20-Poly1305）

所有敏感配置数据都可以使用 XChaCha20-Poly1305 进行静态加密（需启用 `encryption` 特性）：

```rust,ignore
use confers::{Config, XChaCha20Crypto};

// 在 Cargo.toml 中启用 encryption 特性
// features = ["encryption"]

#[derive(Config)]
pub struct SecureConfig {
    #[config(sensitive = true)]
    pub database_url: String,
    #[config(sensitive = true, encrypt = "xchacha20")]
    pub api_key: String,
}

// 低层加密 API：XChaCha20-Poly1305 要求 32 字节密钥
let crypto = XChaCha20Crypto::new();
let key: &[u8] = &[0u8; 32]; // 实际应从密钥服务或环境变量读取

// 加密：返回 (nonce, ciphertext)，nonce 为随机 24 字节
let (nonce, ciphertext) = crypto.encrypt(b"敏感数据", key)?;

// 解密：注意参数顺序为 nonce 在前
let plaintext = crypto.decrypt(&nonce, &ciphertext, key)?;
```

### 内存安全

敏感数据在被丢弃时自动清零（zeroize，需启用 `encryption` 特性）：

```rust,ignore
use confers::security::{SecureString, SensitivityLevel};

let secret = SecureString::new("api-key-12345", SensitivityLevel::High);
// Debug 输出自动脱敏；丢弃时自动清零
```

### 配置安全校验

对配置内容执行内置安全规则校验（SSRF、TLS 配置、JWT 密钥强度、CORS），启用 `security-rules` 特性后可用：

```rust,ignore
use confers::security::rules::SecurityValidatorRegistry;

let registry = SecurityValidatorRegistry::with_defaults();

// 对任意 ConfigProvider 执行全部已注册校验器，返回 SecurityReport
let report = registry.validate_all(&config_provider);

// is_ok(false)：存在任何违规（含警告）即返回 false
if !report.is_ok(false) {
    // 处理安全违规……
}
```

### SSRF 防护

对远程配置 URL 进行校验，防止服务端请求伪造（SSRF）：

- **来源侧**（`remote` 特性）：`HttpPolledSourceBuilder` 在构建期强制 HTTPS，拒绝封锁网段目标（含回环、私网、链路本地地址）；域名主机在异步轮询路径解析 DNS 后再次校验。
- **规则侧**（`security-rules` 特性）：`SsrfValidator` 对任意 `ConfigProvider` 中的 URL 配置做同样的封锁网段与 URL 边界匹配检查，可经 `SecurityValidatorRegistry` 统一执行。

```rust,ignore
use confers::remote::HttpPolledSourceBuilder;

// 明文 http:// 与私网地址会在 build() 阶段被拒绝并返回错误
let source = HttpPolledSourceBuilder::new()
    .url("https://config.example.com/app.toml")
    .build()?;
```

### 审计日志

配置加载、密钥访问与解密事件都会被记录（需启用 `audit` 特性）：

```rust,ignore
use confers::audit::AuditWriter;
use std::path::PathBuf;

// 以 builder 模式创建审计写入器
let writer = AuditWriter::builder()
    .log_dir(PathBuf::from("/var/log/confers"))
    .enabled(true)
    .build();

// 记录审计事件
writer.log_load("config.toml")?;
writer.log_key_access("database_password")?;
writer.log_decrypt("api_key", true)?;
```

审计事件落盘前经 HMAC-SHA256 链式签名（链首使用随机 salt），可用 `verify_audit_chain(path)` 校验日志完整性。

### security 模块 API

```rust,ignore
// EnvSecurityValidator - 环境变量安全校验（security-rules 特性）
use confers::security::EnvSecurityValidator;
let validator = EnvSecurityValidator::strict();
validator.validate_env_name("APP_NAME", None)?;
validator.validate_env_value("production")?;

// ErrorSanitizer - 错误信息中的敏感数据脱敏（encryption 特性）
use confers::security::ErrorSanitizer;
let sanitizer = ErrorSanitizer::default();
let safe_error = sanitizer.sanitize(&error_message);

// ConfigInjector - 带校验与限速的运行时配置注入（security-rules 特性）
use confers::security::ConfigInjector;
let injector = ConfigInjector::new()
    .max_entries(1000)
    .with_dedicated_rate_limiter();
injector.inject("APP_PORT", "8080")?;
```

### 安全审计流程

**内部审计**

我们定期开展内部安全评审：

- **频率**：每季度
- **范围**：新特性、依赖更新、API 变更
- **记录**：审计发现记录在安全通告数据库中

**外部审计**

针对重要版本，我们会邀请外部安全研究人员参与：

- **触发条件**：主版本发布（0.x.0）
- **范围**：全代码库审计
- **结果**：修复完成后发布

---

## ✅ 安全最佳实践

### 面向库使用者

| 实践 | 说明 | 优先级 |
|:-----|:-----|:-------|
| **使用 HTTPS** | 远程配置始终使用 HTTPS | 必须执行 |
| **限制密钥作用域** | 远程访问使用最小权限密钥 | 必须执行 |
| **启用审计日志** | 在生产环境中记录所有配置访问 | 必须执行 |
| **轮换密钥** | 定期轮换加密密钥 | 必须执行 |
| **校验输入** | 永远不要信任用户提供的配置 | 必须执行 |
| **收紧文件权限** | 为配置文件设置 restrictive 权限 | 推荐 |
| **启用加密** | 对敏感配置进行静态加密 | 推荐 |

### 生产环境配置示例

以下是一个应用在自身配置文件中约定安全相关条目的示例（Confers 按字段级属性处理加密与脱敏）：

```toml
[security]
# 启用全部安全特性
encryption = true
audit = true
ssrf_protection = true

[security.tls]
verify = true
min_version = "1.2"
cert_path = "/etc/confers/ca.pem"

[security.audit]
level = "all"
include_sensitive = false
retention_days = 90
```

### 环境变量

```bash
# 生产环境必填
CONFERS_ENCRYPTION_KEY=your-256-bit-key
CONFERS_AUDIT_ENABLED=true

# 可选的安全加固
CONFERS_MAX_MEMORY_MB=512
CONFERS_TIMEOUT_SECONDS=30
CONFERS_SSRF_BLOCKLIST=/etc/confers/blocklist.txt
```

### 安全加固清单

- [ ] 为敏感配置启用加密
- [ ] 为所有远程来源配置 TLS
- [ ] 配置审计日志
- [ ] 实施密钥轮换
- [ ] 启用 SSRF 防护
- [ ] 配置输入校验
- [ ] 设置合理的内存上限
- [ ] 定期审查安全事件

---

## 📎 安全联系方式

| 角色 | 联系方式 |
|:-----|:---------|
| 安全团队 | Kirky-X@outlook.com |
| 维护者 | Kirky-X@outlook.com |

## 📚 相关文档

| 文档 | 说明 |
|:-----|:-----|
| [📘 API 参考](API_REFERENCE.md) | security / audit / secret 模块完整 API |
| [📖 用户指南](USER_GUIDE.md) | 安全配置实践章节 |
| [❓ FAQ](FAQ.md) | 常见安全问题解答 |
