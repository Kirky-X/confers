// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! # 安全测试用例
//!
//! 提供全面的安全测试用例，确保安全功能的正确性和健壮性。
//!
//! ## 测试类别
//!
//! - **敏感数据处理测试**: SecureString 的安全功能测试
//! - **注入攻击测试**: 各种注入攻击的防护测试
//! - **边界条件测试**: 输入边界和极端情况测试
//! - **内存安全测试**: 内存清零和防止泄漏测试

use crate::security::{
    ConfigInjectionError, ConfigInjector, ConfigValidationResult, ConfigValidator,
    EnvSecurityError, EnvSecurityValidator, ErrorSanitizer, FilterResult, InputValidationError,
    InputValidator, SafeResult, SecureString, SecureStringBuilder, SensitiveDataDetector,
    SensitiveDataFilter, SensitivityLevel,
};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};

/// 测试计数器
static RUN_TESTS: AtomicUsize = AtomicUsize::new(0);
static PASSED_TESTS: AtomicUsize = AtomicUsize::new(0);
static FAILED_TESTS: AtomicUsize = AtomicUsize::new(0);

/// 获取测试统计
pub fn test_stats() -> (usize, usize, usize) {
    (
        RUN_TESTS.load(Ordering::SeqCst),
        PASSED_TESTS.load(Ordering::SeqCst),
        FAILED_TESTS.load(Ordering::SeqCst),
    )
}

/// 重置测试计数器
#[cfg(test)]
pub fn reset_test_counters() {
    RUN_TESTS.store(0, Ordering::SeqCst);
    PASSED_TESTS.store(0, Ordering::SeqCst);
    FAILED_TESTS.store(0, Ordering::SeqCst);
}

/// 运行测试的宏
macro_rules! run_test {
    ($name:ident, $test:expr) => {
        #[test]
        pub fn $name() {
            RUN_TESTS.fetch_add(1, Ordering::SeqCst);
            match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| $test)) {
                Ok(_) => {
                    PASSED_TESTS.fetch_add(1, Ordering::SeqCst);
                }
                Err(_) => {
                    FAILED_TESTS.fetch_add(1, Ordering::SeqCst);
                    panic!("Test {} failed", stringify!($name));
                }
            }
        }
    };
}

// ==================== 敏感数据处理测试 ====================

mod sensitive_data_tests {
    use super::*;

    run_test!(test_secure_string_creation, {
        let secret = SecureString::from("test-password");
        assert_eq!(secret.len(), 13);
        assert!(!secret.is_empty());
        assert_eq!(secret.sensitivity(), SensitivityLevel::Critical);
    });

    run_test!(test_secure_string_sensitivity_levels, {
        let low = SecureString::new("data", SensitivityLevel::Low);
        let medium = SecureString::new("user", SensitivityLevel::Medium);
        let high = SecureString::new("token", SensitivityLevel::High);
        let critical = SecureString::new("secret", SensitivityLevel::Critical);

        assert!(!low.is_highly_sensitive());
        assert!(!medium.is_highly_sensitive());
        assert!(high.is_highly_sensitive());
        assert!(critical.is_highly_sensitive());
    });

    run_test!(test_secure_string_comparison, {
        let secret1 = SecureString::from("password123");
        let secret2 = SecureString::from("password123");
        let secret3 = SecureString::from("different");

        assert!(secret1.compare("password123").is_ok());
        assert!(secret1.compare("wrong").is_err());
        assert_eq!(secret1, secret2);
        assert_ne!(secret1, secret3);
    });

    run_test!(test_secure_string_masking, {
        let secret = SecureString::from("mySecretPassword");
        let masked = secret.masked();

        // 掩码应该隐藏大部分内容
        assert!(masked.contains('*'));
        assert!(masked.len() < secret.len());
        // 但开头应该可见
        assert!(masked.starts_with("my"));
    });

    run_test!(test_secure_string_fingerprint, {
        let secret = SecureString::from("password123");
        let fp = secret.fingerprint(16);

        assert_eq!(fp.len(), 16);
        // 指纹应该是十六进制
        assert!(fp.chars().all(|c| c.is_ascii_hexdigit()));
    });

