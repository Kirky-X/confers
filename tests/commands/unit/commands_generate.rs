// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! 单元测试：Generate命令功能
//!
//! 测试GenerateCommand的各种功能，包括模板生成、级别解析等

#[cfg(test)]
mod generate_level_tests {
    use super::super::*;

    /// 测试解析minimal级别
    #[test]
    fn test_parse_minimal_level() {
        assert_eq!(GenerateLevel::parse("minimal"), GenerateLevel::Minimal);
        assert_eq!(GenerateLevel::parse("min"), GenerateLevel::Minimal);
        assert_eq!(GenerateLevel::parse("MINIMAL"), GenerateLevel::Minimal);
    }

    /// 测试解析documentation级别
    #[test]
    fn test_parse_documentation_level() {
        assert_eq!(
            GenerateLevel::parse("documentation"),
            GenerateLevel::Documentation
        );
        assert_eq!(GenerateLevel::parse("doc"), GenerateLevel::Documentation);
        assert_eq!(GenerateLevel::parse("DOC"), GenerateLevel::Documentation);
    }

    /// 测试解析full级别
    #[test]
    fn test_parse_full_level() {
        assert_eq!(GenerateLevel::parse("full"), GenerateLevel::Full);
        assert_eq!(GenerateLevel::parse("FULL"), GenerateLevel::Full);
    }

    /// 测试解析无效级别（默认full）
    #[test]
    fn test_parse_invalid_level() {
        assert_eq!(GenerateLevel::parse("invalid"), GenerateLevel::Full);
        assert_eq!(GenerateLevel::parse("unknown"), GenerateLevel::Full);
        assert_eq!(GenerateLevel::parse(""), GenerateLevel::Full);
    }

    /// 测试级别相等性
    #[test]
    fn test_level_equality() {
        assert_eq!(GenerateLevel::Minimal, GenerateLevel::Minimal);
        assert_eq!(GenerateLevel::Full, GenerateLevel::Full);
        assert_eq!(GenerateLevel::Documentation, GenerateLevel::Documentation);

        assert_ne!(GenerateLevel::Minimal, GenerateLevel::Full);
        assert_ne!(GenerateLevel::Full, GenerateLevel::Documentation);
    }
}

#[cfg(test)]
mod template_generation_tests {
    use super::super::*;
    use serde::{Deserialize, Serialize};
    use std::io::Write;
    use tempfile::NamedTempFile;

    /// 测试配置生成基本功能
    #[test]
    fn test_generate_basic_config() {
        #[derive(Debug, Clone, Serialize, Deserialize, Default)]
        struct TestConfig {
            name: String,
            port: u16,
            enabled: bool,
        }

        let output = NamedTempFile::new().unwrap();
        let output_path = output.path().to_string_lossy().into_owned();

        let result = GenerateCommand::generate_config(
            Some(&output_path),
            "full",
            Some(&"TestConfig".to_string()),
            "toml",
        );

        assert!(result.is_ok());

        let content = std::fs::read_to_string(&output_path).unwrap();
        assert!(content.contains("name"));
        assert!(content.contains("port"));
        assert!(content.contains("enabled"));
    }

    /// 测试生成最小配置
    #[test]
    fn test_generate_minimal_config() {
        #[derive(Debug, Clone, Serialize, Deserialize, Default)]
        struct MinimalConfig {
            value: i32,
        }

        let output = NamedTempFile::new().unwrap();
        let output_path = output.path().to_string_lossy().into_owned();

        let result = GenerateCommand::generate_config(
            Some(&output_path),
            "minimal",
            Some(&"MinimalConfig".to_string()),
            "toml",
        );

        assert!(result.is_ok());

        let content = std::fs::read_to_string(&output_path).unwrap();
        assert!(content.contains("value"));
    }

