// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! E2E: 安全规则与注入(tests/e2e/security_rules_e2e.rs)
//!
//! 场景固化(docs/TEST_SCENARIOS.md §2.7):
//! - SEC-01/02 `EnvSecurityValidator` 合法 env 名/值放行;注入字符(shell 元字符/
//!   空格/换行)拒绝
//! - SEC-03/04 `EnvironmentValidationConfig` 长度上限拒绝;strict() 与 lenient()
//!   对同一超长输入结论不同
//! - SEC-05 `sanitize_for_logging` 按真实行为固化(>100 字符截断为前 97 字符+…)
//! - SEC-12/14/15 Cors/JwtSecret/Tls 三个内置验证器分别检出违规
//! - SEC-13 `SsrfValidator` 拒绝私网/链路本地目标(127.0.0.1/10.x/169.254.x),
//!   公网 HTTPS 放行(is_ip_blocked 交叉验证见 tests/remote)
//! - SEC-16 `SecurityReport` 聚合各验证器违规,按真实行为固化(critical/warning 计数)
//! - SEC-17/SEC-18(ConfigInjector/SensitiveDataDetector)为 pub(crate) 内部 API,
//!   无公开路径可达,由 src 内联 tests(config_injector.rs / input_validation.rs)
//!   覆盖;本文件固化公开面 `EncryptionPrefix`(enc: 前缀识别/剥离)。
//!
//! SEC-06…11 已有覆盖(tests/security/security.rs)。

use confers::interface::ConfigProvider;
use confers::security::rules::{
    CorsValidator, JwtSecretValidator, SecurityValidator, SecurityViolation, SsrfValidator,
    TlsConfigValidator, ViolationSeverity,
};
use confers::security::{EncryptionPrefix, EnvSecurityValidator, EnvironmentValidationConfig};
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
fn sec0102_env_validation_allows_clean_and_rejects_injection() {
    let validator = EnvSecurityValidator::new();

    // 合法名/值放行(SEC-01)。
    validator
        .validate_env_name("APP_DATABASE_HOST", None)
        .expect("clean name must pass");
    validator
        .validate_env_value("postgres://db.local/app")
        .expect("clean value must pass");
    assert!(validator.should_allow_env_var("APP_PORT"));

    // 注入字符拒绝(SEC-02):空格/等号/换行/shell 元字符。
    for bad in ["APP CMD", "APP=CMD", "APP\nCMD", "APP$(whoami)", "APP`id`"] {
        assert!(
            validator.validate_env_name(bad, None).is_err(),
            "injected name '{bad}' must be rejected"
        );
    }
    // 值内换行/控制字符拒绝。
    assert!(validator.validate_env_value("line1\nline2").is_err());
}

#[test]
fn sec0304_length_limits_and_strict_lenient_divergence() {
    // 自定义长度上限拒绝(SEC-03)。
    let tight = EnvSecurityValidator::with_config(
        EnvironmentValidationConfig::new().with_max_value_length(16),
    );
    assert!(tight.validate_env_value("short").is_ok());
    let err = tight
        .validate_env_value("this-value-is-way-longer-than-16")
        .expect_err("over-limit value must be rejected");

    // strict vs lenient 同一输入结论不同(SEC-04):3000 字符值。
    let long_value = "x".repeat(3000);
    let strict = EnvSecurityValidator::strict();
    let lenient = EnvSecurityValidator::lenient();
    assert!(
        strict.validate_env_value(&long_value).is_err(),
        "strict(2048) must reject 3000-char value"
    );
    assert!(
        lenient.validate_env_value(&long_value).is_ok(),
        "lenient(8192, length check off) must accept the same value"
    );

    // 让 err 进入使用,避免 unused 警告:错误可展示。
    let _ = err.to_string();
}

#[test]
fn sec05_sanitize_for_logging_truncates_long_values() {
    let validator = EnvSecurityValidator::new();

    let secret = "super-secret-password-do-not-leak".to_string();
    assert_eq!(
        validator.sanitize_for_logging(&secret),
        secret,
        "values within 100 chars are logged as-is"
    );

    let long_secret = "s".repeat(150);
    let sanitized = validator.sanitize_for_logging(&long_secret);
    assert_eq!(sanitized.chars().count(), 100, "97 chars + '...'");
    assert!(sanitized.ends_with("..."), "truncation marker appended");
    assert!(
        !sanitized.contains(&long_secret),
        "full value must not leak"
    );
}

