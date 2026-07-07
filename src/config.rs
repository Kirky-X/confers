// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Configuration loading and building — public facade.
//!
//! Implementation lives in `crate::impl_::config`. This module provides
//! the public API surface for configuration building, source chains,
//! and resource limits.

pub use crate::impl_::config::{
    config, ConfigBuilder, ConfigLimits, DefaultSource, EnvSource, FileSource, MemorySource,
    ReloadStrategy, SourceChain, SourceChainBuilder,
};
pub use crate::interface::Source;
pub use crate::types::SourceKind;

#[cfg(feature = "remote")]
pub use crate::impl_::config::AsyncSource;

#[cfg(feature = "snapshot")]
pub use crate::snapshot::SnapshotConfig;
