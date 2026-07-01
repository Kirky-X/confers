// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! # 环境变量注入机制
//!
//! 提供安全的运行时配置注入，支持通过环境变量动态配置系统参数。
//!
//! ## 功能特性
//!
//! - **运行时注入**: 动态注入配置值，不依赖编译时常量
//! - **安全验证**: 内置安全验证器，防止注入恶意配置
//! - **类型安全**: 支持多种配置类型的自动转换
//! - **敏感数据保护**: 自动检测和保护敏感配置
//!
//! ## 使用示例
//!
//! ```rust,ignore
//! use confers::security::{ConfigInjector, EnvironmentConfig};
//!
//! // 创建配置注入器
//! let injector = ConfigInjector::new();
//!
//! // 注入配置值
//! if let Err(e) = injector.inject("APP_SECRET", "my-secret-value") {
//!     eprintln!("注入失败: {:?}", e);
//! }
//!
//! // 获取配置
//! if let Some(value) = injector.get("APP_SECRET") {
//!     println!("配置值: {}", value);
//! }
//! ```

use crate::security::patterns::SENSITIVE_DETECTION_PATTERNS;
use crate::security::{EnvSecurityError, EnvSecurityValidator};
use regex::Regex;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock, RwLock};

/// Rate limiter configuration defaults
const DEFAULT_RATE_LIMIT_MAX_REQUESTS: usize = 100;
const DEFAULT_RATE_LIMIT_WINDOW_SECONDS: u64 = 60;

/// Environment variable names for rate limiter configuration
const ENV_RATE_LIMIT_MAX_REQUESTS: &str = "CONFERS_RATE_LIMIT_MAX_REQUESTS";
const ENV_RATE_LIMIT_WINDOW_SECONDS: &str = "CONFERS_RATE_LIMIT_WINDOW_SECONDS";

/// Get rate limit max requests from environment or default.
fn get_rate_limit_max_requests() -> usize {
    std::env::var(ENV_RATE_LIMIT_MAX_REQUESTS)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_RATE_LIMIT_MAX_REQUESTS)
}

/// Get rate limit window seconds from environment or default.
fn get_rate_limit_window_seconds() -> u64 {
    std::env::var(ENV_RATE_LIMIT_WINDOW_SECONDS)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_RATE_LIMIT_WINDOW_SECONDS)
}

/// Configuration injection rate limiter
#[derive(Debug, Clone)]
pub(crate) struct InjectionRateLimiter {
    window_counter: Arc<AtomicU64>,
    window_start: Arc<AtomicU64>,
    max_requests: usize,
    window_seconds: u64,
    rate_limiting_enabled: bool,
}

impl Default for InjectionRateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

impl InjectionRateLimiter {
    /// Create a new rate limiter with default settings.
    ///
    /// Default values can be configured via environment variables:
    /// - `CONFERS_RATE_LIMIT_MAX_REQUESTS`: Maximum requests per window (default: 100)
    /// - `CONFERS_RATE_LIMIT_WINDOW_SECONDS`: Window duration in seconds (default: 60)
    pub fn new() -> Self {
        Self {
            window_counter: Arc::new(AtomicU64::new(0)),
            window_start: Arc::new(AtomicU64::new(0)),
            max_requests: get_rate_limit_max_requests(),
            window_seconds: get_rate_limit_window_seconds(),
            rate_limiting_enabled: true,
        }
    }

    #[allow(dead_code)]
    /// Create a rate limiter that is disabled (for testing)
    pub fn disabled() -> Self {
        Self {
            window_counter: Arc::new(AtomicU64::new(0)),
            window_start: Arc::new(AtomicU64::new(0)),
            max_requests: usize::MAX,
            window_seconds: u64::MAX,
            rate_limiting_enabled: false,
        }
    }

    #[allow(dead_code)]
    /// Create a rate limiter with custom settings
    pub fn with_limits(max_requests: usize, window_seconds: u64) -> Self {
        Self {
            window_counter: Arc::new(AtomicU64::new(0)),
            window_start: Arc::new(AtomicU64::new(0)),
            max_requests,
            window_seconds,
            rate_limiting_enabled: true,
        }
    }

