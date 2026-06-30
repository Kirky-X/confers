//! Configuration interpolation — public facade.
//!
//! Implementation lives in `crate::impl_::interpolation`.

pub use crate::impl_::interpolation::{
    interpolate, interpolate_tracked, InterpolationConfig, InterpolationContext,
    InterpolationResult, InterpolationWarning,
};
