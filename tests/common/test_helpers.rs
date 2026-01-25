// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! 统一的测试工具模块
//!
//! 提供所有测试文件共用的辅助函数和工具

use std::fs;
use std::path::Path;
use tempfile::TempDir;

/// 创建临时目录
///
/// # Returns
/// 临时目录的 TempDir 实例，会在测试结束时自动清理
pub fn setup_temp_dir() -> TempDir {
    TempDir::new().expect("Failed to create temp directory")
}

/// 写入配置文件
///
/// # Arguments
/// * `path` - 配置文件路径
/// * `content` - 配置文件内容
pub fn write_config_file(path: &Path, content: &str) {
    fs::write(path, content).expect("Failed to write config file");
}

/// 验证审计日志
///
/// # Arguments
/// * `audit_path` - 审计日志文件路径
/// * `expected_fields` - 期望包含的字段列表
pub fn verify_audit_log(audit_path: &Path, expected_fields: &[&str]) {
    let audit_content = fs::read_to_string(audit_path).expect("Failed to read audit log");

    for field in expected_fields {
        assert!(
            audit_content.contains(field),
            "Audit log should contain field: {}",
            field
        );
    }
}

/// 创建测试配置内容（JSON 格式）
///
/// # Arguments
/// * `name` - 配置名称
/// * `value` - 配置值
/// * `enabled` - 是否启用
pub fn create_json_config(name: &str, value: i32, enabled: bool) -> String {
    format!(
        r#"{{"name": "{}", "value": {}, "enabled": {}}}"#,
        name, value, enabled
    )
}

/// 创建测试配置内容（TOML 格式）
///
/// # Arguments
/// * `name` - 配置名称
/// * `value` - 配置值
/// * `enabled` - 是否启用
pub fn create_toml_config(name: &str, value: i32, enabled: bool) -> String {
    format!(
        r#"name = "{}"
value = {}
enabled = {}"#,
        name, value, enabled
    )
}

/// 创建测试配置内容（YAML 格式）
///
/// # Arguments
/// * `name` - 配置名称
/// * `value` - 配置值
/// * `enabled` - 是否启用
pub fn create_yaml_config(name: &str, value: i32, enabled: bool) -> String {
    format!(
        r#"name: {}
value: {}
enabled: {}"#,
        name, value, enabled
    )
}

/// 解析审计日志为 JSON
///
/// # Arguments
/// * `audit_path` - 审计日志文件路径
///
/// # Returns
/// 审计日志的 JSON 值
pub fn parse_audit_log(audit_path: &Path) -> serde_json::Value {
    let audit_content = fs::read_to_string(audit_path).expect("Failed to read audit log");

    serde_json::from_str(&audit_content).expect("Failed to parse audit log as JSON")
}

/// 验证格式分布
///
/// # Arguments
/// * `audit_path` - 审计日志文件路径
/// * `expected_formats` - 期望的格式及其数量
pub fn verify_format_distribution(audit_path: &Path, expected_formats: &[(&str, u64)]) {
    let audit_json = parse_audit_log(audit_path);

    if let Some(metadata) = audit_json.get("metadata") {
        if let Some(format_dist) = metadata.get("format_distribution") {
            let format_dist_obj = format_dist
                .as_object()
                .expect("Format distribution should be an object");

            for (format_name, expected_count) in expected_formats {
                let actual_count = format_dist_obj
                    .get(*format_name)
                    .and_then(|v| v.as_u64())
                    .unwrap_or_else(|| {
                        panic!("Format '{}' not found in distribution", format_name)
                    });

                assert_eq!(
                    actual_count, *expected_count,
                    "Format '{}' count mismatch: expected {}, got {}",
                    format_name, expected_count, actual_count
                );
            }
        }
    }
}
