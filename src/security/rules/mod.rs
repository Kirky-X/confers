// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Security validation rule library.
//!
//! Provides a standardized [`SecurityValidator`] trait and built-in validators
//! for common security checks (JWT secret strength, CORS configuration, SSRF
//! protection, TLS settings).
//!
//! Enable the `security-rules` feature to use this module.

mod cors;
mod jwt;
mod ssrf;
mod tls;

use crate::interface::ConfigProvider;

pub use cors::CorsValidator;
pub use jwt::JwtSecretValidator;
pub use ssrf::SsrfValidator;
pub use tls::TlsConfigValidator;

/// Severity level of a security violation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViolationSeverity {
    /// Must be fixed — blocks startup.
    Critical,
    /// Recommended fix — application can continue with warnings.
    Warning,
}

/// A single security violation found during validation.
#[derive(Debug, Clone)]
pub struct SecurityViolation {
    /// Name of the validator that produced this violation.
    pub validator: String,
    /// Configuration field related to this violation (if applicable).
    pub field: Option<String>,
    /// Human-readable description of the violation.
    pub message: String,
    /// Severity level.
    pub severity: ViolationSeverity,
}

impl std::fmt::Display for SecurityViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let severity = match self.severity {
            ViolationSeverity::Critical => "CRITICAL",
            ViolationSeverity::Warning => "WARNING",
        };
        if let Some(ref field) = self.field {
            write!(
                f,
                "[{}] {} (field: {}): {}",
                severity, self.validator, field, self.message
            )
        } else {
            write!(f, "[{}] {}: {}", severity, self.validator, self.message)
        }
    }
}

/// Trait for security validators.
///
/// Security validators perform synchronous, pure-computation checks on
/// configuration values to detect insecure settings. They do not perform
/// any I/O operations.
///
/// # Example
///
/// ```rust,ignore
/// use confers::security::rules::{SecurityValidator, SecurityViolation};
///
/// struct MyValidator;
///
/// impl SecurityValidator for MyValidator {
///     fn validate(&self, config: &dyn ConfigProvider) -> Result<(), Vec<SecurityViolation>> {
///         // Check some configuration value...
///         Ok(())
///     }
///     fn name(&self) -> &'static str { "my_validator" }
///     fn category(&self) -> &'static str { "custom" }
///     fn description(&self) -> &'static str { "My custom security check" }
/// }
/// ```
pub trait SecurityValidator: Send + Sync {
    /// Validate configuration for security issues.
    ///
    /// Returns `Ok(())` if no violations are found, or `Err(violations)` with
    /// all detected violations collected (not fail-fast).
    fn validate(&self, config: &dyn ConfigProvider) -> Result<(), Vec<SecurityViolation>>;

    /// Get the validator name for logging and reporting.
    fn name(&self) -> &'static str;

    /// Get the validator category (e.g., "authentication", "network", "transport").
    fn category(&self) -> &'static str;

    /// Get a human-readable description of what this validator checks.
    fn description(&self) -> &'static str;
}

/// Aggregated result of running all registered security validators.
#[derive(Debug)]
pub struct SecurityReport {
    /// All violations found across all validators.
    pub violations: Vec<SecurityViolation>,
    /// Names of validators that passed without violations.
    pub passed: Vec<String>,
}

impl SecurityReport {
    /// Check if the report indicates an acceptable security posture.
    ///
    /// When `fail_on_warning` is `false` (default), only `Critical` violations
    /// cause failure. When `true`, `Warning` violations also cause failure.
    pub fn is_ok(&self, fail_on_warning: bool) -> bool {
        if fail_on_warning {
            self.violations.is_empty()
        } else {
            !self
                .violations
                .iter()
                .any(|v| v.severity == ViolationSeverity::Critical)
        }
    }

    /// Count of critical violations.
    pub fn critical_count(&self) -> usize {
        self.violations
            .iter()
            .filter(|v| v.severity == ViolationSeverity::Critical)
            .count()
    }

    /// Count of warning violations.
    pub fn warning_count(&self) -> usize {
        self.violations
            .iter()
            .filter(|v| v.severity == ViolationSeverity::Warning)
            .count()
    }
}

