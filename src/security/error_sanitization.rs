// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! # 错误消息脱敏
//!
//! 提供错误消息的安全处理功能，确保敏感信息不会泄露到日志或用户界面。
//!
//! ## 功能特性
//!
//! - **敏感信息脱敏**: 自动检测和脱敏错误消息中的敏感数据
//! - **安全日志输出**: 提供安全的日志记录接口
//! - **错误消息过滤**: 过滤包含敏感信息的错误消息
//! - **自定义脱敏规则**: 支持自定义脱敏模式和规则

use crate::security::patterns::SENSITIVE_KEYWORDS;
use regex::Regex;
use std::collections::HashSet;
use std::sync::{Arc, OnceLock, RwLock};

/// 敏感信息模式
static SENSITIVE_PATTERNS: OnceLock<Vec<(Regex, Replacement)>> = OnceLock::new();

/// 初始化敏感信息模式
fn init_sensitive_patterns() -> Vec<(Regex, Replacement)> {
    vec![
        // API 密钥和令牌 - 使用更严格的模式避免误匹配
        (
            Regex::new(r"(?i)api[ ]?key[\s]*[:=][\s]*([a-zA-Z0-9_\-]+)").unwrap(),
            Replacement::MaskedGroup(1, 8),
        ),
        // 密码
        (
            Regex::new(r"(?i)password[\s]*[:=][\s]*([^;\s]+)").unwrap(),
            Replacement::MaskedGroup(1, 4),
        ),
        // 令牌 - access token, refresh token, auth token
        (
            Regex::new(r"(?i)access[ ]?token[\s]*[:=][\s]*([a-zA-Z0-9_\-\.]+)").unwrap(),
            Replacement::MaskedGroup(1, 6),
        ),
        (
            Regex::new(r"(?i)refresh[ ]?token[\s]*[:=][\s]*([a-zA-Z0-9_\-\.]+)").unwrap(),
            Replacement::MaskedGroup(1, 6),
        ),
        (
            Regex::new(r"(?i)auth[ ]?token[\s]*[:=][\s]*([a-zA-Z0-9_\-\.]+)").unwrap(),
            Replacement::MaskedGroup(1, 6),
        ),
        // 密钥
        (
            Regex::new(r"(?i)secret[ ]?key[\s]*[:=][\s]*([a-zA-Z0-9_\-\/+=]*)").unwrap(),
            Replacement::MaskedGroup(1, 8),
        ),
        (
            Regex::new(r"(?i)private[ ]?key[\s]*[:=][\s]*([a-zA-Z0-9_\-\/+=]*)").unwrap(),
            Replacement::MaskedGroup(1, 8),
        ),
        // 连接字符串
        (
            Regex::new(r"(?i)(database[ ]?url|connection[ ]?string|mongodb(\+ssl)?://)[^\s]+")
                .unwrap(),
            Replacement::Value("***CONNECTION_STRING***".to_string()),
        ),
        // Bearer 令牌
        (
            Regex::new(r"(?i)Bearer\s+([a-zA-Z0-9_\-\.]+)").unwrap(),
            Replacement::MaskedGroup(1, 6),
        ),
        // 基本认证
        (
            Regex::new(r"(?i)Basic\s+([a-zA-Z0-9+/=]+)").unwrap(),
            Replacement::MaskedGroup(1, 6),
        ),
        // 邮箱地址
        (
            Regex::new(r"([a-zA-Z0-9._%+-]+)@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}").unwrap(),
            Replacement::EmailMask,
        ),
        // IP 地址（可选）
        (
            Regex::new(r"\b(?:\d{1,3}\.){3}\d{1,3}\b").unwrap(),
            Replacement::Value("***IP_ADDRESS***".to_string()),
        ),
    ]
}

/// 获取敏感信息模式（懒初始化）
fn get_sensitive_patterns() -> &'static Vec<(Regex, Replacement)> {
    SENSITIVE_PATTERNS.get_or_init(init_sensitive_patterns)
}

/// 替换策略
#[derive(Debug, Clone)]
enum Replacement {
    /// 掩码特定分组
    MaskedGroup(usize, usize),
    /// 完全替换
    Value(String),
    /// 邮箱掩码
    EmailMask,
}

