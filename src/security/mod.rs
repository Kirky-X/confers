// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

pub(crate) mod patterns;
mod prefix;
pub use prefix::EncryptionPrefix;

#[cfg(feature = "security-rules")]
pub mod rules;

use regex::Regex;
use std::collections::HashMap;
use std::sync::OnceLock;

/// Check if a value appears to be encrypted (has "enc:" prefix with content).
///
/// Used to identify values that should bypass normal validation rules,
/// as encrypted values contain safely-encoded content.
pub(crate) fn is_encrypted_value(value: &str) -> bool {
    EncryptionPrefix::Enc.is_prefixed(value) && value.len() > EncryptionPrefix::Enc.as_str().len()
}

/// Validate the format of an encrypted value.
///
/// Checks for proper "enc:" prefix and valid base64-encoded content.
/// Returns an error if the format is invalid.
pub(crate) fn validate_encrypted_format(value: &str) -> Result<(), EnvSecurityError> {
    if !EncryptionPrefix::Enc.is_prefixed(value) {
        return Err(EnvSecurityError::InvalidValueFormat {
            reason: "Missing 'enc:' prefix".to_string(),
        });
    }

    let encrypted_content = EncryptionPrefix::Enc.strip(value).unwrap_or("");
    if encrypted_content.is_empty() {
        return Err(EnvSecurityError::InvalidValueFormat {
            reason: "Empty encrypted content".to_string(),
        });
    }

    if !encrypted_content
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '=')
    {
        return Err(EnvSecurityError::InvalidValueFormat {
            reason: "Invalid characters in encrypted content (expected base64)".to_string(),
        });
    }

    Ok(())
}

/// Compile a regex pattern, returning an error if the pattern is invalid.
pub(crate) fn compile_pattern(pattern: &str) -> Result<Regex, EnvSecurityError> {
    Regex::new(pattern).map_err(|e| EnvSecurityError::InvalidRegex {
        pattern: pattern.to_string(),
        error: e.to_string(),
    })
}

/// Get the list of allowed patterns for environment variable names.
///
/// Returns a lazily-initialized static vector of compiled regex patterns.
/// Patterns define valid naming conventions (e.g., uppercase with underscores).
pub(crate) fn get_allowed_patterns() -> Result<&'static Vec<Regex>, EnvSecurityError> {
    static ALLOWED_PATTERNS: OnceLock<Result<Vec<Regex>, EnvSecurityError>> = OnceLock::new();
    match ALLOWED_PATTERNS.get_or_init(|| {
        vec![
            compile_pattern(r"^[A-Z][A-Z0-9_]*$"),
            compile_pattern(r"^[A-Z][A-Z0-9_]*_[A-Z][A-Z0-9_]*$"),
            compile_pattern(r"^[A-Z][A-Z0-9_]*_[A-Z][A-Z0-9_]*_[A-Z][A-Z0-9_]*$"),
        ]
        .into_iter()
        .collect()
    }) {
        Ok(patterns) => Ok(patterns),
        Err(e) => Err(e.clone()),
    }
}

/// Get the list of blocked patterns for environment variable names.
///
/// Returns a lazily-initialized static vector of compiled regex patterns.
/// Names matching these patterns are rejected (e.g., PATH, HOME, SECRET_*).
pub(crate) fn get_blocked_patterns() -> Result<&'static Vec<Regex>, EnvSecurityError> {
    static BLOCKED_PATTERNS: OnceLock<Result<Vec<Regex>, EnvSecurityError>> = OnceLock::new();
    match BLOCKED_PATTERNS.get_or_init(|| {
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
        .into_iter()
        .collect()
    }) {
        Ok(patterns) => Ok(patterns),
        Err(e) => Err(e.clone()),
    }
}

