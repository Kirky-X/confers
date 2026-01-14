// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

use regex::Regex;
use std::collections::HashMap;
use std::sync::OnceLock;

fn compile_pattern(pattern: &str) -> Regex {
    Regex::new(pattern).unwrap_or_else(|_| panic!("Failed to compile regex pattern: {}", pattern))
}

fn get_allowed_patterns() -> &'static Vec<Regex> {
    static ALLOWED_PATTERNS: OnceLock<Vec<Regex>> = OnceLock::new();
    ALLOWED_PATTERNS.get_or_init(|| {
        vec![
            compile_pattern(r"^[A-Z][A-Z0-9_]*$"),
            compile_pattern(r"^[A-Z][A-Z0-9_]*_[A-Z][A-Z0-9_]*$"),
            compile_pattern(r"^[A-Z][A-Z0-9_]*_[A-Z][A-Z0-9_]*_[A-Z][A-Z0-9_]*$"),
        ]
    })
}

fn get_blocked_patterns() -> &'static Vec<Regex> {
    static BLOCKED_PATTERNS: OnceLock<Vec<Regex>> = OnceLock::new();
    BLOCKED_PATTERNS.get_or_init(|| {
        vec![
            compile_pattern(r"(?i)^(PATH|LD_LIBRARY_PATH|LD_PRELOAD)$"),
            compile_pattern(r"(?i)^(SHELL|HOME|USER|LOGNAME)$"),
            compile_pattern(r"(?i)^(PWD|OLDPWD)$"),
            compile_pattern(r"(?i)^(MAIL|MAILCHECK)$"),
            compile_pattern(r"(?i)^(TERM|TERMCAP)$"),
            compile_pattern(r"(?i)^(DISPLAY|XAUTHORITY)$"),
            compile_pattern(r"(?i)^(SSH_AUTH_SOCK|SSH_AGENT_PID)$"),
            compile_pattern(r"(?i)^(DOCKER_HOST|KUBECONFIG)$"),
            compile_pattern(r"(?i).*(_SECRET|_PASSWORD|_TOKEN|_KEY|_PRIVATE)$"),
            compile_pattern(r".*[;<>&|`$].*"),
            compile_pattern(r"^BASH_FUNC_.*"),
        ]
    })
}

fn get_allowed_pattern_strings() -> &'static Vec<&'static str> {
    static ALLOWED_PATTERNS_STR: OnceLock<Vec<&'static str>> = OnceLock::new();
    ALLOWED_PATTERNS_STR.get_or_init(|| {
        vec![
            r"^[A-Z][A-Z0-9_]*$",
            r"^[A-Z][A-Z0-9_]*_[A-Z][A-Z0-9_]*$",
            r"^[A-Z][A-Z0-9_]*_[A-Z][A-Z0-9_]*_[A-Z][A-Z0-9_]*$",
        ]
    })
}

/// Configuration for environment variable validation
#[derive(Debug, Clone)]
pub struct EnvironmentValidationConfig {
    pub max_name_length: usize,
    pub max_value_length: usize,
    pub enable_blocked_patterns: bool,
    pub enable_length_validation: bool,
    pub allow_encrypted_values: bool,
    pub blocked_patterns: Vec<String>,
    pub allowed_patterns: Vec<String>,
}

impl Default for EnvironmentValidationConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl EnvironmentValidationConfig {
    pub fn new() -> Self {
        Self {
            max_name_length: 256,
            max_value_length: 4096,
            enable_blocked_patterns: true,
            enable_length_validation: true,
            allow_encrypted_values: true,
            blocked_patterns: Vec::new(),
            allowed_patterns: Vec::new(),
        }
    }

    pub fn with_max_name_length(mut self, length: usize) -> Self {
        self.max_name_length = length;
        self
    }

    pub fn with_max_value_length(mut self, length: usize) -> Self {
        self.max_value_length = length;
        self
    }

    pub fn with_blocked_patterns_disabled(mut self) -> Self {
        self.enable_blocked_patterns = false;
        self
    }

    pub fn with_length_validation_disabled(mut self) -> Self {
        self.enable_length_validation = false;
        self
    }

