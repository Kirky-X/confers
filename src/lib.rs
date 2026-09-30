// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! Confers - Production-ready Rust configuration library.
//!
//! A zero-boilerplate configuration library following BrickArchitecture:
//! - Derive macro driven configuration loading
//! - Multi-source with priority chain
//! - Hot reload with progressive deployment
//! - Encryption for sensitive fields
//! - Type-safe configuration keys
//!
//! # Quick Start
//!
//! ```ignore
//! use confers::{new_in_memory, ConfigConnector, ConfigReader, ConfigWriter, ConfigValue, AnnotatedValue, SourceId};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create in-memory config (for testing)
//! let config = new_in_memory();
//!
//! // Use the config
//! let value = AnnotatedValue::new(ConfigValue::string("value"), SourceId::default(), "key");
//! config.set("key", value).await?;
//! let str_value = config.get_string("key").await?;
//!
//! // Lifecycle methods
//! config.health_check().await?;
//! config.shutdown().await;
//! # Ok(())
//! # }
//! ```
//!
//! # BrickArchitecture Error Separation
//!
//! This library separates configuration phase errors from runtime errors:
//!
//! - **`ConfigConfigError`** — Initialization errors (missing fields, parse errors, validation failures)
//! - **`ConfersError`** — Runtime errors (timeout, remote unavailable, decryption failures)
//!
//! Use `new_in_memory()` for the canonical in-memory configuration factory:
//!
//! ```rust
//! use confers::{new_in_memory, interface::ConfigConnector};
//!
//! let config = new_in_memory();
//! ```
pub mod config;
pub mod error;
pub mod flatten;
pub mod format;
// ICU + Fluent internationalization (en/zh; core capability, no feature flag).
pub mod i18n;
pub mod interface;
pub mod loader;
pub mod merger;
pub mod metrics;

// OpenFeature-style flag evaluation (openfeature feature).
#[cfg(feature = "openfeature")]
pub mod openfeature;

// Lazy segmented parsing for oversized documents (lazy-parse feature).
#[cfg(feature = "lazy-parse")]
pub mod lazy;
pub mod tree_transform;

// Internal tracing facade (no-op without the `tracing` feature).
mod telemetry;
pub mod types;

// JSON value re-export: generated code (and users writing `map_json`
// transformations) reference value types without needing their own
// serde_json dependency.
pub use serde_json as json;

// Internal implementation (not exposed)
mod impl_;

// ============== Feature-gated Public Modules ==============

#[cfg(feature = "validation")]
pub mod validator;

#[cfg(feature = "interpolation")]
pub mod interpolation;

#[cfg(feature = "watch")]
pub mod watcher;

pub mod envelope;

pub mod field_crypto;

#[cfg(any(feature = "remote", feature = "security-rules"))]
pub mod ip_blocklist;

pub mod path_validator;

pub mod sensitive_names;

pub use envelope::EncryptedEnvelope;
pub use field_crypto::decrypt_encrypted_tree;
pub use path_validator::PathValidator;

#[cfg(feature = "encryption")]
pub mod secret;

pub mod lifecycle;

#[cfg(feature = "audit")]
pub mod audit;

#[cfg(feature = "dynamic")]
pub mod dynamic;

#[cfg(feature = "migration")]
pub mod migration;

#[cfg(feature = "snapshot")]
pub mod snapshot;

#[cfg(feature = "modules")]
pub mod modules;

#[cfg(feature = "context-aware")]
pub mod context;

#[cfg(feature = "config-bus")]
pub mod bus;

#[cfg(feature = "change-stream")]
pub mod stream;

#[cfg(feature = "canary")]
pub mod canary;

#[cfg(feature = "cli")]
pub mod cli;

#[cfg(feature = "json-schema")]
pub mod schema;

#[cfg(feature = "security")]
pub mod security;

#[cfg(feature = "key-management")]
pub mod key;

#[cfg(feature = "feature-toggle")]
pub mod toggle;

#[cfg(feature = "remote")]
pub mod remote;

