use regex::Regex;
use std::collections::HashMap;
use std::sync::OnceLock;

/// Security validation for environment variable mapping
pub struct EnvSecurityValidator {
    /// Whitelist of allowed environment variable patterns
    allowed_patterns: Vec<Regex>,
    /// Blacklist of dangerous environment variable patterns
    blocked_patterns: Vec<Regex>,
    /// Maximum length for environment variable names
    max_name_length: usize,
    /// Maximum length for environment variable values
    max_value_length: usize,
}

impl Default for EnvSecurityValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl EnvSecurityValidator {
    /// Create a new security validator with default rules
    pub fn new() -> Self {
        Self {
            allowed_patterns: vec![
                // Allow common application patterns
                Regex::new(r"^[A-Z][A-Z0-9_]*$").unwrap(), // Standard env vars like APP_PORT
                Regex::new(r"^[A-Z][A-Z0-9_]*_[A-Z][A-Z0-9_]*$").unwrap(), // Nested like APP_DB_HOST
                Regex::new(r"^[A-Z][A-Z0-9_]*_[A-Z][A-Z0-9_]*_[A-Z][A-Z0-9_]*$").unwrap(), // Deep nested like TEST_DETAILS_COUNT
            ],
            blocked_patterns: vec![
                // Block dangerous system environment variables
                Regex::new(r"(?i)^(PATH|LD_LIBRARY_PATH|LD_PRELOAD)$").unwrap(),
                Regex::new(r"(?i)^(SHELL|HOME|USER|LOGNAME)$").unwrap(),
                Regex::new(r"(?i)^(PWD|OLDPWD)$").unwrap(),
                Regex::new(r"(?i)^(MAIL|MAILCHECK)$").unwrap(),
                Regex::new(r"(?i)^(TERM|TERMCAP)$").unwrap(),
                Regex::new(r"(?i)^(DISPLAY|XAUTHORITY)$").unwrap(),
                Regex::new(r"(?i)^(SSH_AUTH_SOCK|SSH_AGENT_PID)$").unwrap(),
                Regex::new(r"(?i)^(DOCKER_HOST|KUBECONFIG)$").unwrap(),
                // Block variables that could contain secrets
                Regex::new(r"(?i).*(_SECRET|_PASSWORD|_TOKEN|_KEY|_PRIVATE)$").unwrap(),
                // Block variables with special characters that could cause injection
                Regex::new(r".*[;<>&|`$].*").unwrap(),
                // Block variables that look like shell functions
                Regex::new(r"^BASH_FUNC_.*").unwrap(),
            ],
            max_name_length: 256,
            max_value_length: 4096,
        }
    }

    /// Validate an environment variable name
    /// If value is provided and starts with "enc:", secret-related names are allowed
    pub fn validate_env_name(
        &self,
        name: &str,
        value: Option<&str>,
    ) -> Result<(), EnvSecurityError> {
        // Check length
        if name.len() > self.max_name_length {
            return Err(EnvSecurityError::NameTooLong {
                name: name.to_string(),
                max_length: self.max_name_length,
                actual_length: name.len(),
            });
        }

        // Check if name matches any blocked pattern
        for pattern in &self.blocked_patterns {
            if pattern.is_match(name) {
                // Allow secret-related names if value is encrypted
                if let Some(val) = value {
                    if val.starts_with("enc:") {
                        // Encrypted values are safe to use even in secret-named vars
                        continue;
                    }
                }
                return Err(EnvSecurityError::BlockedName {
                    name: name.to_string(),
                    pattern: pattern.as_str().to_string(),
                });
            }
        }

        // Check if name matches any allowed pattern
        let mut matched = false;
        for pattern in &self.allowed_patterns {
            if pattern.is_match(name) {
                matched = true;
                break;
            }
        }

        if !matched {
            return Err(EnvSecurityError::InvalidNameFormat {
                name: name.to_string(),
                expected_patterns: self
                    .allowed_patterns
                    .iter()
                    .map(|p| p.as_str().to_string())
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
        // Skip validation for encrypted values - they contain random base64 characters
        // that may trigger false positives in security checks
        if value.starts_with("enc:") {
            return Ok(());
        }

        // Check length
        if value.len() > self.max_value_length {
            return Err(EnvSecurityError::ValueTooLong {
                value_length: value.len(),
                max_length: self.max_value_length,
            });
        }

        // Check for dangerous content
        if value.contains('\0') {
            return Err(EnvSecurityError::NullByte);
        }

        // Check for potential injection patterns
        if value.contains("${") && value.contains('}') {
            return Err(EnvSecurityError::ShellExpansion {
                value: value.to_string(),
            });
        }

        // Check for command injection patterns
        let dangerous_patterns = [";", "&", "|", "`", "$", "(", ")", "<", ">", "\n", "\r"];
        for pattern in &dangerous_patterns {
            if value.contains(pattern) {
                return Err(EnvSecurityError::CommandInjection {
                    value: value.to_string(),
                    pattern: pattern.to_string(),
                });
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

    /// Create a validator with strict security rules
    pub fn strict() -> Self {
        let mut validator = Self::new();
        // Add more restrictive patterns
        validator.allowed_patterns = vec![
            Regex::new(r"^[A-Z][A-Z0-9_]{2,30}$").unwrap(), // More restrictive length
        ];
        validator.max_name_length = 30;
        validator.max_value_length = 1024;
        validator
    }

    /// Create a validator with relaxed rules for development
    pub fn development() -> Self {
        let mut validator = Self::new();
        // Allow more patterns in development
        validator.allowed_patterns.push(
            Regex::new(r"^[a-zA-Z][a-zA-Z0-9_]*$").unwrap(), // Allow lowercase
        );
        validator
    }
}

/// Global security validator instance
static GLOBAL_VALIDATOR: OnceLock<EnvSecurityValidator> = OnceLock::new();

/// Get the global security validator
pub fn get_global_validator() -> &'static EnvSecurityValidator {
    GLOBAL_VALIDATOR.get_or_init(EnvSecurityValidator::new)
}

/// Set a custom global security validator
pub fn set_global_validator(validator: EnvSecurityValidator) {
    let _ = GLOBAL_VALIDATOR.set(validator);
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
    ShellExpansion { value: String },
    /// Environment variable value contains command injection patterns
    CommandInjection { value: String, pattern: String },
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
            EnvSecurityError::ShellExpansion { value } => {
                write!(
                    f,
                    "Environment variable value contains shell expansion: '{}'",
                    value
                )
            }
            EnvSecurityError::CommandInjection { value, pattern } => {
                write!(
                    f,
                    "Environment variable value '{}' contains dangerous pattern: '{}'",
                    value, pattern
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
}
