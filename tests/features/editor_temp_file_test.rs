// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

use confers::core::loader::is_editor_temp_file;
use confers::Config;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::time::Duration;
use tempfile::tempdir;

#[derive(Debug, Clone, Serialize, Deserialize, Config)]
struct TestConfig {
    port: u16,
    name: String,
}

#[tokio::test]
async fn test_editor_temp_file_detection() {
    // 测试各种编辑器临时文件模式
    let temp_files = vec![
        "config.toml~",
        ".config.toml.",
        "#config.toml#",
        "config.toml.swp",
        "config.toml.swo",
        "config.toml.tmp",
        "file.txt~",
        ".hidden.file.",
        "#backup#",
        "notes.txt.swp",
    ];

    let normal_files = vec![
        "config.toml",
        "app.json",
        "settings.yaml",
        "README.md",
        ".gitignore", // 以.开头但不符合临时文件模式
        "file.backup",
        "config.old",
    ];

    for file in &temp_files {
        let path = Path::new(file);
        assert!(
            is_editor_temp_file(path),
            "{} 应该被识别为编辑器临时文件",
            file
        );
    }

    for file in &normal_files {
        let path = Path::new(file);
        assert!(!is_editor_temp_file(path), "{} 应该被识别为正常文件", file);
    }
}

#[tokio::test]
async fn test_watcher_filters_editor_temp_files() {
    use confers::watcher::ConfigWatcher;

    let temp_dir = tempdir().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    // 写入初始配置
    let initial_config = r#"
port = 8080
name = "initial"
"#;
    fs::write(&config_path, initial_config).unwrap();

    // 创建配置加载器
    let config = TestConfig::load_file(&config_path).load().await.unwrap();
    assert_eq!(config.port, 8080);
    assert_eq!(config.name, "initial");

    // 创建各种编辑器临时文件
    let temp_files = vec![
        temp_dir.path().join("config.toml~"),
        temp_dir.path().join("config.toml.swp"),
        temp_dir.path().join("config.toml.tmp"),
    ];

    // 创建普通文件（应该触发重载）
    let normal_file = temp_dir.path().join("normal.toml");
    fs::write(&normal_file, "initial normal content").unwrap();

    // 创建观察者
    let watcher = ConfigWatcher::new(vec![config_path.clone(), normal_file.clone()]);
    let (_guard, rx) = watcher.watch().unwrap();

    // 创建临时文件（应该被过滤）
    for temp_file in &temp_files {
        fs::write(temp_file, "temp content").unwrap();
        tokio::time::sleep(Duration::from_millis(600)).await; // 等待防抖时间
        fs::remove_file(temp_file).ok();
    }

    // 验证没有收到临时文件的事件（因为它们应该被过滤）
    let result = rx.try_recv();
    assert!(result.is_err(), "临时文件不应该触发事件");

    // 创建普通文件（应该触发事件）
    fs::write(&normal_file, "normal content").unwrap();
    tokio::time::sleep(Duration::from_millis(600)).await; // 等待防抖时间

    // 验证收到普通文件的事件
    let result = rx.try_recv();
    assert!(result.is_ok(), "普通文件应该触发事件");
}

#[tokio::test]
async fn test_normal_file_changes_trigger_reload() {
    let temp_dir = tempdir().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    // 写入初始配置
    let initial_config = r#"
port = 8080
name = "initial"
"#;
    fs::write(&config_path, initial_config).unwrap();

    // 创建配置加载器
    let config = TestConfig::load_file(&config_path).load().await.unwrap();
    assert_eq!(config.port, 8080);
    assert_eq!(config.name, "initial");

    // 修改配置文件（正常文件）
    let updated_config = r#"
port = 9090
name = "updated"
"#;
    fs::write(&config_path, updated_config).unwrap();

    // 重新加载配置
    let updated_config = TestConfig::load_file(&config_path).load().await.unwrap();
    assert_eq!(updated_config.port, 9090);
    assert_eq!(updated_config.name, "updated");
}

#[tokio::test]
async fn test_edge_case_file_names() {
    // 测试边界情况
    let edge_cases = vec![
        ("~", true),            // 只有波浪号
        (".", false),           // 只有点号
        ("#", false),           // 只有井号（不符合#file#模式）
        (".file", false),       // 以点开头但不符合模式
        ("file.", false),       // 以点结尾但不符合模式
        ("#file", false),       // 以井号开头但不符合模式
        ("file#", false),       // 以井号结尾但不符合模式
        (".file.", true),       // 以点开头和结尾
        ("#file#", true),       // 以井号开头和结尾
        ("file~", true),        // 以波浪号结尾
        ("file.backup", false), // 正常的备份文件扩展名
        ("file.tmp", true),     // tmp扩展名
        ("file.temp", false),   // temp扩展名（不在过滤列表中）
        ("file.swp", true),     // swp扩展名
        ("file.swo", true),     // swo扩展名
        ("file.swx", false),    // 类似的但不是编辑器临时文件
    ];

    for (filename, expected) in edge_cases {
        let path = Path::new(filename);
        let result = is_editor_temp_file(path);
        assert_eq!(
            result, expected,
            "文件名 '{}' 应该被识别为临时文件: {}",
            filename, expected
        );
    }
}