/// 错误脱敏器
#[derive(Debug, Clone)]
pub struct ErrorSanitizer {
    /// 自定义规则
    custom_rules: Arc<RwLock<Vec<(Regex, String)>>>,
    /// 敏感关键词
    sensitive_keywords: Arc<RwLock<HashSet<String>>>,
    /// 是否启用严格模式
    strict_mode: bool,
}

impl Default for ErrorSanitizer {
    fn default() -> Self {
        Self::new()
    }
}

impl ErrorSanitizer {
    /// 创建新的脱敏器
    pub fn new() -> Self {
        Self {
            custom_rules: Arc::new(RwLock::new(Vec::new())),
            sensitive_keywords: Arc::new(RwLock::new(Self::default_keywords())),
            strict_mode: false,
        }
    }

    /// 默认敏感关键词
    fn default_keywords() -> HashSet<String> {
        SENSITIVE_KEYWORDS.iter().map(|s| s.to_string()).collect()
    }

    /// 启用严格模式
    pub fn with_strict_mode(mut self) -> Self {
        self.strict_mode = true;
        self
    }

    /// 添加自定义脱敏规则
    pub fn add_rule(&self, pattern: &str, replacement: &str) -> Result<(), Error> {
        let regex = Regex::new(pattern).map_err(|_| Error::InvalidPattern)?;
        let mut rules = self.custom_rules.write().map_err(|_| Error::PoisonedLock)?;
        rules.push((regex, replacement.to_string()));
        Ok(())
    }

    /// 添加敏感关键词
    pub fn add_sensitive_keyword(&self, keyword: &str) {
        let mut keywords = self.sensitive_keywords.write().unwrap();
        keywords.insert(keyword.to_lowercase());
    }

    /// 脱敏错误消息
    pub fn sanitize(&self, message: &str) -> String {
        let mut result = message.to_string();

        // 应用内置规则
        for (ref pattern, ref replacement) in get_sensitive_patterns().iter() {
            result = apply_replacement(&result, pattern, replacement);
        }

        // 应用自定义规则
        let custom_rules = self.custom_rules.read().unwrap();
        for (ref pattern, ref replacement) in custom_rules.iter() {
            result = pattern
                .replace_all(&result, replacement.as_str())
                .to_string();
        }

        // 如果在严格模式，替换所有敏感关键词
        if self.strict_mode {
            let keywords = self.sensitive_keywords.read().unwrap();
            for keyword in keywords.iter() {
                let pattern = Regex::new(&format!(r"(?i)\b{}\b", regex::escape(keyword))).unwrap();
                result = pattern.replace_all(&result, "***").to_string();
            }
        }

        result
    }

    /// 脱敏错误消息并标记
    pub fn sanitize_with_indicator(&self, message: &str) -> (String, bool) {
        let sanitized = self.sanitize(message);
        let contains_sensitive = sanitized.contains("***");
        (sanitized, contains_sensitive)
    }

    /// 检查消息是否包含敏感信息
    pub fn contains_sensitive(&self, message: &str) -> bool {
        // 检查是否匹配任何敏感模式
        for (ref pattern, _) in get_sensitive_patterns().iter() {
            if pattern.is_match(message) {
                return true;
            }
        }

        // 检查敏感关键词
        let keywords = self.sensitive_keywords.read().unwrap();
        let message_lower = message.to_lowercase();
        keywords
            .iter()
            .any(|keyword| message_lower.contains(keyword))
    }

    /// 批量脱敏
    pub fn sanitize_all(&self, messages: &[&str]) -> Vec<String> {
        messages.iter().map(|m| self.sanitize(m)).collect()
    }

    /// 创建安全版本的消息
    pub fn safe_message(&self, message: &str, context: &str) -> String {
        if self.contains_sensitive(message) {
            format!(
                "[{}] Sensitive data detected in error message - sanitized for security",
                context
            )
        } else {
            self.sanitize(message)
        }
    }
}

