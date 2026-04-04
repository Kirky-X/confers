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
pub mod default;

#[allow(unused)]
#[allow(dead_code)]
#[allow(unused_imports)]
pub mod format;

#[allow(unused)]
#[allow(dead_code)]
#[allow(unused_imports)]
pub mod loader;

#[allow(unused)]
#[allow(dead_code)]
#[allow(unused_imports)]
pub mod memory;

#[allow(unused)]
#[allow(dead_code)]
#[allow(unused_imports)]
pub mod merger;

pub mod source;

// ============== Feature-gated Modules ==============

#[cfg(feature = "watch")]
#[allow(unused)]
#[allow(dead_code)]
#[allow(unused_imports)]
pub mod watcher;

#[cfg(feature = "encryption")]
#[allow(unused)]
#[allow(dead_code)]
#[allow(unused_imports)]
pub mod secret;

#[cfg(feature = "config-bus")]
#[allow(unused)]
#[allow(dead_code)]
#[allow(unused_imports)]
pub mod bus;

#[cfg(feature = "remote")]
#[allow(unused)]
#[allow(dead_code)]
#[allow(unused_imports)]
pub mod remote;

#[cfg(feature = "key")]
#[allow(unused)]
#[allow(dead_code)]
#[allow(unused_imports)]
pub mod key;

#[cfg(feature = "security")]
#[allow(unused)]
#[allow(dead_code)]
#[allow(unused_imports)]
pub mod security;
