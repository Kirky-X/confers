// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! 单元测试：安全字符串功能
//!
//! 测试SecureString的各种功能，包括创建、安全比较、敏感度级别等

#[cfg(test)]
mod secure_string_creation_tests {
    use super::super::*;
    use std::thread;

    /// 测试创建基本安全字符串
    #[test]
    fn test_secure_string_creation() {
        let secret = SecureString::from("my-secret-password");
        assert_eq!(secret.as_str(), "my-secret-password");
    }

    /// 测试创建带敏感度级别的安全字符串
    #[test]
    fn test_secure_string_with_sensitivity() {
        let secret = SecureString::new("data", SensitivityLevel::Low);
        assert_eq!(secret.as_str(), "data");

        let secret = SecureString::new("token", SensitivityLevel::High);
        assert_eq!(secret.as_str(), "token");

        let secret = SecureString::new("password", SensitivityLevel::Critical);
        assert_eq!(secret.as_str(), "password");
    }

    /// 测试从字节数组创建安全字符串
    #[test]
    fn test_secure_string_from_bytes() {
        let data = b"binary-data-12345";
        let secret = SecureString::from_bytes(data.to_vec(), SensitivityLevel::Medium);
        assert_eq!(secret.as_bytes(), data);
    }

    /// 测试安全字符串计数器
    #[test]
    fn test_secure_string_counters() {
        // 重置计数器
        reset_secure_string_counters();

        let initial_allocated = allocated_secure_strings();
        let initial_deallocated = deallocated_secure_strings();

        // 创建一些安全字符串
        let _secret1 = SecureString::from("password1");
        let _secret2 = SecureString::from("password2");

        assert_eq!(allocated_secure_strings(), initial_allocated + 2);

        // 释放后计数器应该增加（因为SecureString在离开作用域时释放）
    }

    /// 测试多线程环境下的安全字符串
    #[test]
    fn test_secure_string_thread_safety() {
        let handles: Vec<_> = (0..4)
            .map(|i| {
                thread::spawn(move || {
                    let secret = SecureString::from(&format!("secret-thread-{}", i));
                    secret.as_str().to_string()
                })
            })
            .collect();

        let results: Vec<String> = handles.into_iter().map(|h| h.join().unwrap()).collect();

        assert_eq!(results.len(), 4);
        assert!(results.contains(&"secret-thread-0".to_string()));
        assert!(results.contains(&"secret-thread-1".to_string()));
    }
}

#[cfg(test)]
mod secure_string_comparison_tests {
    use super::super::*;

    /// 测试安全字符串比较 - 相等
    #[test]
    fn test_secure_string_compare_equal() {
        let secret = SecureString::from("password123");
        assert!(secret.compare("password123").is_ok());
    }

    /// 测试安全字符串比较 - 不相等
    #[test]
    fn test_secure_string_compare_not_equal() {
        let secret = SecureString::from("password123");
        assert!(secret.compare("wrongpassword").is_err());
    }

    /// 测试安全字符串比较 - 空字符串
    #[test]
    fn test_secure_string_compare_empty() {
        let secret = SecureString::from("");
        assert!(secret.compare("").is_ok());
        assert!(secret.compare("non-empty").is_err());
    }

    /// 测试安全字符串比较 - 特殊字符
    #[test]
    fn test_secure_string_compare_special_chars() {
        let secret = SecureString::from("p@$$w0rd!#$");
        assert!(secret.compare("p@$$w0rd!#$").is_ok());
        assert!(secret.compare("different").is_err());
    }

    /// 测试安全字符串比较 - Unicode字符
    #[test]
    fn test_secure_string_compare_unicode() {
        let secret = SecureString::from("密码123");
        assert!(secret.compare("密码123").is_ok());
        assert!(secret.compare("wrong").is_err());
    }

    /// 测试安全字符串比较 - 长字符串
    #[test]
    fn test_secure_string_compare_long() {
        let long_string = "a".repeat(1000);
        let secret = SecureString::from(&long_string);
        assert!(secret.compare(&long_string).is_ok());
        assert!(secret.compare(&"b".repeat(1000)).is_err());
    }
}

#[cfg(test)]
mod secure_string_sensitivity_tests {
    use super::super::*;

    /// 测试敏感度级别
    #[test]
    fn test_sensitivity_levels() {
        let low = SensitivityLevel::Low;
        let medium = SensitivityLevel::Medium;
        let high = SensitivityLevel::High;
        let critical = SensitivityLevel::Critical;

        assert!(!low.is_critical_or_high());
        assert!(!medium.is_critical_or_high());
        assert!(high.is_critical_or_high());
        assert!(critical.is_critical_or_high());
    }

    /// 测试不同敏感度级别的字符串
    #[test]
    fn test_different_sensitivity_levels() {
        let low = SecureString::new("internal_data", SensitivityLevel::Low);
        let medium = SecureString::new("user_data", SensitivityLevel::Medium);
        let high = SecureString::new("api_token", SensitivityLevel::High);
        let critical = SecureString::new("master_password", SensitivityLevel::Critical);

        assert_eq!(low.as_str(), "internal_data");
        assert_eq!(medium.as_str(), "user_data");
        assert_eq!(high.as_str(), "api_token");
        assert_eq!(critical.as_str(), "master_password");
    }
}

#[cfg(test)]
mod secure_string_display_tests {
    use super::super::*;

    /// 测试安全字符串显示名称
    #[test]
    fn test_secure_string_display_name() {
        let secret = SecureString::from("password123");
        assert_eq!(secret.display_name(), "password123");
    }

    /// 测试字节数组的安全字符串显示名称
    #[test]
    fn test_bytes_secure_string_display_name() {
        let secret = SecureString::from_bytes(b"binary".to_vec(), SensitivityLevel::Medium);
        assert_eq!(secret.display_name(), "[binary data]");
    }
}

#[cfg(test)]
mod secure_string_conversion_tests {
    use super::super::*;

    /// 测试转换为普通字符串
    #[test]
    fn test_to_plain_string() {
        let secret = SecureString::from("password123");
        let plain = secret.to_plain_string();
        assert_eq!(plain, "password123");
    }

    /// 测试获取字符串切片
    #[test]
    fn test_as_str() {
        let secret = SecureString::from("hello world");
        assert_eq!(secret.as_str(), "hello world");
    }

    /// 测试获取字节切片
    #[test]
    fn test_as_bytes() {
        let secret = SecureString::from("test");
        assert_eq!(secret.as_bytes(), b"test");
    }

    /// 测试UTF-8有效性
    #[test]
    fn test_utf8_validity() {
        let secret = SecureString::from("Hello 世界 🌍");
        assert_eq!(secret.as_str(), "Hello 世界 🌍");
    }
}