    pub fn with_custom_blocked_patterns(mut self, patterns: Vec<String>) -> Self {
        self.blocked_patterns = patterns;
        self
    }

    pub fn with_custom_allowed_patterns(mut self, patterns: Vec<String>) -> Self {
        self.allowed_patterns = patterns;
        self
    }

    pub fn max_name_length(&self) -> usize {
        self.max_name_length
    }

    pub fn max_value_length(&self) -> usize {
        self.max_value_length
    }
}

/// Security validation for environment variable mapping
#[derive(Debug, Clone)]
pub struct EnvSecurityValidator {
    /// Maximum length for environment variable names
    max_name_length: usize,
    /// Maximum length for environment variable values
    max_value_length: usize,
    /// Configuration for validation behavior
    config: EnvironmentValidationConfig,
}

impl Default for EnvSecurityValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl EnvSecurityValidator {
    /// Create a new security validator with default rules
    pub fn new() -> Self {
        Self::with_config(EnvironmentValidationConfig::new())
    }

    /// Create a security validator with custom configuration
    pub fn with_config(config: EnvironmentValidationConfig) -> Self {
        Self {
            max_name_length: config.max_name_length,
            max_value_length: config.max_value_length,
            config,
        }
    }

    /// Create a strict validator for production environments
    pub fn strict() -> Self {
        Self::with_config(
            EnvironmentValidationConfig::new()
                .with_max_name_length(128)
                .with_max_value_length(2048),
        )
    }

    /// Create a lenient validator for testing
    pub fn lenient() -> Self {
        Self::with_config(
            EnvironmentValidationConfig::new()
                .with_max_name_length(1024)
                .with_max_value_length(8192)
                .with_blocked_patterns_disabled()
                .with_length_validation_disabled(),
        )
    }

    /// Validate an environment variable name
    /// If value is provided and starts with "enc:", secret-related names are allowed
    pub fn validate_env_name(
        &self,
        name: &str,
        value: Option<&str>,
    ) -> Result<(), EnvSecurityError> {
        let blocked_patterns = get_blocked_patterns();
        let allowed_patterns = get_allowed_patterns();

        if self.config.enable_length_validation && name.len() > self.max_name_length {
            return Err(EnvSecurityError::NameTooLong {
                name: name.to_string(),
                max_length: self.max_name_length,
                actual_length: name.len(),
            });
        }

        if self.config.enable_blocked_patterns {
            for pattern in blocked_patterns {
                if pattern.is_match(name) {
                    if let Some(val) = value {
                        if self.config.allow_encrypted_values && val.starts_with("enc:") {
                            continue;
                        }
                    }
                    return Err(EnvSecurityError::BlockedName {
                        name: name.to_string(),
                        pattern: pattern.as_str().to_string(),
                    });
                }
            }
        }

        let mut matched = false;
        for pattern in allowed_patterns {
            if pattern.is_match(name) {
                matched = true;
                break;
            }
        }

        if !matched {
            return Err(EnvSecurityError::InvalidNameFormat {
                name: name.to_string(),
                expected_patterns: get_allowed_pattern_strings()
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
            });
        }

        Ok(())
    }

    /// Validate an environment variable name (backward compatible)
    pub fn validate_env_name_simple(&self, name: &str) -> Result<(), EnvSecurityError> {
        self.validate_env_name(name, None)
    }

    /// Validate an environment variable value
    pub fn validate_env_value(&self, value: &str) -> Result<(), EnvSecurityError> {
        if self.config.allow_encrypted_values && value.starts_with("enc:") {
            return Ok(());
        }

        if self.config.enable_length_validation && value.len() > self.max_value_length {
            return Err(EnvSecurityError::ValueTooLong {
                value_length: value.len(),
                max_length: self.max_value_length,
            });
        }

        if self.config.enable_blocked_patterns {
            // Check for control characters (excluding common whitespace)
            if value
                .chars()
                .any(|c| c.is_control() && c != '\n' && c != '\r' && c != '\t')
            {
                return Err(EnvSecurityError::CommandInjection {
                    pattern: "control_character".to_string(),
                });
            }

            if value.contains('\0') {
                return Err(EnvSecurityError::NullByte);
            }

            if value.contains("${") && value.contains('}') {
                return Err(EnvSecurityError::ShellExpansion);
            }

            // Extended list of dangerous patterns
            let dangerous_patterns = [
                ";", "&", "|", "`", "$", "(", ")", "<", ">", "\n", "\r", "\\",
                "\t", // Backslash and tab
                "\\n", "\\r", "\\t", // Escape sequences
                "; ", "& ", "| ", "$ ", // Patterns with space
            ];
            for pattern in &dangerous_patterns {
                if value.contains(pattern) {
                    return Err(EnvSecurityError::CommandInjection {
                        pattern: pattern.to_string(),
                    });
                }
            }
        }

        Ok(())
    }

