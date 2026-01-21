// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

use std::collections::HashMap;
use std::sync::{LazyLock, RwLock};

type CustomValidatorFn = Box<dyn Fn(&str) -> bool + Send + Sync>;

static CUSTOM_VALIDATORS: LazyLock<RwLock<HashMap<String, CustomValidatorFn>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

pub trait CustomValidator: Send + Sync + 'static {
    fn name(&self) -> &'static str;
    fn validate(&self, value: &str) -> bool;
}

pub fn is_email(value: &str) -> bool {
    if value.is_empty() {
        return false;
    }

    let parts: Vec<&str> = value.split('@').collect();
    if parts.len() != 2 {
        return false;
    }

    let local_part = parts[0];
    let domain_part = parts[1];

    if local_part.is_empty() || domain_part.is_empty() {
        return false;
    }

    if local_part.len() > 64 || domain_part.len() > 255 {
        return false;
    }

    let domain_parts: Vec<&str> = domain_part.split('.').collect();
    if domain_parts.len() < 2 {
        return false;
    }

    if domain_parts.iter().any(|p| p.is_empty()) {
        return false;
    }

    if domain_parts.last().is_some_and(|p| p.len() < 2) {
        return false;
    }

    let valid_local_chars: std::collections::HashSet<char> =
        "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789._-+"
            .chars()
            .collect();

    if !local_part.chars().all(|c| valid_local_chars.contains(&c)) {
        return false;
    }

    let valid_domain_chars: std::collections::HashSet<char> =
        "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-."
            .chars()
            .collect();

    if !domain_part.chars().all(|c| valid_domain_chars.contains(&c)) {
        return false;
    }

    if domain_part.starts_with('-') || domain_part.ends_with('-') {
        return false;
    }

    true
}

pub fn is_url(value: &str) -> bool {
    if value.is_empty() {
        return false;
    }

    let valid_schemes = ["http://", "https://"];

    let mut has_valid_scheme = false;
    for scheme in &valid_schemes {
        if value.to_lowercase().starts_with(scheme) {
            has_valid_scheme = true;
            break;
        }
    }

    if !has_valid_scheme {
        return false;
    }

    if let Some(path_start) = value.find("://") {
        let authority = &value[path_start + 3..];
        let authority_end = authority.find('/').unwrap_or(authority.len());
        let authority = &authority[..authority_end];

        if authority.is_empty() {
            return false;
        }

        let user_info_end = authority.find('@').unwrap_or(authority.len());
        let host_port = if user_info_end < authority.len() {
            &authority[user_info_end + 1..]
        } else {
            authority
        };

        let host = if let Some(port_start) = host_port.find(':') {
            &host_port[..port_start]
        } else {
            host_port
        };

        if host.is_empty() {
            return false;
        }

        let valid_host_chars: std::collections::HashSet<char> =
            "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-."
                .chars()
                .collect();

        if !host.chars().all(|c| valid_host_chars.contains(&c)) {
            return false;
        }

        if host.starts_with('-') || host.ends_with('-') {
            return false;
        }

        if host.starts_with("xn--") {
            return true;
        }

        let labels: Vec<&str> = host.split('.').collect();
        if labels.len() < 2 {
            return false;
        }

        for label in &labels {
            if label.is_empty() || label.len() > 63 {
                return false;
            }
        }
    }

    true
}

pub fn register_custom_validator(
    name: &str,
    validator: impl Fn(&str) -> bool + Send + Sync + 'static,
) -> Result<(), String> {
    let mut validators = CUSTOM_VALIDATORS
        .write()
        .map_err(|e| format!("Failed to acquire write lock: {}", e))?;
    validators.insert(name.to_string(), Box::new(validator));
    Ok(())
}

pub fn unregister_custom_validator(name: &str) -> Result<(), String> {
    let mut validators = CUSTOM_VALIDATORS
        .write()
        .map_err(|e| format!("Failed to acquire write lock: {}", e))?;
    validators.remove(name);
    Ok(())
}

pub fn validate_with_custom(name: &str, value: &str) -> Result<bool, String> {
    let validators = CUSTOM_VALIDATORS
        .read()
        .map_err(|e| format!("Failed to acquire read lock: {}", e))?;
    if let Some(validator) = validators.get(name) {
        Ok(validator(value))
    } else {
        Ok(false)
    }
}

pub fn list_custom_validators() -> Result<Vec<String>, String> {
    let validators = CUSTOM_VALIDATORS
        .read()
        .map_err(|e| format!("Failed to acquire read lock: {}", e))?;
    Ok(validators.keys().cloned().collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_emails() {
        assert!(is_email("user@example.com"));
        assert!(is_email("user.name@example.com"));
        assert!(is_email("user+tag@example.com"));
        assert!(is_email("user@sub.example.com"));
    }

    #[test]
    fn test_invalid_emails() {
        assert!(!is_email(""));
        assert!(!is_email("invalid"));
        assert!(!is_email("@example.com"));
        assert!(!is_email("user@"));
        assert!(!is_email("user@example"));
    }

    #[test]
    fn test_valid_urls() {
        assert!(is_url("https://example.com"));
        assert!(is_url("http://example.com/path"));
        assert!(is_url("https://sub.example.com/path/to/resource"));
        assert!(is_url("https://example.com:8080/path"));
    }

    #[test]
    fn test_invalid_urls() {
        assert!(!is_url(""));
        assert!(!is_url("not-a-url"));
        assert!(!is_url("ftp://example.com"));
    }
}