/// Get the string representations of allowed pattern rules.
///
/// Returns static references to the raw pattern strings for error messages
/// and documentation purposes.
pub(crate) fn get_allowed_pattern_strings() -> &'static Vec<&'static str> {
    static ALLOWED_PATTERNS_STR: OnceLock<Vec<&'static str>> = OnceLock::new();
    ALLOWED_PATTERNS_STR.get_or_init(|| {
        vec![
            r"^[A-Z][A-Z0-9_]*$",
            r"^[A-Z][A-Z0-9_]*_[A-Z][A-Z0-9_]*$",
            r"^[A-Z][A-Z0-9_]*_[A-Z][A-Z0-9_]*_[A-Z][A-Z0-9_]*$",
        ]
    })
}

/// Get the list of dangerous patterns that should be rejected in values.
///
/// Returns static references to patterns that indicate potential command
/// injection or shell expansion attempts (e.g., `;`, `$`, `|`).
pub(crate) fn get_dangerous_patterns() -> &'static Vec<&'static str> {
    static DANGEROUS_PATTERNS: OnceLock<Vec<&'static str>> = OnceLock::new();
    DANGEROUS_PATTERNS.get_or_init(|| {
        vec![
            ";", "&", "|", "`", "$", "(", ")", "<", ">", "\n", "\r", "\\", "\t", "\\n", "\\r",
            "\\t", "; ", "& ", "| ", "$ ",
        ]
    })
}

/// Configuration for environment variable validation
#[derive(Debug, Clone)]
pub struct EnvironmentValidationConfig {
    max_name_length: usize,
    max_value_length: usize,
    enable_blocked_patterns: bool,
    enable_length_validation: bool,
    allow_encrypted_values: bool,
    blocked_patterns: Vec<String>,
    allowed_patterns: Vec<String>,
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

    pub fn with_blocked_patterns_check(mut self, enabled: bool) -> Self {
        self.enable_blocked_patterns = enabled;
        self
    }