    /// Check if the request is allowed under rate limit
    /// Returns Ok(()) if allowed, Err(retry_after_seconds) if rate limited
    pub fn check_rate_limit(&self) -> Result<(), u64> {
        if !self.rate_limiting_enabled {
            return Ok(());
        }

        let now_secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        for _ in 0..8 {
            let window_start = self.window_start.load(Ordering::SeqCst);
            if window_start == 0 {
                if self
                    .window_start
                    .compare_exchange(0, now_secs, Ordering::SeqCst, Ordering::SeqCst)
                    .is_ok()
                {
                    return Ok(());
                }
                continue;
            }

            if now_secs - window_start < self.window_seconds {
                let current = self.window_counter.fetch_add(1, Ordering::SeqCst);
                if current as usize >= self.max_requests {
                    self.window_counter.fetch_sub(1, Ordering::SeqCst);
                    let retry_after = self.window_seconds - (now_secs - window_start);
                    return Err(retry_after);
                }
                return Ok(());
            }

            if self
                .window_start
                .compare_exchange(window_start, now_secs, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
            {
                self.window_counter.store(1, Ordering::SeqCst);
                return Ok(());
            }
        }

        // CAS 竞争 8 次仍未成功：保守拒绝而非放行（避免限流被绕过）
        Err(self.window_seconds)
    }

    #[allow(dead_code)]
    /// Get current usage statistics
    pub fn usage_stats(&self) -> (usize, u64, f64) {
        let now_secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let window_start = self.window_start.load(Ordering::SeqCst);
        let counter = self.window_counter.load(Ordering::SeqCst);

        let elapsed = now_secs.saturating_sub(window_start);
        let remaining = self.window_seconds.saturating_sub(elapsed);
        let usage_percent = (counter as f64 / self.max_requests as f64) * 100.0;

        (counter as usize, remaining, usage_percent)
    }
}

#[allow(dead_code)]
/// Global rate limiter instance (enabled by default)
pub(crate) static GLOBAL_RATE_LIMITER: OnceLock<InjectionRateLimiter> = OnceLock::new();

/// Global rate limiter for testing (disabled)
#[cfg(test)]
pub(crate) static TEST_RATE_LIMITER: OnceLock<InjectionRateLimiter> = OnceLock::new();

#[allow(dead_code)]
/// 全局默认配置注入器
pub(crate) static GLOBAL_INJECTOR: OnceLock<Arc<RwLock<ConfigInjector>>> = OnceLock::new();

/// 配置注入器
///
/// 负责管理和注入运行时配置值。
///
/// # 安全特性
///
/// - **安全验证**: 所有注入的值都会经过安全验证
/// - **敏感数据检测**: 自动检测敏感配置字段
/// - **类型转换**: 安全类型转换，防止注入攻击
///
/// # 使用示例
///
/// ```rust,ignore
/// use confers::security::ConfigInjector;
///
/// let injector = ConfigInjector::new();
///
/// // 注入配置
/// injector.inject("APP_PORT", "8080").unwrap();
/// injector.inject("APP_DEBUG", "true").unwrap();
///
/// // 获取配置
/// let port = injector.get("APP_PORT").unwrap();
/// assert_eq!(port, "8080");
/// ```
#[derive(Debug, Clone)]
pub struct ConfigInjector {
    /// 存储的配置值
    values: Arc<RwLock<HashMap<String, String>>>,
    /// 安全验证器
    validator: EnvSecurityValidator,
    /// 敏感字段模式
    sensitive_patterns: Vec<Regex>,
    /// 注入历史记录
    injection_history: Arc<RwLock<Vec<InjectionRecord>>>,
}

impl Default for ConfigInjector {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigInjector {
    /// 创建新的配置注入器
    pub fn new() -> Self {
        Self::with_validator(EnvSecurityValidator::default())
    }

