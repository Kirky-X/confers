// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! # 输入验证增强
//!
//! 提供增强的输入验证功能，包括长度限制、敏感数据自动检测和输入清理。
//!
//! ## 功能特性
//!
//! - **长度限制**: 强制实施输入长度限制
//! - **敏感数据检测**: 自动检测和标记敏感数据
//! - **输入清理**: 提供输入清理和验证功能
//! - **白名单验证**: 支持白名单格式验证

use crate::security::patterns::{SENSITIVE_DETECTION_PATTERNS, SENSITIVE_KEYWORDS};
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

/// 默认危险模式 - 全局缓存
static DEFAULT_DANGEROUS_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    vec![
        Regex::new(r"[;<>&|`$()]").unwrap(),
        Regex::new(r"\$\{.*\}").unwrap(),
        Regex::new(r"`[^`]+`").unwrap(),
        Regex::new(r"\|").unwrap(),
        Regex::new(r"&&").unwrap(),
        Regex::new(r"\|\|").unwrap(),
        Regex::new(r">>").unwrap(),
        Regex::new(r"2>").unwrap(),
        Regex::new(r"\.\.[/\\]").unwrap(),
        Regex::new(r"[/\\]\.\.[/\\]").unwrap(),
        Regex::new(r"(?i)(;?\s*(drop|delete|update|insert|alter|create)\b)").unwrap(),
        Regex::new(r"(?i)(union\s+select\b)").unwrap(),
        Regex::new(r"(?i)'+\s*(or|and)\b").unwrap(),
        Regex::new(r"(?i)--\s*$").unwrap(),
    ]
});

/// 敏感数据检测器
#[derive(Debug, Clone)]
pub struct SensitiveDataDetector {
    /// 敏感模式列表
    sensitive_patterns: Vec<Regex>,
    /// 高敏感度关键词
    high_sensitivity_keywords: HashSet<&'static str>,
    /// 自定义敏感字段
    custom_sensitive_fields: HashSet<String>,
}

impl Default for SensitiveDataDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl SensitiveDataDetector {
    /// 创建新的检测器
    pub fn new() -> Self {
        Self {
            sensitive_patterns: Self::default_sensitive_patterns(),
            high_sensitivity_keywords: Self::default_high_sensitivity_keywords(),
            custom_sensitive_fields: HashSet::new(),
        }
    }

    /// 默认敏感模式
    fn default_sensitive_patterns() -> Vec<Regex> {
        SENSITIVE_DETECTION_PATTERNS.clone()
    }

    /// 默认高敏感度关键词
    fn default_high_sensitivity_keywords() -> HashSet<&'static str> {
        SENSITIVE_KEYWORDS.clone()
    }

    /// 添加自定义敏感字段
    pub fn add_custom_sensitive_field(&mut self, field: impl Into<String>) {
        self.custom_sensitive_fields
            .insert(field.into().to_lowercase());
    }

    /// 检查是否为敏感数据
    pub fn is_sensitive(&self, field_name: &str, field_value: &str) -> SensitivityResult {
        let field_lower = field_name.to_lowercase();
        let value_lower = field_value.to_lowercase();

        // 检查字段名是否在敏感列表中
        if self
            .high_sensitivity_keywords
            .contains(field_lower.as_str())
        {
            return SensitivityResult::High {
                field: field_name.to_string(),
                reason: "high sensitivity keyword in field name".to_string(),
            };
        }

        // 检查自定义敏感字段
        if self.custom_sensitive_fields.contains(&field_lower) {
            return SensitivityResult::Medium {
                field: field_name.to_string(),
                reason: "custom sensitive field".to_string(),
            };
        }

        // 检查字段值是否匹配敏感模式
        for pattern in &self.sensitive_patterns {
            if pattern.is_match(&field_lower) || pattern.is_match(&value_lower) {
                return SensitivityResult::Medium {
                    field: field_name.to_string(),
                    reason: format!("sensitive pattern detected: {}", pattern.as_str()),
                };
            }
        }

        SensitivityResult::Low
    }

    /// 批量检测敏感数据
    pub fn detect_all<'a>(
        &self,
        data: &'a HashMap<String, String>,
    ) -> Vec<(&'a str, SensitivityResult)> {
        data.iter()
            .map(|(k, v)| (k.as_str(), self.is_sensitive(k, v)))
            .filter(|(_, result)| !result.is_low())
            .collect()
    }
}

/// 敏感度检测结果
#[derive(Debug, Clone, PartialEq)]
pub enum SensitivityResult {
    /// 低敏感度
    Low,
    /// 中敏感度
    Medium { field: String, reason: String },
    /// 高敏感度
    High { field: String, reason: String },
}

