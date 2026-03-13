<span id="top"></span>
<div align="center">

<img src="image/confers.png" alt="Confers Logo" width="150" style="margin-bottom: 16px">

# 📘 API Reference Documentation

[🏠 Home](../README.md) • [📖 User Guide](USER_GUIDE.md)

---

</div>

## 📋 Table of Contents

<details open style="padding:16px">
<summary style="cursor:pointer; font-weight:600; color:#1E293B">📑 Table of Contents (click to expand)</summary>

- [Overview](#overview)
- [Core API](#core-api)
  - [Configuration Loader](#configuration-loader)
  - [Key Management](#key-management)
  - [Encryption Functions](#encryption-functions)
  - [Configuration Diff Comparison](#configuration-diff-comparison)
  - [Schema Generation](#schema-generation)
- [Error Handling](#error-handling)
- [Type Definitions](#type-definitions)
- [Examples](#examples)
- [Best Practices](#best-practices)
- [Advanced Features](#advanced-features)
- [Performance Optimization](#performance-optimization)
- [Security Considerations](#security-considerations)
- [Troubleshooting](#troubleshooting)

</details>

---

## Overview

<div align="center" style="margin: 24px 0">

### 🎯 API Design Principles

</div>

<table style="width:100%; border-collapse: collapse">
<tr>
<td align="center" width="25%" style="padding: 16px">
<img src="https://img.icons8.com/fluency/96/000000/easy.png" width="48" height="48"><br>
<b style="color:#166534">Simple</b><br>
<span style="color:#166534">Intuitive and easy to use</span>
</td>
<td align="center" width="25%" style="padding: 16px">
<img src="https://img.icons8.com/fluency/96/000000/security-checked.png" width="48" height="48"><br>
<b style="color:#1E40AF">Secure</b><br>
<span style="color:#1E40AF">Type-safe by default</span>
</td>
<td align="center" width="25%" style="padding: 16px">
<img src="https://img.icons8.com/fluency/96/000000/module.png" width="48" height="48"><br>
<b style="color:#92400E">Composable</b><br>
<span style="color:#92400E">Build complex workflows easily</span>
</td>
<td align="center" width="25%" style="padding: 16px">
<img src="https://img.icons8.com/fluency/96/000000/documentation.png" width="48" height="48"><br>
<b style="color:#5B21B6">Well Documented</b><br>
<span style="color:#5B21B6">Comprehensive documentation support</span>
</td>
</tr>
</table>

### 📦 Feature Description

<div style="padding:16px; margin: 16px 0">

confers provides flexible feature configuration, allowing users to select the functionality they need:

**Feature Presets:**

| Preset | Features | Use Case |
|--------|----------|----------|
| <span style="color:#166534">minimal</span> | `env` | Minimal dependencies (environment variables only) |
| <span style="color:#1E40AF">recommended</span> | `toml` + `env` + `validation` | Recommended for most applications |
| <span style="color:#92400E">dev</span> | `toml` + `json` + `yaml` + `env` + `cli` + `validation` + `schema` + `audit` + `profile` + `watch` + `migration` + `snapshot` + `dynamic` | Development configuration |
| <span style="color:#991B1B">production</span> | `toml` + `env` + `watch` + `encryption` + `validation` + `audit` + `profile` + `metrics` + `schema` + `cli` + `migration` + `dynamic` + `progressive-reload` + `snapshot` | Production configuration |
| <span style="color:#5B21B6">full</span> | All features | Complete feature set |

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
| `validation` | Configuration validation (garde) | ✅ |
| `watch` | File monitoring and hot reload | ❌ |
| `encryption` | XChaCha20 encryption | ❌ |
| `cli` | Command-line integration | ❌ |
| `schema` | JSON Schema generation | ❌ |
| `typescript-schema` | TypeScript type generation | ❌ |
| `parallel` | Parallel validation | ❌ |
| **Security** |||
| `security` | Security module | ❌ |
| `key` | Key management system | ❌ |
| **Advanced Features** |||
| `audit` | Audit logging | ❌ |
| `metrics` | Metrics collection | ❌ |
| `dynamic` | Dynamic fields | ❌ |
| `progressive-reload` | Progressive deployment | ❌ |
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
| **Others** |||
| `config-bus` | Configuration event bus | ❌ |
| `context-aware` | Context-aware configuration | ❌ |
| `modules` | Modular configuration | ❌ |

</div>

> 💡 **Tip**: This API documentation assumes the `full` feature is enabled. With other feature combinations, some APIs may not be available.

---

## Core API

### Configuration Builder

`ConfigBuilder<T>` is the core component for loading and merging configuration from multiple sources, supporting intelligent merging of files, environment variables, remote sources, and other configuration sources.

<div align="center" style="margin: 24px 0">

#### 🏗️ ConfigBuilder Architecture

</div>

```mermaid
graph TB
    subgraph Sources ["📥 Configuration Sources"]
        A[📁 Config Files]
        B[🌐 Environment Variables]
        C[💻 CLI Arguments]
        D[☁️ Remote Sources]
    end
    
    subgraph Loader ["🔧 ConfigBuilder"]
        E[⚡ Smart Merging]
        F[✅ Validation]
        G[🔄 Hot Reload]
    end
    
    subgraph Output ["📤 Output"]
        H[🚀 Type-Safe Configuration]
    end
    
    Sources --> Loader
    Loader --> Output
    
    style Sources fill:#DBEAFE,stroke:#1E40AF
    style Loader fill:#FEF3C7,stroke:#92400E
    style Output fill:#DCFCE7,stroke:#166534
```

#### Creation and Configuration

##### `ConfigBuilder::new()`

Create a new configuration builder instance.

```rust
pub fn new() -> Self
```

**Example:**

```rust
let builder = ConfigBuilder::<AppConfig>::new();
```

**Note:** `ConfigBuilder` implements the `Default` trait. The `new()` method returns an instance with sensible defaults.

##### `defaults(defaults: HashMap<String, ConfigValue>)`

Set default configuration values, which will be used when other sources don't provide them.

```rust
pub fn defaults(mut self, defaults: HashMap<String, ConfigValue>) -> Self
```

**Example:**

```rust
use std::collections::HashMap;
use confers::ConfigValue;

let mut defaults = HashMap::new();
defaults.insert("port".to_string(), ConfigValue::uint(8080));
defaults.insert("host".to_string(), ConfigValue::string("localhost"));

let builder = ConfigBuilder::<AppConfig>::new()
    .defaults(defaults);
```

**Note:** Default values have the lowest priority and will be overridden by other configuration sources.

##### `file(path: impl Into<PathBuf>)`

Add an explicit configuration file. Multiple configuration files are supported, with priority increasing in the order they are added.

```rust
pub fn file(mut self, path: impl Into<PathBuf>) -> Self
```

**Example:**

```rust
let builder = ConfigBuilder::<AppConfig>::new()
    .file("config/base.toml")
    .file("config/development.toml");
```

**Note:** Files are loaded in the order they are added, with later files having higher priority.

##### `file_optional(path: impl Into<PathBuf>)`

Add an optional configuration file. If the file doesn't exist, it will be silently skipped.

```rust
pub fn file_optional(mut self, path: impl Into<PathBuf>) -> Self
```

**Example:**

```rust
let builder = ConfigBuilder::<AppConfig>::new()
    .file("config.toml")
    .file_optional("config.local.toml"); // May not exist
```

##### `env()`

Add an environment source. Environment variables will be loaded and mapped to configuration fields.

```rust
pub fn env(mut self) -> Self
```

**Example:**

```rust
let builder = ConfigBuilder::<AppConfig>::new()
    .file("config.toml")
    .env();
```

##### `env_prefix(prefix: impl Into<String>)`

Add an environment source with a prefix. Only environment variables starting with the prefix will be loaded.

```rust
pub fn env_prefix(mut self, prefix: impl Into<String>) -> Self
```

**Example:**

```rust
let builder = ConfigBuilder::<AppConfig>::new()
    .env_prefix("APP");
// Loads APP_PORT, APP_HOST, etc.
```

**Example:**

```rust
let builder = ConfigBuilder::<AppConfig>::new()
    .env()
    .env_prefix("APP");
```

**Note:** Environment variables have higher priority than configuration files, but lower than memory sources.

##### `watch(enabled: bool)`

Enable or disable file watching for automatic configuration reloading. Configuration will be automatically reloaded when configuration files change.

```rust
pub fn watch(mut self, watch: bool) -> Self
```

**Example:**

```rust
let builder = ConfigBuilder::<AppConfig>::new()
    .file("config.toml")
    .watch(true);
```

**Note:** Enabling file watching requires the `watch` feature.

##### `fail_fast(enabled: bool)`

Enable or disable fail-fast mode. In fail-fast mode, any configuration error will immediately stop the build process.

```rust
pub fn fail_fast(mut self, fail_fast: bool) -> Self
```

##### `validate(enabled: bool)`

Enable or disable validation on load.

```rust
pub fn validate(mut self, validate: bool) -> Self
```

##### `limits(limits: ConfigLimits)`

Set configuration limits for resource management.

```rust
pub fn limits(mut self, limits: ConfigLimits) -> Self
```

##### `strategy(strategy: MergeStrategy)`

Set the merge strategy for combining configuration sources.

```rust
pub fn strategy(mut self, strategy: MergeStrategy) -> Self
```

#### Remote Configuration

<div style="padding:16px; margin: 16px 0">

⚠️ **Note**: Remote configuration requires enabling the `remote` feature. Use the `source()` method to add remote configuration sources.

</div>

##### `source(source: Box<dyn Source>)`

Add a custom configuration source. This is the unified method for adding remote sources.

```rust
pub fn source(mut self, source: Box<dyn Source>) -> Self
```

**Example - HTTP Remote Source:**

```rust
use confers::source::HttpSource;

let http_source = HttpSource::new("https://config-server.example.com/app-config")
    .with_timeout(Duration::from_secs(30));

let config = ConfigBuilder::<AppConfig>::new()
    .source(Box::new(http_source))
    .build()?;
```

**Example - Etcd Source:**

```rust
use confers::source::EtcdSource;

let etcd_source = EtcdSource::new("localhost:2379")
    .with_prefix("/myapp/config");

let config = ConfigBuilder::<AppConfig>::new()
    .source(Box::new(etcd_source))
    .build()?;
```

**Example - Consul Source:**

```rust
use confers::source::ConsulSource;

let consul_source = ConsulSource::new("localhost:8500")
    .with_prefix("myapp/config");

let config = ConfigBuilder::<AppConfig>::new()
    .source(Box::new(consul_source))
    .build()?;
```

#### Audit Features

<div style="padding:16px; margin: 16px 0">

📝 **Tip**: The following methods require enabling the `audit` feature.

</div>

##### `with_audit(enabled: bool)`

Enable or disable audit logging for configuration loading.

```rust
#[cfg(feature = "audit")]
pub fn with_audit(mut self, enabled: bool) -> Self
```

##### `with_audit_file(path: impl Into<String>)`

Configure the path for the audit log file.

```rust
#[cfg(feature = "audit")]
pub fn with_audit_file(mut self, path: impl Into<String>) -> Self
```

##### `with_audit_log(enabled: bool)`

Enable or disable audit logging.

```rust
#[cfg(feature = "audit")]
pub fn with_audit_log(mut self, enabled: bool) -> Self
```

##### `with_audit_log_path(path: impl Into<String>)`

Configure the audit log file path and enable auditing.

```rust
#[cfg(feature = "audit")]
pub fn with_audit_log_path(mut self, path: impl Into<String>) -> Self
```

#### Building Methods

##### `build()`

Build the configuration synchronously, merging all configured sources.

```rust
pub fn build(self) -> ConfigResult<T>
```

**Example:**

```rust
let config = builder.build()?;
```

##### `build_with_fallback(fallback: T)`

Build with a fallback configuration. If the build fails, returns the fallback configuration.

```rust
pub fn build_with_fallback(self, fallback: T) -> BuildResult<T>
```

**Example:**

```rust
let result = builder.build_with_fallback(AppConfig::default());
if result.degraded {
    println!("Using fallback: {:?}", result.degraded_reason);
}
```

##### `build_resilient()`

Build resiliently, collecting warnings instead of failing.

```rust
pub fn build_resilient(self) -> ConfigResult<BuildResult<T>>
```

##### `build_with_watcher()` (async)

Build with hot reload support. Returns a receiver for configuration updates and a watcher guard. Requires `watch` feature.

```rust
#[cfg(feature = "watch")]
pub async fn build_with_watcher(
    self,
) -> ConfigResult<(
    tokio::sync::watch::Receiver<Arc<T>>,
    WatcherGuard,
)>
```

**Example:**

```rust
let (mut rx, _guard) = builder.build_with_watcher().await?;
let config = rx.borrow().clone();
```

#### Format Detection

##### `detect_format_from_content(content: &str) -> Option<Format>`

Intelligently detect configuration format based on file content.

```rust
pub fn detect_format_from_content(content: &str) -> Option<Format>
```

**Supported Detection Formats:** JSON, YAML, TOML, INI

##### `detect_format_from_path(path: &Path) -> Option<Format>`

Detect configuration format based on file extension.

```rust
pub fn detect_format_from_path(path: &Path) -> Option<Format>
```

---

### Key Management

`KeyManager` provides comprehensive management of encryption keys, including rotation, versioning, and key storage. This feature requires enabling the `encryption` feature.

<div align="center" style="margin: 24px 0">

#### 🔐 Key Management Architecture

</div>

```mermaid
graph TB
    subgraph Storage ["📦 Key Storage"]
        A[🔑 Keyring]
        B[📋 Version History]
        C[🛡️ Metadata]
    end
    
    subgraph Manager ["🔧 KeyManager"]
        D[🔄 Rotation Management]
        E[✅ Version Control]
        F[🔒 Secure Storage]
    end
    
    subgraph Operations ["⚡ Operations"]
        G[Create]
        H[Rotate]
        I[Get]
        J[Delete]
    end
    
    Storage --> Manager
    Manager --> Operations
    
    style Storage fill:#FEF3C7,stroke:#92400E
    style Manager fill:#DBEAFE,stroke:#1E40AF
    style Operations fill:#DCFCE7,stroke:#166534
```

#### Creation and Management

##### `KeyManager::new(storage_path: PathBuf)`

Create a new key manager with the specified storage path.

```rust
pub fn new(storage_path: PathBuf) -> Result<Self, ConfigError>
```

**Example:**

```rust
use std::path::PathBuf;

let km = KeyManager::new(PathBuf::from("./keys"))?;
```

##### `initialize(master_key: &[u8; 32], key_id: String, created_by: String)`

Initialize a new keyring with the master key.

```rust
pub fn initialize(
    &mut self,
    master_key: &[u8; 32],
    key_id: String,
    created_by: String,
) -> Result<KeyVersion, ConfigError>
```

**Parameter Description:**

| Parameter | Description |
|-----------|-------------|
| `master_key` | 32-byte master key used to encrypt key storage |
| `key_id` | Unique identifier for the keyring |
| `created_by` | Creator identifier for audit trail |

**Example:**

```rust
use confers::key::KeyManager;
use std::path::PathBuf;

let mut km = KeyManager::new(PathBuf::from("./secure_keys"))?;
let master_key = [0u8; 32]; // Get from secure location
let version = km.initialize(
    &master_key,
    "production".to_string(),
    "security-team".to_string()
)?;
```

##### `rotate_key(master_key: &[u8; 32], key_id: Option<String>, created_by: String, description: Option<String>)`

Rotate the keyring to a new version, supporting key rotation for security compliance.

```rust
pub fn rotate_key(
    &mut self,
    master_key: &[u8; 32],
    key_id: Option<String>,
    created_by: String,
    description: Option<String>,
) -> Result<RotationResult, ConfigError>
```

**Return Value:** `RotationResult` contains pre and post rotation version information and whether re-encryption is needed.

**Example:**

```rust
let result = km.rotate_key(
    &master_key,
    Some("production".to_string()),
    "security-team".to_string(),
    Some("Scheduled key rotation".to_string())
)?;

println!("Key rotated from version {} to {}", result.previous_version, result.new_version);
```

##### `get_key_info(key_id: &str)`

Get metadata and version information for a specific key.

```rust
pub fn get_key_info(&self, key_id: &str) -> Result<KeyInfo, ConfigError>
```

##### `get_active_key_version(key_id: &str, version: u32) -> Result<Vec<u8>, ConfigError>`

Get raw key data for a specific key version.

```rust
pub fn get_active_key_version(&self, key_id: &str, version: u32) -> Result<Vec<u8>, ConfigError>
```

##### `list_key_ids() -> Result<Vec<String>, ConfigError>`

List all managed key IDs.

```rust
pub fn list_key_ids(&self) -> Result<Vec<String>, ConfigError>
```

##### `delete_key_ring(key_id: &str, master_key: &[u8; 32]) -> Result<(), ConfigError>`

Delete the specified keyring.

```rust
pub fn delete_key_ring(&mut self, key_id: &str, master_key: &[u8; 32]) -> Result<(), ConfigError>
```

---

### Encryption Functions

`XChaCha20Crypto` implements XChaCha20-Poly1305 encryption to protect sensitive configuration values, providing authenticated encryption with associated data (AEAD). This feature requires enabling the `encryption` feature.

<div align="center" style="margin: 24px 0">

#### 🔐 Encryption Flow

</div>

```mermaid
graph LR
    A[📝 Plaintext] --> B[🔐 XChaCha20-Poly1305 Encryption]
    B --> C[📦 Output<br/>nonce + ciphertext]
    C --> D[💾 Store or Transmit]
    D --> E[🔓 Decrypt]
    E --> F[✅ Recover Plaintext]

    style B fill:#FEF3C7,stroke:#92400E
    style E fill:#DCFCE7,stroke:#166534
```

#### Creation

##### `XChaCha20Crypto::new()`

Create a new encryptor instance.

```rust
pub fn new() -> Self
```

**Example:**

```rust
use confers::XChaCha20Crypto;

let crypto = XChaCha20Crypto::new();
```

#### Encryption/Decryption Operations

##### `encrypt(plaintext: &[u8], key: &[u8]) -> Result<(Vec<u8>, Vec<u8>), CryptoError>`

Encrypt bytes with a 32-byte key. Returns a tuple of `(nonce, ciphertext)`.

**Features:**

- Uses XChaCha20-Poly1305 algorithm (extended nonce variant of ChaCha20)
- Generates a random 96-bit nonce for each encryption
- Provides authenticated encryption with integrity verification

```rust
pub fn encrypt(&self, plaintext: &[u8], key: &[u8]) -> Result<(Vec<u8>, Vec<u8>), CryptoError>
```

**Example:**

```rust
let key = [0u8; 32]; // Should use a secure random key
let (nonce, ciphertext) = crypto.encrypt(b"my-secret-api-key", &key)?;
```

##### `decrypt(nonce: &[u8], ciphertext: &[u8], key: &[u8]) -> Result<Vec<u8>, CryptoError>`

Decrypt bytes with the nonce, ciphertext, and 32-byte key.

**Features:**

- Requires the same nonce used during encryption
- Verifies Poly1305 authentication tag, tampering detection will trigger an error

```rust
pub fn decrypt(&self, nonce: &[u8], ciphertext: &[u8], key: &[u8]) -> Result<Vec<u8>, CryptoError>
```

**Example:**

```rust
let decrypted = crypto.decrypt(&nonce, &ciphertext, &key)?;
assert_eq!(decrypted, b"my-secret-api-key");
```

#### Key Derivation

##### `derive_field_key(master_key: &[u8], field_path: &str, key_version: &str) -> Result<[u8; 32], CryptoError>`

Derive a field-specific encryption key from a master key using HKDF-SHA256.

```rust
pub fn derive_field_key(
    master_key: &[u8],
    field_path: &str,
    key_version: &str,
) -> Result<[u8; 32], CryptoError>
```

**Example:**

```rust
use confers::derive_field_key;

let master_key = [0u8; 32];
let field_key = derive_field_key(&master_key, "database.password", "v1")?;
```

---

### Key Management

The `key` feature provides complete key lifecycle management, including key creation, storage, rotation, and revocation.

#### KeyManager

The core component for key lifecycle management.

```rust
use confers::key::KeyManager;

// Create key manager
let master_key = [0u8; 32];
let mut manager = KeyManager::new(master_key);

// Create new key
let key = manager.create_key("database", Some("DB encryption key".to_string()))?;

// Get key
let key_data = manager.get_key("database")?;

// List all keys
let keys = manager.list_keys();

// Revoke key
manager.revoke_key("database")?;
```

#### Methods

| Method | Parameters | Return Value | Description |
|--------|------------|--------------|-------------|
| `new(master_key)` | `&[u8; 32]` | `Self` | Create key manager |
| `create_key(id, desc)` | `&str, Option<String>` | `Result<KeyBundle>` | Create new key |
| `get_key(id)` | `&str` | `Option<&KeyBundle>` | Get key |
| `list_keys()` | - | `Vec<KeyInfo>` | List all keys |
| `revoke_key(id)` | `&str` | `Result<()>` | Revoke key |

#### KeyRotationService

Automatic key rotation service.

```rust
use confers::key::{KeyRotationService, KeyRotationPolicy};
use std::time::Duration;

let policy = KeyRotationPolicy::default()
    .with_max_age(Duration::from_secs(86400 * 90)); // 90 days

let service = KeyRotationService::new(manager, policy);
service.check_and_rotate()?;
```

#### KeyStorage

Encrypted key persistence storage.

```rust
use confers::key::KeyStorage;

let storage = KeyStorage::new("/path/to/keys")?;
storage.save(&key_bundle)?;
let loaded = storage.load("key_id")?;
```

---

### Security Module

The `security` feature provides environment variable validation, error sanitization, and secure injection capabilities.

#### EnvSecurityValidator

Environment variable security validator, prevents injection attacks.

```rust
use confers::security::EnvSecurityValidator;

let validator = EnvSecurityValidator::builder()
    .allow_pattern(r"^[A-Z][A-Z0-9_]*$")
    .block_pattern(r".*_SECRET$")
    .block_pattern(r".*_PASSWORD$")
    .build()?;

// Validate single variable
validator.validate_var("APP_NAME")?;
validator.validate_var("DB_PASSWORD")?; // Err: contains _SECRET

// Validate all environment variables
validator.validate_all(std::env::vars())?;
```

#### ErrorSanitizer

Error message sensitive data sanitization.

```rust
use confers::security::ErrorSanitizer;

let sanitizer = ErrorSanitizer::default();
let clean_msg = sanitizer.sanitize(&error_msg);
```

#### ConfigInjector

Secure configuration injector.

```rust
use confers::security::ConfigInjector;

let injector = ConfigInjector::new()
    .with_validator(validator)
    .inject(&config)?;
```

---

### TypeScript Schema Generation

The `typescript-schema` feature supports generating TypeScript definitions from Rust types.

#### generate_typescript

Generate TypeScript type definitions.

```rust
use confers::schema::generate_typescript;

#[derive(confers::Config)]
pub struct AppConfig {
    pub name: String,
    pub port: u16,
    pub debug: bool,
}

let ts = generate_typescript::<AppConfig>();
println!("{}", ts);
```

**Output:**

```typescript
// Auto-generated from Rust
export interface AppConfig {
  name: string;
  port: number;
  debug: boolean;
}
```

---

### Schema Generation

Configuration structures can generate JSON Schema through the `schemars` crate. Requires enabling the `schema` feature.

To generate a Schema, the configuration structure needs to derive the `JsonSchema` trait:

```rust
use serde::{Deserialize, Serialize};
#[cfg(feature = "schema")]
use schemars::JsonSchema;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct AppConfig {
    pub name: String,
    pub port: u16,
}
```

The generated Schema can be used to validate configuration format or generate documentation.

---

### ConfigProvider Trait

`ConfigProvider` is the core trait for configuration access, providing the fundamental interface for accessing configuration values. All configuration providers must implement this trait.

<div align="center" style="margin: 24px 0">

#### 🔌 Configuration Provider Interface

</div>

#### Trait Definition

```rust
pub trait ConfigProvider: Send + Sync {
    fn get_raw(&self, key: &str) -> Option<&AnnotatedValue>;
    fn keys(&self) -> Vec<String>;
    fn has(&self, key: &str) bool { ... }
}
```

#### Method Description

##### `get_raw(&self, key: &str) -> Option<&AnnotatedValue>`

Get a raw annotated value by key. Returns `None` if the key does not exist.

```rust
fn get_raw(&self, key: &str) -> Option<&AnnotatedValue>
```

##### `keys(&self) -> Vec<String>`

Get all configuration keys in dot-notation format (e.g., "database.host").

```rust
fn keys(&self) -> Vec<String>
```

##### `has(&self, key: &str) -> bool`

Check if a key exists.

```rust
fn has(&self, key: &str) -> bool
```

---

### ConfigProviderExt Trait

`ConfigProviderExt` is an extension trait with convenience methods for type-safe accessors. It provides default implementations for all `ConfigProvider` types.

#### Trait Definition

```rust
pub trait ConfigProviderExt: ConfigProvider {
    fn get_string(&self, key: &str) -> Option<String>;
    fn get_int(&self, key: &str) -> Option<i64>;
    fn get_uint(&self, key: &str) -> Option<u64>;
    fn get_float(&self, key: &str) -> Option<f64>;
    fn get_bool(&self, key: &str) -> Option<bool>;
    fn get_typed<T>(&self, key: &str) -> ConfigResult<T>;
    fn get_many<'a>(&self, keys: &[&'a str]) -> HashMap<&'a str, Option<&AnnotatedValue>>;
    fn get_by_path(&self, path: &[&str]) -> Option<&AnnotatedValue>;
}
```

#### Method Description

##### `get_string(&self, key: &str) -> Option<String>`

Get string type configuration value.

**Example:**

```rust
let name = config.get_string("app.name");
assert_eq!(name, Some("my-app".to_string()));
```

##### `get_int(&self, key: &str) -> Option<i64>`

Get integer type configuration value.

**Example:**

```rust
let port = config.get_int("server.port");
assert_eq!(port, Some(8080));
```

##### `get_uint(&self, key: &str) -> Option<u64>`

Get unsigned integer type configuration value.

##### `get_bool(&self, key: &str) -> Option<bool>`

Get boolean type configuration value.

**Example:**

```rust
let debug = config.get_bool("app.debug");
assert_eq!(debug, Some(true));
```

##### `get_float(&self, key: &str) -> Option<f64>`

Get floating-point type configuration value.

##### `get_typed<T>(&self, key: &str) -> ConfigResult<T>`

Get a typed value by key. Returns an error if the value cannot be converted.

```rust
fn get_typed<T>(&self, key: &str) -> ConfigResult<T>
where
    T: std::str::FromStr + Default,
    T::Err: std::fmt::Display
```

##### `get_many<'a>(&self, keys: &[&'a str]) -> HashMap<&'a str, Option<&AnnotatedValue>>`

Get multiple values efficiently. Missing keys will have `None` values.

---

### KeyProvider Trait

`KeyProvider` is a synchronous encryption key provider trait. Implementations provide encryption keys for sensitive field encryption.

```rust
pub trait KeyProvider: Send + Sync {
    fn get_key(&self) -> ConfigResult<ZeroizingBytes>;
    fn provider_type(&self) -> &'static str;
    fn cache_policy(&self) -> KeyCachePolicy { ... }
}
```

#### KeyCachePolicy

```rust
pub enum KeyCachePolicy {
    Ttl,      // Cache with time-to-live (default)
    Forever,  // Cache indefinitely
    Never,    // Never cache
}
```

---

### TypedConfigKey

Type-safe configuration key that binds a configuration path to a specific type for compile-time safety.

```rust
pub struct TypedConfigKey<T> {
    path: &'static str,
    description: Option<&'static str>,
}

impl<T> TypedConfigKey<T> {
    pub const fn new(path: &'static str) -> Self;
    pub const fn with_description(mut self, description: &'static str) -> Self;
    pub fn path(&self) -> &'static str;
    pub fn description(&self) -> Option<&'static str>;
}
```

**Example:**

```rust
use confers::TypedConfigKey;

static DB_HOST: TypedConfigKey<String> =
    TypedConfigKey::new("database.host")
        .with_description("Database hostname");

static DB_PORT: TypedConfigKey<u16> =
    TypedConfigKey::new("database.port");
```

---

## Error Handling

### `ConfigError`

Common error variants encountered during operations.

<div style="padding:16px; margin: 16px 0">

| Variant | Description | Handling Suggestion |
|---------|-------------|---------------------|
| `FileNotFound { filename: PathBuf, source: Option<std::io::Error> }` | Configuration file not found | Check if file path is correct |
| `ParseError { format: String, message: String, location: Option<ParseLocation>, source: Option<Box<dyn Error>> }` | Error parsing configuration | Check configuration file syntax |
| `InvalidValue { key: String, expected_type: String, message: String }` | Invalid value for key | Check value type and format |
| `SizeLimitExceeded { actual: usize, limit: usize }` | File size exceeds limit | Increase size limit or optimize file |
| `IoError(std::io::Error)` | IO operation error | Check file permissions and disk space |
| `MergeConflict { path: String, message: String }` | Merge conflict between sources | Check source priorities |
| `MissingRequiredKey { key: String }` | Required key not found | Ensure all required keys are provided |

</div>

---

## Type Definitions

### Key Related Types

#### `KeyVersion`

```rust
pub struct KeyVersion {
    pub id: String,           // Key version unique identifier
    pub version: u32,         // Version number
    pub created_at: u64,      // Creation timestamp
    pub status: KeyStatus,    // Key status
    pub algorithm: String,    // Encryption algorithm
}
```

#### `KeyStatus`

```rust
pub enum KeyStatus {
    Active,       // Active, can be used for encryption and decryption
    Deprecated,   // Deprecated, only for decrypting historical data
    Compromised,  // Compromised, should be rotated immediately
}
```

#### `KeyInfo`

```rust
pub struct KeyInfo {
    pub key_id: String,           // Keyring ID
    pub current_version: u32,     // Current active version
    pub total_versions: usize,    // Total versions
    pub active_versions: usize,   // Active versions
    pub deprecated_versions: usize, // Deprecated versions
    pub created_at: u64,          // Creation timestamp
    pub last_rotated_at: Option<u64>, // Last rotation time
}
```

#### `RotationResult`

```rust
pub struct RotationResult {
    pub key_id: String,           // Keyring ID
    pub previous_version: u32,    // Pre-rotation version
    pub new_version: u32,         // Post-rotation version
    pub rotated_at: u64,          // Rotation timestamp
    pub reencryption_required: bool, // Whether re-encryption is needed
}
```

---

## Examples

### Basic Configuration Loading

```rust
use confers::ConfigBuilder;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Default, Clone)]
struct AppConfig {
    database_url: String,
    port: u16,
    debug: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ConfigBuilder::<AppConfig>::new()
        .file("config.toml")
        .env()
        .env_prefix("MYAPP")
        .build()?;

    println!("Database: {}", config.database_url);
    println!("Port: {}", config.port);
    Ok(())
}
```

### Key Rotation

```rust
use confers::key::manager::KeyManager;
use std::path::PathBuf;

fn rotate_keys() -> Result<(), Box<dyn std::error::Error>> {
    let mut km = KeyManager::new(PathBuf::from("./keys"))?;
    let master_key = load_master_key()?; // Load master key from secure storage
    
    let result = km.rotate_key(
        &master_key,
        Some("production".to_string()),
        "security-team".to_string(),
        Some("Scheduled rotation".to_string())
    )?;
    
    println!("Key version rotated from {} to {}", result.previous_version, result.new_version);
    Ok(())
}
```

### Multi-Source Configuration Merging

```rust
use confers::ConfigBuilder;
use confers::ConfigValue;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize)]
struct ServerConfig {
    host: String,
    port: i32,
    workers: usize,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut defaults = HashMap::new();
    defaults.insert("host".to_string(), ConfigValue::string("127.0.0.1"));
    defaults.insert("port".to_string(), ConfigValue::int(8080));
    defaults.insert("workers".to_string(), ConfigValue::uint(4));

    let config = ConfigBuilder::<ServerConfig>::new()
        .defaults(defaults)
        .file("server.toml")
        .env()
        .build()?;

    println!("Server running at {}:{}", config.host, config.port);
    Ok(())
}
```

### Configuration Encryption

```rust
use confers::XChaCha20Crypto;

fn encrypt_sensitive_data() -> Result<(), Box<dyn std::error::Error>> {
    let crypto = XChaCha20Crypto::new();
    let key = load_encryption_key()?; // 32-byte key

    let secret = b"my-super-secret-api-key";
    let (nonce, ciphertext) = crypto.encrypt(secret, &key)?;

    println!("Encrypted {} bytes", ciphertext.len());

    let decrypted = crypto.decrypt(&nonce, &ciphertext, &key)?;
    assert_eq!(decrypted, secret);

    Ok(())
}
```

### Configuration Validation

```rust
use confers::ConfigBuilder;
use garde::Validate;

#[derive(Debug, Deserialize, Validate)]
struct ServerConfig {
    #[garde(length(min = 1))]
    host: String,

    #[garde(range(min = 1, max = 65535))]
    port: u16,
}

fn validate_config() -> Result<(), Box<dyn std::error::Error>> {
    let config = ConfigBuilder::<ServerConfig>::new()
        .file("server.toml")
        .build()?;

    config.validate()?; // Uses garde validation
    println!("Configuration is valid");
    Ok(())
}
        options,
    )?;

    Ok(())
}
```

---

## Best Practices

### Configuration Validation

<div style="padding:16px; margin: 16px 0">

Always use serde's validation features to ensure configuration validity:

</div>

```rust
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use chrono::Duration;

#[serde_as]
#[derive(Deserialize, Serialize)]
struct DatabaseConfig {
    #[serde(default = "default_url")]
    url: String,
    
    #[serde(default = "default_pool_size")]
    #[serde(validate(range(min = 1, max = 100)))]
    pool_size: usize,
    
    #[serde_as(as = "serde_with::DurationSeconds<u64>")]
    #[serde(default = "default_timeout")]
    timeout: Duration,
}

fn default_url() -> String {
    "postgres://localhost:5432/app".to_string()
}

fn default_pool_size() -> usize {
    10
}

fn default_timeout() -> Duration {
    Duration::seconds(30)
}
```

### Key Management Security

<div style="padding:16px; margin: 16px 0">

⚠️ In production environments, be sure to manage keys securely:

</div>

```rust
use confers::key::manager::KeyManager;
use std::path::PathBuf;

fn setup_secure_key_management() -> Result<(), Box<dyn std::error::Error>> {
    // Get master key from environment variable or secure storage
    let master_key = std::env::var("MASTER_KEY")
        .map(|s| {
            let mut key = [0u8; 32];
            let key_bytes = s.as_bytes();
            key.copy_from_slice(&key_bytes[..32.min(key_bytes.len())]);
            key
        })?;
    
    let mut km = KeyManager::new(PathBuf::from("/etc/confers/keys"))?;
    
    // Initialize keyring
    km.initialize(
        &master_key,
        "production".to_string(),
        "security-team".to_string(),
    )?;
    
    // Rotate keys regularly (recommended every 90 days)
    let rotation_result = km.rotate_key(
        &master_key,
        Some("production".to_string()),
        "security-team".to_string(),
        Some("Scheduled rotation".to_string()),
    )?;
    
    println!("Key rotated from version {} to {}", 
        rotation_result.previous_version, 
        rotation_result.new_version);
    
    Ok(())
}
```

### Hot Reload Configuration

Use file watching to implement configuration hot reload:

```rust
use confers::ConfigBuilder;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (mut rx, _guard) = ConfigBuilder::<AppConfig>::new()
        .file("config.toml")
        .watch(true)
        .build_with_watcher()
        .await?;

    let config = rx.borrow().clone();
    println!("Initial configuration loaded: {:?}", config);

    // Listen for configuration changes
    loop {
        tokio::time::sleep(Duration::from_secs(60)).await;
        let updated = rx.borrow().clone();
        println!("Configuration monitoring active");
    }
}
```

**Note:** Hot reload functionality requires enabling the `watch` feature.

### Sensitive Data Encryption

Encrypt sensitive configuration values:

```rust
use confers::XChaCha20Crypto;
use serde::Deserialize;

#[derive(Deserialize)]
struct Secrets {
    #[config(encrypt)]
    api_key: String,

    #[config(encrypt)]
    database_password: String,
}

fn decrypt_secrets() -> Result<(), Box<dyn std::error::Error>> {
    let crypto = XChaCha20Crypto::new();
    let key = load_encryption_key()?; // 32-byte key

    let (nonce, ciphertext) = crypto.encrypt(b"my-secret-api-key", &key)?;
    let decrypted = crypto.decrypt(&nonce, &ciphertext, &key)?;

    Ok(())
}
        api_key,
        database_password: "decrypted-password".to_string(),
    })
}
```

---

## Advanced Features

### Custom Format Parser

For configuration formats not supported by the standard library, you can implement custom parsers:

```rust
use confers::{ConfigBuilder, ConfigError};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize)]
struct CustomConfig {
    settings: HashMap<String, String>,
}

fn load_custom_config() -> Result<CustomConfig, ConfigError> {
    let content = std::fs::read_to_string("config.custom")?;
    let config: CustomConfig = toml::from_str(&content)
        .map_err(|e| ConfigError::ParseError {
            format: "custom".into(),
            message: e.to_string(),
            location: None,
            source: Some(Box::new(e)),
        })?;
    Ok(config)
}
```

### Configuration Rollback

Use version history to implement configuration rollback:

```rust
use confers::ConfigBuilder;
use std::path::PathBuf;

fn rollback_to_previous_version() -> Result<(), Box<dyn std::error::Error>> {
    let config_dir = PathBuf::from("/etc/myapp");

    let versions = std::fs::read_dir(config_dir.join("history"))?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|p| p.extension().map(|e| e == "toml").unwrap_or(false))
        .collect::<Vec<_>>();

    if versions.len() >= 2 {
        let previous_version = &versions[versions.len() - 2];

        let config = ConfigBuilder::<AppConfig>::new()
            .file(previous_version)
            .build()?;

        println!("Rolled back to previous configuration version");
        return Ok(());
    }

    Err("Not enough version history for rollback".into())
}
```

---

## Performance Optimization

### Asynchronous Loading

<div style="padding:16px; margin: 16px 0">

💡 **Tip**: For large configurations or remote configuration sources, always use asynchronous loading:

</div>

```rust
use confers::ConfigBuilder;

fn load_config_efficiently() -> Result<(), Box<dyn std::error::Error>> {
    let start = std::time::Instant::now();

    let config = ConfigBuilder::<AppConfig>::new()
        .file("config.toml")
        .env()
        .build()?;

    let elapsed = start.elapsed();
    println!("Configuration loading time: {:?}", elapsed);

    Ok(())
}
```

### Configuration Caching

For frequently accessed configurations, use memory caching:

```rust
use std::sync::Arc;
use tokio::sync::RwLock;
use confers::ConfigBuilder;

struct CachedConfig {
    cache: Arc<RwLock<Option<AppConfig>>>,
}

impl CachedConfig {
    fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(None)),
        }
    }

    async fn get(&self) -> Result<AppConfig, Box<dyn std::error::Error>> {
        {
            let cached = self.cache.read().await;
            if let Some(config) = &*cached {
                return Ok(config.clone());
            }
        }

        let config = ConfigBuilder::<AppConfig>::new()
            .file("config.toml")
            .env()
            .build()?;

        {
            let mut writer = self.cache.write().await;
            *writer = Some(config.clone());
        }

        Ok(config)
    }
}
```

---

## Security Considerations

<div align="center" style="margin: 24px 0">

### 🔒 Security Best Practices

</div>

#### Sensitive Data Handling

<div style="padding:16px; margin: 16px 0">

**How to identify sensitive fields:**

- Passwords, API keys, access tokens
- Database connection strings
- Private keys, certificates
- Personally identifiable information (PII)

**How to encrypt sensitive data:**

```rust
use confers::XChaCha20Crypto;

// Create encryptor
let crypto = XChaCha20Crypto::new();

// Generate or load a 32-byte key
let key = load_encryption_key()?;

// Encrypt sensitive value
let (nonce, ciphertext) = crypto.encrypt(b"my-secret-api-key", &key)?;
println!("Encrypted {} bytes", ciphertext.len());

// Decrypt sensitive value
let decrypted = crypto.decrypt(&nonce, &ciphertext, &key)?;
assert_eq!(decrypted, b"my-secret-api-key");
```

**⚠️ Security Tips:**

- 🔴 **Key Management**: Encryption keys must be stored securely and never committed to version control systems
- 🔴 **Key Rotation**: Regular key rotation is recommended for production environments
- 🔴 **Key Length**: Keys must be exactly 32 bytes (256 bits) for XChaCha20-Poly1305
- 🔴 **Nonce Storage**: Store nonce alongside ciphertext for decryption

</div>

#### Key Management

**How to generate secure keys:**

```rust
use rand::Rng;

// Generate secure random key
let mut key = [0u8; 32];
let mut rng = rand::thread_rng();
rng.fill(&mut key);

// Use with XChaCha20Crypto
let crypto = XChaCha20Crypto::new();
let (nonce, ciphertext) = crypto.encrypt(b"secret", &key)?;
```

**How to rotate keys:**

```rust
use confers::key::manager::KeyManager;
use std::path::PathBuf;

let mut km = KeyManager::new(PathBuf::from("./keys"))?;
let master_key = load_master_key()?; // Load master key from secure storage

// Rotate key
let result = km.rotate_key(
    &master_key,
    Some("production".to_string()),
    "security-team".to_string(),
    Some("Scheduled key rotation".to_string())
)?;

println!("Key version rotated from {} to {}", result.previous_version, result.new_version);
```

**⚠️ Security Tips:**

- 🔴 **Key Storage**: Use Hardware Security Modules (HSM) or Key Management Services
- 🔴 **Key Rotation**: Recommend rotating keys every 90 days
- 🔴 **Key Backup**: Securely backup keys, ensure recovery is possible
- 🔴 **Key Leak Emergency Handling**: If a key is leaked, rotate immediately and notify relevant teams

</div>

#### Audit Log Configuration

**How to enable audit logging:**

```rust
use confers::audit::{AuditLogWriter, RotationConfig};

let rotation_config = RotationConfig {
    max_size_mb: 100,
    max_age_days: 30,
    max_files: 10,
    compress_archived: true,
};

let integrity_key = [0u8; 32]; // Get from secure storage
let writer = AuditLogWriter::new(
    PathBuf::from("/var/log/audit.log"),
    rotation_config,
    integrity_key
)?;
```

**⚠️ Security Tips:**

- 🔴 **Log Integrity**: Audit logs use HMAC signatures to protect integrity
- 🔴 **Log Access Control**: Restrict audit log file access permissions (root/admin only)
- 🔴 **Log Archival**: Regularly archive audit logs to prevent log files from becoming too large
- 🔴 **Log Monitoring**: Monitor audit log access records, detect anomalous access

</div>

#### Production Environment Security Configuration

**Environment variable security configuration:**

```bash
# Use environment variables to store sensitive information
export APP_DATABASE_URL="postgres://user:password@localhost/db"
export APP_API_KEY="your-api-key"
export CONFERS_ENCRYPTION_KEY="base64-encoded-key"
```

**Remote configuration security configuration:**

```rust
// Remote configuration requires using the remote source directly
use confers::remote::HttpPolledSourceBuilder;

let remote_source = HttpPolledSourceBuilder::new()
    .url("https://config.example.com")
    .token("your-access-token")
    .build()?;

let config = ConfigBuilder::<AppConfig>::new()
    .source(Box::new(remote_source))
    .build()?;
```

**⚠️ Security Tips:**

- 🔴 **TLS Configuration**: Always use TLS encryption for remote configuration transmission
- 🔴 **Access Control**: Restrict access permissions to remote configuration services
- 🔴 **Principle of Least Privilege**: Grant only necessary permissions
- 🔴 **Security Audit**: Regularly audit production environment configuration

</div>

#### API Method Security Annotations

**Encryption API:**

```rust
/// Encrypt sensitive configuration value
///
/// # Security Notes
///
/// - ⚠️ **Key Management**: The encryption key must be stored securely and never committed to version control
/// - ⚠️ **Key Rotation**: Regular key rotation is recommended for production environments
/// - ⚠️ **Key Length**: The key must be exactly 32 bytes (256 bits) for XChaCha20-Poly1305
/// - ⚠️ **Nonce Storage**: Store the nonce alongside the ciphertext for decryption
///
/// # Example
///
/// ```rust
/// let crypto = XChaCha20Crypto::new();
/// let (nonce, ciphertext) = crypto.encrypt(b"sensitive-data", &key)?;
/// ```
pub fn encrypt(&self, plaintext: &[u8], key: &[u8]) -> Result<(Vec<u8>, Vec<u8>), CryptoError>
```

**Key Management API:**

```rust
/// Initialize new keyring
///
/// # Security Notes
///
/// - ⚠️ **Master Key**: The master key must be stored securely and never shared
/// - ⚠️ **Key ID**: Use descriptive key IDs (e.g., "production", "staging")
/// - ⚠️ **Created By**: Include creator information for audit trail
/// - ⚠️ **Key Backup**: Ensure you have a secure backup of the master key
///
/// # Example
///
/// ```rust
/// let version = km.initialize(
///     &master_key,
///     "production".to_string(),
///     "security-team".to_string()
/// )?;
/// ```
pub fn initialize(
    &mut self,
    master_key: &[u8; 32],
    key_id: String,
    created_by: String,
) -> Result<KeyVersion, ConfigError>
```

**Audit Log API:**

```rust
/// Log configuration loading to audit log
///
/// # Security Notes
///
/// - ⚠️ **Log Path**: Store audit logs in a secure location with restricted access
/// - ⚠️ **Log Rotation**: Configure log rotation to prevent disk space exhaustion
/// - ⚠️ **Log Integrity**: Audit logs are signed to prevent tampering
/// - ⚠️ **Log Monitoring**: Monitor audit logs for suspicious activity
///
/// # Example
///
/// ```rust
/// AuditLogger::log_to_file(&config, PathBuf::from("/var/log/audit.log"), None)?;
/// ```
pub fn log_to_file<T>(
    config: &T,
    path: &Path,
    validation_error: Option<&str>,
) -> Result<(), ConfigError>
```

**Configuration Validation API:**

```rust
/// Validate configuration range
///
/// # Security Notes
///
/// - ⚠️ **Input Validation**: Always validate user input before use
/// - ⚠️ **Range Checking**: Ensure numeric values are within expected ranges
/// - ⚠️ **Error Messages**: Avoid exposing sensitive information in error messages
/// - ⚠️ **Validation Failures**: Treat validation failures as potential security incidents
///
/// # Example
///
/// ```rust
/// let validator = RangeFieldValidator::new("port", Some(1024.0), Some(65535.0));
/// validator.validate(&config)?;
/// ```
pub fn validate(&self, config: &Value) -> Result<(), ValidationError>
```

---

## Troubleshooting

### Common Issues

<div style="padding:16px; margin: 16px 0">

| Issue | Solution |
|-------|----------|
| **Q: Configuration file not found?** | Check if the file path is correct, make sure to use absolute path or path relative to working directory. Recommend using `with_app_name()` to let the library automatically search standard locations. |
| **Q: Environment variables not working?** | Confirm `with_env(true)` has been called, and check if environment variable names use the correct prefix. For example, configuration field `port` corresponds to environment variable name `<PREFIX>_PORT`. |
| **Q: Encryption/decryption failed?** | Make sure to use the same key for encryption and decryption, check if `CONFERS_ENCRYPTION_KEY` environment variable is correctly set and format is valid Base64 encoding. |
| **Q: Configuration validation failed?** | View detailed validation error messages, ensure configuration values meet all validation constraints. Check if field types match. |
| **Q: Remote configuration loading timeout?** | Check network connection and remote service availability, configure timeout when creating the source: `HttpSource::new(url).with_timeout(Duration::from_secs(60))`. |
| **Q: Memory usage too high?** | Use `with_memory_limit()` to set memory limit, optimize configuration file size, avoid storing large binary data in configuration. |

</div>

### Debug Logging

Enable verbose logging for debugging:

```rust
use env_logger;

fn setup_logging() {
    env_logger::Builder::from_env(
        env_logger::Env::default()
            .default_filter_or("confers=debug")
    ).init();
}
```

Set log level when running the program:

```bash
RUST_LOG=confers=debug ./myapp
```

---

## Cargo Features

| Feature | Description | Default |
|---------|-------------|---------|
| `derive` | Derive macro for configuration structs | Yes |
| `validation` | Configuration validation support | No |
| `watch` | File monitoring and hot reload | No |
| `audit` | Configuration loading audit log | No |
| `schema` | JSON Schema generation | No |
| `parallel` | Parallel validation | No |
| `monitoring` | System monitoring | No |
| `remote` | Remote configuration (etcd, Consul, HTTP) | No |
| `encryption` | Configuration encryption functionality | No |
| `cli` | Command-line tool | No |
| `full` | Enable all features | No |

**Feature Presets:**

| Preset | Included Features | Use Case |
|--------|-------------------|----------|
| `minimal` | `derive` | Configuration loading only (minimal dependencies) |
| `recommended` | `derive`, `validation` | Configuration loading + validation (recommended for most applications) |
| `dev` | `derive`, `validation`, `cli`, `schema`, `audit`, `monitoring` | Development configuration |
| `production` | `derive`, `validation`, `watch`, `encryption`, `remote`, `monitoring` | Production configuration |
| `full` | All features | Complete feature set |

---

<div align="center" style="margin: 32px 0; padding: 24px">

### 💝 Thank You for Using Confers!

If you have questions or suggestions, please visit the [GitHub Repository](https://github.com/Kirky-X/confers).

**[🏠 Back to Home](../README.md)** • **[📖 User Guide](USER_GUIDE.md)**

Made with ❤️ by Kirky.X

**[⬆ Back to Top](#top)**

</div>
