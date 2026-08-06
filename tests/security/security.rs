// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Security integration tests.
//!
//! Tests for security features including KeyRegistry, error sanitization.

#![cfg(feature = "encryption")]

use confers::secret::{KeyRegistry, KeyRotationConfig, SecretBytes, XChaCha20Crypto};
use confers::ConfigError;

/// Generate a test key with a specific pattern.
/// NOTE: This is for testing only, never use in production!
fn make_test_key(seed: u8) -> SecretBytes {
    let mut key = vec![seed; 32];
    // Add some variation to make it more realistic
    for (i, byte) in key.iter_mut().enumerate() {
        *byte = byte.wrapping_add(i as u8);
    }
    SecretBytes::new(key)
}

#[test]
fn test_key_registry_rotation() {
    let registry = KeyRegistry::new(KeyRotationConfig::default());

    let key1 = make_test_key(1);
    let key2 = make_test_key(2);

    registry.register_key("v1".to_string(), key1, true).unwrap();
    let old = registry.rotate_to("v2".to_string(), key2).unwrap();

    assert_eq!(old, "v1");

    let (version, _) = registry.get_primary_key().unwrap();
    assert_eq!(version, "v2");
}

#[test]
fn test_key_registry_try_all_keys() {
    let registry = KeyRegistry::new(KeyRotationConfig::default());

    let key1 = make_test_key(1);
    let key2 = make_test_key(2);

    registry.register_key("v1".to_string(), key1, true).unwrap();
    registry
        .register_key("v2".to_string(), key2, false)
        .unwrap();

    let crypto = XChaCha20Crypto::new();
    let plaintext = b"test data";

    // Use the same helper to get the v2 key bytes
    let k2 = make_test_key(2);
    let (nonce, ciphertext) = crypto.encrypt(plaintext, k2.as_slice()).unwrap();

    let (version, decrypted) = registry
        .try_decrypt_with_all_keys(&nonce, &ciphertext)
        .unwrap();

    assert_eq!(version, "v2");
    assert_eq!(decrypted, plaintext);
}

#[test]
fn test_error_sanitization() {
    let error = ConfigError::FileNotFound {
        filename: "/tmp/test_config.txt".into(),
        source: None,
    };

    let user_message = error.user_message();
    assert!(
        user_message.contains("not found"),
        "expected 'not found' in message, got: {user_message}"
    );
}

// =============================================================================
// Security Rules Integration Tests
// =============================================================================

#[cfg(feature = "security-rules")]
mod security_rules_tests {
    use confers::interface::ConfigProvider;
    use confers::security::rules::{
        SecurityValidatorRegistry, SecurityViolation, ViolationSeverity,
    };
    use confers::types::{AnnotatedValue, ConfigValue, SourceId};
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
    fn test_registry_with_defaults_validates_all() {
        let registry = SecurityValidatorRegistry::with_defaults();
        assert_eq!(registry.validator_count(), 4);

        // Empty config — JWT missing (warning), CORS skipped, SSRF skipped, TLS skipped
        let config = TestProvider::new();
        let report = registry.validate_all(&config);

        // JWT missing produces a warning
        assert!(report
            .violations
            .iter()
            .any(|v| { v.validator == "jwt_secret" && v.severity == ViolationSeverity::Warning }));
    }

    #[test]
    fn test_registry_detects_multiple_violations() {
        let registry = SecurityValidatorRegistry::with_defaults();

        let config = TestProvider::new()
            .with_value("jwt.secret", "short") // too short + weak
            .with_value("cors.allowed_origins", "*") // wildcard
            .with_value("cors.allowed_methods", "GET,POST")
            .with_value("tls.min_version", "1.0") // too old
            .with_value("ssrf.allowed_urls", "https://127.0.0.1/admin"); // SSRF

        let report = registry.validate_all(&config);

        // Verify specific expected violations
        let validators: Vec<&str> = report
            .violations
            .iter()
            .map(|v| v.validator.as_str())
            .collect();
        assert!(
            validators.contains(&"jwt_secret"),
            "expected jwt_secret violation"
        );
        assert!(validators.contains(&"cors"), "expected cors violation");
        assert!(
            validators.contains(&"tls_config"),
            "expected tls_config violation, got: {:?}",
            validators
        );
        assert!(validators.contains(&"ssrf"), "expected ssrf violation");
        assert!(report.critical_count() > 0);
        assert!(!report.is_ok(false));
    }

    #[test]
    fn test_registry_clean_config_passes() {
        let registry = SecurityValidatorRegistry::with_defaults();

        let config = TestProvider::new()
            .with_value(
                "jwt.secret",
                "this_is_a_very_long_secret_that_is_at_least_32_bytes!",
            )
            .with_value("cors.allowed_origins", "https://example.com")
            .with_value("cors.allowed_methods", "GET,POST")
            .with_value("cors.max_age", "3600")
            .with_value("tls.min_version", "1.3")
            .with_value("tls.cipher_suites", "TLS_AES_128_GCM_SHA256")
            .with_value("ssrf.allowed_urls", "https://api.example.com/webhook");

        let report = registry.validate_all(&config);
        assert!(report.is_ok(false));
        assert_eq!(report.critical_count(), 0);
    }

    #[test]
    fn test_fail_on_warning_mode() {
        let registry = SecurityValidatorRegistry::with_defaults().with_fail_on_warning(true);

        // Config with only a warning (JWT missing)
        let config = TestProvider::new();
        let report = registry.validate_all(&config);

        // With fail_on_warning=true, warning-only violations cause failure
        assert!(!report.is_ok(true));
    }

    #[test]
    fn test_custom_validator_registration() {
        use confers::security::rules::SecurityValidator;

        struct AlwaysFailValidator;
        impl SecurityValidator for AlwaysFailValidator {
            fn validate(&self, _config: &dyn ConfigProvider) -> Result<(), Vec<SecurityViolation>> {
                Err(vec![SecurityViolation {
                    validator: "always_fail".to_string(),
                    field: None,
                    message: "always fails".to_string(),
                    severity: ViolationSeverity::Critical,
                }])
            }
            fn name(&self) -> &'static str {
                "always_fail"
            }
            fn category(&self) -> &'static str {
                "custom"
            }
            fn description(&self) -> &'static str {
                "Always fails for testing"
            }
        }

        let mut registry = SecurityValidatorRegistry::new();
        registry.register(Box::new(AlwaysFailValidator));

        let config = TestProvider::new();
        let report = registry.validate_all(&config);
        assert_eq!(report.violations.len(), 1);
        assert_eq!(report.violations[0].validator, "always_fail");
    }
}
