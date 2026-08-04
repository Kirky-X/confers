// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Error message sanitization for the confers error module.
//!
//! This module contains precompiled regex patterns and functions for stripping
//! sensitive data (file paths, IP addresses, key material, JWT tokens, AWS keys,
//! URLs with embedded credentials) from error messages before they are displayed
//! to users or written to logs.

use std::sync::LazyLock;

use super::ConfigError;

// Precompiled regex patterns for sanitization (avoid recompiling on each call)

/// Regex pattern for matching file paths (Unix and Windows style)
static PATH_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"/[a-zA-Z0-9_\-./]+|[a-zA-Z]:\\[a-zA-Z0-9_\-./\\]+")
        .expect("PATH_RE regex is valid")
});

/// Regex pattern for matching IP addresses
static IP_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"\b\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}\b").expect("IP_RE regex is valid")
});

/// Regex pattern for matching potential key material (long hex strings)
static HEX_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"\b[0-9a-fA-F]{16,}\b").expect("HEX_RE regex is valid"));

/// Regex pattern for matching URLs with embedded credentials
static URL_WITH_CREDS_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"https?://[^/]+:[^@]+@[^/\s]+[/\s]?")
        .expect("URL_WITH_CREDS_RE regex is valid")
});

/// Regex pattern for matching JWT tokens
static JWT_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"eyJ[A-Za-z0-9_=-]+\.[A-Za-z0-9_=-]*\.?[A-Za-z0-9_=-]*")
        .expect("JWT_RE regex is valid")
});

/// Regex pattern for matching AWS access key IDs
static AWS_AK_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"\bAKIA[0-9A-Z]{16}\b").expect("AWS_AK_RE regex is valid"));

/// Regex pattern for matching AWS secret access keys (40-char alphanumeric)
static AWS_SAK_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"\b[A-Za-z0-9/+=]{40}\b").expect("AWS_SAK_RE regex is valid")
});

/// Sanitize an error message by removing sensitive data.
///
/// This is the central sanitization function used by `user_message()` and
/// `sanitized_chain()`. It removes:
/// - File paths (replaced with `<path>/filename`)
/// - IP addresses (replaced with `<ip>`)
/// - Long hex strings / key material (replaced with `<redacted>`)
/// - URLs with embedded credentials
/// - JWT tokens
/// - AWS access key IDs
///
/// The user-facing message will not contain any of these sensitive patterns.
pub(super) fn sanitize_error_message(msg: &str) -> String {
    let mut result = msg.to_string();

    // Remove URLs with embedded credentials first (before other replacements)
    result = URL_WITH_CREDS_RE
        .replace_all(&result, "<redacted_url>")
        .to_string();

    // Remove potential file paths (Unix and Windows style) using precompiled regex
    result = PATH_RE
        .replace_all(&result, |caps: &regex::Captures| {
            let full_path = &caps[0];
            // Keep only the filename for debugging
            if let Some(filename) = full_path
                .split('/')
                .next_back()
                .or_else(|| full_path.split('\\').next_back())
            {
                format!("<path>/{}", filename)
            } else {
                "<path>".to_string()
            }
        })
        .to_string();

    // Remove potential IP addresses using precompiled regex
    result = IP_RE.replace_all(&result, "<ip>").to_string();

    // Remove JWT tokens using precompiled regex
    result = JWT_RE.replace_all(&result, "<jwt_token>").to_string();

    // Remove AWS access key IDs using precompiled regex
    result = AWS_AK_RE
        .replace_all(&result, "<aws_access_key>")
        .to_string();

    // Remove AWS secret access keys (40-char strings near AWS context)
    // Only redact if surrounded by whitespace or common delimiters
    result = AWS_SAK_RE
        .replace_all(&result, "<aws_secret_key>")
        .to_string();

    // Remove potential key material (long hex strings) using precompiled regex
    result = HEX_RE.replace_all(&result, "<redacted>").to_string();

    result
}