/// 应用替换
fn apply_replacement(input: &str, pattern: &Regex, replacement: &Replacement) -> String {
    pattern
        .replace_all(input, |caps: &regex::Captures| match replacement {
            Replacement::MaskedGroup(group_idx, visible_chars) => {
                if let Some(matched) = caps.get(*group_idx) {
                    let s = matched.as_str();
                    // 按字符计数避免多字节 UTF-8 切片 panic
                    let char_count = s.chars().count();
                    if char_count <= *visible_chars {
                        "*".repeat(char_count)
                    } else {
                        let prefix: String = s.chars().take(*visible_chars).collect();
                        format!("{}***", prefix)
                    }
                } else {
                    "***".to_string()
                }
            }
            Replacement::Value(repl) => repl.clone(),
            Replacement::EmailMask => {
                if let Some(email) = caps.get(0) {
                    let s = email.as_str();
                    if let Some(at_pos) = s.find('@') {
                        let local_part = &s[..at_pos];
                        let domain = &s[at_pos..];
                        // 按字符计数避免多字节 UTF-8 切片 panic
                        let local_chars = local_part.chars().count();
                        if local_chars <= 2 {
                            "***".to_string() + domain
                        } else {
                            let prefix: String = local_part.chars().take(2).collect();
                            format!("{}**{}", prefix, domain)
                        }
                    } else {
                        "***".to_string()
                    }
                } else {
                    "***".to_string()
                }
            }
        })
        .to_string()
}

/// 安全日志记录器
#[derive(Debug, Clone)]
pub struct SecureLogger {
    /// 脱敏器
    sanitizer: ErrorSanitizer,
    /// 日志级别
    min_level: LogLevel,
}

impl Default for SecureLogger {
    fn default() -> Self {
        Self::new()
    }
}

impl SecureLogger {
    /// 创建新的安全日志记录器
    pub fn new() -> Self {
        Self {
            sanitizer: ErrorSanitizer::new(),
            min_level: LogLevel::Debug,
        }
    }

    /// 设置最小日志级别
    pub fn with_min_level(mut self, level: LogLevel) -> Self {
        self.min_level = level;
        self
    }

    /// 记录调试日志
    pub fn debug(&self, message: &str) {
        self.log(LogLevel::Debug, message);
    }

    /// 记录信息日志
    pub fn info(&self, message: &str) {
        self.log(LogLevel::Info, message);
    }

    /// 记录警告日志
    pub fn warn(&self, message: &str) {
        self.log(LogLevel::Warn, message);
    }

    /// 记录错误日志
    pub fn error(&self, message: &str) {
        self.log(LogLevel::Error, message);
    }

    /// 记录错误（带上下文）
    pub fn error_with_context(&self, context: &str, error: &str) {
        let safe_message = self.sanitizer.safe_message(error, context);
        self.log(LogLevel::Error, &safe_message);
    }

    /// 内部日志记录
    fn log(&self, level: LogLevel, message: &str) {
        if level < self.min_level {
            return;
        }

        let sanitized = self.sanitizer.sanitize(message);
        let log_entry = format!("[{}] {}", level.as_str(), sanitized);
        match level {
            LogLevel::Error | LogLevel::Warn => eprintln!("{log_entry}"),
            LogLevel::Info | LogLevel::Debug => eprintln!("{log_entry}"),
        }
    }

    /// 获取脱敏器引用
    pub fn sanitizer(&self) -> &ErrorSanitizer {
        &self.sanitizer
    }
}

/// 日志级别
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

impl LogLevel {
    /// 转换为字符串
    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
        }
    }
}

/// 错误类型
#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    /// 无效模式
    InvalidPattern,
    /// 锁中毒
    PoisonedLock,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::InvalidPattern => write!(f, "Invalid regex pattern"),
            Error::PoisonedLock => write!(f, "Lock poisoned"),
        }
    }
}

impl std::error::Error for Error {}

/// 安全错误结果
#[derive(Debug, Clone)]
pub struct SafeResult<T> {
    /// 原始结果
    value: Option<T>,
    /// 错误消息（已脱敏）
    error_message: Option<String>,
    /// 是否包含敏感数据
    contained_sensitive: bool,
}

