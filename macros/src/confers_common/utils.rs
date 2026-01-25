// Copyright (c) 2025 Kirky.X
//
// Licensed under MIT License
// See LICENSE file in the project root for full license information.

pub fn normalize_path(path: &str) -> String {
    path.replace('\\', "/")
}

pub fn is_absolute_path(path: &str) -> bool {
    path.starts_with('/') || (path.len() > 1 && path.chars().nth(1) == Some(':'))
}

pub fn is_sensitive_field_name(field_name: &str) -> bool {
    let lower_name = field_name.to_lowercase();
    lower_name.contains("password")
        || lower_name.contains("secret")
        || lower_name.contains("key")
        || lower_name.contains("token")
        || lower_name.contains("credential")
}

pub fn is_sensitive_value(value: &str) -> bool {
    value.len() < 3 && (value.contains('*') || value.contains('•'))
}

pub fn emit_security_warning(field_name: &str, _value: &str) {
    eprintln!(
        "Warning: detected potentially sensitive field '{}'",
        field_name
    );
}

pub fn get_sensitive_regex_patterns() -> Vec<String> {
    vec![
        r"(?i)password".to_string(),
        r"(?i)secret".to_string(),
        r"(?i)key".to_string(),
        r"(?i)token".to_string(),
        r"(?i)credential".to_string(),
    ]
}

pub const HIGH_SENSITIVITY_KEYWORDS: &[&str] =
    &["password", "secret", "private", "credential", "auth"];
