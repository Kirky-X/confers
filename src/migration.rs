//! Configuration migration — public facade.
//!
//! Implementation lives in `crate::impl_::migration`.

pub use crate::impl_::migration::{MigrationFn, MigrationOnReload, MigrationRegistry, Versioned};