// 远程源公开 trait（如 WatchEventSource）以 async-trait 展开，消费者实现
// 这些 trait 必须使用同一宏；此处 re-export 即官方实现路径。
#[cfg(feature = "remote")]
pub use async_trait::async_trait;

// ============== Core Re-exports ==============

pub use lifecycle::Lifecycle;

#[cfg(feature = "snapshot")]
pub use config::SnapshotConfig;
pub use config::{
    ConfigBuilder, ConfigLimits, DefaultSource, EnvSource, FileSource, MemorySource, Source,
    SourceChain, SourceChainBuilder, SourceKind, config,
};

// Error types (BrickArchitecture compliant)
pub use error::{
    BuildResult, ConfersError, ConfersResult, ConfigConfigError, ConfigError, ConfigErrorCode,
    ConfigResult, ErrorCode, InitResult, ParseLocation, SourceWarning,
};

// Interface traits (BrickArchitecture)
pub use interface::{
    ConfigConnector, ConfigProvider, ConfigProviderExt, ConfigReader, ConfigWriter, KeyProvider,
    TypedConfigKey,
};

// Public types
pub use types::{
    AnnotatedValue, ConfigValue, KeyCachePolicy, NoOpMetrics, SourceId, SourceLocation,
    ZeroizingBytes,
};

pub use loader::{
    Format, LoaderConfig, detect_format_from_content, detect_format_from_path, load_file,
    parse_content,
};

pub use flatten::{ConfigFieldKeys, FlattenSpec, hoist_flattened};
pub use tree_transform::{interpolate_keys, interpolate_keys_with_sensitivity, rename_tree_keys};

// Re-export derive macros (feature-gated to match their generated code dependencies)
pub use confers_macros::Config;
#[cfg(feature = "cli")]
pub use confers_macros::ConfigClap;
#[cfg(feature = "migration")]
pub use confers_macros::ConfigMigration;
#[cfg(feature = "modules")]
pub use confers_macros::ConfigModules;
#[cfg(feature = "json-schema")]
pub use confers_macros::ConfigSchema;

// ============== Feature-gated Re-exports ==============

#[cfg(feature = "validation")]
pub use validator::{Validate, ValidationResult, ValidationRule};

#[cfg(feature = "interpolation")]
pub use interpolation::{
    InterpolationConfig, InterpolationContext, InterpolationResult, InterpolationWarning,
    interpolate, interpolate_tracked,
};

#[cfg(feature = "watch")]
pub use watcher::{
    AdaptiveDebouncer, FsWatcher, MultiFsWatcher, ReloadFailurePolicy, WatcherConfig,
    WatcherConfigBuilder, WatcherGuard,
};

#[cfg(feature = "progressive-reload")]
pub use watcher::{
    HealthStatus, PreCommitCheck, ProgressiveReloader, ProgressiveReloaderBuilder,
    ReloadHealthCheck, ReloadOutcome,
};

#[cfg(feature = "hot-reload-kit")]
pub use watcher::{HotReloadLoader, HotReloader};

#[cfg(feature = "encryption")]
pub use secret::{
    SecretBytes, SecretString, XChaCha20Crypto, crypto::CryptoError, derive_field_key,
};

#[cfg(feature = "audit")]
pub use audit::{
    AuditConfig, AuditConfigBuilder, AuditEvent, AuditLevel, AuditSink, AuditWriter,
    AuditWriterBuilder, verify_audit_chain,
};

#[cfg(feature = "dynamic")]
pub use dynamic::{CallbackGuard, DynamicField, DynamicFieldBuilder};

#[cfg(feature = "migration")]
pub use migration::{MigrationFn, MigrationOnReload, MigrationRegistry, Versioned};

#[cfg(feature = "snapshot")]
pub use snapshot::{SnapshotFormat, SnapshotInfo, SnapshotManager};

#[cfg(feature = "modules")]
pub use modules::{ModuleConfig, ModuleRegistry};

