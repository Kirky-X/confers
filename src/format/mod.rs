//! Format converter re-exports.
//!
//! Implementation lives in `crate::impl_::format`. This module provides
//! the public API surface for format detection and conversion.

pub use crate::impl_::format::{
    all_converters, converter_for, detect_format, FormatConverter, FormatFeature, FormatMatch,
};

// Re-export Format from loader for public API compatibility
pub use crate::loader::Format;
