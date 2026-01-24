// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! 单元测试：输入验证功能
//!
//! 测试输入验证器的各种功能，包括长度限制、敏感数据检测、危险模式拦截等

#[cfg(test)]
mod sensitive_data_detector_tests {
    use super::super::*;
    use std::collections::HashMap;

    /// 测试创建敏感数据检测器
    #[test]
    fn test_sensitive_data_detector_creation() {
        let detector = SensitiveDataDetector::new();
        assert!(!detector.high_sensitivity_keywords.is_empty());
        assert!(detector.sensitive_patterns.len() > 0);
    }

    /// 测试检测低敏感度数据
    #[test]
    fn test_detect_low_sensitivity_data() {
        let detector = SensitiveDataDetector::new();

        let result = detector.is_sensitive("username", "john_doe");
        assert!(result.is_low());

        let result = detector.is_sensitive("app_name", "my_application");
        assert!(result.is_low());

        let result = detector.is_sensitive("color", "blue");
        assert!(result.is_low());
    }

    /// 测试检测高敏感度关键词
    #[test]
    fn test_detect_high_sensitivity_keywords() {
        let detector = SensitiveDataDetector::new();

        // 测试password关键词
        let result = detector.is_sensitive("password", "secret123");
        assert!(!result.is_low());
        assert!(result.needs_protection());

        // 测试api_key关键词
        let result = detector.is_sensitive("api_key", "sk-123456");
        assert!(!result.is_low());

        // 测试secret关键词
        let result = detector.is_sensitive("secret_token", "value123");
        assert!(!result.is_low());
    }

    /// 测试检测中敏感度模式
    #[test]
    fn test_detect_medium_sensitivity_patterns() {
        let detector = SensitiveDataDetector::new();

        // 测试包含token的值
        let result = detector.is_sensitive("description", "This contains a token value");
        assert!(!result.is_low());

        // 测试包含key的值
        let result = detector.is_sensitive("note", "The api key is stored securely");
        assert!(!result.is_low());
    }

    /// 测试自定义敏感字段
    #[test]
    fn test_custom_sensitive_fields() {
        let mut detector = SensitiveDataDetector::new();

        // 添加自定义敏感字段
        detector.add_custom_sensitive_field("internal_id");
        detector.add_custom_sensitive_field("tracking_code");

        // 测试自定义字段
        let result = detector.is_sensitive("internal_id", "value123");
        assert!(!result.is_low());
        assert!(result.needs_protection());

        let result = detector.is_sensitive("tracking_code", "abc123");
        assert!(!result.is_low());

        // 确保普通字段不受影响
        let result = detector.is_sensitive("name", "test");
        assert!(result.is_low());
    }

    /// 测试批量检测敏感数据
    #[test]
    fn test_batch_detection() {
        let detector = SensitiveDataDetector::new();

        let mut data = HashMap::new();
        data.insert("username".to_string(), "john_doe".to_string());
        data.insert("password".to_string(), "secret123".to_string());
        data.insert("app_name".to_string(), "myapp".to_string());
        data.insert("api_key".to_string(), "sk-123456".to_string());

        let results = detector.detect_all(&data);

        // 应该检测到password和api_key
        assert_eq!(results.len(), 2);

        let fields: Vec<&str> = results.iter().map(|(k, _)| *k).collect();
        assert!(fields.contains(&"password"));
        assert!(fields.contains(&"api_key"));
    }

    /// 测试敏感度结果描述
    #[test]
    fn test_sensitivity_result_description() {
        let low = SensitivityResult::Low;
        assert_eq!(low.description(), "low sensitivity");

        let medium = SensitivityResult::Medium {
            field: "password".to_string(),
            reason: "test reason".to_string(),
        };
        assert!(medium.description().contains("medium sensitivity"));
        assert!(medium.description().contains("password"));

        let high = SensitivityResult::High {
            field: "password".to_string(),
            reason: "high sensitivity keyword".to_string(),
        };
        assert!(high.description().contains("high sensitivity"));
    }
}

