// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Format converter re-exports.
//!
//! Implementation lives in `crate::impl_::format`. This module provides
//! the public API surface for format detection and conversion.

pub use crate::impl_::format::{
    FormatConverter, FormatFeature, FormatMatch, all_converters, converter_for, detect_format,
};

// Re-export Format from loader for public API compatibility
pub use crate::loader::Format;