impl<T> SafeResult<T> {
    /// 创建成功结果
    pub fn ok(value: T) -> Self {
        Self {
            value: Some(value),
            error_message: None,
            contained_sensitive: false,
        }
    }

    /// 创建错误结果
    pub fn err(message: &str) -> Self {
        let sanitizer = ErrorSanitizer::new();
        let (sanitized, contained_sensitive) = sanitizer.sanitize_with_indicator(message);
        Self {
            value: None,
            error_message: Some(sanitized),
            contained_sensitive,
        }
    }

    /// 检查是否为成功
    pub fn is_ok(&self) -> bool {
        self.value.is_some()
    }

    /// 检查是否为错误
    pub fn is_err(&self) -> bool {
        self.value.is_none()
    }

    /// 获取值（安全版本）
    pub fn value(&self) -> Option<&T> {
        self.value.as_ref()
    }

    /// 获取错误消息
    pub fn error_message(&self) -> Option<&str> {
        self.error_message.as_deref()
    }

    /// 检查是否包含敏感数据
    pub fn contained_sensitive(&self) -> bool {
        self.contained_sensitive
    }

    /// 获取值或panic
    pub fn unwrap(self) -> T {
        self.value.expect("SafeResult::unwrap() on None")
    }

    /// 获取值或默认值
    pub fn unwrap_or(self, default: T) -> T {
        self.value.unwrap_or(default)
    }
}

/// 敏感数据过滤器
#[derive(Debug, Clone)]
pub struct SensitiveDataFilter {
    /// 脱敏器
    sanitizer: ErrorSanitizer,
    /// 允许的消息模式
    allowed_patterns: Vec<Regex>,
    /// 阻止的消息模式
    blocked_patterns: Vec<Regex>,
}

impl Default for SensitiveDataFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl SensitiveDataFilter {
    /// 创建新的过滤器
    pub fn new() -> Self {
        Self {
            sanitizer: ErrorSanitizer::new(),
            allowed_patterns: Vec::new(),
            blocked_patterns: Vec::new(),
        }
    }

    /// 添加允许的模式
    pub fn add_allowed_pattern(&mut self, pattern: &str) -> Result<(), Error> {
        let regex = Regex::new(pattern).map_err(|_| Error::InvalidPattern)?;
        self.allowed_patterns.push(regex);
        Ok(())
    }

    /// 添加阻止的模式
    pub fn add_blocked_pattern(&mut self, pattern: &str) -> Result<(), Error> {
        let regex = Regex::new(pattern).map_err(|_| Error::InvalidPattern)?;
        self.blocked_patterns.push(regex);
        Ok(())
    }

    /// 过滤消息
    pub fn filter(&self, message: &str) -> FilterResult {
        // 先检查是否包含敏感数据
        if !self.sanitizer.contains_sensitive(message) {
            return FilterResult::Allowed(message.to_string());
        }

        // 检查是否被阻止
        for pattern in &self.blocked_patterns {
            if pattern.is_match(message) {
                return FilterResult::Blocked {
                    reason: "message matches blocked pattern".to_string(),
                };
            }
        }

        // 脱敏并允许
        let sanitized = self.sanitizer.sanitize(message);
        FilterResult::Sanitized(sanitized)
    }

    /// 批量过滤
    pub fn filter_all<'a>(&self, messages: &'a [&str]) -> Vec<(&'a str, FilterResult)> {
        messages.iter().map(|m| (*m, self.filter(m))).collect()
    }
}

/// 过滤结果
#[derive(Debug, Clone)]
pub enum FilterResult {
    /// 允许（无敏感数据）
    Allowed(String),
    /// 已脱敏
    Sanitized(String),
    /// 被阻止
    Blocked { reason: String },
}

impl FilterResult {
    /// 检查是否被允许
    pub fn is_allowed(&self) -> bool {
        matches!(self, FilterResult::Allowed(_))
    }

    /// 检查是否被阻止
    pub fn is_blocked(&self) -> bool {
        matches!(self, FilterResult::Blocked { .. })
    }

