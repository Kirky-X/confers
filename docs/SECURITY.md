# 🔒 Confers 安全文档

Confers 将安全性作为核心设计目标。本文档介绍 Confers 的安全策略、漏洞报告流程、内置安全机制与安全最佳实践。

## 📋 目录

<details open>
<summary>📑 目录（点击展开）</summary>

- [支持版本](#-支持版本)
- [漏洞报告流程](#-漏洞报告流程)
- [安全设计概览](#️-安全设计概览)
- [安全最佳实践](#-安全最佳实践)

</details>

---

## 📌 支持版本

我们建议所有用户始终使用最新发布版本，以获得完整的安全修复。各版本的安全修复情况请查阅 [CHANGELOG](../CHANGELOG.md)。

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

```
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

所有敏感配置数据都可以使用 XChaCha20-Poly1305 进行静态加密：

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

// 加密：返回 (密文, nonce)
let (ciphertext, nonce) = crypto.encrypt(b"敏感数据", key)?;

// 解密：注意参数顺序为 nonce 在前
let plaintext = crypto.decrypt(&nonce, &ciphertext, key)?;
```

### 内存安全

敏感数据在被丢弃时自动清零（zeroize）：

```rust,ignore
use confers::security::SecureString;

let secret = SecureString::new("api-key-12345", SensitivityLevel::High);
// 丢弃时自动清零
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

```rust,ignore
use confers::remote::HttpProvider;

let provider = HttpProvider::new()
    .enable_ssrf_protection()
    .validate_remote_url("https://config.example.com/app.toml")?;
```

### 审计日志

所有配置访问与变更都会被记录：

```rust,ignore
use confers::audit::{AuditConfig, AuditLevel};

let audit = AuditConfig::new()
    .set_level(AuditLevel::All)
    .enable_sensitive_field_tracking();

audit.log_access("config.load", "user@example.com")?;
```

### security 模块 API

```rust,ignore
// EnvSecurityValidator - 环境变量安全校验
use confers::security::EnvSecurityValidator;
let validator = EnvSecurityValidator::new();
validator.validate_env_vars()?;

// ErrorSanitizer - 错误信息中的敏感数据脱敏
use confers::security::ErrorSanitizer;
let sanitizer = ErrorSanitizer::default();
let safe_error = sanitizer.sanitize(&error_message);

// ConfigInjector - 安全的运行时注入
use confers::security::ConfigInjector;
let injector = ConfigInjector::new()
    .enable_input_validation();
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

## 安全联系方式

| 角色 | 联系方式 |
|:-----|:---------|
| 安全团队 | Kirky-X@outlook.com |
| 维护者 | Kirky-X@outlook.com |
