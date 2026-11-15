# 🔌 Confers Library Integration Guide

This guide covers embedding `confers` into other Rust projects through the public library API. The CLI executable (the `confers` command) is only a thin wrapper over the same library API, so the patterns below equally describe how the CLI is implemented internally.

> **Note**: `confers::cli` is gated by the `cli` feature, and the types it exposes (`Cli`, `Commands`) are **crate-private**. There is no public `ConfersCli` struct and no `confers::commands` module. To drive confers programmatically, use the library APIs documented below.

## 📋 Table of Contents

- [Quick Start](#-quick-start)
- [Loading Config](#-loading-config)
- [Validation](#-validation)
- [Encryption](#-encryption)
- [Remote Sources](#️-remote-sources)
- [Feature Flags](#-feature-flags)
- [Error Handling](#-error-handling)
- [Migrating from Legacy ConfersCli Snapshots](#-migrating-from-legacy-conferscli-snapshots)
- [Troubleshooting](#-troubleshooting)

---

## 🚀 Quick Start

### 1. Add the dependency

```toml
[dependencies]
confers = { version = "0.6.0", features = ["toml", "json", "dotenv"] }
serde = { version = "1", features = ["derive"] }
```

### 2. Basic usage

```rust
use confers::ConfigBuilder;
use serde::Deserialize;

#[derive(Debug, Default, Deserialize)]
struct ConfersConfig {
    pub name: String,
    pub port: u16,
}

fn main() -> confers::ConfigResult<()> {
    let config = ConfigBuilder::<ConfersConfig>::new()
        .file("config.toml")
        .env()
        .build()?;

    // Access the typed configuration
    println!("name = {}", config.name);
    println!("port = {}", config.port);
    Ok(())
}
```

## 📥 Loading Config

`ConfigBuilder` is the entry point for assembling configuration from multiple sources. Each source contributes a config layer; later-added sources override earlier ones per the merge strategy.

```rust
use confers::ConfigBuilder;

let value = ConfigBuilder::<serde_json::Value>::new()
    .file("base.toml")
    .file("override.toml")
    .env()
    .build_annotated()?; // Returns an AnnotatedValue with provenance
```

### Source chains

For finer priority control, use `SourceChainBuilder` (its methods mirror `ConfigBuilder`: `file()`, `file_optional()`, `env()`, `env_with_prefix()`, `defaults()`, `memory()`, etc.):

```rust
use confers::SourceChainBuilder;

let chain = SourceChainBuilder::new()
    .file("base.toml")
    .env_with_prefix("MYAPP_")
    .build();
```

## ✅ Validation

Enable the `validation` feature, derive `Validate` (from `garde`) on the config struct, and generate the `confers_validate()` helper with `#[config(validate_helper)]`:

```rust
use confers::Config;
use garde::Validate;

#[derive(Config, Validate)]
#[config(validate_helper)]
struct ServerConfig {
    #[garde(email)]
    pub admin_email: String,
    #[garde(range(min = 1, max = 65535))]
    pub port: u16,
}
```

Validation never runs automatically: call `config.confers_validate()?` explicitly after `build()`; `#[config(validate)]` (without `_helper`) is a no-op kept for historical compatibility and generates no code.

## 🔐 Encryption

For sensitive fields, enable the `encryption` feature and use `XChaCha20Crypto`:

```rust
use confers::XChaCha20Crypto;

let crypto = XChaCha20Crypto::new();
let ciphertext = crypto.encrypt(b"secret value", &key)?;
```

## ☁️ Remote Sources

The `remote` feature provides HTTP-polling-based remote config sources:

```rust
use confers::remote::HttpPolledSourceBuilder;

let source = HttpPolledSourceBuilder::new()
    .url("https://config-server.example.com/app-config")
    .interval(std::time::Duration::from_secs(30))
    .build()?;
```

For etcd or Consul backends, enable the `etcd` or `consul` feature and use `EtcdSourceBuilder` (`build()` is async and establishes a gRPC connection internally) or `ConsulSourceBuilder` respectively.

## 🎨 Feature Flags

The library is feature-gated. Common features and the full feature matrix (with default-enabled state) are maintained in [README · Feature Flags](../../README.md#-feature-flags); the complete feature preset list (`default`, `minimal`, `recommended`, `dev`, `production`, `distributed`, `full`) is in the `[features]` definition of `Cargo.toml`.

## 🚨 Error Handling

The library distinguishes **configuration-phase** errors from **runtime** errors:

- `ConfigConfigError`: initialization-phase failures (missing fields, parse errors, validation failures).
- `ConfersError`: runtime failures (timeouts, remote unavailability, decryption failures).

```rust
use confers::{ConfigConfigError, ConfersError};

match result {
    Err(ConfigConfigError::MissingField { field }) => {
        eprintln!("Missing field: {}", field);
    }
    Err(ConfersError::Timeout { .. }) => {
        eprintln!("Operation timed out");
    }
    _ => {}
}
```

The complete variant list is in the [API Reference](API_REFERENCE.md#-error-types).

## 🔄 Migrating from Legacy ConfersCli Snapshots

If you previously relied on snippets referencing `confers::ConfersCli`, `confers::commands::key::KeySubcommand`, or `confers::commands::validate::{ValidateCommand, ValidateLevel}`, migrate to the public APIs above. These types never belonged to the public export surface and have been removed from the documentation; the CLI executable uses `clap` internally and the relevant types remain crate-private.

## 🔧 Troubleshooting

### Feature not enabled

If a symbol cannot be found, confirm the corresponding feature is enabled:

```toml
[dependencies]
confers = { version = "0.6.0", features = ["validation", "encryption"] }
```

### Encryption key issues

`derive_field_key` requires a 32-byte master key. Manage key material securely with `KeyManager` (under the `key-management` feature):

```rust
use confers::key::KeyManager;

let mut km = KeyManager::new()?;
```

### Validation failures

Inspect `ValidationResult` (a type alias of `Result<(), garde::Report>`): on failure, the `garde::Report` records error messages keyed by field path. Note that `ValidationRule` is only an enum of rule values (MinLength/MaxLength/Length/MinValue/Range/Email/Url/Ip/Custom, etc.) and carries no field paths or error messages itself.

---

## 📚 Related Documents

| Document | Description |
|:-----|:-----|
| [📘 API Reference](API_REFERENCE.md) | complete signatures of the APIs used in this guide |
| [🔒 Security](SECURITY.md) | security practices for encryption and key management |
| [🧩 Macro Guide](CONFIG_MACRO_GUIDE.md) | all attributes of `#[derive(Config)]` |