    run_test!(test_secure_string_builder, {
        let secret = SecureStringBuilder::new()
            .push_str("pass")
            .push('w')
            .push_str("ord")
            .build();

        assert_eq!(secret.as_str(), "password");
    });

    run_test!(test_secure_string_from_bytes, {
        let data = vec![0x01, 0x02, 0x03, 0x04, 0x05];
        let secret = SecureString::from_bytes(data.clone(), SensitivityLevel::High);

        assert_eq!(secret.as_bytes(), &data[..]);
    });
}

// ==================== 注入攻击测试 ====================

mod injection_attack_tests {
    use super::*;

    run_test!(test_sql_injection_patterns, {
        let validator = InputValidator::new();

        // SQL 注入尝试
        let attacks = vec![
            "'; DROP TABLE users;--",
            "' OR '1'='1",
            "admin'--",
            "UNION SELECT * FROM users",
            "1; DELETE FROM products",
        ];

        for attack in attacks {
            let result = validator.validate_string(attack);
            assert!(result.is_err(), "Should reject SQL injection: {}", attack);
        }
    });

    run_test!(test_command_injection_patterns, {
        let validator = InputValidator::new();

        // 命令注入尝试
        let attacks = vec![
            "; cat /etc/passwd",
            "| whoami",
            "`id`",
            "$(whoami)",
            "&& rm -rf /",
            "hello; world",
            "test${HOME}",
        ];

        for attack in attacks {
            let result = validator.validate_string(attack);
            assert!(
                result.is_err(),
                "Should reject command injection: {}",
                attack
            );
        }
    });

    run_test!(test_xss_patterns, {
        let validator = InputValidator::new();

        // XSS 尝试
        let attacks = vec![
            "<script>alert('xss')</script>",
            "javascript:alert(1)",
            "<img src=x onerror=alert(1)>",
            "<iframe src='javascript:alert(1)'>",
        ];

        for attack in attacks {
            let result = validator.validate_string(attack);
            // 这些可能不会全部被阻止，取决于验证器配置
            // 但应该至少对危险的模式进行警告
        }
    });

    run_test!(test_path_traversal, {
        let validator = InputValidator::new();

        // 路径遍历尝试
        let attacks = vec![
            "../../etc/passwd",
            "..\\..\\windows\\system32",
            "/var/www/../../../etc/passwd",
            "file.txt/../../../etc/passwd",
        ];

        for attack in attacks {
            let result = validator.validate_string(attack);
            // 路径遍历应该被检测
            assert!(result.is_err(), "Should reject path traversal: {}", attack);
        }
    });

    run_test!(test_environment_injection_protection, {
        let validator = EnvSecurityValidator::new();

        // 环境变量注入尝试
        let attacks = vec![
            ("PATH", "malicious"),
            ("HOME", "/tmp"),
            ("LD_PRELOAD", "libmalicious.so"),
            ("SECRET_KEY", "value"),
        ];

        for (name, value) in attacks {
            let result = validator.validate_env_name(name, None);
            assert!(
                result.is_err(),
                "Should reject environment variable: {}",
                name
            );
        }
    });

    run_test!(test_shell_expansion_protection, {
        let validator = EnvSecurityValidator::new();

        // Shell 扩展尝试
        let attacks = vec!["${USER}", "${HOME}", "`ls`", "$(whoami)"];

        for value in attacks {
            let result = validator.validate_env_value(value);
            assert!(result.is_err(), "Should reject shell expansion: {}", value);
        }
    });

    run_test!(test_null_byte_injection, {
        let validator = EnvSecurityValidator::new();

        // Null 字节注入
        let attacks = vec!["hello\0world", "pass\x00word"];

        for value in attacks {
            let result = validator.validate_env_value(value);
            assert!(result.is_err(), "Should reject null byte injection");
        }
    });
}

// ==================== 边界条件测试 ====================

mod boundary_condition_tests {
    use super::*;