    /// 使用自定义验证器创建注入器
    pub fn with_validator(validator: EnvSecurityValidator) -> Self {
        Self {
            values: Arc::new(RwLock::new(HashMap::new())),
            validator,
            sensitive_patterns: Self::default_sensitive_patterns(),
            injection_history: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// 默认敏感字段模式
    fn default_sensitive_patterns() -> Vec<regex::Regex> {
        SENSITIVE_DETECTION_PATTERNS.clone()
    }

    fn validate_injection(&self, name: &str, value: &str) -> Result<(), ConfigInjectionError> {
        debug_assert!(!name.is_empty());
        self.validator.validate_env_name(name, Some(value))?;
        self.validator.validate_env_value(value)?;
        Ok(())
    }

    /// 注入配置值
    ///
    /// # 参数
    ///
    /// * `name` - 配置名称
    /// * `value` - 配置值
    ///
    /// # 返回
    ///
    /// 成功返回 Ok(())，失败返回错误信息
    pub fn inject(&self, name: &str, value: &str) -> Result<(), ConfigInjectionError> {
        #[cfg(test)]
        if let Err(retry_after) = TEST_RATE_LIMITER
            .get_or_init(InjectionRateLimiter::disabled)
            .check_rate_limit()
        {
            return Err(ConfigInjectionError::RateLimited {
                retry_after_seconds: retry_after,
            });
        }
        #[cfg(not(test))]
        if let Err(retry_after) = GLOBAL_RATE_LIMITER
            .get_or_init(InjectionRateLimiter::new)
            .check_rate_limit()
        {
            return Err(ConfigInjectionError::RateLimited {
                retry_after_seconds: retry_after,
            });
        }

        // 验证配置名称
        self.validate_injection(name, value)?;

        // 检测敏感数据
        let is_sensitive = self.is_sensitive_field(name);

        // 存储配置值
        {
            let mut values = self
                .values
                .write()
                .map_err(|_| ConfigInjectionError::PoisonedLock)?;
            values.insert(name.to_string(), value.to_string());
        }

        // 记录注入历史
        {
            let mut history = self
                .injection_history
                .write()
                .map_err(|_| ConfigInjectionError::PoisonedLock)?;
            history.push(InjectionRecord {
                name: name.to_string(),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                is_sensitive,
            });
        }

        Ok(())
    }

    /// 批量注入配置
    ///
    /// # 参数
    ///
    /// * `config` - 配置映射
    ///
    /// # 返回
    ///
    /// 返回成功和失败的注入记录
    #[allow(clippy::type_complexity)]
    pub fn inject_all(
        &self,
        config: &HashMap<String, String>,
    ) -> Result<(Vec<String>, Vec<(String, String)>), ConfigInjectionError> {
        let mut success = Vec::new();
        let mut failures = Vec::new();

        // Collect all valid injections first
        let mut valid_injections: Vec<(String, String, bool)> = Vec::new();

        for (name, value) in config {
            // Validate first without holding any locks
            match self.validate_injection(name, value) {
                Ok(()) => {
                    let is_sensitive = self.is_sensitive_field(name);
                    valid_injections.push((name.clone(), value.clone(), is_sensitive));
                }
                Err(e) => failures.push((name.clone(), e.to_string())),
            }
        }

        // Single write lock for all valid injections
        if !valid_injections.is_empty() {
            let mut values = self
                .values
                .write()
                .map_err(|_| ConfigInjectionError::PoisonedLock)?;

            let mut history = self
                .injection_history
                .write()
                .map_err(|_| ConfigInjectionError::PoisonedLock)?;

            for (name, value, is_sensitive) in valid_injections {
                values.insert(name.clone(), value);
                history.push(InjectionRecord {
                    name: name.clone(),
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                    is_sensitive,
                });
                success.push(name);
            }
        }

        Ok((success, failures))
    }

    /// 获取配置值
    ///
    /// # 参数
    ///
    /// * `name` - 配置名称
    ///
    /// # 返回
    ///
    /// 如果存在返回 Some(value)，否则返回 None
    pub fn get(&self, name: &str) -> Option<String> {
        let values = self.values.read().ok()?;
        values.get(name).cloned()
    }

    /// 获取配置值（安全版本）
    ///
    /// 敏感数据会自动掩码
    pub fn get_safe(&self, name: &str) -> Option<String> {
        let value = self.get(name)?;
        if self.is_sensitive_field(name) {
            Some(Self::mask_value(&value))
        } else {
            Some(value)
        }
    }

    /// 获取所有配置
    pub fn get_all(&self) -> Result<HashMap<String, String>, ConfigInjectionError> {
        let values = self
            .values
            .read()
            .map_err(|_| ConfigInjectionError::PoisonedLock)?;
        // Collect into a new HashMap while holding the read lock
        Ok(values.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
    }

    /// 获取所有配置（安全版本）
    pub fn get_all_safe(&self) -> Result<HashMap<String, String>, ConfigInjectionError> {
        let values = self
            .values
            .read()
            .map_err(|_| ConfigInjectionError::PoisonedLock)?;
        Ok(values
            .iter()
            .map(|(k, v)| {
                let safe_value = if self.is_sensitive_field(k) {
                    Self::mask_value(v)
                } else {
                    v.clone()
                };
                (k.clone(), safe_value)
            })
            .collect())
    }

    /// 检查配置是否存在
    pub fn contains(&self, name: &str) -> Result<bool, ConfigInjectionError> {
        let values = self
            .values
            .read()
            .map_err(|_| ConfigInjectionError::PoisonedLock)?;
        Ok(values.contains_key(name))
    }

    /// 删除配置
    pub fn remove(&self, name: &str) -> Result<Option<String>, ConfigInjectionError> {
        let mut values = self
            .values
            .write()
            .map_err(|_| ConfigInjectionError::PoisonedLock)?;
        Ok(values.remove(name))
    }

    /// 清空所有配置
    pub fn clear(&self) -> Result<(), ConfigInjectionError> {
        let mut values = self
            .values
            .write()
            .map_err(|_| ConfigInjectionError::PoisonedLock)?;
        values.clear();

        let mut history = self
            .injection_history
            .write()
            .map_err(|_| ConfigInjectionError::PoisonedLock)?;
        history.clear();

        Ok(())
    }

    #[allow(dead_code)]
    /// 获取注入历史
    pub(crate) fn get_injection_history(
        &self,
    ) -> Result<Vec<InjectionRecord>, ConfigInjectionError> {
        let history = self
            .injection_history
            .read()
            .map_err(|_| ConfigInjectionError::PoisonedLock)?;
        Ok(history.clone())
    }

    /// 检查是否为敏感字段
    fn is_sensitive_field(&self, name: &str) -> bool {
        let name_lower = name.to_lowercase();
        self.sensitive_patterns
            .iter()
            .any(|pattern| pattern.is_match(&name_lower))
    }

    /// 掩码敏感值
    fn mask_value(value: &str) -> String {
        // 按字符计数避免多字节 UTF-8 切片 panic
        let char_count = value.chars().count();
        if char_count <= 4 {
            "*".repeat(char_count)
        } else {
            let visible = std::cmp::min(2, char_count / 4);
            let prefix: String = value.chars().take(visible).collect();
            format!("{}{}", prefix, "*".repeat(char_count - visible))
        }
    }

    /// 获取配置数量
    pub fn len(&self) -> Result<usize, ConfigInjectionError> {
        let values = self
            .values
            .read()
            .map_err(|_| ConfigInjectionError::PoisonedLock)?;
        Ok(values.len())
    }

    /// 检查是否为空
    pub fn is_empty(&self) -> Result<bool, ConfigInjectionError> {
        Ok(self.len()? == 0)
    }

    /// 获取安全验证器引用
    pub fn validator(&self) -> &EnvSecurityValidator {
        &self.validator
    }
}

/// 注入历史记录
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub(crate) struct InjectionRecord {
    /// 配置名称
    pub name: String,
    /// 注入时间戳
    pub timestamp: u64,
    /// 是否为敏感配置
    pub is_sensitive: bool,
}

/// 配置注入错误
#[derive(Debug, Clone, PartialEq)]
pub enum ConfigInjectionError {
    /// 安全验证失败
    SecurityValidation(EnvSecurityError),
    /// 锁中毒
    PoisonedLock,
    /// 无效的配置名称
    InvalidName(String),
    /// 无效的配置值
    InvalidValue(String),
    /// 速率限制
    RateLimited { retry_after_seconds: u64 },
}

impl std::fmt::Display for ConfigInjectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigInjectionError::SecurityValidation(e) => {
                write!(f, "Security validation failed: {}", e)
            }
            ConfigInjectionError::PoisonedLock => {
                write!(f, "Configuration lock poisoned")
            }
            ConfigInjectionError::InvalidName(name) => {
                write!(f, "Invalid configuration name: {}", name)
            }
            ConfigInjectionError::InvalidValue(value) => {
                write!(f, "Invalid configuration value: {}", value)
            }
            ConfigInjectionError::RateLimited {
                retry_after_seconds,
            } => {
                write!(
                    f,
                    "Rate limited. Retry after {} seconds",
                    retry_after_seconds
                )
            }
        }
    }
}

impl std::error::Error for ConfigInjectionError {}

impl From<EnvSecurityError> for ConfigInjectionError {
    fn from(e: EnvSecurityError) -> Self {
        ConfigInjectionError::SecurityValidation(e)
    }
}

/// 环境配置包装器
///
/// 提供类型安全的配置访问接口。
///
/// # 使用示例
///
/// ```rust,ignore
/// use confers::security::{EnvironmentConfig, ConfigInjector};
///
/// let injector = ConfigInjector::new();
/// if let Err(e) = injector.inject("APP_PORT", "8080") {
///     eprintln!("注入失败: {:?}", e);
/// }
/// if let Err(e) = injector.inject("APP_DEBUG", "true") {
///     eprintln!("注入失败: {:?}", e);
/// }
///
/// let config = EnvironmentConfig::from_injector(&injector);
///
/// // 获取配置值
/// let port: u16 = config.get("APP_PORT", 8080);
/// let debug: bool = config.get("APP_DEBUG", false);
/// ```
#[derive(Debug, Clone)]
pub struct EnvironmentConfig<'a> {
    /// 关联的注入器
    injector: &'a ConfigInjector,
}

impl<'a> EnvironmentConfig<'a> {
    /// 从注入器创建配置
    pub fn from_injector(injector: &'a ConfigInjector) -> Self {
        Self { injector }
    }

