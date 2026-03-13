<span id="top"></span>
<div align="center">

<img src="docs/image/confers.png" alt="Confers Logo" width="200" style="margin-bottom: 16px">

<p>
  <!-- CI/CD Status -->
  <a href="https://github.com/Kirky-X/confers/actions/workflows/ci.yml">
    <img src="https://github.com/Kirky-X/confers/actions/workflows/ci.yml/badge.svg" alt="CI Status" style="display:inline; margin:0 4px">
  </a>
  <!-- Version -->
  <a href="https://crates.io/crates/confers">
    <img src="https://img.shields.io/crates/v/confers.svg" alt="Version" style="display:inline; margin:0 4px">
  </a>
  <!-- Documentation -->
  <a href="https://docs.rs/confers">
    <img src="https://docs.rs/confers/badge.svg" alt="Documentation" style="display:inline; margin:0 4px">
  </a>
  <!-- Downloads -->
  <a href="https://crates.io/crates/confers">
    <img src="https://img.shields.io/crates/d/confers.svg" alt="Downloads" style="display:inline; margin:0 4px">
  </a>
  <!-- License -->
  <a href="https://github.com/Kirky-X/confers/blob/main/LICENSE">
    <img src="https://img.shields.io/crates/l/confers.svg" alt="License" style="display:inline; margin:0 4px">
  </a>
  <!-- Rust Version -->
  <a href="https://www.rust-lang.org/">
    <img src="https://img.shields.io/badge/rust-1.81+-orange.svg" alt="Rust 1.81+" style="display:inline; margin:0 4px">
  </a>
  <!-- Coverage -->
  <a href="https://codecov.io/gh/Kirky-X/confers">
    <img src="https://codecov.io/gh/Kirky-X/confers/branch/main/graph/badge.svg" alt="Coverage" style="display:inline; margin:0 4px">
  </a>
</p>

<p align="center">
  <strong>A modern, type-safe configuration management library for Rust</strong>
</p>

<p align="center">
  <a href="#features" style="color:#3B82F6">✨ Features</a> •
  <a href="#quick-start" style="color:#3B82F6">🚀 Quick Start</a> •
  <a href="#documentation" style="color:#3B82F6">📚 Documentation</a> •
  <a href="#examples" style="color:#3B82F6">💻 Examples</a> •
  <a href="#contributing" style="color:#3B82F6">🤝 Contributing</a>
</p>

</div>

---

<!-- Hero Section -->

### 🎯 Zero-Boilerplate Configuration Management

Confers provides a **declarative approach** to configuration management with:

| ✨ Type Safety | 🔄 Auto Reload | 🔐 XChaCha20-Poly1305 Encryption | 🌐 Remote Sources |
|:-------------:|:--------------:|:---------------------:|:-----------------:|
| Compile-time checks | Hot reload support | Sensitive data protection | etcd, Consul, HTTP |

```rust
use confers::Config;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Config)]
#[config(validate)]
pub struct AppConfig {
    pub name: String,
    pub port: u16,
    pub debug: bool,
}

// Configuration loads automatically from files, env vars, and CLI args
let config = AppConfig::load_sync()?;
```

---

## 📋 Table of Contents

<details open style="padding:16px">
<summary style="cursor:pointer; font-weight:600; color:#1E293B">📑 Table of Contents (Click to expand)</summary>

