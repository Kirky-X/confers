# ❓ Confers FAQ

This page collects Confers' frequently asked questions and answers, grouped by topic. No answer found? Feel free to ask on [GitHub Issues](https://github.com/Kirky-X/confers/issues).

## 📋 Table of Contents

- [General Questions](#-general-questions)
- [Installation and Configuration](#-installation-and-configuration)
- [Usage and Features](#️-usage-and-features)
- [Performance](#-performance)
- [Troubleshooting](#-troubleshooting)

---

## 🧭 General Questions

### ❓ What is Confers?

**Confers** is a modern, type-safe Rust configuration management library. It provides:

| ✨ Feature | Description |
|:-------:|:-----|
| **Zero boilerplate** | define config with just `#[derive(Config)]` |
| **Type safety** | config structures type-checked at compile time |
| **Multi-source support** | files, environment variables, and remote sources merged automatically |

It targets **Rust developers** who need a robust, production-grade configuration management solution.

**Learn more**: the [User Guide](USER_GUIDE.md)

### ❓ Why use Confers?

| Feature | Confers | Figment | Config-rs |
|:-----|:-------:|:-------:|:---------:|
| Type safety | ✅ **strong** | ✅ good | ⚠️ manual |
| Hot reload | ✅ **built-in** | ⚠️ manual | ⚠️ manual |
| Validation | ✅ **integrated** | ⚠️ manual | ⚠️ manual |
| Audit logging | ✅ **included** | ❌ none | ❌ none |

**Core advantages:**

- 🚀 **Zero boilerplate**: load complex config with minimal code
- 🔄 **Smart merging**: handles priority across multiple sources automatically
- 🛡️ **Secure**: built-in sensitive field encryption and redaction support
- 📊 **Observable**: detailed audit logs tracing every config value's provenance

### ❓ Is Confers production-ready?

**Current status**: ✅ **production-ready!**

**Ready capabilities:**

- ✅ Stable core loading logic
- ✅ Mainstream formats supported (TOML, JSON, YAML)
- ✅ Environment variable overrides
- ✅ Validation framework
- ✅ Remote sources (`remote`, `etcd`, `consul`, `etcd-watch`): public interfaces frozen, no breaking changes before 1.0 (consistent with the [README roadmap](../../README.md#️-roadmap))

**Maturity indicators:**

- 📊 Extensive test suite
- 🔄 Actively maintained
- 🛡️ Security-centered design
- 📖 Continuously improving documentation

> **Note**: always consult the [changelog](CHANGELOG.md) before upgrading.

### ❓ Which platforms are supported?

| Platform | Architecture | Status | Notes |
|:-----|:-----|:----:|:-----|
| **Linux** | x86_64 | ✅ fully supported | primary platform |
| | ARM64 | ✅ fully supported | tested on ARM servers |
| **macOS** | x86_64 | ✅ fully supported | Intel Macs |
| | ARM64 | ✅ fully supported | Apple Silicon (M1/M2/M3) |
| **Windows** | x86_64 | ✅ fully supported | Windows 10+ |

### ❓ How can I contribute?

**Ways to participate:**

| Code Contributions | Non-Code Contributions |
|:---------|:-----------|
| 🐛 Fix bugs | 📖 Write tutorials |
| ✨ Add features | 🎨 Design assets |
| 📝 Improve docs | 🌍 Translate documents |
| ✅ Write tests | 💬 Answer questions |

**Getting started:**

1. 🍴 Fork the repository
2. 🌱 Create a branch
3. ✏️ Modify code
4. ✅ Add tests
5. 📤 Submit a PR

**Guide**: the [Contributing Guide](CONTRIBUTING.md)

### ❓ Where can I get help?

**Support channels:**

| Channel | Description | Response Time |
|:-----|:-----|:--------:|
| 🐛 [GitHub Issues](https://github.com/Kirky-X/confers/issues) | bug reports and feature requests | critical bugs acknowledged within 48 hours |
| 💬 [GitHub Discussions](https://github.com/Kirky-X/confers/discussions) | Q&A and idea exchange | 2-3 days |

### ❓ What license does the project use?

This project is released under the [MIT license](../../LICENSE), an OSI-recognized permissive open-source license (consistent with [README · License](../../README.md#-license)).

**All the rights you get:**

- ✅ Use
- ✅ Modify
- ✅ Distribute
- ✅ Private use
- ✅ Commercial use and sale (just keep the copyright and license notice)

---

## 📦 Installation and Configuration

### ❓ How do I install it?

**Rust projects:**

Add to `Cargo.toml`:

```toml
[dependencies]
confers = "0.6.0"
serde = { version = "1.0", features = ["derive"] }
```

Or use cargo:

```bash
cargo add confers serde --features serde/derive
```

**Optional features:**

```toml
confers = { version = "0.6.0", features = ["watch", "remote", "cli"] }
```

**Installation verification:**

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

### ❓ How do I choose the right feature combination?

**Feature presets (recommended):**

The per-preset feature lists and use cases of the six presets `minimal` / `recommended` / `dev` / `production` / `distributed` / `full` are maintained in [README · Feature Presets](../../README.md#-feature-flags); the Cargo.toml snippets below apply directly.

**Usage examples:**

```toml
# Minimal usage
[dependencies]
confers = { version = "0.6.0", default-features = false, features = ["minimal"] }

# Recommended configuration
[dependencies]
confers = { version = "0.6.0", default-features = false, features = ["recommended"] }

# Production configuration
[dependencies]
confers = { version = "0.6.0", default-features = false, features = ["production"] }

# Distributed system configuration
[dependencies]
confers = { version = "0.6.0", default-features = false, features = ["distributed"] }

# All-features configuration
[dependencies]
confers = { version = "0.6.0", features = ["full"] }
```

> 💡 **Tip**: the default features are `toml`, `json`, and `dotenv`. For validation, use the `recommended` preset or enable the `validation` feature explicitly.

### ❓ How much do dependency counts differ across feature combinations?

| Feature Combination | Direct Dependencies | With Transitive (deduplicated) | Compile Time | Binary Size |
|:--------|:--------:|:--------:|:--------:|:----------:|
| `minimal` | 16 | 96 | shortest | smallest |
| `recommended` | 22 | 120 | short | small |
| `dev` | 33 | 168 | medium | medium |
| `production` | 37 | 178 | medium | medium |
| `cli` | 21 | 134 | medium | small |
| `full` | 46 | 253 | long | large |

> 📏 Dependency counts measured on the v0.6.0 workspace (`cargo tree -e normal --no-default-features --features <combination>`: direct dependencies are depth-1 deduplicated counts; with-transitive counts all nodes deduplicated by package name); **the "with transitive" column includes the confers root package itself** — excluding the root, `recommended` is 119 and `security-rules` alone is 104; counts drift slightly as dependency versions refresh.

Choosing the right feature combination can significantly reduce compile time and binary size.

### ❓ What are the system requirements?

**Minimum requirements:**

| Component | Requirement | Recommended |
|:-----|:----:|:--------:|
| Rust version | 1.97.1+ | latest stable |
| Memory | very low | - |
| Disk space | very low | - |

**Optional conditions:**

- 🔧 The `watch` feature requires OS-level file notification support
- ☁️ The `remote` feature requires network access to your config center

---

## 🛠️ Usage and Features

### ❓ How do I get started with basic usage quickly?

**5-minute quick start:**

```rust
use confers::Config;
use serde::{Deserialize, Serialize};

// 1. Define the config struct
#[derive(Config, Serialize, Deserialize, Debug)]
#[config(env_prefix = "APP_")]
struct ConfersConfig {
    host: String,
    port: u16,
    debug: bool,
}

fn main() -> anyhow::Result<()> {
    // 2. Load the configuration from default sources
    let config = ConfersConfig::load_sync()?;

    println!("Host: {}, Port: {}", config.host, config.port);
    Ok(())
}
```

### ❓ Which formats and sources are supported?

**Supported formats:**

| ✅ Format | Description |
|:-------:|:-----|
| TOML | the preferred format |
| JSON | the universal format |
| YAML | human-readable |
| INI | the simple format |

**Supported sources:**

| ✅ Source | Description |
|:-------:|:-----|
| Files | explicit path via `ConfigBuilder::file()` (TOML, JSON, YAML, INI supported); no auto-search |
| Environment variables | custom prefixes supported |
| CLI arguments | integrated with `clap` |
| Remote | Etcd, Consul, HTTP, Kubernetes, Nacos |
| Defaults | specified in the struct definition |
| Memory | set programmatically |

**Supported remote configs:**

| ✅ Remote Source | Description |
|:----------------:|:------------|
| Etcd | distributed key-value store (the `etcd` feature, with the `etcd-watch` native watch stream) |
| Consul | service discovery and config (the `consul` feature) |
| HTTP | config pulled via HTTP(S) polling (the `remote` feature) |
| Kubernetes | ConfigMap/Secret mounted-volume source and API source (the `k8s` feature) |
| Nacos | config center (the `nacos` feature) |

> 💡 In confers, Redis serves only as a message-bus backend for config change broadcast (the `redis-bus` feature), not as a config source.

### ❓ Can I validate config?

**Yes!** Confers integrates with the `garde` validation library.

```rust
use confers::{Config, ConfigBuilder};
use garde::Validate;
use serde::{Deserialize, Serialize};

#[derive(Config, Serialize, Deserialize, Validate, Debug)]
#[config(validate_helper)]
struct ConfersConfig {
    #[garde(length(min = 1))]
    host: String,

    #[garde(range(min = 1024, max = 65535))]
    port: u16,
}

let config = ConfigBuilder::<ConfersConfig>::new()
    .file("config.toml")
    .build()?;

// Validation does not run automatically: call it explicitly after building
config.confers_validate()?;
```

**Note**: add `garde = { version = "0.23", features = ["derive", "email"] }` to your dependencies and enable confers' `validation` feature. `#[config(validate_helper)]` generates `confers_validate()`, which must be invoked explicitly after `build()` — the load/build pipeline never validates automatically; `#[config(validate)]` (without `_helper`) is a no-op kept for historical compatibility and generates no code.

### ❓ Is Confers secure?

**Yes!** Security is a core concern of Confers.

**Security features:**

| Implementation | Protection |
|:-----|:---------|
| ✅ Memory safety (Rust) | ✅ buffer-overflow resistant |
| ✅ Sensitive field redaction | ✅ side-channel resistant |
| ✅ Constant-time encryption | ✅ memory zeroing (zeroize) |
| ✅ Secure path validation | ✅ encryption at rest (requires the `encryption` feature) |

### ❓ How do I report a security vulnerability?

**Please report security issues responsibly:**

1. **Do not** open a public GitHub issue
2. **Preferred channel**: private disclosure via [GitHub Security Advisories](https://github.com/Kirky-X/confers/security/policy)
3. **Alternative channel**: email Kirky-X@outlook.com
4. **Please include:**
    - Vulnerability description
    - Reproduction steps
    - Potential impact

**Response timeline:**

- 📧 First response: acknowledged within 48 hours
- 🔍 Initial assessment: within 7 days
- 📢 Public disclosure: after the fix ships

The full security policy is in [🔒 Security](SECURITY.md).

---

## ⚡ Performance

### ❓ How fast is Confers?

Benchmark data (loading, merging, change streams, zero-copy hot path, and other scopes) is maintained in the [Performance Guide · Performance Baselines](PERFORMANCE.md#-performance-baselines); a typical small config loads in about 0.7 µs on a development machine. To reproduce yourself:

```bash
cargo bench
```

### ❓ What about memory usage?

**Typical memory usage:**

Confers' memory footprint is very low; standard application configs are usually **under 1MB**. It uses `serde` for zero-copy deserialization wherever possible.

**Memory safety:**

- ✅ No memory leaks (verified by continuous testing)
- ✅ Sensitive data can be zeroed after use
- ✅ Fully leverages Rust's ownership model for safety

---

## 🔧 Troubleshooting

### ❓ "FileNotFound" error

**Problem:**

```text
Error: Configuration file not found: config.toml
```

**Solution:**

1. `confers` never searches any directories automatically — confirm that what you pass to `ConfigBuilder::file()` / `Config::load_file()` is the file's exact path
2. Confirm the file exists and its extension is a supported format (TOML, JSON, YAML, INI)
3. If the error comes from the derived macro's `load_sync()`: it reads only environment variables and field defaults, never files — when you need a file, switch to `ConfigBuilder::file()` or `load_file()` with an explicit path

### ❓ "ValidationError" error

**Problem:**

```text
Error: Validation failed: ...
```

**Solution:**

1. Read the error message to identify which field failed and why
2. Ensure the config file or environment variables match the expected format and constraints

---

## 🎯 Still have questions?

| Open an Issue | Start a Discussion | Send an Email |
|:---------------:|:------------------:|:----------:|
| [🐛 Report an issue](https://github.com/Kirky-X/confers/issues) | [💬 Community discussions](https://github.com/Kirky-X/confers/discussions) | [📧 Contact support](mailto:Kirky-X@outlook.com) |

**[📖 User Guide](USER_GUIDE.md)** • **[🔧 Online API Docs](https://docs.rs/confers)** • **[🏠 Back to Home](../../README.md)**
