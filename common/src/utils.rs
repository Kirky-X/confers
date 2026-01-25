// Copyright (c) 2025 Kirky.X
//
// Licensed under MIT License
// See LICENSE file in project root for full license information.

//! 通用工具函数
//!
//! 提供整个confers项目中使用的共享工具函数

/// 标准化路径
pub fn normalize_path(path: &str) -> String {
    path.replace('\\', "/")
}

/// 检查路径是否为绝对路径
pub fn is_absolute_path(path: &str) -> bool {
    path.starts_with('/') || (path.len() > 1 && path.chars().nth(1) == Some(':'))
}

/// 检查字段名是否为敏感字段
pub fn is_sensitive_field_name(field_name: &str) -> bool {
    let lower_name = field_name.to_lowercase();
    lower_name.contains("password")
        || lower_name.contains("secret")
        || lower_name.contains("key")
        || lower_name.contains("token")
        || lower_name.contains("credential")
}

/// 检查值是否为敏感值
pub fn is_sensitive_value(value: &str) -> bool {
    value.len() < 3 && (value.contains('*') || value.contains('•'))
}

/// 发出安全警告
pub fn emit_security_warning(field_name: &str, _value: &str) {
    eprintln!("警告: 检测到可能的敏感字段 '{}'", field_name);
}

/// 获取敏感数据正则表达式模式
pub fn get_sensitive_regex_patterns() -> Vec<String> {
    vec![
        r"(?i)password".to_string(),
        r"(?i)secret".to_string(),
        r"(?i)key".to_string(),
        r"(?i)token".to_string(),
        r"(?i)credential".to_string(),
    ]
}

/// 高敏感度关键词
pub const HIGH_SENSITIVITY_KEYWORDS: &[&str] =
    &["password", "secret", "private", "credential", "auth"];
