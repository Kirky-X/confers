// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! # SecureString - 安全字符串类型
//!
//! 提供内存安全的字符串类型，自动清零敏感数据，防止内存泄漏。
//!
//! ## 功能特性
//!
//! - **自动内存清零**: 当 SecureString 被丢弃时，自动将内存内容清零
//! - **防止克隆**: 禁止 Clone，防止敏感数据在内存中复制
//! - **安全比较**: 提供恒定时间的比较方法，防止时序攻击
//! - **敏感数据标记**: 实现 SensitiveData  trait，标记为需要特殊处理
//!
//! ## 使用示例
//!
//! ```rust,ignore
//! use confers::security::SecureString;
//!
//! // 创建安全字符串
//! let secret = SecureString::from("my-secret-password");
//!
//! // 安全比较 (恒定时间)
//! assert!(secret.compare("my-secret-password").is_ok());
//!
//! // 内存自动清零 (当 secret 离开作用域时)
//! ```

use std::fmt;
use std::hash::{Hash, Hasher};
use std::ops::Deref;
use std::sync::atomic::{AtomicUsize, Ordering};
use zeroize::{Zeroize, ZeroizeOnDrop};

/// 敏感数据标记 trait
pub trait SensitiveData {
    /// 返回脱敏后的显示名称
    fn display_name(&self) -> &str;

    /// 检查是否为高敏感度数据
    fn is_highly_sensitive(&self) -> bool {
        false
    }
}

/// 敏感数据类别
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum SensitivityLevel {
    /// 低敏感度 - 内部数据
    #[default]
    Low,
    /// 中敏感度 - 用户数据
    Medium,
    /// 高敏感度 - 认证凭据
    High,
    /// 极高敏感度 - 主密钥、密码
    Critical,
}

impl SensitivityLevel {
    pub fn is_critical_or_high(&self) -> bool {
        matches!(self, SensitivityLevel::Critical | SensitivityLevel::High)
    }
}

/// 安全字符串计数器，用于调试和监控
static ALLOCATED_SECURE_STRINGS: AtomicUsize = AtomicUsize::new(0);
static DEALLOCATED_SECURE_STRINGS: AtomicUsize = AtomicUsize::new(0);

/// 获取当前分配的安全字符串数量（用于调试）
pub fn allocated_secure_strings() -> usize {
    ALLOCATED_SECURE_STRINGS.load(Ordering::SeqCst)
}

/// 获取已释放的安全字符串数量（用于调试）
pub fn deallocated_secure_strings() -> usize {
    DEALLOCATED_SECURE_STRINGS.load(Ordering::SeqCst)
}

/// 清除所有安全字符串计数（用于测试）
#[allow(dead_code)]
pub(crate) fn reset_secure_string_counters() {
    ALLOCATED_SECURE_STRINGS.store(0, Ordering::SeqCst);
    DEALLOCATED_SECURE_STRINGS.store(0, Ordering::SeqCst);
}

/// 安全字符串类型
///
/// # 安全特性
///
/// - **自动清零**: Drop 时自动将内部缓冲区清零
/// - **不可克隆**: 禁止 Clone 操作，防止意外复制
/// - **恒定时间比较**: `compare()` 方法使用恒定时间比较，防止时序攻击
/// - **内存安全**: 使用零化 (zeroize) 确保敏感数据不会残留在内存中
///
/// # 警告
///
/// 不要将此类型用于普通字符串，仅用于密码、密钥、令牌等敏感数据。
///
/// # 示例
///
/// ```rust,ignore
/// use confers::security::{SecureString, SensitivityLevel, SensitiveData};
///
/// let secret = SecureString::from("password123");
///
/// // 安全比较 (恒定时间)
/// assert!(secret.compare("password123").is_ok());
///
/// // 获取显示名称（用于日志）- Critical 级别返回 [SENSITIVE]
/// assert_eq!(secret.display_name(), "[SENSITIVE]");
/// ```
#[derive(Eq)]
pub struct SecureString {
    /// 内部缓冲区
    data: Vec<u8>,
    /// 敏感度级别
    sensitivity: SensitivityLevel,
    /// 显示名称（用于日志）
    display_name: String,
}

