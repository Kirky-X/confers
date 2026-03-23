# Confers Config Macro Complete Usage Guide

## Overview

`#[derive(Config)]` is the core macro of the Confers library. It automatically generates complete configuration management functionality for Rust structs. This macro is located in `macros/src/lib.rs` and implements code generation through `codegen.rs` and `parse.rs`.

---

## 1. Struct-Level Attributes

### 1.1 Enable Validation

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
#[config(validate)]  // Enable configuration validation
pub struct AppConfig {
    pub name: String,
    pub port: u16,
}
```

**Effects**:
- Automatically implements the `validator::Validate` trait
- Validates all fields when `config.validate()` is called

---

### 1.2 Environment Variable Prefix

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
#[config(env_prefix = "APP_")]  // Reads APP_NAME, APP_PORT, etc.
pub struct AppConfig {
    pub name: String,
    pub port: u16,
}
```

**Effects**:
- Adds a prefix when reading environment variables
- Example: `APP_NAME=myapp` maps to the `name` field

---

### 1.3 Application Name

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
#[config(app_name = "myapp")]  // Configuration directory name
pub struct AppConfig {
    pub name: String,
}
```

**Effects**:
- Specifies the directory name when searching for configuration files
- Searches paths like `~/.config/myapp/`, `/etc/myapp/`, etc.

---

### 1.4 Strict Mode

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
#[config(strict = true)]  // Exit on CLI argument parsing errors
pub struct AppConfig {
    pub name: String,
}
```

**Effects**:
- Returns an error when CLI argument parsing fails
- Non-strict mode ignores errors

---

### 1.5 File Watching (Hot Reload)

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
#[config(watch = true)]  // Enable file watching
pub struct AppConfig {
    #[config(default = 8080)]
    pub port: u16,
}
```

**Effects**:
- Requires the `watch` feature to be enabled
- Use `ConfigBuilder::build_with_watcher()` to get a watcher

---

### 1.6 Configuration Version

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
#[config(version = 2)]  // Configuration version for migrations
pub struct AppConfig {
    pub name: String,
}
```

**Effects**:
- Used with configuration migrations
- Enables version tracking for schema evolution

---

### 1.7 Profile Overlay

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
#[config(profile)]  // Enable profile overlay
#[config(profile_env = "APP_ENV")]  // Profile environment variable
pub struct AppConfig {
    pub name: String,
}
```

**Effects**:
- Enables loading profile-specific configuration files
- Profile determined by `profile_env` environment variable

---

## 2. Field-Level Attributes

### 2.1 Default Values

**Method 1: New Syntax (Recommended)**
```rust
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
pub struct AppConfig {
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

**Method 2: Old Syntax for String Types**
```rust
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
pub struct AppConfig {
    #[config(default = "\"default_value\".to_string()")]
    pub name: String,
}
```

**Effects**:
- Uses default value when the field is missing from the configuration file
- Automatically implements the `Default` trait

---

### 2.2 Field Description

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
pub struct AppConfig {
    #[config(description = "Server port number")]
    pub port: u16,