impl SensitivityResult {
    /// 检查是否为低敏感度
    pub fn is_low(&self) -> bool {
        matches!(self, SensitivityResult::Low)
    }

    /// 检查是否需要保护
    pub fn needs_protection(&self) -> bool {
        !self.is_low()
    }

    /// 获取描述
    pub fn description(&self) -> String {
        match self {
            SensitivityResult::Low => "low sensitivity".to_string(),
            SensitivityResult::Medium { field, reason } => {
                format!("medium sensitivity: {} - {}", field, reason)
            }
            SensitivityResult::High { field, reason } => {
                format!("high sensitivity: {} - {}", field, reason)
            }
        }
    }
}

/// 输入验证器
#[derive(Debug, Clone)]
pub struct InputValidator {
    /// 最大字符串长度
    max_string_length: usize,
    /// 最大数组长度
    max_array_length: usize,
    /// 最大深度
    max_depth: usize,
    /// 允许的字符模式
    allowed_chars_pattern: Option<Regex>,
    /// 危险模式列表
    dangerous_patterns: Vec<Regex>,
    /// 白名单格式
    whitelist_patterns: Vec<Regex>,
}

impl Default for InputValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl InputValidator {
    /// 创建新的验证器
    pub fn new() -> Self {
        Self {
            max_string_length: 1024,
            max_array_length: 100,
            max_depth: 10,
            allowed_chars_pattern: None,
            dangerous_patterns: Self::default_dangerous_patterns(),
            whitelist_patterns: Vec::new(),
        }
    }

    /// 默认危险模式
    fn default_dangerous_patterns() -> Vec<Regex> {
        DEFAULT_DANGEROUS_PATTERNS.clone()
    }

    /// 设置最大字符串长度
    pub fn with_max_string_length(mut self, length: usize) -> Self {
        self.max_string_length = length;
        self
    }

    /// 设置最大数组长度
    pub fn with_max_array_length(mut self, length: usize) -> Self {
        self.max_array_length = length;
        self
    }

    /// 设置最大深度
    pub fn with_max_depth(mut self, depth: usize) -> Self {
        self.max_depth = depth;
        self
    }

    /// 设置允许的字符模式
    pub fn with_allowed_chars_pattern(mut self, pattern: &str) -> Self {
        self.allowed_chars_pattern = Regex::new(pattern).ok();
        self
    }

    /// 添加白名单格式
    pub fn add_whitelist_pattern(mut self, pattern: &str) -> Self {
        if let Ok(regex) = Regex::new(pattern) {
            self.whitelist_patterns.push(regex);
        }
        self
    }

    /// 验证字符串
    pub fn validate_string(&self, value: &str) -> Result<(), InputValidationError> {
        // 检查长度
        if value.len() > self.max_string_length {
            return Err(InputValidationError::TooLong {
                max: self.max_string_length,
                actual: value.len(),
            });
        }

        // 检查允许的字符
        if let Some(ref pattern) = self.allowed_chars_pattern {
            if !pattern.is_match(value) {
                return Err(InputValidationError::InvalidCharacters);
            }
        }

        // 检查危险模式
        for pattern in &self.dangerous_patterns {
            if pattern.is_match(value) {
                return Err(InputValidationError::DangerousPattern {
                    pattern: pattern.as_str().to_string(),
                });
            }
        }

        Ok(())
    }

    /// 验证并清理字符串
    pub fn sanitize_string(&self, value: &str) -> Result<String, InputValidationError> {
        // 先清理危险字符
        let mut result = String::new();
        for c in value.chars() {
            if !self.is_dangerous_char(c) {
                result.push(c);
            }
        }

        // 验证清理后的结果
        self.validate_string(&result)?;

        Ok(result)
    }

    /// 检查字符是否为危险字符
    fn is_dangerous_char(&self, c: char) -> bool {
        matches!(
            c,
            ';' | '<' | '>' | '&' | '|' | '`' | '$' | '(' | ')' | '\0' | '{' | '}'
        )
    }

    /// 验证字段名称
    pub fn validate_field_name(&self, name: &str) -> Result<(), InputValidationError> {
        if name.is_empty() {
            return Err(InputValidationError::EmptyFieldName);
        }

        if name.len() > self.max_string_length {
            return Err(InputValidationError::TooLong {
                max: self.max_string_length,
                actual: name.len(),
            });
        }

        // 字段名只能包含字母、数字、下划线和连字符
        let valid_pattern = Regex::new(r"^[a-zA-Z][a-zA-Z0-9_-]*$").unwrap();
        if !valid_pattern.is_match(name) {
            return Err(InputValidationError::InvalidFieldNameFormat);
        }

        Ok(())
    }

