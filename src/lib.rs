//! Confers - Production-ready Rust configuration library.
//!
//! A zero-boilerplate configuration library with:
//! - Derive macro driven configuration loading
//! - Multi-source with priority chain
//! - Hot reload with progressive deployment
//! - Encryption for sensitive fields
//! - Type-safe configuration keys

pub mod config;
pub mod error;
pub mod format;
pub mod loader;
pub mod merger;
pub mod parser;
pub mod traits;
pub mod value;

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

#[cfg(feature = "typescript-schema")]
pub mod schema;

#[cfg(feature = "security")]
pub mod security;

#[cfg(feature = "key")]
pub mod key;

#[cfg(feature = "remote")]
pub mod remote;

#[cfg(feature = "snapshot")]
pub use config::SnapshotConfig;
pub use config::{
    config, ConfigBuilder, ConfigLimits, DefaultSource, EnvSource, FileSource, MemorySource,
    ReloadStrategy, Source, SourceChain, SourceChainBuilder, SourceKind,
};
pub use error::{BuildResult, BuildWarning, ConfigError, ConfigResult, ErrorCode, ParseLocation};
pub use loader::{
    detect_format_from_content, detect_format_from_path, load_file, parse_content, Format,
    LoaderConfig,
};
pub use traits::{ConfigProvider, ConfigProviderExt, KeyProvider, TypedConfigKey};
pub use value::{AnnotatedValue, ConfigValue, SourceId, SourceLocation};

// Re-export derive macros
pub use confers_macros::Config;
pub use confers_macros::ConfigClap;
pub use confers_macros::ConfigMigration;
pub use confers_macros::ConfigModules;
pub use confers_macros::ConfigSchema;

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

/// Prelude for common imports.
pub mod prelude {
    pub use crate::config::{config, ConfigBuilder, ConfigLimits};
    pub use crate::error::{BuildResult, ConfigError, ConfigResult, ErrorCode};
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
