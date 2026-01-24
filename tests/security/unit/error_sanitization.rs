// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! 单元测试：错误消息脱敏功能
//!
//! 测试ErrorSanitizer的各种功能，包括敏感信息脱敏、安全日志输出等

#[cfg(test)]
mod error_sanitizer_creation_tests {
    use super::super::*;

    /// 测试创建错误脱敏器
    #[test]
    fn test_error_sanitizer_creation() {
        let sanitizer = ErrorSanitizer::new();
        assert!(!sanitizer.strict_mode());
    }

    /// 测试创建严格模式的脱敏器
    #[test]
    fn test_strict_mode_sanitizer() {
        let sanitizer = ErrorSanitizer::new().with_strict_mode(true);
        assert!(sanitizer.strict_mode());
    }

    /// 测试创建带自定义规则的脱敏器
    #[test]
    fn test_sanitizer_with_custom_rules() {
        let mut sanitizer = ErrorSanitizer::new();
        sanitizer.add_custom_rule(r"custom-\d+", "***CUSTOM***");

        let sanitized = sanitizer.sanitize("Error: custom-12345 failed");
        assert!(sanitized.contains("***CUSTOM***"));
    }
}

#[cfg(test)]
mod error_sanitization_tests {
    use super::super::*;

    /// 测试脱敏API密钥
    #[test]
    fn test_sanitize_api_key() {
        let sanitizer = ErrorSanitizer::new();

        let error = "API request failed: api_key=sk-123456789abcdef";
        let sanitized = sanitizer.sanitize(error);

        assert!(!sanitized.contains("sk-123456789abcdef"));
        assert!(sanitized.contains("api_key"));
    }

    /// 测试脱敏密码
    #[test]
    fn test_sanitize_password() {
        let sanitizer = ErrorSanitizer::new();

        let error = "Authentication failed: password=secret123";
        let sanitized = sanitizer.sanitize(error);

        assert!(!sanitized.contains("secret123"));
        assert!(sanitized.contains("password"));
    }

    /// 测试脱敏令牌
    #[test]
    fn test_sanitize_tokens() {
        let sanitizer = ErrorSanitizer::new();

        let error = "Token validation error: access_token=eyJhbGciOiJIUzI1NiJ9";
        let sanitized = sanitizer.sanitize(error);

        assert!(!sanitized.contains("eyJhbGciOiJIUzI1NiJ9"));
        assert!(sanitized.contains("access_token"));
    }

    /// 测试脱敏Bearer令牌
    #[test]
    fn test_sanitize_bearer_token() {
        let sanitizer = ErrorSanitizer::new();

        let error = "Authorization failed: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9";
        let sanitized = sanitizer.sanitize(error);

        assert!(!sanitized.contains("eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9"));
        assert!(sanitized.contains("Bearer"));
    }

    /// 测试脱敏连接字符串
    #[test]
    fn test_sanitize_connection_string() {
        let sanitizer = ErrorSanitizer::new();

        let error =
            "Database connection failed: database_url=postgresql://user:pass@localhost:5432/db";
        let sanitized = sanitizer.sanitize(error);

        assert!(!sanitized.contains("postgresql://user:pass@localhost:5432/db"));
        assert!(sanitized.contains("***CONNECTION_STRING***"));
    }

    /// 测试脱敏MongoDB连接字符串
    #[test]
    fn test_sanitize_mongodb_connection() {
        let sanitizer = ErrorSanitizer::new();

        let error = "MongoDB connection failed: connection_string=mongodb+srv://admin:password123@cluster.mongodb.net/test?retryWrites=true";
        let sanitized = sanitizer.sanitize(error);

        assert!(!sanitized.contains("password123"));
        assert!(sanitized.contains("***CONNECTION_STRING***"));
    }

    /// 测试脱敏敏感密钥
    #[test]
    fn test_sanitize_secret_key() {
        let sanitizer = ErrorSanitizer::new();

        let error = "Encryption error: secret_key=sk_live_abcdefghijklmnop";
        let sanitized = sanitizer.sanitize(error);

        assert!(!sanitized.contains("sk_live_abcdefghijklmnop"));
        assert!(sanitized.contains("secret_key"));
    }