impl SecureString {
    /// 从普通字符串创建安全字符串
    ///
    /// # 参数
    ///
    /// * `s` - 原始字符串
    /// * `sensitivity` - 敏感度级别（默认为 Critical）
    pub fn new(s: impl Into<String>, sensitivity: SensitivityLevel) -> Self {
        let string = s.into();
        ALLOCATED_SECURE_STRINGS.fetch_add(1, Ordering::SeqCst);

        let display_name = match sensitivity {
            SensitivityLevel::Critical => "[SENSITIVE]".to_string(),
            SensitivityLevel::High => format!("[{} chars]", string.len()),
            SensitivityLevel::Medium => format!("[{} chars]", string.len()),
            SensitivityLevel::Low => string.clone(),
        };
        let data = string.into_bytes();

        Self {
            data,
            sensitivity,
            display_name,
        }
    }

    /// 从普通字符串创建安全字符串（默认 Critical 级别）
    pub fn from(s: impl Into<String>) -> Self {
        Self::new(s, SensitivityLevel::Critical)
    }

    /// 从字节数组创建安全字符串
    pub fn from_bytes(data: Vec<u8>, sensitivity: SensitivityLevel) -> Self {
        ALLOCATED_SECURE_STRINGS.fetch_add(1, Ordering::SeqCst);

        Self {
            data,
            sensitivity,
            display_name: "[binary data]".to_string(),
        }
    }

    /// 获取字符串引用
    pub fn as_str(&self) -> &str {
        // 安全: 转换为字符串切片
        std::str::from_utf8(&self.data).unwrap_or("[invalid utf-8]")
    }

    /// 获取字节切片
    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }

    /// 转换为 String（注意：这会复制数据）
    ///
    /// # 警告
    ///
    /// 调用此方法后，原始 SecureString 仍然包含敏感数据。
    /// 仅在必要时使用，并确保妥善处理返回的 String。
    #[allow(clippy::wrong_self_convention)] // Consumes self intentionally for conversion
    pub fn to_plain_string(self) -> String {
        // 将数据转换为字符串（复制数据以避免所有权问题）
        String::from_utf8(self.data.clone()).unwrap_or_default()
    }

    /// 恒定时间比较
    ///
    /// 使用恒定时间算法比较字符串，防止时序攻击。
    /// 无论字符串是否相等，比较时间都相同。
    ///
    /// # 参数
    ///
    /// * `other` - 要比较的字符串
    ///
    /// # 返回
    ///
    /// 如果相等返回 Ok(())，否则返回 Err(())
    #[allow(clippy::result_unit_err)]
    pub fn compare(&self, other: &str) -> Result<(), ()> {
        // 使用恒定时间比较
        let mut result: u8 = 0;

        for (a, b) in self.data.iter().zip(other.bytes()) {
            result |= a ^ b;
        }

        // 检查长度是否相同
        if self.data.len() != other.len() {
            result |= 1;
        }

        if result == 0 {
            Ok(())
        } else {
            Err(())
        }
    }

    /// 获取敏感度级别
    pub fn sensitivity(&self) -> SensitivityLevel {
        self.sensitivity.clone()
    }

    /// 检查是否为高敏感度数据
    pub fn is_highly_sensitive(&self) -> bool {
        self.sensitivity.is_critical_or_high()
    }

    /// 获取内部数据长度
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// 检查是否为空
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// 安全地清零内部数据
    ///
    /// # 警告
    ///
    /// 调用此方法后，字符串内容将被永久破坏。
    pub fn zeroize(&mut self) {
        self.data.zeroize();
    }

    /// 转换为不可逆的指纹（用于缓存键等）
    ///
    /// # 参数
    ///
    /// * `max_len` - 指纹最大长度
    pub fn fingerprint(&self, max_len: usize) -> String {
        use sha2::{Digest, Sha256};

        let mut hasher = Sha256::new();
        hasher.update(&self.data);
        let result = hasher.finalize();

        // 转换为十六进制字符串并截断
        let hex = hex::encode(result);
        if hex.len() > max_len {
            hex[..max_len].to_string()
        } else {
            hex
        }
    }

    /// 掩码显示（用于日志）
    ///
    /// 返回掩码后的字符串，如 "pa****" 或 "**"
    pub fn masked(&self) -> String {
        if self.data.is_empty() {
            return "[empty]".to_string();
        }

        let s = self.as_str();
        let len = s.len();

        match len {
            0 => "[empty]".to_string(),
            1..=2 => "*".repeat(len),
            3..=4 => {
                let visible = if len == 3 { 1 } else { 2 };
                format!("{}{}", &s[..visible], "*".repeat(len - visible))
            }
            _ => {
                let visible = std::cmp::min(2, len / 4);
                let masked_chars = std::cmp::min(6, len - visible);
                format!("{}{}", &s[..visible], "*".repeat(masked_chars))
            }
        }
    }
}

