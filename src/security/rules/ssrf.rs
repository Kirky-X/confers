// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! SSRF (Server-Side Request Forgery) protection validator.

use super::{SecurityValidator, SecurityViolation, ViolationSeverity};
use crate::interface::ConfigProvider;
use ipnet::IpNet;
use std::net::IpAddr;
use std::str::FromStr;
use std::sync::LazyLock;

/// Private/reserved IP ranges that should be blocked for SSRF protection.
static BLOCKED_NETWORKS: LazyLock<Vec<IpNet>> = LazyLock::new(|| {
    vec![
        // IPv4 private ranges
        IpNet::from_str("127.0.0.0/8").unwrap(),     // Loopback
        IpNet::from_str("10.0.0.0/8").unwrap(),      // Class A private
        IpNet::from_str("172.16.0.0/12").unwrap(),   // Class B private
        IpNet::from_str("192.168.0.0/16").unwrap(),  // Class C private
        IpNet::from_str("169.254.0.0/16").unwrap(),  // Link-local
        IpNet::from_str("0.0.0.0/8").unwrap(),       // Current network
        IpNet::from_str("100.64.0.0/10").unwrap(),   // Shared address space (CGN)
        IpNet::from_str("192.0.0.0/24").unwrap(),    // IETF protocol assignments
        IpNet::from_str("192.0.2.0/24").unwrap(),    // TEST-NET-1
        IpNet::from_str("198.51.100.0/24").unwrap(), // TEST-NET-2
        IpNet::from_str("203.0.113.0/24").unwrap(),  // TEST-NET-3
        IpNet::from_str("224.0.0.0/4").unwrap(),     // Multicast
        IpNet::from_str("240.0.0.0/4").unwrap(),     // Reserved
        IpNet::from_str("255.255.255.255/32").unwrap(), // Broadcast
        // IPv6 private/reserved ranges
        IpNet::from_str("::1/128").unwrap(),                         // Loopback
        IpNet::from_str("fc00::/7").unwrap(),                        // Unique local
        IpNet::from_str("fe80::/10").unwrap(),                       // Link-local
        IpNet::from_str("::ffff:127.0.0.0/104").unwrap(),            // IPv4-mapped loopback
        IpNet::from_str("::ffff:10.0.0.0/104").unwrap(),             // IPv4-mapped private
        IpNet::from_str("::ffff:172.16.0.0/108").unwrap(),           // IPv4-mapped private
        IpNet::from_str("::ffff:192.168.0.0/112").unwrap(),          // IPv4-mapped private
        IpNet::from_str("::ffff:169.254.0.0/112").unwrap(),          // IPv4-mapped link-local
    ]
});

/// Validates URLs in configuration to prevent SSRF attacks.
///
/// Checks that URL configuration values do not point to private/reserved
/// IP ranges and optionally enforces HTTPS scheme.
///
/// By default, checks the `ssrf.allowed_urls` configuration field.
/// Use [`SsrfValidator::with_config_key`] to check a different key.
pub struct SsrfValidator {
    config_key: String,
    whitelist: Vec<String>,
    enforce_https: bool,
}

impl SsrfValidator {
    /// Create a new SSRF validator checking `ssrf.allowed_urls`.
    pub fn new() -> Self {
        Self {
            config_key: "ssrf.allowed_urls".to_string(),
            whitelist: Vec::new(),
            enforce_https: true,
        }
    }

    /// Create a validator that checks a specific configuration key.
    pub fn with_config_key(key: impl Into<String>) -> Self {
        Self {
            config_key: key.into(),
            whitelist: Vec::new(),
            enforce_https: true,
        }
    }

    /// Add URLs/patterns to the whitelist (these bypass SSRF checks).
    pub fn with_whitelist(mut self, urls: Vec<String>) -> Self {
        self.whitelist = urls;
        self
    }

    /// Set whether to enforce HTTPS scheme.
    pub fn with_enforce_https(mut self, enforce: bool) -> Self {
        self.enforce_https = enforce;
        self
    }

    /// Check if an IP address is in a blocked (private/reserved) range.
    fn is_blocked_ip(ip: &IpAddr) -> bool {
        BLOCKED_NETWORKS.iter().any(|net| net.contains(ip))
    }

    /// Check if a URL is in the whitelist.
    ///
    /// Parses both whitelist entries and the candidate URL, then compares
    /// scheme + host (exact) + path (prefix). This prevents bypasses like
    /// `https://127.0.0.1.evil.com` matching whitelist entry `https://127.0.0.1`.
    fn is_whitelisted(&self, url_str: &str) -> bool {
        let parsed = match url::Url::parse(url_str) {
            Ok(u) => u,
            Err(_) => return false,
        };

        self.whitelist.iter().any(|w| {
            let w_parsed = match url::Url::parse(w) {
                Ok(u) => u,
                Err(_) => return false,
            };
            // Scheme and host must match exactly
            if parsed.scheme() != w_parsed.scheme() {
                return false;
            }
            if parsed.host() != w_parsed.host() {
                return false;
            }
            // Path must be exact match or start with whitelist path + '/'
            // Note: url crate returns "/" for URLs without explicit path (e.g. "https://example.com")
            let w_path = w_parsed.path();
            let path = parsed.path();
            if w_path.is_empty() || w_path == "/" {
                // Whitelist entry has no specific path — any path is allowed
                true
            } else {
                path == w_path || path.starts_with(&format!("{w_path}/"))
            }
        })
    }
}

