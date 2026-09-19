// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! Error message sanitization for the confers error module.
//!
//! This module contains precompiled regex patterns and functions for stripping
//! sensitive data (file paths, IP addresses, key material, JWT tokens, AWS keys,
//! URLs with embedded credentials) from error messages before they are displayed
//! to users or written to logs.

use std::sync::LazyLock;

use super::ConfigError;

// Precompiled regex patterns for sanitization (avoid recompiling on each call)

/// Regex pattern for matching file paths (Unix and Windows style), while
/// keeping whole URLs as one token so their path segments are never rewritten
/// as local file paths (https://example.com/api/v1/users stays intact)
static PATH_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"(https?://\S+)|(/[a-zA-Z0-9_\-./]+)|([a-zA-Z]:\\[a-zA-Z0-9_\-./\\]+)")
        .expect("PATH_RE regex is valid")
});

/// Regex pattern for matching IP addresses
static IP_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"\b\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}\b").expect("IP_RE regex is valid")
});

/// Regex pattern for matching long hex strings (>= 32 chars: AES-128 hex
/// keys, SHA-256 digests, 64-hex-char key material), redacted unconditionally
static HEX_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"\b[0-9a-fA-F]{32,}\b").expect("HEX_RE regex is valid"));

/// Regex pattern for matching shorter hex runs (16-31 chars), redacted only
/// when sensitive context keywords appear nearby (git SHAs, device IDs, and
/// other benign hex runs must not be masked)
static HEX_SHORT_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"\b[0-9a-fA-F]{16,31}\b").expect("HEX_SHORT_RE regex is valid")
});

/// Regex pattern for matching URLs with embedded credentials.
///
/// Only `http`/`https` schemes are matched. Other schemes that commonly embed
/// credentials (`ftp://`, `redis://`, `smtp://`, `mongodb://`, ...) are NOT
/// covered by this pattern and their credentials are not redacted here.
static URL_WITH_CREDS_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"(?P<scheme>https?)://[^/]+:[^@]+@[^/\s]+[/\s]?")
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

/// Regex pattern for matching AWS-secret-shaped tokens (40-char alphanumeric)
static AWS_SAK_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"\b[A-Za-z0-9/+=]{40}\b").expect("AWS_SAK_RE regex is valid")
});

/// Keywords that, when present near a candidate token, mark it as likely
/// credential material (AWS secret keys, hex keys, ...)
static SENSITIVE_CONTEXT_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"(?i)(aws|secret|token|key|credential|password|signature|auth)")
        .expect("SENSITIVE_CONTEXT_RE regex is valid")
});

/// Number of characters scanned before/after a candidate token for sensitive
/// context keywords
const CONTEXT_WINDOW: usize = 40;

/// Largest index at or below `i` that lies on a UTF-8 char boundary of `s`
fn floor_char_boundary(s: &str, i: usize) -> usize {
    let mut i = i.min(s.len());
    while i > 0 && !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}

/// Smallest index at or above `i` that lies on a UTF-8 char boundary of `s`
fn ceil_char_boundary(s: &str, i: usize) -> usize {
    if i >= s.len() {
        return s.len();
    }
    let mut i = i;
    while i < s.len() && !s.is_char_boundary(i) {
        i += 1;
    }
    i
}

/// Replace every match of `re` whose surrounding text (within
/// [`CONTEXT_WINDOW`] characters on either side) contains a sensitive context
/// keyword; matches without such context are left untouched.
///
/// A bare 40-char base64-shaped token or a 16-31 char hex run is far too
/// generic to redact blindly (any opaque session id matches), so context is
/// required before treating them as secrets.
fn redact_with_context(text: &str, re: &regex::Regex, replacement: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut last = 0;
    for m in re.find_iter(text) {
        let before_start = floor_char_boundary(text, m.start().saturating_sub(CONTEXT_WINDOW));
        let after_end = ceil_char_boundary(text, m.end() + CONTEXT_WINDOW);
        let has_context = SENSITIVE_CONTEXT_RE.is_match(&text[before_start..m.start()])
            || SENSITIVE_CONTEXT_RE.is_match(&text[m.end()..after_end]);
        out.push_str(&text[last..m.start()]);
        if has_context {
            out.push_str(replacement);
        } else {
            out.push_str(m.as_str());
        }
        last = m.end();
    }
    out.push_str(&text[last..]);
    out
}