    /// 测试脱敏私钥
    #[test]
    fn test_sanitize_private_key() {
        let sanitizer = ErrorSanitizer::new();

        let error = "SSL error: private_key=-----BEGIN RSA PRIVATE KEY-----\nMIIEowIBAAKCAQEA0Z3VS5JJcds3xfn/ygWyF8DtQ8j9WdaLrtWVWbmZmRF\n...-----END RSA PRIVATE KEY-----";
        let sanitized = sanitizer.sanitize(error);

        assert!(!sanitized.contains("BEGIN RSA PRIVATE KEY"));
        assert!(sanitized.contains("private_key"));
    }

    /// 测试脱敏基本认证
    #[test]
    fn test_sanitize_basic_auth() {
        let sanitizer = ErrorSanitizer::new();

        let error = "HTTP error: Basic dXNlcm5hbWU6cGFzc3dvcmQ=";
        let sanitized = sanitizer.sanitize(error);

        assert!(!sanitized.contains("dXNlcm5hbWU6cGFzc3dvcmQ="));
        assert!(sanitized.contains("Basic"));
    }

    /// 测试脱敏邮箱地址
    #[test]
    fn test_sanitize_email() {
        let sanitizer = ErrorSanitizer::new();

        let error = "User registration failed for user@example.com";
        let sanitized = sanitizer.sanitize(error);

        assert!(!sanitized.contains("user@example.com"));
    }

    /// 测试脱敏IP地址
    #[test]
    fn test_sanitize_ip_address() {
        let sanitizer = ErrorSanitizer::new();

        let error = "Connection from 192.168.1.100 was rejected";
        let sanitized = sanitizer.sanitize(error);

        assert!(!sanitized.contains("192.168.1.100"));
        assert!(sanitized.contains("***IP_ADDRESS***"));
    }
}

#[cfg(test)]
mod sensitive_data_filter_tests {
    use super::super::*;

    /// 测试创建敏感数据过滤器
    #[test]
    fn test_sensitive_data_filter_creation() {
        let filter = SensitiveDataFilter::new();
        assert!(filter.is_empty());
    }

    /// 测试添加阻止模式
    #[test]
    fn test_add_blocked_pattern() {
        let mut filter = SensitiveDataFilter::new();
        filter.add_blocked_pattern(r"(?i)password").unwrap();

        let result = filter.filter("Enter your password here");
        assert!(result.1.is_blocked());
    }

    /// 测试添加允许模式
    #[test]
    fn test_add_allowed_pattern() {
        let mut filter = SensitiveDataFilter::new();
        filter.add_blocked_pattern(r"(?i)password").unwrap();
        filter.add_allowed_pattern(r"password_policy").unwrap();

        let result = filter.filter("Password must meet password_policy requirements");
        assert!(result.1.is_allowed());
    }

    /// 测试批量过滤
    #[test]
    fn test_batch_filtering() {
        let mut filter = SensitiveDataFilter::new();
        filter.add_blocked_pattern(r"(?i)password").unwrap();
        filter.add_blocked_pattern(r"(?i)secret").unwrap();

        let messages = vec![
            "Normal message",
            "Password: secret123",
            "API Key: sk-12345",
            "Just a test",
        ];

        let results = filter.filter_all(&messages);

        assert!(results[0].1.is_allowed());
        assert!(results[1].1.is_blocked());
        assert!(results[2].1.is_allowed()); // secret not in message
        assert!(results[3].1.is_allowed());
    }
}

#[cfg(test)]
mod security_context_tests {
    use super::super::*;

    /// 测试创建安全上下文
    #[test]
    fn test_security_context_creation() {
        let context = SecurityContext::new();
        assert!(!context.has_sensitive_data());
    }

    /// 测试添加敏感数据到上下文
    #[test]
    fn test_add_sensitive_data_to_context() {
        let mut context = SecurityContext::new();

        context.add_sensitive_field("password");
        context.add_sensitive_field("api_key");

        assert!(context.has_sensitive_data());
        assert!(context.is_sensitive_field("password"));
        assert!(context.is_sensitive_field("api_key"));
        assert!(!context.is_sensitive_field("name"));
    }

    /// 测试检查字段是否敏感
    #[test]
    fn test_check_sensitive_field() {
        let mut context = SecurityContext::new();

        context.add_sensitive_field("token");

        assert!(context.is_sensitive_field("token"));
        assert!(context.is_sensitive_field("TOKEN"));
        assert!(context.is_sensitive_field("Token"));
        assert!(!context.is_sensitive_field("username"));
    }
}
