// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Validation support using garde.
//!
//! This module provides integration with the `garde` validation framework.
//! Enable the `validation` feature to use this module.
//!
//! # Usage
//!
//! Add `#[config(validate)]` to your struct and use garde's validation attributes:
//!
//! ```rust
//! use confers::Config;
//! use garde::Validate;
//! use serde::Deserialize;
//!
//! #[derive(Debug, Config, Deserialize, Validate)]
//! #[config(validate)]
//! struct AppConfig {
//!     #[garde(length(min = 1, max = 253))]
//!     host: String,
//!
//!     #[garde(range(min = 1, max = 65535))]
//!     port: u16,
//! }
//! ```

// Re-export garde's Validate trait
#[cfg(feature = "validation")]
pub use garde::Validate;

/// Validation result type alias.
#[cfg(feature = "validation")]
pub type ValidationResult = Result<(), garde::Report>;

/// Validation rules that can be applied to configuration fields.
///
/// These are used by the derive macro to generate validation code.
#[cfg(feature = "validation")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationRule {
    /// Minimum length for strings
    MinLength(usize),
    /// Maximum length for strings
    MaxLength(usize),
    /// Length range for strings
    Length { min: usize, max: usize },
    /// Minimum value for numbers
    MinValue(i64),
    /// Maximum value for numbers
    MaxValue(i64),
    /// Value range for numbers
    Range { min: i64, max: i64 },
    /// Email format
    Email,
    /// URL format
    Url,
    /// IP address format
    Ip,
    /// Custom validation function
    Custom(String),
}

#[cfg(feature = "validation")]
impl ValidationRule {
    /// Parse a validation rule from a string.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        let s = s.trim();

        // Check for length rules
        if let Some(inner) = s.strip_prefix("length(") {
            if let Some(inner) = inner.strip_suffix(')') {
                let parts: Vec<&str> = inner.split(',').map(|p| p.trim()).collect();
                let mut min = 0;
                let mut max = usize::MAX;

                for part in parts {
                    if let Some(v) = part.strip_prefix("min=") {
                        min = v.parse().ok()?;
                    } else if let Some(v) = part.strip_prefix("max=") {
                        max = v.parse().ok()?;
                    }
                }

                return Some(Self::Length { min, max });
            }
        }

        // Check for range rules
        if let Some(inner) = s.strip_prefix("range(") {
            if let Some(inner) = inner.strip_suffix(')') {
                let parts: Vec<&str> = inner.split(',').map(|p| p.trim()).collect();
                let mut min = i64::MIN;
                let mut max = i64::MAX;

                for part in parts {
                    if let Some(v) = part.strip_prefix("min=") {
                        min = v.parse().ok()?;
                    } else if let Some(v) = part.strip_prefix("max=") {
                        max = v.parse().ok()?;
                    }
                }

                return Some(Self::Range { min, max });
            }
        }

        // Check for simple rules
        match s {
            "email" => Some(Self::Email),
            "url" => Some(Self::Url),
            "ip" => Some(Self::Ip),
            _ => None,
        }
    }
}

/// No-op module when validation is disabled.
#[cfg(not(feature = "validation"))]
pub(crate) mod no_validation {
    //! Placeholder module when validation feature is disabled.

    /// Placeholder trait for Validate when validation is disabled.
    pub trait Validate {}

    /// Placeholder for validation result.
    pub type ValidationResult = Result<(), ()>;
}

#[cfg(not(feature = "validation"))]
pub use no_validation::{Validate, ValidationResult};

#[cfg(all(test, feature = "validation"))]
mod tests {
    use super::*;

    #[test]
    fn test_validation_rule_parse() {
        let rule = ValidationRule::from_str("length(min=1, max=100)");
        assert!(matches!(
            rule,
            Some(ValidationRule::Length { min: 1, max: 100 })
        ));

        let rule = ValidationRule::from_str("range(min=1, max=65535)");
        assert!(matches!(
            rule,
            Some(ValidationRule::Range { min: 1, max: 65535 })
        ));

        let rule = ValidationRule::from_str("email");
        assert!(matches!(rule, Some(ValidationRule::Email)));
    }

