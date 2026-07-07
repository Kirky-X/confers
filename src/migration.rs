// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Configuration migration — public facade.
//!
//! Implementation lives in `crate::impl_::migration`.

pub use crate::impl_::migration::{MigrationFn, MigrationOnReload, MigrationRegistry, Versioned};
