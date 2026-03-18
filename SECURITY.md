# Security Policy

<span id="top"></span>

<div align="center">

<img src="docs/image/confers.png" alt="Confers Logo" width="150" style="margin-bottom: 16px">

### Security at Confers

[🏠 Home](README.md) • [📖 User Guide](docs/USER_GUIDE.md) • [🐛 Report Vulnerability](#reporting)

---

</div>

## Table of Contents

- [Reporting Security Vulnerabilities](#reporting)
- [Security Features](#features)
- [Best Practices](#best-practices)
- [Dependency Security](#dependencies)
- [Security Audit Process](#audit-process)

---

## <span id="reporting">Reporting Security Vulnerabilities</span>

We take security vulnerabilities seriously. If you discover a security issue, please report it responsibly.

### How to Report

**Please DO NOT file a public GitHub issue for security vulnerabilities.**

| Method | Contact | Response Time |
|:-------|:--------|:--------------|
| **Email** | security@confers.dev | Within 48 hours |
| **GitHub Security Advisories** | [Report via GH Advisory](https://github.com/Kirky-X/confers/security/advisories/new) | Within 48 hours |
| **Encrypted Email** | PGP key available on our security page | Within 48 hours |

### What to Include

When reporting, please include:

1. **Description**: Clear description of the vulnerability
2. **Steps to Reproduce**: Detailed steps to reproduce the issue
3. **Impact Assessment**: How this vulnerability could be exploited
4. **Affected Versions**: Which versions are affected
5. **Suggested Fix** (optional): If you have identified a potential fix

### Our Commitment

| Stage | Timeline | Action |
|:------|:---------|:-------|
| **Acknowledgment** | Within 48 hours | Confirm receipt of report |
| **Initial Assessment** | Within 7 days | Severity classification |
| **Fix Development** | Varies by severity | Priority fix implementation |
| **Coordinated Disclosure** | After fix available | Public announcement |

### Severity Classification

| Severity | Examples | Response |
|:---------|:---------|:---------|
| **Critical** | Remote code execution, data exfiltration | Fix within 72 hours |
| **High** | Privilege escalation, denial of service | Fix within 7 days |
| **Medium** | Information disclosure, bypass | Fix within 30 days |
| **Low** | Minor security improvements | Fix in next release |

---

## <span id="features">Security Features</span>

Confers includes multiple layers of security to protect your configuration data.

### Encryption (XChaCha20-Poly1305)

All sensitive configuration data can be encrypted at rest using XChaCha20-Poly1305:

```rust
use confers::{Config, encryption::EncryptionManager};

// Enable encryption feature in Cargo.toml
// features = ["encryption"]

#[derive(Config)]
pub struct SecureConfig {
    #[config(sensitive = true)]
    pub database_url: String,
    #[config(sensitive = true)]
    pub api_key: String,
}

// Encrypt configuration
let manager = EncryptionManager::new(key);
let encrypted = manager.encrypt(&config)?;
```

### Memory Safety

Sensitive data is automatically zeroized when dropped:

```rust
use confers::security::SecureString;

let secret = SecureString::new("api-key-12345", SensitivityLevel::High);
// Automatically zeroized when dropped
```

### Input Validation

All user inputs are validated to prevent injection attacks:

```rust
use confers::validator::{InputValidator, ValidationConfig};

let validator = InputValidator::new()
    .enable_sql_injection_check()
    .enable_command_injection_check();

let result = validator.validate(user_input);
```

### SSRF Protection

Remote configuration URLs are validated to prevent Server-Side Request Forgery:

```rust
use confers::remote::HttpProvider;

let provider = HttpProvider::new()
    .enable_ssrf_protection()
    .validate_remote_url("https://config.example.com/app.toml")?;
```

### Audit Logging

All configuration access and changes are logged:

```rust
use confers::audit::{AuditConfig, AuditLevel};

let audit = AuditConfig::new()
    .set_level(AuditLevel::All)
    .enable_sensitive_field_tracking();

audit.log_access("config.load", "user@example.com")?;
```

### Security Module APIs

```rust
// EnvSecurityValidator - Environment variable security
use confers::security::EnvSecurityValidator;
let validator = EnvSecurityValidator::new();
validator.validate_env_vars()?;

// ErrorSanitizer - Sensitive data redaction in errors
use confers::security::ErrorSanitizer;
let sanitizer = ErrorSanitizer::default();
let safe_error = sanitizer.sanitize(&error_message);

// ConfigInjector - Secure runtime injection
use confers::security::ConfigInjector;
let injector = ConfigInjector::new()
    .enable_input_validation();
```

---

## <span id="best-practices">Security Best Practices</span>

### For Library Users

| Practice | Description | Priority |
|:---------|:------------|:---------|
| **Use HTTPS** | Always use HTTPS for remote configuration | Required |
| **Limit Key Scope** | Use least-privilege keys for remote access | Required |
| **Enable Audit Logging** | Track all configuration access in production | Required |
| **Rotate Keys** | Regularly rotate encryption keys | Required |
| **Validate Inputs** | Never trust user-provided configuration | Required |
| **Secure File Permissions** | Set restrictive permissions on config files | Recommended |
| **Use Encryption** | Encrypt sensitive config at rest | Recommended |

### Configuration Example (Production)

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

```bash
# Required for production
CONFERS_ENCRYPTION_KEY=your-256-bit-key
CONFERS_AUDIT_ENABLED=true

# Optional security hardening
CONFERS_MAX_MEMORY_MB=512
CONFERS_TIMEOUT_SECONDS=30
CONFERS_SSRF_BLOCKLIST=/etc/confers/blocklist.txt
```

### Hardening Checklist

- [ ] Enable encryption for sensitive configuration
- [ ] Configure TLS for all remote sources
- [ ] Set up audit logging
- [ ] Implement key rotation
- [ ] Enable SSRF protection
- [ ] Configure input validation
- [ ] Set appropriate memory limits
- [ ] Review security events regularly

---

## <span id="dependencies">Dependency Security</span>

### Dependency Management

We use `cargo-audit` to monitor known vulnerabilities in dependencies:

```bash
# Run security audit
cargo audit

# Update advisory database
cargo audit --fetch-index
```

### Minimum Supported Rust Version (MSRV)

Confers requires Rust 1.81+ to ensure:
- Latest security fixes in the Rust standard library
- Stable async trait support
- Memory safety guarantees

### Dependency Approval Process

All new dependencies must meet:
1. **Active Maintenance**: Recent commits (within 6 months)
2. **Security History**: No known unfixed vulnerabilities
3. **Minimal Dependencies**: Prefer small, focused crates
4. **License Compatibility**: MIT or Apache-2.0 preferred

### Known and Accepted Risks

| Dependency | Risk | Mitigation |
|:-----------|:-----|:----------|
| `serde` | Complex serialization | Widely audited, essential |
| `tokio` | Large binary | Only when async features used |
| `reqwest` | HTTP client surface | Enable TLS explicitly |

---

## <span id="audit-process">Security Audit Process</span>

### Internal Audits

We conduct regular internal security reviews:
- **Frequency**: Quarterly
- **Scope**: New features, dependency updates, API changes
- **Documentation**: Findings tracked in security advisory DB

### External Audits

For major releases, we engage external security researchers:
- **Trigger**: Major version releases (0.x.0)
- **Scope**: Full codebase audit
- **Results**: Published after remediation

### Vulnerability Disclosure Timeline

```
Day 0: Vulnerability discovered
Day 1-2: Report acknowledged
Day 3-10: Severity assessed, fix developed
Day 11-30: Fix released (if severity allows)
Day 31+: Public disclosure (if still unfixed)
```

### Security-Related Commits

All security fixes follow this process:

1. **Private Branch**: Fix developed in private branch
2. **CVE Filing**: If applicable, file CVE with MITRE
3. **Coordinated Release**: Fix released simultaneously with disclosure
4. **Post-Mortem**: Internal review of how vulnerability occurred

---

## Security Contacts

| Role | Contact |
|:-----|:--------|
| Security Team | security@confers.dev |
| Maintainer | Kirky-X@outlook.com |
| PGP Key | Available on keys.openpgp.org |

---

<div align="center">

**[⬆ Back to Top](#top)**

Built with security in mind by Kirky.X

</div>
