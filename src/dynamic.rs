// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Dynamic configuration fields — public facade.
//!
//! Implementation lives in `crate::impl_::dynamic`.

pub use crate::impl_::dynamic::{CallbackGuard, DynamicField, DynamicFieldBuilder};

#[cfg(feature = "watch")]
pub use crate::impl_::dynamic::FieldWatcher;