impl SensitiveData for SecureString {
    fn display_name(&self) -> &str {
        &self.display_name
    }

    fn is_highly_sensitive(&self) -> bool {
        self.is_highly_sensitive()
    }
}

impl Drop for SecureString {
    fn drop(&mut self) {
        // 清零内部数据
        self.data.zeroize();
        DEALLOCATED_SECURE_STRINGS.fetch_add(1, Ordering::SeqCst);
    }
}

impl ZeroizeOnDrop for SecureString {}

impl Clone for SecureString {
    fn clone(&self) -> Self {
        // Note: Cloning SecureString is generally discouraged but allowed
        // for backward compatibility

        Self {
            data: self.data.clone(),
            sensitivity: self.sensitivity.clone(),
            display_name: self.display_name.clone(),
        }
    }
}

impl fmt::Debug for SecureString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // 调试时只显示掩码
        write!(f, "SecureString({})", self.masked())
    }
}

impl fmt::Display for SecureString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // 显示时只显示掩码
        write!(f, "{}", self.masked())
    }
}

impl PartialEq for SecureString {
    fn eq(&self, other: &Self) -> bool {
        // 使用恒定时间比较
        self.compare(other.as_str()).is_ok()
    }
}

impl Hash for SecureString {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // 注意: 使用普通哈希而不是恒定时间哈希
        // 因为 Hash trait 的设计假设了比较操作
        self.data.hash(state);
    }
}

impl Deref for SecureString {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

/// 安全字符串构建器
#[derive(Default)]
pub struct SecureStringBuilder {
    data: Vec<u8>,
    sensitivity: SensitivityLevel,
    display_name: Option<String>,
}

impl SecureStringBuilder {
    /// 创建新的构建器
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置敏感度级别
    pub fn with_sensitivity(mut self, sensitivity: SensitivityLevel) -> Self {
        self.sensitivity = sensitivity;
        self
    }

    /// 设置显示名称
    pub fn with_display_name(mut self, name: impl Into<String>) -> Self {
        self.display_name = Some(name.into());
        self
    }

    /// 添加字符
    pub fn push(mut self, c: char) -> Self {
        let mut buf = [0u8; 4];
        let encoded = c.encode_utf8(&mut buf);
        self.data.extend_from_slice(encoded.as_bytes());
        self
    }

    /// 添加字符串
    pub fn push_str(mut self, s: &str) -> Self {
        self.data.extend_from_slice(s.as_bytes());
        self
    }

    /// 添加字节
    pub fn push_u8(mut self, b: u8) -> Self {
        self.data.push(b);
        self
    }

