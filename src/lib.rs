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
//! Use `new_in_memory_validated()` for BrickArchitecture-compliant initialization:
//!
//! ```rust
//! use confers::{new_in_memory_validated, ConfigConnector, ConfigConfigError};
//!
//! # fn main() -> Result<(), ConfigConfigError> {
//! let config = new_in_memory_validated(1000)?; // Returns Result, validates capacity > 0
//! # Ok(())
//! # }
//! ```

// ============== Public Modules ==============

pub mod config;
pub mod error;
pub mod format;
pub mod loader;
pub mod merger;
pub mod traits;
pub mod value;

// Internal implementation (not exposed)
mod impl_;

// ============== Feature-gated Public Modules ==============

#[cfg(feature = "validation")]
pub mod validator;

#[cfg(feature = "interpolation")]
pub mod interpolation;

#[cfg(feature = "watch")]
pub mod watcher;

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

#[cfg(feature = "cli")]
pub mod cli;

#[cfg(feature = "typescript-schema")]
pub mod schema;

#[cfg(feature = "security")]
pub mod security;

#[cfg(feature = "key")]
pub mod key;

#[cfg(feature = "remote")]
pub mod remote;

// ============== Core Re-exports ==============

pub use lifecycle::Lifecycle;

#[cfg(feature = "snapshot")]
pub use config::SnapshotConfig;
pub use config::{
    config, ConfigBuilder, ConfigLimits, DefaultSource, EnvSource, FileSource, MemorySource,
    ReloadStrategy, Source, SourceChain, SourceChainBuilder, SourceKind,
};

// Error types (BrickArchitecture compliant)
pub use error::{
    BuildResult, ConfersError, ConfersResult, ConfigConfigError, ConfigError, ConfigErrorCode,
    ConfigResult, ErrorCode, InitResult, ParseLocation, SourceWarning,
};

// Interface traits (BrickArchitecture)
pub use traits::{
    ConfigConnector, ConfigProvider, ConfigProviderExt, ConfigReader, ConfigWriter, KeyProvider,
    TypedConfigKey,
};

// Public types
pub use value::{AnnotatedValue, ConfigValue, SourceId, SourceLocation};

pub use loader::{
    detect_format_from_content, detect_format_from_path, load_file, parse_content, Format,
    LoaderConfig,
};

// Re-export derive macros
pub use macros::Config;
pub use macros::ConfigClap;
pub use macros::ConfigMigration;
pub use macros::ConfigModules;
pub use macros::ConfigSchema;

// ============== Feature-gated Re-exports ==============

#[cfg(feature = "validation")]
pub use validator::{Validate, ValidationResult, ValidationRule};

#[cfg(feature = "interpolation")]
pub use interpolation::{
    interpolate, interpolate_tracked, InterpolationConfig, InterpolationContext,
    InterpolationResult, InterpolationWarning,
};

#[cfg(feature = "watch")]
pub use watcher::{
    AdaptiveDebouncer, FsWatcher, MultiFsWatcher, WatcherConfig, WatcherConfigBuilder, WatcherGuard,
};

#[cfg(feature = "progressive-reload")]
pub use watcher::{
    HealthStatus, ProgressiveReloader, ProgressiveReloaderBuilder, ReloadHealthCheck, ReloadOutcome,
};

#[cfg(feature = "encryption")]
pub use secret::{
    crypto::CryptoError, derive_field_key, SecretBytes, SecretString, XChaCha20Crypto,
};

#[cfg(feature = "audit")]
pub use audit::{
    AuditConfig, AuditConfigBuilder, AuditEvent, AuditLevel, AuditWriter, AuditWriterBuilder,
};

#[cfg(feature = "dynamic")]
pub use dynamic::{CallbackGuard, DynamicField, DynamicFieldBuilder};

#[cfg(feature = "migration")]
pub use migration::{MigrationFn, MigrationOnReload, MigrationRegistry, Versioned};

#[cfg(feature = "snapshot")]
pub use snapshot::{SnapshotFormat, SnapshotInfo, SnapshotManager, SnapshotOptions};

#[cfg(feature = "modules")]
pub use modules::{ModuleConfig, ModuleRegistry};

#[cfg(feature = "context-aware")]
pub use context::{
    ContextAwareField, ContextAwareFieldBuilder, ContextRule, ContextValue, EvaluationContext,
};

#[cfg(feature = "config-bus")]
pub use bus::{BusBuilder, BusEventLimiter, ConfigBus, ConfigChangeEvent, InMemoryBus};

#[cfg(feature = "remote")]
pub use remote::{HttpPolledSource, HttpPolledSourceBuilder, PolledSource};

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
pub fn new_in_memory() -> impl ConfigConnector {
    impl_::memory::InMemoryConfig::new()
}

/// Create an in-memory config with custom capacity.
///
/// Note: This function does not validate capacity. For production use,
/// prefer `new_in_memory_validated()` which returns a Result.
///
/// # Example
///
/// ```rust
/// use confers::{new_in_memory_with_capacity, ConfigConnector};
///
/// let config = new_in_memory_with_capacity(1000);
/// ```
pub fn new_in_memory_with_capacity(max_capacity: u64) -> impl ConfigConnector {
    impl_::memory::InMemoryConfigBuilder::default()
        .max_capacity(max_capacity)
        .build()
}

/// Create a validated in-memory config with capacity limit.
///
/// # BrickArchitecture
///
/// This factory function returns `Result` for initialization failures,
/// following BrickArchitecture fail-fast principle.
///
/// # Errors
///
/// Returns `ConfigConfigError::InvalidValue` if capacity is 0.
///
/// # Example
///
/// ```rust
/// use confers::{new_in_memory_validated, ConfigConnector, ConfigConfigError};
///
/// # fn example() -> Result<(), ConfigConfigError> {
/// let config = new_in_memory_validated(1000)?;
/// # Ok(())
/// # }
/// ```
pub fn new_in_memory_validated(
    max_capacity: u64,
) -> Result<impl ConfigConnector, ConfigConfigError> {
    impl_::memory::InMemoryConfig::new_validated(max_capacity)
}

// ============== Prelude ==============

/// Prelude for common imports.
pub mod prelude {
    pub use crate::config::{config, ConfigBuilder, ConfigLimits};
    pub use crate::error::{
        BuildResult, ConfersError, ConfigConfigError, ConfigError, ConfigResult, ErrorCode,
    };
    pub use crate::lifecycle::Lifecycle;
    pub use crate::loader::{Format, LoaderConfig};
    pub use crate::traits::{
        ConfigConnector, ConfigProvider, ConfigProviderExt, ConfigReader, ConfigWriter,
        TypedConfigKey,
    };
    pub use crate::value::{AnnotatedValue, ConfigValue};
    pub use crate::Config;

    #[cfg(feature = "validation")]
    pub use crate::validator::Validate;

    #[cfg(feature = "interpolation")]
    pub use crate::interpolation::{interpolate, InterpolationConfig};

    #[cfg(feature = "dynamic")]
    pub use crate::dynamic::{CallbackGuard, DynamicField, DynamicFieldBuilder};
}