#[test]
fn sec121415_builtin_validators_detect_violations() {
    // CORS:通配 origin(Warning)+ 有 origins 无 methods(SEC-12)。
    let cors_report = CorsValidator::new()
        .validate(&TestProvider::new().with_value("cors.allowed_origins", "*"))
        .expect_err("wildcard origin must be flagged");
    assert!(
        cors_report
            .iter()
            .any(|v| v.severity == ViolationSeverity::Warning)
    );

    // JWT:短密钥与常见弱密钥(Critical)(SEC-14)。
    for weak in ["short", "password", "changeme"] {
        let violations = JwtSecretValidator::new()
            .validate(&TestProvider::new().with_value("jwt.secret", weak))
            .expect_err("weak jwt secret must be flagged");
        assert!(
            violations
                .iter()
                .any(|v| v.severity == ViolationSeverity::Critical),
            "weak secret '{weak}' must be critical"
        );
    }
    // 足够长的密钥通过。
    let strong = "a-very-long-and-random-jwt-secret-value-32+";
    assert!(
        JwtSecretValidator::new()
            .validate(&TestProvider::new().with_value("jwt.secret", strong))
            .is_ok()
    );

    // TLS:过旧 min_version(Critical)(SEC-15)。
    let tls_violations = TlsConfigValidator::new()
        .validate(&TestProvider::new().with_value("tls.min_version", "1.0"))
        .expect_err("TLS 1.0 must be flagged");
    assert!(
        tls_violations
            .iter()
            .any(|v| v.severity == ViolationSeverity::Critical)
    );
}

#[test]
fn sec13_ssrf_validator_blocks_private_targets() {
    let validator = SsrfValidator::new();

    // 私网/回环/链路本地目标拒绝(SEC-13)。
    for private_url in [
        "https://127.0.0.1/admin",
        "https://10.0.0.5/internal",
        "https://169.254.169.254/metadata",
        "https://192.168.1.1/router",
    ] {
        let violations = validator
            .validate(&TestProvider::new().with_value("ssrf.allowed_urls", private_url))
            .expect_err(&format!("private target {private_url} must be flagged"));
        assert!(
            violations.iter().any(|v| v.validator == "ssrf"),
            "ssrf violation expected for {private_url}"
        );
    }

    // 公网 HTTPS 放行。
    assert!(
        validator
            .validate(
                &TestProvider::new().with_value("ssrf.allowed_urls", "https://api.example.com")
            )
            .is_ok()
    );
}

#[test]
fn sec16_report_aggregates_counts_by_severity() {
    let mut registry = confers::security::rules::SecurityValidatorRegistry::new();

    struct TwoViolations;
    impl SecurityValidator for TwoViolations {
        fn validate(&self, _config: &dyn ConfigProvider) -> Result<(), Vec<SecurityViolation>> {
            Err(vec![
                SecurityViolation {
                    validator: "two".to_string(),
                    field: None,
                    message: "critical one".to_string(),
                    severity: ViolationSeverity::Critical,
                },
                SecurityViolation {
                    validator: "two".to_string(),
                    field: None,
                    message: "warning one".to_string(),
                    severity: ViolationSeverity::Warning,
                },
            ])
        }
        fn name(&self) -> &'static str {
            "two"
        }
        fn category(&self) -> &'static str {
            "custom"
        }
        fn description(&self) -> &'static str {
            "Emits one critical and one warning"
        }
    }

    registry.register(Box::new(TwoViolations));
    let report = registry.validate_all(&TestProvider::new());

    assert_eq!(report.critical_count(), 1);
    assert_eq!(report.warning_count(), 1);
    assert!(
        !report.is_ok(false),
        "critical must fail regardless of flag"
    );
    assert!(!report.is_ok(true));
    assert!(report.passed.is_empty(), "failing validator not in passed");

    // 通过的验证器进入 passed 列表(SEC-16 汇总语义)。
    let empty_report = confers::security::rules::SecurityValidatorRegistry::new()
        .validate_all(&TestProvider::new());
    assert!(empty_report.is_ok(true));
}

#[test]
fn sec18_encryption_prefix_recognition_and_strip() {
    // enc: 前缀识别与剥离(SEC-18 公开面;敏感模式表见 src/security/patterns.rs 内联测试)。
    let prefix = EncryptionPrefix::Enc;
    let encrypted = "enc:Q2lwaGVydGV4dA==";
    assert!(prefix.is_prefixed(encrypted));
    assert!(!prefix.is_prefixed("plain-value"));
    assert_eq!(prefix.strip(encrypted), Some("Q2lwaGVydGV4dA=="));
    assert_eq!(prefix.strip("plain-value"), None);
}
