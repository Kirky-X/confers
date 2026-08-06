// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! TLS configuration security validator.

use super::{SecurityValidator, SecurityViolation, ViolationSeverity};
use crate::interface::ConfigProvider;

/// Known weak cipher suites that should be rejected.
const WEAK_CIPHER_SUITES: &[&str] = &[
    // RC4-based (broken)
    "TLS_RSA_WITH_RC4_128_SHA",
    "TLS_RSA_WITH_RC4_128_MD5",
    "TLS_ECDHE_RSA_WITH_RC4_128_SHA",
    // 3DES-based (SWEET32 attack)
    "TLS_RSA_WITH_3DES_EDE_CBC_SHA",
    "TLS_ECDHE_RSA_WITH_3DES_EDE_CBC_SHA",
    // DES-based (trivially broken)
    "TLS_RSA_WITH_DES_CBC_SHA",
    // Export-grade (FREAK/Logjam)
    "TLS_RSA_EXPORT_WITH_RC4_40_MD5",
    "TLS_RSA_EXPORT_WITH_DES40_CBC_SHA",
    // NULL ciphers (no encryption)
    "TLS_RSA_WITH_NULL_MD5",
    "TLS_RSA_WITH_NULL_SHA",
    "TLS_RSA_WITH_NULL_SHA256",
    // CBC with MD5 (weak MAC)
    "TLS_RSA_WITH_AES_128_CBC_MD5",
];

/// Minimum acceptable TLS version string.
const MIN_TLS_VERSION: &str = "1.2";

/// Validates TLS configuration for security issues.
///
/// Checks:
/// - `tls.min_version` is at least "1.2"
/// - `tls.cipher_suites` does not contain known weak cipher suites
pub struct TlsConfigValidator;

impl TlsConfigValidator {
    /// Create a new TLS configuration validator.
    pub fn new() -> Self {
        Self
    }
}

impl Default for TlsConfigValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl SecurityValidator for TlsConfigValidator {
    fn validate(&self, config: &dyn ConfigProvider) -> Result<(), Vec<SecurityViolation>> {
        let mut violations = Vec::new();

        let has_min_version = config.get_raw("tls.min_version").is_some();
        let has_cipher_suites = config.get_raw("tls.cipher_suites").is_some();

        // If no TLS config at all, skip
        if !has_min_version && !has_cipher_suites {
            return Ok(());
        }

        // Check min_version
        if let Some(value) = config.get_raw("tls.min_version") {
            #[allow(deprecated)]
            if let Some(version_str) = value.as_string() {
                let normalized = version_str.trim().to_lowercase();
                // Accept "1.2", "1.3", "TLSv1.2", "TLSv1.3", etc.
                let version_num = normalized
                    .strip_prefix("tlsv")
                    .unwrap_or(&normalized);

                if version_num < MIN_TLS_VERSION {
                    violations.push(SecurityViolation {
                        validator: self.name().to_string(),
                        field: Some("tls.min_version".to_string()),
                        message: format!(
                            "TLS minimum version '{}' is below recommended minimum of '{}'",
                            version_str, MIN_TLS_VERSION
                        ),
                        severity: ViolationSeverity::Critical,
                    });
                }
            }
        }

        // Check cipher suites
        if let Some(value) = config.get_raw("tls.cipher_suites") {
            #[allow(deprecated)]
            if let Some(suites_str) = value.as_string() {
                for suite in suites_str.split(',') {
                    let suite = suite.trim();
                    if suite.is_empty() {
                        continue;
                    }
                    // Case-insensitive comparison
                    let suite_upper = suite.to_uppercase();
                    if WEAK_CIPHER_SUITES
                        .iter()
                        .any(|&weak| weak.eq_ignore_ascii_case(&suite_upper))
                    {
                        violations.push(SecurityViolation {
                            validator: self.name().to_string(),
                            field: Some("tls.cipher_suites".to_string()),
                            message: format!("Weak cipher suite detected: {suite}"),
                            severity: ViolationSeverity::Warning,
                        });
                    }
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
        "tls_config"
    }

    fn category(&self) -> &'static str {
        "transport"
    }

    fn description(&self) -> &'static str {
        "Validates TLS configuration uses minimum version 1.2+ and no weak cipher suites"
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
    fn test_no_tls_config_skips() {
        let validator = TlsConfigValidator::new();
        let config = TestProvider::new();
        assert!(validator.validate(&config).is_ok());
    }

    #[test]
    fn test_tls_1_2_ok() {
        let validator = TlsConfigValidator::new();
        let config = TestProvider::new().with_value("tls.min_version", "1.2");
        assert!(validator.validate(&config).is_ok());
    }

    #[test]
    fn test_tls_1_3_ok() {
        let validator = TlsConfigValidator::new();
        let config = TestProvider::new().with_value("tls.min_version", "1.3");
        assert!(validator.validate(&config).is_ok());
    }

    #[test]
    fn test_tlsv_prefix_ok() {
        let validator = TlsConfigValidator::new();
        let config = TestProvider::new().with_value("tls.min_version", "TLSv1.3");
        assert!(validator.validate(&config).is_ok());
    }

    #[test]
    fn test_tls_1_0_critical() {
        let validator = TlsConfigValidator::new();
        let config = TestProvider::new().with_value("tls.min_version", "1.0");
        let result = validator.validate(&config);
        assert!(result.is_err());
        let violations = result.unwrap_err();
        assert!(violations
            .iter()
            .any(|v| v.severity == ViolationSeverity::Critical
                && v.message.contains("below recommended")));
    }

    #[test]
    fn test_tls_1_1_critical() {
        let validator = TlsConfigValidator::new();
        let config = TestProvider::new().with_value("tls.min_version", "1.1");
        assert!(validator.validate(&config).is_err());
    }

    #[test]
    fn test_weak_cipher_warning() {
        let validator = TlsConfigValidator::new();
        let config = TestProvider::new()
            .with_value("tls.cipher_suites", "TLS_RSA_WITH_RC4_128_SHA,TLS_AES_128_GCM_SHA256");
        let result = validator.validate(&config);
        assert!(result.is_err());
        let violations = result.unwrap_err();
        assert!(violations
            .iter()
            .any(|v| v.severity == ViolationSeverity::Warning
                && v.message.contains("Weak cipher")));
    }

    #[test]
    fn test_strong_cipher_ok() {
        let validator = TlsConfigValidator::new();
        let config = TestProvider::new()
            .with_value("tls.cipher_suites", "TLS_AES_128_GCM_SHA256,TLS_AES_256_GCM_SHA384");
        assert!(validator.validate(&config).is_ok());
    }

    #[test]
    fn test_null_cipher_warning() {
        let validator = TlsConfigValidator::new();
        let config = TestProvider::new()
            .with_value("tls.cipher_suites", "TLS_RSA_WITH_NULL_SHA256");
        let result = validator.validate(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_trait_properties() {
        let validator = TlsConfigValidator::new();
        assert_eq!(validator.name(), "tls_config");
        assert_eq!(validator.category(), "transport");
    }
}