impl Default for SsrfValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl SecurityValidator for SsrfValidator {
    fn validate(&self, config: &dyn ConfigProvider) -> Result<(), Vec<SecurityViolation>> {
        let mut violations = Vec::new();

        let Some(value) = config.get_raw(&self.config_key) else {
            return Ok(());
        };

        let Some(urls_str) = value.as_str() else {
            return Ok(());
        };

        // Parse comma-separated URLs
        for url_entry in urls_str.split(',') {
            let url_str = url_entry.trim();
            if url_str.is_empty() {
                continue;
            }

            // Check whitelist first
            if self.is_whitelisted(url_str) {
                continue;
            }

            // Parse URL
            let parsed = match url::Url::parse(url_str) {
                Ok(u) => u,
                Err(_) => {
                    violations.push(SecurityViolation {
                        validator: self.name().to_string(),
                        field: Some(self.config_key.clone()),
                        message: format!("Invalid URL: {url_str}"),
                        severity: ViolationSeverity::Critical,
                    });
                    continue;
                }
            };

            // Check scheme
            if self.enforce_https && parsed.scheme() != "https" {
                violations.push(SecurityViolation {
                    validator: self.name().to_string(),
                    field: Some(self.config_key.clone()),
                    message: format!("URL uses non-HTTPS scheme '{}': {url_str}", parsed.scheme()),
                    severity: ViolationSeverity::Warning,
                });
            }

            // Check host against blocked IP ranges
            if let Some(host) = parsed.host_str() {
                // Strip brackets from IPv6 addresses (e.g. "[::1]" -> "::1")
                // Malformed brackets (e.g. "[::1") → report as violation, don't silently skip
                let (host, malformed_ipv6) = if let Some(inner) = host.strip_prefix('[') {
                    match inner.strip_suffix(']') {
                        Some(stripped) => (stripped, false),
                        None => (inner, true), // missing closing bracket
                    }
                } else {
                    (host, false)
                };

                if let Ok(ip) = IpAddr::from_str(host) {
                    if Self::is_blocked_ip(&ip) {
                        violations.push(SecurityViolation {
                            validator: self.name().to_string(),
                            field: Some(self.config_key.clone()),
                            message: format!(
                                "URL points to private/reserved IP range: {url_str}"
                            ),
                            severity: ViolationSeverity::Critical,
                        });
                    }
                } else if malformed_ipv6 {
                    // Malformed IPv6 address that can't be parsed — flag as warning
                    violations.push(SecurityViolation {
                        validator: self.name().to_string(),
                        field: Some(self.config_key.clone()),
                        message: format!(
                            "Malformed IPv6 address in URL (missing closing bracket): {url_str}"
                        ),
                        severity: ViolationSeverity::Warning,
                    });
                }
                // Note: DNS resolution is not performed here (sync, no IO).
                // Hostnames are checked only if they are literal IP addresses.
                // For hostname-based SSRF protection, use DNS resolution at runtime.
            }
        }

        if violations.is_empty() {
            Ok(())
        } else {
            Err(violations)
        }
    }