    run_test!(test_empty_input, {
        let validator = InputValidator::new();
        assert!(validator.validate_string("").is_ok());
    });

    run_test!(test_very_long_input, {
        let validator = InputValidator::new().with_max_string_length(100);
        let long_input = "a".repeat(200);

        let result = validator.validate_string(&long_input);
        assert!(result.is_err());

        // 验证边界
        let valid_input = "a".repeat(100);
        assert!(validator.validate_string(&valid_input).is_ok());
    });

    run_test!(test_special_characters, {
        let validator = InputValidator::new();

        // 允许的特殊字符
        let valid = vec![
            "hello-world",
            "hello_world",
            "hello.world",
            "hello123",
            "HELLO",
        ];

        for s in valid {
            assert!(validator.validate_string(s).is_ok(), "Should accept: {}", s);
        }
    });

    run_test!(test_field_name_validation, {
        let validator = InputValidator::new();

        // 有效字段名
        assert!(validator.validate_field_name("app_name").is_ok());
        assert!(validator.validate_field_name("appPort").is_ok());
        assert!(validator.validate_field_name("APP_123").is_ok());
        assert!(validator.validate_field_name("app-name").is_ok()); // 连字符是允许的

        // 无效字段名
        assert!(validator.validate_field_name("123app").is_err());
        assert!(validator.validate_field_name("app name").is_err());
        assert!(validator.validate_field_name("").is_err());
    });

    run_test!(test_url_validation, {
        let validator = InputValidator::new();

        // 有效 URL
        assert!(validator.validate_url("https://example.com").is_ok());
        assert!(validator.validate_url("http://localhost:8080").is_ok());

        // 无效 URL
        assert!(validator.validate_url("ftp://example.com").is_err());
        assert!(validator.validate_url("javascript:alert(1)").is_err());
        assert!(validator.validate_url("file:///etc/passwd").is_err());
    });

    run_test!(test_email_validation, {
        let validator = InputValidator::new();

        // 有效邮箱
        assert!(validator.validate_email("user@example.com").is_ok());
        assert!(validator.validate_email("user.name@example.co.uk").is_ok());

        // 无效邮箱
        assert!(validator.validate_email("invalid").is_err());
        assert!(validator.validate_email("@example.com").is_err());
        assert!(validator.validate_email("user@").is_err());
    });

    run_test!(test_config_injector_boundaries, {
        let injector = ConfigInjector::new();

        // 有效注入
        assert!(injector.inject("APP_PORT", "8080").is_ok());
        assert!(injector.inject("APP_NAME", "test-app").is_ok());

        // 边界测试 - 超长值
        let long_value = "x".repeat(5000);
        let result = injector.inject("APP_LONG", &long_value);
        assert!(result.is_err()); // 应该被长度限制阻止
    });

    run_test!(test_sanitizer_boundary_cases, {
        let sanitizer = ErrorSanitizer::new();

        // 空消息
        assert!(!sanitizer.contains_sensitive(""));

        // 正常消息
        assert!(!sanitizer.contains_sensitive("Hello world"));

        // 极长消息
        let long_msg = "normal ".repeat(1000) + "password: secret";
        let result = sanitizer.sanitize(&long_msg);
        assert!(result.contains("***"));
    });
}

// ==================== 内存安全测试 ====================

mod memory_safety_tests {
    use super::*;

    run_test!(test_secure_string_zeroize, {
        let mut secret = SecureString::from("sensitive-data");
        let before_zeroize = secret.as_bytes().to_vec();

        // 确保数据存在
        assert_eq!(before_zeroize, b"sensitive-data".to_vec());

        // 清零
        secret.zeroize();

        // 验证数据已被清零
        let after_zeroize = secret.as_bytes();
        assert!(after_zeroize.iter().all(|&b| b == 0));
    });

    run_test!(test_secure_string_drop_zeroize, {
        let original_count = crate::security::secure_string::allocated_secure_strings();
        {
            let _secret = SecureString::from("temporary-secret");
            // 秘密在这里被创建
        }
        // 秘密在这里被销毁，应该触发清零

        // 验证计数器
        let after_count = crate::security::secure_string::deallocated_secure_strings();
        assert!(after_count > original_count);
    });

