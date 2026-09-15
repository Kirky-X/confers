<div align="center">

<img src="docs/assets/confers.png" alt="Confers Logo" width="180">

[![CI Status](https://github.com/Kirky-X/confers/actions/workflows/ci.yml/badge.svg)](https://github.com/Kirky-X/confers/actions/workflows/ci.yml) [![Version](https://img.shields.io/crates/v/confers.svg)](https://crates.io/crates/confers) [![Docs.rs](https://docs.rs/confers/badge.svg)](https://docs.rs/confers) [![Downloads](https://img.shields.io/crates/d/confers.svg)](https://crates.io/crates/confers) [![License](https://img.shields.io/crates/l/confers.svg)](LICENSE) [![Rust](https://img.shields.io/badge/rust-1.97.1%2B-orange.svg)](https://www.rust-lang.org/) [![Coverage](https://codecov.io/gh/Kirky-X/confers/branch/main/graph/badge.svg)](https://codecov.io/gh/Kirky-X/confers)

**[中文](README.md)** | English

**A production-ready Rust configuration library with zero boilerplate**

[✨ Features](#-features) • [🚀 Quick Start](#-quick-start) • [📚 Documentation](#-documentation) • [💻 Examples](#-examples) • [🤝 Contributing](#-contributing)

</div>

---

<div align="center">

### 🎯 Declare Once, Merge Everywhere

Tag a struct with `#[derive(Config)]`, merge sources by priority, and get strongly typed fields at compile time:

<table style="width:100%; border-collapse: collapse">
<tr>
<td align="center" width="25%">🧩<br><b>Derive-Macro Driven</b><br><span style="color:#64748B">Loading · Defaults · Env Overrides</span></td>
<td align="center" width="25%">🛡️<br><b>Type-Safe</b><br><span style="color:#64748B">Typed on merge · optional validation</span></td>
<td align="center" width="25%">🔄<br><b>Hot Reload</b><br><span style="color:#64748B">Progressive rollout · automatic rollback</span></td>
<td align="center" width="25%">🔐<br><b>End-to-End Encryption</b><br><span style="color:#64748B">Authenticated encryption · per-field subkeys</span></td>
</tr>
</table>

</div>

---

## 📋 Table of Contents

- [✨ Features](#-features)
- [🚀 Quick Start](#-quick-start)
- [🎨 Feature Flags](#-feature-flags)
- [📚 Documentation](#-documentation)
- [💻 Examples](#-examples)
- [🏗️ Architecture](#️-architecture)
- [🧪 Testing](#-testing)
- [📊 Performance](#-performance)
- [🔒 Security](#-security)
- [🗺️ Roadmap](#️-roadmap)
- [🤝 Contributing](#-contributing)
- [📋 Changelog](#-changelog)
- [📄 License](#-license)
- [🙏 Acknowledgments](#-acknowledgments)
- [📞 Contact & Support](#-contact--support)
- [⭐ Star History](#-star-history)

---

## ✨ Features

<table style="width:100%; border-collapse: collapse">
<tr>
<td width="50%" style="vertical-align:top; padding: 12px">🧩 <b>Derive-Macro Driven</b><br><span style="color:#64748B"><code>#[derive(Config)]</code> and <code>#[config(...)]</code> attributes generate loading, defaults, and env overrides at compile time</span></td>
<td width="50%" style="vertical-align:top; padding: 12px">🗂️ <b>Multi-Format Support</b><br><span style="color:#64748B">TOML, JSON, YAML, INI, and <code>.env</code> with content-based format detection</span></td>
</tr>
<tr>
<td width="50%" style="vertical-align:top; padding: 12px">🔗 <b>Multi-Source Priority Chain</b><br><span style="color:#64748B">Files, environment variables, memory, and remote sources merged in declaration order; every value carries source and location metadata</span></td>
<td width="50%" style="vertical-align:top; padding: 12px">🛡️ <b>Type Safety & Validation</b><br><span style="color:#64748B">Merged results deserialize into strongly typed structs with optional garde rules</span></td>
</tr>
<tr>
<td width="50%" style="vertical-align:top; padding: 12px">🔄 <b>Hot Reload</b><br><span style="color:#64748B">File watching with adaptive debouncing, progressive rollout, and health-check rollback</span></td>
<td width="50%" style="vertical-align:top; padding: 12px">⚡ <b>Dynamic Fields</b><br><span style="color:#64748B">Lock-free runtime updates built on arc-swap, with callbacks and field watchers</span></td>
</tr>
<tr>
<td width="50%" style="vertical-align:top; padding: 12px">🔐 <b>Config Encryption</b><br><span style="color:#64748B">XChaCha20-Poly1305 authenticated encryption with HKDF-SHA256 per-field key derivation</span></td>
<td width="50%" style="vertical-align:top; padding: 12px">🌐 <b>Remote Configuration</b><br><span style="color:#64748B">HTTP polling, etcd v3, Consul, Nacos, Kubernetes, with circuit breaker and SSRF protection</span></td>
</tr>
<tr>
<td width="50%" style="vertical-align:top; padding: 12px">📢 <b>Change Broadcast</b><br><span style="color:#64748B">NATS / Redis Pub-Sub message bus for multi-instance config sync</span></td>
<td width="50%" style="vertical-align:top; padding: 12px">📊 <b>Schema Generation</b><br><span style="color:#64748B">Automatic JSON Schema and TypeScript type definitions</span></td>
</tr>
<tr>
<td width="50%" style="vertical-align:top; padding: 12px">🧾 <b>Audit Logging</b><br><span style="color:#64748B">HMAC-signed integrity with automatic sensitive-field masking</span></td>
<td width="50%" style="vertical-align:top; padding: 12px">🧰 <b>CLI Diagnostics</b><br><span style="color:#64748B">inspect, validate, diff, export, snapshot, schema, doctor subcommands</span></td>
</tr>
</table>

Beyond the core capabilities above, config migration, snapshots and rollback, variable interpolation, modular configuration, context-aware values, runtime feature toggles, OpenFeature-style evaluation, security rule validators, key management and cloud KMS, lazy segmented parsing, and the unified change stream are all provided as independent feature flags as well; the complete feature matrix (mirroring the `[features]` section of `Cargo.toml`) lives in the [🎨 Feature Flags](#-feature-flags) section.

---

## 🚀 Quick Start

### 📦 Installation

```bash
cargo add confers
```

Requires Rust 1.97.1 or later (MSRV, matching the repository `rust-toolchain.toml`). Default features include `toml`, `json`, and `env`; the installation commands and feature lists for every preset (`minimal` / `recommended` / `dev` / `production` / `distributed` / `full`) live in the [🎨 Feature Flags](#-feature-flags) section below.

### 💡 Minimal Example

The following example is adapted from [`examples/src/examples/basic_usage.rs`](examples/src/examples/basic_usage.rs):

```rust
use confers::Config;
use serde::Deserialize;

#[derive(Config, Deserialize, Debug, Clone)]
pub struct AppConfig {
    /// Server bind address
    #[config(default = "127.0.0.1".to_string())]
    pub host: String,

    /// Server bind port
    #[config(default = 8080u16)]
    pub port: u16,

    /// Log level
    #[config(default = "info".to_string())]
    pub log_level: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load field defaults; the HOST, PORT, and LOG_LEVEL env vars override them
    let config = AppConfig::load_sync()?;

    println!("Listening on {}:{}", config.host, config.port);
    Ok(())
}
```

```bash
# Environment variables override defaults
export PORT=9000
cargo run    # Output: Listening on 127.0.0.1:9000
```

### 🧭 Core Concepts

- **Source chain**: declare `FileSource`, `EnvSource`, `MemorySource`, and remote sources via `ConfigBuilder` / `SourceChainBuilder`; declaration order is priority, later sources override earlier ones.
- **Annotated values**: every config value is wrapped in an `AnnotatedValue` carrying `SourceId` and `SourceLocation` (down to row and column), so conflicts are traceable.
- **Two-phase errors**: initialization failures return `ConfigConfigError`; runtime failures return `ConfersError`, keeping the two classes of problems separate.
- **Feature gating**: every optional capability is an independent feature; compiled artifacts only include what you enable, down to a minimal `env` + `json`.

---

## 🎨 Feature Flags

### 📦 Feature Presets

| Preset | Installation | Features | Use Case |
|------|----------|----------|----------|
| Default | `cargo add confers` | `toml`, `json`, `env` | Works out of the box |
| `minimal` | `cargo add confers --no-default-features --features minimal` | `env`, `json` | Minimal loading |
| `recommended` | `cargo add confers --no-default-features --features recommended` | `toml`, `env`, `validation`, `json`, `security-rules` | Most applications |
| `dev` | `cargo add confers --features dev` | `toml`, `json`, `yaml`, `env`, `cli`, `validation`, `schema`, `audit`, `watch`, `migration`, `snapshot`, `dynamic` | Full development toolset |
| `production` | `cargo add confers --features production` | `toml`, `env`, `watch`, `encryption`, `validation`, `audit`, `schema`, `cli`, `migration`, `dynamic`, `progressive-reload`, `snapshot`, `security-rules`, `feature-toggle` | Production environments |
| `distributed` | `cargo add confers --features distributed` | `toml`, `json`, `env`, `watch`, `validation`, `config-bus`, `progressive-reload`, `audit` | Distributed systems |
| `full` | `cargo add confers --features full` | All features | Complete capability set |

### 📋 Feature Matrix

The table below mirrors the `[features]` section of `Cargo.toml`, where `default = ["toml", "json", "env"]`.

<table style="width:100%; border-collapse: collapse">
<tr><th style="text-align:left">Feature</th><th style="text-align:center">Default</th><th style="text-align:left">Description</th></tr>
<tr><td colspan="3" style="background:#F8FAFC"><b>Format Support</b></td></tr>
<tr><td><code>toml</code></td><td align="center">✅</td><td>TOML configuration files</td></tr>
<tr><td><code>json</code></td><td align="center">✅</td><td>JSON configuration files</td></tr>
<tr><td><code>yaml</code></td><td align="center">❌</td><td>YAML configuration files</td></tr>
<tr><td><code>ini</code></td><td align="center">❌</td><td>INI configuration files</td></tr>
<tr><td><code>env</code></td><td align="center">✅</td><td>Environment variable loading and <code>.env</code> files</td></tr>
<tr><td><code>dotenv</code></td><td align="center">❌</td><td>Alias of <code>env</code></td></tr>
<tr><td colspan="3" style="background:#F8FAFC"><b>Core Capabilities</b></td></tr>
<tr><td><code>validation</code></td><td align="center">❌</td><td>Configuration validation built on garde</td></tr>
<tr><td><code>watch</code></td><td align="center">❌</td><td>File watching and hot reload with adaptive debouncing</td></tr>
<tr><td><code>encryption</code></td><td align="center">❌</td><td>XChaCha20-Poly1305 encryption with HKDF per-field key derivation</td></tr>
<tr><td><code>cli</code></td><td align="center">❌</td><td>The confers command-line diagnostics tool</td></tr>
<tr><td><code>schema</code></td><td align="center">❌</td><td>JSON Schema generation</td></tr>
<tr><td><code>typescript-schema</code></td><td align="center">❌</td><td>TypeScript type generation (alias of <code>schema</code>)</td></tr>
<tr><td><code>dynamic</code></td><td align="center">❌</td><td>Dynamic fields with lock-free arc-swap reads</td></tr>
<tr><td><code>progressive-reload</code></td><td align="center">❌</td><td>Progressive reload with canary rollout and health-check rollback (includes <code>watch</code>)</td></tr>
<tr><td><code>audit</code></td><td align="center">❌</td><td>Audit logging with HMAC integrity and sensitive-field masking</td></tr>
<tr><td><code>migration</code></td><td align="center">❌</td><td>Configuration version migration</td></tr>
<tr><td><code>snapshot</code></td><td align="center">❌</td><td>Snapshots and rollback</td></tr>
<tr><td><code>interpolation</code></td><td align="center">❌</td><td><code>${VAR}</code> interpolation with nested defaults</td></tr>
<tr><td><code>tracing</code></td><td align="center">❌</td><td>Enable the internal tracing facade (no-op when disabled)</td></tr>
<tr><td colspan="3" style="background:#F8FAFC"><b>Security</b></td></tr>
<tr><td><code>security</code></td><td align="center">❌</td><td>Security module: encryption integration, error sanitization, env validation (includes <code>encryption</code>)</td></tr>
<tr><td><code>security-rules</code></td><td align="center">❌</td><td>Built-in JWT, CORS, SSRF, TLS validators with a registry</td></tr>
<tr><td><code>key</code></td><td align="center">❌</td><td>Key lifecycle management and rotation (includes <code>encryption</code>)</td></tr>
<tr><td><code>keyring</code></td><td align="center">❌</td><td>Key storage backends (file, MasterKey, secret-tool)</td></tr>
<tr><td><code>cloud-kms</code></td><td align="center">❌</td><td>Cloud KMS key providers including Vault Transit (requires <code>remote</code>)</td></tr>
<tr><td colspan="3" style="background:#F8FAFC"><b>Remote Sources</b></td></tr>
<tr><td><code>remote</code></td><td align="center">❌</td><td>HTTP polling source with SSRF protection and circuit breaker</td></tr>
<tr><td><code>etcd</code></td><td align="center">❌</td><td>etcd v3 integration (includes <code>remote</code>)</td></tr>
<tr><td><code>etcd-watch</code></td><td align="center">❌</td><td>etcd watch subscription (includes <code>etcd</code>)</td></tr>
<tr><td><code>consul</code></td><td align="center">❌</td><td>HashiCorp Consul integration (includes <code>remote</code>)</td></tr>
<tr><td><code>nacos</code></td><td align="center">❌</td><td>Nacos configuration center integration (includes <code>remote</code>)</td></tr>
<tr><td><code>k8s</code></td><td align="center">❌</td><td>Kubernetes ConfigMap / Secret sources, both mounted volumes and API (includes <code>remote</code>)</td></tr>
<tr><td colspan="3" style="background:#F8FAFC"><b>Message Bus & Change Stream</b></td></tr>
<tr><td><code>config-bus</code></td><td align="center">❌</td><td>Config change event bus built on tokio broadcast</td></tr>
<tr><td><code>nats-bus</code></td><td align="center">❌</td><td>NATS bus backend (includes <code>config-bus</code>)</td></tr>
<tr><td><code>redis-bus</code></td><td align="center">❌</td><td>Redis Pub/Sub bus backend (includes <code>config-bus</code>)</td></tr>
<tr><td><code>change-stream</code></td><td align="center">❌</td><td>Unified change stream port reusing the config-bus transport (includes <code>watch</code>)</td></tr>
<tr><td colspan="3" style="background:#F8FAFC"><b>Organization & Extensions</b></td></tr>
<tr><td><code>modules</code></td><td align="center">❌</td><td>Modular config groups with a registry</td></tr>
<tr><td><code>context-aware</code></td><td align="center">❌</td><td>Context-aware (tenant-dimension) configuration</td></tr>
<tr><td><code>feature-toggle</code></td><td align="center">❌</td><td>Runtime feature toggle registry</td></tr>
<tr><td><code>openfeature</code></td><td align="center">❌</td><td>OpenFeature-style flag evaluation (includes <code>feature-toggle</code> and <code>context-aware</code>)</td></tr>
<tr><td><code>lazy</code></td><td align="center">❌</td><td>Lazy segmented parsing of oversized documents</td></tr>
</table>

---

## 📚 Documentation

| Document | Description |
|------|------|
| [📖 User Guide](docs/USER_GUIDE.md) | Complete tutorial from installation to advanced usage |
| [📘 API Reference](docs/API_REFERENCE.md) | Detailed description of all public APIs |
| [🏗️ Architecture](docs/ARCHITECTURE.md) | Design principles, module layout, and data flow |
| [🧭 Config Macro Guide](docs/CONFIG_MACRO_GUIDE.md) | Complete usage of the `Config` derive macro and `#[config(...)]` attributes |
| [📈 Performance Guide](docs/PERFORMANCE.md) | Benchmark data, measurement methodology, and optimization tips |
| [🔒 Security](docs/SECURITY.md) | Security design, best practices, and vulnerability handling records |
| [❓ FAQ](docs/FAQ.md) | Frequently asked questions |
| [📚 Library Integration Guide](docs/LIBRARY_INTEGRATION.md) | How to integrate the confers CLI into your projects |
| [🧪 Test Scenario Matrix](docs/TEST_SCENARIOS.md) | Exhaustive E2E acceptance scenario matrix |
| [📋 Changelog](docs/CHANGELOG.md) | Change records for every release |
| [🤝 Contributing Guide](docs/CONTRIBUTING.md) | How to participate in project development |
| [📦 Online API Docs](https://docs.rs/confers) | Latest documentation auto-generated on docs.rs |
| [📦 crates.io](https://crates.io/crates/confers) | Release page |

---

## 💻 Examples

All 21 runnable examples live in the [`examples/`](examples/) directory, each mapped to a `cargo run --bin` target and covering every feature domain: basic loading, hot reload, encryption and key rotation, validation, interpolation, audit, snapshots, migration, dynamic fields, remote sources (HTTP / etcd / Consul), bus, progressive reload, and schema generation. For the per-example file list, required services, and acceptance criteria, see the [test scenario doc · examples run list](docs/TEST_SCENARIOS.md).

```bash
# Run a single example (from the examples/ directory)
cd examples && cargo run --bin basic_usage
cd examples && cargo run --bin encryption

# Verify all examples compile
cd examples && ./verify_examples.sh
```

### 🤖 CLI Tool

Beyond the runnable examples, the bundled CLI diagnostics tool (`cli` feature, entry point `src/cli/main.rs`) troubleshoots configuration in real projects:

```bash
cargo install confers --features cli
```

It provides the `inspect`, `validate`, `export`, `diff`, `snapshot`, `schema`, `get`, `doctor`, and `docs --agent` subcommands, with the exit code contract: 0 success, 1 configuration error, 2 I/O error. For arguments, output, and usage examples of every command, see the [User Guide · CLI tool](docs/USER_GUIDE.md#-命令行工具); to integrate the CLI into your own projects, see the [Library Integration Guide](docs/LIBRARY_INTEGRATION.md).

---

## 🏗️ Architecture

Confers follows a facade-plus-implementation layered design: the public modules under `src/` only re-export while the real implementation lives in `src/impl_/`, and optional capabilities are gated behind independent features. The core data path runs from source chain registration, through loader format detection and parsing (errors pinned to row and column), `MergeEngine` deep merging along the source chain, and finally serde deserialization into strongly typed structs with optional garde validation and sensitive-field decryption; the derive macros are generated at compile time by the `confers-macros` proc-macro crate in the workspace.

For the architecture diagram, the public core and feature-gated module tables, the loading / hot reload / multi-instance change broadcast data flows, and the interface segregation (`ConfigReader` / `ConfigWriter` / `ConfigConnector` / `ConfigProvider` traits) plus security and performance design, see the [Architecture doc](docs/ARCHITECTURE.md).

---

## 🧪 Testing

### 🎯 Test Strategy

The test pyramid spans seven layers: inline unit tests in `src/`, integration tests organized by domain (`tests/core`, `tests/security`, `tests/remote`, `tests/watcher`, `tests/cli`), 24 E2E suites explicitly registered via `[[test]]` (`tests/e2e`), macro tests (`macros/tests`, trybuild compile-fail cases), fuzz testing (`fuzz/`, 3 cargo-fuzz targets), Criterion benchmarks (`benches/`, 9 groups), and doc tests on public APIs. For per-layer commands, the 357-row scenario matrix, and the E2E file mapping, see the [test scenario doc](docs/TEST_SCENARIOS.md).

### ▶️ Commands (matching CI)

```bash
# Full test run (CI matrix runs default / recommended / full)
cargo test --workspace --features full

# Lint and format gates
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo fmt --all -- --check

# Coverage gate: at least 80% line coverage
cargo llvm-cov --workspace --all-features --fail-under-lines 80

# Benchmarks
cargo bench --features dev --benches

# Fuzz testing (from the fuzz/ directory)
cargo fuzz run parser
```

Bus-related integration tests require a local NATS service; CI uses a `nats:2.10 -js` container (see `docker-compose.test.yml`).

### 📊 Test Scale

As of v0.6.0-rc.3: about 2100+ unit tests (inline in `src/`), 600 integration and E2E tests (`tests/`, 53 files), 3 fuzz targets, and 9 Criterion benchmark groups; the coverage gate requires at least 80% line coverage and is enforced by both CI and the pre-push hook. For the detailed counts, see the [test scenario doc · statistics](docs/TEST_SCENARIOS.md#6-统计汇总).

---

## 📊 Performance

Baselines were collected locally on a development machine (criterion median): loading 50 fields takes about 691 ns and 200 fields about 713 ns, shallow merging 1000 entries about 20.9 µs, a change-stream roundtrip with one subscriber about 1.98 µs, and zero-copy reads (`get_shared`) are about 2.1x faster than deep copies (`get_raw`) for large values. Real-world performance depends on config complexity and hardware; run `cargo bench` to reproduce. For the full benchmark tables and measurement methodology, see the [Performance Guide · baselines](docs/PERFORMANCE.md#-性能基线); for the design highlights (lock-free dynamic reads, `IndexMap` key order, `compact_str` interning, adaptive debouncing, feature-gate trimming), see the [Architecture doc · performance design](docs/ARCHITECTURE.md#-性能设计).

---

## 🔒 Security

### 🛡️ Security Design

Security design centers on protecting sensitive data across its full lifecycle: XChaCha20-Poly1305 authenticated encryption with HKDF-SHA256 per-field key derivation, zeroize-on-drop memory safety, key version and rotation governance, built-in JWT/CORS/SSRF/TLS validators, error sanitization with HMAC-signed audit logs, plus SSRF validation and a circuit breaker for remote sources. Mechanism-level details live in the [Architecture doc · Security Design](docs/ARCHITECTURE.md#-安全设计); security best practices and the vulnerability handling process are covered by the [Security doc](docs/SECURITY.md).

### ⛓️ Supply Chain and Gates

Supply chain gates — `cargo deny check`, `cargo audit`, the lefthook private-key scan, and the coverage gate — are enforced by both CI and local Git hooks; for the full checklist, see the [Security doc · Supply Chain and Gates](docs/SECURITY.md#供应链与门禁).

### 🚨 Reporting Security Issues

Please do not report security vulnerabilities through public issues. Use the private GitHub [Security Advisories](https://github.com/Kirky-X/confers/security/advisories/new) disclosure channel instead. The project commits to acknowledging reports within 48 hours and providing an initial assessment within 7 days. See the full policy in [SECURITY.md](docs/SECURITY.md).

---

## 🗺️ Roadmap

<table style="width:100%; border-collapse: collapse">
<tr><th style="text-align:center">Status</th><th style="text-align:left">Area</th><th style="text-align:left">Items</th></tr>
<tr><td align="center">✅</td><td>Core engine</td><td>Derive macros, multi-format support, source chain merging, env and CLI overrides</td></tr>
<tr><td align="center">✅</td><td>Validation and schema</td><td>garde validation, JSON Schema generation, TypeScript type generation</td></tr>
<tr><td align="center">✅</td><td>Hot updates</td><td>File watching hot reload, progressive rollout, dynamic fields, snapshot rollback, config migration, variable interpolation</td></tr>
<tr><td align="center">✅</td><td>Security and audit</td><td>XChaCha20-Poly1305 encryption, key management and rotation, audit logging, security rule validators</td></tr>
<tr><td align="center">✅</td><td>Remote and bus</td><td>HTTP polling, etcd, Consul, Nacos, Kubernetes ConfigMap / Secret, NATS and Redis buses</td></tr>
<tr><td align="center">🚧</td><td>Remote source maturity</td><td><code>remote</code>, <code>etcd</code>, and <code>consul</code> are in beta; interfaces may change</td></tr>
<tr><td align="center">📋</td><td>Performance</td><td>Benchmark suite refinement (criterion baselines), memory footprint optimization for large configs, zero-copy hot path for high-frequency reads</td></tr>
<tr><td align="center">📋</td><td>Cloud-native integration</td><td>Service mesh support, distributed tracing integration</td></tr>
</table>

---

## 🤝 Contributing

For the detailed contribution workflow and code standards, see the [🤝 Contributing Guide](docs/CONTRIBUTING.md).

### 🛠️ Development Environment

The toolchain is Rust 1.97.1 (pinned in `rust-toolchain.toml`); run `cargo fmt --all -- --check` and `cargo clippy --all-targets --all-features -- -D warnings` before committing; [lefthook](https://github.com/evilmartians/lefthook) Git hooks run rustfmt, clippy, `cargo deny check`, and private-key scanning on pre-commit, enforce Conventional Commits on commit-msg, and run `cargo audit` plus the coverage gate on pre-push. For the full environment setup, see the [Contributing Guide · environment prep](docs/CONTRIBUTING.md#-环境准备).

### 💖 Ways to Contribute

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

---

## 📋 Changelog

For the full version history, see the [📋 Changelog](docs/CHANGELOG.md) (following the [Keep a Changelog](https://keepachangelog.com/en/1.0.0/) format and semantic versioning).

| Version | Date | Highlights |
|------|------|------|
| 0.6.0-rc.3 | 2026-09-10 | Unified change stream port (`change-stream`); new Kubernetes / Nacos sources and native etcd watch; CLI gained `doctor`, `schema`, and `get` subcommands; zero-copy hot path, HMAC-chained audit logs, and lazy segmented parsing |
| 0.6.0-rc.2 | 2026-09-07 | Refreshed 12+ 0.x dependencies (async-nats 0.50, chacha20poly1305 0.11, garde 0.23, and more); solidified the test pyramid by adding and registering 7 E2E suites |
| 0.5.1 | 2026-08-06 | New `SecurityValidator` security rules and `FeatureToggleRegistry` runtime toggles; fixed SSRF whitelist bypass, TLS version comparison, and more |

---

## 📄 License

This project is licensed under the MIT License with the [Commons Clause](LICENSE) v1.0 condition: the right to sell the software is excluded unless separately authorized. See [LICENSE](LICENSE).

---

## 🙏 Acknowledgments

### 🌟 Core Dependencies

Confers stands on the shoulders of these excellent open source projects:

| Dependency | Purpose |
|------|------|
| [serde](https://github.com/serde-rs/serde) | Serialization and deserialization framework |
| [tokio](https://github.com/tokio-rs/tokio) | Async runtime |
| [garde](https://github.com/jprochazk/garde) | Configuration validation |
| [arc-swap](https://github.com/vorner/arc-swap) | Lock-free concurrent snapshots |
| [chacha20poly1305](https://github.com/RustCrypto/AEADs) | Authenticated encryption |
| [notify-debouncer-full](https://github.com/notify-rs/notify) | File watching with debouncing |
| [clap](https://github.com/clap-rs/clap) | CLI framework |
| [schemars](https://github.com/GREsau/schemars) | JSON Schema generation |
| [criterion](https://github.com/bheisler/criterion.rs) | Benchmarking |

### 💝 Special Thanks

Thanks to the Rust community and all [contributors](https://github.com/Kirky-X/confers/graphs/contributors).

---

## 📞 Contact & Support

<table style="width:100%; max-width: 600px">
<tr>
<td align="center" width="33%">
<a href="https://github.com/Kirky-X/confers/issues"><b style="color:#991B1B">Issues</b></a><br>
<span style="color:#64748B">Report bugs & issues</span>
</td>
<td align="center" width="33%">
<a href="https://github.com/Kirky-X/confers/discussions"><b style="color:#1E40AF">Discussions</b></a><br>
<span style="color:#64748B">Ask questions & share ideas</span>
</td>
<td align="center" width="33%">
<a href="https://github.com/Kirky-X/confers"><b style="color:#1E293B">GitHub</b></a><br>
<span style="color:#64748B">View source code</span>
</td>
</tr>
</table>

---

## ⭐ Star History

[![Star History Chart](https://api.star-history.com/svg?repos=Kirky-X/confers&type=Date)](https://star-history.com/#Kirky-X/confers&Date)

If you find this project useful, please consider giving it a ⭐️!

**Built by Kirky.X**

---

<sub>© 2026 Kirky.X. All rights reserved.</sub>
