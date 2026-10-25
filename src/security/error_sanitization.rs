// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! # 错误消息脱敏
//!
//! 提供错误消息的安全处理功能，确保敏感信息不会泄露到日志或用户界面。
//!
//! ## 功能特性
//!
//! - **敏感信息脱敏**: 自动检测和脱敏错误消息中的敏感数据
//! - **自定义脱敏规则**: 支持自定义脱敏模式和规则

use crate::security::patterns::{SENSITIVE_KEYWORDS, contains_as_token};
use regex::Regex;
use std::collections::HashSet;
use std::sync::{Arc, OnceLock, RwLock};

/// Maximum number of characters [`ErrorSanitizer::sanitize`] processes.
///
/// Input longer than this is truncated (on a char boundary) before pattern
/// matching. The built-in regexes are linear-time (the `regex` crate
/// guarantees no catastrophic backtracking); the cap is defense-in-depth
/// against unbounded input sizes, and content beyond the cap is dropped
/// rather than leaked.
const MAX_SANITIZE_INPUT_CHARS: usize = 10_000;

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
///
/// 语义说明:本类型的 `MaskedGroup` 替换会**保留值的前 4-8 个字符**
/// (如 `pa****`),与库内其他脱敏路径(定长 `********` / `[REDACTED]`)不同。
/// 适用于需要在日志中辨识凭证"种类"的诊断场景;若要求不泄露前缀,请改用
/// [`crate::error::sanitize::sanitize_error_message`] 全路径。
///
/// # Warning
///
/// 保留的前缀对短凭证而言就是凭证本身的一部分(例如 4 字符的口令前缀
/// 仍可缩小搜索空间)。**不要**把本类型的输出当作"已脱敏"的凭据写进日志
/// 或告警载荷;需要全量替换时用
/// [`crate::error::sanitize::sanitize_error_message`]。
///
/// 公开面只有 [`crate::security::SecurityErrorSanitizer`] 一个名字（裸名
/// `ErrorSanitizer` 不再从 `security` 模块导出）。[`crate::key::ErrorSanitizer`]
/// 是另一个基于主密钥做全量替换的无关类型。
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
        let mut keywords = self
            .sensitive_keywords
            .write()
            .unwrap_or_else(|e| e.into_inner());
        keywords.insert(keyword.to_lowercase());
    }

    /// 脱敏错误消息
    ///
    /// Input longer than `MAX_SANITIZE_INPUT_CHARS` characters is truncated
    /// before matching (see the constant's documentation).
    pub fn sanitize(&self, message: &str) -> String {
        // Truncate oversized input before matching, counting characters so the
        // cut always lands on a UTF-8 boundary.
        let mut result = if message.chars().count() > MAX_SANITIZE_INPUT_CHARS {
            message.chars().take(MAX_SANITIZE_INPUT_CHARS).collect()
        } else {
            message.to_string()
        };

        // 应用内置规则
        for (pattern, replacement) in get_sensitive_patterns().iter() {
            result = apply_replacement(&result, pattern, replacement);
        }

        // 应用自定义规则
        let custom_rules = self.custom_rules.read().unwrap_or_else(|e| e.into_inner());
        for (pattern, replacement) in custom_rules.iter() {
            result = pattern
                .replace_all(&result, replacement.as_str())
                .to_string();
        }

        // 如果在严格模式，替换所有敏感关键词
        if self.strict_mode {
            let keywords = self
                .sensitive_keywords
                .read()
                .unwrap_or_else(|e| e.into_inner());
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
    ///
    /// Keywords are matched with token boundaries (see
    /// `patterns::contains_as_token`), consistent with the word-boundary
    /// behavior of strict-mode sanitization: `my_key` matches the keyword
    /// `key`, while `monkey`, `whiskey`, or `secretary` do not.
    pub fn contains_sensitive(&self, message: &str) -> bool {
        for (pattern, _) in get_sensitive_patterns().iter() {
            if pattern.is_match(message) {
                return true;
            }
        }

        // 检查敏感关键词（token 边界匹配，避免子串误报）
        let keywords = self
            .sensitive_keywords
            .read()
            .unwrap_or_else(|e| e.into_inner());
        let message_lower = message.to_lowercase();
        keywords
            .iter()
            .any(|keyword| contains_as_token(&message_lower, keyword))
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
    fn test_sanitize_input_length_cap() {
        let sanitizer = ErrorSanitizer::new();

        // Input beyond the cap is truncated before matching: sensitive data
        // past the cap is dropped entirely and never appears in the output.
        let mut oversized = "a".repeat(MAX_SANITIZE_INPUT_CHARS + 5_000);
        oversized.push_str(" password: supersecret123"); // pragma: allowlist secret
        let result = sanitizer.sanitize(&oversized);
        assert!(!result.contains("supersecret123"));
        assert!(result.chars().count() <= MAX_SANITIZE_INPUT_CHARS);

        // Sensitive data within the cap is still masked as usual.
        let within = format!(
            "{} password: abc",
            "x".repeat(MAX_SANITIZE_INPUT_CHARS - 20)
        ); // pragma: allowlist secret
        let result = sanitizer.sanitize(&within);
        assert!(!result.contains("password: abc"));
        assert!(result.contains("***"));
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
    fn test_contains_sensitive_token_boundaries() {
        // Token-bounded keywords match.
        let sanitizer = ErrorSanitizer::new();
        assert!(sanitizer.contains_sensitive("my_key is configured"));
        assert!(sanitizer.contains_sensitive("client_secret: xxx")); // pragma: allowlist secret
        // Substrings inside larger words no longer match (no false positives).
        assert!(!sanitizer.contains_sensitive("the monkey escaped"));
        assert!(!sanitizer.contains_sensitive("a whiskey bar"));
        assert!(!sanitizer.contains_sensitive("the secretary will call"));
        assert!(!sanitizer.contains_sensitive("the author wrote a book"));
        assert!(!sanitizer.contains_sensitive("keyboard layout"));
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
}