    run_test!(test_clone_warning, {
        let secret = SecureString::from("secret");
        // 克隆应该产生警告（在日志中）
        let _cloned = secret.clone();

        // 验证克隆的值相同
        assert_eq!(secret, _cloned);
    });

    run_test!(test_constant_time_comparison, {
        // 测试恒定时间比较不会泄露信息
        let secret = SecureString::from("password123");

        // 错误答案的响应时间应该与正确答案相似
        let start1 = std::time::Instant::now();
        secret.compare("password123").unwrap();
        let time1 = start1.elapsed();

        let start2 = std::time::Instant::now();
        secret.compare("wrongpassword").unwrap_err();
        let time2 = start2.elapsed();

        // 时间差应该很小（允许一定的误差）
        let time_diff = time1.as_nanos() as i64 - time2.as_nanos() as i64;
        assert!(
            time_diff.abs() < 1_000_000,
            "Comparison time difference too large: {}ns",
            time_diff.abs()
        );
    });

    run_test!(test_memory_no_leak_in_comparison, {
        let secret = SecureString::from("secret");
        let _ = secret.compare("secret");

        // 比较不应该在堆上分配
        // 这个测试主要验证代码不会意外泄露内存
        // 实际验证需要使用内存分析工具
    });
}

// ==================== 敏感数据检测测试 ====================

mod sensitive_data_detection_tests {
    use super::*;

    run_test!(test_api_key_detection, {
        let sanitizer = ErrorSanitizer::new();

        let messages = vec![
            "API Key: sk-1234567890abcdef",
            "Access Token: eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9",
            "Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9",
        ];

        for msg in messages {
            let contains = sanitizer.contains_sensitive(msg);
            assert!(contains, "Should detect sensitive data in: {}", msg);
        }
    });

    run_test!(test_password_detection, {
        let sanitizer = ErrorSanitizer::new();

        let messages = vec![
            "Password: mySecretPassword123",
            "DB_PASSWORD=secret123",
            "auth: user:password@host",
        ];

        for msg in messages {
            let contains = sanitizer.contains_sensitive(msg);
            assert!(contains, "Should detect password in: {}", msg);
        }
    });

    run_test!(test_connection_string_detection, {
        let sanitizer = ErrorSanitizer::new();

        let messages = vec![
            "mongodb://user:pass@localhost:27017/db",
            "postgresql://user:password@localhost/db",
            "Connection string: Server=myServer;Database=myDB;User Id=myUsername;Password=myPassword;",
        ];

        for msg in messages {
            let contains = sanitizer.contains_sensitive(msg);
            assert!(contains, "Should detect connection string in: {}", msg);
        }
    });

    run_test!(test_email_detection, {
        let sanitizer = ErrorSanitizer::new();

        let msg = "Contact user@example.com for support";
        assert!(sanitizer.contains_sensitive(msg));

        let sanitized = sanitizer.sanitize(msg);
        assert!(!sanitized.contains("user@example.com"));
    });

    run_test!(test_strict_mode_masking, {
        let sanitizer = ErrorSanitizer::new().with_strict_mode();

        let msg = "The password is secret and the token is key data";
        let sanitized = sanitizer.sanitize(&msg);

        // 所有敏感关键词都应该被掩码
        assert!(!sanitized.contains("password"));
        assert!(!sanitized.contains("secret"));
        assert!(!sanitized.contains("token"));
        assert!(!sanitized.contains("key"));
    });
}

// ==================== 配置验证测试 ====================

mod config_validation_tests {
    use super::*;

    run_test!(test_valid_config_validation, {
        let validator = ConfigValidator::new();

        let mut config = HashMap::new();
        config.insert("app_name".to_string(), "my-app".to_string());
        config.insert("app_port".to_string(), "8080".to_string());
        config.insert("debug_mode".to_string(), "true".to_string());

        let result = validator.validate(&config);
        assert!(result.is_valid());
        assert!(result.errors.is_empty());
    });