    /// Validate a complete environment variable mapping
    pub fn validate_env_mapping(
        &self,
        mapping: &HashMap<String, String>,
    ) -> Result<(), EnvSecurityError> {
        for (field_name, env_name) in mapping {
            self.validate_env_name_simple(env_name)?;

            // Also validate that the field name is reasonable
            if field_name.is_empty() || field_name.contains(' ') {
                return Err(EnvSecurityError::InvalidFieldName {
                    field_name: field_name.to_string(),
                });
            }
        }
        Ok(())
    }

    /// Sanitize an environment variable value for logging
    pub fn sanitize_for_logging(&self, value: &str) -> String {
        if value.len() > 100 {
            format!("{}...", &value[..97])
        } else {
            value.to_string()
        }
    }

    /// Check if an environment variable should be allowed
    pub fn should_allow_env_var(&self, name: &str) -> bool {
        self.validate_env_name_simple(name).is_ok()
    }
}

/// Security validation errors
#[derive(Debug, Clone, PartialEq)]
pub enum EnvSecurityError {
    /// Environment variable name is too long
    NameTooLong {
        name: String,
        max_length: usize,
        actual_length: usize,
    },
    /// Environment variable name matches a blocked pattern
    BlockedName { name: String, pattern: String },
    /// Environment variable name doesn't match allowed patterns
    InvalidNameFormat {
        name: String,
        expected_patterns: Vec<String>,
    },
    /// Environment variable value is too long
    ValueTooLong {
        value_length: usize,
        max_length: usize,
    },
    /// Environment variable value contains null bytes
    NullByte,
    /// Environment variable value contains shell expansion
    ShellExpansion,
    /// Environment variable value contains command injection patterns
    CommandInjection { pattern: String },
    /// Invalid field name in mapping
    InvalidFieldName { field_name: String },
}

impl std::fmt::Display for EnvSecurityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EnvSecurityError::NameTooLong {
                name,
                max_length,
                actual_length,
            } => {
                write!(
                    f,
                    "Environment variable name '{}' is too long: {} > {}",
                    name, actual_length, max_length
                )
            }
            EnvSecurityError::BlockedName { name, pattern } => {
                write!(
                    f,
                    "Environment variable name '{}' matches blocked pattern: {}",
                    name, pattern
                )
            }
            EnvSecurityError::InvalidNameFormat {
                name,
                expected_patterns,
            } => {
                write!(
                    f,
                    "Environment variable name '{}' doesn't match any allowed pattern: {:?}",
                    name, expected_patterns
                )
            }
            EnvSecurityError::ValueTooLong {
                value_length,
                max_length,
            } => {
                write!(
                    f,
                    "Environment variable value is too long: {} > {}",
                    value_length, max_length
                )
            }
            EnvSecurityError::NullByte => {
                write!(f, "Environment variable value contains null bytes")
            }
            EnvSecurityError::ShellExpansion => {
                write!(f, "Environment variable value contains shell expansion")
            }
            EnvSecurityError::CommandInjection { pattern } => {
                write!(
                    f,
                    "Environment variable value contains dangerous pattern: '{}'",
                    pattern
                )
            }
            EnvSecurityError::InvalidFieldName { field_name } => {
                write!(
                    f,
                    "Invalid field name in environment mapping: '{}'",
                    field_name
                )
            }
        }
    }
}

