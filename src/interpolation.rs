// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! Configuration interpolation — public facade.
//!
//! Implementation lives in `crate::impl_::interpolation`.

pub use crate::impl_::interpolation::{
    InterpolationConfig, InterpolationContext, InterpolationResult, InterpolationWarning,
    interpolate, interpolate_tracked, interpolate_with_config,
};