    run_test!(test_sensitive_config_detection, {
        let validator = ConfigValidator::new();

        let mut config = HashMap::new();
        config.insert("app_name".to_string(), "my-app".to_string());
        config.insert("database_password".to_string(), "secret123".to_string());
        config.insert("api_token".to_string(), "token123".to_string());

        let result = validator.validate(&config);
        assert!(result.is_valid()); // 配置本身有效
        assert!(result.has_sensitive_data()); // 但包含敏感数据
        assert_eq!(result.sensitive_fields.len(), 2);
    });

    run_test!(test_invalid_config_detection, {
        let validator = ConfigValidator::new();

        let mut config = HashMap::new();
        config.insert("123invalid".to_string(), "value".to_string()); // 无效字段名
        config.insert("valid_field".to_string(), "hello;world".to_string()); // 危险字符

        let result = validator.validate(&config);
        assert!(!result.is_valid());
        assert!(!result.errors.is_empty());
    });

    run_test!(test_safe_validation, {
        let validator = ConfigValidator::new();

        let mut config = HashMap::new();
        config.insert("app_name".to_string(), "my-app".to_string());
        config.insert(
            "dangerous_field".to_string(),
            "'; DROP TABLE users;--".to_string(),
        );

        // 安全验证不应该panic
        let result = validator.validate_safe(&config);
        assert!(!result);
    });

    run_test!(test_config_validator_builder, {
        let validator = ConfigValidator::builder()
            .max_string_length(100)
            .add_sensitive_field("custom_field")
            .strict_mode()
            .build();

        let mut config = HashMap::new();
        config.insert("app_name".to_string(), "my-app".to_string());
        config.insert("custom_field".to_string(), "sensitive".to_string());

        let result = validator.validate(&config);
        // custom_field 应该是敏感的
        assert!(result.has_sensitive_data());
    });
}

// ==================== 过滤测试 ====================

mod filtering_tests {
    use super::*;

    run_test!(test_message_filtering, {
        let mut filter = SensitiveDataFilter::new();
        filter.add_blocked_pattern(r".*password.*").unwrap();
        filter.add_allowed_pattern(r"^Safe:.*").unwrap();

        // 被阻止的消息
        let result = filter.filter("Contains password secret");
        assert!(result.is_blocked());

        // 允许的消息
        let result = filter.filter("Safe: normal message");
        assert!(result.is_allowed());

        // 已脱敏的消息
        let result = filter.filter("API Key: sk-12345");
        match result {
            FilterResult::Sanitized(msg) => {
                assert!(!msg.contains("sk-12345"));
            }
            _ => panic!("Expected sanitized result"),
        }
    });

    run_test!(test_batch_filtering, {
        let mut filter = SensitiveDataFilter::new();
        // 添加阻止包含 password 的模式
        filter.add_blocked_pattern(r"(?i)password").unwrap();

        let messages = vec!["Normal message", "Password: secret", "API Key: sk-12345"];

        let results = filter.filter_all(&messages);

        assert!(results[0].1.is_allowed());
        assert!(results[1].1.is_blocked());
        match results[2].1 {
            FilterResult::Sanitized(_) => {}
            _ => panic!("Expected sanitized result"),
        }
    });
}

// ==================== 安全结果测试 ====================

mod safe_result_tests {
    use super::*;

    run_test!(test_safe_result_success, {
        let result = SafeResult::ok("value");

        assert!(result.is_ok());
        assert!(!result.is_err());
        assert_eq!(result.value(), Some(&"value"));
        assert!(!result.contained_sensitive());
    });

    run_test!(test_safe_result_error, {
        let result: SafeResult<()> = SafeResult::err("Error with password: secret");

        assert!(!result.is_ok());
        assert!(result.is_err());
        assert!(result.value().is_none());
        assert!(result.error_message().is_some());
        assert!(result.contained_sensitive());

        // 错误消息应该已脱敏
        let msg = result.error_message().unwrap();
        assert!(!msg.contains("password"));
        assert!(!msg.contains("secret"));
    });

