// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! 单元测试：命令行工具验证功能
//!
//! 测试命令行工具的验证功能

use confers::commands::validate::ValidateCommand;
use confers::commands::validate::ValidateLevel;
use std::io::Write;
use tempfile::NamedTempFile;

#[test]
fn test_validate_command_exists() {
    // 测试验证命令存在且可调用
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(b"name = \"test\"\nvalue = 42\n").unwrap();

    let result =
        ValidateCommand::execute_generic(file.path().to_str().unwrap(), ValidateLevel::Minimal);
    assert!(result.is_ok(), "Validate command should exist and work");
}

#[test]
fn test_validate_level_parsing() {
    // 测试验证级别解析
    assert_eq!(
        ValidateLevel::from_str("minimal").unwrap(),
        ValidateLevel::Minimal
    );
    assert_eq!(
        ValidateLevel::from_str("full").unwrap(),
        ValidateLevel::Full
    );
    assert_eq!(
        ValidateLevel::from_str("documentation").unwrap(),
        ValidateLevel::Documentation
    );
}

#[test]
fn test_validate_toml_file() {
    // 测试 TOML 文件验证
    let mut file = NamedTempFile::with_suffix(".toml").unwrap();
    file.write_all(b"[app]\nname = \"test\"\nport = 8080\n")
        .unwrap();

    let result =
        ValidateCommand::execute_generic(file.path().to_str().unwrap(), ValidateLevel::Full);
    assert!(result.is_ok(), "Valid TOML file should pass validation");
}

#[test]
fn test_validate_json_file() {
    // 测试 JSON 文件验证
    let mut file = NamedTempFile::with_suffix(".json").unwrap();
    file.write_all(b"{\"app\": {\"name\": \"test\", \"port\": 8080}}\n")
        .unwrap();

    let result =
        ValidateCommand::execute_generic(file.path().to_str().unwrap(), ValidateLevel::Full);
    assert!(result.is_ok(), "Valid JSON file should pass validation");
}

#[test]
fn test_validate_yaml_file() {
    // 测试 YAML 文件验证
    let mut file = NamedTempFile::with_suffix(".yaml").unwrap();
    file.write_all(b"app:\n  name: test\n  port: 8080\n")
        .unwrap();

    let result =
        ValidateCommand::execute_generic(file.path().to_str().unwrap(), ValidateLevel::Full);
    assert!(result.is_ok(), "Valid YAML file should pass validation");
}

#[test]
fn test_validate_invalid_toml() {
    // 测试无效 TOML 文件
    let mut file = NamedTempFile::with_suffix(".toml").unwrap();
    file.write_all(b"[invalid toml\n").unwrap();

    let result =
        ValidateCommand::execute_generic(file.path().to_str().unwrap(), ValidateLevel::Full);
    assert!(result.is_err(), "Invalid TOML file should fail validation");
}

#[test]
fn test_validate_nonexistent_file() {
    // 测试不存在的文件
    let result = ValidateCommand::execute_generic("/nonexistent/file.toml", ValidateLevel::Full);
    assert!(result.is_err(), "Nonexistent file should fail validation");
}
