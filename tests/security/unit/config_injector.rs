// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! 单元测试：配置注入功能
//!
//! 测试ConfigInjector的各种功能，包括配置注入、获取、验证等

#[cfg(test)]
mod config_injector_creation_tests {
    use super::super::*;

    /// 测试创建配置注入器
    #[test]
    fn test_config_injector_creation() {
        let injector = ConfigInjector::new();
        assert!(injector.get("TEST_KEY").is_none());
    }

    /// 测试创建禁用的速率限制器
    #[test]
    fn test_disabled_rate_limiter() {
        let limiter = InjectionRateLimiter::disabled();
        assert!(limiter.check_rate_limit().is_ok());
    }

    /// 测试创建自定义速率限制器
    #[test]
    fn test_custom_rate_limiter() {
        let limiter = InjectionRateLimiter::with_limits(50, 30);
        assert_eq!(limiter.max_requests, 50);
        assert_eq!(limiter.window_seconds, 30);
    }
}

#[cfg(test)]
mod config_injection_tests {
    use super::super::*;

    /// 测试注入有效配置
    #[test]
    fn test_inject_valid_config() {
        let injector = ConfigInjector::new();

        let result = injector.inject("APP_NAME", "test_application");
        assert!(result.is_ok());

        let value = injector.get("APP_NAME");
        assert!(value.is_some());
        assert_eq!(value.unwrap(), "test_application");
    }

    /// 测试注入多个配置值
    #[test]
    fn test_inject_multiple_configs() {
        let injector = ConfigInjector::new();

        injector
            .inject("DATABASE_URL", "postgresql://localhost/db")
            .unwrap();
        injector.inject("API_KEY", "sk-123456789").unwrap();
        injector.inject("DEBUG", "true").unwrap();

        assert_eq!(
            injector.get("DATABASE_URL").unwrap(),
            "postgresql://localhost/db"
        );
        assert_eq!(injector.get("API_KEY").unwrap(), "sk-123456789");
        assert_eq!(injector.get("DEBUG").unwrap(), "true");
    }

    /// 测试更新已存在的配置
    #[test]
    fn test_update_existing_config() {
        let injector = ConfigInjector::new();

        injector.inject("VERSION", "1.0.0").unwrap();
        assert_eq!(injector.get("VERSION").unwrap(), "1.0.0");

        injector.inject("VERSION", "2.0.0").unwrap();
        assert_eq!(injector.get("VERSION").unwrap(), "2.0.0");
    }

    /// 测试获取不存在的配置
    #[test]
    fn test_get_nonexistent_config() {
        let injector = ConfigInjector::new();
        assert!(injector.get("NONEXISTENT").is_none());
    }

    /// 测试获取所有配置
    #[test]
    fn test_get_all_configs() {
        let injector = ConfigInjector::new();

        injector.inject("KEY1", "value1").unwrap();
        injector.inject("KEY2", "value2").unwrap();
        injector.inject("KEY3", "value3").unwrap();

        let all = injector.get_all();
        assert_eq!(all.len(), 3);
        assert!(all.contains(&"KEY1".to_string()));
        assert!(all.contains(&"KEY2".to_string()));
        assert!(all.contains(&"KEY3".to_string()));
    }

    /// 测试清空所有配置
    #[test]
    fn test_clear_all_configs() {
        let injector = ConfigInjector::new();

        injector.inject("KEY1", "value1").unwrap();
        injector.inject("KEY2", "value2").unwrap();

        assert_eq!(injector.get_all().len(), 2);

        injector.clear_all();

        assert!(injector.get("KEY1").is_none());
        assert!(injector.get("KEY2").is_none());
    }
}

#[cfg(test)]
mod config_validation_tests {
    use super::super::*;

    /// 测试注入敏感配置
    #[test]
    fn test_inject_sensitive_config() {
        let injector = ConfigInjector::new();

        // 密码应该被标记为敏感
        let result = injector.inject("DB_PASSWORD", "secret123");
        assert!(result.is_ok());
    }

    /// 测试注入包含敏感关键词的配置
    #[test]
    fn test_inject_config_with_sensitive_keywords() {
        let injector = ConfigInjector::new();

        // 包含api_key的配置
        let result = injector.inject("OLD_API_KEY", "sk-123456789");
        assert!(result.is_ok());
    }

    /// 测试注入空值配置
    #[test]
    fn test_inject_empty_value() {
        let injector = ConfigInjector::new();

        let result = injector.inject("EMPTY_KEY", "");
        assert!(result.is_ok());

        let value = injector.get("EMPTY_KEY");
        assert!(value.is_some());
        assert_eq!(value.unwrap(), "");
    }

    /// 测试注入特殊字符配置
    #[test]
    fn test_inject_special_chars() {
        let injector = ConfigInjector::new();

        let result = injector.inject("SPECIAL_CHARS", "value with spaces and-dash_underscore");
        assert!(result.is_ok());
        assert_eq!(
            injector.get("SPECIAL_CHARS").unwrap(),
            "value with spaces and-dash_underscore"
        );
    }
}

#[cfg(test)]
mod injection_rate_limiter_tests {
    use super::super::*;
    use std::thread;
    use std::time::Duration;

    /// 测试速率限制器基本功能
    #[test]
    fn test_rate_limiter_basic() {
        let limiter = InjectionRateLimiter::with_limits(5, 60);

        // 前5次应该成功
        for i in 0..5 {
            let result = limiter.check_rate_limit();
            assert!(result.is_ok(), "Request {} should be allowed", i);
        }
    }

    /// 测试速率限制超出限制
    #[test]
    fn test_rate_limiter_exceeded() {
        let limiter = InjectionRateLimiter::with_limits(3, 60);

        // 前3次应该成功
        for _ in 0..3 {
            assert!(limiter.check_rate_limit().is_ok());
        }

        // 第4次应该被限制
        let result = limiter.check_rate_limit();
        assert!(result.is_err());
    }

    /// 测试速率限制器窗口重置
    #[test]
    fn test_rate_limiter_window_reset() {
        let limiter = InjectionRateLimiter::with_limits(2, 1); // 1秒窗口

        // 使用完配额
        assert!(limiter.check_rate_limit().is_ok());
        assert!(limiter.check_rate_limit().is_ok());
        assert!(limiter.check_rate_limit().is_err());

        // 等待窗口重置
        thread::sleep(Duration::from_secs(2));

        // 应该再次允许请求
        assert!(limiter.check_rate_limit().is_ok());
    }
}

#[cfg(test)]
mod injection_history_tests {
    use super::super::*;

    /// 测试注入历史记录
    #[test]
    fn test_injection_history() {
        let injector = ConfigInjector::new();

        injector.inject("KEY1", "value1").unwrap();
        injector.inject("KEY2", "value2").unwrap();

        let history = injector.get_injection_history();
        assert!(history.len() >= 2);
    }

    /// 测试敏感配置的注入历史
    #[test]
    fn test_sensitive_config_history() {
        let injector = ConfigInjector::new();

        injector.inject("SECRET_KEY", "secret-value").unwrap();

        let history = injector.get_injection_history();
        // 历史中应该包含敏感配置的信息，但不包含实际值
        let sensitive_entries: Vec<_> = history
            .iter()
            .filter(|(name, _, _)| *name == "SECRET_KEY")
            .collect();

        assert!(!sensitive_entries.is_empty());
    }
}