#[cfg(test)]
mod input_validator_tests {
    use super::super::*;
    use serde_json::json;

    /// 测试创建输入验证器
    #[test]
    fn test_input_validator_creation() {
        let validator = InputValidator::new();
        assert_eq!(validator.max_string_length, 1024);
        assert_eq!(validator.max_array_length, 100);
        assert_eq!(validator.max_depth, 10);
    }

    /// 测试设置最大字符串长度
    #[test]
    fn test_max_string_length() {
        let validator = InputValidator::new().with_max_string_length(2048);
        assert_eq!(validator.max_string_length, 2048);
    }

    /// 测试设置最大数组长度
    #[test]
    fn test_max_array_length() {
        let validator = InputValidator::new().with_max_array_length(200);
        assert_eq!(validator.max_array_length, 200);
    }

    /// 测试设置最大深度
    #[test]
    fn test_max_depth() {
        let validator = InputValidator::new().with_max_depth(20);
        assert_eq!(validator.max_depth, 20);
    }

    /// 测试验证有效输入
    #[test]
    fn test_validate_valid_input() {
        let validator = InputValidator::new();
        let result = validator.validate_input("valid input", "test_field");
        assert!(result.is_ok());
    }

    /// 测试拦截shell元字符
    #[test]
    fn test_reject_shell_metacharacters() {
        let validator = InputValidator::new();

        // 测试分号
        let result = validator.validate_input("value; ls", "test");
        assert!(result.is_err());

        // 测试管道符
        let result = validator.validate_input("value | cat", "test");
        assert!(result.is_err());

        // 测试反引号
        let result = validator.validate_input("value `ls`", "test");
        assert!(result.is_err());

        // 测试美元符
        let result = validator.validate_input("value $HOME", "test");
        assert!(result.is_err());
    }

    /// 测试拦截SQL注入模式
    #[test]
    fn test_reject_sql_injection() {
        let validator = InputValidator::new();

        // 测试DROP语句
        let result = validator.validate_input("DROP TABLE users", "sql");
        assert!(result.is_err());

        // 测试UNION SELECT
        let result = validator.validate_input("' OR '1'='1' --", "sql");
        assert!(result.is_err());

        // 测试DELETE语句
        let result = validator.validate_input("DELETE FROM table", "sql");
        assert!(result.is_err());
    }

    /// 测试拦截路径遍历
    #[test]
    fn test_reject_path_traversal() {
        let validator = InputValidator::new();

        // 测试../
        let result = validator.validate_input("../../../etc/passwd", "path");
        assert!(result.is_err());

        // 测试..\
        let result = validator.validate_input("..\\..\\windows\\system32", "path");
        assert!(result.is_err());
    }

    /// 测试危险模式拦截
    #[test]
    fn test_reject_dangerous_patterns() {
        let validator = InputValidator::new();

        // 测试&&
        let result = validator.validate_input("cmd && echo hack", "cmd");
        assert!(result.is_err());

        // 测试||
        let result = validator.validate_input("cmd || echo hack", "cmd");
        assert!(result.is_err());

        // 测试>>
        let result = validator.validate_input("echo >> /tmp/file", "cmd");
        assert!(result.is_err());
    }

    /// 测试白名单模式
    #[test]
    fn test_whitelist_patterns() {
        let validator = InputValidator::new().with_allowed_chars_pattern(r"^[a-zA-Z0-9_-]+$");

        // 有效字符应该通过
        let result = validator.validate_input("valid-name_123", "field");
        assert!(result.is_ok());

        // 无效字符应该被拒绝
        let result = validator.validate_input("invalid@name!", "field");
        assert!(result.is_err());
    }

    /// 测试配置值验证
    #[test]
    fn test_validate_config_value() {
        let validator = InputValidator::new();

        let valid_config = json!({
            "name": "test_app",
            "port": 8080
        });

        let result = validator.validate_config_value(&valid_config, "app_config");
        assert!(result.is_ok());
    }
}
