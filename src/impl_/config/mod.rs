//! Internal configuration implementation module.
//!
//! Concrete implementations of `ConfigBuilder`, `SourceChain`, configuration sources,
//! and resource limits. Public API surface is re-exported by `src/config.rs` facade.

pub(crate) mod builder;
pub(crate) mod chain;
pub(crate) mod limits;
pub(crate) mod source;

pub use builder::{config, ConfigBuilder, ReloadStrategy};
pub use chain::{SourceChain, SourceChainBuilder};
pub use limits::ConfigLimits;
pub use source::{DefaultSource, EnvSource, FileSource, MemorySource};

#[cfg(feature = "remote")]
pub use crate::interface::AsyncSource;
