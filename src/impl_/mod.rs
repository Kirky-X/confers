//! Internal implementation module (not exposed externally).
//!
//! This directory contains all concrete implementations following BrickArchitecture.
//! Public traits are defined in `src/traits.rs`.
//!
//! ## Core Modules
//!
//! - `default` - Primary ConfigImpl implementation
//! - `memory` - InMemoryConfig using moka cache
//! - `loader` - Format loading
//! - `merger` - Merge engine
//! - `format` - Format detection and parsing
//! - `source` - Source implementations

// Feature gates cause dead_code warnings under some feature combinations.
// Each module is annotated only where needed; verify with `cargo clippy --features full`.
#[allow(dead_code)]
pub(crate) mod default;
#[allow(dead_code)]
pub(crate) mod format;
#[allow(dead_code)]
pub(crate) mod loader;
#[allow(dead_code)]
pub(crate) mod memory;
#[allow(dead_code)]
pub(crate) mod merger;
#[allow(dead_code)]
pub(crate) mod source;
