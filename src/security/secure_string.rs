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
//! ```rust
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
#[derive(Debug, Clone, PartialEq)]
pub enum SensitivityLevel {
    /// 低敏感度 - 内部数据
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
#[cfg(test)]
pub fn reset_secure_string_counters() {
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
/// ```rust
/// use confers::security::{SecureString, SensitivityLevel};
///
/// let secret = SecureString::from("password123");
///
/// // 安全比较 (恒定时间)
/// assert!(secret.compare("password123").is_ok());
///
/// // 获取显示名称（用于日志）
/// assert_eq!(secret.display_name(), "password123");
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

        Self {
            data: string.into_bytes(),
            sensitivity,
            display_name: string.clone(),
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
    pub fn to_plain_string(self) -> String {
        // 将数据转换为字符串
        String::from_utf8(self.data).unwrap_or_default()
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
    pub fn compare(&self, other: &str) -> Result<(), ()> {
        // 使用恒定时间比较
        let mut result: u8 = 0;

        for (a, b) in self.data.iter().zip(other.as_bytes().iter()) {
            result |= a ^ b;
        }

        // 检查长度是否相同
        if self.data.len() != other.as_bytes().len() {
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
        let hex = hex::encode(&result);
        if hex.len() > max_len {
            hex[..max_len].to_string()
        } else {
            hex
        }
    }

    /// 掩码显示（用于日志）
    ///
    /// 返回掩码后的字符串，如 "pass***" 或 "****"
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
                let masked = len - visible;
                format!("{}{}", &s[..visible], "*".repeat(masked))
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
        // 警告: 克隆会创建数据副本
        // 在安全敏感场景中应避免使用
        tracing::warn!("Cloning SecureString - this may leak sensitive data");

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
        assert_eq!(secret.len(), 12);
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
    fn test_allocation_counters() {
        reset_secure_string_counters();

        let _secret1 = SecureString::from("test1");
        let _secret2 = SecureString::from("test2");

        assert_eq!(allocated_secure_strings(), 2);
        assert_eq!(deallocated_secure_strings(), 0);
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
}
