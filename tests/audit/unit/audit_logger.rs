// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! 单元测试：审计日志功能
//!
//! 测试审计日志的记录功能

use confers::audit::{AuditConfig, AuditLogger, Sanitize};
use serde::{Deserialize, Serialize};
use tempfile::NamedTempFile;
use std::io::Write;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TestConfig {
    pub name: String,
    pub value: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret: Option<String>,
}

impl Sanitize for TestConfig {
    fn sanitize(&self) -> serde_json::Value {
        let mut map = serde_json::Map::new();
        map.insert("name".to_string(), serde_json::json!(self.name));
        map.insert("value".to_string(), serde_json::json!(self.value));
        if let Some(ref secret) = self.secret {
            map.insert("secret".to_string(), serde_json::json!("***"));
        }
        serde_json::Value::Object(map)
    }
}

#[test]
fn test_audit_logger_creation() {
    // 测试审计日志创建
    let config = TestConfig {
        name: "test".to_string(),
        value: 42,
        secret: Some("password".to_string()),
    };
    let temp_file = NamedTempFile::new().unwrap();

    let result = AuditLogger::log_to_file(&config, temp_file.path(), None);
    assert!(result.is_ok(), "Audit logger should create log successfully");

    // 验证文件内容
    let content = std::fs::read_to_string(temp_file.path()).unwrap();
    assert!(content.contains("name"), "Log should contain config data");
    assert!(content.contains("test"), "Log should contain config value");
}

#[test]
fn test_audit_logger_with_validation_error() {
    // 测试带验证错误的审计日志
    let config = TestConfig {
        name: "test".to_string(),
        value: 42,
        secret: None,
    };
    let temp_file = NamedTempFile::new().unwrap();

    let result = AuditLogger::log_to_file(
        &config,
        temp_file.path(),
        Some("Validation failed: invalid value"),
    );
    assert!(result.is_ok(), "Audit logger should log with validation error");

    let content = std::fs::read_to_string(temp_file.path()).unwrap();
    assert!(content.contains("Validation failed"), "Log should contain error message");
}

#[test]
fn test_audit_logger_with_source() {
    // 测试带配置源的审计日志
    let config = TestConfig {
        name: "test".to_string(),
        value: 42,
        secret: None,
    };
    let temp_file = NamedTempFile::new().unwrap();

    let audit_config = AuditConfig {
        validation_error: None,
        config_source: Some("config.toml".to_string()),
        load_duration: None,
        config_sources_status: None,
        files_attempted: None,
        files_loaded: None,
        format_distribution: None,
        env_vars_count: None,
        memory_usage_mb: None,
    };

    let result = AuditLogger::log_to_file_with_source(&config, temp_file.path(), audit_config);
    assert!(result.is_ok(), "Audit logger should log with source info");

    let content = std::fs::read_to_string(temp_file.path()).unwrap();
    assert!(content.contains("config.toml"), "Log should contain config source");
}

#[test]
fn test_audit_logger_sanitize_sensitive_data() {
    // 测试敏感数据脱敏
    let config = TestConfig {
        name: "test".to_string(),
        value: 42,
        secret: Some("my_secret_password".to_string()),
    };
    let temp_file = NamedTempFile::new().unwrap();

    let result = AuditLogger::log_to_file(&config, temp_file.path(), None);
    assert!(result.is_ok(), "Audit logger should sanitize sensitive data");

    let content = std::fs::read_to_string(temp_file.path()).unwrap();
    assert!(!content.contains("my_secret_password"), "Secret should be sanitized");
    assert!(content.contains("***"), "Sanitized marker should be present");
}

#[test]
fn test_audit_logger_append_mode() {
    // 测试审计日志追加模式
    let config1 = TestConfig {
        name: "test1".to_string(),
        value: 42,
        secret: None,
    };
    let config2 = TestConfig {
        name: "test2".to_string(),
        value: 84,
        secret: None,
    };
    let temp_file = NamedTempFile::new().unwrap();

    // 第一次写入
    let result1 = AuditLogger::log_to_file(&config1, temp_file.path(), None);
    assert!(result1.is_ok());

    // 第二次写入
    let result2 = AuditLogger::log_to_file(&config2, temp_file.path(), None);
    assert!(result2.is_ok());

    // 验证两条记录都存在
    let content = std::fs::read_to_string(temp_file.path()).unwrap();
    assert!(content.contains("test1"), "First entry should be present");
    assert!(content.contains("test2"), "Second entry should be present");
}