#[cfg(feature = "context-aware")]
pub use context::{
    ContextAwareField, ContextAwareFieldBuilder, ContextRule, ContextValue, EvaluationContext,
};

#[cfg(feature = "config-bus")]
pub use bus::{BusBuilder, BusEventLimiter, ConfigBus, ConfigChangeEvent, InMemoryBus};

#[cfg(feature = "change-stream")]
pub use stream::{ChangeEvent, ChangeSource, ChangeStream, InMemoryChangeStream};

#[cfg(feature = "canary")]
pub use canary::{
    BASELINE_GROUP, CANARY_GROUP, CanaryOrchestrator, MeshWeightPublisher, ORCHESTRATOR_KEY,
    RolloutHealthCheck, RolloutOutcome, RolloutPlan,
};

#[cfg(feature = "remote")]
pub use remote::{HttpPolledSource, HttpPolledSourceBuilder, PolledSource};

#[cfg(feature = "security-rules")]
pub use security::rules::{
    CorsValidator, JwtSecretValidator, RemappedConfigProvider, SecurityReport, SecurityValidator,
    SecurityValidatorRegistry, SecurityViolation, SsrfValidator, TlsConfigValidator,
    ViolationSeverity,
};

#[cfg(feature = "feature-toggle")]
pub use toggle::{FeatureInfo, FeatureToggle, FeatureToggleRegistry};

// ============== Factory Functions (BrickArchitecture) ==============

/// Create an in-memory configuration store.
///
/// This is the simplest way to create a configuration instance,
/// ideal for testing and prototyping.
///
/// # Example
///
/// ```ignore
/// use confers::{new_in_memory, ConfigConnector, ConfigReader, ConfigWriter, ConfigValue, AnnotatedValue, SourceId};
///
/// # async fn example() -> Result<(), confers::ConfersError> {
/// let config = new_in_memory();
///
/// let value = AnnotatedValue::new(ConfigValue::string("value"), SourceId::default(), "key");
/// config.set("key", value).await?;
/// let str_value = config.get_string("key").await?;
/// assert_eq!(str_value, Some("value".to_string()));
/// # Ok(())
/// # }
/// ```
#[cfg(feature = "async-core")]
pub fn new_in_memory() -> impl ConfigConnector + impl_::memory::SharedValueReader {
    impl_::memory::InMemoryConfig::new()
}

#[cfg(not(feature = "async-core"))]
pub fn new_in_memory() -> impl ConfigConnector {
    impl_::memory::InMemoryConfig::new()
}

#[cfg(feature = "async-core")]
pub use impl_::memory::SharedValueReader;

// ============== Prelude ==============

/// Prelude for common imports.
pub mod prelude {
    pub use crate::Config;
    pub use crate::config::{ConfigBuilder, ConfigLimits, config};
    pub use crate::error::{
        BuildResult, ConfersError, ConfigConfigError, ConfigError, ConfigResult, ErrorCode,
    };
    pub use crate::interface::{
        ConfigConnector, ConfigProvider, ConfigProviderExt, ConfigReader, ConfigWriter,
        TypedConfigKey,
    };
    pub use crate::lifecycle::Lifecycle;
    pub use crate::loader::{Format, LoaderConfig};
    pub use crate::types::{AnnotatedValue, ConfigValue};

    #[cfg(feature = "validation")]
    pub use crate::validator::Validate;

    #[cfg(feature = "interpolation")]
    pub use crate::interpolation::{InterpolationConfig, interpolate};

    #[cfg(feature = "dynamic")]
    pub use crate::dynamic::{CallbackGuard, DynamicField, DynamicFieldBuilder};

    #[cfg(feature = "security-rules")]
    pub use crate::security::rules::{
        SecurityReport, SecurityValidator, SecurityValidatorRegistry, SecurityViolation,
        ViolationSeverity,
    };

    #[cfg(feature = "feature-toggle")]
    pub use crate::toggle::{FeatureInfo, FeatureToggle, FeatureToggleRegistry};
}
