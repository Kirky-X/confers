// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Configuration interpolation — public facade.
//!
//! Implementation lives in `crate::impl_::interpolation`.

pub use crate::impl_::interpolation::{
    interpolate, interpolate_tracked, InterpolationConfig, InterpolationContext,
    InterpolationResult, InterpolationWarning,
};
