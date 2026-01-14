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

use regex::Regex;
use std::collections::HashSet;
use std::sync::OnceLock;

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
        vec![
            Regex::new(r"(?i)password").unwrap(),
            Regex::new(r"(?i)secret").unwrap(),
            Regex::new(r"(?i)token").unwrap(),
            Regex::new(r"(?i)api_key").unwrap(),
            Regex::new(r"(?i)access_key").unwrap(),
            Regex::new(r"(?i)private_key").unwrap(),
            Regex::new(r"(?i)credential").unwrap(),
            Regex::new(r"(?i)auth").unwrap(),
            Regex::new(r"(?i)key").unwrap(),
            Regex::new(r"(?i)cert").unwrap(),
            Regex::new(r"(?i)password_hash").unwrap(),
            Regex::new(r"(?i)session_id").unwrap(),
        ]
    }

    /// 默认高敏感度关键词
    fn default_high_sensitivity_keywords() -> HashSet<&'static str> {
        let mut set = HashSet::new();
        set.insert("password");
        set.insert("secret");
        set.insert("private_key");
        set.insert("master_key");
        set.insert("encryption_key");
        set.insert("api_secret");
        set.insert("access_token");
        set.insert("refresh_token");
        set.insert("client_secret");
        set.insert("db_password");
        set.insert("admin_password");
        set
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
        vec![
            Regex::new(r"[;<>&|`$()]").unwrap(), // Shell 元字符
            Regex::new(r"\$\{.*\}").unwrap(),    // Shell 变量展开
            Regex::new(r"`[^`]+`").unwrap(),     // 命令替换
            Regex::new(r"\|").unwrap(),          // 管道
            Regex::new(r"&&").unwrap(),          // 条件执行
            Regex::new(r"\|\|").unwrap(),        // 条件执行
            Regex::new(r">>").unwrap(),          // 追加重定向
            Regex::new(r"2>").unwrap(),          // 错误重定向
        ]
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
        // 先验证
        self.validate_string(value)?;

        // 清理危险字符
        let mut result = String::new();
        for c in value.chars() {
            if !self.is_dangerous_char(c) {
                result.push(c);
            }
        }

        Ok(result)
    }

    /// 检查字符是否为危险字符
    fn is_dangerous_char(&self, c: char) -> bool {
        matches!(
            c,
            ';' | '<' | '>' | '&' | '|' | '`' | '$' | '(' | ')' | '\0'
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
    pub fn validate_all(
        &self,
        data: &HashMap<String, String>,
    ) -> Vec<(&String, InputValidationError)> {
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
            is_valid: errors.is_empty(),
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

/// 配置验证结果
#[derive(Debug, Clone)]
pub struct ConfigValidationResult {
    /// 是否有效
    pub is_valid: bool,
    /// 验证错误列表
    pub errors: Vec<ConfigValidationError>,
    /// 敏感字段列表
    pub sensitive_fields: Vec<(String, SensitivityResult)>,
}

impl ConfigValidationResult {
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

        assert!(result.is_valid);
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
}