/// Registry that collects and executes security validators.
///
/// Use [`SecurityValidatorRegistry::with_defaults`] to create a registry
/// pre-loaded with all built-in validators, or [`SecurityValidatorRegistry::new`]
/// for an empty registry and add validators manually.
pub struct SecurityValidatorRegistry {
    validators: Vec<Box<dyn SecurityValidator>>,
    /// Whether to treat warnings as failures in `validate_all`.
    fail_on_warning: bool,
}

impl SecurityValidatorRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            validators: Vec::new(),
            fail_on_warning: false,
        }
    }

    /// Create a registry pre-loaded with all built-in validators.
    pub fn with_defaults() -> Self {
        let mut registry = Self::new();
        registry.register(Box::new(JwtSecretValidator::new()));
        registry.register(Box::new(CorsValidator::new()));
        registry.register(Box::new(SsrfValidator::new()));
        registry.register(Box::new(TlsConfigValidator::new()));
        registry
    }

    /// Set whether warnings should be treated as failures.
    pub fn with_fail_on_warning(mut self, fail: bool) -> Self {
        self.fail_on_warning = fail;
        self
    }

    /// Register a custom security validator.
    pub fn register(&mut self, validator: Box<dyn SecurityValidator>) {
        self.validators.push(validator);
    }

    /// Execute all registered validators and collect results.
    pub fn validate_all(&self, config: &dyn ConfigProvider) -> SecurityReport {
        let mut violations = Vec::new();
        let mut passed = Vec::new();

        for validator in &self.validators {
            match validator.validate(config) {
                Ok(()) => {
                    passed.push(validator.name().to_string());
                }
                Err(v) => {
                    violations.extend(v);
                }
            }
        }

        SecurityReport { violations, passed }
    }

    /// Get the number of registered validators.
    pub fn validator_count(&self) -> usize {
        self.validators.len()
    }
}

