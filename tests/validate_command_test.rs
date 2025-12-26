use confers::commands::validate::{ValidateCommand, ValidateLevel};
use confers::Config;
use serde::{Deserialize, Serialize};
use std::fs;
use tempfile::tempdir;

#[derive(Debug, Clone, Serialize, Deserialize, Config)]
#[config(validate)]
struct TestConfig {
    #[config(default = "8080")]
    port: u16,

    #[config(default = "\"localhost\".to_string()")]
    host: String,
}

#[test]
fn test_validate_command_with_valid_config() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("config.json");

    // 创建有效的配置
    let config_content = r#"{"port": 8080, "host": "example.com"}"#;
    fs::write(&config_path, config_content).unwrap();

    // 使用验证命令验证
    let result = ValidateCommand::execute::<TestConfig>(
        config_path.to_str().unwrap(),
        ValidateLevel::Minimal,
    );

    assert!(result.is_ok(), "Valid config should pass validation");
}

#[test]
fn test_validate_command_with_invalid_config() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("config.json");

    // 创建无效的配置（端口超出范围）
    let config_content = r#"{"port": 70000, "host": "example.com"}"#;
    fs::write(&config_path, config_content).unwrap();

    // 使用验证命令验证
    let result = ValidateCommand::execute::<TestConfig>(
        config_path.to_str().unwrap(),
        ValidateLevel::Minimal,
    );

    assert!(result.is_err(), "Invalid config should fail validation");
}

#[test]
fn test_validate_command_with_nonexistent_file() {
    let config_path = "/nonexistent/config.json";

    let result = ValidateCommand::execute::<TestConfig>(config_path, ValidateLevel::Minimal);

    assert!(result.is_err(), "Nonexistent file should fail validation");
}

#[test]
fn test_validate_command_generic_syntax_check() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("config.json");

    // 创建语法正确的JSON配置
    let config_content = r#"{"port": 8080, "host": "example.com"}"#;
    fs::write(&config_path, config_content).unwrap();

    // 使用通用验证命令
    let result =
        ValidateCommand::execute_generic(config_path.to_str().unwrap(), ValidateLevel::Minimal);

    assert!(result.is_ok(), "Syntax check should pass for valid JSON");
}

#[test]
fn test_validate_command_generic_with_invalid_syntax() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("config.json");

    // 创建语法错误的JSON配置
    let config_content = r#"{"port": 8080, "host": "example.com""#; // 缺少闭合括号
    fs::write(&config_path, config_content).unwrap();

    // 使用通用验证命令
    let result =
        ValidateCommand::execute_generic(config_path.to_str().unwrap(), ValidateLevel::Minimal);

    assert!(result.is_err(), "Syntax check should fail for invalid JSON");
}
