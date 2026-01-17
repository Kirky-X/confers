// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! 单元测试：基础验证功能
//!
//! 测试配置验证的基础功能

use confers::Config;
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, Config, Validate)]
#[config(env_prefix = "APP")]
struct BasicValidationConfig {
    #[validate(length(min = 1, max = 50))]
    #[config(default = "\"default_app\".to_string()")]
    app_name: String,

    #[validate(range(min = 1, max = 65535))]
    #[config(default = 8080)]
    server_port: u16,

    #[validate(email)]
    #[config(default = "\"admin@example.com\".to_string()")]
    admin_email: String,
}

#[test]
fn test_basic_validation_valid_config() {
    let config = BasicValidationConfig {
        app_name: "my_app".to_string(),
        server_port: 8080,
        admin_email: "admin@example.com".to_string(),
    };

    let validation_result = config.validate();
    assert!(validation_result.is_ok(), "Valid config should pass validation");
}

#[test]
fn test_basic_validation_invalid_app_name_too_short() {
    let config = BasicValidationConfig {
        app_name: "".to_string(),
        server_port: 8080,
        admin_email: "admin@example.com".to_string(),
    };

    let validation_result = config.validate();
    assert!(validation_result.is_err(), "Empty app name should fail validation");
}

#[test]
fn test_basic_validation_invalid_app_name_too_long() {
    let config = BasicValidationConfig {
        app_name: "a".repeat(51),
        server_port: 8080,
        admin_email: "admin@example.com".to_string(),
    };

    let validation_result = config.validate();
    assert!(validation_result.is_err(), "App name too long should fail validation");
}

#[test]
fn test_basic_validation_invalid_port_too_low() {
    let config = BasicValidationConfig {
        app_name: "my_app".to_string(),
        server_port: 0,
        admin_email: "admin@example.com".to_string(),
    };

    let validation_result = config.validate();
    assert!(validation_result.is_err(), "Port too low should fail validation");
}

#[test]
fn test_basic_validation_invalid_port_too_high() {
    let config = BasicValidationConfig {
        app_name: "my_app".to_string(),
        server_port: 70000,
        admin_email: "admin@example.com".to_string(),
    };

    let validation_result = config.validate();
    assert!(validation_result.is_err(), "Port too high should fail validation");
}

#[test]
fn test_basic_validation_invalid_email() {
    let config = BasicValidationConfig {
        app_name: "my_app".to_string(),
        server_port: 8080,
        admin_email: "not_an_email".to_string(),
    };

    let validation_result = config.validate();
    assert!(validation_result.is_err(), "Invalid email should fail validation");
}

#[test]
fn test_basic_validation_default_values() {
    let config = BasicValidationConfig::default();

    let validation_result = config.validate();
    assert!(validation_result.is_ok(), "Default values should be valid");

    assert_eq!(config.app_name, "default_app");
    assert_eq!(config.server_port, 8080);
    assert_eq!(config.admin_email, "admin@example.com");
}