    /// 获取处理后的消息
    pub fn message(&self) -> Option<&str> {
        match self {
            FilterResult::Allowed(msg) => Some(msg),
            FilterResult::Sanitized(msg) => Some(msg),
            FilterResult::Blocked { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strict_mode() {
        let sanitizer = ErrorSanitizer::new().with_strict_mode();
        let result = sanitizer.sanitize("The password is secret and token is key123");
        assert!(!result.contains("password"));
        assert!(!result.contains("secret"));
        assert!(!result.contains("token"));
    }

    #[test]
    fn test_safe_message() {
        let sanitizer = ErrorSanitizer::new();

        let result = sanitizer.safe_message("Database connection failed", "DB");
        assert_eq!(result, "Database connection failed");

        let result = sanitizer.safe_message("API Key: sk-12345", "API");
        assert!(result.contains("sanitized"));
    }

    #[test]
    fn test_secure_logger() {
        let logger = SecureLogger::new().with_min_level(LogLevel::Debug);

        // 应该能够记录而不panic
        logger.debug("Debug message");
        logger.info("Info message");
        logger.warn("Warning message");
        logger.error("Error with API Key: sk-12345");
    }

    #[test]
    fn test_safe_result() {
        let success = SafeResult::ok("value");
        assert!(success.is_ok());
        assert!(!success.is_err());
        assert_eq!(success.value(), Some(&"value"));

        let failure: SafeResult<()> = SafeResult::err("Error with password: secret");
        assert!(!failure.is_ok());
        assert!(failure.is_err());
        assert!(failure.error_message().is_some());
        assert!(failure.contained_sensitive());
    }

    #[test]
    fn test_sensitive_data_filter() {
        let mut filter = SensitiveDataFilter::new();
        filter.add_blocked_pattern(r".*password.*").unwrap();

        // 被阻止的消息
        let result = filter.filter("Contains password secret");
        assert!(result.is_blocked());

        // 允许的消息
        let result = filter.filter("Normal message");
        assert!(result.is_allowed());

        // 已脱敏的消息
        let result = filter.filter("API Key: sk-12345");
        match result {
            FilterResult::Sanitized(msg) => {
                assert!(!msg.contains("sk-12345"));
            }
            _ => unreachable!("Expected sanitized result"),
        }
    }

    #[test]
    fn test_error_sanitization() {
        let sanitizer = ErrorSanitizer::new();

        // Test with a pattern that works - using : as separator
        let result = sanitizer.sanitize("API Key: secret123");
        println!("API Key result: '{}'", result);
        assert!(
            !result.contains("secret123"),
            "API key not masked: {}",
            result
        );
        assert!(result.contains("***"), "No masking: {}", result);

        // 脱敏密码
        let result = sanitizer.sanitize("Password: mySecretPassword123");
        assert!(!result.contains("mySecretPassword123"));
        assert!(result.contains("***"));

        // 脱敏令牌
        let result = sanitizer.sanitize("Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9");
        assert!(!result.contains("eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9"));

        // 脱敏邮箱
        let result = sanitizer.sanitize("Contact: user@example.com");
        assert!(!result.contains("user@example.com"));
        assert!(result.contains("**"), "Email not masked: {}", result);
    }

    #[test]
    fn test_api_key_masking() {
        let sanitizer = ErrorSanitizer::new();
        // 长 API key：保留前 8 位，其余掩码
        let result = sanitizer.sanitize("API Key: sk-1234567890abcdef"); // pragma: allowlist secret
        assert!(!result.contains("sk-1234567890abcdef"));
        assert!(result.contains("***"));
        // 短 API key（<=8 字符）：全部掩码
        let result = sanitizer.sanitize("API Key: abcd"); // pragma: allowlist secret
        assert!(!result.contains("abcd"));
        assert!(result.contains("****"));
        // 使用 = 分隔（模式用 [ ]? 可选空格，非下划线）
        let result = sanitizer.sanitize("api key=secret12345"); // pragma: allowlist secret
        assert!(!result.contains("secret12345"));
    }

    #[test]
    fn test_password_and_token_masking() {
        let sanitizer = ErrorSanitizer::new();
        // password: 保留前 4 位
        let result = sanitizer.sanitize("password: mySecretPass"); // pragma: allowlist secret
        assert!(!result.contains("mySecretPass"));
        assert!(result.contains("***"));
        // access token: 保留前 6 位（模式用 [ ]? 可选空格）
        let result = sanitizer.sanitize("access token: abcdefghijk"); // pragma: allowlist secret
        assert!(!result.contains("abcdefghijk"));
        assert!(result.contains("***"));
        // refresh token
        let result = sanitizer.sanitize("refresh token: tok1234567890"); // pragma: allowlist secret
        assert!(!result.contains("tok1234567890"));
        // auth token
        let result = sanitizer.sanitize("auth token: authval123456"); // pragma: allowlist secret
        assert!(!result.contains("authval123456"));
    }

    #[test]
    fn test_secret_and_private_key_masking() {
        let sanitizer = ErrorSanitizer::new();
        let result = sanitizer.sanitize("secret key: ABCDEFGHIJ123456"); // pragma: allowlist secret
        assert!(!result.contains("ABCDEFGHIJ123456"));
        assert!(result.contains("***"));
        let result = sanitizer.sanitize("private key: PRIVKEYVALUE123"); // pragma: allowlist secret
        assert!(!result.contains("PRIVKEYVALUE123"));
    }

    #[test]
    fn test_connection_string_masking() {
        let sanitizer = ErrorSanitizer::new();
        // mongodb 连接字符串（含凭证）
        let result = sanitizer.sanitize("mongodb://user:pass@host:27017/db"); // pragma: allowlist secret
        assert!(!result.contains("user:pass@host:27017"));
        assert!(result.contains("CONNECTION_STRING"));
        // mongodb+ssl
        let result = sanitizer.sanitize("mongodb+ssl://user:pass@host/db"); // pragma: allowlist secret
        assert!(result.contains("CONNECTION_STRING"));
    }

    #[test]
    fn test_bearer_and_basic_auth_masking() {
        let sanitizer = ErrorSanitizer::new();
        // Bearer 令牌
        let result = sanitizer.sanitize("Bearer eyJhbGciOiJIUzI1NiJ9.signature"); // pragma: allowlist secret
        assert!(!result.contains("eyJhbGciOiJIUzI1NiJ9"));
        assert!(result.contains("***"));
        // Basic 认证
        let result = sanitizer.sanitize("Basic dXNlcjpwYXNz"); // pragma: allowlist secret
        assert!(!result.contains("dXNlcjpwYXNz"));
        assert!(result.contains("***"));
    }

    #[test]
    fn test_email_masking_variants() {
        let sanitizer = ErrorSanitizer::new();
        // 长 local part：保留前 2 位 + 域名
        let result = sanitizer.sanitize("Contact: aliceuser@example.com");
        assert!(!result.contains("aliceuser@example.com"));
        assert!(result.contains("@example.com"));
        // 短 local part（<=2）：local part 全部替换为 ***
        let result = sanitizer.sanitize("ab@x.co");
        assert!(result.contains("***@x.co"));
        assert!(!result.contains("ab@"));
    }

    #[test]
    fn test_ip_address_masking() {
        let sanitizer = ErrorSanitizer::new();
        let result = sanitizer.sanitize("Connecting to 192.168.1.1 failed");
        assert!(!result.contains("192.168.1.1"));
        assert!(result.contains("IP_ADDRESS"));
        let result = sanitizer.sanitize("from 10.0.0.1");
        assert!(result.contains("IP_ADDRESS"));
    }

    #[test]
    fn test_aws_key_like_value_detected_and_safe() {
        // ErrorSanitizer 无 access_key 内置模式，但 contains_sensitive 通过 "key" 关键词
        // 能检测到该消息含敏感信息，safe_message 返回安全指示而不泄露原值
        let sanitizer = ErrorSanitizer::new();
        let msg = "access_key: AKIAIOSFODNN7EXAMPLE"; // pragma: allowlist secret
                                                      // 关键词检测：含 "key" 子串即视为敏感
        assert!(sanitizer.contains_sensitive(msg));
        // safe_message 应返回安全指示消息，且不泄露原值
        let safe = sanitizer.safe_message(msg, "AWS");
        assert!(!safe.contains("AKIAIOSFODNN7EXAMPLE"));
        assert!(safe.contains("Sensitive data detected"));
        assert!(safe.contains("[AWS]"));
    }

    #[test]
    fn test_add_rule_valid_and_invalid() {
        let sanitizer = ErrorSanitizer::new();
        // 有效规则
        assert!(sanitizer.add_rule(r"session_\d+", "[SESSION]").is_ok());
        let result = sanitizer.sanitize("error in session_12345 failed");
        assert!(result.contains("[SESSION]"));
        assert!(!result.contains("session_12345"));
        // 无效正则
        let err = sanitizer.add_rule(r"(", "").unwrap_err();
        assert_eq!(err, Error::InvalidPattern);
    }

    #[test]
    fn test_add_sensitive_keyword_strict() {
        let sanitizer = ErrorSanitizer::new().with_strict_mode();
        sanitizer.add_sensitive_keyword("proprietary");
        // 非严格模式下关键词不会单独被替换；严格模式会替换
        let result = sanitizer.sanitize("the proprietary value leaked");
        assert!(!result.contains("proprietary"));
        assert!(result.contains("***"));
    }

    #[test]
    fn test_contains_sensitive_and_sanitize_all() {
        let sanitizer = ErrorSanitizer::new();
        // 含敏感模式
        assert!(sanitizer.contains_sensitive("API Key: sk-abc123"));
        // 含敏感关键词
        assert!(sanitizer.contains_sensitive("the password was set"));
        // 不含敏感
        assert!(!sanitizer.contains_sensitive("normal log message"));
        // 批量脱敏
        let results = sanitizer.sanitize_all(&["API Key: sk-abcdef", "clean message"]);
        assert!(!results[0].contains("sk-abcdef"));
        assert_eq!(results[1], "clean message");
    }

    #[test]
    fn test_sanitize_with_indicator() {
        let sanitizer = ErrorSanitizer::new();
        let (sanitized, flagged) = sanitizer.sanitize_with_indicator("API Key: sk-abcdef");
        assert!(flagged);
        assert!(sanitized.contains("***"));
        let (sanitized, flagged) = sanitizer.sanitize_with_indicator("nothing sensitive");
        assert!(!flagged);
        assert_eq!(sanitized, "nothing sensitive");
    }

    #[test]
    fn test_safe_message_branches() {
        let sanitizer = ErrorSanitizer::new();
        // 含敏感 -> 返回指示消息
        let result = sanitizer.safe_message("API Key: sk-abcdef", "CTX");
        assert!(result.contains("Sensitive data detected"));
        assert!(result.contains("CTX"));
        // 不含敏感 -> 原样返回（脱敏后无变化）
        let result = sanitizer.safe_message("all good here", "CTX");
        assert_eq!(result, "all good here");
    }

    #[test]
    fn test_error_sanitizer_default() {
        let sanitizer = ErrorSanitizer::default();
        // default 与 new 行为一致
        let result = sanitizer.sanitize("Password: secret123"); // pragma: allowlist secret
        assert!(!result.contains("secret123"));
    }

    #[test]
    fn test_error_display_variants() {
        assert_eq!(
            format!("{}", Error::InvalidPattern),
            "Invalid regex pattern"
        );
        assert_eq!(format!("{}", Error::PoisonedLock), "Lock poisoned");
    }

    #[test]
    fn test_log_level_as_str() {
        assert_eq!(LogLevel::Debug.as_str(), "DEBUG");
        assert_eq!(LogLevel::Info.as_str(), "INFO");
        assert_eq!(LogLevel::Warn.as_str(), "WARN");
        assert_eq!(LogLevel::Error.as_str(), "ERROR");
    }

    #[test]
    fn test_secure_logger_levels_and_context() {
        // 高最小级别：低级别日志被跳过（不 panic）
        let logger = SecureLogger::new().with_min_level(LogLevel::Warn);
        logger.debug("skipped debug");
        logger.info("skipped info");
        logger.warn("warn with API Key: sk-abcdef"); // pragma: allowlist secret
        logger.error("error msg");

        // error_with_context：含敏感时返回指示消息
        logger.error_with_context("DB", "failed with password: secret123"); // pragma: allowlist secret
        logger.error_with_context("DB", "plain failure");

        // sanitizer() 访问器
        let sanitized = logger.sanitizer().sanitize("Password: p"); // pragma: allowlist secret
        assert!(!sanitized.contains("Password: p"));
    }

    #[test]
    fn test_secure_logger_default() {
        let logger = SecureLogger::default();
        // 默认 min_level=Debug，所有级别均记录（不 panic）
        logger.debug("d");
        logger.info("i");
        logger.warn("w");
        logger.error("e");
    }

    #[test]
    fn test_safe_result_ok_and_err() {
        // 成功结果
        let ok: SafeResult<i32> = SafeResult::ok(42);
        assert!(ok.is_ok());
        assert!(!ok.is_err());
        assert_eq!(ok.value(), Some(&42));
        assert!(!ok.contained_sensitive());
        assert_eq!(ok.unwrap(), 42);
        assert_eq!(SafeResult::ok(7).unwrap_or(0), 7);

        // 错误结果（含敏感）
        let err: SafeResult<i32> = SafeResult::err("Error password: secret123"); // pragma: allowlist secret
        assert!(!err.is_ok());
        assert!(err.is_err());
        assert!(err.error_message().is_some());
        assert!(err.contained_sensitive());
        assert_eq!(err.unwrap_or(0), 0);

        // 错误结果（不含敏感）
        let err: SafeResult<i32> = SafeResult::err("plain failure");
        assert!(!err.contained_sensitive());
        assert_eq!(err.error_message(), Some("plain failure"));
    }

    #[test]
    #[should_panic(expected = "SafeResult::unwrap()")]
    fn test_safe_result_unwrap_panic_on_err() {
        let err: SafeResult<i32> = SafeResult::err("failure");
        let _ = err.unwrap();
    }

    #[test]
    fn test_sensitive_data_filter_allowed_blocked_sanitized() {
        let mut filter = SensitiveDataFilter::new();
        assert!(filter.add_allowed_pattern(r"^ok").is_ok());
        assert!(filter.add_blocked_pattern(r".*password.*").is_ok());

        // Allowed：无敏感
        let r = filter.filter("normal message");
        assert!(r.is_allowed());
        assert!(!r.is_blocked());
        assert_eq!(r.message(), Some("normal message"));

        // Blocked：命中阻止模式
        let r = filter.filter("contains password here");
        assert!(r.is_blocked());
        assert!(!r.is_allowed());
        assert_eq!(r.message(), None);

        // Sanitized：含敏感但未被阻止
        let r = filter.filter("API Key: sk-abcdef");
        match r {
            FilterResult::Sanitized(msg) => {
                assert!(!msg.contains("sk-abcdef"));
                assert!(msg.contains("***"));
                assert!(FilterResult::Sanitized(msg.clone()).message().is_some());
            }
            _ => unreachable!("expected Sanitized"),
        }
    }

    #[test]
    fn test_sensitive_data_filter_invalid_pattern() {
        let mut filter = SensitiveDataFilter::new();
        assert_eq!(
            filter.add_allowed_pattern(r"(").unwrap_err(),
            Error::InvalidPattern
        );
        assert_eq!(
            filter.add_blocked_pattern(r"[").unwrap_err(),
            Error::InvalidPattern
        );
    }

    #[test]
    fn test_sensitive_data_filter_default_and_all() {
        let filter = SensitiveDataFilter::default();
        // 默认无阻止模式：含敏感 -> Sanitized
        let results = filter.filter_all(&["normal", "API Key: sk-abcdef"]);
        assert_eq!(results.len(), 2);
        assert!(results[0].1.is_allowed());
        // 第二条被脱敏（非 Allowed）
        assert!(!results[1].1.is_allowed());
    }

    #[test]
    fn test_filter_result_message_all_variants() {
        assert_eq!(FilterResult::Allowed("a".to_string()).message(), Some("a"));
        assert_eq!(
            FilterResult::Sanitized("b".to_string()).message(),
            Some("b")
        );
        assert_eq!(
            FilterResult::Blocked {
                reason: "r".to_string()
            }
            .message(),
            None
        );
    }
}
