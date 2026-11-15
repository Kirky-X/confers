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
# Run the security audit
cargo audit

# Update the advisory database
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
Day 0: vulnerability discovered
Day 1-2: report receipt confirmed
Day 3-10: severity graded, fix developed
Day 11-30: fix released (as severity allows)
Day 31+: public disclosure (if still unfixed)
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

// Enable the encryption feature in Cargo.toml
// features = ["encryption"]

#[derive(Config)]
pub struct SecureConfig {
    // sensitive fields must be of type SecretString / SecretBytes,
    // otherwise the macro expansion fails
    #[config(sensitive = true)]
    pub database_url: SecretString,
    #[config(sensitive = true, encrypt = "xchacha20")]
    pub api_key: String,
}

// Low-level crypto API: XChaCha20-Poly1305 requires a 32-byte key (this layer only checks length,
// it does not reject all-zero constant keys); weak (constant-byte) key rejection happens at higher entry points:
// ConfigBuilder::master_key、EnvKeyProvider/FileKeyProvider。
// Keys should be randomly generated; never use constant keys.
let crypto = XChaCha20Crypto::new();
let mut key_bytes = [0u8; 32];
getrandom::fill(&mut key_bytes).expect("OS entropy"); // in practice, read from a key service or environment variable
let key: &[u8] = &key_bytes;

// Encrypt: returns (nonce, ciphertext); the nonce is 24 random bytes
let (nonce, ciphertext) = crypto.encrypt(b"敏感数据", key)?;

// Decrypt: note the argument order — nonce first
let plaintext = crypto.decrypt(&nonce, &ciphertext, key)?;
```

### Memory Safety

Sensitive data is zeroed automatically when dropped (zeroize; requires the `security` feature — decoupled from `encryption`, relying only on the lightweight zeroize/sha2/hex primitives and never pulling in the encryption stack):

```rust,ignore
use confers::security::{SecureString, SensitivityLevel};

let secret = SecureString::new("api-key-12345", SensitivityLevel::High);
// Debug output is redacted automatically; zeroed on drop
```

### Config Security Validation

Run built-in security rule validation over config content (SSRF, TLS settings, JWT secret strength, CORS), available with the `security-rules` feature:

```rust,ignore
use confers::security::rules::SecurityValidatorRegistry;

let registry = SecurityValidatorRegistry::with_defaults();

// Runs all registered validators against any ConfigProvider implementation and returns a SecurityReport
// (confers ships no built-in implementation; implement the ConfigProvider trait yourself)
let report = registry.validate_all(&config_provider);

// is_ok(false): only Critical violations return false (with fail_on_warning=false, Warnings do not matter); is_ok(true) requires zero violations
if !report.is_ok(false) {
    // handle security violations…
}
```

### SSRF Protection

Remote config URLs are validated against server-side request forgery (SSRF):

- **Source side** (`remote` feature): `HttpPolledSourceBuilder` enforces HTTPS at build time and rejects blocked network-range targets (loopback, private, link-local included); hostname-based targets are re-validated after DNS resolution on the async polling path.
- **Rules side** (`security-rules` feature): `SsrfValidator` applies the same blocked-range and URL boundary-matching checks to URL config in any `ConfigProvider`, executable uniformly via `SecurityValidatorRegistry`.

```rust,ignore
use confers::remote::HttpPolledSourceBuilder;

// Plain http:// and private-network addresses are rejected at build() with an error
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

// Create the audit writer with the builder pattern
let writer = AuditWriter::builder()
    .log_dir(PathBuf::from("/var/log/confers"))
    .enabled(true)
    .build();

// Record an audit event
writer.log_load("config.toml")?;
writer.log_key_access("database_password")?;
writer.log_decrypt("api_key", true)?;
```

Before hitting disk, audit events are HMAC-SHA256 chain-signed (with a random salt at the chain head); `verify_audit_chain(path)` validates log integrity.

**Threat model and external keys**: in the default mode the salt doubles as the HMAC key and is stored plaintext in the log file header, so the chain only defends against partial tampering, read-only auditors, and lazy rewrites — an attacker who can rewrite the whole file can forge a valid chain. For higher-assurance scenarios, inject an external key (env/secret service/KMS) via `AuditWriterBuilder::hmac_key(...)`; the file then no longer contains the MAC key, and verification must use `verify_audit_chain_with_key(path, key)`.

**Encrypted-value envelope**: encrypted values uniformly use the `enc:v1:<key_version>:<base64(nonce||ciphertext)>` envelope (the legacy `enc:v1:<payload>` and `enc:<payload>` forms remain readable, with key_version defaulting to `v1`); both `verify` and `doctor` parse it accordingly. Key rotation locates the decryption key via the envelope's `key_version`.

### security Module API

```rust,ignore
// EnvSecurityValidator - environment variable security validation (security feature)
use confers::security::EnvSecurityValidator;
let validator = EnvSecurityValidator::strict();
validator.validate_env_name("APP_NAME", None)?;
validator.validate_env_value("production")?;

// SecurityErrorSanitizer - redacts sensitive data in error messages (security feature)
// Note: this sanitizer's mask keeps the first 4-8 characters of the masked value (e.g. pa****)
// to identify the credential kind; to leak no prefix at all, use
// confers::error::sanitize::sanitize_error_message。
// The bare-name ErrorSanitizer is no longer exported (disambiguation vs key::ErrorSanitizer);
// security::SecureLogger / SafeResult / SensitiveDataFilter /
// the five deprecated types FilterResult / LogLevel etc. have been removed entirely.
use confers::security::SecurityErrorSanitizer;

// ConfigInjector - runtime config injection with validation and rate limiting (security-rules feature)
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
# Enable all security features
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
# Required in production (encryption master key: 32 bytes hex-encoded, or a 32-byte ASCII key;
# can also be injected explicitly via ConfigBuilder::master_key to override)
CONFERS_MASTER_KEY=your-256-bit-key

# Other built-in variables
CONFERS_LANG=zh                       # message language (overrides system locale detection)
CONFERS_RATE_LIMIT_MAX_REQUESTS=100   # config-injection rate limit: max requests per window
CONFERS_RATE_LIMIT_WINDOW_SECONDS=60  # config-injection rate limit: window length (seconds)
# Locale detection chain (fallback order when CONFERS_LANG is unset): LC_ALL / LC_MESSAGES / LANG

# Key/credential integration reads (when the corresponding features are enabled; all are "fallbacks when not explicitly configured" — the crate runs fine without them)
VAULT_TOKEN=...            # token fallback for the Vault KeyProvider (encryption+remote) and the Cloud KMS Vault backend (cloud-kms)
AWS_ACCESS_KEY_ID=...      # AWS KMS (cloud-kms): access key fallback
AWS_SECRET_ACCESS_KEY=...
AWS_SESSION_TOKEN=...      # optional: session token fallback
KUBERNETES_SERVICE_HOST=...        # in-cluster API detection (k8s): service host
KUBERNETES_SERVICE_PORT_HTTPS=...  # optional; falls back to KUBERNETES_SERVICE_PORT, then 443
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
