//! Configuration loading and building.
//!
//! This module provides the core configuration building functionality:
//! - `ConfigBuilder` - Fluent API for building configuration
//! - `SourceChain` - Chain of configuration sources
//! - `Source` trait - Abstraction for configuration sources
//! - `ConfigLimits` - Safety and resource limits

mod builder;
mod chain;
mod limits;
mod source;

pub use builder::{config, ConfigBuilder, ReloadStrategy, SnapshotConfig};
pub use chain::{SourceChain, SourceChainBuilder};
pub use limits::ConfigLimits;
pub use source::{DefaultSource, EnvSource, FileSource, MemorySource, Source, SourceKind};

// Re-export commonly used types
#[doc(hidden)]
pub use crate::value::ConfigValue;
