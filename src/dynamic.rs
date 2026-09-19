// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! Dynamic configuration fields — public facade.
//!
//! Implementation lives in `crate::impl_::dynamic`.

pub use crate::impl_::dynamic::{CallbackGuard, DynamicField, DynamicFieldBuilder};

#[cfg(feature = "watch")]
pub use crate::impl_::dynamic::FieldWatcher;