    /// 验证 URL
    pub fn validate_url(&self, url: &str) -> Result<(), InputValidationError> {
        // 检查长度
        if url.len() > self.max_string_length {
            return Err(InputValidationError::TooLong {
                max: self.max_string_length,
                actual: url.len(),
            });
        }

        // 解析 URL
        let parsed = url::Url::parse(url).map_err(|_| InputValidationError::InvalidUrl)?;

        // 只允许 HTTP/HTTPS
        if !matches!(parsed.scheme(), "http" | "https") {
            return Err(InputValidationError::InvalidUrlScheme);
        }

        // 检查危险模式
        for pattern in &self.dangerous_patterns {
            if pattern.is_match(url) {
                return Err(InputValidationError::DangerousPattern {
                    pattern: pattern.as_str().to_string(),
                });
            }
        }

        Ok(())
    }

    /// 验证电子邮件
    pub fn validate_email(&self, email: &str) -> Result<(), InputValidationError> {
        if email.len() > self.max_string_length {
            return Err(InputValidationError::TooLong {
                max: self.max_string_length,
                actual: email.len(),
            });
        }

        let email_pattern =
            Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap();
        if !email_pattern.is_match(email) {
            return Err(InputValidationError::InvalidEmail);
        }

        Ok(())
    }

    /// 验证白名单格式
    pub fn validate_whitelist(&self, value: &str) -> Result<(), InputValidationError> {
        if self.whitelist_patterns.is_empty() {
            return Ok(());
        }

        for pattern in &self.whitelist_patterns {
            if pattern.is_match(value) {
                return Ok(());
            }
        }

        Err(InputValidationError::NotInWhitelist)
    }

    /// 批量验证
    pub fn validate_all<'a>(
        &'a self,
        data: &'a HashMap<String, String>,
    ) -> Vec<(&'a String, InputValidationError)> {
        let mut errors = Vec::new();

        for (name, value) in data {
            if let Err(e) = self.validate_field_name(name) {
                errors.push((name, e));
                continue;
            }

            if let Err(e) = self.validate_string(value) {
                errors.push((name, e));
            }
        }

        errors
    }
}

/// 输入验证错误
#[derive(Debug, Clone, PartialEq)]
pub enum InputValidationError {
    /// 字符串太长
    TooLong { max: usize, actual: usize },
    /// 包含无效字符
    InvalidCharacters,
    /// 检测到危险模式
    DangerousPattern { pattern: String },
    /// 字段名为空
    EmptyFieldName,
    /// 字段名格式无效
    InvalidFieldNameFormat,
    /// URL 无效
    InvalidUrl,
    /// URL 方案无效
    InvalidUrlScheme,
    /// 电子邮件无效
    InvalidEmail,
    /// 不在白名单中
    NotInWhitelist,
    /// 深度超出限制
    DepthExceeded { max: usize, actual: usize },
    /// 数组太长
    ArrayTooLong { max: usize, actual: usize },
}

impl std::fmt::Display for InputValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InputValidationError::TooLong { max, actual } => {
                write!(f, "Input too long: max={}, actual={}", max, actual)
            }
            InputValidationError::InvalidCharacters => {
                write!(f, "Input contains invalid characters")
            }
            InputValidationError::DangerousPattern { pattern } => {
                write!(f, "Input contains dangerous pattern: {}", pattern)
            }
            InputValidationError::EmptyFieldName => {
                write!(f, "Field name is empty")
            }
            InputValidationError::InvalidFieldNameFormat => {
                write!(f, "Field name format is invalid")
            }
            InputValidationError::InvalidUrl => {
                write!(f, "URL is invalid")
            }
            InputValidationError::InvalidUrlScheme => {
                write!(f, "URL scheme is not allowed")
            }
            InputValidationError::InvalidEmail => {
                write!(f, "Email format is invalid")
            }
            InputValidationError::NotInWhitelist => {
                write!(f, "Input does not match any whitelist pattern")
            }
            InputValidationError::DepthExceeded { max, actual } => {
                write!(f, "Nesting depth exceeded: max={}, actual={}", max, actual)
            }
            InputValidationError::ArrayTooLong { max, actual } => {
                write!(f, "Array too long: max={}, actual={}", max, actual)
            }
        }
    }
}

impl std::error::Error for InputValidationError {}

/// 配置验证器构建器
#[derive(Default)]
pub struct ConfigValidatorBuilder {
    validator: InputValidator,
    sensitive_detector: SensitiveDataDetector,
}