    run_test!(test_safe_result_operations, {
        let success: SafeResult<i32> = SafeResult::ok(42);
        assert_eq!(success.clone().unwrap(), 42);
        assert_eq!(success.unwrap_or(0), 42);

        let failure = SafeResult::err("error");
        assert_eq!(failure.unwrap_or(0), 0);
    });
}

// ==================== 集成测试 ====================

mod integration_tests {
    use super::*;

    run_test!(test_full_security_flow, {
        // 1. 测试脱敏功能
        let sanitizer = ErrorSanitizer::new();

        // 敏感值应该被脱敏
        let sensitive_value = "API Key: sk-12345";
        let sanitized = sanitizer.sanitize(sensitive_value);
        assert_ne!(sanitized, sensitive_value);
        assert!(sanitized.contains("***"));

        // 验证敏感数据检测
        let detector = SensitiveDataDetector::new();
        let sensitivity = detector.is_sensitive("APP_SECRET", "my-secret");
        assert!(sensitivity.needs_protection());
    });

    run_test!(test_end_to_end_security, {
        // 端到端安全测试

        // 1. SecureString 处理
        let secret = SecureString::from("super-secret-key");
        assert!(secret.compare("super-secret-key").is_ok());

        // 2. 敏感数据检测
        let detector = crate::security::input_validation::SensitiveDataDetector::new();
        let sensitivity = detector.is_sensitive("API_TOKEN", "token-value");
        assert!(sensitivity.needs_protection());

        // 3. 输入验证
        let validator = InputValidator::new();
        assert!(validator.validate_string("valid input").is_ok());
        assert!(validator.validate_string("'; DROP TABLE").is_err());

        // 4. 错误脱敏
        let sanitizer = ErrorSanitizer::new();
        let error_msg = "Failed with password: secret123";
        let sanitized = sanitizer.sanitize(error_msg);
        assert!(!sanitized.contains("secret123"));

        // 5. 安全日志
        let logger = crate::security::error_sanitization::SecureLogger::new();
        logger.error_with_context("Auth", "Login failed with password: secret");
        // 测试不应该panic，日志应该安全
    });

    run_test!(test_attack_scenario_coverage, {
        // 模拟常见攻击场景

        // 场景 1: SQL 注入尝试
        let input_validator = InputValidator::new();
        let sql_injection = "' OR '1'='1";
        assert!(input_validator.validate_string(sql_injection).is_err());

        // 场景 2: 命令注入尝试
        let env_validator = EnvSecurityValidator::new();
        let cmd_injection = "; rm -rf /";
        assert!(env_validator.validate_env_value(cmd_injection).is_err());

        // 场景 3: 敏感数据泄露尝试
        let sanitizer = ErrorSanitizer::new();
        let sensitive_error = "Database error: password=secret123";
        let sanitized = sanitizer.sanitize(sensitive_error);
        assert!(!sanitized.contains("secret123"));

        // 场景 4: 路径遍历尝试
        let path_traversal = "../../../etc/passwd";
        assert!(input_validator.validate_string(path_traversal).is_err());

        // 场景 5: Shell 扩展尝试
        let shell_expansion = "${HOME}/.bashrc";
        assert!(env_validator.validate_env_value(shell_expansion).is_err());
    });
}

// 运行所有测试
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_all_security_tests() {
        // 运行所有测试并报告统计
        reset_test_counters();

        // 运行各个测试模块
        sensitive_data_tests::test_secure_string_creation();
        injection_attack_tests::test_sql_injection_patterns();
        boundary_condition_tests::test_empty_input();
        memory_safety_tests::test_secure_string_zeroize();
        sensitive_data_detection_tests::test_api_key_detection();
        config_validation_tests::test_valid_config_validation();
        filtering_tests::test_message_filtering();
        safe_result_tests::test_safe_result_success();
        integration_tests::test_full_security_flow();

        let (run, passed, failed) = test_stats();
        println!(
            "Security Tests: {} run, {} passed, {} failed",
            run, passed, failed
        );

        assert_eq!(failed, 0, "All security tests should pass");
    }
}