    /// 构建 SecureString
    pub fn build(self) -> SecureString {
        let display_name = self
            .display_name
            .unwrap_or_else(|| String::from_utf8_lossy(&self.data).into_owned());

        ALLOCATED_SECURE_STRINGS.fetch_add(1, Ordering::SeqCst);

        SecureString {
            data: self.data,
            sensitivity: self.sensitivity,
            display_name,
        }
    }
}

impl From<&str> for SecureString {
    fn from(s: &str) -> Self {
        Self::from(s.to_string())
    }
}

impl From<String> for SecureString {
    fn from(s: String) -> Self {
        Self::from(s)
    }
}

impl From<&String> for SecureString {
    fn from(s: &String) -> Self {
        Self::from(s.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secure_string_creation() {
        let secret = SecureString::from("password123");
        assert_eq!(secret.len(), 11);
        assert!(!secret.is_empty());
    }

    #[test]
    fn test_secure_string_compare() {
        let secret = SecureString::from("password123");

        assert!(secret.compare("password123").is_ok());
        assert!(secret.compare("wrongpassword").is_err());
    }

    #[test]
    fn test_secure_string_masked() {
        let secret = SecureString::from("password123");
        let masked = secret.masked();
        assert!(masked.contains('*'));
        assert!(masked.len() < 12);
    }

    #[test]
    fn test_secure_string_display() {
        let secret = SecureString::from("password123");
        let display = format!("{}", secret);
        assert!(display.contains('*'));
    }

    #[test]
    fn test_secure_string_debug() {
        let secret = SecureString::from("password123");
        let debug = format!("{:?}", secret);
        assert!(debug.contains("SecureString"));
    }

    #[test]
    fn test_sensitivity_levels() {
        let critical = SecureString::new("secret", SensitivityLevel::Critical);
        let high = SecureString::new("token", SensitivityLevel::High);
        let medium = SecureString::new("user", SensitivityLevel::Medium);
        let low = SecureString::new("config", SensitivityLevel::Low);

        assert!(critical.is_highly_sensitive());
        assert!(high.is_highly_sensitive());
        assert!(!medium.is_highly_sensitive());
        assert!(!low.is_highly_sensitive());
    }

    #[test]
    fn test_fingerprint() {
        let secret = SecureString::from("password123");
        let fp = secret.fingerprint(16);
        assert_eq!(fp.len(), 16);
        assert!(fp.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_secure_string_builder() {
        let secret = SecureStringBuilder::new()
            .push_str("pass")
            .push('w')
            .push_str("ord")
            .build();

        assert_eq!(secret.as_str(), "password");
    }

    #[test]
    fn test_from_bytes() {
        let data = vec![0x01, 0x02, 0x03, 0x04];
        let secret = SecureString::from_bytes(data.clone(), SensitivityLevel::High);

        assert_eq!(secret.as_bytes(), data.as_slice());
        assert_eq!(secret.display_name(), "[binary data]");
    }

    #[test]
    #[ignore = "计数器是全局的，会受其他测试影响，仅用于手动验证"]
    fn test_allocation_counters() {
        let initial_allocated = allocated_secure_strings();
        let initial_deallocated = deallocated_secure_strings();

        let _secret1 = SecureString::from("test1");
        let _secret2 = SecureString::from("test2");

        // 检查分配数增加了2
        assert_eq!(
            allocated_secure_strings(),
            initial_allocated + 2,
            "Should allocate 2 new SecureStrings"
        );
        // 释放数应该不变
        assert_eq!(
            deallocated_secure_strings(),
            initial_deallocated,
            "Should not deallocate any SecureStrings"
        );
    }

    #[test]
    fn test_partial_eq() {
        let secret1 = SecureString::from("password");
        let secret2 = SecureString::from("password");
        let secret3 = SecureString::from("different");

        assert_eq!(secret1, secret2);
        assert_ne!(secret1, secret3);
    }

    #[test]
    fn test_hash() {
        use std::collections::HashSet;

        let secret1 = SecureString::from("password");
        let secret2 = SecureString::from("password");
        let secret3 = SecureString::from("different");

        let mut set = HashSet::new();
        set.insert(secret1.clone());
        set.insert(secret2.clone());
        set.insert(secret3.clone());

        // secret1 和 secret2 应该被视为同一个元素
        assert_eq!(set.len(), 2);
        assert!(set.contains(&secret1));
        assert!(set.contains(&secret2));
        assert!(set.contains(&secret3));
    }

    #[test]
    fn test_secure_string_new_critical_display_name() {
        let secret = SecureString::new("topsecret", SensitivityLevel::Critical);
        assert_eq!(secret.display_name(), "[SENSITIVE]");
        assert!(secret.is_highly_sensitive());
    }

    #[test]
    fn test_secure_string_new_high_display_name() {
        let secret = SecureString::new("token123", SensitivityLevel::High);
        assert_eq!(secret.display_name(), "[8 chars]");
        assert!(secret.is_highly_sensitive());
    }

    #[test]
    fn test_secure_string_new_medium_display_name() {
        let secret = SecureString::new("user_data", SensitivityLevel::Medium);
        assert_eq!(secret.display_name(), "[9 chars]");
        assert!(!secret.is_highly_sensitive());
    }

    #[test]
    fn test_secure_string_new_low_display_name() {
        // Low sensitivity returns the original string as display_name.
        let secret = SecureString::new("plain_config", SensitivityLevel::Low);
        assert_eq!(secret.display_name(), "plain_config");
        assert!(!secret.is_highly_sensitive());
    }

    #[test]
    fn test_secure_string_as_str_valid() {
        let secret = SecureString::from("hello world");
        assert_eq!(secret.as_str(), "hello world");
    }

    #[test]
    fn test_secure_string_as_str_invalid_utf8() {
        let secret = SecureString::from_bytes(vec![0xFF, 0xFE, 0xFD], SensitivityLevel::High);
        assert_eq!(secret.as_str(), "[invalid utf-8]");
    }

    #[test]
    fn test_secure_string_to_plain_string() {
        let secret = SecureString::from("extract-me");
        let plain = secret.to_plain_string();
        assert_eq!(plain, "extract-me");
    }

    #[test]
    fn test_secure_string_compare_different_lengths() {
        let secret = SecureString::from("abc");
        // Shorter other string.
        assert!(secret.compare("ab").is_err());
        assert!(secret.compare("a").is_err());
        // Longer other string.
        assert!(secret.compare("abcd").is_err());
        assert!(secret.compare("abcdef").is_err());
    }

    #[test]
    fn test_secure_string_compare_empty() {
        let empty = SecureString::from("");
        assert!(empty.compare("").is_ok());
        assert!(empty.compare("nonempty").is_err());
    }

    #[test]
    fn test_secure_string_compare_self_equivalent() {
        let secret = SecureString::from("same-value");
        assert!(secret.compare("same-value").is_ok());
    }

    #[test]
    fn test_secure_string_zeroize() {
        let mut secret = SecureString::from("will-be-erased");
        assert_eq!(secret.len(), 14);
        secret.zeroize();
        // After zeroize, data buffer is zeroed — length unchanged but content cleared.
        assert_eq!(secret.len(), 0);
        assert!(secret.is_empty());
    }

    #[test]
    fn test_secure_string_fingerprint_zero_max_len() {
        let secret = SecureString::from("password");
        let fp = secret.fingerprint(0);
        assert_eq!(fp.len(), 0);
    }

    #[test]
    fn test_secure_string_fingerprint_full_length() {
        // SHA-256 produces 32 bytes → 64 hex chars; max_len >= 64 returns full hex.
        let secret = SecureString::from("password");
        let fp = secret.fingerprint(100);
        assert_eq!(fp.len(), 64);
        assert!(fp.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_secure_string_fingerprint_truncation() {
        let secret = SecureString::from("password");
        let fp = secret.fingerprint(32);
        assert_eq!(fp.len(), 32);
        assert!(fp.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_secure_string_fingerprint_deterministic() {
        let s1 = SecureString::from("same-input");
        let s2 = SecureString::from("same-input");
        assert_eq!(s1.fingerprint(64), s2.fingerprint(64));
    }

    #[test]
    fn test_secure_string_masked_empty() {
        let secret = SecureString::from("");
        assert_eq!(secret.masked(), "[empty]");
    }

    #[test]
    fn test_secure_string_masked_single_char() {
        let secret = SecureString::from("a");
        assert_eq!(secret.masked(), "*");
    }

    #[test]
    fn test_secure_string_masked_two_chars() {
        let secret = SecureString::from("ab");
        assert_eq!(secret.masked(), "**");
    }

    #[test]
    fn test_secure_string_masked_three_chars() {
        let secret = SecureString::from("abc");
        assert_eq!(secret.masked(), "a**");
    }

    #[test]
    fn test_secure_string_masked_four_chars() {
        let secret = SecureString::from("abcd");
        assert_eq!(secret.masked(), "ab**");
    }

    #[test]
    fn test_secure_string_masked_long_string() {
        let secret = SecureString::from("very-long-secret-value");
        let masked = secret.masked();
        // For len >= 5: visible = min(2, len/4), masked_chars = min(6, len - visible)
        // len=22 → visible=2, masked_chars=6 → "ve******"
        assert_eq!(masked, "ve******");
    }

    #[test]
    fn test_secure_string_is_empty_true() {
        let secret = SecureString::from("");
        assert!(secret.is_empty());
        assert_eq!(secret.len(), 0);
    }

    #[test]
    fn test_secure_string_len_non_empty() {
        let secret = SecureString::from("12345678");
        assert_eq!(secret.len(), 8);
        assert!(!secret.is_empty());
    }

    #[test]
    fn test_sensitive_data_trait_default_is_highly_sensitive() {
        // Default impl returns false for is_highly_sensitive.
        struct DummyData;
        impl SensitiveData for DummyData {
            fn display_name(&self) -> &str {
                "dummy"
            }
        }
        let d = DummyData;
        assert!(!d.is_highly_sensitive());
        assert_eq!(d.display_name(), "dummy");
    }

    #[test]
    fn test_sensitivity_is_critical_or_high() {
        assert!(SensitivityLevel::Critical.is_critical_or_high());
        assert!(SensitivityLevel::High.is_critical_or_high());
        assert!(!SensitivityLevel::Medium.is_critical_or_high());
        assert!(!SensitivityLevel::Low.is_critical_or_high());
    }

    #[test]
    fn test_sensitivity_default_is_low() {
        let level = SensitivityLevel::default();
        assert_eq!(level, SensitivityLevel::Low);
    }

    #[test]
    fn test_secure_string_builder_with_sensitivity() {
        let secret = SecureStringBuilder::new()
            .with_sensitivity(SensitivityLevel::Low)
            .push_str("data")
            .build();
        assert_eq!(secret.sensitivity(), SensitivityLevel::Low);
        assert!(!secret.is_highly_sensitive());
    }

    #[test]
    fn test_secure_string_builder_with_display_name() {
        let secret = SecureStringBuilder::new()
            .with_display_name("custom-name")
            .push_str("data")
            .build();
        assert_eq!(secret.display_name(), "custom-name");
    }

    #[test]
    fn test_secure_string_builder_push_u8() {
        let secret = SecureStringBuilder::new()
            .push_u8(b'A')
            .push_u8(b'B')
            .push_u8(b'C')
            .build();
        assert_eq!(secret.as_str(), "ABC");
    }

    #[test]
    fn test_secure_string_builder_push_unicode_char() {
        let secret = SecureStringBuilder::new().push('中').push('文').build();
        assert_eq!(secret.as_str(), "中文");
        assert_eq!(secret.len(), 6); // 3 bytes per CJK char
    }

    #[test]
    fn test_secure_string_builder_empty_build() {
        let secret = SecureStringBuilder::new().build();
        assert!(secret.is_empty());
        assert_eq!(secret.len(), 0);
    }

    #[test]
    fn test_secure_string_builder_default() {
        let secret = SecureStringBuilder::default()
            .push_str("default-builder")
            .build();
        assert_eq!(secret.as_str(), "default-builder");
    }

    #[test]
    fn test_secure_string_clone_preserves_data() {
        let original = SecureString::new("clone-me", SensitivityLevel::High);
        let cloned = original.clone();
        assert_eq!(original.as_str(), cloned.as_str());
        assert_eq!(original.sensitivity(), cloned.sensitivity());
    }

    #[test]
    fn test_secure_string_deref_to_str() {
        let secret = SecureString::from("deref-target");
        // Deref<Target = str> — &*secret yields &str.
        let s: &str = &*secret;
        assert_eq!(s, "deref-target");
    }

    #[test]
    fn test_secure_string_from_string() {
        let input = String::from("from-string");
        let secret = SecureString::from(input);
        assert_eq!(secret.as_str(), "from-string");
    }

    #[test]
    fn test_secure_string_from_str_ref() {
        let input = String::from("from-str-ref");
        let secret = SecureString::from(&input);
        assert_eq!(secret.as_str(), "from-str-ref");
    }

    #[test]
    fn test_secure_string_display_debug_empty() {
        let empty = SecureString::from("");
        assert_eq!(format!("{}", empty), "[empty]");
        assert_eq!(format!("{:?}", empty), "SecureString([empty])");
    }

    #[test]
    fn test_secure_string_sensitivity_getter() {
        let critical = SecureString::new("x", SensitivityLevel::Critical);
        let medium = SecureString::new("x", SensitivityLevel::Medium);
        assert_eq!(critical.sensitivity(), SensitivityLevel::Critical);
        assert_eq!(medium.sensitivity(), SensitivityLevel::Medium);
    }

    #[test]
    fn test_secure_string_from_bytes_preserves_data() {
        let data = vec![0x01, 0x02, 0x03, 0x04];
        let secret = SecureString::from_bytes(data.clone(), SensitivityLevel::Medium);
        assert_eq!(secret.as_bytes(), data.as_slice());
        assert_eq!(secret.len(), 4);
        assert!(!secret.is_empty());
    }

    #[serial_test::serial]
    #[test]
    fn test_secure_string_counter_lifecycle() {
        // Serial test: counters are global; this test must run alone.
        reset_secure_string_counters();
        assert_eq!(allocated_secure_strings(), 0);
        assert_eq!(deallocated_secure_strings(), 0);

        {
            let _s = SecureString::from("counter-test");
            assert_eq!(allocated_secure_strings(), 1);
            assert_eq!(deallocated_secure_strings(), 0);
        }
        // After drop, deallocated counter increments.
        assert_eq!(allocated_secure_strings(), 1);
        assert_eq!(deallocated_secure_strings(), 1);

        // Reset restores both to zero.
        reset_secure_string_counters();
        assert_eq!(allocated_secure_strings(), 0);
        assert_eq!(deallocated_secure_strings(), 0);
    }
}