    #[config(description = "Database connection URL")]
    pub database_url: String,
}
```

**Effects**:
- Generates CLI help information
- Used for JSON Schema generation

---

### 2.3 Configuration Name Mapping

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
pub struct AppConfig {
    #[config(name = "app_name")]  // Use app_name in configuration file
    pub name: String,
}
```

**Effects**:
- Field name is `name`, but configuration key is `app_name`

---

### 2.4 Environment Variable Name Mapping

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
#[config(env_prefix = "APP")]
pub struct AppConfig {
    #[config(name_env = "CUSTOM_PORT")]  // Reads APP_CUSTOM_PORT
    pub port: u16,
}
```

**Priority**: `name_env` > Auto-derived

---

### 2.5 CLI Argument Names

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
pub struct AppConfig {
    #[config(name_clap_long = "server-port")]
    pub port: u16,

    #[config(name_clap_short = 'p')]
    pub port2: u16,
}
```

**Effects**:
- CLI arguments: `--server-port` or `-p`

---

### 2.6 Validation Rules

Confers uses the `garde` validation library. To enable validation, derive `garde::Validate` and add validation attributes to fields:

**Range Validation**
```rust
use confers::Config;
use garde::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, Config, Validate)]
#[config(validate)]
pub struct AppConfig {
    #[garde(range(min = 1, max = 65535))]
    pub port: u16,

    #[garde(range(min = 0, max = 100))]
    pub rate: i32,
}
```

**Length Validation**
```rust
#[derive(Debug, Clone, Serialize, Deserialize, Config, Validate)]
#[config(validate)]
pub struct AppConfig {
    #[garde(length(min = 3, max = 50))]
    pub username: String,
}
```

**Built-in Validators**
```rust
#[derive(Debug, Clone, Serialize, Deserialize, Config, Validate)]
#[config(validate)]
pub struct AppConfig {
    #[garde(email)]
    pub email: String,

    #[garde(url)]
    pub website: String,
}
```

**Note:** The `#[config(validate)]` attribute enables validation during build, but the actual validation rules are specified using `#[garde(...)]` attributes from the `garde` crate.

---

### 2.7 Sensitive Fields

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
pub struct AppConfig {
    #[config(sensitive = true)]
    pub password: String,

    #[config(sensitive = true)]
    pub api_key: String,
}
```

**Effects**:
- Automatically masked in audit logs
- Sensitive information is not output in plain text

---

### 2.8 Flattened Fields

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
pub struct DatabaseConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize, Config)]
pub struct AppConfig {
    #[config(flatten)]
    pub database: DatabaseConfig,

    pub app_name: String,
}
```

**Effects**:
- Fields of nested structures are promoted to the top level
- Supports both `database.host` and `database_host` access methods

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

### 2.9 Skip Fields

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
pub struct AppConfig {
    pub name: String,

