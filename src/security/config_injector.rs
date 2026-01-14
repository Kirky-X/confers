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
//! ```rust
//! use confers::security::{ConfigInjector, EnvironmentConfig};
//!
//! // 创建配置注入器
//! let injector = ConfigInjector::new();
//!
//! // 注入配置值
//! injector.inject("APP_SECRET", "my-secret-value");
//!
//! // 获取配置
//! let value = injector.get("APP_SECRET").unwrap();
//! ```

use crate::security::{EnvSecurityError, EnvSecurityValidator, EnvironmentValidationConfig};
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// 全局默认配置注入器
pub static GLOBAL_INJECTOR: Lazy<Arc<RwLock<ConfigInjector>>> =
    Lazy::new(|| Arc::new(RwLock::new(ConfigInjector::new())));

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
/// ```rust
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
        Self::with_validator(EnvSecurityValidator::new())
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
    fn default_sensitive_patterns() -> Vec<Regex> {
        vec![
            Regex::new(r"(?i)(secret|password|token|key|auth|credential)").unwrap(),
            Regex::new(r"(?i)(api_key|access_token|refresh_token)").unwrap(),
            Regex::new(r"(?i)(private_key|public_key)").unwrap(),
            Regex::new(r"(?i)(database_url|connection_string)").unwrap(),
        ]
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
        // 验证配置名称
        self.validator.validate_env_name(name, Some(value))?;

        // 验证配置值
        self.validator.validate_env_value(value)?;

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
    pub fn inject_all(
        &self,
        config: &HashMap<String, String>,
    ) -> (Vec<String>, Vec<(String, String)>) {
        let mut success = Vec::new();
        let mut failures = Vec::new();

        for (name, value) in config {
            match self.inject(name, value) {
                Ok(_) => success.push(name.clone()),
                Err(e) => failures.push((name.clone(), e.to_string())),
            }
        }

        (success, failures)
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
    pub fn get_all(&self) -> HashMap<String, String> {
        let values = self.values.read().unwrap();
        values.clone()
    }

    /// 获取所有配置（安全版本）
    pub fn get_all_safe(&self) -> HashMap<String, String> {
        let values = self.values.read().unwrap();
        values
            .iter()
            .map(|(k, v)| {
                let safe_value = if self.is_sensitive_field(k) {
                    Self::mask_value(v)
                } else {
                    v.clone()
                };
                (k.clone(), safe_value)
            })
            .collect()
    }

    /// 检查配置是否存在
    pub fn contains(&self, name: &str) -> bool {
        let values = self.values.read().unwrap();
        values.contains_key(name)
    }

    /// 删除配置
    pub fn remove(&self, name: &str) -> Option<String> {
        let mut values = self.values.write().unwrap();
        values.remove(name)
    }

    /// 清空所有配置
    pub fn clear(&self) {
        let mut values = self.values.write().unwrap();
        values.clear();

        let mut history = self.injection_history.write().unwrap();
        history.clear();
    }

    /// 获取注入历史
    pub fn get_injection_history(&self) -> Vec<InjectionRecord> {
        let history = self.injection_history.read().unwrap();
        history.clone()
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
        if value.len() <= 4 {
            "*".repeat(value.len())
        } else {
            let visible = std::cmp::min(2, value.len() / 4);
            format!("{}{}", &value[..visible], "*".repeat(value.len() - visible))
        }
    }

    /// 获取配置数量
    pub fn len(&self) -> usize {
        let values = self.values.read().unwrap();
        values.len()
    }

    /// 检查是否为空
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// 获取安全验证器引用
    pub fn validator(&self) -> &EnvSecurityValidator {
        &self.validator
    }
}

/// 注入历史记录
#[derive(Debug, Clone)]
pub struct InjectionRecord {
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
/// ```rust
/// use confers::security::{EnvironmentConfig, ConfigInjector};
///
/// let injector = ConfigInjector::new();
/// injector.inject("APP_PORT", "8080").unwrap();
/// injector.inject("APP_DEBUG", "true").unwrap();
///
/// let config = EnvironmentConfig::from_injector(&injector);
///
/// // 获取配置值
/// let port: u16 = config.get("APP_PORT").unwrap_or(8080);
/// let debug: bool = config.get("APP_DEBUG").unwrap_or(false);
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
        let value = self.injector.get(name)?;
        value.parse().ok().unwrap_or(default)
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
pub mod macros {
    use super::*;

    /// 安全注入配置宏
    ///
    /// # 使用示例
    ///
    /// ```rust
    /// use confers::security::ConfigInjector;
    ///
    /// let injector = ConfigInjector::new();
    /// unsafe_inject!(injector, {
    ///     "APP_NAME" => "my-app",
    ///     "APP_PORT" => "8080",
    /// });
    /// ```
    #[macro_export]
    macro_rules! safe_inject {
        ($injector:expr, { $($name:expr => $value:expr),+ }) => {
            $(
                let _ = $injector.inject($name, $value);
            )+
        };
    }

    /// 从环境变量安全注入配置
    ///
    /// # 使用示例
    ///
    /// ```rust
    /// use confers::security::ConfigInjector;
    ///
    /// let injector = ConfigInjector::new();
    /// inject_from_env!(injector, "APP_", ["PORT", "HOST", "DEBUG"]);
    /// ```
    #[macro_export]
    macro_rules! inject_from_env {
        ($injector:expr, $prefix:expr, [$($name:expr),+]) => {
            $(
                if let Ok(value) = std::env::var(format!("{}{}", $prefix, $name)) {
                    let _ = $injector.inject(&format!("{}{}", $prefix, $name), &value);
                }
            )+
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
        assert!(injector.contains("APP_PORT"));
        assert!(!injector.contains("APP_NONEXISTENT"));
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
        let injector = ConfigInjector::new();
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
        let injector = ConfigInjector::new();

        let mut config = HashMap::new();
        config.insert("APP_PORT".to_string(), "8080".to_string());
        config.insert("APP_HOST".to_string(), "localhost".to_string());
        config.insert("APP_SECRET".to_string(), "secret".to_string());

        let (success, failures) = injector.inject_all(&config);

        assert_eq!(success.len(), 3);
        assert!(failures.is_empty());
    }

    #[test]
    fn test_injection_history() {
        let injector = ConfigInjector::new();
        injector.inject("APP_PORT", "8080").unwrap();
        injector.inject("APP_SECRET", "secret").unwrap();

        let history = injector.get_injection_history();
        assert_eq!(history.len(), 2);
        assert!(history[0].is_sensitive == false);
        assert!(history[1].is_sensitive == true);
    }

    #[test]
    fn test_environment_config() {
        let injector = ConfigInjector::new();
        injector.inject("APP_PORT", "8080").unwrap();
        injector.inject("APP_DEBUG", "true").unwrap();
        injector.inject("APP_NAME", "test-app").unwrap();

        let config = EnvironmentConfig::from_injector(&injector);

        assert_eq!(config.get::<u16>("APP_PORT", 8080), 8080);
        assert_eq!(config.get::<bool>("APP_DEBUG", false), true);
        assert_eq!(config.get_string("APP_NAME", "default"), "test-app");
    }

    #[test]
    fn test_clear_and_remove() {
        let injector = ConfigInjector::new();
        injector.inject("APP_PORT", "8080").unwrap();
        injector.inject("APP_SECRET", "secret").unwrap();

        assert_eq!(injector.len(), 2);

        // 移除一个配置
        let removed = injector.remove("APP_PORT");
        assert_eq!(removed, Some("8080".to_string()));
        assert_eq!(injector.len(), 1);

        // 清空所有配置
        injector.clear();
        assert!(injector.is_empty());
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
}
