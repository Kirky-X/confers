<span id="top"></span>
<div align="center">

<img src="image/confers.png" alt="Confers Logo" width="150" style="margin-bottom: 16px">

# ❓ Frequently Asked Questions (FAQ)


[🏠 Home](../README.md) • [📖 User Guide](USER_GUIDE.md) • [🔧 API Reference](API_REFERENCE.md)

---

</div>

## 📋 Table of Contents

<details open style="padding:16px">
<summary style="cursor:pointer; font-weight:600; color:#1E293B">📑 Table of Contents (click to expand)</summary>

- [General Questions](#general-questions)
- [Installation and Configuration](#installation-and-configuration)
- [Usage and Features](#usage-and-features)
- [Performance](#performance)
- [Security](#security)
- [Troubleshooting](#troubleshooting)
- [Contributing](#contributing)
- [Licensing](#licensing)

</details>

---

## General Questions

<div align="center" style="margin: 24px 0">

### 🤔 About the Project

</div>

<details style="padding:16px; margin: 8px 0">
<summary style="cursor:pointer; font-weight:600; color:#1E293B">❓ What is Confers?</summary>

**Confers** is a modern, type-safe Rust configuration management library. It provides:

| ✨ Feature | Description |
|:----------:|:------------|
| **Zero Boilerplate** | Define configuration with just `#[derive(Config)]` |
| **Type Safe** | Compile-time type checking for configuration structures |
| **Multi-source Support** | Automatic merging of files, environment variables, and remote sources |

It is designed for **Rust developers** who need a robust, production-grade approach to configuration management.

**Learn more:** [User Guide](USER_GUIDE.md)

</details>

<details style="padding:16px; margin: 8px 0">
<summary style="cursor:pointer; font-weight:600; color:#1E293B">❓ Why should I use Confers?</summary>

<div style="padding:16px">

| Feature | Confers | Figment | Config-rs |
|:--------|:-------:|:-------:|:---------:|
| Type Safety | ✅ **Strong** | ✅ Good | ⚠️ Manual |
| Hot Reload | ✅ **Built-in** | ⚠️ Manual | ⚠️ Manual |
| Validation | ✅ **Integrated** | ⚠️ Manual | ⚠️ Manual |
| Audit Logging | ✅ **Included** | ❌ No | ❌ No |

</div>

**Key Advantages:**

- 🚀 **Zero Boilerplate**: Load complex configurations with minimal code
- 🔄 **Smart Merging**: Automatically handles priority between multiple sources
- 🛡️ **Security**: Built-in support for sensitive field encryption and masking
- 📊 **Observability**: Detailed audit logs tracking the source of every configuration value

</details>

<details style="padding:16px; margin: 8px 0">
<summary style="cursor:pointer; font-weight:600; color:#1E293B">❓ Is Confers ready for production?</summary>

<div style="padding:16px; margin: 16px 0">

**Current Status:** ✅ **Production Ready!**

</div>

<table style="width:100%; border-collapse: collapse">
<tr>
<td width="50%" style="padding: 16px">

**Ready Features:**

- ✅ Core loading logic is stable
- ✅ Supports major formats (TOML, JSON, YAML)
- ✅ Environment variable override
- ✅ Validation framework
- ✅ Remote sources (Etcd, Consul)

</td>
<td width="50%" style="padding: 16px">

**Maturity Indicators:**

- 📊 Extensive test suite
- 🔄 Regular maintenance
- 🛡️ Security-focused design
- 📖 Growing documentation

</td>
</tr>
</table>

> **Note:** Always check the [CHANGELOG](../CHANGELOG.md) before upgrading versions.

</details>

<details style="padding:16px; margin: 8px 0">
<summary style="cursor:pointer; font-weight:600; color:#1E293B">❓ What platforms are supported?</summary>

<div style="padding:16px">

| Platform | Architecture | Status | Notes |
|:---------|:-------------|:------:|:------|
| **Linux** | x86_64 | ✅ Fully Supported | Primary platform |
| | ARM64 | ✅ Fully Supported | Tested on ARM servers |
| **macOS** | x86_64 | ✅ Fully Supported | Intel Mac |
| | ARM64 | ✅ Fully Supported | Apple Silicon (M1/M2/M3) |
| **Windows** | x86_64 | ✅ Fully Supported | Windows 10+ |

</div>

</details>

---

## Installation and Configuration

<div align="center" style="margin: 24px 0">

### 🚀 Quick Start

</div>

<details style="padding:16px; margin: 8px 0">
<summary style="cursor:pointer; font-weight:600; color:#1E293B">❓ How do I install it?</summary>

**For Rust Projects:**

Add to your `Cargo.toml`:

```toml
[dependencies]
confers = "0.4.0"
serde = { version = "1.0", features = ["derive"] }
```

Or use cargo:

```bash
cargo add confers serde --features serde/derive
```

**Optional Features:**

```toml
confers = { version = "0.4.0", features = ["watch", "remote", "cli"] }
```

**Verification:**

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

</details>

<details style="padding:16px; margin: 8px 0">
<summary style="cursor:pointer; font-weight:600; color:#1E293B">❓ How do I choose the right feature combination?</summary>

<div style="padding:16px">

**Feature Presets (Recommended):**

| Preset | Description | Use Case |
|:------:|:------------|:---------|
| <span style="color:#166534; padding:4px 8px">minimal</span> | Environment variables + JSON | Only need basic config loading |
| <span style="color:#1E40AF; padding:4px 8px">recommended</span> | TOML + JSON + Env + Validation | Most applications (recommended) |
| <span style="color:#92400E; padding:4px 8px">dev</span> | Development config (with watch, snapshot) | Development and debugging |
| <span style="color:#991B1B; padding:4px 8px">production</span> | Production config (with encryption) | Production environments |
| <span style="color:#5B21B6; padding:4px 8px">distributed</span> | Distributed systems config | Microservices and distributed systems |
| <span style="color:#166534; padding:4px 8px">full</span> | All features | Need complete functionality |

**Usage Examples:**

```toml
# Minimal usage
[dependencies]
confers = { version = "0.4.0", default-features = false, features = ["minimal"] }

# Recommended configuration
[dependencies]
confers = { version = "0.4.0", default-features = false, features = ["recommended"] }

# Production configuration
[dependencies]
confers = { version = "0.4.0", default-features = false, features = ["production"] }

# Distributed systems configuration
[dependencies]
confers = { version = "0.4.0", default-features = false, features = ["distributed"] }

# Full feature configuration
[dependencies]
confers = { version = "0.4.0", features = ["full"] }
```

</div>

> 💡 **Tip**: Default features are `toml`, `json`, and `env`. For validation functionality, use the `recommended` preset or explicitly enable the `validation` feature.

</details>

<details style="padding:16px; margin: 8px 0">
<summary style="cursor:pointer; font-weight:600; color:#1E293B">❓ How do dependency counts differ across feature combinations?</summary>

<div style="padding:16px">

| Feature Combination | Dependencies | Compile Time | Binary Size |
|:--------------------|:------------:|:------------:|:-----------:|
| `minimal` | ~15 | Shortest | Smallest |
| `recommended` | ~20 | Short | Small |
| `dev` | ~30 | Medium | Medium |
| `production` | ~35 | Medium | Medium |
| `cli` | ~25 | Medium | Small |
| `full` | ~50+ | Long | Large |

</div>

Choosing the right feature combination can significantly reduce compile time and binary size.

</details>

<details style="padding:16px; margin: 8px 0">
<summary style="cursor:pointer; font-weight:600; color:#1E293B">❓ What are the system requirements?</summary>

**Minimum Requirements:**

| Component | Requirement | Recommended |
|:----------|:-----------:|:-----------:|
| Rust Version | 1.75+ | Latest stable |
| Memory | Minimal | - |
| Disk Space | Minimal | - |

**Optional:**

- 🔧 `watch` feature requires OS-level file notification support
- ☁️ `remote` feature requires network access to configuration centers

</details>

---

## Usage and Features

<div align="center" style="margin: 24px 0">

### 💡 Using the API

</div>

<details style="padding:16px; margin: 8px 0">
<summary style="cursor:pointer; font-weight:600; color:#1E293B">❓ How do I get started with basic usage?</summary>

**5-Minute Quick Start:**

```rust
use confers::Config;
use serde::{Deserialize, Serialize};

// 1. Define configuration structure
#[derive(Config, Serialize, Deserialize, Debug)]
#[config(env_prefix = "APP_")]
struct AppConfig {
    host: String,
    port: u16,
    debug: bool,
}

fn main() -> anyhow::Result<()> {
    // 2. Load configuration from default sources
    let config = AppConfig::load_sync()?;

    println!("Host: {}, Port: {}", config.host, config.port);
    Ok(())
}
```

</details>

<details style="padding:16px; margin: 8px 0">
<summary style="cursor:pointer; font-weight:600; color:#1E293B">❓ What formats and sources are supported?</summary>

**Supported Formats:**

| ✅ Format | Description |
|:---------:|:------------|
| TOML | Preferred format |
| JSON | Universal format |
| YAML | Human-readable |
| INI | Simple format |

**Supported Sources:**

| ✅ Source | Description |
|:---------:|:------------|
| File | Auto-detects `config.{toml,json,yaml,ini}` |
| Environment Variables | Supports custom prefixes |
| CLI Arguments | Integrates with `clap` |
| Remote | Etcd, Consul, HTTP |
| Default Values | Specified in struct definition |
| Memory | Set programmatically |

**Supported Remote Configuration:**

| ✅ Remote Source | Description |
|:----------------:|:------------|
| Etcd | Distributed key-value store |
| Consul | Service discovery and configuration |
| HTTP | Fetch configuration via HTTP(S) |
| Redis | Cache backend (with `cache-redis` feature) |

</details>

<details style="padding:16px; margin: 8px 0">
<summary style="cursor:pointer; font-weight:600; color:#1E293B">❓ Can I validate configuration?</summary>

**Yes!** Confers integrates with the `garde` validation crate.

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

**Note:** Add `garde = { version = "0.22", features = ["derive"] }` to your dependencies.

</details>

---

## Performance

<div align="center" style="margin: 24px 0">

### ⚡ Speed and Optimization

</div>

<details style="padding:16px; margin: 8px 0">
<summary style="cursor:pointer; font-weight:600; color:#1E293B">❓ How fast is Confers?</summary>

**Benchmark Results (loading 100+ keys):**

| Source | Format | Latency (Average) |
|:-------|:-------|:-----------------:|
| Local File | TOML | ~0.5 ms |
| Environment Variables | - | ~0.1 ms |
| Remote (Etcd) | JSON | ~5-20 ms |

**Run benchmarks yourself:**

```bash
cargo bench
```

</details>

<details style="padding:16px; margin: 8px 0">
<summary style="cursor:pointer; font-weight:600; color:#1E293B">❓ What about memory usage?</summary>

**Typical Memory Usage:**

Confers uses very little memory, typically **less than 1MB** for standard application configurations. It uses `serde` for zero-copy deserialization whenever possible.

**Memory Safety:**

- ✅ No memory leaks (verified through continuous testing)
- ✅ Sensitive data can be zeroed after use
- ✅ Leverages Rust's ownership model for safety

</details>

---

## Security

<div align="center" style="margin: 24px 0">

### 🔒 Security Features

</div>

<details style="padding:16px; margin: 8px 0">
<summary style="cursor:pointer; font-weight:600; color:#1E293B">❓ Is Confers secure?</summary>

**Yes!** Security is a core focus of Confers.

<div style="padding:16px; margin: 16px 0">

**Security Features:**

| Implementation | Protection |
|:---------------|:-----------|
| ✅ Memory Safety (Rust) | ✅ Buffer overflow protection |
| ✅ Sensitive field masking | ✅ Side-channel attack resistance |
| ✅ Constant-time encryption | ✅ Memory zeroization (zeroize) |
| ✅ Secure path validation | ✅ Encryption at rest (v0.4.0+) |

</div>

</details>

<details style="padding:16px; margin: 8px 0">
<summary style="cursor:pointer; font-weight:600; color:#1E293B">❓ How do I report a security vulnerability?</summary>

**Please report security issues responsibly:**

1. **Do NOT** create a public GitHub issue
2. **Email:** Kirky-X@outlook.com
3. **Include:**
    - Description of the vulnerability
    - Steps to reproduce
    - Potential impact

**Response Timeline:**

- 📧 Initial response: 24 hours
- 🔍 Evaluation: 72 hours
- 📢 Public disclosure: After fix is released

</details>

---

## Troubleshooting

<div align="center" style="margin: 24px 0">

### 🔧 Common Issues

</div>

<details style="padding:16px; margin: 8px 0">
<summary style="cursor:pointer; font-weight:600; color:#991B1B">❓ I'm getting a "FileNotFound" error</summary>

**Problem:**

```
Error: Configuration file not found: config.toml
```

**Solution:**

1. Ensure the file is in the root directory or `config/` directory
2. Check the filename (supported: `config.toml`, `config.json`, `config.yaml`, `config.ini`)
3. If using a custom path, ensure the path is correct

</details>

<details style="padding:16px; margin: 8px 0">
<summary style="cursor:pointer; font-weight:600; color:#92400E">❓ I'm getting a "ValidationError"</summary>

**Problem:**

```
Error: Validation failed: ...
```

**Solution:**

1. Review the error message to see which field failed and why
2. Ensure your configuration file or environment variables match the expected format and constraints

</details>

---

## Contributing

<div align="center" style="margin: 24px 0">

### 🤝 Join the Community

</div>

<details style="padding:16px; margin: 8px 0">
<summary style="cursor:pointer; font-weight:600; color:#1E293B">❓ How can I contribute?</summary>

**Ways to Contribute:**

| Code Contributions | Non-Code Contributions |
|:-------------------|:-----------------------|
| 🐛 Fix bugs | 📖 Write tutorials |
| ✨ Add features | 🎨 Design assets |
| 📝 Improve documentation | 🌍 Translate documentation |
| ✅ Write tests | 💬 Answer questions |

**Getting Started:**

1. 🍴 Fork the repository
2. 🌱 Create a branch
3. ✏️ Make changes
4. ✅ Add tests
5. 📤 Submit PR

**Guide:** [CONTRIBUTING.md](CONTRIBUTING.md)

</details>

<details style="padding:16px; margin: 8px 0">
<summary style="cursor:pointer; font-weight:600; color:#1E293B">❓ Where can I get help?</summary>

**Support Channels:**

| Channel | Description | Response Time |
|:--------|:------------|:-------------:|
| 🐛 [GitHub Issues](https://github.com/Kirky-X/confers/issues) | Bug reports and feature requests | Critical bugs: 24 hours |
| 💬 [GitHub Discussions](https://github.com/Kirky-X/confers/discussions) | Q&A and ideas | 2-3 days |

</details>

---

## Licensing

<div align="center" style="margin: 24px 0">

### 📄 License Information

</div>

<details style="padding:16px; margin: 8px 0">
<summary style="cursor:pointer; font-weight:600; color:#1E293B">❓ What license is this?</summary>

**Dual Licensed:**

<table style="width:100%; border-collapse: collapse">
<tr>
<td width="50%" style="padding: 16px; text-align:center">

**MIT License**

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](../LICENSE-MIT)

**Permissions:**
- ✅ Commercial use
- ✅ Modification
- ✅ Distribution
- ✅ Private use

</td>
<td width="50%" style="padding: 16px; text-align:center">

**Apache License 2.0**

[![License: Apache 2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](../LICENSE-APACHE)

**Permissions:**
- ✅ Commercial use
- ✅ Modification
- ✅ Distribution
- ✅ Patent grant

</td>
</tr>
</table>

**You may use either license.**

</details>

---

<div align="center" style="margin: 32px 0; padding: 24px">

### 🎯 Still have questions?

| Create an Issue | Start a Discussion | Send Email |
|:---------------:|:------------------:|:----------:|
| [🐛 Report Issue](https://github.com/Kirky-X/confers/issues) | [💬 Community Discussion](https://github.com/Kirky-X/confers/discussions) | [📧 Contact Support](mailto:Kirky-X@outlook.com) |

---

**[📖 User Guide](USER_GUIDE.md)** • **[🔧 API Documentation](https://docs.rs/confers)** • **[🏠 Home](../README.md)**

Made with ❤️ by Kirky.X

**[⬆ Back to Top](#top)**

</div>
