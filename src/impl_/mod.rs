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
//! - `source` - Source implementations

// Allow unused - these are internal implementations that may not be
// directly used until features are enabled
#[allow(unused)]
#[allow(dead_code)]
#[allow(unused_imports)]
pub(crate) mod default;

#[allow(unused)]
#[allow(dead_code)]
#[allow(unused_imports)]
pub(crate) mod format;

#[allow(unused)]
#[allow(dead_code)]
#[allow(unused_imports)]
pub(crate) mod loader;

#[allow(unused)]
#[allow(dead_code)]
#[allow(unused_imports)]
pub(crate) mod memory;

#[allow(unused)]
#[allow(dead_code)]
#[allow(unused_imports)]
pub(crate) mod merger;

pub(crate) mod source;
