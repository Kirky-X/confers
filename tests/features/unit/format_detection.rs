// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! 单元测试：格式检测功能
//!
//! 测试配置文件格式的自动检测功能

use confers::utils::file_format::{detect_format_by_content, detect_format_by_extension, detect_format_smart, FileFormat};
use std::io::Write;
use tempfile::NamedTempFile;

#[test]
fn test_format_detection_json() {
    // 测试 JSON 格式检测
    let mut file = NamedTempFile::with_suffix(".json").unwrap();
    file.write_all(b"{\n  \"name\": \"test\",\n  \"value\": 42\n}").unwrap();

    let result = detect_format_by_content(file.path());
    assert_eq!(result, Some(FileFormat::Json), "Should detect JSON format");
}

#[test]
fn test_format_detection_toml() {
    // 测试 TOML 格式检测
    let mut file = NamedTempFile::with_suffix(".toml").unwrap();
    file.write_all(b"[app]\nname = \"test\"\nvalue = 42\n").unwrap();

    let result = detect_format_by_content(file.path());
    assert_eq!(result, Some(FileFormat::Toml), "Should detect TOML format");
}

#[test]
fn test_format_detection_yaml() {
    // 测试 YAML 格式检测
    let mut file = NamedTempFile::with_suffix(".yaml").unwrap();
    file.write_all(b"---\napp:\n  name: test\n  value: 42\n").unwrap();

    let result = detect_format_by_content(file.path());
    assert_eq!(result, Some(FileFormat::Yaml), "Should detect YAML format");
}

#[test]
fn test_format_detection_by_extension() {
    // 测试通过扩展名检测格式
    let json_file = NamedTempFile::with_suffix(".json").unwrap();
    let toml_file = NamedTempFile::with_suffix(".toml").unwrap();
    let yaml_file = NamedTempFile::with_suffix(".yaml").unwrap();
    let yml_file = NamedTempFile::with_suffix(".yml").unwrap();

    assert_eq!(
        detect_format_by_extension(json_file.path()),
        Some(FileFormat::Json)
    );
    assert_eq!(
        detect_format_by_extension(toml_file.path()),
        Some(FileFormat::Toml)
    );
    assert_eq!(
        detect_format_by_extension(yaml_file.path()),
        Some(FileFormat::Yaml)
    );
    assert_eq!(
        detect_format_by_extension(yml_file.path()),
        Some(FileFormat::Yaml)
    );
}

#[test]
fn test_format_detection_smart() {
    // 测试智能格式检测（优先扩展名，后备内容）
    let mut file = NamedTempFile::with_suffix(".json").unwrap();
    file.write_all(b"{\n  \"name\": \"test\"\n}").unwrap();

    let result = detect_format_smart(file.path());
    assert_eq!(result, Some(FileFormat::Json), "Smart detection should work");
}

#[test]
fn test_format_detection_unknown() {
    // 测试未知格式
    let mut file = NamedTempFile::with_suffix(".txt").unwrap();
    file.write_all(b"unknown content").unwrap();

    let result = detect_format_by_content(file.path());
    assert_eq!(result, None, "Should return None for unknown format");
}

#[test]
fn test_format_detection_empty_file() {
    // 测试空文件
    let file = NamedTempFile::new().unwrap();

    let result = detect_format_by_content(file.path());
    assert_eq!(result, None, "Should return None for empty file");
}

#[test]
fn test_format_detection_yaml_with_document_marker() {
    // 测试带文档标记的 YAML
    let mut file = NamedTempFile::with_suffix(".yaml").unwrap();
    file.write_all(b"---\n# YAML document\nname: test\n").unwrap();

    let result = detect_format_by_content(file.path());
    assert_eq!(result, Some(FileFormat::Yaml), "Should detect YAML with document marker");
}

#[test]
fn test_format_detection_json_array() {
    // 测试 JSON 数组
    let mut file = NamedTempFile::with_suffix(".json").unwrap();
    file.write_all(b"[\n  {\"name\": \"test1\"},\n  {\"name\": \"test2\"}\n]").unwrap();

    let result = detect_format_by_content(file.path());
    assert_eq!(result, Some(FileFormat::Json), "Should detect JSON array");
}

#[test]
fn test_format_detection_toml_with_nested_tables() {
    // 测试带嵌套表的 TOML
    let mut file = NamedTempFile::with_suffix(".toml").unwrap();
    file.write_all(b"[database]\nhost = \"localhost\"\n[database.connection]\nport = 5432\n").unwrap();

    let result = detect_format_by_content(file.path());
    assert_eq!(result, Some(FileFormat::Toml), "Should detect TOML with nested tables");
}