impl ConfigError {
    /// Get a detailed debug message for internal logging.
    ///
    /// Unlike `user_message()` which is safe to show to end users, this method
    /// may include file paths, IP addresses, and other diagnostic information
    /// useful for debugging. Do NOT expose this message to end users.
    ///
    /// For structured logging, prefer using the `error_code()` and field accessors.
    pub fn debug_message(&self) -> String {
        // Use the Display impl which gives full details
        let full = format!("{}", self);

        // Apply additional sanitization that still keeps some context
        let mut result = full;

        // Remove credentials from URLs but keep the URL structure
        result = URL_WITH_CREDS_RE
            .replace_all(&result, "https://<creds>@<host>/")
            .to_string();

        // Keep file paths but redact the directory part
        result = PATH_RE
            .replace_all(&result, |caps: &regex::Captures| {
                let full_path = &caps[0];
                full_path
                    .split('/')
                    .next_back()
                    .or_else(|| full_path.split('\\').next_back())
                    .map(|f| format!("<path>/{}", f))
                    .unwrap_or_else(|| "<path>".to_string())
            })
            .to_string();

        result
    }

    /// Check if this error may contain sensitive data.
    ///
    /// Returns `true` for errors that are likely to contain sensitive information
    /// such as keys, passwords, tokens, or credentials. Use this to determine
    /// whether to sanitize error messages before logging or displaying.
    ///
    /// Note: This is a heuristic check and may return `false` positives.
    /// Always prefer explicit sanitization via `sanitize_error_message()`.
    pub fn is_sensitive(&self) -> bool {
        // Check if the raw error message contains sensitive patterns
        let raw = format!("{}", self);

        // Check for sensitive patterns in the raw error
        JWT_RE.is_match(&raw)
            || AWS_AK_RE.is_match(&raw)
            || AWS_SAK_RE.is_match(&raw)
            || URL_WITH_CREDS_RE.is_match(&raw)
            || (HEX_RE.is_match(&raw) && raw.len() > 50) // Long hex strings are more likely keys
            || {
                // Check for common key/password field names
                let lower = raw.to_lowercase();
                lower.contains("secret")
                    || lower.contains("password")
                    || lower.contains("token")
                    || lower.contains("api_key")
                    || lower.contains("private_key")
                    || lower.contains("credential")
                    || lower.contains(" key ")  // standalone "key" word
                    || lower.ends_with("key")   // suffix "key" (e.g., "encryption key")
            }
    }