    #[config(skip)]
    pub temp_field: String,  // Will not be loaded from configuration
}
```

**Effects**:
- This field will not be loaded from the configuration file
- Uses the struct's default value

---

### 2.10 Encrypted Fields

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
pub struct AppConfig {
    #[config(encrypt = "xchacha20")]
    pub database_password: String,

    #[config(encrypt = "xchacha20")]
    pub api_key: String,
}
```

**Effects**:
- Field value is automatically decrypted when loaded
- Requires the `encryption` feature to be enabled
- Uses XChaCha20-Poly1305 encryption algorithm

---

### 2.11 Interpolation

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
pub struct AppConfig {
    #[config(interpolate)]
    pub database_url: String,  // Supports ${VAR} syntax
}
```

**Effects**:
- Enables variable interpolation for this field
- Supports `${VAR}` and `${VAR:-default}` syntax
- Requires the `interpolation` feature

---

### 2.12 Merge Strategy

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
pub struct AppConfig {
    #[config(merge_strategy = "append")]
    pub hosts: Vec<String>,

    #[config(merge_strategy = "deep_merge")]
    pub settings: HashMap<String, String>,
}
```

**Available Strategies**:
- `replace`: Replace existing value (default)
- `append`: Append to arrays
- `prepend`: Prepend to arrays
- `join`: Join array values
- `deep_merge`: Deep merge maps

---

### 2.13 Dynamic Fields

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
pub struct AppConfig {
    #[config(dynamic)]
    pub feature_flags: HashMap<String, bool>,
}
```

**Effects**:
- Generates a `DynamicField` handle for runtime updates
- Requires the `dynamic` feature
- Enables hot-reloadable configuration sections

---

### 2.14 Module Groups

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
pub struct AppConfig {
    #[config(module_group = "database")]
    pub db_host: String,

    #[config(module_group = "database")]
    pub db_port: u16,
}
```

**Effects**:
- Groups related fields for modular configuration
- Enables module-level reload and validation
- Requires the `modules` feature

---

## 3. Comprehensive Example

```rust
use confers::Config;
use garde::Validate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Config, Validate)]
#[config(
    validate,                                    // Enable validation
    env_prefix = "APP_",                         // Environment variable prefix
    app_name = "myapp",                         // Application name
    strict = false,                              // Non-strict mode
    watch = false,                               // Don't watch file changes
    version = 1,                                 // Configuration version
)]
pub struct AppConfig {
    // ============ Basic Types ============
    #[config(description = "Application name")]
    pub name: String,

    #[config(default = 8080, description = "Server port")]
    pub port: u16,

    #[config(default = false, description = "Debug mode")]
    pub debug: bool,

    // ============ String Types ============
    #[config(default = "\"localhost\".to_string()", description = "Server host")]
    pub host: String,

    // ============ Validation Rules (using garde) ============
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

    // ============ Sensitive Fields ============
    #[config(sensitive = true, description = "Database password")]
    pub db_password: String,

    #[config(sensitive = true, description = "API key")]
    pub api_key: String,

    // ============ Encrypted Fields ============
    #[config(encrypt = "xchacha20", description = "Secret token")]
    pub secret_token: String,

    // ============ Interpolation ============
    #[config(interpolate, description = "Database URL")]
    pub database_url: String,

    // ============ Custom Mapping ============
    #[config(name_env = "CUSTOM_DATABASE_URL", description = "Custom database URL")]
    pub custom_db_url: String,

    // ============ Nested Configuration ============
    #[config(flatten, description = "Database configuration")]
    pub database: DatabaseConfig,

    // ============ Skip Fields ============
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

## 4. Auto-Generated Methods

After using the `#[derive(Config)]` macro, the struct automatically gains the following methods:

### 4.1 Configuration Builder

```rust
use confers::ConfigBuilder;

// Basic loading with ConfigBuilder
let config = ConfigBuilder::<AppConfig>::new()
    .file("config.toml")
    .env()
    .build()?;

// With environment prefix
let config = ConfigBuilder::<AppConfig>::new()
    .file("config.toml")
    .env_prefix("APP_")
    .build()?;

// With hot reload (requires watch feature)
let (rx, guard) = ConfigBuilder::<AppConfig>::new()
    .file("config.toml")
    .watch(true)
    .build_with_watcher().await?;
```

### 4.2 Helper Functions

```rust
// Convenient config() function
let config = confers::config::<AppConfig>()
    .file("config.toml")
    .env()
    .build()?;
```

### 4.3 Schema Generation

```rust
// Generate JSON Schema (requires schema feature)
// Note: Requires deriving ConfigSchema
let schema = AppConfig::json_schema();

// Generate TypeScript types (requires typescript-schema feature)
let ts_type = AppConfig::typescript_type();
```

### 4.4 Other Methods

```rust
// Get default values
let default = AppConfig::default();

// Access configuration values
let value = config.some_field;
```

---

## 5. Complete Usage Examples

### 5.1 Basic Usage

**Define Configuration Struct**
```rust
use confers::Config;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Config)]
#[config(validate)]
#[config(env_prefix = "APP_")]
pub struct ServerConfig {
    pub host: String,

    #[config(default = 8080)]
    pub port: u16,

    #[config(default = true)]
    pub enabled: bool,
}
```

**Create Configuration File `config.toml`**
```toml
host = "0.0.0.0"
port = 9000
enabled = false
```

**Use Configuration**
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

**Environment Variable Override**
```bash
export APP_PORT=3000
export APP_ENABLED=true
cargo run
```

### 5.2 Sensitive Configuration Encryption

```rust
use confers::Config;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Config)]
pub struct SecureConfig {
    #[config(sensitive = true)]
    pub password: String,

    #[config(encrypt = "xchacha20")]
    pub api_secret: String,
}
```

**Encryption uses XChaCha20-Poly1305 algorithm. Store nonce alongside ciphertext.**

### 5.3 Hot Reload

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

    let (rx, guard) = ConfigBuilder::<HotReloadConfig>::new()
        .file("config.toml")
        .watch(true)
        .build_with_watcher().await?;

    let config = rx.borrow().clone();
    println!("Initial port: {}", config.port);

    // Application running...
    // When config file changes, rx will receive updates

