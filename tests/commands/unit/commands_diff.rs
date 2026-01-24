// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! 单元测试：Diff命令功能
//!
//! 测试DiffCommand的各种功能，包括配置比较、格式渲染等

#[cfg(test)]
mod diff_format_tests {
    use super::super::*;

    /// 测试解析unified格式
    #[test]
    fn test_parse_unified_format() {
        assert_eq!(
            DiffFormat::from_str("unified").unwrap(),
            DiffFormat::Unified
        );
        assert_eq!(
            DiffFormat::from_str("UNIFIED").unwrap(),
            DiffFormat::Unified
        );
    }

    /// 测试解析context格式
    #[test]
    fn test_parse_context_format() {
        assert_eq!(
            DiffFormat::from_str("context").unwrap(),
            DiffFormat::Context
        );
        assert_eq!(
            DiffFormat::from_str("CONTEXT").unwrap(),
            DiffFormat::Context
        );
    }

    /// 测试解析normal格式
    #[test]
    fn test_parse_normal_format() {
        assert_eq!(DiffFormat::from_str("normal").unwrap(), DiffFormat::Normal);
    }

    /// 测试解析side-by-side格式
    #[test]
    fn test_parse_side_by_side_format() {
        assert_eq!(
            DiffFormat::from_str("side-by-side").unwrap(),
            DiffFormat::SideBySide
        );
        assert_eq!(
            DiffFormat::from_str("sidebyside").unwrap(),
            DiffFormat::SideBySide
        );
        assert_eq!(
            DiffFormat::from_str("side").unwrap(),
            DiffFormat::SideBySide
        );
    }

    /// 测试解析strict格式
    #[test]
    fn test_parse_strict_format() {
        assert_eq!(DiffFormat::from_str("strict").unwrap(), DiffFormat::Strict);
    }

    /// 测试解析无效格式
    #[test]
    fn test_parse_invalid_format() {
        assert!(DiffFormat::from_str("invalid").is_err());
        assert!(DiffFormat::from_str("unknown").is_err());
    }
}

#[cfg(test)]
mod diff_options_tests {
    use super::super::*;

    /// 测试默认选项
    #[test]
    fn test_default_options() {
        let options = DiffOptions::default();

        assert_eq!(options.format, DiffFormat::Unified);
        assert_eq!(options.context_lines, 3);
        assert!(!options.show_line_numbers);
        assert!(!options.ignore_whitespace);
        assert!(!options.case_insensitive);
        assert!(!options.strict);
        assert!(options.output.is_none());
    }

    /// 测试自定义选项
    #[test]
    fn test_custom_options() {
        let options = DiffOptions {
            format: DiffFormat::SideBySide,
            context_lines: 5,
            show_line_numbers: true,
            ignore_whitespace: true,
            case_insensitive: true,
            strict: true,
            output: Some("/output/path".to_string()),
        };

        assert_eq!(options.format, DiffFormat::SideBySide);
        assert_eq!(options.context_lines, 5);
        assert!(options.show_line_numbers);
        assert!(options.ignore_whitespace);
        assert!(options.case_insensitive);
        assert!(options.strict);
        assert_eq!(options.output, Some("/output/path".to_string()));
    }

    /// 测试选项克隆
    #[test]
    fn test_options_clone() {
        let options = DiffOptions {
            format: DiffFormat::Context,
            context_lines: 7,
            show_line_numbers: true,
            ignore_whitespace: false,
            case_insensitive: false,
            strict: false,
            output: Some("/clone/path".to_string()),
        };

        let cloned = options.clone();

        assert_eq!(cloned.format, options.format);
        assert_eq!(cloned.context_lines, options.context_lines);
        assert_eq!(cloned.show_line_numbers, options.show_line_numbers);
        assert_eq!(cloned.output, options.output);
    }
}

#[cfg(test)]
mod config_comparison_tests {
    use super::super::*;
    use serde_json::json;
    use std::io::Write;
    use tempfile::NamedTempFile;

