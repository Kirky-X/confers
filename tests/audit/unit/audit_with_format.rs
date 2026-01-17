// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! 单元测试：审计日志格式化功能
//!
//! 测试审计日志的格式化输出功能

use confers::audit::{AuditConfig, AuditLogger, Sanitize};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write;
use std::path::Path;
use std::time::Duration;
use tempfile::NamedTempFile;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TestConfig {
    pub name: String,
    pub port: u16,
}

impl Sanitize for TestConfig {
    fn sanitize(&self) -> serde_json::Value {
        let mut map = serde_json::Map::new();
        map.insert("name".to_string(), serde_json::json!(self.name));
        map.insert("port".to_string(), serde_json::json!(self.port));
        serde_json::Value::Object(map)
    }
}

#[test]
fn test_audit_format_validation() {
    // 测试审计日志格式验证
    let config = TestConfig {
        name: "test".to_string(),
        port: 8080,
    };
    let temp_file = NamedTempFile::new().unwrap();

    let result = AuditLogger::log_to_file(&config, temp_file.path(), None);
    assert!(result.is_ok(), "Audit log should be created");

    let content = std::fs::read_to_string(temp_file.path()).unwrap();
    assert!(content.starts_with('{'), "Log should be valid JSON");
    assert!(content.ends_with('}\n'), "Log should end with } and newline");

    // 验证 JSON 格式
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert!(json.is_object(), "Log should be a JSON object");
    assert!(json.get("metadata").is_some(), "Log should have metadata");
    assert!(json.get("config").is_some(), "Log should have config");
}

#[test]
fn test_audit_format_with_load_duration() {
    // 测试带加载时间的审计日志格式
    let config = TestConfig {
        name: "test".to_string(),
        port: 8080,
    };
    let temp_file = NamedTempFile::new().unwrap();

    let audit_config = AuditConfig {
        validation_error: None,
        config_source: None,
        load_duration: Some(Duration::from_millis(123)),
        config_sources_status: None,
        files_attempted: None,
        files_loaded: None,
        format_distribution: None,
        env_vars_count: None,
        memory_usage_mb: None,
    };

    let result = AuditLogger::log_to_file_with_source(&config, temp_file.path(), audit_config);
    assert!(result.is_ok());

    let content = std::fs::read_to_string(temp_file.path()).unwrap();
    assert!(content.contains("load_duration_ms"), "Log should contain load duration");
    assert!(content.contains("123"), "Log should contain duration value");
}

#[test]
fn test_audit_format_with_format_distribution() {
    // 测试带格式分布的审计日志格式
    let config = TestConfig {
        name: "test".to_string(),
        port: 8080,
    };
    let temp_file = NamedTempFile::new().unwrap();

    let mut format_dist = HashMap::new();
    format_dist.insert("toml".to_string(), 3);
    format_dist.insert("json".to_string(), 2);

    let audit_config = AuditConfig {
        validation_error: None,
        config_source: None,
        load_duration: None,
        config_sources_status: None,
        files_attempted: Some(5),
        files_loaded: Some(5),
        format_distribution: Some(format_dist),
        env_vars_count: Some(10),
        memory_usage_mb: None,
    };

    let result = AuditLogger::log_to_file_with_source(&config, temp_file.path(), audit_config);
    assert!(result.is_ok());

    let content = std::fs::read_to_string(temp_file.path()).unwrap();
    assert!(content.contains("format_distribution"), "Log should contain format distribution");
    assert!(content.contains("toml"), "Log should contain toml format");
    assert!(content.contains("json"), "Log should contain json format");
}

#[test]
fn test_audit_format_with_memory_usage() {
    // 测试带内存使用的审计日志格式
    let config = TestConfig {
        name: "test".to_string(),
        port: 8080,
    };
    let temp_file = NamedTempFile::new().unwrap();

    let audit_config = AuditConfig {
        validation_error: None,
        config_source: None,
        load_duration: None,
        config_sources_status: None,
        files_attempted: None,
        files_loaded: None,
        format_distribution: None,
        env_vars_count: None,
        memory_usage_mb: Some(45.6),
    };

    let result = AuditLogger::log_to_file_with_source(&config, temp_file.path(), audit_config);
    assert!(result.is_ok());

    let content = std::fs::read_to_string(temp_file.path()).unwrap();
    assert!(content.contains("memory_usage_mb"), "Log should contain memory usage");
    assert!(content.contains("45.6"), "Log should contain memory value");
}

#[test]
fn test_audit_format_metadata_completeness() {
    // 测试审计日志元数据完整性
    let config = TestConfig {
        name: "test".to_string(),
        port: 8080,
    };
    let temp_file = NamedTempFile::new().unwrap();

    let result = AuditLogger::log_to_file(&config, temp_file.path(), None);
    assert!(result.is_ok());

    let content = std::fs::read_to_string(temp_file.path()).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();
    let metadata = json.get("metadata").unwrap();

    // 验证必需的元数据字段
    assert!(metadata.get("timestamp").is_some(), "Should have timestamp");
    assert!(metadata.get("process_id").is_some(), "Should have process_id");
    assert!(metadata.get("os_info").is_some(), "Should have os_info");
}