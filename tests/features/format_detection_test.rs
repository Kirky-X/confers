// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

use confers::Config;
use serde::{Deserialize, Serialize};
use std::fs;
use tempfile::tempdir;

#[derive(Debug, Clone, Serialize, Deserialize, Config, PartialEq)]
struct TestConfig {
    #[config(default = "\"default\".to_string()")]
    name: String,
    #[config(default = "0")]
    value: i32,
}

#[tokio::test]
async fn test_format_detection_by_content_json() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("config");

    // 创建没有扩展名的JSON文件
    fs::write(&file_path, r#"{"name": "json_test", "value": 42}"#).unwrap();

    let config: TestConfig = TestConfig::load_file(&file_path)
        .with_format_detection("ByContent")
        .load()
        .await
        .unwrap();

    assert_eq!(config.name, "json_test");
    assert_eq!(config.value, 42);
}

#[tokio::test]
async fn test_format_detection_by_content_yaml() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("config");

    // 创建没有扩展名的YAML文件
    fs::write(&file_path, "name: yaml_test\nvalue: 123").unwrap();

    let config: TestConfig = TestConfig::load_file(&file_path)
        .with_format_detection("ByContent")
        .load()
        .await
        .unwrap();

    assert_eq!(config.name, "yaml_test");
    assert_eq!(config.value, 123);
}

#[tokio::test]
async fn test_format_detection_by_content_toml() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("config");

    // 创建没有扩展名的TOML文件
    // TOML需要正确的结构，这里我们测试简单的键值对
    fs::write(&file_path, "name = \"toml_test\"\nvalue = 999").unwrap();

    let config: TestConfig = TestConfig::load_file(&file_path)
        .with_format_detection("ByContent")
        .load()
        .await
        .unwrap();

    assert_eq!(config.name, "toml_test");
    assert_eq!(config.value, 999);
}

#[tokio::test]
async fn test_format_detection_by_extension() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("config.json");

    // 创建有扩展名的JSON文件
    fs::write(&file_path, r#"{"name": "extension_test", "value": 77}"#).unwrap();

    let config: TestConfig = TestConfig::load_file(&file_path)
        .with_format_detection("ByExtension")
        .load()
        .await
        .unwrap();

    assert_eq!(config.name, "extension_test");
    assert_eq!(config.value, 77);
}

#[tokio::test]
async fn test_format_detection_default_by_content() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("config");

    // 创建没有扩展名的JSON文件，测试默认行为
    fs::write(&file_path, r#"{"name": "default_test", "value": 555}"#).unwrap();

    let config: TestConfig = TestConfig::load_file(&file_path).load().await.unwrap();

    assert_eq!(config.name, "default_test");
    assert_eq!(config.value, 555);
}

#[tokio::test]
async fn test_format_detection_detailed() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("config");

    let content = r#"{"name": "json_test", "value": 42}"#;
    fs::write(&file_path, content).unwrap();

    let detected_format =
        confers::core::loader::ConfigLoader::<TestConfig>::detect_format(&file_path);
    assert!(detected_format.is_some(), "Should detect JSON format");

    let config: TestConfig = TestConfig::load_file(&file_path)
        .with_format_detection("ByContent")
        .load()
        .await
        .unwrap();

    assert_eq!(config.name, "json_test");
    assert_eq!(config.value, 42);
}
