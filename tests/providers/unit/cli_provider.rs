// Copyright (c) 2025 Kirky.X
//
// Licensed under MIT License
// See LICENSE file in project root for full license information.

//! 单元测试：CLI提供者
//!
//! 测试CLI配置提供者的功能，包括参数解析、配置加载和错误处理

#[cfg(test)]
mod cli_provider_tests {
    use super::super::*;
    use std::collections::HashMap;

    /// 测试CLI提供者基本功能
    #[test]
    fn test_cli_provider_basic() {
        let args = vec![
            ("config".to_string(), "config.toml".to_string()),
            ("port".to_string(), "8080".to_string()),
        ];

        // 模拟CLI提供者创建
        let provider = crate::providers::CliProvider::new(args);

        assert!(provider.is_available());
    }

    /// 测试CLI参数解析
    #[test]
    fn test_cli_args_parsing() {
        let args = vec![
            ("debug".to_string(), "true".to_string()),
            ("log_level".to_string(), "info".to_string()),
        ];

        let provider = crate::providers::CliProvider::new(args);

        // 测试获取配置值
        if let Some(value) = provider.get("debug") {
            assert_eq!(value, "true");
        } else {
            panic!("Expected debug value");
        }
    }

    /// 测试CLI提供者优先级
    #[test]
    fn test_cli_provider_priority() {
        let args = vec![("port".to_string(), "3000".to_string())];

        let provider = crate::providers::CliProvider::new(args);

        // CLI提供者应该具有高优先级
        assert!(provider.priority() > 100);
    }

    /// 测试空CLI参数
    #[test]
    fn test_empty_cli_args() {
        let args: Vec<(String, String)> = vec![];
        let provider = crate::providers::CliProvider::new(args);

        assert!(provider.is_available());

        // 空参数应该返回None
        assert_eq!(provider.get("nonexistent"), None);
    }

    /// 测试CLI提供者名称
    #[test]
    fn test_cli_provider_name() {
        let args = vec![];
        let provider = crate::providers::CliProvider::new(args);

        assert_eq!(provider.name(), "cli");
    }

    /// 测试CLI参数覆盖
    #[test]
    fn test_cli_override_behavior() {
        let args = vec![("port".to_string(), "9999".to_string())];

        let provider = crate::providers::CliProvider::new(args);

        // 测试覆盖行为
        if let Some(port) = provider.get("port") {
            assert_eq!(port, "9999");
        } else {
            panic!("Expected port override");
        }
    }
}