/// Sanitize an error message by removing sensitive data.
///
/// This is the central sanitization function used by `user_message()` and
/// `sanitized_chain()`. It removes:
/// - File paths (replaced with `<path>/filename`); URL path segments are
///   preserved (a URL is not a file path)
/// - IP addresses (replaced with `<ip>`)
/// - Long hex strings / key material (>= 32 chars replaced with `<redacted>`;
///   16-31 char hex runs only when sensitive context keywords appear nearby)
/// - URLs with embedded credentials (replaced with `<redacted_url>`)
/// - JWT tokens (replaced with `<jwt_token>`)
/// - AWS access key IDs (`AKIA...`, replaced with `<aws_access_key>`)
/// - AWS-secret-shaped 40-char tokens, but only when sensitive context
///   keywords appear nearby (replaced with `<aws_secret_key>`)
///
/// The user-facing message will not contain any of these sensitive patterns.
///
/// This is also the crate-wide sanitizer for free-form output text (e.g. the
/// CLI `diff --sanitize` output), so all redaction rules live in one place.
pub(crate) fn sanitize_error_message(msg: &str) -> String {
    let mut result = msg.to_string();

    // Remove URLs with embedded credentials first (before other replacements)
    result = URL_WITH_CREDS_RE
        .replace_all(&result, "<redacted_url>")
        .to_string();

    // Remove potential file paths (Unix and Windows style) using precompiled regex.
    // Whole URLs are matched as one token and left untouched.
    result = PATH_RE
        .replace_all(&result, |caps: &regex::Captures| {
            // URL path segments (e.g. https://example.com/api/v1/users) are
            // not local file paths and must not be rewritten.
            if caps.get(1).is_some() {
                return caps[0].to_string();
            }
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

    // Remove AWS secret access keys (40-char base64-shaped tokens), but only
    // when sensitive context keywords appear nearby: the bare pattern is far
    // too generic and would mask any opaque token.
    result = redact_with_context(&result, &AWS_SAK_RE, "<aws_secret_key>");

    // Remove potential key material: hex runs >= 32 chars (AES-128 hex keys,
    // SHA-256 digests, ...) are redacted unconditionally; shorter runs
    // (16-31 chars, often git SHAs or device IDs) only with sensitive context.
    result = HEX_RE.replace_all(&result, "<redacted>").to_string();
    result = redact_with_context(&result, &HEX_SHORT_RE, "<redacted>");

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

        // Remove credentials from URLs but keep the URL structure,
        // preserving the original scheme (http stays http, not https)
        result = URL_WITH_CREDS_RE
            .replace_all(&result, |caps: &regex::Captures| {
                format!("{}://<creds>@<host>/", &caps["scheme"])
            })
            .to_string();

        // Keep file paths but redact the directory part. Whole URLs are
        // matched as one token and left untouched.
        result = PATH_RE
            .replace_all(&result, |caps: &regex::Captures| {
                if caps.get(1).is_some() {
                    return caps[0].to_string();
                }
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

    // =============================================================================
    // #43: URL path segments must not be rewritten as file paths
    // =============================================================================

    #[test]
    fn test_sanitize_error_message_preserves_url_path() {
        let msg = "API docs at https://example.com/api/v1/users returned 200";
        let sanitized = sanitize_error_message(msg);
        assert!(
            sanitized.contains("https://example.com/api/v1/users"),
            "URL path must not be rewritten, got: {}",
            sanitized
        );
        assert!(!sanitized.contains("<path>"), "got: {}", sanitized);
    }

    #[test]
    fn test_sanitize_error_message_still_redacts_local_paths() {
        let msg = "Failed to load /home/user/project/secrets.txt";
        let sanitized = sanitize_error_message(msg);
        assert!(
            sanitized.contains("<path>/secrets.txt"),
            "got: {}",
            sanitized
        );
        assert!(
            !sanitized.contains("/home/user/project/"),
            "got: {}",
            sanitized
        );
    }

    // =============================================================================
    // #42: AWS-secret-shaped tokens require sensitive context
    // =============================================================================

    #[test]
    fn test_sanitize_error_message_aws_secret_key_with_context() {
        // Real AWS secret access key scenario: credential name next to the
        // value (40 alphanumeric chars, no '/' so PATH_RE cannot bite first)
        let msg = "AWS_SECRET_ACCESS_KEY=wJalrXUtnFEMIK7MDENGbPxRfiCYEXAMPLEKEY12"; // pragma: allowlist secret
        let sanitized = sanitize_error_message(msg);
        assert!(
            sanitized.contains("<aws_secret_key>"),
            "expected AWS secret key to be redacted, got: {}",
            sanitized
        );
        assert!(!sanitized.contains("wJalrXUtnFEMIK7MDENG"));
    }

    #[test]
    fn test_sanitize_error_message_generic_40char_token_not_redacted() {
        // 40-char base64-shaped token with no sensitive keywords nearby:
        // an opaque session/trace id must survive sanitization
        let msg = "Random opaque string WmbHKbCUrlo8AucIwYzKemDrVNhbergQmBfPsdxZ for tracing";
        let sanitized = sanitize_error_message(msg);
        assert!(
            !sanitized.contains("<aws_secret_key>"),
            "generic token must not be treated as an AWS secret: {}",
            sanitized
        );
        assert!(
            sanitized.contains("WmbHKbCUrlo8AucIwYzKemDrVNhbergQmBfPsdxZ"),
            "got: {}",
            sanitized
        );
    }

    // =============================================================================
    // #47: hex redaction thresholds
    // =============================================================================

    #[test]
    fn test_sanitize_error_message_sha256_hex_redacted() {
        // 64 hex chars (SHA-256-sized key material) is redacted unconditionally
        let key_hex = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"; // pragma: allowlist secret
        let msg = format!("encryption key digest {}", key_hex);
        let sanitized = sanitize_error_message(&msg);
        assert!(sanitized.contains("<redacted>"), "got: {}", sanitized);
        assert!(!sanitized.contains(key_hex), "got: {}", sanitized);
    }

    #[test]
    fn test_sanitize_error_message_short_hex_without_context_kept() {
        // 16-31 char hex runs without sensitive context (git SHA, device id)
        // must not be masked
        let msg = "commit 1234567890abcdef is on device 0123456789abcdef01234567";
        let sanitized = sanitize_error_message(msg);
        assert!(
            !sanitized.contains("<redacted>"),
            "benign hex must be kept, got: {}",
            sanitized
        );
        assert!(sanitized.contains("1234567890abcdef"), "got: {}", sanitized);
    }

    #[test]
    fn test_sanitize_error_message_short_hex_with_context_redacted() {
        // 16-31 char hex run with a sensitive keyword nearby is a secret
        let msg = "api_key: 0123456789abcdef01234567 (see logs)"; // pragma: allowlist secret
        let sanitized = sanitize_error_message(msg);
        assert!(sanitized.contains("<redacted>"), "got: {}", sanitized);
        assert!(
            !sanitized.contains("0123456789abcdef01234567"),
            "got: {}",
            sanitized
        );
    }

    // =============================================================================
    // #44: debug_message preserves the original URL scheme
    // =============================================================================

    #[test]
    fn test_debug_message_preserves_http_scheme() {
        let err = ConfigError::InvalidValue {
            key: "db.url".into(),
            expected_type: "url".into(),
            message: "connect failed for http://alice:hunter2@example.com/db".into(), // pragma: allowlist secret
        };
        let debug = err.debug_message();
        assert!(
            debug.contains("http://<creds>@<host>/"),
            "original scheme must be preserved, got: {}",
            debug
        );
        assert!(
            !debug.contains("https://<creds>"),
            "http must not be relabeled as https, got: {}",
            debug
        );
        assert!(!debug.contains("alice:hunter2"), "got: {}", debug);
    }

    #[test]
    fn test_debug_message_preserves_https_scheme() {
        let err = ConfigError::InvalidValue {
            key: "db.url".into(),
            expected_type: "url".into(),
            message: "connect failed for https://alice:hunter2@example.com/db".into(), // pragma: allowlist secret
        };
        let debug = err.debug_message();
        assert!(debug.contains("https://<creds>@<host>/"), "got: {}", debug);
        assert!(!debug.contains("alice:hunter2"), "got: {}", debug);
    }
}
