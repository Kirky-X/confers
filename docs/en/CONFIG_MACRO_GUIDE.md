# 🧩 Confers Macro Guide

This document fully covers the usage of the `#[derive(Config)]` macro, including struct-level and field-level attributes, generated methods, and validation integration.

## 📋 Table of Contents

- [Overview](#-overview)
- [Struct-Level Attributes](#struct-level-attributes)
- [Struct-Level Attributes](#struct-level-attributes)
- [Field-Level Attributes](#field-level-attributes)
- [Field-Level Attributes](#field-level-attributes)
- [Comprehensive Example](#-comprehensive-example)
- [Generated Methods](#️-generated-methods)
- [Full Usage Examples](#-full-usage-examples)
- [Attribute Quick Reference](#-attribute-quick-reference)
- [Validation with Garde](#-validation-with-garde)
- [Feature Dependencies](#-feature-dependencies)
- [Best Practices](#-best-practices)
- [Troubleshooting](#-troubleshooting)

---

## 🎯 Overview

`#[derive(Config)]` is the core macro of the Confers library. It automatically generates full configuration management functionality for Rust structs. The macro lives in `macros/src/lib.rs` and implements code generation through the `codegen/` directory (load/schema/clap and other modules) and `parse.rs`.

---

## 🏗️ Struct-Level Attributes

### Enabling Validation

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Config, garde::Validate)]
#[config(validate_helper)]  // generates the confers_validate() helper (validation feature required)
pub struct ConfersConfig {
    pub name: String,
    pub port: u16,
}
```

**Effect**:
- Generates the explicit helper `confers_validate(&self)`: runs garde validation internally and flattens the report into a message string
- Validation never runs automatically: the load/build pipeline never calls it; the caller invokes it explicitly after `build()`
- `#[config(validate)]` (without `_helper`) is a no-op kept for historical compatibility and generates no code

---

### Environment Variable Prefix

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
#[config(env_prefix = "APP_")]  // reads APP_NAME, APP_PORT, etc.
pub struct ConfersConfig {
    pub name: String,
    pub port: u16,
}
```

**Effect**:
- Prefixes environment variable reads (the prefix is concatenated verbatim without auto-appending underscores: `"APP_"` → `APP_NAME`, while `"APP"` yields `APPNAME`)
- Example: `APP_NAME=myapp` maps to the `name` field

---

### Application Name

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
#[config(app_name = "myapp")]  // application identifier
pub struct ConfersConfig {
    pub name: String,
}
```

**Effect**:
- An optional application identifier; the macro only validates length and non-emptiness at expansion
- Serves as the application display name in the generated CLI helper code
- Does **not** participate in any config-file directory search; `confers` has no logic to auto-search `~/.config/`, `/etc/`, etc. — config files must be given explicitly via `ConfigBuilder::file()`

---

### Strict Mode and File Watching (retained attributes)

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
#[config(strict = true, watch = true)]
pub struct ConfersConfig {
    #[config(default = 8080)]
    pub port: u16,
}
```

**Current status**:
- `strict` and `watch` are struct-level attributes; the macro parses them, but the currently generated code never reads these switches — they **have no effect for now**
- The CLI argument parser's error tolerance is not controlled by `strict`; hot reload is not controlled by the struct-level `watch`
- For hot reload use `FsWatcher` / `MultiFsWatcher` directly (requires the `watch` feature; `build_with_watcher()` is deprecated and does not reload on file changes); for the full watch loop see [Full Usage Examples · Hot Reload](#hot-reload)
- For hot reload use `FsWatcher` / `MultiFsWatcher` directly (requires the `watch` feature; `build_with_watcher()` is deprecated and does not reload on file changes); for the full watch loop see [Full Usage Examples · Hot Reload](#hot-reload)

---

### Config Version

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
#[config(version = 2)]  // config version used for migrations
pub struct ConfersConfig {
    pub name: String,
}
```

**Effect**:
- Works together with config migration
- Version tracking supporting schema evolution

---

### Batch Renaming (rename_all)

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
#[config(rename_all = "camelCase")]  // config file keys use camelCase
pub struct ConfersConfig {
    pub database_url: String,  // config key is databaseUrl
}
```

**Effect**:
- codegen maps external keys back to serde field names before deserialization (explicit serde names take priority)
- Illegal styles error at macro expansion; `camelCase`, `snake_case`, and `kebab-case` are supported

---

### Profile Overlay

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
#[config(profile, profile_env = "APP_ENV")]  // APP_ENV=production activates the production override
pub struct ConfersConfig {
    pub log_level: String,
}
```

**Effect**:
- Activates the corresponding profile overlay per the environment variable named by `profile_env`
- Works independently, unrelated to the `modules` feature: when `APP_ENV` is non-empty, `config.production.toml` (`<stem>.<env>.<ext>`) is looked up in the base file's directory and loaded as an overlay; a missing overlay file is silently skipped

---

## 🏷️ Field-Level Attributes

### Defaults

**Style 1: new syntax (recommended)**
```rust
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
pub struct ConfersConfig {
    #[config(default = "default_value")]
    pub name: String,

    #[config(default = 8080)]
    pub port: u32,

    #[config(default = 3.14)]
    pub rate: f64,

    #[config(default = true)]
    pub debug: bool,
}
```

**Style 2: old syntax for strings**
```rust
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
pub struct ConfersConfig {
    #[config(default = "\"default_value\".to_string()")]
    pub name: String,
}
```

**Effect**:
- The default is used when the field is missing from config files
- Implements the `Default` trait automatically

---

### Field Descriptions

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
pub struct ConfersConfig {
    #[config(description = "Server port number")]
    pub port: u16,

    #[config(description = "Database connection URL")]
    pub database_url: String,
}
```

**Effect**:
- Generates CLI help text
- Used for JSON Schema generation

---

### Config Key Name Mapping

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
pub struct ConfersConfig {
    #[config(name = "app_name")]  // use app_name in the config file
    pub name: String,
}
```

**Effect**:
- The field is named `name`, but the config key is `app_name`

---

### Environment Variable Name Mapping

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
#[config(env_prefix = "APP_")]
pub struct ConfersConfig {
    #[config(name_env = "CUSTOM_PORT")]  // reads CUSTOM_PORT (used as-is, no prefix stacking)
    pub port: u16,
}
```

**Priority**: `name_env` > auto-derivation (once `name_env` is declared, the field reads only the declared environment variable name — no `env_prefix` stacking and no fallback to the default derived naming)

---

### CLI Argument Names

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
pub struct ConfersConfig {
    #[config(name_clap_long = "server-port")]
    pub port: u16,

    #[config(name_clap_short = 'p')]
    pub port2: u16,
}
```

**Effect**:
- CLI arguments: `--server-port` or `-p`

---

### Validation Rules

Confers uses the `garde` validation library: derive `garde::Validate`, declare rules with `#[garde(...)]` attributes (range, length, email, URL, pattern, custom functions, etc.), and generate the `confers_validate()` helper with `#[config(validate_helper)]`, invoked explicitly by the caller after `build()` (the build/load pipeline never validates automatically). The syntax of each rule type is covered uniformly in the [✅ Validation with Garde](#-validation-with-garde) section.

---

### Sensitive Fields

```rust
use confers::secret::SecretString;

#[derive(Debug, Clone, Serialize, Deserialize, Config)]
pub struct ConfersConfig {
    // sensitive fields must use the SecretString / SecretBytes types (encryption feature required);
    // otherwise macro expansion fails
    #[config(sensitive = true)]
    pub password: SecretString,

    #[config(sensitive = true)]
    pub api_key: SecretString,
}
```

**Effect**:
- Automatically redacted in audit logs
- Sensitive information is never output in plaintext

---

### Flattened Fields

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
pub struct DatabaseConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize, Config)]
pub struct ConfersConfig {
    #[config(flatten)]
    pub database: DatabaseConfig,

    pub app_name: String,
}
```

**Effect**:
- Nested-structure fields can be written at the top level: a top-level `host` is relocated to `database.host` before loading
- The `database.host` dotted nesting syntax (serde-native) is also supported; when both appear, the explicit nested value wins

**Integration with serde**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NestedConfig {
    #[serde(flatten)]
    pub inner: InnerConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Config)]
pub struct InnerConfig {
    pub value: String,
}
```

---

### Skipped Fields

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
pub struct ConfersConfig {
    pub name: String,

    #[config(skip)]
    pub temp_field: String,  // not loaded from configuration
}
```

**Effect**:
- The field is not loaded from config files
- Uses the struct's default value

---

### Encrypted Fields

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
pub struct ConfersConfig {
    #[config(encrypt = "xchacha20")]
    pub database_password: String,

    #[config(encrypt = "xchacha20")]
    pub api_key: String,
}
```

**Effect**:
- Field values are decrypted automatically at load
- Requires the `encryption` feature
- Uses the XChaCha20-Poly1305 encryption algorithm

---

### Variable Interpolation

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
pub struct ConfersConfig {
    #[config(interpolate)]
    pub database_url: String,  // supports ${VAR} syntax
}
```

**Effect**:
- Enables variable interpolation for the field
- Supports `${VAR}` and `${VAR:default}` syntax (shell-style `${VAR:-default}` also accepted)
- Requires the `interpolation` feature

---

### Merge Strategies

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
pub struct ConfersConfig {
    #[config(merge_strategy = "append")]
    pub hosts: Vec<String>,
}
```

**Available strategies**:
- `replace`: replace the existing value (default)
- `append`: append to the array
- `prepend`: prepend to the array
- `join`: join array values

> Note: Maps deep-merge unconditionally at the engine level, so a `deep_merge`
> strategy value is neither needed nor accepted any more; `join_append` is an
> alternative spelling. The original `deep_merge` behaved exactly like `replace`, removed in 0.6.0.

---

### Dynamic Fields

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
pub struct ConfersConfig {
    #[config(dynamic)]
    pub feature_flags: HashMap<String, bool>,
}
```

**Effect**:
- Generates a `DynamicField` handle for runtime updates
- Requires the `dynamic` feature
- Supports hot-updatable config partitions

---

### Module Groups

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
pub struct ConfersConfig {
    #[config(module_group = "database")]
    pub db_host: String,

    #[config(module_group = "database")]
    pub db_port: u16,
}
```

**Effect**:
- Groups related fields for modular configuration
- Supports module-level reload and validation
- Requires the `modules` feature

---

## 📦 Comprehensive Example

```rust
use confers::Config;
use confers::secret::SecretString;
use garde::Validate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Config, Validate)]
#[config(
    validate_helper,                             // generates the confers_validate() manual validation helper
    env_prefix = "APP_",                         // environment variable prefix
    app_name = "myapp",                         // application name
    version = 1,                                 // config version
)]
pub struct ConfersConfig {
    // ============ Basic types ============
    #[config(description = "Application name")]
    pub name: String,

    #[config(default = 8080, description = "Server port")]
    pub port: u16,

    #[config(default = false, description = "Debug mode")]
    pub debug: bool,

    // ============ String types ============
    #[config(default = "\"localhost\".to_string()", description = "Server host")]
    pub host: String,

    // ============ Validation rules (with garde) ============
    #[garde(range(min = 1, max = 65535))]
    #[config(description = "Admin port")]
    pub admin_port: u16,

    #[garde(length(min = 3, max = 100))]
    #[config(description = "Username")]
    pub username: String,

    #[garde(email)]
    #[config(description = "Email address")]
    pub email: String,

    #[garde(url)]
    #[config(description = "Website URL")]
    pub website: String,

    // ============ Sensitive fields (must be SecretString / SecretBytes; encryption feature required) ============
    #[config(sensitive = true, description = "Database password")]
    pub db_password: SecretString,

    #[config(sensitive = true, description = "API key")]
    pub api_key: SecretString,

    // ============ Encrypted fields ============
    #[config(encrypt = "xchacha20", description = "Secret token")]
    pub secret_token: String,

    // ============ Variable interpolation ============
    #[config(interpolate, description = "Database URL")]
    pub database_url: String,

    // ============ Custom mapping ============
    #[config(name_env = "CUSTOM_DATABASE_URL", description = "Custom database URL")]
    pub custom_db_url: String,

    // ============ Nested config ============
    #[config(flatten, description = "Database configuration")]
    pub database: DatabaseConfig,

    // ============ Skipped fields ============
    #[config(skip)]
    pub runtime_data: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DatabaseConfig {
    pub host: String,
    pub port: u16,
    pub name: String,
}
```

---

## ⚙️ Generated Methods

After using the `#[derive(Config)]` macro, the struct automatically gains the following methods:

### Config Builder

```rust
use confers::ConfigBuilder;

// Basic loading with ConfigBuilder
let config = ConfigBuilder::<ConfersConfig>::new()
    .file("config.toml")
    .env()
    .build()?;

// With an environment variable prefix
let config = ConfigBuilder::<ConfersConfig>::new()
    .file("config.toml")
    .env_prefix("APP_")
    .build()?;
```

Hot reload (requires the `watch` feature) does not go through `ConfigBuilder` chained methods; use `FsWatcher` directly instead; for the full watch loop see [Full Usage Examples · Hot Reload](#hot-reload).
Hot reload (requires the `watch` feature) does not go through `ConfigBuilder` chained methods; use `FsWatcher` directly instead; for the full watch loop see [Full Usage Examples · Hot Reload](#hot-reload).

### Helper Function

```rust
// The convenient config() function
let config = confers::config::<ConfersConfig>()
    .file("config.toml")
    .env()
    .build()?;
```

### Schema Generation

```rust
// Generate a JSON Schema (requires the schema feature)
// Note: derive ConfigSchema as well
let schema = ConfersConfig::json_schema();

// Generate TypeScript types (requires the typescript-schema feature)
// The derive macro's built-in typescript_type() is currently a stub (returns only the interface name, no fields);
// for real TS generation use TypeScriptGenerator:
use confers::schema::TypeScriptGenerator;
let ts_type = TypeScriptGenerator::generate::<ConfersConfig>()?;
```

### Other Methods

```rust
// Get default values
let default = ConfersConfig::default();

// Access config values
let value = config.some_field;
```

---

## 💡 Full Usage Examples

### Basic Usage

**Define the config struct**
```rust
use confers::Config;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Config)]
#[config(validate_helper)]
#[config(env_prefix = "APP_")]
pub struct ServerConfig {
    pub host: String,

    #[config(default = 8080)]
    pub port: u16,

    #[config(default = true)]
    pub enabled: bool,
}
```

**Create the config file `config.toml`**
```toml
host = "0.0.0.0"
port = 9000
enabled = false
```

**Use the config**
```rust
use confers::ConfigBuilder;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ConfigBuilder::<ServerConfig>::new()
        .file("config.toml")
        .env_prefix("APP_")
        .build()?;

    println!("Host: {}", config.host);
    println!("Port: {}", config.port);
    println!("Enabled: {}", config.enabled);

    Ok(())
}
```

**Environment variable overrides**
```bash
export APP_PORT=3000
export APP_ENABLED=true
cargo run
```

### Sensitive Config Encryption

```rust
use confers::Config;
use confers::secret::SecretString;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Config)]
pub struct SecureConfig {
    // sensitive fields must use the SecretString / SecretBytes types (encryption feature required)
    #[config(sensitive = true)]
    pub password: SecretString,

    #[config(encrypt = "xchacha20")]
    pub api_secret: String,
}
```

**Encryption uses the XChaCha20-Poly1305 algorithm. The nonce is stored alongside the ciphertext.**

### Hot Reload

```rust
use confers::ConfigBuilder;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[derive(Debug, Clone, Serialize, Deserialize, Config)]
    #[config(watch = true)]
    pub struct HotReloadConfig {
        #[config(default = 8080)]
        pub port: u16,
    }

    let config = ConfigBuilder::<HotReloadConfig>::new()
        .file("config.toml")
        .build()?;

    println!("Initial port: {}", config.port);

    // Watch for file changes and rebuild the configuration
    let mut watcher = confers::watcher::FsWatcher::new("config.toml", 200).await?;
    while let Some(changed_path) = watcher.recv().await {
        println!("Config file changed: {:?}", changed_path);
        let new_config = ConfigBuilder::<HotReloadConfig>::new()
            .file("config.toml")
            .build()?;
        // Apply new_config to your application state
    }

    Ok(())
}
```

---

## 📑 Attribute Quick Reference

### Struct-Level Attributes

| Attribute | Purpose |
|------|------|
| `validate` | ⚠️ historical no-op generating no code (kept for compatibility so old code still compiles); use `validate_helper` for validation |
| `validate_helper` | generates the manual validation helper `confers_validate()` (requires deriving garde::Validate and the `validation` feature; the load/build pipeline never calls it automatically) |
| `env_prefix` | environment variable prefix |
| `app_name` | application name (validation + CLI display name, no directory search) |
| `strict` | ⚠️ currently no effect (parsed but not read by codegen) |
| `watch` | ⚠️ the struct-level form currently has no effect; the field-level `watch` generates a field watcher that works normally |
| `version` | config version number for migration |
| `rename_all` | batch rename config keys (`camelCase` / `snake_case` / `kebab-case`) |
| `profile` | enable the profile overlay |
| `profile_env` | names the environment variable driving profile switching (e.g. `APP_ENV`) |

### Field-Level Attributes

| Attribute | Purpose |
|------|------|
| `default` | default value expression |
| `description` | field description for documentation |
| `name` | override the config key name |
| `name_env` | override the environment variable name (used verbatim, no `env_prefix` stacking) |
| `name_clap_long` | CLI long argument name |
| `name_clap_short` | CLI short argument character |
| `sensitive` | mark as a sensitive field (hidden in logs); the field type must be `SecretString` / `SecretBytes` (requires the `encryption` feature) |
| `encrypt` | encryption algorithm (e.g. "xchacha20") |
| `flatten` | flatten nested config |
| `skip` | skip the field at load time |
| `interpolate` | enable variable interpolation |
| `merge_strategy` | multi-source merge strategy |
| `dynamic` | generate a DynamicField handle |
| `watch` | field-level hot-reload subscriber (`field_watcher`) |
| `module_group` | group for modular configuration |

---

## ✅ Validation with Garde

Validation is handled by the `garde` crate. Derive `garde::Validate` and use `#[garde(...)]` attributes:

### Range Validation

```rust
#[garde(range(min = 1, max = 65535))]
pub port: u16,
```

Supported data types:
- u8, u16, u32, u64, u128, usize
- i8, i16, i32, i64, i128, isize
- f32, f64

### Length Validation

```rust
#[garde(length(min = 0, max = 100))]
pub username: String,
```

Supported:
- String length
- Array length

### Built-in Validators

> Each attribute requires garde's corresponding feature: `email` needs `email`, `url` needs `url`, `pattern` needs `pattern` (`regex`). This repo's manifest ships `["derive", "email"]` by default; using `url`/`pattern` requires adding the corresponding feature on your own `garde` dependency — add it before copying the examples below, otherwise compilation fails.

**Email validation** (garde feature `email`, already in this repo's manifest)
```rust
#[garde(email)]
pub email: String,
```

**URL validation** (add the garde feature `url` yourself)
```rust
#[garde(url)]
pub website: String,
```

**Pattern validation** (add the garde feature `pattern` yourself)
```rust
#[garde(pattern(r"^[A-Z]{2}\d{6}$"))]
pub id_code: String,
```

### Custom Validation

```rust
#[garde(custom(my_validator))]
pub field: String,

// The second argument is the struct's context type &Self::Context (&() when #[garde(context)] is not used)
fn my_validator(value: &str, _: &()) -> garde::Result {
    if value.contains("invalid") {
        return Err(garde::Error::new("value contains invalid content"));
    }
    Ok(())
}
```

---

## 🎨 Feature Dependencies

| Attribute/Method | Required Feature |
|-----------|----------|
| `#[config(validate_helper)]` (generates `confers_validate()`) | `validation` |
| `#[config(validate)]` (historical no-op, generates no code) | none |
| `#[config(watch = true)]` (struct-level, currently no effect) | `watch` |
| `json_schema()` | `json-schema` |
| `TypeScriptGenerator::generate::<T>()` | `typescript-schema` |
| CLI argument support | `cli` |
| Encryption support | `encryption` |
| Remote config | `remote` |
| Variable interpolation | `interpolation` |
| Dynamic fields | `dynamic` |
| Module groups | `modules` |

---

## 🌟 Best Practices

### Recommended Configuration

```toml
# Cargo.toml
[dependencies]
confers = { version = "0.6.0", features = ["recommended"] }
garde = { version = "0.23", features = ["derive", "email"] }
```

The feature list of the `recommended` preset is in [README · Feature Presets](../../README.md#-feature-flags).

### Development Configuration

```toml
# Cargo.toml
[dependencies]
confers = { version = "0.6.0", features = ["dev"] }
garde = { version = "0.23", features = ["derive", "email"] }
```

The `dev` feature includes most developer-convenience features.

### Production Configuration

```toml
# Cargo.toml
[dependencies]
confers = { version = "0.6.0", features = ["production"] }
garde = { version = "0.23", features = ["derive", "email"] }
```

The feature list of the `production` preset is likewise in [README · Feature Presets](../../README.md#-feature-flags).

---

## 🔧 Troubleshooting

### Common Issues

**Q: Config values loading incorrectly?**
A: Check that the environment variable prefix is correct and confirm the config file format matches.

**Q: Validation failing without knowing why?**
A: Generate `confers_validate()` with `#[config(validate_helper)]` and invoke it explicitly after `build()` (validation never runs automatically), then inspect the returned message for the field paths and errors from the garde report.

**Q: Sensitive fields leaking into logs?**
A: Make sure sensitive fields are marked with `sensitive = true` and that their type is `SecretString` / `SecretBytes` (requires the `encryption` feature) — marking a plain `String` field `sensitive` errors at macro expansion.

**Q: Hot reload not working?**
A: Make sure the `watch` feature is enabled and use `FsWatcher` / `MultiFsWatcher` to watch changes and rebuild the config (`build_with_watcher()` is deprecated and does not reload on file changes).

---

## 📚 Related Documents

| Document | Description |
|:-----|:-----|
| [📖 User Guide](USER_GUIDE.md) | config loading modes and source combinations |
| [📘 API Reference](API_REFERENCE.md) | the runtime APIs used by generated macro code |
| [⚡ Performance Guide](PERFORMANCE.md) | controlling macro codegen's impact on binary size |

---

*This document was written for Confers v0.6.0.*
