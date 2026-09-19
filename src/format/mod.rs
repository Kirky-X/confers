// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! Format converter re-exports.
//!
//! Implementation lives in `crate::impl_::format`. This module provides
//! the public API surface for format detection and conversion.

pub use crate::impl_::format::{
    FormatConverter, FormatFeature, FormatMatch, all_converters, converter_for, detect_format,
};

// Re-export Format from loader for public API compatibility
pub use crate::loader::Format;
