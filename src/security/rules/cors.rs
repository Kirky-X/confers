// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! CORS configuration validator.

use super::{SecurityValidator, SecurityViolation, ViolationSeverity};
use crate::interface::ConfigProvider;

/// Maximum reasonable value for `cors.max_age` in seconds (24 hours).
const MAX_AGE_LIMIT: u64 = 86400;

/// Validates CORS configuration for security issues.
///
/// Checks:
/// - `cors.allowed_origins` does not contain wildcard `*`
/// - `cors.allowed_methods` is non-empty
/// - `cors.max_age` does not exceed 86400 seconds
pub struct CorsValidator;

impl CorsValidator {
    /// Create a new CORS validator.
    pub fn new() -> Self {
        Self
    }
}

impl Default for CorsValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl SecurityValidator for CorsValidator {
    fn validate(&self, config: &dyn ConfigProvider) -> Result<(), Vec<SecurityViolation>> {
        let mut violations = Vec::new();

        // If no CORS config at all, skip validation
        let has_origins = config.get_raw("cors.allowed_origins").is_some();
        let has_methods = config.get_raw("cors.allowed_methods").is_some();
        let has_max_age = config.get_raw("cors.max_age").is_some();

        if !has_origins && !has_methods && !has_max_age {
            return Ok(());
        }

        // Check allowed_origins for wildcard
        if let Some(value) = config.get_raw("cors.allowed_origins") {
            #[allow(deprecated)]
            if let Some(origins_str) = value.as_string() {
                if origins_str.contains('*') {
                    violations.push(SecurityViolation {
                        validator: self.name().to_string(),
                        field: Some("cors.allowed_origins".to_string()),
                        message: "CORS allowed_origins contains wildcard '*' — any origin can access the API".to_string(),
                        severity: ViolationSeverity::Warning,
                    });
                }
            }
        }

        // Check allowed_methods is non-empty
        if let Some(value) = config.get_raw("cors.allowed_methods") {
            #[allow(deprecated)]
            if let Some(methods_str) = value.as_string() {
                let trimmed = methods_str.trim();
                if trimmed.is_empty() || trimmed == "[]" {
                    violations.push(SecurityViolation {
                        validator: self.name().to_string(),
                        field: Some("cors.allowed_methods".to_string()),
                        message: "CORS allowed_methods is empty — no HTTP methods are allowed"
                            .to_string(),
                        severity: ViolationSeverity::Critical,
                    });
                }
            }
        } else if has_origins {
            // Origins configured but no methods — likely misconfiguration
            violations.push(SecurityViolation {
                validator: self.name().to_string(),
                field: Some("cors.allowed_methods".to_string()),
                message: "CORS allowed_origins configured but allowed_methods is missing".to_string(),
                severity: ViolationSeverity::Critical,
            });
        }

        // Check max_age
        if let Some(value) = config.get_raw("cors.max_age") {
            #[allow(deprecated)]
            if let Some(age_str) = value.as_string() {
                if let Ok(age) = age_str.parse::<u64>() {
                    if age > MAX_AGE_LIMIT {
                        violations.push(SecurityViolation {
                            validator: self.name().to_string(),
                            field: Some("cors.max_age".to_string()),
                            message: format!(
                                "CORS max_age {}s exceeds recommended maximum of {}s (24h)",
                                age, MAX_AGE_LIMIT
                            ),
                            severity: ViolationSeverity::Warning,
                        });
                    }
                }
            }
            #[allow(deprecated)]
            if let Some(age) = value.as_i64() {
                if age as u64 > MAX_AGE_LIMIT {
                    violations.push(SecurityViolation {
                        validator: self.name().to_string(),
                        field: Some("cors.max_age".to_string()),
                        message: format!(
                            "CORS max_age {}s exceeds recommended maximum of {}s (24h)",
                            age, MAX_AGE_LIMIT
                        ),
                        severity: ViolationSeverity::Warning,
                    });
                }
            }
        }

        if violations.is_empty() {
            Ok(())
        } else {
            Err(violations)
        }
    }

    fn name(&self) -> &'static str {
        "cors"
    }

    fn category(&self) -> &'static str {
        "network"
    }

    fn description(&self) -> &'static str {
        "Validates CORS configuration does not use wildcard origins or have empty methods"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{AnnotatedValue, ConfigValue, SourceId};
    use std::collections::HashMap;

    struct TestProvider(HashMap<String, AnnotatedValue>);

    impl TestProvider {
        fn new() -> Self {
            Self(HashMap::new())
        }

        fn with_value(mut self, key: &str, value: &str) -> Self {
            self.0.insert(
                key.to_string(),
                AnnotatedValue::new(ConfigValue::string(value), SourceId::new("test"), key),
            );
            self
        }
    }

    impl ConfigProvider for TestProvider {
        fn get_raw(&self, key: &str) -> Option<&AnnotatedValue> {
            self.0.get(key)
        }

        fn keys(&self) -> Vec<String> {
            self.0.keys().cloned().collect()
        }
    }

    #[test]
    fn test_no_cors_config_skips() {
        let validator = CorsValidator::new();
        let config = TestProvider::new();
        assert!(validator.validate(&config).is_ok());
    }

    #[test]
    fn test_wildcard_origin_warning() {
        let validator = CorsValidator::new();
        let config = TestProvider::new()
            .with_value("cors.allowed_origins", "*")
            .with_value("cors.allowed_methods", "GET,POST");
        let result = validator.validate(&config);
        assert!(result.is_err());
        let violations = result.unwrap_err();
        assert!(violations
            .iter()
            .any(|v| v.severity == ViolationSeverity::Warning
                && v.message.contains("wildcard")));
    }

    #[test]
    fn test_empty_methods_critical() {
        let validator = CorsValidator::new();
        let config = TestProvider::new()
            .with_value("cors.allowed_origins", "https://example.com")
            .with_value("cors.allowed_methods", "");
        let result = validator.validate(&config);
        assert!(result.is_err());
        let violations = result.unwrap_err();
        assert!(violations
            .iter()
            .any(|v| v.severity == ViolationSeverity::Critical
                && v.message.contains("empty")));
    }

    #[test]
    fn test_origins_without_methods_critical() {
        let validator = CorsValidator::new();
        let config = TestProvider::new().with_value("cors.allowed_origins", "https://example.com");
        let result = validator.validate(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_excessive_max_age_warning() {
        let validator = CorsValidator::new();
        let config = TestProvider::new()
            .with_value("cors.allowed_origins", "https://example.com")
            .with_value("cors.allowed_methods", "GET,POST")
            .with_value("cors.max_age", "172800");
        let result = validator.validate(&config);
        assert!(result.is_err());
        let violations = result.unwrap_err();
        assert!(violations
            .iter()
            .any(|v| v.severity == ViolationSeverity::Warning
                && v.message.contains("max_age")));
    }

    #[test]
    fn test_valid_cors_config() {
        let validator = CorsValidator::new();
        let config = TestProvider::new()
            .with_value("cors.allowed_origins", "https://example.com")
            .with_value("cors.allowed_methods", "GET,POST,PUT")
            .with_value("cors.max_age", "3600");
        assert!(validator.validate(&config).is_ok());
    }

    #[test]
    fn test_trait_properties() {
        let validator = CorsValidator::new();
        assert_eq!(validator.name(), "cors");
        assert_eq!(validator.category(), "network");
    }
}
