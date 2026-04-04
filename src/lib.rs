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
//! ```no_run
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

// ============== Public Modules ==============

pub mod config;
pub mod error;
pub mod format;
pub mod interface;
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

#[cfg(feature = "snapshot")]
pub use config::SnapshotConfig;
pub use config::{
    config, ConfigBuilder, ConfigLimits, DefaultSource, EnvSource, FileSource, MemorySource,
    ReloadStrategy, Source, SourceChain, SourceChainBuilder, SourceKind,
};

// Error types (BrickArchitecture compliant)
pub use error::{
    BuildResult, BuildWarning, ConfersError, ConfersResult, ConfigError, ConfigResult, ErrorCode,
    ParseLocation,
};

// Interface traits (BrickArchitecture)
pub use interface::{ConfigConnector, ConfigReader, ConfigWriter};

// Public types
pub use value::{AnnotatedValue, ConfigValue, SourceId, SourceLocation};

pub use loader::{
    detect_format_from_content, detect_format_from_path, load_file, parse_content, Format,
    LoaderConfig,
};
pub use traits::{ConfigProvider, ConfigProviderExt, KeyProvider, TypedConfigKey};

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
pub use bus::{BusBuilder, ConfigBus, ConfigChangeEvent, InMemoryBus};

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
/// ```rust
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

// ============== Prelude ==============

/// Prelude for common imports.
pub mod prelude {
    pub use crate::config::{config, ConfigBuilder, ConfigLimits};
    pub use crate::error::{BuildResult, ConfigError, ConfigResult, ErrorCode};
    pub use crate::interface::{ConfigConnector, ConfigReader, ConfigWriter};
    pub use crate::loader::{Format, LoaderConfig};
    pub use crate::traits::{ConfigProvider, ConfigProviderExt, TypedConfigKey};
    pub use crate::value::{AnnotatedValue, ConfigValue};
    pub use crate::Config;

    #[cfg(feature = "validation")]
    pub use crate::validator::Validate;

    #[cfg(feature = "interpolation")]
    pub use crate::interpolation::{interpolate, InterpolationConfig};

    #[cfg(feature = "dynamic")]
    pub use crate::dynamic::{CallbackGuard, DynamicField, DynamicFieldBuilder};
}