impl std::error::Error for EnvSecurityError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_valid_env_name() {
        let validator = EnvSecurityValidator::new();
        assert!(validator.validate_env_name_simple("APP_PORT").is_ok());
        assert!(validator.validate_env_name_simple("DATABASE_HOST").is_ok());
        assert!(validator.validate_env_name_simple("REDIS_PORT").is_ok());
    }

    #[test]
    fn test_validate_blocked_env_name() {
        let validator = EnvSecurityValidator::new();
        assert!(validator.validate_env_name_simple("PATH").is_err());
        assert!(validator.validate_env_name_simple("HOME").is_err());
        assert!(validator.validate_env_name_simple("SECRET_KEY").is_err());
        assert!(validator.validate_env_name_simple("API_TOKEN").is_err());
    }

    #[test]
    fn test_validate_invalid_env_name_format() {
        let validator = EnvSecurityValidator::new();
        assert!(validator.validate_env_name_simple("app_port").is_err()); // lowercase
        assert!(validator.validate_env_name_simple("APP-PORT").is_err()); // dash
        assert!(validator.validate_env_name_simple("123PORT").is_err()); // starts with number
    }

    #[test]
    fn test_validate_env_name_length() {
        let validator = EnvSecurityValidator::new();

        // Valid: exactly 256 characters (max allowed)
        let valid_256 = "A".repeat(256);
        assert!(validator.validate_env_name_simple(&valid_256).is_ok());

        // Valid: less than 256 characters
        let valid_255 = "A".repeat(255);
        assert!(validator.validate_env_name_simple(&valid_255).is_ok());

        // Invalid: more than 256 characters
        let invalid_257 = "A".repeat(257);
        assert!(validator.validate_env_name_simple(&invalid_257).is_err());
    }

    #[test]
    fn test_validate_dangerous_env_value() {
        let validator = EnvSecurityValidator::new();
        assert!(validator.validate_env_value("hello").is_ok());
        assert!(validator.validate_env_value("test123").is_ok());

        assert!(validator.validate_env_value("hello;world").is_err()); // command injection
        assert!(validator.validate_env_value("hello|world").is_err()); // pipe
        assert!(validator.validate_env_value("hello${world}").is_err()); // shell expansion
        assert!(validator.validate_env_value("hello\0world").is_err()); // null byte
    }

    #[test]
    fn test_validate_env_mapping() {
        let validator = EnvSecurityValidator::new();
        let mut mapping = HashMap::new();
        mapping.insert("port".to_string(), "APP_PORT".to_string());
        mapping.insert("host".to_string(), "DATABASE_HOST".to_string());

        assert!(validator.validate_env_mapping(&mapping).is_ok());

        // Invalid field name
        let mut bad_mapping = HashMap::new();
        bad_mapping.insert("invalid field".to_string(), "APP_PORT".to_string());
        assert!(validator.validate_env_mapping(&bad_mapping).is_err());

        // Invalid env name
        let mut bad_env_mapping = HashMap::new();
        bad_env_mapping.insert("port".to_string(), "PATH".to_string());
        assert!(validator.validate_env_mapping(&bad_env_mapping).is_err());
    }

    #[test]
    fn test_custom_length_limits() {
        let config = EnvironmentValidationConfig::new()
            .with_max_name_length(100)
            .with_max_value_length(500);
        let validator = EnvSecurityValidator::with_config(config);

        let valid_100 = "A".repeat(100);
        assert!(validator.validate_env_name_simple(&valid_100).is_ok());

        let invalid_101 = "A".repeat(101);
        assert!(validator.validate_env_name_simple(&invalid_101).is_err());

        assert!(validator.validate_env_value(&"x".repeat(500)).is_ok());
        assert!(validator.validate_env_value(&"x".repeat(501)).is_err());
    }

    #[test]
    fn test_strict_validator() {
        let validator = EnvSecurityValidator::strict();

        let valid_128 = "A".repeat(128);
        assert!(validator.validate_env_name_simple(&valid_128).is_ok());

        let invalid_129 = "A".repeat(129);
        assert!(validator.validate_env_name_simple(&invalid_129).is_err());

        assert!(validator.validate_env_value(&"x".repeat(2048)).is_ok());
        assert!(validator.validate_env_value(&"x".repeat(2049)).is_err());
    }

    #[test]
    fn test_lenient_validator() {
        let validator = EnvSecurityValidator::lenient();

        let long_name = "A".repeat(500);
        assert!(validator.validate_env_name_simple(&long_name).is_ok());

        let long_value = "x".repeat(5000);
        assert!(validator.validate_env_value(&long_value).is_ok());

        assert!(validator.validate_env_name_simple("PATH").is_ok());
        assert!(validator.validate_env_value("hello;world").is_ok());
    }

    #[test]
    fn test_disabled_blocked_patterns() {
        let config = EnvironmentValidationConfig::new().with_blocked_patterns_disabled();
        let validator = EnvSecurityValidator::with_config(config);

        assert!(validator.validate_env_name_simple("PATH").is_ok());
        assert!(validator.validate_env_name_simple("HOME").is_ok());
        assert!(validator.validate_env_name_simple("SECRET_KEY").is_ok());
    }

    #[test]
    fn test_disabled_length_validation() {
        let config = EnvironmentValidationConfig::new().with_length_validation_disabled();
        let validator = EnvSecurityValidator::with_config(config);

        let very_long_name = "A".repeat(1000);
        assert!(validator.validate_env_name_simple(&very_long_name).is_ok());

        let very_long_value = "x".repeat(10000);
        assert!(validator.validate_env_value(&very_long_value).is_ok());
    }

    #[test]
    fn test_disabled_encrypted_value_skip() {
        let config = EnvironmentValidationConfig::new()
            .with_length_validation_disabled()
            .with_blocked_patterns_disabled()
            .with_custom_blocked_patterns(vec![r".*SECRET.*".to_string()]);
        let validator = EnvSecurityValidator::with_config(config);

        let encrypted_value = "enc:ABC123XYZ789";
        assert!(validator.validate_env_value(encrypted_value).is_ok());

        let secret_with_encrypted = "MY_SECRET";
        assert!(validator
            .validate_env_name(secret_with_encrypted, Some(encrypted_value))
            .is_ok());
    }

    #[test]
    fn test_global_config_functions() {
        let config = EnvironmentValidationConfig::new()
            .with_max_name_length(512)
            .with_max_value_length(8192);

        let validator = EnvSecurityValidator::with_config(config.clone());

        assert_eq!(config.max_name_length(), 512);
        assert_eq!(config.max_value_length(), 8192);

        let long_name = "A".repeat(512);
        assert!(validator.validate_env_name_simple(&long_name).is_ok());

        let invalid_513 = "A".repeat(513);
        assert!(validator.validate_env_name_simple(&invalid_513).is_err());
    }

    #[test]
    fn test_config_builder_pattern() {
        let config = EnvironmentValidationConfig::new()
            .with_max_name_length(64)
            .with_max_value_length(1024)
            .with_blocked_patterns_disabled()
            .with_length_validation_disabled();

        assert_eq!(config.max_name_length, 64);
        assert_eq!(config.max_value_length, 1024);
        assert!(!config.enable_blocked_patterns);
        assert!(!config.enable_length_validation);
    }
}

