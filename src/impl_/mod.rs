//! Internal implementation module (not exposed externally).
//!
//! This directory contains all concrete implementations following BrickArchitecture.
//! Public traits are defined in `src/interface.rs`.
//!
//! ## Core Modules
//!
//! - `default` - Primary ConfigImpl implementation
//! - `memory` - InMemoryConfig using moka cache
//! - `loader` - Format loading
//! - `merger` - Merge engine
//! - `format` - Format detection and parsing
//! - `config` - ConfigBuilder, SourceChain, sources, limits
//
// Feature gates and BrickArchitecture facade pattern cause dead_code warnings
// under some feature combinations (e.g. pub items exposed via facade but only
// consumed externally). This is baseline behavior — each module is annotated
// with `#[allow(dead_code)]` to match the pre-refactor convention.
// Verify with `cargo clippy --features full -- -D warnings` after changes.
#[cfg(feature = "audit")]
#[allow(dead_code)]
pub(crate) mod audit;
#[allow(dead_code)]
pub(crate) mod config;
#[cfg(feature = "context-aware")]
#[allow(dead_code)]
pub(crate) mod context;
#[allow(dead_code)]
pub(crate) mod convert;
#[allow(dead_code)]
pub(crate) mod default;
#[cfg(feature = "dynamic")]
#[allow(dead_code)]
pub(crate) mod dynamic;
#[allow(dead_code)]
pub(crate) mod format;
#[cfg(feature = "interpolation")]
#[allow(dead_code)]
pub(crate) mod interpolation;
#[allow(dead_code)]
pub(crate) mod lifecycle;
#[allow(dead_code)]
pub(crate) mod loader;
#[allow(dead_code)]
pub(crate) mod memory;
#[allow(dead_code)]
pub(crate) mod merger;
#[cfg(feature = "migration")]
#[allow(dead_code)]
pub(crate) mod migration;
#[cfg(feature = "modules")]
#[allow(dead_code)]
pub(crate) mod modules;
#[cfg(feature = "typescript-schema")]
#[allow(dead_code)]
pub(crate) mod schema;
#[cfg(feature = "snapshot")]
#[allow(dead_code)]
pub(crate) mod snapshot;
#[cfg(feature = "validation")]
#[allow(dead_code)]
pub(crate) mod validator;
