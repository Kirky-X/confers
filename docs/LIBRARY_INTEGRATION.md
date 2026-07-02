# Library Integration Guide

This guide explains how to embed `confers` in another Rust project using the
public library API. The CLI binary (`confers` command) is shipped as a thin
wrapper around the same library API, so the patterns below also describe what
the CLI does internally.

> **Note:** `confers::cli` is gated behind the `cli` feature flag and the
> types it exposes (`Cli`, `Commands`) are **crate-private**. There is no
> public `ConfersCli` struct and no `confers::commands` module. To drive
> confers programmatically, use the library API documented below.

## Quick Start

### 1. Add Dependency

```toml
[dependencies]
confers = { version = "0.4.0", features = ["toml", "json", "env"] }
```

### 2. Basic Usage

```rust
use confers::{ConfigBuilder, ConfigConnector, ConfigReader};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct AppConfig {
    pub name: String,
    pub port: u16,
}

fn main() -> confers::BuildResult<()> {
    let config = ConfigBuilder::<AppConfig>::new()
        .file("config.toml")
        .env()
        .build()?;

    // Access the typed configuration
    println!("name = {}", config.name);
    println!("port = {}", config.port);
    Ok(())
}
```

## Loading Configuration

`ConfigBuilder` is the entry point for assembling a configuration from
multiple sources. Each source contributes a layer, and later sources
override earlier ones according to the merge strategy.

```rust
use confers::ConfigBuilder;

let value = ConfigBuilder::<serde_json::Value>::new()
    .file("base.toml")
    .file("override.toml")
    .env()
    .build_annotated()?; // Returns an AnnotatedValue with provenance
```

### Source Chain

For finer control over priority, use `SourceChainBuilder`:

```rust
use confers::{SourceChainBuilder, FileSource, EnvSource};

let chain = SourceChainBuilder::new()
    .add_source(FileSource::new("base.toml"))
    .add_source(EnvSource::new())
    .build();
```

## Validation

Enable the `validation` feature and derive `Validate` (from `garde`) on the
config struct:

```rust
use confers::Config;
use garde::Validate;

#[derive(Config, Validate)]
#[config(validate)]
struct ServerConfig {
    #[garde(email)]
    pub admin_email: String,
    #[garde(range(min = 1, max = 65535))]
    pub port: u16,
}
```

## Encryption

For sensitive fields, enable the `encryption` feature and use
`XChaCha20Crypto`:

```rust
use confers::XChaCha20Crypto;

let crypto = XChaCha20Crypto::new();
let ciphertext = crypto.encrypt(b"secret value", &key)?;
```

## Remote Sources

The `remote` feature provides HTTP-polled remote configuration sources:

```rust
use confers::remote::HttpPolledSourceBuilder;

let source = HttpPolledSourceBuilder::new()
    .url("https://config-server.example.com/app-config")
    .interval(std::time::Duration::from_secs(30))
    .build()?;
```

For etcd or Consul backends, enable the `etcd` or `consul` feature and use
`EtcdSourceBuilder` / `ConsulSourceBuilder` respectively.

## Feature Flags

The library is feature-gated. See `Cargo.toml` for the full list of feature
presets (`default`, `recommended`, `dev`, `production`, `full`). Common
features:

| Feature        | Description                              |
| -------------- | ---------------------------------------- |
| `toml`         | TOML format support                      |
| `json`         | JSON format support                      |
| `yaml`         | YAML format support                      |
| `env`          | Environment variable source              |
| `validation`   | Schema validation via `garde`           |
| `watch`        | File watching for hot reload            |
| `encryption`   | Field-level encryption (XChaCha20)      |
| `audit`        | Audit logging                            |
| `cli`          | CLI binary (does not expose a public API) |

## Error Handling

The library distinguishes **configuration phase** errors from **runtime**
errors:

- `ConfigConfigError` — initialization-time failures (missing fields, parse
  errors, validation failures).
- `ConfersError` — runtime failures (timeouts, remote unavailable, decryption
  failures).

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

## Migration from Earlier `ConfersCli` Snapshots

If you previously relied on snippets referencing `confers::ConfersCli`,
`confers::commands::key::KeySubcommand`, or
`confers::commands::validate::{ValidateCommand, ValidateLevel}`, please
migrate to the public API above. Those types were never part of the public
export surface and have been removed from the documentation; the CLI binary
internally uses `clap` and the types remain crate-private.

## Troubleshooting

### Feature Not Enabled

If a symbol is missing, verify the corresponding feature is enabled:

```toml
[dependencies]
confers = { version = "0.4.0", features = ["validation", "encryption"] }
```

### Encryption Key Issues

`derive_field_key` requires a 32-byte master key. Use `KeyManager` (under the
`key` feature) to manage key material securely:

```rust
use confers::key::KeyManager;
use std::path::PathBuf;

let mut km = KeyManager::new(PathBuf::from("./secure_keys"))?;
```

### Validation Failures

Inspect `ValidationResult` for the list of failing rules and the offending
field paths. Each `ValidationRule` reports the field path, the rule name, and
a human-readable message.