    fn name(&self) -> &'static str {
        "ssrf"
    }

    fn category(&self) -> &'static str {
        "network"
    }

    fn description(&self) -> &'static str {
        "Validates URLs do not point to private/reserved IP ranges (SSRF protection)"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{AnnotatedValue, ConfigValue, SourceId};
    use std::collections::HashMap;

    struct TestProvider(HashMap<String, AnnotatedValue>);

    impl TestProvider {
        fn new() -> Self {
            Self(HashMap::new())
        }

        fn with_value(mut self, key: &str, value: &str) -> Self {
            self.0.insert(
                key.to_string(),
                AnnotatedValue::new(ConfigValue::string(value), SourceId::new("test"), key),
            );
            self
        }
    }

    impl ConfigProvider for TestProvider {
        fn get_raw(&self, key: &str) -> Option<&AnnotatedValue> {
            self.0.get(key)
        }

        fn keys(&self) -> Vec<String> {
            self.0.keys().cloned().collect()
        }
    }

    #[test]
    fn test_no_config_skips() {
        let validator = SsrfValidator::new();
        let config = TestProvider::new();
        assert!(validator.validate(&config).is_ok());
    }

    #[test]
    fn test_valid_https_url() {
        let validator = SsrfValidator::new();
        let config = TestProvider::new()
            .with_value("ssrf.allowed_urls", "https://api.example.com/webhook");
        assert!(validator.validate(&config).is_ok());
    }

    #[test]
    fn test_loopback_blocked() {
        let validator = SsrfValidator::new();
        let config =
            TestProvider::new().with_value("ssrf.allowed_urls", "https://127.0.0.1/admin");
        let result = validator.validate(&config);
        assert!(result.is_err());
        let violations = result.unwrap_err();
        assert!(violations
            .iter()
            .any(|v| v.severity == ViolationSeverity::Critical && v.message.contains("private")));
    }

    #[test]
    fn test_private_10_blocked() {
        let validator = SsrfValidator::new();
        let config =
            TestProvider::new().with_value("ssrf.allowed_urls", "https://10.0.0.1/internal");
        let result = validator.validate(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_private_172_blocked() {
        let validator = SsrfValidator::new();
        let config = TestProvider::new()
            .with_value("ssrf.allowed_urls", "https://172.16.0.1/internal");
        assert!(validator.validate(&config).is_err());
    }

    #[test]
    fn test_private_192_blocked() {
        let validator = SsrfValidator::new();
        let config = TestProvider::new()
            .with_value("ssrf.allowed_urls", "https://192.168.1.1/router");
        assert!(validator.validate(&config).is_err());
    }

    #[test]
    fn test_ipv6_loopback_blocked() {
        let validator = SsrfValidator::new();
        let config =
            TestProvider::new().with_value("ssrf.allowed_urls", "https://[::1]/admin");
        assert!(validator.validate(&config).is_err());
    }

    #[test]
    fn test_non_https_warning() {
        let validator = SsrfValidator::new();
        let config = TestProvider::new()
            .with_value("ssrf.allowed_urls", "http://api.example.com/webhook");
        let result = validator.validate(&config);
        assert!(result.is_err());
        let violations = result.unwrap_err();
        assert!(violations
            .iter()
            .any(|v| v.severity == ViolationSeverity::Warning && v.message.contains("non-HTTPS")));
    }

    #[test]
    fn test_invalid_url_critical() {
        let validator = SsrfValidator::new();
        let config =
            TestProvider::new().with_value("ssrf.allowed_urls", "not-a-valid-url");
        let result = validator.validate(&config);
        assert!(result.is_err());
        let violations = result.unwrap_err();
        assert!(violations
            .iter()
            .any(|v| v.severity == ViolationSeverity::Critical && v.message.contains("Invalid URL")));
    }

    #[test]
    fn test_whitelist_exact_host_match() {
        let validator = SsrfValidator::new()
            .with_whitelist(vec!["https://127.0.0.1".to_string()]);
        // Exact host match with subpath — should be whitelisted
        assert!(validator.is_whitelisted("https://127.0.0.1/admin"));
        // Exact host, no subpath
        assert!(validator.is_whitelisted("https://127.0.0.1"));
    }

    #[test]
    fn test_whitelist_prefix_bypass_prevented() {
        let validator = SsrfValidator::new()
            .with_whitelist(vec!["https://127.0.0.1".to_string()]);
        // Different host (127.0.0.1.evil.com) must NOT match whitelist entry for 127.0.0.1
        assert!(!validator.is_whitelisted("https://127.0.0.1.evil.com/admin"));
        // Different scheme must NOT match
        assert!(!validator.is_whitelisted("http://127.0.0.1/admin"));
        // Completely different host must NOT match
        assert!(!validator.is_whitelisted("https://evil.com/"));
    }

    #[test]
    fn test_multiple_urls() {
        let validator = SsrfValidator::new();
        let config = TestProvider::new().with_value(
            "ssrf.allowed_urls",
            "https://api.example.com,https://10.0.0.1/bad",
        );
        let result = validator.validate(&config);
        assert!(result.is_err());
        let violations = result.unwrap_err();
        // Should have at least 1 critical for the private IP
        assert!(violations
            .iter()
            .any(|v| v.severity == ViolationSeverity::Critical));
    }

    #[test]
    fn test_custom_config_key() {
        let validator = SsrfValidator::with_config_key("webhooks.urls");
        let config = TestProvider::new()
            .with_value("webhooks.urls", "https://127.0.0.1/hook");
        assert!(validator.validate(&config).is_err());
    }

    #[test]
    fn test_enforce_https_disabled() {
        let validator = SsrfValidator::new().with_enforce_https(false);
        let config = TestProvider::new()
            .with_value("ssrf.allowed_urls", "http://api.example.com/webhook");
        assert!(validator.validate(&config).is_ok());
    }

    #[test]
    fn test_trait_properties() {
        let validator = SsrfValidator::new();
        assert_eq!(validator.name(), "ssrf");
        assert_eq!(validator.category(), "network");
    }
}