    /// 测试生成JSON格式配置
    #[test]
    fn test_generate_json_config() {
        #[derive(Debug, Clone, Serialize, Deserialize, Default)]
        struct JsonConfig {
            data: String,
            count: usize,
        }

        let output = NamedTempFile::new().unwrap();
        let output_path = output.path().to_string_lossy().into_owned();

        let result = GenerateCommand::generate_config(
            Some(&output_path),
            "full",
            Some(&"JsonConfig".to_string()),
            "json",
        );

        assert!(result.is_ok());

        let content = std::fs::read_to_string(&output_path).unwrap();
        assert!(content.contains("data"));
        assert!(content.contains("count"));
    }

    /// 测试生成YAML格式配置
    #[test]
    fn test_generate_yaml_config() {
        #[derive(Debug, Clone, Serialize, Deserialize, Default)]
        struct YamlConfig {
            setting: String,
            level: i32,
        }

        let output = NamedTempFile::new().unwrap();
        let output_path = output.path().to_string_lossy().into_owned();

        let result = GenerateCommand::generate_config(
            Some(&output_path),
            "full",
            Some(&"YamlConfig".to_string()),
            "yaml",
        );

        assert!(result.is_ok());

        let content = std::fs::read_to_string(&output_path).unwrap();
        assert!(content.contains("setting"));
        assert!(content.contains("level"));
    }

    /// 测试输出到标准输出
    #[test]
    fn test_generate_to_stdout() {
        #[derive(Debug, Clone, Serialize, Deserialize, Default)]
        struct StdoutConfig {
            test: String,
        }

        // None表示输出到stdout
        let result = GenerateCommand::generate_config(
            None,
            "full",
            Some(&"StdoutConfig".to_string()),
            "toml",
        );

        assert!(result.is_ok());
    }

    /// 测试无效格式处理
    #[test]
    fn test_invalid_format_handling() {
        #[derive(Debug, Clone, Serialize, Deserialize, Default)]
        struct FormatConfig {
            value: String,
        }

        let output = NamedTempFile::new().unwrap();
        let output_path = output.path().to_string_lossy().into_owned();

        // 使用不支持的格式应该返回错误
        let result = GenerateCommand::generate_config(
            Some(&output_path),
            "full",
            Some(&"FormatConfig".to_string()),
            "unsupported_format",
        );

        // 应该返回错误，因为格式不支持
        assert!(result.is_err());
    }
}

#[cfg(test)]
mod generate_integration_tests {
    use super::super::*;
    use tempfile::TempDir;

    /// 测试生成临时目录配置
    #[test]
    fn test_generate_to_temp_directory() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("test_config.toml");
        let output_str = output_path.to_string_lossy().into_owned();

        #[derive(Debug, Clone, Serialize, Deserialize, Default)]
        struct TempConfig {
            name: String,
        }

        let result = GenerateCommand::generate_config(
            Some(&output_str),
            "full",
            Some(&"TempConfig".to_string()),
            "toml",
        );

        assert!(result.is_ok());
        assert!(output_path.exists());
    }

    /// 测试生成多个配置
    #[test]
    fn test_generate_multiple_configs() {
        let temp_dir = TempDir::new().unwrap();

        #[derive(Debug, Clone, Serialize, Deserialize, Default)]
        struct Config1 {
            field1: String,
        }

        #[derive(Debug, Clone, Serialize, Deserialize, Default)]
        struct Config2 {
            field2: i32,
        }

        let path1 = temp_dir.path().join("config1.toml");
        let path2 = temp_dir.path().join("config2.toml");

        let result1 = GenerateCommand::generate_config(
            Some(&path1.to_string_lossy().into_owned()),
            "full",
            Some(&"Config1".to_string()),
            "toml",
        );

        let result2 = GenerateCommand::generate_config(
            Some(&path2.to_string_lossy().into_owned()),
            "minimal",
            Some(&"Config2".to_string()),
            "json",
        );

        assert!(result1.is_ok());
        assert!(result2.is_ok());
        assert!(path1.exists());
        assert!(path2.exists());
    }
}
