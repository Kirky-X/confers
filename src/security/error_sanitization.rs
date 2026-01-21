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
        let mut set = HashSet::new();
        set.insert("password".to_string());
        set.insert("secret".to_string());
        set.insert("token".to_string());
        set.insert("key".to_string());
        set.insert("credential".to_string());
        set.insert("auth".to_string());
        set.insert("private".to_string());
        set.insert("encryption".to_string());
        set
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
                    if s.len() <= *visible_chars {
                        "*".repeat(s.len())
                    } else {
                        format!("{}***", &s[..*visible_chars])
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
                        if local_part.len() <= 2 {
                            "***".to_string() + domain
                        } else {
                            format!("{}**{}", &local_part[..2], domain)
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

        #[cfg(feature = "tracing")]
        match level {
            LogLevel::Error => tracing::error!("{}", log_entry),
            LogLevel::Warn => tracing::warn!("{}", log_entry),
            LogLevel::Info => tracing::info!("{}", log_entry),
            LogLevel::Debug => tracing::debug!("{}", log_entry),
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
            _ => panic!("Expected sanitized result"),
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
}