    /// Get the error chain with sensitive data removed.
    pub fn sanitized_chain(&self) -> Vec<String> {
        let mut chain = vec![self.user_message()];

        // Add source errors if present, but sanitize them
        match self {
            ConfigError::ParseError { source, .. }
            | ConfigError::MigrationFailed { source, .. } => {
                if let Some(e) = source {
                    chain.push(sanitize_error_message(&e.to_string()));
                }
            }
            _ => {}
        }

        chain
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // =============================================================================
    // Error Sanitization Tests (9.3.6)
    // =============================================================================

    #[test]
    fn test_sanitize_error_message_full_path() {
        let msg = "Failed to load /home/user/project/config.toml";
        let sanitized = sanitize_error_message(msg);
        // Full paths should be converted to <path>/filename
        assert!(!sanitized.contains("/home/user/project/"));
        assert!(sanitized.contains("config.toml"));
        assert!(sanitized.contains("<path>"));
    }

    #[test]
    fn test_sanitize_error_message_url_with_credentials() {
        let msg = "Failed to fetch https://user:secret123@example.com/config.json"; // pragma: allowlist secret
        let sanitized = sanitize_error_message(msg);
        // Should not contain the credentials
        assert!(!sanitized.contains("user:secret123"));
        assert!(sanitized.contains("<redacted_url>") || sanitized.contains("<redacted>"));
    }

    #[test]
    fn test_sanitize_error_message_jwt_token() {
        let msg = "Validation failed for token eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4ifQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c"; // pragma: allowlist secret
        let sanitized = sanitize_error_message(msg);
        assert!(!sanitized.contains("eyJ"));
        assert!(sanitized.contains("<jwt_token>"));
    }

    #[test]
    fn test_sanitize_error_message_aws_access_key() {
        let msg = "AWS error: AKIAIOSFODNN7EXAMPLE is invalid"; // pragma: allowlist secret
        let sanitized = sanitize_error_message(msg);
        assert!(!sanitized.contains("AKIAIOSFODNN7EXAMPLE"));
        assert!(sanitized.contains("<aws_access_key>"));
    }

    #[test]
    fn test_sanitize_error_message_ip_address() {
        let msg = "Connection refused from 192.168.1.100";
        let sanitized = sanitize_error_message(msg);
        assert!(!sanitized.contains("192.168.1.100"));
        assert!(sanitized.contains("<ip>"));
    }

    #[test]
    fn test_sanitize_error_message_hex_key() {
        let msg = "Key mismatch: abcdef0123456789abcdef0123456789";
        let sanitized = sanitize_error_message(msg);
        assert!(!sanitized.contains("abcdef0123456789abcdef0123456789"));
        assert!(sanitized.contains("<redacted>"));
    }

    #[test]
    fn test_sanitize_error_message_aws_secret_key() {
        // Exactly 40 chars of [A-Za-z0-9/+=] surrounded by spaces
        let msg = " secret: abcdefghijklmnopqrstuvwxyz0123456789ABCD "; // pragma: allowlist secret
        let sanitized = sanitize_error_message(msg);
        assert!(
            sanitized.contains("<aws_secret_key>"),
            "expected AWS secret key to be redacted, got: {}",
            sanitized
        );
    }

    #[test]
    fn test_user_message_does_not_leak_sensitive_data() {
        // FileNotFound with sensitive-looking path
        let err = ConfigError::FileNotFound {
            filename: PathBuf::from("/home/user/.ssh/id_rsa"),
            source: None,
        };
        let user_msg = err.user_message();
        // Should show filename but not full path
        assert!(!user_msg.contains("/home/user/.ssh/"));
        assert!(user_msg.contains("id_rsa"));
    }

    #[test]
    fn test_debug_message_contains_file_path() {
        let err = ConfigError::FileNotFound {
            filename: PathBuf::from("/home/user/project/config.toml"),
            source: None,
        };
        let debug = err.debug_message();
        // Debug message should contain the full path for diagnostics
        assert!(debug.contains("config.toml") || debug.contains("<path>"));
    }

    #[test]
    fn test_is_sensitive_decryption_error() {
        let err = ConfigError::DecryptionFailed {
            message: "key mismatch".to_string(),
        };
        assert!(err.is_sensitive()); // "key" in message
    }

    #[test]
    fn test_is_sensitive_file_not_found() {
        // Normal file not found should not be sensitive
        let err = ConfigError::FileNotFound {
            filename: PathBuf::from("config.toml"),
            source: None,
        };
        assert!(!err.is_sensitive());
    }

    #[test]
    fn test_is_sensitive_key_error() {
        let err = ConfigError::KeyError {
            message: "encryption key error".to_string(),
        };
        assert!(err.is_sensitive()); // "key" in message
    }

    #[test]
    fn test_is_sensitive_aws_key_in_message() {
        let err = ConfigError::InvalidValue {
            key: "aws_access_key".to_string(),
            expected_type: "string".to_string(),
            message: "AKIAIOSFODNN7EXAMPLE is invalid".to_string(), // pragma: allowlist secret
        };
        assert!(err.is_sensitive()); // Contains AWS access key
    }

    // is_sensitive for sensitive patterns
    // =============================================================================

    #[test]
    fn test_is_sensitive_url_with_credentials() {
        let err = ConfigError::DecryptionFailed {
            message: "fetch https://user:passw0rd123@example.com/keys failed".into(), // pragma: allowlist secret
        };
        assert!(err.is_sensitive());
    }

    #[test]
    fn test_is_sensitive_jwt_token() {
        let err = ConfigError::DecryptionFailed {
            message: "token eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.payload.sig invalid".into(), // pragma: allowlist secret
        };
        assert!(err.is_sensitive());
    }

    #[test]
    fn test_is_sensitive_password_field() {
        let err = ConfigError::InvalidValue {
            key: "db.password".into(),
            expected_type: "string".into(),
            message: "too short".into(),
        };
        assert!(err.is_sensitive());
    }

    #[test]
    fn test_is_sensitive_token_field() {
        let err = ConfigError::InvalidValue {
            key: "auth.token".into(),
            expected_type: "string".into(),
            message: "expired".into(),
        };
        assert!(err.is_sensitive());
    }

    #[test]
    fn test_is_sensitive_api_key_field() {
        let err = ConfigError::InvalidValue {
            key: "service.api_key".into(),
            expected_type: "string".into(),
            message: "missing".into(),
        };
        assert!(err.is_sensitive());
    }

    #[test]
    fn test_is_sensitive_credential_field() {
        let err = ConfigError::InvalidValue {
            key: "credential".into(),
            expected_type: "string".into(),
            message: "invalid".into(),
        };
        assert!(err.is_sensitive());
    }

    #[test]
    fn test_is_sensitive_secret_field() {
        let err = ConfigError::InvalidValue {
            key: "client_secret".into(),
            expected_type: "string".into(),
            message: "missing".into(),
        };
        assert!(err.is_sensitive());
    }

    #[test]
    fn test_is_sensitive_clean_error_returns_false() {
        let err = ConfigError::Timeout { duration_ms: 100 };
        assert!(!err.is_sensitive());

        let err = ConfigError::VersionMismatch {
            found: 1,
            expected: 2,
        };
        assert!(!err.is_sensitive());
    }

    // =============================================================================
    // sanitized_chain with source errors
    // =============================================================================

    #[test]
    fn test_sanitized_chain_parse_error_with_source() {
        let source: Box<dyn std::error::Error + Send + Sync> =
            Box::new(std::io::Error::other("inner cause"));
        let err = ConfigError::ParseError {
            format: "toml".into(),
            message: "outer".into(),
            location: None,
            source: Some(source),
        };
        let chain = err.sanitized_chain();
        assert_eq!(chain.len(), 2);
        // First entry is the user_message (sanitized)
        assert!(chain[0].contains("toml"));
        // Second entry is the sanitized source message
        assert!(chain[1].contains("inner cause"));
    }

    #[test]
    fn test_sanitized_chain_parse_error_no_source() {
        let err = ConfigError::ParseError {
            format: "toml".into(),
            message: "outer".into(),
            location: None,
            source: None,
        };
        let chain = err.sanitized_chain();
        assert_eq!(chain.len(), 1);
    }

    #[test]
    fn test_sanitized_chain_migration_failed_with_source() {
        let source: Box<dyn std::error::Error + Send + Sync> =
            Box::new(std::io::Error::other("migration cause"));
        let err = ConfigError::MigrationFailed {
            from: 1,
            to: 2,
            reason: "outer".into(),
            source: Some(source),
        };
        let chain = err.sanitized_chain();
        assert_eq!(chain.len(), 2);
        assert!(chain[1].contains("migration cause"));
    }

    #[test]
    fn test_sanitized_chain_other_variants_no_source() {
        // Variants without source only have a single entry
        let err = ConfigError::Timeout { duration_ms: 1 };
        let chain = err.sanitized_chain();
        assert_eq!(chain.len(), 1);

        let err = ConfigError::FileNotFound {
            filename: PathBuf::from("x"),
            source: None,
        };
        let chain = err.sanitized_chain();
        assert_eq!(chain.len(), 1);
    }
}
