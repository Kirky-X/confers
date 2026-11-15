# 🔒 Confers Security

Confers treats security as a core design goal. This document covers Confers' security policy, vulnerability reporting process, built-in security mechanisms, and security best practices.

## 📋 Table of Contents

- [Supported Versions and Maintenance Policy](#-supported-versions-and-maintenance-policy)
- [Vulnerability Reporting Process](#-vulnerability-reporting-process)
- [Security Design Overview](#️-security-design-overview)
- [Security Best Practices](#-security-best-practices)
- [Related Documents](#-related-documents)

---

## 📌 Supported Versions and Maintenance Policy

This project is in the 0.x pre-release stage and **maintains only the current minor version line** (`0.6.x`, including rc pre-releases within that line).

- **Current line**: receives bug fixes and security fixes, delivered with the latest release on that line.
- **Older minor lines**: go EOL the moment the first version of the next minor line ships — no further fixes of any kind, and no backported security patches.
- **No LTS**: before 1.0 there is no cross-minor API compatibility promise; minor-position upgrades may contain breaking changes — read the [CHANGELOG](CHANGELOG.md) before upgrading.
- **Visible on crates.io ≠ supported**: older versions remain downloadable (unless yanked for severe defects) but are outside the supported scope.

| Version Line | Status | Description |
|--------|------|------|
| `0.6.x` (current) | ✅ supported | the only line receiving fixes |
| `≤ 0.5.x` | ❌ EOL | superseded by the current line, unmaintained — please upgrade |

### Minimum Supported Rust Version (MSRV)

Confers requires Rust 1.97.1+ to ensure:

- The Rust standard library includes the latest security fixes
- Stable async trait support
- Full memory safety guarantees

### Dependency Security

We use `cargo-audit` to monitor dependencies for known vulnerabilities:

```bash
# 运行安全审计
cargo audit

# 更新漏洞通告数据库
cargo audit --fetch-index
```

### Dependency Approval Process

Every new dependency must satisfy:

1. **Actively maintained**: recent commits (within 6 months)
2. **Security history**: no known unfixed vulnerabilities
3. **Minimal dependencies**: prefer small, focused crates
4. **License compatibility**: prefer MIT or Apache-2.0

### Known and Acceptable Risks

| Dependency | Risk | Mitigation |
|:-----|:-----|:---------|
| `serde` | complex serialization logic | extensively audited, indispensable |
| `tokio` | large binary size | pulled in only when async-related features are enabled |
| `reqwest` | HTTP client attack surface | TLS explicitly enabled |

### Supply Chain and Gates

The repository enforces supply-chain security through the following automated gates, executed doubly by CI and local Git hooks (lefthook):

| Gate | Tool | When | Description |
|:-----|:-----|:---------|:-----|
| Dependency vulnerabilities / licenses / banned deps / source checks | `cargo deny check` (config in `deny.toml`) | CI and pre-commit | blocks dependencies with known vulnerabilities, non-compliant licenses, or banned status |
| RustSec advisory scanning | `cargo audit` | CI and pre-push | scans `Cargo.lock` against known RustSec advisories |
| Private-key leak scanning | lefthook `no-private-key` | pre-commit | blocks PEM private keys and `sk-`-style tokens from entering commits |
| Coverage gate | `cargo llvm-cov --fail-under-lines 90` | CI and pre-push | line coverage no less than 90% |

---

## 🐛 Vulnerability Reporting Process

We take security vulnerabilities very seriously. If you discover a security issue, please report it responsibly.

### How to Report

**Do not open a public GitHub issue for security vulnerabilities.**

| Channel | Contact | Response Time |
|:-----|:---------|:---------|
| **GitHub security advisory (preferred)** | [report via GH Advisory](https://github.com/Kirky-X/confers/security/policy) | within 48 hours |
| **Email (alternative)** | Kirky-X@outlook.com | within 48 hours |

### What to Include

When reporting, please include:

1. **Description**: a clear description of the vulnerability itself
2. **Reproduction steps**: detailed steps reproducing the issue
3. **Impact assessment**: how the vulnerability might be exploited
4. **Affected versions**: which versions are affected
5. **Fix suggestions** (optional): if you already have a potential fix in mind

### ⏱️ Response Commitments

| Phase | Timeline | Action |
|:-----|:-------|:-----|
| **Acknowledgment** | within 48 hours | confirm receipt of the report |
| **Initial assessment** | within 7 days | complete severity rating |
| **Fix development** | severity-dependent | implement the fix by priority |
| **Coordinated disclosure** | after the fix is available | publish a public announcement |

### Severity Levels

| Severity | Example | Response Requirement |
|:---------|:-----|:---------|
| **Critical (severe)** | remote code execution, data exfiltration | fix within 72 hours |
| **High** | privilege escalation, denial of service | fix within 7 days |
| **Medium** | information disclosure, security bypass | fix within 30 days |
| **Low** | general security improvements | fix in the next release |

### Vulnerability Disclosure Timeline

```text
Day 0：发现漏洞
Day 1-2：确认收到报告
Day 3-10：完成定级，开发修复
Day 11-30：发布修复（视严重程度允许的情况）
Day 31+：公开披露（若仍未修复）
```

### Security-Related Commit Process

All security fixes follow this process:

1. **Private branch**: fixes developed on a private branch
2. **CVE filing**: where applicable, file a CVE with MITRE
3. **Coordinated release**: fix and disclosure ship in sync
4. **Retrospective**: internally review how the vulnerability arose

---

## 🛡️ Security Design Overview

Confers protects your configuration data through multiple layers of security mechanisms.

### Encryption (XChaCha20-Poly1305)

All sensitive configuration data can be encrypted at rest with XChaCha20-Poly1305 (requires the `encryption` feature):

```rust,ignore
use confers::{Config, XChaCha20Crypto};
use confers::secret::SecretString;

// 在 Cargo.toml 中启用 encryption 特性
// features = ["encryption"]

#[derive(Config)]
pub struct SecureConfig {
    // sensitive 字段必须为 SecretString / SecretBytes 类型，
    // 否则宏展开期报错
    #[config(sensitive = true)]
    pub database_url: SecretString,
    #[config(sensitive = true, encrypt = "xchacha20")]
    pub api_key: String,
}

// 低层加密 API：XChaCha20-Poly1305 要求 32 字节密钥（本层只校验长度，
// 不拒绝全零等常量密钥）；弱（常量字节）密钥拒绝在更高层入口生效：
// ConfigBuilder::master_key、EnvKeyProvider/FileKeyProvider。
// 密钥应随机生成，勿使用常量密钥。
let crypto = XChaCha20Crypto::new();
let mut key_bytes = [0u8; 32];
getrandom::fill(&mut key_bytes).expect("OS entropy"); // 实际应从密钥服务或环境变量读取
let key: &[u8] = &key_bytes;

// 加密：返回 (nonce, ciphertext)，nonce 为随机 24 字节
let (nonce, ciphertext) = crypto.encrypt(b"敏感数据", key)?;

// 解密：注意参数顺序为 nonce 在前
let plaintext = crypto.decrypt(&nonce, &ciphertext, key)?;
```

### Memory Safety

Sensitive data is zeroed automatically when dropped (zeroize; requires the `security` feature — decoupled from `encryption`, relying only on the lightweight zeroize/sha2/hex primitives and never pulling in the encryption stack):

```rust,ignore
use confers::security::{SecureString, SensitivityLevel};

let secret = SecureString::new("api-key-12345", SensitivityLevel::High);
// Debug 输出自动脱敏；丢弃时自动清零
```

### Config Security Validation

Run built-in security rule validation over config content (SSRF, TLS settings, JWT secret strength, CORS), available with the `security-rules` feature:

```rust,ignore
use confers::security::rules::SecurityValidatorRegistry;

let registry = SecurityValidatorRegistry::with_defaults();

// 对任意 ConfigProvider 实现执行全部已注册校验器，返回 SecurityReport
// （confers 不提供内置实现，需自行实现 ConfigProvider trait）
let report = registry.validate_all(&config_provider);

// is_ok(false)：仅 Critical 违规返回 false（fail_on_warning=false 时 Warning 不影响）；is_ok(true) 才要求零违规
if !report.is_ok(false) {
    // 处理安全违规……
}
```

### SSRF Protection

Remote config URLs are validated against server-side request forgery (SSRF):

- **Source side** (`remote` feature): `HttpPolledSourceBuilder` enforces HTTPS at build time and rejects blocked network-range targets (loopback, private, link-local included); hostname-based targets are re-validated after DNS resolution on the async polling path.
- **Rules side** (`security-rules` feature): `SsrfValidator` applies the same blocked-range and URL boundary-matching checks to URL config in any `ConfigProvider`, executable uniformly via `SecurityValidatorRegistry`.

```rust,ignore
use confers::remote::HttpPolledSourceBuilder;

// 明文 http:// 与私网地址会在 build() 阶段被拒绝并返回错误
let source = HttpPolledSourceBuilder::new()
    .url("https://config.example.com/app.toml")
    .build()?;
```

The public interfaces of the remote sources (`remote`/`etcd`/`consul`/`etcd-watch`) are frozen; the SSRF defaults above and the security-related defaults below are frozen contracts and will not be loosened or broken before 1.0:

- `HttpPolledSourceBuilder::stale_on_error` defaults to off — a fetch failure errors immediately (fail-loud) and never silently falls back to possibly-stale config;
- TLS for etcd / Consul is configured explicitly via `EtcdTlsConfig` / `ConsulTlsConfig`; without it connections are plaintext, and production should configure TLS for all remote sources per the [security hardening checklist](#security-hardening-checklist).
- TLS for etcd / Consul is configured explicitly via `EtcdTlsConfig` / `ConsulTlsConfig`; without it connections are plaintext, and production should configure TLS for all remote sources per the [security hardening checklist](#security-hardening-checklist).

### Audit Logging

Config loads, key access, and decryption events are all recorded (requires the `audit` feature):

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

Before hitting disk, audit events are HMAC-SHA256 chain-signed (with a random salt at the chain head); `verify_audit_chain(path)` validates log integrity.

**Threat model and external keys**: in the default mode the salt doubles as the HMAC key and is stored plaintext in the log file header, so the chain only defends against partial tampering, read-only auditors, and lazy rewrites — an attacker who can rewrite the whole file can forge a valid chain. For higher-assurance scenarios, inject an external key (env/secret service/KMS) via `AuditWriterBuilder::hmac_key(...)`; the file then no longer contains the MAC key, and verification must use `verify_audit_chain_with_key(path, key)`.

**Encrypted-value envelope**: encrypted values uniformly use the `enc:v1:<key_version>:<base64(nonce||ciphertext)>` envelope (the legacy `enc:v1:<payload>` and `enc:<payload>` forms remain readable, with key_version defaulting to `v1`); both `verify` and `doctor` parse it accordingly. Key rotation locates the decryption key via the envelope's `key_version`.

### security Module API

```rust,ignore
// EnvSecurityValidator - 环境变量安全校验（security 特性）
use confers::security::EnvSecurityValidator;
let validator = EnvSecurityValidator::strict();
validator.validate_env_name("APP_NAME", None)?;
validator.validate_env_value("production")?;

// SecurityErrorSanitizer - 错误信息中的敏感数据脱敏（security 特性）
// 注意：该脱敏器的掩码会保留被遮蔽值的前 4-8 个字符（如 pa****），
// 用于辨识凭证种类；需要不泄露任何前缀时用
// confers::error::sanitize::sanitize_error_message。
// 裸名 ErrorSanitizer 不再导出（与 key::ErrorSanitizer 消歧），
// security::SecureLogger / SafeResult / SensitiveDataFilter /
// FilterResult / LogLevel 五个已废弃类型已整体移除。
use confers::security::SecurityErrorSanitizer;

// ConfigInjector - 带校验与限速的运行时配置注入（security-rules 特性）
use confers::security::ConfigInjector;
let injector = ConfigInjector::new()
    .max_entries(1000)
    .with_dedicated_rate_limiter();
injector.inject("APP_PORT", "8080")?;
```

### Security Audit Process

**Internal audits**

We run internal security reviews periodically:

- **Frequency**: quarterly
- **Scope**: new features, dependency updates, API changes
- **Records**: findings are recorded in the security advisory database

**External audits**

For major releases we invite external security researchers:

- **Trigger**: major releases (0.x.0)
- **Scope**: full-codebase audit
- **Results**: published once fixes are complete

---

## ✅ Security Best Practices

### For Library Consumers

| Practice | Description | Priority |
|:-----|:-----|:-------|
| **Use HTTPS** | always use HTTPS for remote config | mandatory |
| **Limit key scope** | least-privilege keys for remote access | mandatory |
| **Enable audit logging** | record all config access in production | mandatory |
| **Rotate keys** | rotate encryption keys periodically | mandatory |
| **Validate input** | never trust user-supplied configuration | mandatory |
| **Tighten file permissions** | set restrictive permissions on config files | recommended |
| **Enable encryption** | encrypt sensitive config at rest | recommended |

### Production Configuration Example

Below is an example of an application declaring security-related entries in its own config file (Confers handles encryption and redaction via field-level attributes):

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

### Environment Variables

The built-in environment variables actually read by `confers` code (verified exhaustively against `src/`):

```bash
# 生产环境必填（加密主密钥：32 字节 hex 编码，或 32 字节 ASCII 密钥；
# 也可经 ConfigBuilder::master_key 显式注入覆盖）
CONFERS_MASTER_KEY=your-256-bit-key

# 其余内置变量
CONFERS_LANG=zh                       # 消息语言（覆盖系统 locale 检测）
CONFERS_RATE_LIMIT_MAX_REQUESTS=100   # 配置注入限流：窗口内最大请求数
CONFERS_RATE_LIMIT_WINDOW_SECONDS=60  # 配置注入限流：窗口时长（秒）
# locale 检测链（CONFERS_LANG 未设置时按序回退）：LC_ALL / LC_MESSAGES / LANG

# 密钥/凭证集成读取（对应特性启用时；均为「未显式配置时的回退」，不设也可运行）
VAULT_TOKEN=...            # Vault KeyProvider（encryption+remote）与 Cloud KMS Vault backend（cloud-kms）的 token 回退
AWS_ACCESS_KEY_ID=...      # AWS KMS（cloud-kms）：访问密钥回退
AWS_SECRET_ACCESS_KEY=...
AWS_SESSION_TOKEN=...      # 可选：会话令牌回退
KUBERNETES_SERVICE_HOST=...        # In-cluster API 探测（k8s）：服务主机
KUBERNETES_SERVICE_PORT_HTTPS=...  # 可选；缺省回退 KUBERNETES_SERVICE_PORT，再缺省 443
KUBERNETES_SERVICE_PORT=...
```

> ⚠️ **Note**: security options such as audit switches, memory caps, timeouts, and SSRF blacklists have no corresponding built-in environment variables — set them explicitly in code via configuration items like `AuditConfig`, `ConfigLimits`, and `SecurityValidatorRegistry`.

### Security Hardening Checklist

- [ ] Enable encryption for sensitive config
- [ ] Configure TLS for all remote sources
- [ ] Configure audit logging
- [ ] Implement key rotation
- [ ] Enable SSRF protection
- [ ] Configure input validation
- [ ] Set sensible memory caps
- [ ] Review security events periodically

---

## 📎 Security Contact

| Role | Contact |
|:-----|:---------|
| Kirky.X (security report recipient) | Kirky-X@outlook.com |

## 📚 Related Documents

| Document | Description |
|:-----|:-----|
| [📘 API Reference](API_REFERENCE.md) | complete API of the security / audit / secret modules |
| [📖 User Guide](USER_GUIDE.md) | security configuration practices section |
| [❓ FAQ](FAQ.md) | frequently asked security questions |
