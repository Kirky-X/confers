# 📘 Confers API Reference

This reference document fully covers Confers' public APIs, including the config builder, core traits, error types, and the interfaces of each feature-gated module. The document assumes the `full` feature is enabled; with other feature combinations, some APIs may be unavailable.

## 📋 Table of Contents

- [Overview](#-overview)
- [Core API](#-core-api)
- [Configuration/Extension API](#-configurationextension-api)
- [Error Types](#-error-types)
- [Feature-Gated API](#-feature-gated-api)
- [Usage Examples](#-usage-examples)
- [Best Practices](#-best-practices)
- [Related Documents](#-related-documents)

---

## 🎯 Overview

### API Design Principles

| Principle | Description |
|:-----|:-----|
| **Simple** | Intuitive and easy to use |
| **Safe** | Type-safe by default |
| **Composable** | Easily build complex workflows |
| **Well-documented** | Fully backed by documentation |

### 📦 Feature Notes

confers provides flexible feature configuration; users can pick the capabilities they need. The feature presets (`minimal` / `recommended` / `dev` / `production` / `distributed` / `full`) and the list and default-enabled state of every single feature are maintained in [README · Feature Flags](../../README.md#-feature-flags) (the feature matrix maps item by item to the `[features]` definition in `Cargo.toml`).

---

## 🧱 Core API

### Config Builder (ConfigBuilder)

`ConfigBuilder<T>` is the core component that loads and merges configuration from multiple sources, supporting intelligent merging of config sources such as files, environment variables, and remote sources.

```mermaid
graph TB
    subgraph Sources ["Config sources"]
        A["Config files"]
        B["Environment variables"]
        C["In-memory source"]
        D["Remote sources"]
    end

    subgraph Loader ["ConfigBuilder"]
        E["Smart merge"]
        F["Validation"]
        G["Hot reload"]
    end

    subgraph Output ["Output"]
        H["Type-safe configuration"]
    end

    Sources --> Loader
    Loader --> Output
```

#### Creation and Configuration

##### `ConfigBuilder::new()`

Creates a new config builder instance.

```rust
pub fn new() -> Self
```

**Example:**

```rust
let builder = ConfigBuilder::<ConfersConfig>::new();
```

**Notes**: `ConfigBuilder` implements the `Default` trait. The `new()` method returns an instance with sensible defaults.

##### `defaults(defaults: HashMap<String, ConfigValue>)`

Sets default config values, used when no other source provides the corresponding value.

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

let builder = ConfigBuilder::<ConfersConfig>::new()
    .defaults(defaults);
```

**Notes**: defaults have the lowest priority and are overridden by other config sources.

##### `file(path: impl Into<PathBuf>)`

Adds an explicit config file. Supports multiple config files; priority increases with insertion order.

```rust
pub fn file(mut self, path: impl Into<PathBuf>) -> Self
```

**Example:**

```rust
let builder = ConfigBuilder::<ConfersConfig>::new()
    .file("config/base.toml")
    .file("config/development.toml");
```

**Notes**: files load in insertion order; later-added files have higher priority.

##### `file_optional(path: impl Into<PathBuf>)`

Adds an optional config file. A missing file is silently skipped.

```rust
pub fn file_optional(mut self, path: impl Into<PathBuf>) -> Self
```

**Example:**

```rust
let builder = ConfigBuilder::<ConfersConfig>::new()
    .file("config.toml")
    .file_optional("config.local.toml"); // may not exist
```

##### `env()`

Adds an environment variable source. Environment variables are loaded and mapped to config fields.

```rust
pub fn env(mut self) -> Self
```

**Example:**

```rust
let builder = ConfigBuilder::<ConfersConfig>::new()
    .file("config.toml")
    .env();
```

##### `env_prefix(prefix: impl Into<String>)`

Adds a prefixed environment variable source: only environment variables starting with the prefix are loaded, mapped to config fields after the prefix is stripped. The prefix is matched and stripped verbatim, with or without a trailing separator: one or more leading extra separators are consumed, so both `"APP_"` and `"APP"` map `APP_PORT` to `port` (`"APP_"` is recommended). Variables with only separators and no key part (e.g. `APP` itself when the prefix is `"APP"`) cannot map to any field and are skipped without error.

```rust
pub fn env_prefix(mut self, prefix: impl Into<String>) -> Self
```

**Example:**

```rust
let builder = ConfigBuilder::<ConfersConfig>::new()
    .env_prefix("APP_");
// Loads APP_PORT, APP_HOST, etc.
```

**Example:**

```rust
let builder = ConfigBuilder::<ConfersConfig>::new()
    .env()
    .env_prefix("APP_");
```

**Notes**: environment variables have higher priority than config files but lower than memory sources. `.env()` and `.env_prefix()` add two mutually independent sources; when both are called, `.env()` still collects all environment variables.

##### `with_snapshot(config: SnapshotConfig)` (requires the `snapshot` feature)

Enables snapshot configuration for rollback support. When a build fails (source collection failure, limit violation, pre-load validation rejection, deserialization error), the collected merged/partial state is still persisted for disaster recovery via `confers snapshot restore`.

```rust
#[cfg(feature = "snapshot")]
pub fn with_snapshot(mut self, config: SnapshotConfig) -> Self
```

`SnapshotConfig` fields (hand-written `Default` impl):

| Field | Type | Default | Description |
|------|------|--------|------|
| `dir` | `PathBuf` | `"config-snapshots"` | directory where snapshots are written |
| `max_snapshots` | `usize` | 30 | cap on retained snapshots (oldest evicted beyond the cap) |
| `format` | `SnapshotFormat` | `Toml` | snapshot serialization format |
| `include_provenance` | `bool` | `true` | whether provenance info is written with the snapshot (for `confers snapshot restore` and audit tracing) |


##### `fail_fast(enabled: bool)`

Enables or disables fail-fast mode. In fail-fast mode, any config error aborts the build immediately.

```rust
pub fn fail_fast(mut self, fail_fast: bool) -> Self
```

> **Note**: the `watch()`, `validate()` and `with_audit*()` methods were removed in earlier versions. File watching is now handled directly via `FsWatcher`/`MultiFsWatcher`; validation is performed explicitly via the `validation` feature and the `confers_validate()` generated by the `#[config(validate_helper)]` attribute (the build/load pipeline does not validate automatically; `#[config(validate)]` is kept as a no-op for compatibility); audit logging is configured via the `AuditWriter` builder.

##### `limits(limits: ConfigLimits)`

Sets config limits for resource management. The following fields are enforced in `build()`/`build_annotated()`/`build_resilient()`:

- `allowed_extensions`: file extension whitelist, validated before build (sources with an explicit `with_format` override are exempt — that is the established path for loading extension-less/unrecognized-extension files);
- `max_file_size_bytes` / `max_total_size`: per-file and total-size caps, validated before reading any source;
- `max_sources`: cap on the total number of sources;
- `allow_remote`: defaults to `false`; sources declared as `SourceKind::Remote` are rejected unless `with_allow_remote(true)` is called explicitly;
- `max_nesting_depth` / `max_total_fields` / `max_array_length` / `max_string_length`: structural validation of the merge result.

```rust
pub fn limits(mut self, limits: ConfigLimits) -> Self
```

##### `strategy(strategy: MergeStrategy)`

Sets the merge strategy between config sources.

```rust
pub fn strategy(mut self, strategy: MergeStrategy) -> Self
```

#### Remote Configuration

> ⚠️ **Note**: remote configuration requires the `remote` feature. Use the `source()` method to add remote config sources.

##### `source(source: Box<dyn Source>)`

Adds a custom config source. This is the unified method for adding remote sources.

```rust
pub fn source(mut self, source: Box<dyn Source>) -> Self
```

**Example - HTTP remote source:**

```rust
use confers::remote::HttpPolledSourceBuilder;

let http_source = HttpPolledSourceBuilder::new()
    .url("https://config-server.example.com/app-config")
    .timeout(Duration::from_secs(30))
    .build()?;

let config = ConfigBuilder::<ConfersConfig>::new()
    .source(Box::new(http_source))
    .build()?;
```

> **Note**: headers configured via `with_header(...)` (typically `Authorization: Bearer …`) are sent only on redirects to the **same origin** (scheme + host + port). Cross-origin redirects are decided by the remote server and are not trusted by the caller, so credentials are not carried.

**Example - Etcd source:**

```rust
use confers::remote::EtcdSourceBuilder;

// Note: EtcdSourceBuilder::build() is async (it establishes the gRPC connection internally)
let etcd_source = EtcdSourceBuilder::new()
    .endpoints(vec!["localhost:2379"])
    .prefix("/myapp/config")
    .build()
    .await?;

let config = ConfigBuilder::<ConfersConfig>::new()
    .source(Box::new(etcd_source))
    .build()?;
```

> **Note**: as with etcd, an empty prefix reads the entire KV space and `build()` errors; full-store reads require an explicit `allow_full_store_read(true)`.

**Example - Consul source:**

```rust
use confers::remote::ConsulSourceBuilder;

let consul_source = ConsulSourceBuilder::new()
    .address("localhost:8500")
    .prefix("myapp/config")
    .build()?;

let config = ConfigBuilder::<ConfersConfig>::new()
    .source(Box::new(consul_source))
    .build()?;
```

> **Note**: like etcd, an empty prefix reads the entire KV space and `build()` errors; full-store reads require an explicit `allow_full_store_read(true)`.

#### Build Methods

##### `build()`

Builds the configuration synchronously, merging all configured sources.

```rust
pub fn build(self) -> ConfigResult<T>
```

**Example:**

```rust
let config = builder.build()?;
```

##### `build_with_fallback(fallback: T)`

Builds with a fallback config. When the build fails, the fallback config is returned.

```rust
pub fn build_with_fallback(self, fallback: T) -> BuildResult<T>
```

**Example:**

```rust
let result = builder.build_with_fallback(ConfersConfig::default());
if result.degraded {
    println!("Using fallback: {:?}", result.degraded_reason);
}
```

##### `build_resilient()`

Resilient build that collects warnings instead of failing outright.

```rust
pub fn build_resilient(self) -> ConfigResult<BuildResult<T>>
```

##### `build_with_watcher()` (async, deprecated)

> **Deprecated** (since 0.3.0): this method does **not** perform hot reload on file changes. For full hot-reload support, use `FsWatcher`/`MultiFsWatcher` directly.

Builds with listener support. Returns the receiver of config updates and the watcher guard. Requires the `watch` feature.

```rust
#[deprecated(since = "0.3.0", note = "Does not reload on file changes. Use FsWatcher/MultiFsWatcher directly.")]
#[cfg(feature = "watch")]
pub async fn build_with_watcher(
    self,
) -> ConfigResult<(
    tokio::sync::watch::Receiver<Arc<T>>,
    WatcherGuard,
)>
```

#### Format Detection

##### `detect_format_from_content(content: &str) -> Option<Format>`

Detects the config format intelligently from file content.

```rust
pub fn detect_format_from_content(content: &str) -> Option<Format>
```

**Supported detection formats**: JSON, YAML, TOML, INI

##### `detect_format_from_path(path: &Path) -> Option<Format>`

Detects the config format from the file extension.

```rust
pub fn detect_format_from_path(path: &Path) -> Option<Format>
```

---

## 🔌 Configuration/Extension API

### ConfigProvider Trait

`ConfigProvider` is the core trait for config access, providing the base interface for reading config values. All config providers must implement this trait.

#### Trait Definition

```rust
pub trait ConfigProvider: Send + Sync {
    fn get_raw(&self, key: &str) -> Option<&AnnotatedValue>;
    fn keys(&self) -> Vec<String>;
    fn has(&self, key: &str) -> bool { ... }
}
```

#### Method Reference

##### `get_raw(&self, key: &str) -> Option<&AnnotatedValue>`

Gets the raw annotated value by key. Returns `None` when the key does not exist.

```rust
fn get_raw(&self, key: &str) -> Option<&AnnotatedValue>
```

##### `keys(&self) -> Vec<String>`

Gets all config keys in dotted format (e.g. `"database.host"`).

```rust
fn keys(&self) -> Vec<String>
```

##### `has(&self, key: &str) -> bool`

Checks whether a key exists.

```rust
fn has(&self, key: &str) -> bool
```

---

### ConfigProviderExt Trait

`ConfigProviderExt` is an extension trait providing type-safe convenience accessors. It supplies default implementations for all `ConfigProvider` types.

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

#### Method Reference

##### `get_string(&self, key: &str) -> Option<String>`

Gets a string config value.

**Example:**

```rust
let name = config.get_string("app.name");
assert_eq!(name, Some("my-app".to_string()));
```

##### `get_int(&self, key: &str) -> Option<i64>`

Gets an integer config value.

**Example:**

```rust
let port = config.get_int("server.port");
assert_eq!(port, Some(8080));
```

##### `get_uint(&self, key: &str) -> Option<u64>`

Gets an unsigned integer config value.

##### `get_bool(&self, key: &str) -> Option<bool>`

Gets a boolean config value.

**Example:**

```rust
let debug = config.get_bool("app.debug");
assert_eq!(debug, Some(true));
```

##### `get_float(&self, key: &str) -> Option<f64>`

Gets a float config value.

##### `get_typed<T>(&self, key: &str) -> ConfigResult<T>`

Gets a value of the specified type by key. Returns an error when the value cannot be converted.

```rust
fn get_typed<T>(&self, key: &str) -> ConfigResult<T>
where
    T: std::str::FromStr + Default,
    T::Err: std::fmt::Display
```

##### `get_many<'a>(&self, keys: &[&'a str]) -> HashMap<&'a str, Option<&AnnotatedValue>>`

Efficiently fetches multiple values at once. Missing keys map to `None` values.

---

### KeyProvider Trait

`KeyProvider` is the synchronous encryption key provider trait. Implementations supply keys for sensitive-field encryption.

```rust
pub trait KeyProvider: Send + Sync {
    fn get_key(&self) -> ConfigResult<ZeroizingBytes>;
    fn provider_type(&self) -> &'static str;
    fn cache_policy(&self) -> KeyCachePolicy { ... }
}
```

> **Note**: `ZeroizingBytes` carries the master key; its `Debug` impl is redacted (always `[REDACTED]`), consistent with `zeroize::Zeroizing`. Any `{:?}` formatting (logs, `unwrap()` panic text, wrapper structs with derived `Debug`) never outputs the raw key bytes.

#### KeyCachePolicy

```rust
pub enum KeyCachePolicy {
    /// Never cache keys — re-fetch on every access.
    NoCache,
    /// Cache with a time-to-live (defaults to 1 hour when constructed via [`Default`]).
    CacheWithTtl(Duration),
    /// Cache indefinitely until explicitly invalidated.
    CacheIndefinitely,
}
```

The default is `CacheWithTtl` (1-hour TTL). This policy controls token caching behavior of key providers such as Vault.

---

### TypedConfigKey

Type-safe config key binding a config path to a specific type, providing compile-time safety.

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

## 🚨 Error Types

### `ConfigConfigError` (config phase)

Config-phase errors encountered during initialization. These errors indicate fundamental problems preventing configuration loading.

| Variant | Description | Handling Advice |
|------|------|----------|
| `MissingField { field: String }` | required config field missing | supply the missing field in a config source |
| `InvalidValue { field: String, expected_type: String, message: String }` | config value invalid | check the value's type and format |
| `FileNotFound { filename: PathBuf, source: Option<std::io::Error> }` | config file not found | check that the file path is correct |
| `ParseError { format: String, message: String, location: Option<ParseLocation>, source: Option<Box<dyn Error + Send + Sync>> }` | config source parse error | check the config file syntax |
| `SizeLimitExceeded { actual: usize, limit: usize }` | config exceeds the size limit | raise the limit or slim down the config |
| `ValidationFailed { field: String, rule: String, message: String }` | validation constraint not satisfied | fix the config value to satisfy the constraint |
| `VersionMismatch { found: u32, expected: u32 }` | config version mismatch | update the config version |
| `SourceChainError { message: String, source_index: usize }` | source-chain config error | check the source configuration |
| `InterpolationError { variable: String, message: String }` | interpolation config error | check the interpolation variable syntax |
| `CircularReference { path: String }` | circular reference detected | fix the variable reference chain |

### `ConfigError` / `ConfersError` (runtime phase)

Config errors encountered at runtime. `ConfersError` is a type alias of `ConfigError`.

| Variant | Description | Handling Advice |
|------|------|----------|
| `FileNotFound { filename: PathBuf, source: Option<std::io::Error> }` | config file not found | check that the file path is correct |
| `ParseError { format: String, message: String, location: Option<ParseLocation>, source: Option<Box<dyn Error>> }` | config parse failed | check the config file syntax |
| `ValidationFailed { field: String, rule: String, message: String }` | field validation failed | check the field value's constraints |
| `SchemaValidationFailed { count: usize }` | schema validation failed | check the config against the schema |
| `DecryptionFailed { message: String }` | decryption failed | check the encryption key and data |
| `RemoteUnavailable { error_type: String, retryable: bool }` | remote source unavailable | retry if retryable, and check the network |
| `VersionMismatch { found: u32, expected: u32 }` | version mismatch | update the config version |
| `MigrationFailed { from: u32, to: u32, reason: String, source: Option<Box<dyn Error>> }` | migration failed | check the migration functions |
| `ModuleNotFound { group: String, module: String }` | module or profile not found | check the module/profile names |
| `ReloadRolledBack { reason: String }` | reload rolled back | check the health-check validators |
| `ReloadRejected { reason: String }` | progressive reload rejected by the pre-commit check (the `PreCommitCheck` of `ProgressiveReloader::with_pre_commit_check` returned `Err`) | fix the config per reason and retry the reload |
| `IoError(std::io::Error)` | IO operation error | check file permissions and disk space |
| `InvalidValue { key: String, expected_type: String, message: String }` | value of a key invalid | check the value's type and format |
| `SourceChainError { message: String, source_index: usize }` | source-chain error | check the source configuration |
| `Timeout { duration_ms: u64 }` | operation timed out | increase the timeout or check the network |
| `SizeLimitExceeded { actual: usize, limit: usize }` | file exceeds the size limit | raise the limit or slim down the file |
| `InterpolationError { variable: String, message: String }` | interpolation error | check the variable syntax |
| `KeyError { message: String }` | encryption key error | check the key configuration |
| `CircularReference { path: String }` | circular reference detected | fix the variable reference chain |
| `LockPoisoned { resource: String }` | Mutex/RwLock poisoned | retry the operation or restart the service |
| `MultiSource { source: MultiSourceError }` | multiple sources failed | check each source's errors |
| `ConcurrencyConflict { key: String, message: String, expected_type: Option<String> }` | concurrency conflict | retry the operation |
| `KeyRotationFailed { from_version: String, to_version: String, reason: String }` | key rotation failed | check the key version and permissions |
| `WatcherError { message: String, path: Option<PathBuf>, recoverable: bool }` | watcher error | check the file path and permissions |
| `OverrideBlocked { key: String, reason: String, override_source: Option<String> }` | override blocked | check the protection rules |
| `HealthCheckFailed { reason: String }` | health check failed | check the health-check configuration |

### Related Type Definitions

#### `KeyVersion`

```rust
pub struct KeyVersion {
    pub id: String,           // unique key-version identifier
    pub version: u32,         // version number
    pub created_at: u64,      // creation timestamp
    pub status: KeyStatus,    // key status
    pub algorithm: String,    // encryption algorithm
}
```

#### `KeyStatus`

```rust
pub enum KeyStatus {
    Active,       // active; usable for encrypt/decrypt
    Deprecated,   // deprecated; decrypt-only for historical data
    Compromised,  // compromised; rotate immediately
    Expired,      // expired; past its validity window
}
```

#### `KeyInfo`

```rust
pub struct KeyInfo {
    pub key_id: String,           // key-ring ID
    pub current_version: u32,     // current active version
    pub total_versions: usize,    // total number of versions
    pub active_versions: usize,   // number of active versions
    pub deprecated_versions: usize, // number of deprecated versions
    pub created_at: u64,          // creation timestamp
    pub last_rotated_at: Option<u64>, // last rotation time
}
```

#### `RotationResult`

```rust
pub struct RotationResult {
    pub key_id: String,           // key-ring ID
    pub previous_version: u32,    // version before rotation
    pub new_version: u32,         // version after rotation
    pub rotated_at: u64,          // rotation timestamp
    pub reencryption_required: bool, // whether re-encryption is required
}
```

---

## 🚪 Feature-Gated API

The following APIs are gated by Cargo features and available only after enabling the corresponding feature.

### Dynamic Fields (`dynamic` feature)

Field-level dynamic config handles: `DynamicField<T>` provides O(1) lock-free reads via `ArcSwap` (RCU mechanism) and carries highly concurrent callback registration in a `DashMap`; callback lifetime is managed by `CallbackGuard` (RAII, dropped guards auto-deregister). The three types are re-exported at the root and in the prelude (`confers::{DynamicField, DynamicFieldBuilder, CallbackGuard}`); `T` is constrained to `Clone + Send + Sync + 'static`.

| Method | Signature | Description |
|------|------|------|
| `new` | `fn new(initial: T) -> Self` | construct the handle with an initial value |
| `get` | `fn get(&self) -> T` | lock-free read returning a cloned value |
| `get_ref` | `fn get_ref(&self) -> Arc<T>` | lock-free, zero-copy read (better than `get` for large values) |
| `update` | `fn update(&self, new_val: T)` | atomic replacement and callback dispatch; re-entrant `update` inside a callback is dispatched inline (without re-entering the non-reentrant update lock), preserving the existing nested-dispatch semantics |
| `on_change` | `fn on_change(&self, f: impl Fn(&T) + Send + Sync + 'static) -> CallbackGuard` | register a change callback and return an RAII guard |
| `callback_count` | `fn callback_count(&self) -> usize` | number of currently registered callbacks |
| `callback_panic_count` | `fn callback_panic_count(&self) -> u64` | callback panic count (isolated, never propagated) |
| `builder` | `fn builder() -> DynamicFieldBuilder<T>` | builder entry point |

`DynamicFieldBuilder<T>`: `initial(value)` sets the initial value, `build()` produces the handle (`Default` is an empty builder). Calling `build()` without an initial value panics (`expect("DynamicFieldBuilder: call initial() before build()")`) — failing loudly instead of silently defaulting.

> **⚠️ `subscribe` is deprecated** (since 0.3.0, source annotation #[deprecated(note = "Use `on_change` instead")]): `DynamicField::subscribe(f)` behaves identically to `on_change(f)` (forwarding impl), kept only for backward compatibility; new code should use `on_change` uniformly.

`FieldWatcher<T: ConfigProvider>` (also requires the `watch` feature; exported from `confers::dynamic`): projects the underlying watch channel into "which fields changed" — `new(rx: watch::Receiver<Arc<T>>, fields: Vec<Arc<str>>)` is constructed by the wiring layer, `async changed_for(&mut self) -> (Arc<T>, Vec<Arc<str>>)` waits and returns the snapshot plus changed field names, `watched_fields() -> &[Arc<str>]` reads the watch list.

### Variable Interpolation (`interpolation` feature)

Free-function entry points (root re-exports `InterpolationConfig` / `InterpolationContext` / `InterpolationResult` / `InterpolationWarning`; the prelude also exports `interpolate`):

| Function | Signature | Description |
|------|------|------|
| `interpolate` | `fn interpolate<F>(template: &str, resolver: &F) -> ConfigResult<String>`, `F: Fn(&str) -> Option<String>` | interpolation with default config; returns `ConfigError::InterpolationError` when a variable is missing without a default or a circular reference is detected |
| `interpolate_tracked` | `fn interpolate_tracked<F>(...)` | same as above, and produces tracking results (actually resolved variables and the warning list) |
| `interpolate_with_config` | `fn interpolate_with_config<F>(template: &str, resolver: &F, config: &InterpolationConfig) -> ConfigResult<String>` | full config control |

`InterpolationConfig` (`new()` equals `Default`; `with_sensitive_var(name)` appends a sensitive variable name):

| Field | Type | Default | Description |
|------|------|--------|------|
| `max_depth` | `usize` | 10 | maximum recursion depth of nested interpolation |
| `allow_unresolved` | `bool` | `false` | when `true`, unresolved variables keep their literal text instead of erroring |
| `sensitive_vars` | `HashSet<String>` | empty set | list of variable names treated as sensitive |
| `warn_sensitive_interpolation` | `bool` | `true` | emits a warning when sensitive fields go through interpolation |

### OpenFeature-Style Flag Evaluation (`openfeature` feature)

Module `confers::openfeature` (the README feature table notes that it implies the evaluation surface of `feature-toggle` and `context-aware`).

| Type / Function | Description |
|------------|------|
| `FeatureProvider` (trait, `Send + Sync`) | port: `name() -> &str`, `resolve_bool(flag_key: &str, default: bool, ctx: &EvaluationContext) -> EvaluationDetail<bool>`, `resolve_string(flag_key: &str, default: &str, ctx: &EvaluationContext) -> EvaluationDetail<String>` |
| `NoOpProvider` | the spec-required empty implementation, always returning the caller's default |
| `StaticFlagProvider` | declarative flags: `new()` + `with_flag(key: impl Into<String>, flag: FlagConfig)` chained registration; attribute rules match first, then deterministic percentage bucketing by `targeting_key` |
| `ToggleRegistryProvider` | `new(Arc<FeatureToggleRegistry>)`: bridges an existing runtime toggle registry into an OpenFeature provider |
| `OpenFeatureClient` | `new()` / `with_provider(Arc<dyn FeatureProvider>)`, `set_provider(...)`, `provider_name() -> String`, `bool_value(flag_key, default, ctx)`, `bool_details(...)`, `string_value(flag_key, default, ctx)` |
| `EvaluationDetail<T>` | `value: T` / `variant: Option<String>` / `reason: ResolutionReason`; helper constructors `default_reason(value)`, `with_reason(value, variant, reason)` |
| `ResolutionReason` | `Static` (flag's static default) / `TargetingMatch` (rule hit) / `Default` (flag absent, falls back to the caller's default) / `Error` (evaluation errored, falls back) |
| `bucket_index` | `fn bucket_index(flag_key: &str, targeting_key: &str) -> u32`: stable bucketing by flag key + targeting key; repeated evaluations of the same key agree (reproducible gray release) |

`FlagConfig` (declarative flag):

| Field | Type | Description |
|------|------|------|
| `default_variant` | `String` | variant id provided when no rule matches |
| `variants` | `HashMap<String, String>` | variant id → string payload |
| `targeting` | `Vec<TargetingRule>` | evaluated in declaration order, **first match wins** |
| `percentage` | `Option<PercentageRollout>` | percentage rollout applied after targeting rules |

Construction and composition: `FlagConfig::on_off(default_on: bool)` (produces an `on`/`off` single-variant boolean flag with payloads `"true"`/`"false"`), `with_rule(TargetingRule)`, `with_rollout(PercentageRollout)`.
`TargetingRule::when_attribute(...)` internally compares `AttributeMatch { attribute, expected }` against the `EvaluationContext` (all constraints of one rule must match); `PercentageRollout { rollout: u8 (0..=100), variant_in: String, variant_off: String }`.

### Key Management (`key-management` feature)

`KeyManager` provides comprehensive encryption key management, including rotation, versioning, and key storage. Requires the `key-management` feature (implicitly enables `encryption`; the old name `key` is a compatibility alias).

`KeyRotationConfig` (`key_registry` module, injected at `KeyRegistry` construction; hand-written `Default` impl):

| Field | Type | Default | Description |
|------|------|--------|------|
| `max_key_versions` | `usize` | 3 | cap on historical versions retained per key ring (the oldest beyond the cap is evicted per rotation policy) |
| `cache_policy` | `KeyCachePolicy` | `KeyCachePolicy::default()` | key-resolution cache policy |


```mermaid
graph TB
    subgraph Storage ["Key storage"]
        A["Key rings"]
        B["Version history"]
        C["Metadata"]
    end

    subgraph Manager ["KeyManager"]
        D["Rotation management"]
        E["Version control"]
        F["Secure storage"]
    end

    subgraph Operations ["Operations"]
        G["Create"]
        H["Rotate"]
        I["Fetch"]
        J["Delete"]
    end

    Storage --> Manager
    Manager --> Operations
```

#### Creation and Management

##### `KeyManager::new()`

Creates a new key manager. Key data is encrypted with the master key; persistence is the caller's responsibility.

```rust
pub fn new() -> Result<Self, ConfigError>
```

**Example:**

```rust
let km = KeyManager::new()?;
```

##### `initialize(master_key: &[u8; 32], key_id: String, created_by: String)`

Initializes a new key ring with a master key.

```rust
pub fn initialize(
    &mut self,
    master_key: &[u8; 32],
    key_id: String,
    created_by: String,
) -> Result<KeyVersion, ConfigError>
```

**Parameters:**

| Parameter | Description |
|------|------|
| `master_key` | 32-byte master key used to encrypt key storage |
| `key_id` | unique identifier of the key ring |
| `created_by` | creator identity for audit tracing |

**Example:**

```rust
use confers::key::KeyManager;

let mut km = KeyManager::new()?;
let master_key = [0u8; 32]; // obtain from a secure location
let version = km.initialize(
    &master_key,
    "production".to_string(),
    "security-team".to_string()
)?;
```

##### `rotate_key(master_key: &[u8; 32], key_id: Option<String>, created_by: String, description: Option<String>)`

Rotates the key to a new version, supporting security-compliance key rotation.

```rust
pub fn rotate_key(
    &mut self,
    master_key: &[u8; 32],
    key_id: Option<String>,
    created_by: String,
    description: Option<String>,
) -> Result<RotationResult, ConfigError>
```

**Returns**: `RotationResult` contains the pre- and post-rotation version info and whether re-encryption is required.

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

Gets metadata and version info for the specified key.

```rust
pub fn get_key_info(&self, key_id: &str) -> Result<KeyInfo, ConfigError>
```

##### `get_key_by_version(key_id: &str, version: u32) -> Result<Option<&KeyBundle>, ConfigError>`

Gets the key data of the specified key version; returns `None` when the version does not exist.

```rust
pub fn get_key_by_version(&self, key_id: &str, version: u32) -> Result<Option<&KeyBundle>, ConfigError>
```

##### `generate_key() -> Result<[u8; 32], ConfigError>`

Generates a new 256-bit key using cryptographically secure randomness.

```rust
pub fn generate_key(&mut self) -> Result<[u8; 32], ConfigError>
```

##### `list_keys() -> Vec<KeyInfo>`

Lists metadata of all key rings.

```rust
pub fn list_keys(&self) -> Vec<KeyInfo>
```

#### Key Lifecycle Management

The `key-management` feature provides full key lifecycle management, including key creation, storage, rotation, and revocation.

```rust
use confers::key::KeyManager;

// Create the key manager
let master_key = [0u8; 32]; // obtain from a secure location
let mut manager = KeyManager::new()?;

// Initialize a new key ring
let version = manager.initialize(
    &master_key,
    "production".to_string(),
    "security-team".to_string()
)?;

// Generate a new key
let key = manager.generate_key()?;

// List all keys
let keys = manager.list_keys();

// Rotate the key
let result = manager.rotate_key(
    &master_key,
    Some("production".to_string()),
    "security-team".to_string(),
    Some("Scheduled rotation".to_string())
)?;
```

#### Method Overview

| Method | Parameters | Returns | Description |
|------|------|--------|------|
| `new()` | - | `Result<Self>` | create the key manager |
| `initialize(master_key, key_id, created_by)` | `&[u8; 32], String, String` | `Result<KeyVersion>` | initialize a new key ring |
| `generate_key()` | - | `Result<[u8; 32]>` | generate a new random key |
| `rotate_key(master_key, key_id, created_by, description)` | see above | `Result<RotationResult>` | rotate the key to a new version |
| `list_keys()` | - | `Vec<KeyInfo>` | list all key rings |
| `get_key_info(key_id)` | `&str` | `Result<KeyInfo>` | get key-ring metadata |

#### KeyRotationService

Automatic key rotation service.

```rust
use confers::key::{KeyRotationPolicy, KeyRotationService};

// Rotation policy (all fields public; adjust as needed)
let policy = KeyRotationPolicy {
    max_versions: 5,
    rotation_interval_days: 90, // rotating every 90 days is recommended
    grace_period_days: 14,
    auto_rotate: true,
    notify_before_expiry_days: 30,
};

let mut service = KeyRotationService::new(policy);

// Generate, validate, and execute the rotation (associated/instance methods; requires an initialized key ring)
// let plan = KeyRotationService::create_rotation_plan(&key_ring, 2)?;
// KeyRotationService::validate_rotation(&key_ring, &plan, &policy)?;
// let result = service.execute_rotation(
//     &mut key_ring,
//     &master_key,
//     "security-team".to_string(),
//     Some("Scheduled rotation".to_string()),
// )?;
```

#### KeyStorage

Encrypted persistent key storage.

```rust
use confers::key::KeyStorage;
use std::path::PathBuf;

let mut storage = KeyStorage::new(PathBuf::from("/path/to/keys"))?;
storage.set_master_key(&master_key); // set the master key used to encrypt/decrypt key data
storage.save()?;                     // persist keys to the storage path
storage.load()?;                     // load keys from the storage path
```

---

### Encryption Functions (`encryption` feature)

`XChaCha20Crypto` implements XChaCha20-Poly1305 encryption to protect sensitive config values, providing authenticated encryption with associated data (AEAD). Requires the `encryption` feature.

> **Historical algorithm disclosure**: AES-256-GCM has been superseded by XChaCha20-Poly1305. The error variant `CryptoError::LegacyDecryptionFailed` ("legacy decryption failed (AES-256-GCM)") remains in the enum, annotated in source as `#[deprecated(since = "0.4.2", note = "AES-256-GCM is superseded by XChaCha20-Poly1305; this variant will be removed in v0.5")]`; but **the removal was not carried out as of the `0.6.x` release line**, and no code path in `src/` constructs that variant any more (the crate's dependencies contain no AES implementation either) — the AES-256-GCM decryption path effectively no longer exists, so the original "migrate before v0.5" deadline no longer applies to users. Deployments still holding AES-256-GCM historical ciphertext should re-encrypt with the new algorithm as soon as possible (read-old-write-new suffices to migrate) and schedule the key rings whose `KeyStatus::Deprecated` (see the key status enum) for rotation.


```mermaid
graph LR
    A["Plaintext"] --> B["XChaCha20-Poly1305 encrypt"]
    B --> C["Output<br/>nonce + ciphertext"]
    C --> D["Store or transmit"]
    D --> E["Decrypt"]
    E --> F["Recover plaintext"]
```

#### Creation

##### `XChaCha20Crypto::new()`

Creates a new encryptor instance.

```rust
pub fn new() -> Self
```

**Example:**

```rust
use confers::XChaCha20Crypto;

let crypto = XChaCha20Crypto::new();
```

#### Encrypt/Decrypt Operations

##### `encrypt(plaintext: &[u8], key: &[u8]) -> Result<(Vec<u8>, Vec<u8>), CryptoError>`

Encrypts a byte sequence with a 32-byte key. Returns a `(nonce, ciphertext)` tuple.

**Features:**

- Uses the XChaCha20-Poly1305 algorithm (extended-nonce variant of ChaCha20)
- Each encryption generates a random 192-bit (24-byte) nonce
- Provides authenticated encryption with integrity verification

```rust
pub fn encrypt(&self, plaintext: &[u8], key: &[u8]) -> Result<(Vec<u8>, Vec<u8>), CryptoError>
```

**Example:**

```rust
let key = [0u8; 32]; // use a securely random key
let (nonce, ciphertext) = crypto.encrypt(b"my-secret-api-key", &key)?;
```

##### `decrypt(nonce: &[u8], ciphertext: &[u8], key: &[u8]) -> Result<Zeroizing<Vec<u8>>, CryptoError>`

Decrypts a byte sequence with the nonce, ciphertext, and 32-byte key. The result is wrapped in `Zeroizing`; the plaintext is zeroed automatically on drop.

**Features:**

- Requires the same nonce used at encryption time
- Verifies the Poly1305 authentication tag; tampering returns an error

```rust
pub fn decrypt(&self, nonce: &[u8], ciphertext: &[u8], key: &[u8]) -> Result<Zeroizing<Vec<u8>>, CryptoError>
```

**Example:**

```rust
let decrypted = crypto.decrypt(&nonce, &ciphertext, &key)?;
// Zeroizing<Vec<u8>> derefs to Vec, so as_slice() is available
assert_eq!(decrypted.as_slice(), b"my-secret-api-key");
```

#### Key Derivation

##### `derive_field_key(master_key: &[u8], field_path: &str, key_version: &str) -> Result<Zeroizing<[u8; 32]>, CryptoError>`

Derives a field-level encryption key from the master key using HKDF-SHA256. The result is wrapped in `Zeroizing`; the key material is zeroed automatically on drop.

```rust
pub fn derive_field_key(
    master_key: &[u8],
    field_path: &str,
    key_version: &str,
) -> Result<Zeroizing<[u8; 32]>, CryptoError>
```

**Example:**

```rust
use confers::derive_field_key;

let master_key = [0u8; 32];
let field_key = derive_field_key(&master_key, "database.password", "v1")?;
// field_key: Zeroizing<[u8; 32]>; derefs to [u8; 32]
```

---

### Security Module (`security` feature)

The `security` feature provides environment variable validation, error redaction, and secure injection capabilities.

#### EnvSecurityValidator

Environment variable security validator preventing injection attacks.

```rust
use confers::security::{EnvSecurityValidator, EnvironmentValidationConfig};
use std::collections::HashMap;

// Customize validation rules via EnvironmentValidationConfig
let config = EnvironmentValidationConfig::new()
    .with_custom_blocked_patterns(vec![
        r".*_SECRET$".to_string(),
        r".*_PASSWORD$".to_string(),
    ]);
let validator = EnvSecurityValidator::with_config(config);

// Validate a single environment variable name; the second argument is a value context:
// passing an encrypted value starting with "enc:" skips the sensitive-name block rules
validator.validate_env_name("APP_NAME", None)?;
validator.validate_env_name("DB_PASSWORD", None)?; // Err: hits the *_PASSWORD$ block rule

// Validate an environment variable value (length and content checks)
validator.validate_env_value("production")?;

// Batch-validate "config field name -> environment variable name" mappings
let mut mapping = HashMap::new();
mapping.insert("app_name".to_string(), "APP_NAME".to_string());
validator.validate_env_mapping(&mapping)?;
```

You can also use `EnvSecurityValidator::strict()` / `EnvSecurityValidator::lenient()` to adopt the preset strict/lenient rules directly.

#### SecurityErrorSanitizer

Redacts sensitive data in error messages (this export within the `security` module is gated by the `security` feature).

> **Note**: the masking rules of `confers::security::SecurityErrorSanitizer` **keep the first 4-8 characters of the masked value** (e.g. `pa****`) so credential kinds remain recognizable in logs; for full replacement that leaks no prefix, use `confers::error::sanitize::sanitize_error_message`. It and `confers::key::ErrorSanitizer` (master-key-based full replacement) are two unrelated types — the message sanitizer is exposed only under the single name `SecurityErrorSanitizer`; the bare name `ErrorSanitizer` is no longer exported.

```rust
use confers::security::SecurityErrorSanitizer;

let sanitizer = SecurityErrorSanitizer::default();
let clean_msg = sanitizer.sanitize(&error_msg);
```

> The five deprecated types `security::SecureLogger` / `SafeResult` / `SensitiveDataFilter` / `FilterResult` / `LogLevel` have been removed entirely (including the bare `security::ErrorSanitizer` export); use `confers::error::sanitize::sanitize_error_message` uniformly instead.

#### ConfigInjector

Secure config injector.

> **Note**: `ConfigInjector` is gated by the `security-rules` feature and publicly exported via `confers::security::ConfigInjector`. It provides runtime config injection with environment variable name validation and process-level rate limiting:

```rust
use confers::security::ConfigInjector;

let injector = ConfigInjector::new()
    .max_entries(1000)
    .with_dedicated_rate_limiter();

injector.inject("APP_PORT", "8080")?;
let port = injector.get("APP_PORT")?;
```

#### SecurityValidatorRegistry (`security-rules` feature)

The `security-rules` feature provides a standardized library of security validation rules with built-in validators and a registry; validation is never automatic — the caller must explicitly construct a registry and call `validate_all`.

```rust
use confers::interface::ConfigProvider;
use confers::security::rules::{SecurityValidator, SecurityValidatorRegistry};
use confers::types::{AnnotatedValue, ConfigValue, SourceId};
use std::collections::HashMap;

// The confers public surface ships no built-in ConfigProvider implementation;
// implement the trait yourself on the type holding the config keys/values
struct ConfigMap(HashMap<String, AnnotatedValue>);

impl ConfigProvider for ConfigMap {
    fn get_raw(&self, key: &str) -> Option<&AnnotatedValue> {
        self.0.get(key)
    }
    fn keys(&self) -> Vec<String> {
        self.0.keys().cloned().collect()
    }
}

// Use the built-in validators (JWT, CORS, SSRF, TLS)
let registry = SecurityValidatorRegistry::with_defaults();
let config = ConfigMap(HashMap::new());
let report = registry.validate_all(&config);

if !report.is_ok(false) {
    for v in &report.violations {
        eprintln!("[{:?}] {}: {}", v.severity, v.validator, v.message);
    }
}
```

**Built-in validators:**

| Validator | Category | Checks |
|--------|------|----------|
| `JwtSecretValidator` | `jwt` | key length ≥ 32 bytes, weak-secret detection |
| `CorsValidator` | `cors` | wildcard `*` detection, non-empty methods, max_age ≤ 86400s |
| `SsrfValidator` | `ssrf` | 19 blocked CIDR ranges (IPv4 + IPv6), whitelist support |
| `TlsConfigValidator` | `tls` | min_version ≥ 1.2, 12 weak cipher suites |

**Custom validators:**

```rust
use confers::security::rules::{SecurityValidator, SecurityViolation, ViolationSeverity};
use confers::interface::ConfigProvider;

struct MyValidator;

impl SecurityValidator for MyValidator {
    fn validate(&self, config: &dyn ConfigProvider) -> Result<(), Vec<SecurityViolation>> {
        // custom validation logic
        Ok(())
    }
    fn name(&self) -> &'static str { "my_validator" }
    fn category(&self) -> &'static str { "custom" }
    fn description(&self) -> &'static str { "My custom validator" }
}

let mut registry = SecurityValidatorRegistry::new();
registry.register(Box::new(MyValidator));
```

---

### Runtime Feature Toggles (`feature-toggle` feature)

The `feature-toggle` feature provides runtime feature toggles, complementing compile-time `cfg` features.

```rust
use confers::toggle::FeatureToggleRegistry;

let registry = FeatureToggleRegistry::new();

// Register and control features
registry.register("new_ui", "New UI Design", false);
registry.enable("new_ui");
assert!(registry.is_enabled("new_ui"));

// Toggle from configuration (config is your own ConfigProvider implementation,
// e.g. the ConfigMap instance in the SecurityValidatorRegistry example above)
registry.load_from_config(&config, "features");

// List all toggles
for info in registry.list() {
    println!("{}: enabled={}, desc={}", info.name, info.enabled, info.description);
}
```

> **Note**: runtime toggles complement compile-time `cfg(feature = ...)` flags. Compile-time flags control code pruning and inclusion; runtime toggles switch behavior without recompiling.

---

### Modular Configuration (`modules` feature)

The `modules` feature provides composable config-group support, allowing config module combinations to be selected at runtime. Suited to managing multiple sets of environment configs (e.g. database: mysql/postgresql, cache: redis/memory).

#### ModuleConfig

A config module holding profile paths and the active profile.

```rust
use confers::modules::ModuleConfig;
use std::path::PathBuf;

let config = ModuleConfig::new(
    "database",
    vec![
        ("mysql", PathBuf::from("conf/db/mysql.toml")),
        ("postgresql", PathBuf::from("conf/db/postgresql.toml")),
    ],
    Some("mysql"),
);
```

#### ModuleConfig Methods

| Method | Parameters | Returns | Description |
|------|------|--------|------|
| `new(name, paths, default)` | `&str, Vec<(&str, PathBuf)>, Option<&str>` | `Self` | create a new module config |
| `name()` | - | `&str` | get the module name |
| `active_profile()` | - | `&str` | get the active profile name |
| `profiles()` | - | `Vec<Arc<str>>` | get the available profile list |
| `profile_count()` | - | `usize` | get the profile count |
| `get_profile(profile)` | `&str` | `Option<&PathBuf>` | get a profile path by name |
| `has_profile(profile)` | `&str` | `bool` | check whether a profile exists |
| `set_active_profile(profile)` | `&str` | `Result<(), ConfigError>` | set the active profile (validated) |

**Example:**

```rust
use confers::modules::ModuleConfig;
use std::path::PathBuf;

let mut config = ModuleConfig::new(
    "database",
    vec![
        ("mysql", PathBuf::from("conf/db/mysql.toml")),
        ("postgresql", PathBuf::from("conf/db/postgresql.toml")),
    ],
    Some("mysql"),
);

// Access module information
assert_eq!(config.name(), "database");
assert_eq!(config.active_profile(), "mysql");
assert_eq!(config.profile_count(), 2);

// Switch profile
config.set_active_profile("postgresql")?;
assert_eq!(config.active_profile(), "postgresql");
```

#### ModuleRegistry

Registry managing config groups (modules).

```rust
use confers::modules::ModuleRegistry;
use std::path::PathBuf;

// ModuleRegistry has no new(); create it with Default or with_capacity
let mut registry = ModuleRegistry::default();
```

#### ModuleRegistry Methods

| Method | Parameters | Returns | Description |
|------|------|--------|------|
| `default()` (`Default` trait) | - | `Self` | create a new empty module registry |
| `with_capacity(capacity)` | `usize` | `Self` | create with preallocated capacity |
| `register_group(name, profiles, default)` | `&str, Vec<(&str, PathBuf)>, Option<&str>` | - | register a new config group |
| `get(name)` | `&str` | `Option<&ModuleConfig>` | get module config by name |
| `set_active_profile(name, profile)` | `&str, &str` | `Result<(), ConfigError>` | set a group's active profile |
| `get_active_profile(name)` | `&str` | `Option<Arc<str>>` | get a group's active profile name |
| `active_profiles()` | - | `HashMap<Arc<str>, Arc<str>>` | get all active profiles |
| `load_module(name, profile, config)` | `&str, &str, &LoaderConfig` | `Result<AnnotatedValue, ConfigError>` | load a module with the given profile |
| `load_active(name, config)` | `&str, &LoaderConfig` | `Result<AnnotatedValue, ConfigError>` | load with the active profile |
| `list_groups()` | - | `Vec<Arc<str>>` | list all registered group names |
| `resolve_from_env(prefix)` | `Option<&str>` | `&mut Self` | resolve the profile from environment variables |
| `validate_active_profiles()` | - | `Result<(), ConfigError>` | validate that all active-profile paths are valid |

**Example:**

```rust
use confers::modules::ModuleRegistry;
use confers::loader::LoaderConfig;
use std::path::PathBuf;

let mut registry = ModuleRegistry::default();

// Register config groups
registry.register_group(
    "database",
    vec![
        ("mysql", PathBuf::from("conf/db/mysql.toml")),
        ("postgresql", PathBuf::from("conf/db/postgresql.toml")),
    ],
    Some("mysql"),
);

registry.register_group(
    "cache",
    vec![
        ("redis", PathBuf::from("conf/cache/redis.toml")),
        ("memory", PathBuf::from("conf/cache/memory.toml")),
    ],
    Some("redis"),
);

// Load modules under the specified profile
let config = registry.load_module("database", "postgresql", &LoaderConfig::default())?;

// Or load under the active profile
let config = registry.load_active("database", &LoaderConfig::default())?;

// Switch profile
registry.set_active_profile("database", "mysql")?;

// Resolve from an environment variable (e.g. DATABASE_PROFILE=postgresql)
registry.resolve_from_env(Some(""));
```

`LoaderConfig` fields (hand-written `Default` impl; the module loading entry points `load_module` / `load_active` take a reference to it):

| Field | Type | Default | Description |
|------|------|--------|------|
| `max_size` | `usize` | 10 MiB (`10 * 1024 * 1024`) | per-file read cap; over-limit reads rejected (guards large-file DoS) |
| `allowed_base_dirs` | `Vec<PathBuf>` | `["."]` | allowed base directories; paths must resolve into one of them |
| `allow_absolute` | `bool` | `false` | whether absolute paths are allowed (off by default, a security-leaning choice) |
| `check_symlinks` | `bool` | `true` | whether symlink escapes are checked |
| `format` | `Option<Format>` | `None` | explicit format override: once set, `load_file` parses in this format instead of inferring from the extension (for files lacking extensions or with extensions outside the recognized list) |
| `redact_error_paths` | `bool` | `false` | when enabled, the `FileNotFound` error of `load_file` carries only `file_name()` instead of the full resolved path, for embedders who must not leak server filesystem structure (logs/API responses); off by default to preserve existing error-message behavior |

> There is also an internal constant `MAX_PATH_LENGTH = 4096` limiting path length (not configurable).


---

### TypeScript Schema Generation (`typescript-schema` feature)

The `typescript-schema` feature supports generating TypeScript type definitions from Rust types.

#### generate_typescript

Generates TypeScript type definitions.

```rust
use confers::schema::TypeScriptGenerator;
use schemars::JsonSchema;

// generate requires the type to implement schemars::JsonSchema
#[derive(confers::Config, JsonSchema)]
pub struct ConfersConfig {
    pub name: String,
    pub port: u16,
    pub debug: bool,
}

let ts = TypeScriptGenerator::generate::<ConfersConfig>()?;
println!("{}", ts);
```

**Output** (the generator only concatenates `export interface` / `export type`, emitting no auto-generated-marker comment):

```typescript
export interface ConfersConfig {
  name: string;
  port: number;
  debug: boolean;
}
```

---

### JSON Schema Generation (`json-schema` feature)

Config structures can generate JSON Schema via the `schemars` crate. Requires the `json-schema` feature (the old name `schema` is a compatibility alias).

To generate a Schema, the config struct needs to derive the `JsonSchema` trait:

```rust
use serde::{Deserialize, Serialize};
#[cfg(feature = "json-schema")]
use schemars::JsonSchema;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "json-schema", derive(JsonSchema))]
pub struct ConfersConfig {
    pub name: String,
    pub port: u16,
}
```

The generated Schema can be used to validate config format or generate documentation.

---

## 💡 Usage Examples

### Basic Config Loading

```rust
use confers::ConfigBuilder;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Default, Clone)]
struct ConfersConfig {
    database_url: String,
    port: u16,
    debug: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ConfigBuilder::<ConfersConfig>::new()
        .file("config.toml")
        .env()
        .env_prefix("MYAPP_")
        .build()?;

    println!("Database: {}", config.database_url);
    println!("Port: {}", config.port);
    Ok(())
}
```

### Key Rotation

```rust
use confers::key::KeyManager;

fn rotate_keys() -> Result<(), Box<dyn std::error::Error>> {
    let mut km = KeyManager::new()?;
    let master_key = load_master_key()?; // load the master key from secure storage

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

### Multi-Source Config Merging

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
    defaults.insert("port".to_string(), ConfigValue::integer(8080));
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

### Config Encryption

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

### Config Validation

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

    config.validate()?; // validate with garde
    println!("Configuration is valid");
    Ok(())
}
```

### Custom Format Parser

For config formats not supported by the standard library, implement a custom parser:

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

### Config Rollback

Implement config rollback using version history:

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

        let config = ConfigBuilder::<ConfersConfig>::new()
            .file(previous_version)
            .build()?;

        println!("Rolled back to previous configuration version");
        return Ok(());
    }

    Err("Not enough version history for rollback".into())
}
```

---

## 🌟 Best Practices

### Config Validation

Always validate config in the loading flow: enable the `validation` feature and use `#[config(validate_helper)]` to generate `confers_validate()`, then invoke it explicitly after `build()` — the build/load pipeline does not validate automatically. For complete examples and rule syntax, see [User Guide · Validation and Sanitization](USER_GUIDE.md#-validation-and-sanitization) and [Macro Guide · Validation with Garde](CONFIG_MACRO_GUIDE.md#-validation-with-garde).

### Key Management Security

⚠️ In production, always manage keys securely: obtain the master key from environment variables or secure storage, never hard-code it or commit it to version control, and rotate it periodically (recommended every 90 days). For `KeyManager` initialization and rotation usage, see [Key Management (key-management feature)](#key-management-key-management-feature) and [Usage Examples · Key Rotation](#key-rotation).
⚠️ In production, always manage keys securely: obtain the master key from environment variables or secure storage, never hard-code it or commit it to version control, and rotate it periodically (recommended every 90 days). For `KeyManager` initialization and rotation usage, see [Key Management (key-management feature)](#key-management-key-management-feature) and [Usage Examples · Key Rotation](#key-rotation).

### Hot-Reload Configuration

Use `FsWatcher` to watch config file changes and reload (requires the `watch` feature); parameters such as debouncing are tuned via `WatcherConfig::builder()`; the pause policy for consecutive reload failures is executed via `ReloadFailurePolicy::new(WatcherConfig)` (when the failure count reaches `max_consecutive_failures`, a `failure_pause_ms` pause window opens; while `pause_remaining()` returns `Some`, reloads are skipped, and the window closes automatically at expiry); for the complete watch-loop recipe, see [User Guide · File Watching and Hot Reload](USER_GUIDE.md#-file-watching-and-hot-reload). For the deprecation note on `build_with_watcher()`, see [ConfigBuilder · build_with_watcher()](#build_with_watcher-async-deprecated).

### Sensitive Data Encryption

Declare `#[config(encrypt = "xchacha20")]` on a struct field to have sensitive config values decrypted automatically at load; for `XChaCha20Crypto` encrypt/decrypt usage, see [Encryption Functions (encryption feature)](#encryption-functions-encryption-feature) and [Usage Examples · Config Encryption](#config-encryption).
Declare `#[config(encrypt = "xchacha20")]` on a struct field to have sensitive config values decrypted automatically at load; for `XChaCha20Crypto` encrypt/decrypt usage, see [Encryption Functions (encryption feature)](#encryption-functions-encryption-feature) and [Usage Examples · Config Encryption](#config-encryption).

### Performance Optimization

#### Async Loading

> 💡 **Tip**: for large configs or remote config sources, always load asynchronously:

```rust
use confers::ConfigBuilder;

fn load_config_efficiently() -> Result<(), Box<dyn std::error::Error>> {
    let start = std::time::Instant::now();

    let config = ConfigBuilder::<ConfersConfig>::new()
        .file("config.toml")
        .env()
        .build()?;

    let elapsed = start.elapsed();
    println!("Configuration loading time: {:?}", elapsed);

    Ok(())
}
```

#### Config Caching

Use in-memory caching for frequently accessed configs:

```rust
use std::sync::Arc;
use tokio::sync::RwLock;
use confers::ConfigBuilder;

struct CachedConfig {
    cache: Arc<RwLock<Option<ConfersConfig>>>,
}

impl CachedConfig {
    fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(None)),
        }
    }

    async fn get(&self) -> Result<ConfersConfig, Box<dyn std::error::Error>> {
        {
            let cached = self.cache.read().await;
            if let Some(config) = &*cached {
                return Ok(config.clone());
            }
        }

        let config = ConfigBuilder::<ConfersConfig>::new()
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

### Security Considerations

#### Sensitive Data Handling

**How to identify sensitive fields:**

- Passwords, API keys, access tokens
- Database connection strings
- Private keys, certificates
- Personally identifiable information (PII)

**How to encrypt sensitive data:**

Use `XChaCha20Crypto`'s `encrypt` / `decrypt` (returns `(nonce, ciphertext)`; the key must be exactly 32 bytes); for usage see [Encryption Functions (encryption feature)](#encryption-functions-encryption-feature) and [Usage Examples · Config Encryption](#config-encryption).
Use `XChaCha20Crypto`'s `encrypt` / `decrypt` (returns `(nonce, ciphertext)`; the key must be exactly 32 bytes); for usage see [Encryption Functions (encryption feature)](#encryption-functions-encryption-feature) and [Usage Examples · Config Encryption](#config-encryption).

**⚠️ Security tips:**

- 🔴 **Key management**: encryption keys must be stored securely and never committed to version control
- 🔴 **Key rotation**: periodic key rotation is recommended in production
- 🔴 **Key length**: the XChaCha20-Poly1305 key must be exactly 32 bytes (256 bits)
- 🔴 **Nonce storage**: the nonce must be stored alongside the ciphertext for decryption

#### `_FILE` Secret File Policy

Files pointed to by `<VAR>_FILE` are adjudicated uniformly by [`PathValidator`](https://docs.rs/confers/latest/confers/path_validator/struct.PathValidator.html) (`EnvSource` and the derive macro share the same policy):

- The path must exist and resolve to a regular file (devices, FIFOs, directories rejected)
- Sensitive system locations rejected (`/etc/shadow`, `/etc/passwd`, `/etc/gshadow`, `/etc/sudoers`, `/etc/ssh`, `/root`, etc.)
- Credential directories rejected (`.ssh`, `.aws`, `.gnupg`, `.kube`, `.gcloud`, `.env`, `.docker`, `docker`) and credential files rejected (`.netrc`, `.npmrc`, `.pypirc`, `.git-credentials`, `credentials`, etc.)
- **Extension-less dotfiles rejected** (`.secret`, `.myapp/.token`, etc.) — exactly the naming convention users employ for credentials
- Extension whitelist (`txt`/`json`/`yaml`/`yml`/`toml`/`ini`/`env`/`secret`/`key`/`pem`/`crt`)
- Read size cap [`PathValidator::MAX_SECRET_FILE_SIZE`] (10 MiB); over-limit returns `ConfigError::SizeLimitExceeded`

Both validation and reading operate on the **canonicalized path**, avoiding a symlink-swap window between validation and opening; after opening, the path is re-checked to still point to the same file (unix compares device/inode, windows compares creation time/modification time/size), rejecting reads when the final segment has been swapped for a symlink.

Prefer the one-step `PathValidator::validate_and_read_secret_file` (validate + read):

```rust
use confers::PathValidator;

let validator = PathValidator::new();
let content = validator.validate_and_read_secret_file("/run/secrets/db_password")?;
```

If step-by-step calls are needed, the argument of `read_secret_file` **must** be the canonical path returned by `validate_and_resolve`:

```rust
let canonical = validator.validate_and_resolve("/run/secrets/db_password")?;
let content = PathValidator::read_secret_file(&canonical)?;
```

#### Redaction and Error-Output Contract

The `Debug` impls of the following types are redacted; `{:?}` never outputs sensitive content:

| Type | `Debug` output |
|:-----|:-------------|
| `ZeroizingBytes` | `[REDACTED]` (master key bytes) |
| `SecretBytes` / `SecretString` | `[REDACTED]` |
| `SourceLocation` | only `source_name` / `line` / `column`, no server-side absolute paths |

Other contracts:

- The `source` of `ConfigError::ParseError` keeps only the parser's message and span, not the parsed document text (both parse paths — `impl_/loader.rs` and the public `confers::format::converter_for` — behave the same)
- `AnnotatedValue::conflict_report` and `MergeEngine::report_conflict` redact overridden values by key name; to redact by an explicit path list, use `AnnotatedValue::conflict_report_with_sensitive_paths(&low, &high, &["db.password"])`
- Snapshot redaction (`to_json_with_mode(Redacted, …)`) replaces an entire sensitive section with the string `"[REDACTED]"` (not an object) — downstream consumers parsing snapshots structurally must tolerate the node-type mismatch
- `RemoteUnavailable.error_type` is a stable label (`redis_connection_failed`, `nats_connect_failed`, `CircuitBreakerOpen`, `HTTP_503`, etc.) plus redacted detail; no endpoint addresses, response bodies, or raw driver errors
- For full redaction of free text (no prefix kept), use `confers::error::sanitize::sanitize_error_message`

```rust
let safe = confers::error::sanitize::sanitize_error_message(&raw_error);
```

#### Key Management

**How to generate secure keys:**

```rust
use rand::Rng;

// Generate a securely random key
let mut key = [0u8; 32];
let mut rng = rand::thread_rng();
rng.fill(&mut key);

// Use together with XChaCha20Crypto
let crypto = XChaCha20Crypto::new();
let (nonce, ciphertext) = crypto.encrypt(b"secret", &key)?;
```

**How to rotate keys:**

Use `KeyManager::rotate_key()` to rotate the key to a new version; for usage see [Key Management (key-management feature)](#key-management-key-management-feature) and [Usage Examples · Key Rotation](#key-rotation).
Use `KeyManager::rotate_key()` to rotate the key to a new version; for usage see [Key Management (key-management feature)](#key-management-key-management-feature) and [Usage Examples · Key Rotation](#key-rotation).

**⚠️ Security tips:**

- 🔴 **Key storage**: use a hardware security module (HSM) or a key management service
- 🔴 **Key rotation**: rotate keys every 90 days
- 🔴 **Key backup**: back up keys securely to guarantee recoverability
- 🔴 **Key-leak incident response**: rotate immediately after a leak and notify the relevant teams

#### Audit Log Configuration

**How to enable audit logging:**

```rust
use confers::audit::{AuditWriter, AuditConfig};
use std::path::PathBuf;

// Create the audit configuration
let audit_config = AuditConfig::builder()
    .log_dir(PathBuf::from("/var/log/confers"))
    .enabled(true)
    .build();

// Create the audit writer
let writer = AuditWriter::builder()
    .log_dir(PathBuf::from("/var/log/confers"))
    .enabled(true)
    .build();

// Record audit events (each method returns ConfigResult<()>; always handle errors)
writer.log_load("config.toml")?;
writer.log_key_access("database_password")?;
writer.log_decrypt("api_key", true)?;
```

**⚠️ Security tips:**

- 🔴 **Log integrity**: audit logs are HMAC-signed to protect integrity
- 🔴 **Log access control**: restrict access to audit log files (root/administrators only)
- 🔴 **Log archiving**: archive audit logs periodically to keep log files from growing unbounded
- 🔴 **Log monitoring**: monitor audit log access records to detect anomalous access

#### Production Security Configuration

**Environment variable security configuration:**

```bash
# Store secrets in environment variables
export APP_DATABASE_URL="postgres://user:password@localhost/db"  # pragma: allowlist secret
export APP_API_KEY="your-api-key"  # pragma: allowlist secret
# CONFERS_MASTER_KEY accepts a 64-hex-character 32-byte key, or a 32-byte ASCII key (base64 is not parsed)
export CONFERS_MASTER_KEY="64-hex-chars-or-32-ascii-chars"  # pragma: allowlist secret
```

**Remote config security configuration:**

```rust
// Remote configuration requires using the remote sources directly
use confers::remote::HttpPolledSourceBuilder;

let remote_source = HttpPolledSourceBuilder::new()
    .url("https://config.example.com")
    .timeout(std::time::Duration::from_secs(30))
    .build()?;

let config = ConfigBuilder::<ConfersConfig>::new()
    .source(Box::new(remote_source))
    .build()?;
```

**⚠️ Security tips:**

- 🔴 **TLS configuration**: remote config transport always uses TLS encryption
- 🔴 **Access control**: restrict access to the remote config service
- 🔴 **Least privilege**: grant only the necessary permissions
- 🔴 **Security audit**: audit production configs periodically

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

**Key management API:**

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

**Audit log API:**

```rust
/// Log configuration loading event
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
/// use confers::audit::{AuditWriter, AuditConfig};
/// use std::path::PathBuf;
///
/// let writer = AuditWriter::builder()
///     .log_dir(PathBuf::from("/var/log/confers"))
///     .enabled(true)
///     .build();
///
/// writer.log_load("config.toml");
/// ```
pub fn log_load(&self, source: &str) -> ConfigResult<()>
```

**Config validation API:**

Validation is enabled via the `validation` feature and the `confers_validate()` generated by the `#[config(validate_helper)]` attribute, invoked explicitly by the caller after `ConfigBuilder::build()` (the build/load pipeline does not validate automatically; `#[config(validate)]` is kept as a no-op for compatibility); on failure it returns `Err(String)` (a flattened garde report).

```rust
/// Configuration validation using garde derive macro
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
/// use confers::Config;
/// use garde::Validate;
/// use serde::Deserialize;
///
/// #[derive(Config, Validate, Deserialize)]
/// #[config(validate_helper)]
/// struct ServerConfig {
///     #[garde(range(min = 1, max = 65535))]
///     port: u16,
/// }
///
/// // Validation does not run automatically: the caller invokes it explicitly after build()
/// let config = ConfigBuilder::<ServerConfig>::new()
///     .file("config.toml")
///     .build()?;
/// config.confers_validate()?;
/// ```
```

### Troubleshooting

#### Common Issues

| Issue | Solution |
|------|----------|
| **Q: Config file not found?** | Check that the file path is correct; specify paths explicitly with `.file("config.toml")`, optional files with `.file_optional()` (silently skipped when absent), or plug in a custom source via `.source()`. |
| **Q: Environment variables not taking effect?** | Confirm `.env()` or `.env_prefix("PREFIX_")` was called, and check that the variable names use the correct prefix. The prefix is matched and stripped verbatim, with or without a trailing separator: one or more leading extra separators are consumed, so both `.env_prefix("APP_")` and `.env_prefix("APP")` map `APP_PORT` to `port` (`"APP_"` is recommended). |
| **Q: Encryption/decryption failing?** | Make sure encryption and decryption use the same 32-byte key; when deriving field keys with `derive_field_key`, `field_path` and `key_version` must match the encryption-time values exactly. |
| **Q: Config validation failing?** | Read the detailed validation error message and make sure config values satisfy all validation constraints. Check that field types match. |
| **Q: Remote config loading times out?** | Check network connectivity and remote service availability; configure a timeout when building the source: `HttpPolledSourceBuilder::new().url(url).timeout(Duration::from_secs(60)).build()?`. |
| **Q: Memory usage too high?** | Use `.limits(ConfigLimits { .. })` to cap file size, nesting depth, key count, etc.; slim down config files; avoid storing large binary blobs in config. |

#### Debug Logging

The confers core does not bind to a specific logging facade; errors are returned uniformly via `ConfigResult` / `ConfersError` for precise handling.

The repository's example programs use `tracing` + `tracing-subscriber` to output runtime status; pair with `RUST_LOG` to control the log level:

```bash
# Run an example at debug level
RUST_LOG=debug cargo run -p confers-examples --bin basic_usage
```

---

## 📚 Related Documents

| Document | Description |
|:-----|:-----|
| [📖 User Guide](USER_GUIDE.md) | installation, core concepts, and best practices |
| [🏗️ Architecture](ARCHITECTURE.md) | module layout and data flows |
| [🧩 Macro Guide](CONFIG_MACRO_GUIDE.md) | all attributes of `#[derive(Config)]` |
| [🔒 Security](SECURITY.md) | security mechanisms and vulnerability reporting process |
| [⚡ Performance Guide](PERFORMANCE.md) | performance design and benchmark data |

For questions or suggestions, visit the [GitHub repository](https://github.com/Kirky-X/confers).

**[🏠 Back to Home](../../README.md)**