impl Default for SecurityValidatorRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::AnnotatedValue;
    use std::collections::HashMap;

    /// Simple in-memory ConfigProvider for testing.
    struct TestProvider(HashMap<String, AnnotatedValue>);

    impl TestProvider {
        fn new() -> Self {
            Self(HashMap::new())
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
    fn test_violation_severity_equality() {
        assert_eq!(ViolationSeverity::Critical, ViolationSeverity::Critical);
        assert_eq!(ViolationSeverity::Warning, ViolationSeverity::Warning);
        assert_ne!(ViolationSeverity::Critical, ViolationSeverity::Warning);
    }

    #[test]
    fn test_security_violation_display_with_field() {
        let v = SecurityViolation {
            validator: "jwt_secret".to_string(),
            field: Some("jwt.secret".to_string()),
            message: "Secret too short".to_string(),
            severity: ViolationSeverity::Critical,
        };
        let display = format!("{v}");
        assert!(display.contains("CRITICAL"));
        assert!(display.contains("jwt_secret"));
        assert!(display.contains("jwt.secret"));
        assert!(display.contains("Secret too short"));
    }

    #[test]
    fn test_security_violation_display_without_field() {
        let v = SecurityViolation {
            validator: "cors".to_string(),
            field: None,
            message: "Missing allowed_methods".to_string(),
            severity: ViolationSeverity::Warning,
        };
        let display = format!("{v}");
        assert!(display.contains("WARNING"));
        assert!(!display.contains("field:"));
    }

    #[test]
    fn test_security_report_is_ok_no_violations() {
        let report = SecurityReport {
            violations: vec![],
            passed: vec!["jwt_secret".to_string()],
        };
        assert!(report.is_ok(false));
        assert!(report.is_ok(true));
    }

    #[test]
    fn test_security_report_is_ok_critical_only() {
        let report = SecurityReport {
            violations: vec![SecurityViolation {
                validator: "jwt_secret".to_string(),
                field: None,
                message: "too short".to_string(),
                severity: ViolationSeverity::Critical,
            }],
            passed: vec![],
        };
        assert!(!report.is_ok(false));
        assert!(!report.is_ok(true));
    }

    #[test]
    fn test_security_report_is_ok_warning_only() {
        let report = SecurityReport {
            violations: vec![SecurityViolation {
                validator: "cors".to_string(),
                field: None,
                message: "wildcard origin".to_string(),
                severity: ViolationSeverity::Warning,
            }],
            passed: vec![],
        };
        // Warning only: ok when fail_on_warning=false, not ok when true
        assert!(report.is_ok(false));
        assert!(!report.is_ok(true));
    }

    #[test]
    fn test_security_report_counts() {
        let report = SecurityReport {
            violations: vec![
                SecurityViolation {
                    validator: "a".to_string(),
                    field: None,
                    message: "critical 1".to_string(),
                    severity: ViolationSeverity::Critical,
                },
                SecurityViolation {
                    validator: "b".to_string(),
                    field: None,
                    message: "warning 1".to_string(),
                    severity: ViolationSeverity::Warning,
                },
                SecurityViolation {
                    validator: "c".to_string(),
                    field: None,
                    message: "critical 2".to_string(),
                    severity: ViolationSeverity::Critical,
                },
            ],
            passed: vec!["d".to_string()],
        };
        assert_eq!(report.critical_count(), 2);
        assert_eq!(report.warning_count(), 1);
    }

    #[test]
    fn test_registry_new_is_empty() {
        let registry = SecurityValidatorRegistry::new();
        assert_eq!(registry.validator_count(), 0);
    }

    #[test]
    fn test_registry_register_and_count() {
        struct DummyValidator;
        impl SecurityValidator for DummyValidator {
            fn validate(&self, _config: &dyn ConfigProvider) -> Result<(), Vec<SecurityViolation>> {
                Ok(())
            }
            fn name(&self) -> &'static str {
                "dummy"
            }
            fn category(&self) -> &'static str {
                "test"
            }
            fn description(&self) -> &'static str {
                "Dummy validator"
            }
        }

        let mut registry = SecurityValidatorRegistry::new();
        registry.register(Box::new(DummyValidator));
        assert_eq!(registry.validator_count(), 1);
    }

    #[test]
    fn test_registry_validate_all_collects_violations() {
        struct PassValidator;
        impl SecurityValidator for PassValidator {
            fn validate(&self, _config: &dyn ConfigProvider) -> Result<(), Vec<SecurityViolation>> {
                Ok(())
            }
            fn name(&self) -> &'static str {
                "pass"
            }
            fn category(&self) -> &'static str {
                "test"
            }
            fn description(&self) -> &'static str {
                "Always passes"
            }
        }

        struct FailValidator;
        impl SecurityValidator for FailValidator {
            fn validate(&self, _config: &dyn ConfigProvider) -> Result<(), Vec<SecurityViolation>> {
                Err(vec![SecurityViolation {
                    validator: "fail".to_string(),
                    field: None,
                    message: "always fails".to_string(),
                    severity: ViolationSeverity::Critical,
                }])
            }
            fn name(&self) -> &'static str {
                "fail"
            }
            fn category(&self) -> &'static str {
                "test"
            }
            fn description(&self) -> &'static str {
                "Always fails"
            }
        }

        let mut registry = SecurityValidatorRegistry::new();
        registry.register(Box::new(PassValidator));
        registry.register(Box::new(FailValidator));

        let config = TestProvider::new();
        let report = registry.validate_all(&config);

        assert_eq!(report.violations.len(), 1);
        assert_eq!(report.critical_count(), 1);
        assert_eq!(report.passed.len(), 1);
        assert_eq!(report.passed[0], "pass");
    }

    #[test]
    fn test_registry_with_defaults_has_four_validators() {
        let registry = SecurityValidatorRegistry::with_defaults();
        assert_eq!(registry.validator_count(), 4);
    }

    #[test]
    fn test_registry_default_is_empty() {
        let registry = SecurityValidatorRegistry::default();
        assert_eq!(registry.validator_count(), 0);
    }

    #[test]
    fn test_registry_with_fail_on_warning() {
        let registry = SecurityValidatorRegistry::new().with_fail_on_warning(true);
        assert!(registry.fail_on_warning);
    }
}