    Ok(())
}
```

---

## 6. Attribute Summary Table

### Struct-Level Attributes

| Attribute | Purpose |
|-----------|---------|
| `validate` | Enable configuration validation (requires garde::Validate derive) |
| `env_prefix` | Environment variable prefix |
| `app_name` | Application name (config directory) |
| `strict` | Strict mode for CLI parsing |
| `watch` | Enable file watching |
| `version` | Configuration version for migrations |
| `profile` | Enable profile overlay |
| `profile_env` | Profile environment variable name |

### Field-Level Attributes

| Attribute | Purpose |
|-----------|---------|
| `default` | Default value expression |
| `description` | Field description for documentation |
| `name` | Configuration key name override |
| `name_env` | Environment variable name override |
| `name_clap_long` | CLI long argument name |
| `name_clap_short` | CLI short argument character |
| `sensitive` | Mark field as sensitive (hidden in logs) |
| `encrypt` | Encryption algorithm (e.g., "xchacha20") |
| `flatten` | Flatten nested configuration |
| `skip` | Skip this field during loading |
| `interpolate` | Enable variable interpolation |
| `merge_strategy` | Merge strategy for multi-source |
| `dynamic` | Generate DynamicField handle |
| `module_group` | Group for modular configuration |

---

## 7. Validation with Garde

Validation is handled by the `garde` crate. Derive `garde::Validate` and use `#[garde(...)]` attributes:

### 7.1 Range Validation

```rust
#[garde(range(min = 1, max = 65535))]
pub port: u16,
```

Supported data types:
- u8, u16, u32, u64, u128, usize
- i8, i16, i32, i64, i128, isize
- f32, f64

### 7.2 Length Validation

```rust
#[garde(length(min = 0, max = 100))]
pub username: String,
```

Supports:
- String length
- Array length

### 7.3 Built-in Validators

**email validation**
```rust
#[garde(email)]
pub email: String,
```

**url validation**
```rust
#[garde(url)]
pub website: String,
```

**pattern validation**
```rust
#[garde(pattern(r"^[A-Z]{2}\d{6}$"))]
pub id_code: String,
```

### 7.4 Custom Validation

```rust
#[garde(custom(my_validator))]
pub field: String,

fn my_validator(value: &str, _: &garde::ValidateContext) -> garde::Result {
    if value.contains("invalid") {
        return Err(garde::Error::new("value contains invalid content"));
    }
    Ok(())
}
```

---

## 8. Feature Dependencies

| Attribute/Method | Required Feature |
|------------------|------------------|
| `#[config(validate)]` | `validation` |
| `#[config(watch = true)]` | `watch` |
| `json_schema()` | `schema` |
| `typescript_type()` | `typescript-schema` |
| CLI argument support | `cli` |
| Encryption support | `encryption` |
| Remote configuration | `remote` |
| Interpolation | `interpolation` |
| Dynamic fields | `dynamic` |
| Module groups | `modules` |

---

## 9. Best Practices

### 9.1 Recommended Configuration

```toml
# Cargo.toml
[dependencies]
confers = { version = "0.3", features = ["recommended"] }
garde = { version = "0.22", features = ["derive"] }
```

The `recommended` feature includes: `toml`, `json`, `env`, `validation`

### 9.2 Development Environment Configuration

```toml
# Cargo.toml
[dependencies]
confers = { version = "0.3", features = ["dev"] }
garde = { version = "0.22", features = ["derive"] }
```

The `dev` feature includes most features for development convenience.

### 9.3 Production Environment Configuration

```toml
# Cargo.toml
[dependencies]
confers = { version = "0.3", features = ["production"] }
garde = { version = "0.22", features = ["derive"] }
```

The `production` feature includes: `toml`, `env`, `watch`, `encryption`, `validation`, `audit`, `profile`, `metrics`, `schema`, `cli`, `migration`, `dynamic`, `progressive-reload`, `snapshot`

---

## 10. Troubleshooting

### 10.1 Common Issues

**Q: Configuration values not loading correctly?**
A: Check if the environment variable prefix is correct, and confirm the configuration file format matches.

**Q: Validation failed but don't know why?**
A: Use `strict = true` mode to see detailed error messages.

**Q: Sensitive fields leaked in logs?**
A: Make sure to mark sensitive fields with `sensitive = true` attribute.

**Q: Hot reload not working?**
A: Ensure the `watch` feature is enabled and you're using the `load_with_watcher()` method.

---

*This document is based on Confers v0.3.0*