    /// 获取配置值并转换为指定类型
    ///
    /// # 类型参数
    ///
    /// * `T` - 目标类型
    ///
    /// # 参数
    ///
    /// * `name` - 配置名称
    /// * `default` - 默认值
    ///
    /// # 返回
    ///
    /// 转换后的值
    pub fn get<T>(&self, name: &str, default: T) -> T
    where
        T: std::str::FromStr,
        <T as std::str::FromStr>::Err: std::fmt::Display,
    {
        let value = self.injector.get(name);
        match value {
            Some(v) => v.parse().ok().unwrap_or(default),
            None => default,
        }
    }

    /// 获取必填配置值
    ///
    /// # 返回
    ///
    /// 如果配置不存在或转换失败，返回错误
    pub fn get_required<T>(&self, name: &str) -> Result<T, ConfigInjectionError>
    where
        T: std::str::FromStr,
        <T as std::str::FromStr>::Err: std::fmt::Display,
    {
        let value = self
            .injector
            .get(name)
            .ok_or_else(|| ConfigInjectionError::InvalidName(name.to_string()))?;

        value
            .parse()
            .map_err(|e| ConfigInjectionError::InvalidValue(format!("{}: {}", name, e)))
    }

    /// 获取字符串配置
    pub fn get_string(&self, name: &str, default: &str) -> String {
        self.injector
            .get(name)
            .unwrap_or_else(|| default.to_string())
    }

