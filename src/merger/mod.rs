// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Configuration merge engine.
//!
//! Implementation lives in `crate::impl_::merger`. This module provides
//! the public API surface for configuration merge strategies.

pub use crate::impl_::merger::{CustomMergeFn, MergeEngine, MergeStrategy};