    /// 测试比较相同的配置
    #[test]
    fn test_compare_identical_configs() {
        let config1 = r#"{"name": "test", "value": 123}"#;
        let config2 = r#"{"name": "test", "value": 123}"#;

        let file1 = NamedTempFile::new().unwrap();
        let file2 = NamedTempFile::new().unwrap();

        file1.write_all(config1.as_bytes()).unwrap();
        file2.write_all(config2.as_bytes()).unwrap();

        let result = DiffCommand::execute(
            file1.path().to_str().unwrap(),
            file2.path().to_str().unwrap(),
            DiffOptions::default(),
        );

        assert!(result.is_ok());
    }

    /// 测试比较不同的配置
    #[test]
    fn test_compare_different_configs() {
        let config1 = r#"{"name": "test1", "value": 123}"#;
        let config2 = r#"{"name": "test2", "value": 456}"#;

        let file1 = NamedTempFile::new().unwrap();
        let file2 = NamedTempFile::new().unwrap();

        file1.write_all(config1.as_bytes()).unwrap();
        file2.write_all(config2.as_bytes()).unwrap();

        let result = DiffCommand::execute(
            file1.path().to_str().unwrap(),
            file2.path().to_str().unwrap(),
            DiffOptions::default(),
        );

        assert!(result.is_ok());
    }

    /// 测试比较不同格式的配置
    #[test]
    fn test_compare_different_formats() {
        let toml1 = r#"
name = "test1"
value = 123
"#;
        let json2 = r#"{"name": "test2", "value": 456}"#;

        let file1 = NamedTempFile::new().unwrap();
        let file2 = NamedTempFile::new().unwrap();

        file1.write_all(toml1.as_bytes()).unwrap();
        file2.write_all(json2.as_bytes()).unwrap();

        let result = DiffCommand::execute(
            file1.path().to_str().unwrap(),
            file2.path().to_str().unwrap(),
            DiffOptions::default(),
        );

        assert!(result.is_ok());
    }

    /// 测试严格模式比较
    #[test]
    fn test_strict_mode_comparison() {
        let config1 = r#"{"name": "test", "value": 123}"#;
        let config2 = r#"{"name": "test", "value": 123, "extra": "field"}"#;

        let file1 = NamedTempFile::new().unwrap();
        let file2 = NamedTempFile::new().unwrap();

        file1.write_all(config1.as_bytes()).unwrap();
        file2.write_all(config2.as_bytes()).unwrap();

        let mut options = DiffOptions::default();
        options.strict = true;

        let result = DiffCommand::execute(
            file1.path().to_str().unwrap(),
            file2.path().to_str().unwrap(),
            options,
        );

        assert!(result.is_ok());
    }
}

#[cfg(test)]
mod diff_output_tests {
    use super::super::*;
    use tempfile::TempDir;

    /// 测试输出到文件
    #[test]
    fn test_output_to_file() {
        let temp_dir = TempDir::new().unwrap();

        let config1 = r#"{"name": "test1"}"#;
        let config2 = r#"{"name": "test2"}"#;

        let file1 = temp_dir.path().join("config1.json");
        let file2 = temp_dir.path().join("config2.json");
        let output = temp_dir.path().join("diff_output.txt");

        std::fs::write(&file1, config1).unwrap();
        std::fs::write(&file2, config2).unwrap();

        let mut options = DiffOptions::default();
        options.output = Some(output.to_string_lossy().into_owned());

        let result =
            DiffCommand::execute(file1.to_str().unwrap(), file2.to_str().unwrap(), options);

        assert!(result.is_ok());
        assert!(output.exists());

        let output_content = std::fs::read_to_string(&output).unwrap();
        assert!(!output_content.is_empty());
    }

    /// 测试忽略空白比较
    #[test]
    fn test_ignore_whitespace_comparison() {
        let config1 = r#"{ "name": "test" }"#;
        let config2 = r#"{"name":"test"}"#;

        let file1 = NamedTempFile::new().unwrap();
        let file2 = NamedTempFile::new().unwrap();

        file1.write_all(config1.as_bytes()).unwrap();
        file2.write_all(config2.as_bytes()).unwrap();

        let mut options = DiffOptions::default();
        options.ignore_whitespace = true;

        let result = DiffCommand::execute(
            file1.path().to_str().unwrap(),
            file2.path().to_str().unwrap(),
            options,
        );

        // 忽略空白时，配置应该被视作相同
        assert!(result.is_ok());
    }
}