impl ConfigValidatorBuilder {
    /// 创建新的构建器
    pub fn new() -> Self {
        Self::default()
    }

    /// 构建验证器
    pub fn build(self) -> ConfigValidator {
        ConfigValidator {
            input_validator: self.validator,
            sensitive_detector: self.sensitive_detector,
        }
    }

    /// 设置最大字符串长度
    pub fn max_string_length(mut self, length: usize) -> Self {
        self.validator = self.validator.with_max_string_length(length);
        self
    }

    /// 添加自定义敏感字段
    pub fn add_sensitive_field(mut self, field: &str) -> Self {
        self.sensitive_detector.add_custom_sensitive_field(field);
        self
    }

    /// 启用严格模式
    pub fn strict_mode(self) -> Self {
        self.max_string_length(256)
            .add_sensitive_field("token")
            .add_sensitive_field("password")
    }
}

/// 配置验证器
#[derive(Debug, Clone)]
pub struct ConfigValidator {
    input_validator: InputValidator,
    sensitive_detector: SensitiveDataDetector,
}

impl ConfigValidator {
    /// 创建新的验证器
    pub fn new() -> Self {
        Self {
            input_validator: InputValidator::new(),
            sensitive_detector: SensitiveDataDetector::new(),
        }
    }

    /// 使用构建器创建
    pub fn builder() -> ConfigValidatorBuilder {
        ConfigValidatorBuilder::new()
    }

    /// 获取敏感数据检测器（用于测试）
    #[cfg(test)]
    pub fn sensitive_detector(&self) -> &SensitiveDataDetector {
        &self.sensitive_detector
    }

    /// 验证配置数据
    pub fn validate(&self, data: &HashMap<String, String>) -> ConfigValidationResult {
        let mut errors = Vec::new();
        let mut sensitive_fields = Vec::new();

        for (name, value) in data {
            // 验证字段名
            if let Err(e) = self.input_validator.validate_field_name(name) {
                errors.push(ConfigValidationError::FieldError {
                    field: name.clone(),
                    error: e,
                });
            }

            // 验证字段值
            if let Err(e) = self.input_validator.validate_string(value) {
                errors.push(ConfigValidationError::FieldError {
                    field: name.clone(),
                    error: e,
                });
            }

            // 检测敏感数据
            let sensitivity = self.sensitive_detector.is_sensitive(name, value);
            if sensitivity.needs_protection() {
                sensitive_fields.push((name.clone(), sensitivity));
            }
        }

        ConfigValidationResult {
            errors,
            sensitive_fields,
        }
    }

    /// 安全地验证配置（不返回详细错误信息）
    pub fn validate_safe(&self, data: &HashMap<String, String>) -> bool {
        data.iter().all(|(name, value)| {
            self.input_validator.validate_field_name(name).is_ok()
                && self.input_validator.validate_string(value).is_ok()
        })
    }
}

impl Default for ConfigValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// 配置验证结果
#[derive(Debug, Clone)]
pub struct ConfigValidationResult {
    /// 验证错误列表
    pub errors: Vec<ConfigValidationError>,
    /// 敏感字段列表
    pub sensitive_fields: Vec<(String, SensitivityResult)>,
}