    pub fn with_length_validation(mut self, enabled: bool) -> Self {
        self.enable_length_validation = enabled;
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

    pub fn with_encrypted_values(mut self, allow: bool) -> Self {
        self.allow_encrypted_values = allow;
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
    /// Create a security validator with default configuration
    pub fn new() -> Self {
        Self::with_config(EnvironmentValidationConfig::default())
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

    /// Validate an environment variable name.
    ///
    /// If `value` is `Some` and starts with `"enc:"`, secret-related blocked
    /// patterns are skipped (encrypted values are trusted to carry sensitive
    /// names). This bypass is intentional: callers that enable
    /// `allow_encrypted_values` opt in to this behavior.
    pub fn validate_env_name(
        &self,
        name: &str,
        value: Option<&str>,
    ) -> Result<(), EnvSecurityError> {
        // Use custom patterns if configured, otherwise fall back to global patterns.
        // Custom patterns *replace* global patterns entirely — this is intentional,
        // allowing full customization. Callers who want to extend the global set
        // should retrieve it and append their own patterns.
        let custom_blocked: Vec<regex::Regex> = if !self.config.blocked_patterns.is_empty() {
            self.compile_custom_patterns(&self.config.blocked_patterns)?
        } else {
            Vec::new()
        };
        let custom_allowed: Vec<regex::Regex> = if !self.config.allowed_patterns.is_empty() {
            self.compile_custom_patterns(&self.config.allowed_patterns)?
        } else {
            Vec::new()
        };

        let blocked_patterns = if !custom_blocked.is_empty() {
            custom_blocked.iter().collect::<Vec<&regex::Regex>>()
        } else {
            let global = get_blocked_patterns()?;
            global.iter().collect()
        };
        let allowed_patterns = if !custom_allowed.is_empty() {
            custom_allowed.iter().collect::<Vec<&regex::Regex>>()
        } else {
            let global = get_allowed_patterns()?;
            global.iter().collect()
        };

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
                    if let Some(val) = value
                        && self.config.allow_encrypted_values
                        && val.starts_with("enc:")
                    {
                        continue;
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

    /// Check if value contains control characters (excluding common whitespace)
    fn has_control_characters(value: &str) -> bool {
        value
            .chars()
            .any(|c| c.is_control() && c != '\n' && c != '\r' && c != '\t')
    }

    /// Find the first dangerous pattern in the value, returning None if safe
    fn find_dangerous_pattern(value: &str) -> Option<String> {
        let patterns = get_dangerous_patterns();
        for pattern in patterns.iter() {
            if value.contains(pattern) {
                return Some(pattern.to_string());
            }
        }
        None
    }

    /// Validate an environment variable value.
    ///
    /// Checks control characters, null bytes, shell expansion (`${...}`),
    /// dangerous shell metacharacters, and the configured value-length limit.
    ///
    /// # Scope of validation for `enc:` values
    ///
    /// When `allow_encrypted_values` is enabled and the value starts with
    /// `"enc:"`, **only the base64 character set of the payload is verified**
    /// (see `validate_encrypted_format`). Control-character, null-byte,
    /// shell-expansion, dangerous-pattern, and length checks are all skipped,
    /// and the payload is never decrypted or otherwise inspected here — the
    /// scope of validation for `enc:` values is exactly this format check.
    /// This is a deliberate design decision: callers opt in via
    /// `EnvironmentValidationConfig`.
    pub fn validate_env_value(&self, value: &str) -> Result<(), EnvSecurityError> {
        if self.config.allow_encrypted_values && is_encrypted_value(value) {
            validate_encrypted_format(value)?;
            return Ok(());
        }

        // Early return if blocked patterns check is disabled
        if !self.config.enable_blocked_patterns {
            return Ok(());
        }

        // Check for control characters (excluding common whitespace)
        if Self::has_control_characters(value) {
            return Err(EnvSecurityError::CommandInjection {
                pattern: "control_character".to_string(),
            });
        }

        // Check for null bytes
        if value.contains('\0') {
            return Err(EnvSecurityError::NullByte);
        }

        // Check for shell expansion patterns
        if value.contains("${") && value.contains('}') {
            return Err(EnvSecurityError::ShellExpansion);
        }

        // Check for dangerous patterns
        if let Some(pattern) = Self::find_dangerous_pattern(value) {
            return Err(EnvSecurityError::CommandInjection { pattern });
        }

        // Length validation (moved to the end as a guard clause)
        if self.config.enable_length_validation && value.len() > self.max_value_length {
            return Err(EnvSecurityError::ValueTooLong {
                value_length: value.len(),
                max_length: self.max_value_length,
            });
        }

        Ok(())
    }

    /// Validate a complete environment variable mapping.
    ///
    /// The mapping maps configuration field names to environment variable
    /// *names*, and only names are validated here: each env name must match
    /// the allowed naming format and not hit a blocked pattern (see
    /// [`Self::validate_env_name`]), and each field name must be non-empty and
    /// free of spaces.
    ///
    /// Environment variable **values** are intentionally not validated by
    /// this method — a name-only mapping carries no values. Actual values are
    /// validated via [`Self::validate_env_value`] at the point where they are
    /// read and injected (see [`ConfigInjector::inject`] and the
    /// `inject_from_env!` macro, which both validate the real value).
    pub fn validate_env_mapping(
        &self,
        mapping: &HashMap<String, String>,
    ) -> Result<(), EnvSecurityError> {
        for (field_name, env_name) in mapping {
            // Only the naming format of the env var is checked here; a
            // name-only mapping carries no value to validate.
            self.validate_env_name(env_name, None)?;

            // Also validate that the field name is reasonable
            if field_name.is_empty() || field_name.contains(' ') {
                return Err(EnvSecurityError::InvalidFieldName {
                    field_name: field_name.to_string(),
                });
            }
        }
        Ok(())
    }

    /// Compile custom patterns, returning an error if any pattern is invalid.
    /// Unlike `filter_map(|p| Regex::new(p).ok())`, this does not silently
    /// drop malformed patterns — an invalid regex is a configuration error
    /// that must be surfaced to the caller.
    fn compile_custom_patterns(&self, patterns: &[String]) -> Result<Vec<Regex>, EnvSecurityError> {
        patterns.iter().map(|p| compile_pattern(p)).collect()
    }

    /// Sanitize an environment variable value for logging.
    ///
    /// Values are genuinely **masked**, not merely truncated: values longer
    /// than 4 characters keep only their first and last character with the
    /// middle replaced by `***` (e.g. `"abcdef"` → `"a***f"`), and values of
    /// up to 4 characters are masked entirely so no character leaks. The
    /// output is therefore always at most 5 characters, and the full value
    /// never survives sanitization.
    pub fn sanitize_for_logging(&self, value: &str) -> String {
        // Count by chars to avoid slicing into multi-byte UTF-8 characters.
        let char_count = value.chars().count();
        if char_count <= 4 {
            "*".repeat(char_count)
        } else {
            let first = value.chars().next().unwrap_or('*');
            let last = value.chars().next_back().unwrap_or('*');
            format!("{first}***{last}")
        }
    }

    /// Check if an environment variable should be allowed
    pub fn should_allow_env_var(&self, name: &str) -> bool {
        self.validate_env_name(name, None).is_ok()
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
    /// Invalid regex pattern
    InvalidRegex { pattern: String, error: String },
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
    /// Invalid value format
    InvalidValueFormat { reason: String },
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
            EnvSecurityError::InvalidRegex { pattern, error } => {
                write!(f, "Invalid regex pattern '{}': {}", pattern, error)
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
            EnvSecurityError::InvalidValueFormat { reason } => {
                write!(f, "Invalid value format: {}", reason)
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
        let validator = EnvSecurityValidator::default();
        assert!(validator.validate_env_name("APP_PORT", None).is_ok());
        assert!(validator.validate_env_name("DATABASE_HOST", None).is_ok());
        assert!(validator.validate_env_name("REDIS_PORT", None).is_ok());
    }

    #[test]
    fn test_validate_blocked_env_name() {
        let validator = EnvSecurityValidator::default();
        assert!(validator.validate_env_name("PATH", None).is_err());
        assert!(validator.validate_env_name("HOME", None).is_err());
        assert!(validator.validate_env_name("SECRET_KEY", None).is_err());
        assert!(validator.validate_env_name("API_TOKEN", None).is_err());
    }

    #[test]
    fn test_validate_invalid_env_name_format() {
        let validator = EnvSecurityValidator::default();
        assert!(validator.validate_env_name("app_port", None).is_err()); // lowercase
        assert!(validator.validate_env_name("APP-PORT", None).is_err()); // dash
        assert!(validator.validate_env_name("123PORT", None).is_err()); // starts with number
    }

    #[test]
    fn test_validate_env_name_length() {
        let validator = EnvSecurityValidator::default();

        // Valid: exactly 256 characters (max allowed)
        let valid_256 = "A".repeat(256);
        assert!(validator.validate_env_name(&valid_256, None).is_ok());

        // Valid: less than 256 characters
        let valid_255 = "A".repeat(255);
        assert!(validator.validate_env_name(&valid_255, None).is_ok());

        // Invalid: more than 256 characters
        let invalid_257 = "A".repeat(257);
        assert!(validator.validate_env_name(&invalid_257, None).is_err());
    }

    #[test]
    fn test_validate_dangerous_env_value() {
        let validator = EnvSecurityValidator::default();
        assert!(validator.validate_env_value("hello").is_ok());
        assert!(validator.validate_env_value("test123").is_ok());

        assert!(validator.validate_env_value("hello;world").is_err()); // command injection
        assert!(validator.validate_env_value("hello|world").is_err()); // pipe
        assert!(validator.validate_env_value("hello${world}").is_err()); // shell expansion
        assert!(validator.validate_env_value("hello\0world").is_err()); // null byte
    }

    #[test]
    fn test_dangerous_patterns_oncelock_optimization() {
        // Test that get_dangerous_patterns returns consistent results
        let patterns1 = get_dangerous_patterns();
        let patterns2 = get_dangerous_patterns();
        // Should be the same reference (OnceLock ensures single initialization)
        assert!(std::ptr::eq(patterns1, patterns2));

        // Test that all expected patterns are present
        let patterns = get_dangerous_patterns();
        assert!(patterns.contains(&";"));
        assert!(patterns.contains(&"&"));
        assert!(patterns.contains(&"|"));
        assert!(patterns.contains(&"`"));
        assert!(patterns.contains(&"$"));
        assert!(patterns.contains(&"\\n"));
        assert!(patterns.contains(&"\\r"));
        assert!(patterns.contains(&"\\t"));
        assert!(patterns.contains(&"; "));
        assert!(patterns.contains(&"& "));
        assert!(patterns.contains(&"| "));
        assert!(patterns.contains(&"$ "));
    }

    #[test]
    fn test_command_injection_patterns_with_optimized_loop() {
        let validator = EnvSecurityValidator::default();

        // Test all dangerous patterns from the OnceLock-optimized list
        assert!(validator.validate_env_value("test;command").is_err());
        assert!(validator.validate_env_value("test&command").is_err());
        assert!(validator.validate_env_value("test|command").is_err());
        assert!(validator.validate_env_value("test`command`").is_err());
        assert!(validator.validate_env_value("test$command").is_err());
        assert!(validator.validate_env_value("test\ncommand").is_err());
        assert!(validator.validate_env_value("test\rcommand").is_err());
        assert!(validator.validate_env_value("test\tcommand").is_err());
        assert!(validator.validate_env_value("test; command").is_err());
        assert!(validator.validate_env_value("test& command").is_err());
        assert!(validator.validate_env_value("test| command").is_err());
        assert!(validator.validate_env_value("test$ command").is_err());
    }

    #[test]
    fn test_validate_env_mapping() {
        let validator = EnvSecurityValidator::default();
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
    fn test_validate_env_mapping_only_validates_names() {
        // Regression: validate_env_mapping must not treat the env var *name*
        // as a value. With a custom allowed pattern admitting names that would
        // fail VALUE validation (parentheses), only the name format matters.
        let config = EnvironmentValidationConfig::new()
            .with_custom_allowed_patterns(vec![r"^[A-Z][A-Z0-9_()]*$".to_string()]);
        let validator = EnvSecurityValidator::with_config(config);

        let mut mapping = HashMap::new();
        mapping.insert("field".to_string(), "MY(VAR)".to_string());
        assert!(
            validator.validate_env_mapping(&mapping).is_ok(),
            "name must not be validated as a value"
        );

        // Name-format violations are still rejected.
        let mut bad = HashMap::new();
        bad.insert("field".to_string(), "lower_case_name".to_string());
        assert!(validator.validate_env_mapping(&bad).is_err());
    }

    #[test]
    fn test_sanitize_for_logging_masks_values() {
        let validator = EnvSecurityValidator::default();

        // Short values (<= 4 chars) are masked entirely.
        assert_eq!(validator.sanitize_for_logging(""), "");
        assert_eq!(validator.sanitize_for_logging("a"), "*");
        assert_eq!(validator.sanitize_for_logging("abcd"), "****");
        // Longer values keep only the first and last character.
        assert_eq!(validator.sanitize_for_logging("abcdef"), "a***f");
        assert_eq!(validator.sanitize_for_logging(&"s".repeat(150)), "s***s");
        // Multi-byte characters are masked without panicking.
        assert_eq!(validator.sanitize_for_logging("密码测试值"), "密***值");
        // The full value never survives sanitization.
        let secret = "super-secret-password-do-not-leak";
        let sanitized = validator.sanitize_for_logging(secret);
        assert_eq!(sanitized, "s***k");
        assert!(!sanitized.contains(secret));
    }

    #[test]
    fn test_custom_length_limits() {
        let config = EnvironmentValidationConfig::new()
            .with_max_name_length(100)
            .with_max_value_length(500);
        let validator = EnvSecurityValidator::with_config(config);

        let valid_100 = "A".repeat(100);
        assert!(validator.validate_env_name(&valid_100, None).is_ok());

        let invalid_101 = "A".repeat(101);
        assert!(validator.validate_env_name(&invalid_101, None).is_err());

        assert!(validator.validate_env_value(&"x".repeat(500)).is_ok());
        assert!(validator.validate_env_value(&"x".repeat(501)).is_err());
    }

    #[test]
    fn test_strict_validator() {
        let validator = EnvSecurityValidator::strict();

        let valid_128 = "A".repeat(128);
        assert!(validator.validate_env_name(&valid_128, None).is_ok());

        let invalid_129 = "A".repeat(129);
        assert!(validator.validate_env_name(&invalid_129, None).is_err());

        assert!(validator.validate_env_value(&"x".repeat(2048)).is_ok());
        assert!(validator.validate_env_value(&"x".repeat(2049)).is_err());
    }

    #[test]
    fn test_lenient_validator() {
        let validator = EnvSecurityValidator::lenient();

        let long_name = "A".repeat(500);
        assert!(validator.validate_env_name(&long_name, None).is_ok());

        let long_value = "x".repeat(5000);
        assert!(validator.validate_env_value(&long_value).is_ok());

        assert!(validator.validate_env_name("PATH", None).is_ok());
        assert!(validator.validate_env_value("hello;world").is_ok());
    }

    #[test]
    fn test_disabled_blocked_patterns() {
        let config = EnvironmentValidationConfig::new().with_blocked_patterns_disabled();
        let validator = EnvSecurityValidator::with_config(config);

        assert!(validator.validate_env_name("PATH", None).is_ok());
        assert!(validator.validate_env_name("HOME", None).is_ok());
        assert!(validator.validate_env_name("SECRET_KEY", None).is_ok());
    }

    #[test]
    fn test_disabled_length_validation() {
        let config = EnvironmentValidationConfig::new().with_length_validation_disabled();
        let validator = EnvSecurityValidator::with_config(config);

        let very_long_name = "A".repeat(1000);
        assert!(validator.validate_env_name(&very_long_name, None).is_ok());

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

        let secret_with_encrypted = "MY_SECRET"; // pragma: allowlist secret
        assert!(
            validator
                .validate_env_name(secret_with_encrypted, Some(encrypted_value))
                .is_ok()
        );
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
        assert!(validator.validate_env_name(&long_name, None).is_ok());

        let invalid_513 = "A".repeat(513);
        assert!(validator.validate_env_name(&invalid_513, None).is_err());
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

// Security primitives wired to the public API via feature gates.
// `config_injector` and `input_validation` are gated behind `security-rules`;
// `secure_string` is gated behind `encryption`.
#[cfg(feature = "security-rules")]
pub(crate) mod config_injector;
pub(crate) mod error_sanitization;
#[cfg(feature = "security-rules")]
pub(crate) mod input_validation;
#[cfg(feature = "encryption")]
pub(crate) mod secure_string;

#[cfg(feature = "encryption")]
pub use error_sanitization::{
    Error as SanitizationError, ErrorSanitizer, FilterResult, LogLevel, SafeResult, SecureLogger,
    SensitiveDataFilter,
};

// ── Public API re-exports ──────────────────────────────────────────────
#[cfg(feature = "security-rules")]
pub use config_injector::{ConfigInjectionError, ConfigInjector, EnvironmentConfig};
#[cfg(feature = "security-rules")]
pub use input_validation::{
    ConfigValidationError, ConfigValidationResult, ConfigValidator, ConfigValidatorBuilder,
    InputValidationError, InputValidator, SensitiveDataDetector, SensitivityResult,
};
#[cfg(feature = "encryption")]
pub use secure_string::{
    SecureString, SecureStringBuilder, SensitiveData, SensitivityLevel, allocated_secure_strings,
    deallocated_secure_strings,
};
