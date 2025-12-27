<div align="center">

# 📘 API Reference

### Complete API Documentation

[🏠 Home](../README.md) • [📖 User Guide](USER_GUIDE.md) • [🏗️ Architecture](ARCHITECTURE.md)

---

</div>

## 📋 Table of Contents

- [Overview](#overview)
- [Core API](#core-api)
  - [ConfigLoader](#configloader)
  - [Key Management](#key-management)
  - [Encryption](#encryption)
- [Error Handling](#error-handling)
- [Type Definitions](#type-definitions)
- [Examples](#examples)

---

## Overview

<div align="center">

### 🎯 API Design Principles

</div>

<table>
<tr>
<td width="25%" align="center">
<img src="https://img.icons8.com/fluency/96/000000/easy.png" width="64"><br>
<b>Simple</b><br>
Intuitive and easy to use
</td>
<td width="25%" align="center">
<img src="https://img.icons8.com/fluency/96/000000/security-checked.png" width="64"><br>
<b>Safe</b><br>
Type-safe and secure by default
</td>
<td width="25%" align="center">
<img src="https://img.icons8.com/fluency/96/000000/module.png" width="64"><br>
<b>Composable</b><br>
Build complex workflows easily
</td>
<td width="25%" align="center">
<img src="https://img.icons8.com/fluency/96/000000/documentation.png" width="64"><br>
<b>Well-documented</b><br>
Comprehensive documentation
</td>
</tr>
</table>

---

## Core API

### ConfigLoader

`ConfigLoader<T>` is the central component for loading and merging configuration from multiple sources.

#### `ConfigLoader::new()`

Create a new configuration loader instance.

```rust
pub fn new() -> Self
```

#### `with_defaults(defaults: T)`

Set default configuration values.

```rust
pub fn with_defaults(mut self, defaults: T) -> Self
```

#### `with_file(path: impl AsRef<Path>)`

Add an explicit configuration file.

```rust
pub fn with_file(mut self, path: impl AsRef<Path>) -> Self
```

#### `with_app_name(name: impl Into<String>)`

Set the application name for standard config file locations (e.g., `/etc/<app_name>/config.toml`).

```rust
pub fn with_app_name(mut self, name: impl Into<String>) -> Self
```

#### `with_env(enabled: bool)`

Enable or disable loading from environment variables.

```rust
pub fn with_env(mut self, enabled: bool) -> Self
```

#### `with_env_prefix(prefix: impl Into<String>)`

Set the prefix for environment variables (e.g., `APP_PORT`).

```rust
pub fn with_env_prefix(mut self, prefix: impl Into<String>) -> Self
```

#### `with_watch(enabled: bool)`

Enable or disable file watching for automatic configuration reloads.

```rust
pub fn with_watch(mut self, watch: bool) -> Self
```

#### `with_audit(enabled: bool)`

Enable or disable audit logging of configuration loading.

```rust
pub fn with_audit(mut self, enabled: bool) -> Self
```

#### `load()`

Load the configuration asynchronously.

```rust
pub async fn load(&self) -> Result<T, ConfigError>
```

#### `load_sync_with_audit()`

Load the configuration synchronously with audit support (requires `audit` feature).

```rust
pub fn load_sync_with_audit(&self) -> Result<T, ConfigError>
```

---

### Key Management

`KeyManager` provides comprehensive management for cryptographic keys, including rotation and versioning.

#### `KeyManager::new(storage_path: PathBuf)`

Create a new key manager with the specified storage path.

```rust
pub fn new(storage_path: PathBuf) -> Result<Self, ConfigError>
```

#### `initialize(master_key: &[u8; 32], key_id: String, created_by: String)`

Initialize a new key ring with a master key.

```rust
pub fn initialize(
    &mut self,
    master_key: &[u8; 32],
    key_id: String,
    created_by: String,
) -> Result<KeyVersion, ConfigError>
```

#### `rotate_key(master_key: &[u8; 32], key_id: Option<String>, created_by: String, description: Option<String>)`

Rotate a key ring to a new version.

```rust
pub fn rotate_key(
    &mut self,
    master_key: &[u8; 32],
    key_id: Option<String>,
    created_by: String,
    description: Option<String>,
) -> Result<RotationResult, ConfigError>
```

#### `get_key_info(key_id: &str)`

Get metadata and version information for a specific key.

```rust
pub fn get_key_info(&self, key_id: &str) -> Result<KeyInfo, ConfigError>
```

---

### Encryption

`ConfigEncryption` implements AES-256-GCM encryption for securing sensitive configuration values.

#### `ConfigEncryption::new(key_bytes: [u8; 32])`

Create a new encryptor with a 32-byte key.

```rust
pub fn new(key_bytes: [u8; 32]) -> Self
```

#### `ConfigEncryption::from_env()`

Create an encryptor using the `CONFERS_ENCRYPTION_KEY` environment variable.

```rust
pub fn from_env() -> Result<Self, ConfigError>
```

#### `encrypt(plaintext: &str)`

Encrypt a string value. Returns a formatted string: `enc:AES256GCM:<nonce>:<ciphertext>`.

```rust
pub fn encrypt(&self, plaintext: &str) -> Result<String, ConfigError>
```

#### `decrypt(encrypted_value: &str)`

Decrypt a formatted encrypted string.

```rust
pub fn decrypt(&self, encrypted_value: &str) -> Result<String, ConfigError>
```

---

## Error Handling

### `ConfigError`

Common error variants encountered during operations.

| Variant | Description |
|---------|-------------|
| `FileNotFound` | Configuration file not found at the specified path |
| `FormatDetectionFailed` | Failed to detect file format (TOML, JSON, YAML) |
| `ParseError` | Error parsing configuration content |
| `ValidationError` | Configuration failed validation checks |
| `KeyNotFound` | The requested key ID was not found |
| `KeyRotationFailed` | An error occurred during key rotation |
| `MemoryLimitExceeded` | Current memory usage exceeds the configured limit |
| `RemoteError` | Error loading configuration from remote sources (etcd, http) |

---

## Type Definitions

### `KeyVersion`

```rust
pub struct KeyVersion {
    pub id: String,
    pub version: u32,
    pub created_at: u64,
    pub status: KeyStatus,
    pub algorithm: String,
}
```

### `KeyInfo`

```rust
pub struct KeyInfo {
    pub key_id: String,
    pub current_version: u32,
    pub total_versions: usize,
    pub active_versions: usize,
    pub deprecated_versions: usize,
    pub created_at: u64,
    pub last_rotated_at: Option<u64>,
}
```

### `RotationResult`

```rust
pub struct RotationResult {
    pub key_id: String,
    pub previous_version: u32,
    pub new_version: u32,
    pub rotated_at: u64,
    pub reencryption_required: bool,
}
```

---

## Examples

### Basic Configuration Loading

```rust
use confers::ConfigLoader;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Default, Clone)]
struct AppConfig {
    database_url: String,
    port: u16,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let loader = ConfigLoader::<AppConfig>::new()
        .with_app_name("myapp")
        .with_file("config.toml")
        .with_env(true)
        .with_env_prefix("MYAPP");

    let config = loader.load().await?;
    println!("Database: {}", config.database_url);
    Ok(())
}
```

### Key Rotation

```rust
use confers::key::manager::KeyManager;
use std::path::PathBuf;

fn rotate_my_keys() -> Result<(), Box<dyn std::error::Error>> {
    let mut km = KeyManager::new(PathBuf::from("./keys"))?;
    let master_key = [0u8; 32]; // In production, load this securely
    
    let result = km.rotate_key(
        &master_key,
        Some("default".to_string()),
        "admin".to_string(),
        Some("Scheduled rotation".to_string())
    )?;
    
    println!("Rotated key to version: {}", result.new_version);
    Ok(())
}
```