impl ConfigValidationResult {
    /// 检查是否有效
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }

    /// 检查是否有敏感数据
    pub fn has_sensitive_data(&self) -> bool {
        !self.sensitive_fields.is_empty()
    }

    /// 获取错误报告
    pub fn error_report(&self) -> String {
        self.errors
            .iter()
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// 配置验证错误
#[derive(Debug, Clone)]
pub enum ConfigValidationError {
    /// 字段错误
    FieldError {
        field: String,
        error: InputValidationError,
    },
    /// 敏感数据警告
    SensitiveDataWarning {
        field: String,
        sensitivity: SensitivityResult,
    },
}

impl std::fmt::Display for ConfigValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigValidationError::FieldError { field, error } => {
                write!(f, "Field '{}': {}", field, error)
            }
            ConfigValidationError::SensitiveDataWarning { field, sensitivity } => {
                write!(f, "Field '{}': {}", field, sensitivity.description())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sensitive_data_detection() {
        let detector = SensitiveDataDetector::new();

        // 高敏感度字段
        assert!(detector
            .is_sensitive("password", "value")
            .needs_protection());
        assert!(detector
            .is_sensitive("secret_key", "value")
            .needs_protection());

        // 中敏感度字段
        assert!(detector
            .is_sensitive("api_token", "value")
            .needs_protection());
        assert!(detector
            .is_sensitive("user_token", "value")
            .needs_protection());

        // 低敏感度字段
        assert!(!detector
            .is_sensitive("username", "value")
            .needs_protection());
        assert!(!detector.is_sensitive("port", "8080").needs_protection());
    }

    #[test]
    fn test_input_validation() {
        let validator = InputValidator::new();

        // 有效输入
        assert!(validator.validate_string("hello world").is_ok());
        assert!(validator.validate_field_name("app_port").is_ok());

        // 无效输入
        assert!(validator.validate_string("hello;world").is_err());
        assert!(validator.validate_string("hello${world}").is_err());
        assert!(validator.validate_field_name("").is_err());
        assert!(validator.validate_field_name("123port").is_err());
    }

    #[test]
    fn test_url_validation() {
        let validator = InputValidator::new();

        // 有效 URL
        assert!(validator.validate_url("https://example.com").is_ok());
        assert!(validator.validate_url("http://localhost:8080").is_ok());

        // 无效 URL
        assert!(validator.validate_url("ftp://example.com").is_err());
        assert!(validator.validate_url("javascript:alert(1)").is_err());
    }

    #[test]
    fn test_email_validation() {
        let validator = InputValidator::new();

        // 有效邮箱
        assert!(validator.validate_email("user@example.com").is_ok());

        // 无效邮箱
        assert!(validator.validate_email("invalid-email").is_err());
        assert!(validator.validate_email("@example.com").is_err());
    }

    #[test]
    fn test_sanitization() {
        let validator = InputValidator::new();

        let input = "hello; world ${test}";
        let sanitized = validator.sanitize_string(input).unwrap();

        assert!(!sanitized.contains(';'));
        assert!(!sanitized.contains('$'));
        assert_eq!(sanitized, "hello world test");
    }

    #[test]
    fn test_config_validation() {
        let validator = ConfigValidator::new();

        let mut config = HashMap::new();
        config.insert("app_name".to_string(), "my-app".to_string());
        config.insert("app_port".to_string(), "8080".to_string());
        config.insert("database_password".to_string(), "secret".to_string());

        let result = validator.validate(&config);

        assert!(result.is_valid());
        assert!(result.has_sensitive_data());
        assert_eq!(result.sensitive_fields.len(), 1);
    }

    #[test]
    fn test_custom_sensitive_field() {
        let detector = SensitiveDataDetector::new();
        let mut custom_detector = detector.clone();
        custom_detector.add_custom_sensitive_field("custom_field");

        assert!(custom_detector
            .is_sensitive("custom_field", "value")
            .needs_protection());
    }

    #[test]
    fn test_detector_default_and_clone() {
        let detector = SensitiveDataDetector::default();
        // 高敏感度关键词命中
        let high = detector.is_sensitive("password", "v");
        assert!(matches!(high, SensitivityResult::High { .. }));
        // 克隆后行为一致
        let cloned = detector.clone();
        assert!(cloned.is_sensitive("token", "v").needs_protection());
        // 非敏感字段
        assert!(detector.is_sensitive("name", "value").is_low());
    }

    #[test]
    fn test_sensitivity_result_variants_and_description() {
        let low = SensitivityResult::Low;
        assert!(low.is_low());
        assert!(!low.needs_protection());
        assert_eq!(low.description(), "low sensitivity");

        let medium = SensitivityResult::Medium {
            field: "api_token".to_string(),
            reason: "custom".to_string(),
        };
        assert!(!medium.is_low());
        assert!(medium.needs_protection());
        assert!(medium.description().contains("medium sensitivity"));
        assert!(medium.description().contains("api_token"));

        let high = SensitivityResult::High {
            field: "password".to_string(),
            reason: "kw".to_string(),
        };
        assert!(!high.is_low());
        assert!(high.needs_protection());
        assert!(high.description().contains("high sensitivity"));
    }

    #[test]
    fn test_detect_all_filters_low() {
        let detector = SensitiveDataDetector::new();
        let mut data = HashMap::new();
        data.insert("password".to_string(), "secret".to_string());
        data.insert("username".to_string(), "alice".to_string());
        data.insert("api_key".to_string(), "val".to_string());

        let results = detector.detect_all(&data);
        // 只返回非 Low 的条目
        assert_eq!(results.len(), 2);
        for (_, r) in &results {
            assert!(r.needs_protection());
        }
    }

    #[test]
    fn test_custom_sensitive_field_case_insensitive() {
        let mut detector = SensitiveDataDetector::new();
        detector.add_custom_sensitive_field("MyCustomField");
        // 添加时小写化，检测时字段名大小写均应命中
        assert!(detector
            .is_sensitive("mycustomfield", "v")
            .needs_protection());
        assert!(detector
            .is_sensitive("MYCUSTOMFIELD", "v")
            .needs_protection());
        // 自定义字段标记为 Medium
        let r = detector.is_sensitive("mycustomfield", "v");
        assert!(matches!(r, SensitivityResult::Medium { .. }));
    }

    #[test]
    fn test_xss_input_rejected() {
        let validator = InputValidator::new();
        // < > ( ) 属于危险字符集合
        assert!(validator
            .validate_string("<script>alert(1)</script>")
            .is_err());
        assert!(validator.validate_string("<img onerror=alert(1)>").is_err());
        assert!(validator.validate_string("<svg onload=alert(1)>").is_err());
        assert!(validator
            .validate_string("javascript:alert(document.cookie)")
            .is_err());
        // 清理后应剥离 <>
        let sanitized = validator
            .sanitize_string("<script>alert(1)</script>")
            .unwrap();
        assert!(!sanitized.contains('<'));
        assert!(!sanitized.contains('>'));
        assert!(!sanitized.contains('('));
    }

    #[test]
    fn test_sql_injection_rejected() {
        let validator = InputValidator::new();
        // 分号 + DROP
        assert!(validator.validate_string("1; DROP TABLE users").is_err());
        // UNION SELECT
        assert!(validator
            .validate_string("1 UNION SELECT password FROM users")
            .is_err());
        // 引号 + OR
        assert!(validator.validate_string("' OR '1'='1").is_err());
        // SQL 注释结尾
        assert!(validator.validate_string("admin'--").is_err());
        // DELETE
        assert!(validator.validate_string("; DELETE FROM accounts").is_err());
    }

    #[test]
    fn test_path_traversal_rejected() {
        let validator = InputValidator::new();
        assert!(validator.validate_string("../../etc/passwd").is_err());
        assert!(validator
            .validate_string("..\\..\\windows\\system32")
            .is_err());
        assert!(validator.validate_string("/etc/../passwd").is_err());
        assert!(validator
            .validate_string("var/log/../../etc/shadow")
            .is_err());
    }

    #[test]
    fn test_command_injection_rejected() {
        let validator = InputValidator::new();
        // 分号
        assert!(validator.validate_string("; rm -rf /").is_err());
        // 管道
        assert!(validator.validate_string("| cat /etc/passwd").is_err());
        // && 和 ||
        assert!(validator.validate_string("true && whoami").is_err());
        assert!(validator.validate_string("false || malicious").is_err());
        // 命令替换 $(...)
        assert!(validator.validate_string("$(whoami)").is_err());
        // 反引号
        assert!(validator.validate_string("`whoami`").is_err());
        // shell 变量展开
        assert!(validator.validate_string("${HOME}").is_err());
        // 重定向
        assert!(validator.validate_string(">> /tmp/file").is_err());
        assert!(validator.validate_string("2> /dev/null").is_err());
    }

    #[test]
    fn test_validate_string_too_long() {
        let validator = InputValidator::new().with_max_string_length(5);
        assert!(validator.validate_string("abcdef").is_err());
        let err = validator.validate_string("abcdef").unwrap_err();
        assert!(matches!(
            err,
            InputValidationError::TooLong { max: 5, actual: 6 }
        ));
        // 边界：恰好等于上限应通过
        assert!(validator.validate_string("abcde").is_ok());
    }

    #[test]
    fn test_validate_string_invalid_characters() {
        // 允许字符模式不匹配 -> InvalidCharacters
        let validator = InputValidator::new().with_allowed_chars_pattern(r"^[a-z]+$");
        assert!(validator.validate_string("hello").is_ok());
        let err = validator.validate_string("Hello").unwrap_err();
        assert_eq!(err, InputValidationError::InvalidCharacters);
        let err2 = validator.validate_string("hello123").unwrap_err();
        assert_eq!(err2, InputValidationError::InvalidCharacters);
    }

    #[test]
    fn test_dangerous_pattern_error_carries_pattern() {
        let validator = InputValidator::new();
        let err = validator.validate_string("a;b").unwrap_err();
        match err {
            InputValidationError::DangerousPattern { pattern } => {
                assert!(!pattern.is_empty());
            }
            _ => panic!("expected DangerousPattern"),
        }
    }

    #[test]
    fn test_sanitize_string_strips_all_dangerous_chars() {
        let validator = InputValidator::new();
        // 全部为危险字符 -> 清理后为空字符串
        let sanitized = validator.sanitize_string(";<>|`$(){}").unwrap();
        assert_eq!(sanitized, "");
        // 混合：危险字符被剥离，其余保留
        let sanitized = validator.sanitize_string("he;ll{o}").unwrap();
        assert_eq!(sanitized, "hello");
        // 含 null 字节
        let sanitized = validator.sanitize_string("a\0b").unwrap();
        assert_eq!(sanitized, "ab");
    }

    #[test]
    fn test_validate_field_name_variants() {
        let validator = InputValidator::new();
        // 空
        assert_eq!(
            validator.validate_field_name("").unwrap_err(),
            InputValidationError::EmptyFieldName
        );
        // 以数字开头
        assert_eq!(
            validator.validate_field_name("1abc").unwrap_err(),
            InputValidationError::InvalidFieldNameFormat
        );
        // 含空格
        assert_eq!(
            validator.validate_field_name("a b").unwrap_err(),
            InputValidationError::InvalidFieldNameFormat
        );
        // 有效：单字母、含 _ - 、全大写
        assert!(validator.validate_field_name("a").is_ok());
        assert!(validator.validate_field_name("app_name").is_ok());
        assert!(validator.validate_field_name("app-name").is_ok());
        assert!(validator.validate_field_name("APP").is_ok());
        // 过长
        let v = InputValidator::new().with_max_string_length(3);
        assert!(matches!(
            v.validate_field_name("abcd").unwrap_err(),
            InputValidationError::TooLong { max: 3, actual: 4 }
        ));
    }

    #[test]
    fn test_validate_url_variants() {
        let validator = InputValidator::new();
        // 无效 URL（无法解析）
        assert_eq!(
            validator.validate_url("not a url").unwrap_err(),
            InputValidationError::InvalidUrl
        );
        // 非法 scheme
        assert_eq!(
            validator.validate_url("ftp://example.com").unwrap_err(),
            InputValidationError::InvalidUrlScheme
        );
        assert_eq!(
            validator.validate_url("file:///etc/passwd").unwrap_err(),
            InputValidationError::InvalidUrlScheme
        );
        // 含危险模式
        assert!(matches!(
            validator.validate_url("https://example.com;a").unwrap_err(),
            InputValidationError::DangerousPattern { .. }
        ));
        assert!(matches!(
            validator
                .validate_url("https://example.com/${x}")
                .unwrap_err(),
            InputValidationError::DangerousPattern { .. }
        ));
        // 过长
        let v = InputValidator::new().with_max_string_length(5);
        assert!(matches!(
            v.validate_url("https://a").unwrap_err(),
            InputValidationError::TooLong { .. }
        ));
        // 有效
        assert!(validator.validate_url("https://example.com/path").is_ok());
    }

    #[test]
    fn test_validate_email_variants() {
        let validator = InputValidator::new();
        // 无效
        assert_eq!(
            validator.validate_email("invalid-email").unwrap_err(),
            InputValidationError::InvalidEmail
        );
        assert_eq!(
            validator.validate_email("@example.com").unwrap_err(),
            InputValidationError::InvalidEmail
        );
        assert_eq!(
            validator.validate_email("user@").unwrap_err(),
            InputValidationError::InvalidEmail
        );
        assert_eq!(
            validator.validate_email("user@example").unwrap_err(),
            InputValidationError::InvalidEmail
        );
        // 过长
        let v = InputValidator::new().with_max_string_length(5);
        assert!(matches!(
            v.validate_email("a@b.co").unwrap_err(),
            InputValidationError::TooLong { .. }
        ));
        // 有效
        assert!(validator
            .validate_email("user.name+tag@example.com")
            .is_ok());
    }

    #[test]
    fn test_validate_whitelist() {
        // 无白名单 -> 一律放行
        let validator = InputValidator::new();
        assert!(validator.validate_whitelist("anything").is_ok());
        // 添加白名单
        let validator = InputValidator::new()
            .add_whitelist_pattern(r"^abc")
            .add_whitelist_pattern(r"^xyz");
        assert!(validator.validate_whitelist("abc123").is_ok());
        assert!(validator.validate_whitelist("xyz789").is_ok());
        // 不在白名单
        assert_eq!(
            validator.validate_whitelist("def456").unwrap_err(),
            InputValidationError::NotInWhitelist
        );
    }

    #[test]
    fn test_validate_all_collects_errors() {
        let validator = InputValidator::new();
        let mut data = HashMap::new();
        data.insert("valid_name".to_string(), "valid_value".to_string());
        // 字段名非法（以数字开头）
        data.insert("1bad".to_string(), "ok".to_string());
        // 值含危险模式
        data.insert("good_name".to_string(), "a;b".to_string());

        let errors = validator.validate_all(&data);
        assert_eq!(errors.len(), 2);
        // 应包含两个字段错误
        let names: Vec<&str> = errors.iter().map(|(n, _)| n.as_str()).collect();
        assert!(names.contains(&"1bad"));
        assert!(names.contains(&"good_name"));
    }

    #[test]
    fn test_input_validation_error_display_all_variants() {
        // 覆盖所有 Display 分支
        let s = format!("{}", InputValidationError::TooLong { max: 1, actual: 2 });
        assert!(s.contains("max=1"));
        assert!(s.contains("actual=2"));

        assert!(format!("{}", InputValidationError::InvalidCharacters).contains("invalid"));
        assert!(format!(
            "{}",
            InputValidationError::DangerousPattern {
                pattern: "x".into()
            }
        )
        .contains("x"));
        assert!(format!("{}", InputValidationError::EmptyFieldName).contains("empty"));
        assert!(format!("{}", InputValidationError::InvalidFieldNameFormat).contains("invalid"));
        assert!(format!("{}", InputValidationError::InvalidUrl).contains("URL"));
        assert!(format!("{}", InputValidationError::InvalidUrlScheme).contains("scheme"));
        assert!(format!("{}", InputValidationError::InvalidEmail).contains("Email"));
        assert!(format!("{}", InputValidationError::NotInWhitelist).contains("whitelist"));
        assert!(format!(
            "{}",
            InputValidationError::DepthExceeded { max: 1, actual: 2 }
        )
        .contains("depth"));
        assert!(format!(
            "{}",
            InputValidationError::ArrayTooLong { max: 1, actual: 2 }
        )
        .contains("Array"));
    }

    #[test]
    fn test_config_validator_builder_and_strict() {
        // builder 链式配置
        let validator = ConfigValidator::builder()
            .max_string_length(10)
            .add_sensitive_field("custom_token")
            .build();
        let mut data = HashMap::new();
        data.insert("custom_token".to_string(), "short".to_string());
        let result = validator.validate(&data);
        assert!(result.has_sensitive_data());

        // strict_mode：max_string_length=256，token/password 为敏感
        let strict = ConfigValidatorBuilder::new().strict_mode().build();
        let mut data = HashMap::new();
        data.insert("token".to_string(), "abc".to_string());
        data.insert("password".to_string(), "xyz".to_string());
        let result = strict.validate(&data);
        assert_eq!(result.sensitive_fields.len(), 2);
    }

    #[test]
    fn test_config_validator_default_and_safe() {
        let validator = ConfigValidator::default();
        let mut data = HashMap::new();
        data.insert("app_name".to_string(), "myapp".to_string());
        // validate_safe 对有效配置返回 true
        assert!(validator.validate_safe(&data));
        // 含危险值 -> false
        let mut bad = HashMap::new();
        bad.insert("app_name".to_string(), "a;b".to_string());
        assert!(!validator.validate_safe(&bad));
        // 字段名非法 -> false
        let mut bad_name = HashMap::new();
        bad_name.insert("1bad".to_string(), "ok".to_string());
        assert!(!validator.validate_safe(&bad_name));
    }

    #[test]
    fn test_config_validation_result_report_and_sensitive() {
        let validator = ConfigValidator::new();
        // 无错误：report 为空
        let mut ok = HashMap::new();
        ok.insert("app_name".to_string(), "myapp".to_string());
        let result = validator.validate(&ok);
        assert!(result.is_valid());
        assert!(!result.has_sensitive_data());
        assert_eq!(result.error_report(), "");

        // 有错误：report 非空
        let mut bad = HashMap::new();
        bad.insert("1bad".to_string(), "a;b".to_string());
        let result = validator.validate(&bad);
        assert!(!result.is_valid());
        assert!(!result.error_report().is_empty());
        // 报告中应出现字段名
        assert!(result.error_report().contains("1bad"));
    }

    #[test]
    fn test_config_validation_error_display() {
        let field_err = ConfigValidationError::FieldError {
            field: "f".to_string(),
            error: InputValidationError::EmptyFieldName,
        };
        assert!(format!("{}", field_err).contains("f"));
        let warn = ConfigValidationError::SensitiveDataWarning {
            field: "pw".to_string(),
            sensitivity: SensitivityResult::High {
                field: "pw".to_string(),
                reason: "r".to_string(),
            },
        };
        let s = format!("{}", warn);
        assert!(s.contains("pw"));
        assert!(s.contains("high sensitivity"));
    }
}
