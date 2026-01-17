// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! 单元测试：嵌套验证功能
//!
//! 测试嵌套配置结构的验证功能

use confers::Config;
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
struct ServerConfig {
    #[validate(range(min = 1, max = 65535))]
    port: u16,

    #[validate(length(min = 1))]
    host: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Config, Validate)]
#[config(env_prefix = "APP")]
struct NestedValidationConfig {
    #[validate(length(min = 1))]
    app_name: String,

    #[validate]
    server: ServerConfig,

    #[validate(length(min = 3))]
    environment: String,
}

#[test]
fn test_nested_validation_valid_config() {
    let config = NestedValidationConfig {
        app_name: "my_app".to_string(),
        server: ServerConfig {
            port: 8080,
            host: "localhost".to_string(),
        },
        environment: "production".to_string(),
    };

    let validation_result = config.validate();
    assert!(validation_result.is_ok(), "Valid nested config should pass validation");
}

#[test]
fn test_nested_validation_invalid_server_port() {
    let config = NestedValidationConfig {
        app_name: "my_app".to_string(),
        server: ServerConfig {
            port: 70000,
            host: "localhost".to_string(),
        },
        environment: "production".to_string(),
    };

    let validation_result = config.validate();
    assert!(validation_result.is_err(), "Invalid server port should fail validation");
}

#[test]
fn test_nested_validation_invalid_server_host() {
    let config = NestedValidationConfig {
        app_name: "my_app".to_string(),
        server: ServerConfig {
            port: 8080,
            host: "".to_string(),
        },
        environment: "production".to_string(),
    };

    let validation_result = config.validate();
    assert!(validation_result.is_err(), "Invalid server host should fail validation");
}

#[test]
fn test_nested_validation_invalid_environment() {
    let config = NestedValidationConfig {
        app_name: "my_app".to_string(),
        server: ServerConfig {
            port: 8080,
            host: "localhost".to_string(),
        },
        environment: "ab".to_string(),
    };

    let validation_result = config.validate();
    assert!(validation_result.is_err(), "Invalid environment should fail validation");
}

#[test]
fn test_nested_validation_multiple_errors() {
    let config = NestedValidationConfig {
        app_name: "".to_string(),
        server: ServerConfig {
            port: 70000,
            host: "".to_string(),
        },
        environment: "ab".to_string(),
    };

    let validation_result = config.validate();
    assert!(validation_result.is_err(), "Multiple validation errors should fail");

    if let Err(errors) = validation_result {
        let error_string = errors.to_string();
        assert!(error_string.contains("app_name") || error_string.contains("server"), "Should contain validation errors");
    }
}