    #[test]
    fn test_validation_rule_length_min_only() {
        let rule = ValidationRule::from_str("length(min=5)");
        assert!(matches!(
            rule,
            Some(ValidationRule::Length { min: 5, max }) if max == usize::MAX
        ));
    }

    #[test]
    fn test_validation_rule_length_max_only() {
        let rule = ValidationRule::from_str("length(max=10)");
        assert!(matches!(
            rule,
            Some(ValidationRule::Length { min: 0, max: 10 })
        ));
    }

    #[test]
    fn test_validation_rule_length_no_close_paren() {
        let rule = ValidationRule::from_str("length(min=5, max=10");
        assert!(rule.is_none());
    }

    #[test]
    fn test_validation_rule_length_invalid_values() {
        let rule = ValidationRule::from_str("length(min=abc)");
        assert!(rule.is_none());

        let rule = ValidationRule::from_str("length(max=xyz)");
        assert!(rule.is_none());
    }

    #[test]
    fn test_validation_rule_range_min_only() {
        let rule = ValidationRule::from_str("range(min=10)");
        assert!(matches!(
            rule,
            Some(ValidationRule::Range { min: 10, max }) if max == i64::MAX
        ));
    }

    #[test]
    fn test_validation_rule_range_max_only() {
        let rule = ValidationRule::from_str("range(max=100)");
        assert!(matches!(
            rule,
            Some(ValidationRule::Range { min, max: 100 }) if min == i64::MIN
        ));
    }

    #[test]
    fn test_validation_rule_range_no_close_paren() {
        let rule = ValidationRule::from_str("range(min=1, max=100");
        assert!(rule.is_none());
    }

    #[test]
    fn test_validation_rule_range_invalid_values() {
        let rule = ValidationRule::from_str("range(min=abc)");
        assert!(rule.is_none());

        let rule = ValidationRule::from_str("range(max=xyz)");
        assert!(rule.is_none());
    }

    #[test]
    fn test_validation_rule_url_and_ip() {
        let rule = ValidationRule::from_str("url");
        assert!(matches!(rule, Some(ValidationRule::Url)));

        let rule = ValidationRule::from_str("ip");
        assert!(matches!(rule, Some(ValidationRule::Ip)));
    }

    #[test]
    fn test_validation_rule_unknown_returns_none() {
        let rule = ValidationRule::from_str("unknown_rule");
        assert!(rule.is_none());

        let rule = ValidationRule::from_str("length");
        assert!(rule.is_none());
    }

    #[test]
    fn test_validation_rule_trim_whitespace() {
        let rule = ValidationRule::from_str("  email  ");
        assert!(matches!(rule, Some(ValidationRule::Email)));

        let rule = ValidationRule::from_str("\turl\n");
        assert!(matches!(rule, Some(ValidationRule::Url)));
    }

    #[test]
    fn test_validation_rule_variants_construct_and_derives() {
        // Construct all variants to cover Debug/Clone/PartialEq/Eq derives
        let min_len = ValidationRule::MinLength(5);
        let max_len = ValidationRule::MaxLength(10);
        let min_val = ValidationRule::MinValue(0);
        let max_val = ValidationRule::MaxValue(100);
        let custom = ValidationRule::Custom("my_rule".into());

        // Debug
        assert!(format!("{:?}", min_len).contains("MinLength"));
        assert!(format!("{:?}", max_len).contains("MaxLength"));
        assert!(format!("{:?}", min_val).contains("MinValue"));
        assert!(format!("{:?}", max_val).contains("MaxValue"));
        assert!(format!("{:?}", custom).contains("Custom"));

        // Clone + Eq
        assert_eq!(min_len, min_len.clone());
        assert_eq!(max_len, max_len.clone());
        assert_eq!(min_val, min_val.clone());
        assert_eq!(max_val, max_val.clone());
        assert_eq!(custom, custom.clone());

        // Inequality
        assert_ne!(min_len, max_len);
        assert_ne!(min_val, max_val);
    }
}