// 安全模块导出
#[cfg(feature = "encryption")]
pub mod secure_string;
#[cfg(feature = "encryption")]
pub use secure_string::{
    allocated_secure_strings, deallocated_secure_strings, reset_secure_string_counters,
    SecureString, SecureStringBuilder, SensitiveData, SensitivityLevel,
};

#[cfg(feature = "encryption")]
pub mod config_injector;
#[cfg(feature = "encryption")]
pub use config_injector::{
    ConfigInjectionError, ConfigInjector, EnvironmentConfig, InjectionRecord,
};

#[cfg(feature = "encryption")]
pub mod input_validation;
#[cfg(feature = "encryption")]
pub use input_validation::{
    ConfigValidationError as ConfigFieldValidationError, ConfigValidationResult, ConfigValidator,
    ConfigValidatorBuilder, InputValidationError, InputValidator, SensitiveDataDetector,
    SensitivityResult,
};

#[cfg(feature = "encryption")]
pub mod error_sanitization;
#[cfg(feature = "encryption")]
pub use error_sanitization::{
    Error as SanitizationError, ErrorSanitizer, FilterResult, LogLevel, SafeResult, SecureLogger,
    SensitiveDataFilter,
};

#[cfg(all(test, feature = "encryption"))]
mod security_tests;