    /// 获取布尔配置
    pub fn get_bool(&self, name: &str, default: bool) -> bool {
        self.get::<bool>(name, default)
    }

    /// 获取数值配置
    pub fn get_number<T>(&self, name: &str, default: T) -> T
    where
        T: std::str::FromStr + std::clone::Clone,
        <T as std::str::FromStr>::Err: std::fmt::Display,
    {
        self.get::<T>(name, default)
    }
}

/// 配置宏辅助函数
pub(crate) mod macros {
    /// 安全注入配置宏
    ///
    /// # 使用示例
    ///
    /// ```rust,ignore
    /// use confers::security::ConfigInjector;
    /// use confers::safe_inject;
    ///
    /// let injector = ConfigInjector::new();
    /// safe_inject!(injector, {
    ///     "APP_NAME" => "my-app",
    ///     "APP_PORT" => "8080"
    /// });
    /// ```
    #[macro_export]
    macro_rules! safe_inject {
        ($injector:expr, { $($name:expr => $value:expr),+ }) => {
            vec![
                $(
                    $injector.inject($name, $value).map_err(|e| ($name, e))
                ),+
            ]
            .into_iter()
            .filter_map(|r| r.err())
            .collect::<Vec<_>>()
        };
    }

    /// 从环境变量安全注入配置
    ///
    /// # 使用示例
    ///
    /// ```rust,ignore
    /// use confers::security::ConfigInjector;
    /// use confers::inject_from_env;
    ///
    /// let injector = ConfigInjector::new();
    /// inject_from_env!(injector, "APP_", ["PORT", "HOST", "DEBUG"]);
    /// ```
    #[macro_export]
    macro_rules! inject_from_env {
        ($injector:expr, $prefix:expr, [$($name:expr),+]) => {
            vec![
                $(
                    if let Ok(value) = std::env::var(format!("{}{}", $prefix, $name)) {
                        $injector.inject(&format!("{}{}", $prefix, $name), &value).map_err(|e| (concat!($prefix, $name), e))
                    } else {
                        Ok(())
                    }
                ),+
            ]
            .into_iter()
            .filter_map(|r| r.err())
            .collect::<Vec<_>>()
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_injector_basic() {
        let injector = ConfigInjector::new();

        // 注入配置
        assert!(injector.inject("APP_PORT", "8080").is_ok());
        assert!(injector.inject("APP_DEBUG", "true").is_ok());

        // 获取配置
        assert_eq!(injector.get("APP_PORT"), Some("8080".to_string()));
        assert_eq!(injector.get("APP_DEBUG"), Some("true".to_string()));

        // 检查存在性
        assert!(injector.contains("APP_PORT").unwrap());
        assert!(!injector.contains("APP_NONEXISTENT").unwrap());
    }

    #[test]
    fn test_sensitive_field_detection() {
        let injector = ConfigInjector::new();

        // 敏感字段
        assert!(injector.is_sensitive_field("APP_SECRET"));
        assert!(injector.is_sensitive_field("API_TOKEN"));
        assert!(injector.is_sensitive_field("DATABASE_PASSWORD"));

        // 非敏感字段
        assert!(!injector.is_sensitive_field("APP_PORT"));
        assert!(!injector.is_sensitive_field("APP_HOST"));
        assert!(!injector.is_sensitive_field("DEBUG_MODE"));
    }

    #[test]
    fn test_safe_retrieval() {
        let validator = EnvSecurityValidator::lenient();
        let injector = ConfigInjector::with_validator(validator);
        injector.inject("APP_SECRET", "my-secret-value").unwrap();
        injector.inject("APP_PORT", "8080").unwrap();

        // 敏感字段应该被掩码
        let secret = injector.get_safe("APP_SECRET").unwrap();
        assert!(secret.contains('*'));
        assert_ne!(secret, "my-secret-value");

        // 非敏感字段应该原样返回
        let port = injector.get_safe("APP_PORT").unwrap();
        assert_eq!(port, "8080");
    }

    #[test]
    fn test_batch_injection() {
        let validator = EnvSecurityValidator::lenient();
        let injector = ConfigInjector::with_validator(validator);

        let mut config = HashMap::new();
        config.insert("APP_PORT".to_string(), "8080".to_string());
        config.insert("APP_HOST".to_string(), "localhost".to_string());
        config.insert("APP_SECRET".to_string(), "secret".to_string());

        let (success, failures) = injector.inject_all(&config).unwrap();

        assert_eq!(success.len(), 3);
        assert!(failures.is_empty());
    }

    #[test]
    fn test_injection_history() {
        let validator = EnvSecurityValidator::lenient();
        let injector = ConfigInjector::with_validator(validator);
        injector.inject("APP_PORT", "8080").unwrap();
        injector.inject("APP_SECRET", "secret").unwrap();

        let history = injector.get_injection_history().unwrap();
        assert_eq!(history.len(), 2);
        assert!(!history[0].is_sensitive);
        assert!(history[1].is_sensitive);
    }

    #[test]
    fn test_environment_config() {
        let injector = ConfigInjector::new();
        injector.inject("APP_PORT", "8080").unwrap();
        injector.inject("APP_DEBUG", "true").unwrap();
        injector.inject("APP_NAME", "test-app").unwrap();

        let config = EnvironmentConfig::from_injector(&injector);

        assert_eq!(config.get::<u16>("APP_PORT", 8080), 8080);
        assert!(config.get::<bool>("APP_DEBUG", false));
        assert_eq!(config.get_string("APP_NAME", "default"), "test-app");
    }

    #[test]
    fn test_clear_and_remove() {
        let injector = ConfigInjector::new();
        injector.inject("APP_PORT", "8080").unwrap();
        injector
            .inject("APP_CONFIG_VALUE", "secret-key-value")
            .unwrap();

        assert_eq!(injector.len().unwrap(), 2);

        // 移除一个配置
        let removed = injector.remove("APP_PORT").unwrap();
        assert_eq!(removed, Some("8080".to_string()));
        assert_eq!(injector.len().unwrap(), 1);

        // 清空所有配置
        injector.clear().unwrap();
        assert!(injector.is_empty().unwrap());
    }

    #[test]
    fn test_validation_failure() {
        let injector = ConfigInjector::new();

        // 尝试注入无效配置名称
        assert!(injector.inject("path", "value").is_err());
        assert!(injector.inject("HOME", "value").is_err());

        // 尝试注入危险值
        assert!(injector.inject("APP_TEST", "hello;world").is_err());
        assert!(injector.inject("APP_TEST", "hello${world}").is_err());
    }

    #[test]
    fn test_inject_blocked_and_invalid_name_errors() {
        let injector = ConfigInjector::new();
        // 阻止的环境变量名
        let err = injector.inject("PATH", "value").unwrap_err();
        assert!(matches!(
            err,
            ConfigInjectionError::SecurityValidation(EnvSecurityError::BlockedName { .. })
        ));
        let err = injector.inject("SHELL", "value").unwrap_err();
        assert!(matches!(
            err,
            ConfigInjectionError::SecurityValidation(EnvSecurityError::BlockedName { .. })
        ));
        // 格式不合法（小写）
        let err = injector.inject("lowercase", "value").unwrap_err();
        assert!(matches!(
            err,
            ConfigInjectionError::SecurityValidation(EnvSecurityError::InvalidNameFormat { .. })
        ));
        // 含危险字符的名称
        let err = injector.inject("APP;NAME", "value").unwrap_err();
        assert!(err.to_string().contains("blocked") || err.to_string().contains("Invalid"));
    }

    #[test]
    fn test_inject_value_dangerous_patterns() {
        let injector = ConfigInjector::new();
        // 命令注入：危险模式 ;
        let err = injector.inject("APP_TEST", "hello;world").unwrap_err();
        assert!(matches!(
            err,
            ConfigInjectionError::SecurityValidation(EnvSecurityError::CommandInjection { .. })
        ));
        // shell 展开 ${...}
        let err = injector.inject("APP_TEST", "hello${world}").unwrap_err();
        assert!(matches!(
            err,
            ConfigInjectionError::SecurityValidation(EnvSecurityError::ShellExpansion)
        ));
        // 控制字符（含 null 与其它控制字符均归为 control_character）
        let err = injector.inject("APP_TEST", "abc\x01def").unwrap_err();
        assert!(matches!(
            err,
            ConfigInjectionError::SecurityValidation(EnvSecurityError::CommandInjection { .. })
        ));
        // 管道符
        let err = injector.inject("APP_TEST", "a|b").unwrap_err();
        assert!(err.to_string().contains("dangerous") || err.to_string().contains("Security"));
    }

    #[test]
    fn test_inject_length_errors() {
        let injector = ConfigInjector::new();
        // 名称过长（默认上限 256）
        let long_name = "A".repeat(300);
        let err = injector.inject(&long_name, "value").unwrap_err();
        assert!(matches!(
            err,
            ConfigInjectionError::SecurityValidation(EnvSecurityError::NameTooLong { .. })
        ));
        // 值过长（默认上限 4096）
        let long_value = "a".repeat(5000);
        let err = injector.inject("APP_TEST", &long_value).unwrap_err();
        assert!(matches!(
            err,
            ConfigInjectionError::SecurityValidation(EnvSecurityError::ValueTooLong { .. })
        ));
    }

    #[test]
    fn test_sensitive_field_masking() {
        // 使用宽松验证器以注入敏感命名的字段
        let injector = ConfigInjector::with_validator(EnvSecurityValidator::lenient());
        injector.inject("APP_SECRET", "super-secret-value").unwrap(); // pragma: allowlist secret
        injector.inject("API_TOKEN", "token-1234567890").unwrap(); // pragma: allowlist secret
        injector.inject("APP_PORT", "8080").unwrap();

        // 敏感字段：get_safe 掩码
        let secret = injector.get_safe("APP_SECRET").unwrap();
        assert!(secret.contains('*'));
        assert_ne!(secret, "super-secret-value");
        let token = injector.get_safe("API_TOKEN").unwrap();
        assert!(token.contains('*'));
        assert_ne!(token, "token-1234567890");
        // 非敏感字段：原样返回
        assert_eq!(injector.get_safe("APP_PORT").unwrap(), "8080");

        // get_all_safe：敏感字段掩码、非敏感原样
        let all_safe = injector.get_all_safe().unwrap();
        assert!(all_safe.get("APP_SECRET").unwrap().contains('*'));
        assert!(all_safe.get("API_TOKEN").unwrap().contains('*'));
        assert_eq!(all_safe.get("APP_PORT").unwrap(), "8080");

        // get_all：原样返回（不掩码）
        let all = injector.get_all().unwrap();
        assert_eq!(all.get("APP_SECRET").unwrap(), "super-secret-value"); // pragma: allowlist secret
    }

    #[test]
    fn test_mask_value_short_and_long() {
        // 通过 get_safe 间接测试 mask_value：短值（<=4）全部掩码
        let injector = ConfigInjector::with_validator(EnvSecurityValidator::lenient());
        injector.inject("APP_SECRET", "ab").unwrap(); // pragma: allowlist secret
        let masked = injector.get_safe("APP_SECRET").unwrap();
        assert_eq!(masked, "**");
        // 5 字符值：保留前 1 位（min(2, 5/4=1)），其余掩码
        injector.inject("APP_KEY", "abcde").unwrap(); // pragma: allowlist secret
        let masked = injector.get_safe("APP_KEY").unwrap();
        assert_eq!(masked, "a****");
    }

    #[test]
    fn test_contains_remove_nonexistent() {
        let injector = ConfigInjector::new();
        injector.inject("APP_PORT", "8080").unwrap();
        // contains false
        assert!(!injector.contains("MISSING").unwrap());
        // remove 不存在 -> Ok(None)
        let removed = injector.remove("MISSING").unwrap();
        assert_eq!(removed, None);
        // remove 存在 -> Ok(Some)
        let removed = injector.remove("APP_PORT").unwrap();
        assert_eq!(removed, Some("8080".to_string()));
        assert!(!injector.contains("APP_PORT").unwrap());
    }

    #[test]
    fn test_len_is_empty_and_validator_ref() {
        let injector = ConfigInjector::new();
        assert!(injector.is_empty().unwrap());
        assert_eq!(injector.len().unwrap(), 0);
        injector.inject("APP_PORT", "8080").unwrap();
        injector.inject("APP_HOST", "localhost").unwrap();
        assert_eq!(injector.len().unwrap(), 2);
        assert!(!injector.is_empty().unwrap());
        // validator() 返回引用
        assert!(injector.validator().should_allow_env_var("APP_OK"));
    }

    #[test]
    fn test_inject_all_mixed_success_and_failures() {
        let injector = ConfigInjector::new();
        let mut config = HashMap::new();
        config.insert("APP_PORT".to_string(), "8080".to_string());
        // 无效名称
        config.insert("lowercase".to_string(), "v".to_string());
        // 无效值
        config.insert("APP_TEST".to_string(), "a;b".to_string());

        let (success, failures) = injector.inject_all(&config).unwrap();
        assert_eq!(success.len(), 1);
        assert!(success.contains(&"APP_PORT".to_string()));
        assert_eq!(failures.len(), 2);
        let failed_names: Vec<&str> = failures.iter().map(|(n, _)| n.as_str()).collect();
        assert!(failed_names.contains(&"lowercase"));
        assert!(failed_names.contains(&"APP_TEST"));
        // 失败记录包含错误描述
        for (_, msg) in &failures {
            assert!(!msg.is_empty());
        }
    }

    #[test]
    fn test_inject_history_after_operations() {
        let injector = ConfigInjector::with_validator(EnvSecurityValidator::lenient());
        injector.inject("APP_PORT", "8080").unwrap();
        injector.inject("APP_SECRET", "secret").unwrap(); // pragma: allowlist secret
        let history = injector.get_injection_history().unwrap();
        assert_eq!(history.len(), 2);
        assert!(!history[0].is_sensitive);
        assert!(history[1].is_sensitive);
        // clear 后历史也被清空
        injector.clear().unwrap();
        assert!(injector.get_injection_history().unwrap().is_empty());
        assert!(injector.is_empty().unwrap());
    }

    #[test]
    fn test_environment_config_get_required() {
        let injector = ConfigInjector::new();
        injector.inject("APP_PORT", "8080").unwrap();
        injector.inject("APP_BAD", "notanumber").unwrap();

        let config = EnvironmentConfig::from_injector(&injector);
        // 必填：成功解析
        assert_eq!(config.get_required::<u16>("APP_PORT").unwrap(), 8080);
        // 必填：缺失 -> InvalidName
        let err = config.get_required::<u16>("MISSING").unwrap_err();
        assert!(matches!(err, ConfigInjectionError::InvalidName(_)));
        // 必填：解析失败 -> InvalidValue
        let err = config.get_required::<u16>("APP_BAD").unwrap_err();
        assert!(matches!(err, ConfigInjectionError::InvalidValue(_)));
    }

    #[test]
    fn test_environment_config_get_with_default() {
        let injector = ConfigInjector::new();
        injector.inject("APP_PORT", "8080").unwrap();
        injector.inject("APP_DEBUG", "true").unwrap();
        injector.inject("APP_BAD", "notanumber").unwrap();

        let config = EnvironmentConfig::from_injector(&injector);
        // 命中且解析成功
        assert_eq!(config.get::<u16>("APP_PORT", 99), 8080);
        // 命中但解析失败 -> 默认
        assert_eq!(config.get::<u16>("APP_BAD", 99), 99);
        // 缺失 -> 默认
        assert_eq!(config.get::<u16>("MISSING", 99), 99);
        // 字符串
        assert_eq!(config.get_string("APP_PORT", "default"), "8080");
        assert_eq!(config.get_string("MISSING", "default"), "default");
        // 布尔
        assert!(config.get_bool("APP_DEBUG", false));
        assert!(!config.get_bool("MISSING", false));
        // 数值
        assert_eq!(config.get_number::<u32>("APP_PORT", 0), 8080);
        assert_eq!(config.get_number::<u32>("MISSING", 7), 7);
    }

    #[test]
    fn test_config_injection_error_display_all_variants() {
        let s = format!(
            "{}",
            ConfigInjectionError::SecurityValidation(EnvSecurityError::NullByte)
        );
        assert!(s.contains("Security validation failed"));

        assert_eq!(
            format!("{}", ConfigInjectionError::PoisonedLock),
            "Configuration lock poisoned"
        );
        assert_eq!(
            format!("{}", ConfigInjectionError::InvalidName("nm".to_string())),
            "Invalid configuration name: nm"
        );
        assert_eq!(
            format!("{}", ConfigInjectionError::InvalidValue("vl".to_string())),
            "Invalid configuration value: vl"
        );
        assert_eq!(
            format!(
                "{}",
                ConfigInjectionError::RateLimited {
                    retry_after_seconds: 30
                }
            ),
            "Rate limited. Retry after 30 seconds"
        );
    }

    #[test]
    fn test_config_injection_error_from_env_security() {
        let env_err = EnvSecurityError::NullByte;
        let inj_err: ConfigInjectionError = env_err.into();
        assert!(matches!(
            inj_err,
            ConfigInjectionError::SecurityValidation(EnvSecurityError::NullByte)
        ));
    }

    #[test]
    fn test_config_injector_default_and_with_validator() {
        // default 等价于 new
        let injector = ConfigInjector::default();
        assert!(injector.inject("APP_PORT", "8080").is_ok());
        // 自定义验证器
        let strict_injector = ConfigInjector::with_validator(EnvSecurityValidator::strict());
        assert!(strict_injector.inject("APP_PORT", "8080").is_ok());
    }

    #[test]
    fn test_rate_limiter_disabled_always_ok() {
        let limiter = InjectionRateLimiter::disabled();
        // 禁用限流：多次调用均放行
        for _ in 0..10 {
            assert!(limiter.check_rate_limit().is_ok());
        }
    }

    #[test]
    fn test_rate_limiter_enforces_limit() {
        // max_requests=1：Call1 初始化窗口，Call2 计入 1 次请求（放行），Call3 超限被拒
        let limiter = InjectionRateLimiter::with_limits(1, 3600);
        assert!(limiter.check_rate_limit().is_ok());
        assert!(limiter.check_rate_limit().is_ok());
        assert!(limiter.check_rate_limit().is_err());
    }

    #[test]
    fn test_rate_limiter_usage_stats() {
        // 全新限流器：window_start=0 -> remaining=0
        let limiter = InjectionRateLimiter::with_limits(100, 60);
        let (counter, remaining, usage) = limiter.usage_stats();
        assert_eq!(counter, 0);
        assert_eq!(remaining, 0);
        assert_eq!(usage, 0.0);

        // 初始化窗口后：remaining 接近窗口长度，计数仍为 0（首调用不计数）
        assert!(limiter.check_rate_limit().is_ok());
        let (counter, remaining, usage) = limiter.usage_stats();
        assert_eq!(counter, 0);
        assert_eq!(usage, 0.0);
        assert!(remaining > 0 && remaining <= 60);
    }
}
