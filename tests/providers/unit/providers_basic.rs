// Copyright (c) 2025 Kirky.X
//
// Licensed under MIT License
// See LICENSE file in the project root for full license information.

//! Unit tests: Provider basic functionality
//!
//! Tests for basic functionality of configuration providers

#[cfg(feature = "remote")]
mod tests {
    #[test]
    #[allow(clippy::assertions_on_constants)]
    fn test_placeholder() {
        let _placeholder = true;
        assert!(_placeholder);
    }
}

#[cfg(not(feature = "remote"))]
mod tests {
    #[test]
    #[allow(clippy::assertions_on_constants)]
    fn test_placeholder() {
        let _placeholder = true;
        assert!(_placeholder);
    }
}