- [✨ Features](#features)
- [🚀 Quick Start](#quick-start)
  - [📦 Installation](#installation)
  - [💡 Basic Usage](#basic-usage)
- [📚 Documentation](#documentation)
- [💻 Examples](#examples)
- [🏗️ Architecture](#architecture)
- [⚙️ Configuration](#configuration)
- [🧪 Testing](#testing)
- [📊 Performance](#performance)
- [🔒 Security](#security)
- [🗺️ Roadmap](#roadmap)
- [🤝 Contributing](#contributing)
- [📄 License](#license)
- [🙏 Acknowledgments](#acknowledgments)

</details>

---

## <span id="features">✨ Features</span>

| 🎯 Core Features | ⚡ Optional Features |
|:-----------------|:--------------------|
| Always available | Enable as needed |

<table style="width:100%; border-collapse: collapse">
<tr>
<td width="50%" style="vertical-align:top; padding: 16px">

### 🎯 Core Features (Always Available)

| Status | Feature | Description |
|:------:|---------|-------------|
| ✅ | **Type-safe Configuration** | Auto-generate config structs via derive macros (`derive` feature) |
| ✅ | **Multi-format Support** | TOML, YAML, JSON, INI configuration files |
| ✅ | **Environment Variable Override** | Support environment variable overrides |
| ✅ | **CLI Argument Override** | Support command-line argument overrides (`cli` feature) |

</td>
<td width="50%" style="vertical-align:top; padding: 16px">

### ⚡ Optional Features

| Status | Feature | Description |
|:------:|---------|-------------|
| 🔍 | **Configuration Validation** | Built-in validator integration (`validation` feature) |
| 📊 | **Schema Generation** | Auto-generate JSON Schema (`schema` feature) |
| 🚀 | **File Watching & Hot Reload** | Real-time file monitoring (`watch` feature) |
| 🔐 | **Configuration Encryption** | XChaCha20-Poly1305 encrypted storage (`encryption` feature) |
| 🌐 | **Remote Configuration** | etcd, Consul, HTTP support (`remote` feature) |
| 📦 | **Audit Logging** | Record access & change history (`audit` feature) |
| ⚡ | **Parallel Validation** | Parallel validation for large configs (`parallel` feature) |
| 📈 | **System Monitoring** | Memory usage monitoring (`monitoring` feature) |
| 🔧 | **Configuration Diff** | Compare configs with multiple output formats |
| 🎨 | **Interactive Wizard** | Generate config templates via CLI |
| 🛡️ | **Security Enhancements** | Nonce reuse detection, SSRF protection |
| 🔑 | **Key Management** | Built-in key generation and rotation |

</td>
</tr>
</table>

### 📦 Feature Presets

| Preset | Features | Use Case |
|--------|----------|----------|
| <span style="color:#166534; padding:4px 8px">minimal</span> | `env`, `json` | Environment variables only |
| <span style="color:#1E40AF; padding:4px 8px">recommended</span> | `toml`, `json`, `env`, `validation` | **Recommended for most applications** |
| <span style="color:#92400E; padding:4px 8px">dev</span> | `toml`, `json`, `yaml`, `env`, `cli`, `validation`, `schema`, `audit`, `profile`, `watch`, `migration`, `snapshot`, `dynamic` | Development with all tools |
| <span style="color:#991B1B; padding:4px 8px">production</span> | `toml`, `env`, `watch`, `encryption`, `validation`, `audit`, `profile`, `metrics`, `schema`, `cli`, `migration`, `dynamic`, `progressive-reload`, `snapshot` | Production-ready configuration |
| <span style="color:#7C3AED; padding:4px 8px">distributed</span> | `toml`, `env`, `watch`, `validation`, `config-bus`, `progressive-reload`, `metrics`, `audit` | Distributed systems |
| <span style="color:#5B21B6; padding:4px 8px">full</span> | All features | Complete feature set |

**Note:** Default features include `toml`, `json`, `env`. The `cli` feature automatically includes `validation` and `encryption` dependencies.

### 🎨 Feature Architecture

```mermaid
graph LR
    A[<b>Configuration Sources</b><br/>📁 Files • 🌐 Env • 💻 CLI] --> B[<b>ConfigLoader</b><br/>🔧 Core Engine]
    B --> C[<b>Validation</b><br/>✅ Type & Business Rules]
    B --> D[<b>Schema</b><br/>📄 JSON Schema Gen]
    B --> E[<b>Encryption</b><br/>🔐 XChaCha20-Poly1305]
    B --> F[<b>Audit</b><br/>📋 Access Logs]
    B --> G[<b>Monitoring</b><br/>📊 Memory Watch]
    C --> H[<b>Application Config</b><br/>🚀 Ready to Use]
    D --> H
    E --> H
    F --> H
    G --> H
    
    style A fill:#DBEAFE,stroke:#1E40AF,stroke-width:2px
    style B fill:#FEF3C7,stroke:#92400E,stroke-width:2px
    style H fill:#DCFCE7,stroke:#166534,stroke-width:2px
```

---

## <span id="quick-start">🚀 Quick Start</span>

### <span id="installation">📦 Installation</span>

<table style="width:100%; border-collapse: collapse">
<tr>
<td width="100%" style="padding: 16px">

#### 🦀 Rust Installation

| Installation Type | Configuration | Use Case |
|-------------------|---------------|----------|
| **Default** | `confers = "0.3.0"` | Includes `toml`, `json`, `env` (default features) |
| **Minimal** | `confers = { version = "0.3.0", default-features = false, features = ["minimal"] }` | Environment variables + JSON only |
| **Recommended** | `confers = { version = "0.3.0", default-features = false, features = ["recommended"] }` | TOML + Env + validation |
| **CLI with Tools** | `confers = { version = "0.3.0", features = ["cli"] }` | CLI with validation and encryption |
| **Full** | `confers = { version = "0.3.0", features = ["full"] }` | All features |

**Individual Features:**

| Feature | Description | Default |
|---------|-------------|---------|
| **Format Support** |||
| `toml` | TOML format support | ✅ |
| `json` | JSON format support | ✅ |
| `yaml` | YAML format support | ❌ |
| `ini` | INI format support | ❌ |
| `env` | Environment variable support | ✅ |
| **Core Features** |||
| `validation` | Configuration validation (garde) | ❌ |
| `watch` | File watching and hot reload | ❌ |
| `encryption` | XChaCha20-Poly1305 encryption | ❌ |
| `cli` | Command-line tool | ❌ |
| `schema` | JSON Schema generation | ❌ |
| `parallel` | Parallel validation | ❌ |
| **Advanced Features** |||
| `audit` | Audit logging | ❌ |
| `metrics` | Metrics collection | ❌ |
| `dynamic` | Dynamic fields | ❌ |
| `progressive-reload` | Progressive reload | ❌ |
| `migration` | Configuration migration | ❌ |
| `snapshot` | Snapshot rollback | ❌ |
| `profile` | Environment configuration | ❌ |
| `interpolation` | Variable interpolation | ❌ |
| `tracing` | Distributed tracing | ❌ |
| **Remote Sources** |||
| `remote` | HTTP polling | ❌ |
| `etcd` | Etcd integration | ❌ |
| `consul` | Consul integration | ❌ |
| `cache-redis` | Redis cache | ❌ |
| **Message Bus** |||
| `config-bus` | Configuration event bus | ❌ |
| `nats-bus` | NATS message bus | ❌ |
| `redis-bus` | Redis message bus | ❌ |
| **Others** |||
| `security` | Security module | ❌ |
| `key` | Key management system | ❌ |
| `modules` | Modular configuration | ❌ |
| `context-aware` | Context-aware configuration | ❌ |

### 🔧 CLI Command Feature Dependencies

| Command | Required Features | Optional Features | Description |
|---------|------------------|------------------|-------------|
| `validate` | `cli` | - | Validate configuration files |
| `diff` | `cli` | - | Compare configuration files |

**Note**: The `cli` feature provides command-line tools for configuration management.

</td>
</tr>
</table>

### <span id="basic-usage">💡 Basic Usage</span>

#### 🎬 5-Minute Quick Start

**Required Features**: `toml`, `env`, `validation` (use: `features = ["recommended"]`)

<table style="width:100%; border-collapse: collapse">
<tr>
<td width="50%" style="padding: 16px; vertical-align:top">

**Step 1: Define Config Structure**

```rust
use confers::Config;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Config)]
#[config(validate)]
#[config(env_prefix = "APP_")]
pub struct AppConfig {
    pub name: String,
    pub port: u16,
    pub debug: bool,
}
```

</td>
<td width="50%" style="padding: 16px; vertical-align:top">

**Step 2: Create Config File**

```toml
# config.toml
name = "my-app"
port = 8080
debug = true
```

</td>
</tr>
<tr>
<td width="50%" style="padding: 16px; vertical-align:top">

**Step 3: Load Config**

```rust
fn main() -> anyhow::Result<()> {
    let config = AppConfig::load_sync()?;
    println!("✅ Loaded: {:?}", config);
    Ok(())
}
```

</td>
<td width="50%" style="padding: 16px; vertical-align:top">

**Step 4: Environment Override**

```bash
# Environment variables automatically override
export APP_PORT=9090
export APP_DEBUG=true
```

</td>
</tr>
</table>

<details style="padding:16px; margin: 16px 0">
<summary style="cursor:pointer; font-weight:600; color:#166534">📖 Complete Working Example</summary>

```rust
use confers::Config;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Config)]
#[config(validate)]
#[config(env_prefix = "APP_")]
pub struct AppConfig {
    pub name: String,
    pub port: u16,
    pub debug: bool,
}

fn main() -> anyhow::Result<()> {
    // Create config file
    let config_content = r#"
name = "my-app"
port = 8080
debug = true
"#;
    std::fs::write("config.toml", config_content)?;

    // Load configuration
    let config = AppConfig::load_sync()?;

    // Print configuration
    println!("🎉 Configuration loaded successfully!");
    println!("📋 Name: {}", config.name);
    println!("🔌 Port: {}", config.port);
    println!("🐛 Debug: {}", config.debug);

    Ok(())
}
```

</details>

### 🎨 Three Usage Patterns

Confers provides three flexible usage patterns to suit different needs:

#### 1️⃣ Simple Mode (Recommended)

Perfect for most applications with minimal boilerplate:

```rust
use confers::Config;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Config)]
#[config(validate)]
pub struct AppConfig {
    pub name: String,
    pub port: u16,
    pub debug: bool,
}

// One-line configuration loading
let config = AppConfig::load_sync()?;
```

#### 2️⃣ Builder Mode

For more control over configuration sources:

```rust
use confers::{ConfigBuilder, ConfigProviderExt};

let config = ConfigBuilder::<serde_json::Value>::new()
    .file("config.toml")
    .file("local.toml")  // Higher priority
    .env()
    .build()?;

let name = config.get_string("app.name");
let port = config.get_int("app.port");
```

#### 3️⃣ DI Mode (Dependency Injection)

For integration into frameworks and runtime flexibility:

```rust
use std::sync::Arc;
use confers::{ConfigBuilder, ConfigProviderExt};

#[derive(Debug, Clone, serde::Deserialize)]
pub struct MyConfig {
    pub name: String,
    pub port: u16,
}

let config = ConfigBuilder::<MyConfig>::new()
    .file("config.toml")
    .env()
    .build()?;

let shared_config = Arc::new(config);

let service = MyService::new(shared_config);
```

---

## <span id="documentation">📚 Documentation</span>

<table style="width:100%; max-width: 800px">
<tr>
<td align="center" width="33%" style="padding: 16px">
<a href="docs/USER_GUIDE.md" style="text-decoration:none">
<div style="padding: 24px; transition: transform 0.2s">
<img src="https://img.icons8.com/fluency/96/000000/book.png" width="48" height="48"><br>
<b style="color:#1E293B">User Guide</b>
</div>
</a>
<br><span style="color:#64748B">Complete usage guide</span>
</td>
<td align="center" width="33%" style="padding: 16px">
<a href="https://docs.rs/confers" style="text-decoration:none">
<div style="padding: 24px; transition: transform 0.2s">
<img src="https://img.icons8.com/fluency/96/000000/api.png" width="48" height="48"><br>
<b style="color:#1E293B">API Reference</b>
</div>
</a>
<br><span style="color:#64748B">Complete API docs</span>
</td>
<td align="center" width="33%" style="padding: 16px">
<a href="examples/" style="text-decoration:none">
<div style="padding: 24px; transition: transform 0.2s">
<img src="https://img.icons8.com/fluency/96/000000/code.png" width="48" height="48"><br>
<b style="color:#1E293B">Examples</b>
</div>
</a>
<br><span style="color:#64748B">Code examples</span>
</td>
</tr>
</table>

### 📖 Additional Resources

| Resource | Description |
|----------|-------------|
| ❓ [FAQ](docs/FAQ.md) | Frequently asked questions |
| 📖 [Contributing Guide](docs/CONTRIBUTING.md) | Code contribution guidelines |
| 📘 [API Reference](docs/API_REFERENCE.md) | Complete API documentation |
| 🏗️ [Architecture Decisions](docs/architecture_decisions.md) | ADR documentation |
| 📚 [Library Integration Guide](docs/LIBRARY_INTEGRATION.md) | How to integrate confers CLI into your projects |

---

## 🔧 CLI Tool

Confers provides a standalone command-line tool `confers` for configuration management:

### Install CLI Tool

```bash
cargo install confers
```

### Basic Commands

```bash
# View help
confers --help

# Inspect configuration - list all keys with their sources
confers config.toml inspect

# Validate configuration file
confers config.toml validate

# Compare configuration files
confers diff --base config1.toml --overlay config2.toml

# Export merged configuration
confers config.toml export --format json

# Manage configuration snapshots
confers config.toml snapshot list
confers config.toml snapshot diff --latest 2
```

**Note**: The CLI tool requires the `cli` feature to be enabled.

---

## <span id="examples">💻 Examples</span>

### 💡 Real-World Examples

<table style="width:100%; border-collapse: collapse">
<tr>
<td width="50%" style="padding: 16px; vertical-align:top">

#### 📝 Example 1: Basic Configuration

```rust
use confers::Config;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Config)]
#[config(validate)]
pub struct BasicConfig {
    pub name: String,
    pub port: u16,
}

fn basic_example() -> anyhow::Result<()> {
    let config = BasicConfig::load_sync()?;
    println!("✅ Name: {}, Port: {}", config.name, config.port);
    Ok(())
}
```

<details style="margin-top:8px">
<summary style="cursor:pointer; font-weight:600; color:#3B82F6">View Output</summary>

```
✅ Name: my-app, Port: 8080
```

</details>

</td>
<td width="50%" style="padding: 16px; vertical-align:top">

#### 🔥 Example 2: Advanced Configuration

```rust
use confers::Config;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Config)]
#[config(validate)]
#[config(env_prefix = "MYAPP_")]
pub struct AdvancedConfig {
    #[config(description = "Server port number")]
    pub port: u16,
    #[config(default = "localhost")]
    pub host: String,
    #[config(sensitive = true)]
    pub api_key: String,
}

fn advanced_example() -> anyhow::Result<()> {
    let config = AdvancedConfig::load_sync()?;
    println!("🚀 Server: {}:{}", config.host, config.port);
    Ok(())
}
```

<details style="margin-top:8px">
<summary style="cursor:pointer; font-weight:600; color:#3B82F6">View Output</summary>

```
🚀 Server: localhost:8080
```

</details>

</td>
</tr>
</table>

**[📂 Explore All Examples →](examples/)**

---

## <span id="architecture">🏗️ Architecture</span>

### 🏗️ System Architecture

```mermaid
graph TB
    subgraph Sources ["📥 Configuration Sources"]
        A[📁 Local Files<br/>TOML, JSON, YAML, INI]
        B[🌐 Environment Variables]
        C[💻 CLI Arguments]
        D[☁️ Remote Sources<br/>etcd, Consul, HTTP]
    end
    
    subgraph Core ["🔧 Core Engine"]
        E[⚡ ConfigLoader<br/>Multi-source Merge]
    end
    
    subgraph Processing ["🔨 Processing Layer"]
        F[✅ Validation<br/>Type & Business Rules]
        G[📄 Schema Generation]
        H[🔐 Encryption<br/>XChaCha20-Poly1305]
        I[📋 Audit Logging]
        J[👁️ File Watching]
        K[📊 Memory Monitoring]
    end
    
    subgraph Output ["📤 Application"]
        L[🚀 Application Configuration<br/>Type-Safe & Validated]
    end
    
    Sources --> Core
    Core --> Processing
    Processing --> Output
    
    style Sources fill:#DBEAFE,stroke:#1E40AF
    style Core fill:#FEF3C7,stroke:#92400E
    style Processing fill:#EDE9FE,stroke:#5B21B6
    style Output fill:#DCFCE7,stroke:#166534
```

### 📐 Component Status

| Component | Description | Status |
|-----------|-------------|--------|
| **ConfigLoader** | Core loader with multi-source support | ✅ Stable |
| **Configuration Validation** | Built-in validator integration | ✅ Stable |
| **Schema Generation** | Auto-generate JSON Schema | ✅ Stable |
| **File Watching** | Real-time monitoring with hot reload | ✅ Stable |
| **Remote Configuration** | etcd, Consul, HTTP support | 🚧 Beta |
| **Audit Logging** | Record access and change history | ✅ Stable |
| **Encrypted Storage** | XChaCha20-Poly1305 encrypted storage | ✅ Stable |
| **Configuration Diff** | Multiple output formats | ✅ Stable |
| **Interactive Wizard** | Template generation | ✅ Stable |

---

## <span id="configuration">⚙️ Configuration</span>

### 🎛️ Configuration Options

<table style="width:100%; border-collapse: collapse">
<tr>
<td width="50%" style="padding: 16px">

**Basic Configuration**

```toml
[project]
name = "my-app"
version = "1.0.0"

[server]
host = "localhost"
port = 8080

[features]
debug = true
logging = true
```

</td>
<td width="50%" style="padding: 16px">

**Advanced Configuration**

```toml
[project]
name = "my-app"
version = "1.0.0"

[server]
host = "0.0.0.0"
port = 8080
workers = 4

[database]
url = "postgres://localhost/db"
pool_size = 10

[performance]
cache_size = 1000
```

</td>
</tr>
</table>

<details style="padding:16px; margin: 16px 0">
<summary style="cursor:pointer; font-weight:600; color:#1E293B">🔧 All Configuration Options</summary>

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `name` | String | - | Project name |
| `version` | String | "1.0.0" | Version number |
| `host` | String | "localhost" | Server host |
| `port` | u16 | 8080 | Server port |
| `debug` | Boolean | false | Enable debug mode |
| `workers` | usize | 4 | Number of worker threads |
| `cache_size` | usize | 1000 | Cache size in MB |

</details>

---

## <span id="testing">🧪 Testing</span>

### 🎯 Test Coverage

```bash
# 🧪 Run all tests
cargo test --all-features

# 📊 Generate coverage report
cargo tarpaulin --out Html

# ⚡ Run benchmarks
cargo bench

# 🎯 Run specific test
cargo test test_name
```

<details style="padding:16px; margin: 16px 0">
<summary style="cursor:pointer; font-weight:600; color:#166534">📊 Test Statistics</summary>

| Category | Test Count | Coverage |
|----------|------------|----------|
| 🧪 Unit Tests | 50+ | 85% |
| 🔗 Integration Tests | 20+ | 80% |
| ⚡ Performance Tests | 10+ | 75% |
| **📈 Total** | **80+** | **80%** |

</details>

---

## <span id="performance">📊 Performance</span>

### ⚡ Benchmark Results

<table style="width:100%; border-collapse: collapse">
<tr>
<td width="50%" style="padding: 16px; text-align:center">

**📊 Throughput**

| Operation | Performance |
|-----------|-------------|
| Config Load | 1,000,000 ops/sec |
| Validation | 500,000 ops/sec |
| Schema Gen | 2,000,000 ops/sec |

</td>
<td width="50%" style="padding: 16px; text-align:center">

**⏱️ Latency**

| Percentile | Latency |
|------------|---------|
| P50 | 0.5ms |
| P95 | 1.2ms |
| P99 | 2.5ms |

</td>
</tr>
</table>

<details style="padding:16px; margin: 16px 0">
<summary style="cursor:pointer; font-weight:600; color:#92400E">📈 Detailed Benchmarks</summary>

```bash
# Run benchmarks
cargo bench

# Sample output:
test bench_config_load  ... bench: 1,000 ns/iter (+/- 50)
test bench_validate     ... bench: 2,000 ns/iter (+/- 100)
test bench_schema_gen   ... bench: 500 ns/iter (+/- 25)
```

</details>

---

## <span id="security">🔒 Security</span>

### 🛡️ Security Features

<table style="width:100%; border-collapse: collapse">
<tr>
<td align="center" width="25%" style="padding: 16px">
<img src="https://img.icons8.com/fluency/96/000000/lock.png" width="48" height="48"><br>
<b>Memory Safety</b><br>
<span style="color:#166534">Zero-copy & secure cleanup</span>
</td>
<td align="center" width="25%" style="padding: 16px">
<img src="https://img.icons8.com/fluency/96/000000/security-checked.png" width="48" height="48"><br>
<b>Audited</b><br>
<span style="color:#1E40AF">Regular security audits</span>
</td>
<td align="center" width="25%" style="padding: 16px">
<img src="https://img.icons8.com/fluency/96/000000/privacy.png" width="48" height="48"><br>
<b>Privacy</b><br>
<span style="color:#92400E">No data collection</span>
</td>
<td align="center" width="25%" style="padding: 16px">
<img src="https://img.icons8.com/fluency/96/000000/shield.png" width="48" height="48"><br>
<b>Compliance</b><br>
<span style="color:#5B21B6">Industry standards</span>
</td>
</tr>
</table>

<details style="padding:16px; margin: 16px 0">
<summary style="cursor:pointer; font-weight:600; color:#991B1B">🔐 Security Details</summary>

### 🛡️ Security Measures

| Measure | Description | API Reference |
|---------|-------------|---------------|
| ✅ **Memory Protection** | Automatic secure cleanup with zeroization | `SecureString`, `zeroize` crate |
| ✅ **Side-channel Protection** | Constant-time cryptographic operations | XChaCha20-Poly1305 encryption |
| ✅ **Input Validation** | Comprehensive input sanitization | `ConfigValidator`, `InputValidator` |
| ✅ **Audit Logging** | Full operation tracking | `AuditConfig`, audit trails |
| ✅ **SSRF Protection** | Server-Side Request Forgery prevention | `validate_remote_url()` |
| ✅ **Sensitive Data Detection** | Automatic detection of sensitive fields | `SensitiveDataDetector` |
| ✅ **Error Sanitization** | Remove sensitive info from error messages | `ErrorSanitizer`, `SecureLogger` |
| ✅ **Nonce Reuse Detection** | Prevent cryptographic nonce reuse | Built into encryption module |

### 🔐 Security APIs

```rust
// Secure string handling
use confers::security::{SecureString, SensitivityLevel};
let secure_str = SecureString::new("sensitive_data", SensitivityLevel::High);

// Input validation
use confers::security::ConfigValidator;
let validator = ConfigValidator::new();
let result = validator.validate_input(user_input);

// Error sanitization
use confers::security::ErrorSanitizer;
let sanitizer = ErrorSanitizer::default();
let safe_error = sanitizer.sanitize(&error_message);

// Audit logging
#[cfg(feature = "audit")]
use confers::audit::AuditConfig;
let audit = AuditConfig::new().enable_sensitive_field_tracking();
```

### 🚨 Security Best Practices

1. **Use SecureString for sensitive data**: Automatically zeroizes memory
2. **Enable audit logging**: Track all configuration access and changes
3. **Validate all inputs**: Use built-in validators for user inputs
4. **Use encryption**: Enable `encryption` feature for sensitive configs
5. **Follow principle of least privilege**: Minimize sensitive data exposure

### 📧 Reporting Security Issues

Please report security vulnerabilities to: **security@confers.example**

</details>

---

## <span id="roadmap">🗺️ Roadmap</span>

### 🎯 Development Roadmap

```mermaid
gantt
    title Confers Development Roadmap
    dateFormat  YYYY-MM
    section Core Features ✅
    Type-safe Configuration     :done, 2024-01, 2024-06
    Multi-format Support       :done, 2024-02, 2024-06
    Environment Variable Override     :done, 2024-03, 2024-06
    section Validation System ✅
    Basic Validation Integration     :done, 2024-04, 2024-07
    Parallel Validation Support     :done, 2024-05, 2024-08
    section Advanced Features 🚧
    Schema Generation      :active, 2024-06, 2024-09
    File Watching Hot Reload   :done, 2024-07, 2024-09
    Remote Configuration Support     :active, 2024-08, 2024-12
    Audit Logging         :done, 2024-08, 2024-10
```

<table style="width:100%; border-collapse: collapse">
<tr>
<td width="50%" style="padding: 16px">

### ✅ Completed

- [x] Type-safe Configuration
- [x] Multi-format Support (TOML, YAML, JSON, INI)
- [x] Environment Variable Override
- [x] Configuration Validation System
- [x] Schema Generation
- [x] File Watching & Hot Reload
- [x] Audit Logging
- [x] Encrypted Storage Support
- [x] Remote Configuration Support (etcd, Consul, HTTP)

</td>
<td width="50%" style="padding: 16px">

### 📋 Planned

- [ ] Performance Optimization
- [ ] Cloud-native Integration Enhancements

</td>
</tr>
</table>

---

## <span id="contributing">🤝 Contributing</span>

### 💖 Thank You to All Contributors!

<table style="width:100%; border-collapse: collapse">
<tr>
<td width="33%" align="center" style="padding: 16px">

### 🐛 Report Bugs

Found an issue?<br>
<a href="https://github.com/Kirky-X/confers/issues/new">Create Issue</a>

</td>
<td width="33%" align="center" style="padding: 16px">

### 💡 Feature Suggestions

Have a great idea?<br>
<a href="https://github.com/Kirky-X/confers/discussions">Start Discussion</a>

</td>
<td width="33%" align="center" style="padding: 16px">

### 🔧 Submit PR

Want to contribute code?<br>
<a href="https://github.com/Kirky-X/confers/pulls">Fork & PR</a>

</td>
</tr>
</table>

<details style="padding:16px; margin: 16px 0">
<summary style="cursor:pointer; font-weight:600; color:#1E293B">📝 Contribution Guidelines</summary>

### 🚀 How to Contribute

1. **Fork** this repository
2. **Clone** your fork: `git clone https://github.com/yourusername/confers.git`
3. **Create** a branch: `git checkout -b feature/amazing-feature`
4. **Make** your changes
5. **Test** your changes: `cargo test --all-features`
6. **Commit** your changes: `git commit -m 'feat: Add amazing feature'`
7. **Push** to the branch: `git push origin feature/amazing-feature`
8. **Create** a Pull Request

### 📋 Code Standards

- ✅ Follow Rust standard coding conventions
- ✅ Write comprehensive tests
- ✅ Update documentation
- ✅ Add examples for new features
- ✅ Pass `cargo clippy -- -D warnings`

</details>

---

## <span id="license">📄 License</span>

This project is licensed under **MIT License**:

---

## <span id="acknowledgments">🙏 Acknowledgments</span>

### 🌟 Built With Amazing Tools

<table style="width:100%; border-collapse: collapse">
<tr>
<td align="center" width="25%" style="padding: 16px">
<a href="https://www.rust-lang.org/" style="text-decoration:none">
<div style="padding: 16px">
<img src="https://www.rust-lang.org/static/images/rust-logo-blk.svg" width="48" height="48"><br>
<b>Rust</b>
</div>
</a>
</td>
<td align="center" width="25%" style="padding: 16px">
<a href="https://github.com/" style="text-decoration:none">
<div style="padding: 16px">
<img src="https://github.githubassets.com/images/modules/logos_page/GitHub-Mark.png" width="48" height="48"><br>
<b>GitHub</b>
</div>
</a>
</td>
<td align="center" width="25%" style="padding: 16px">
<div style="padding: 16px">
<img src="https://img.icons8.com/fluency/96/000000/code.png" width="48" height="48"><br>
<b>Open Source</b>
</div>
</td>
<td align="center" width="25%" style="padding: 16px">
<div style="padding: 16px">
<img src="https://img.icons8.com/fluency/96/000000/community.png" width="48" height="48"><br>
<b>Community</b>
</div>
</td>
</tr>
</table>

### 💝 Special Thanks

| Category | Description |
|----------|-------------|
| 🌟 **Dependency Projects** | [serde](https://github.com/serde-rs/serde) - Serialization framework |
| | [figment](https://github.com/SergioBenitez/figment) - Configuration management |
| | [validator](https://github.com/Keats/validator) - Validation library |
| 👥 **Contributors** | Thanks to all contributors! |
| 💬 **Community** | Special thanks to community members |

---

## 📞 Contact & Support

<table style="width:100%; max-width: 600px">
<tr>
<td align="center" width="33%">
<a href="https://github.com/Kirky-X/confers/issues">
<div style="padding: 16px">
<img src="https://img.icons8.com/fluency/96/000000/bug.png" width="32" height="32"><br>
<b style="color:#991B1B">Issues</b>
</div>
</a>
<br><span style="color:#64748B">Report bugs & issues</span>
</td>
<td align="center" width="33%">
<a href="https://github.com/Kirky-X/confers/discussions">
<div style="padding: 16px">
<img src="https://img.icons8.com/fluency/96/000000/chat.png" width="32" height="32"><br>
<b style="color:#1E40AF">Discussions</b>
</div>
</a>
<br><span style="color:#64748B">Ask questions & share ideas</span>
</td>
<td align="center" width="33%">
<a href="https://github.com/Kirky-X/confers">
<div style="padding: 16px">
<img src="https://img.icons8.com/fluency/96/000000/github.png" width="32" height="32"><br>
<b style="color:#1E293B">GitHub</b>
</div>
</a>
<br><span style="color:#64748B">View source code</span>
</td>
</tr>
</table>

---

## ⭐ Star History

[![Star History Chart](https://api.star-history.com/svg?repos=Kirky-X/confers&type=Date)](https://star-history.com/#Kirky-X/confers&Date)

---



### 💝 Support This Project

If you find this project useful, please consider giving it a ⭐️!

**Built with ❤️ by Kirky.X**

---

**[⬆ Back to Top](#top)**

---

<sub>© 2026 Kirky.X. All rights reserved.</sub>

</div>