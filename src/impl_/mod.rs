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

// Internal implementations may have unused items depending on feature flags
#[allow(dead_code)]
#[allow(unused_imports)]
pub(crate) mod default;
#[allow(dead_code)]
#[allow(unused_imports)]
pub(crate) mod format;
#[allow(dead_code)]
#[allow(unused_imports)]
pub(crate) mod loader;
#[allow(dead_code)]
#[allow(unused_imports)]
pub(crate) mod memory;
#[allow(dead_code)]
#[allow(unused_imports)]
pub(crate) mod merger;
#[allow(dead_code)]
#[allow(unused_imports)]
pub(crate) mod source;
