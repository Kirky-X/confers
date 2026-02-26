//! Validation support using garde.
//!
//! This module provides integration with the `garde` validation framework.
//! Enable the `validation` feature to use this module.
//!
//! # Usage
//!
//! Add `#[config(validate)]` to your struct and use garde's validation attributes:
//!
//! ```ignore
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
    pub fn from_str(s: &str) -> Option<Self> {
        let s = s.trim();
        
        // Check for length rules
        if s.starts_with("length(") && s.ends_with(')') {
            let inner = &s[7..s.len()-1];
            let parts: Vec<&str> = inner.split(',').map(|p| p.trim()).collect();
            let mut min = 0;
            let mut max = usize::MAX;
            
            for part in parts {
                if part.starts_with("min=") {
                    min = part[4..].parse().ok()?;
                } else if part.starts_with("max=") {
                    max = part[4..].parse().ok()?;
                }
            }
            
            return Some(Self::Length { min, max });
        }
        
        // Check for range rules
        if s.starts_with("range(") && s.ends_with(')') {
            let inner = &s[6..s.len()-1];
            let parts: Vec<&str> = inner.split(',').map(|p| p.trim()).collect();
            let mut min = i64::MIN;
            let mut max = i64::MAX;
            
            for part in parts {
                if part.starts_with("min=") {
                    min = part[4..].parse().ok()?;
                } else if part.starts_with("max=") {
                    max = part[4..].parse().ok()?;
                }
            }
            
            return Some(Self::Range { min, max });
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
pub mod no_validation {
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
        assert!(matches!(rule, Some(ValidationRule::Length { min: 1, max: 100 })));
        
        let rule = ValidationRule::from_str("range(min=1, max=65535)");
        assert!(matches!(rule, Some(ValidationRule::Range { min: 1, max: 65535 })));
        
        let rule = ValidationRule::from_str("email");
        assert!(matches!(rule, Some(ValidationRule::Email)));
    }
}
