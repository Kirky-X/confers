// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Validate)]
#[allow(dead_code)]
struct TestConfig {
    #[validate(length(min = 1))]
    pub name: String,
    pub value: i32,
    pub enabled: bool,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            value: 0,
            enabled: false,
        }
    }
}

impl confers::Sanitize for TestConfig {
    fn sanitize(&self) -> serde_json::Value {
        // Basic sanitization - return the config as JSON
        serde_json::json!({
            "name": self.name.trim(),
            "value": self.value,
            "enabled": self.enabled
        })
    }
}

impl confers::ConfigMap for TestConfig {
    fn to_map(&self) -> std::collections::HashMap<String, confers::figment::value::Value> {
        use confers::figment::value::Value;

        let mut map = std::collections::HashMap::new();
        map.insert("name".to_string(), Value::from(self.name.clone()));
        map.insert("value".to_string(), Value::from(self.value));
        map.insert("enabled".to_string(), Value::from(self.enabled));
        map
    }

    fn env_mapping() -> std::collections::HashMap<String, String> {
        // Simple environment mapping for testing
        let mut mapping = std::collections::HashMap::new();
        mapping.insert("name".to_string(), "TEST_NAME".to_string());
        mapping.insert("value".to_string(), "TEST_VALUE".to_string());
        mapping.insert("enabled".to_string(), "TEST_ENABLED".to_string());
        mapping
    }
}

#[cfg(feature = "audit")]
#[tokio::test]
async fn test_audit_logger_comprehensive_metadata() {
    use confers::core::ConfigLoader;
    use std::fs;
    use tempfile::TempDir;

    // Set environment variables directly for testing
    std::env::set_var("RUN_ENV", "test");
    std::env::set_var("TEST_NAME", "env_override");
    std::env::set_var("TEST_VALUE", "500");
    std::env::set_var("TEST_ENABLED", "false");

    // Debug: Check if environment variables are set
    println!("DEBUG: RUN_ENV = {:?}", std::env::var("RUN_ENV"));
    println!("DEBUG: TEST_NAME = {:?}", std::env::var("TEST_NAME"));
    println!("DEBUG: TEST_VALUE = {:?}", std::env::var("TEST_VALUE"));
    println!("DEBUG: TEST_ENABLED = {:?}", std::env::var("TEST_ENABLED"));

    let temp_dir = TempDir::new().unwrap();
    let audit_log_path = temp_dir.path().join("audit.log");

    // Create multiple config files with different formats
    let config_dir = temp_dir.path().join("config");
    fs::create_dir(&config_dir).unwrap();

    // JSON config
    let json_path = config_dir.join("config.json");
    fs::write(
        &json_path,
        r#"{"name": "json_config", "value": 42, "enabled": true}"#,
    )
    .unwrap();

    // YAML config
    let yaml_path = config_dir.join("config.yaml");
    fs::write(&yaml_path, "name: yaml_config\nvalue: 100\nenabled: true").unwrap();

    // TOML config (higher priority)
    let toml_path = config_dir.join("config.toml");
    fs::write(
        &toml_path,
        "name = \"toml_config\"\nvalue = 200\nenabled = true",
    )
    .unwrap();

    // Environment-specific config
    let env_toml_path = config_dir.join("app.test.toml");
    fs::write(
        &env_toml_path,
        "name = \"env_config\"\nvalue = 300\nenabled = false",
    )
    .unwrap();

    let loader = ConfigLoader::<TestConfig>::new()
        .with_app_name("app")
        .with_files(vec![
            json_path.clone(),
            yaml_path.clone(),
            toml_path.clone(),
            env_toml_path.clone(),
        ])
        .with_format_detection("ByExtension")
        .with_env_prefix("TEST")
        .with_env(true)
        .with_audit_log(true)
        .with_audit_log_path(audit_log_path.to_str().unwrap().to_string());

    let result = loader.load().await;
    assert!(result.is_ok(), "Config loading failed: {:?}", result.err());

    let config = result.unwrap();

    println!("Loaded config: {:?}", config);

    // Verify config values (TOML should win due to priority, but env vars override)
    println!("Expected name: 'env_override', got: '{}'", config.name);
    println!("Expected value: 500, got: {}", config.value);
    println!("Expected enabled: false, got: {}", config.enabled);

    // Verify config values (environment variables should override all files)
    assert_eq!(config.name, "env_override");
    assert_eq!(config.value, 500);
    assert!(!config.enabled);

    // 验证审计日志已创建
    assert!(audit_log_path.exists(), "Audit log file was not created");

    let audit_content = fs::read_to_string(&audit_log_path).unwrap();
    println!("Audit log content:\n{}", audit_content);

    // 解析审计日志以了解发生了什么
    let audit_json: serde_json::Value = serde_json::from_str(&audit_content).unwrap();
    println!("Config source: {}", audit_json["metadata"]["config_source"]);
    println!("Files loaded: {:?}", audit_json["metadata"]["files_loaded"]);
    println!(
        "Environment vars count: {}",
        audit_json["metadata"]["env_vars_count"]
    );

    // Verify metadata in audit log
    assert!(
        audit_content.contains("format_distribution"),
        "Missing format distribution tracking"
    );
    assert!(audit_content.contains("json"), "JSON format not tracked");
    assert!(audit_content.contains("yaml"), "YAML format not tracked");
    assert!(audit_content.contains("toml"), "TOML format not tracked");

    // Verify performance metrics
    assert!(
        audit_content.contains("load_duration_ms"),
        "Missing load duration"
    );
    assert!(
        audit_content.contains("memory_usage_mb"),
        "Missing memory usage"
    );

    // Verify file statistics
    assert!(
        audit_content.contains("files_attempted"),
        "Missing files attempted count"
    );
    assert!(
        audit_content.contains("files_loaded"),
        "Missing files loaded count"
    );

    // Verify environment info
    assert!(
        audit_content.contains("env_vars_count"),
        "Missing environment variables count"
    );

    // Verify config sources status
    assert!(
        audit_content.contains("explicit_file"),
        "Missing explicit file status"
    );
    assert!(audit_content.contains("Success"), "Missing success status");

    // 清理环境变量
    std::env::remove_var("RUN_ENV");
    std::env::remove_var("TEST_NAME");
    std::env::remove_var("TEST_VALUE");
    std::env::remove_var("TEST_ENABLED");

    // 强制一个小延迟以确保环境变化传播
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
}

#[tokio::test]
#[cfg(feature = "audit")]
async fn test_audit_logger_with_validation_error() {
    use confers::core::ConfigLoader;
    use std::fs;
    use tempfile::TempDir;

    // 等待一下以确保前一个测试清理完成
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    // 取消设置环境变量以确保隔离
    std::env::remove_var("TEST_NAME");
    std::env::remove_var("TEST_VALUE");
    std::env::remove_var("TEST_ENABLED");
    std::env::remove_var("RUN_ENV");

    // 强制垃圾回收以确保干净状态
    std::env::remove_var("TEST_NAME");
    std::env::remove_var("TEST_VALUE");
    std::env::remove_var("TEST_ENABLED");
    std::env::remove_var("RUN_ENV");

    // 调试：检查配置加载前的环境变量
    println!("TEST_NAME env var: {:?}", std::env::var("TEST_NAME"));
    println!("TEST_VALUE env var: {:?}", std::env::var("TEST_VALUE"));
    println!("TEST_ENABLED env var: {:?}", std::env::var("TEST_ENABLED"));
    println!("RUN_ENV env var: {:?}", std::env::var("RUN_ENV"));

    let temp_dir = TempDir::new().unwrap();
    let audit_log_path = temp_dir.path().join("audit_error.log");

    // Create invalid config
    let config_path = temp_dir.path().join("config.json");
    fs::write(
        &config_path,
        r#"{"name": "", "value": -1, "enabled": true}"#,
    )
    .unwrap();

    let loader = ConfigLoader::<TestConfig>::new()
        .with_files(vec![config_path.clone()])
        .with_audit_log(true)
        .with_audit_log_path(audit_log_path.to_str().unwrap().to_string());

    let result = loader.load().await;
    assert!(
        result.is_ok(),
        "Config loading should succeed even with validation warnings"
    );

    // 验证即使存在验证问题也创建了审计日志
    assert!(audit_log_path.exists(), "Audit log file was not created");

    let audit_content = fs::read_to_string(&audit_log_path).unwrap();
    println!("Audit log with validation issues:\n{}", audit_content);

    // 验证配置已加载（在我们的简单验证中允许空名称）
    let config = result.unwrap();
    assert_eq!(config.name, ""); // Empty name from config
    assert_eq!(config.value, -1); // Negative value from config
}

#[cfg(feature = "audit")]
#[tokio::test]
async fn test_audit_logger_format_distribution_tracking() {
    use confers::core::ConfigLoader;
    use std::fs;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let audit_log_path = temp_dir.path().join("format_dist.log");

    // Create multiple config files with same base name but different extensions
    let config_dir = temp_dir.path().join("config");
    fs::create_dir(&config_dir).unwrap();

    // Multiple formats in same directory
    let json_path = config_dir.join("app.json");
    let yaml_path = config_dir.join("app.yaml");
    let toml_path = config_dir.join("app.toml");

    fs::write(
        &json_path,
        r#"{"name": "json", "value": 1, "enabled": true}"#,
    )
    .unwrap();
    fs::write(&yaml_path, "name: yaml\nvalue: 2\nenabled: true").unwrap();
    fs::write(&toml_path, "name = \"toml\"\nvalue = 3\nenabled = true").unwrap();

    let loader = ConfigLoader::<TestConfig>::new()
        .with_files(vec![json_path, yaml_path, toml_path])
        .with_audit_log(true)
        .with_audit_log_path(audit_log_path.to_str().unwrap().to_string());

    let result = loader.load().await;
    assert!(result.is_ok(), "Config loading failed: {:?}", result.err());

    let audit_content = fs::read_to_string(&audit_log_path).unwrap();
    println!("Format distribution audit:\n{}", audit_content);

    // Parse JSON to verify format distribution
    let audit_json: serde_json::Value = serde_json::from_str(&audit_content).unwrap();

    // Check if format_distribution exists and has expected formats
    if let Some(format_dist) = audit_json["metadata"]["format_distribution"].as_object() {
        println!("Format distribution found: {:?}", format_dist);

        // Verify all formats are tracked (with more robust error handling)
        if let Some(json_count) = format_dist.get("json").and_then(|v| v.as_u64()) {
            assert_eq!(json_count, 1);
        } else {
            panic!("JSON format not found in distribution");
        }

        if let Some(yaml_count) = format_dist.get("yaml").and_then(|v| v.as_u64()) {
            assert_eq!(yaml_count, 1);
        } else {
            panic!("YAML format not found in distribution");
        }

        if let Some(toml_count) = format_dist.get("toml").and_then(|v| v.as_u64()) {
            assert_eq!(toml_count, 1);
        } else {
            panic!("TOML format not found in distribution");
        }
    } else {
        panic!("format_distribution not found in audit log metadata");
    }
}

use confers::audit::AuditConfig;
use std::fs;
use tempfile::TempDir;

#[tokio::test]
async fn test_audit_logger_basic_functionality() {
    let temp_dir = TempDir::new().unwrap();
    let audit_log_path = temp_dir.path().join("audit_basic.log");

    let config = TestConfig::default();

    let result = confers::audit::AuditLogger::log_to_file(&config, &audit_log_path, None);

    assert!(result.is_ok());
    assert!(audit_log_path.exists());

    let log_content = fs::read_to_string(&audit_log_path).unwrap();
    let audit_entry: serde_json::Value = serde_json::from_str(&log_content).unwrap();

    assert!(audit_entry.get("metadata").is_some());
    assert!(audit_entry.get("config").is_some());

    let metadata = audit_entry.get("metadata").unwrap();
    assert!(metadata.get("timestamp").is_some());
    assert!(metadata.get("process_id").is_some());
    assert_eq!(
        metadata.get("validation_status").unwrap().as_str().unwrap(),
        "Success"
    );
}

#[tokio::test]
async fn test_audit_logger_with_source_info() {
    let temp_dir = TempDir::new().unwrap();
    let audit_log_path = temp_dir.path().join("audit_source.log");

    let config = TestConfig::default();

    let audit_config = AuditConfig {
        validation_error: None,
        config_source: Some("test_config.toml".to_string()),
        load_duration: Some(std::time::Duration::from_millis(150)),
        config_sources_status: Some(vec![
            (
                "file1.toml".to_string(),
                "Success".to_string(),
                None,
                Some(std::time::Duration::from_millis(50)),
            ),
            (
                "file2.json".to_string(),
                "Failed".to_string(),
                Some("Parse error".to_string()),
                None,
            ),
        ]),
        files_attempted: Some(2),
        files_loaded: Some(1),
        format_distribution: None,
        env_vars_count: Some(15),
        memory_usage_mb: None,
    };

    let result = confers::audit::AuditLogger::log_to_file_with_source(
        &config,
        &audit_log_path,
        audit_config,
    );

    assert!(result.is_ok());
    assert!(audit_log_path.exists());

    let log_content = fs::read_to_string(&audit_log_path).unwrap();
    let audit_entry: serde_json::Value = serde_json::from_str(&log_content).unwrap();

    let metadata = audit_entry.get("metadata").unwrap();
    assert_eq!(
        metadata.get("load_duration_ms").unwrap().as_u64().unwrap(),
        150
    );
    assert_eq!(
        metadata.get("config_source").unwrap().as_str().unwrap(),
        "test_config.toml"
    );

    let config_sources = metadata.get("config_sources").unwrap().as_array().unwrap();
    assert_eq!(config_sources.len(), 2);

    let source1 = &config_sources[0];
    assert_eq!(
        source1.get("source").unwrap().as_str().unwrap(),
        "file1.toml"
    );
    assert_eq!(source1.get("status").unwrap().as_str().unwrap(), "Success");

    let source2 = &config_sources[1];
    assert_eq!(
        source2.get("source").unwrap().as_str().unwrap(),
        "file2.json"
    );
    assert_eq!(source2.get("status").unwrap().as_str().unwrap(), "Failed");
}

#[tokio::test]
async fn test_audit_logger_multiple_entries() {
    let temp_dir = TempDir::new().unwrap();
    let audit_log_path = temp_dir.path().join("audit_multiple.log");

    for i in 0..3 {
        let config = TestConfig {
            name: format!("test{}", i),
            value: i * 10,
            enabled: true,
        };

        let result = confers::audit::AuditLogger::log_to_file(&config, &audit_log_path, None);
        assert!(result.is_ok());
    }

    let log_content = fs::read_to_string(&audit_log_path).unwrap();
    let lines: Vec<&str> = log_content.lines().collect();
    assert_eq!(lines.len(), 3);

    for (i, line) in lines.iter().enumerate() {
        let audit_entry: serde_json::Value = serde_json::from_str(line).unwrap();
        let config_data = audit_entry.get("config").unwrap();
        assert_eq!(
            config_data.get("name").unwrap().as_str().unwrap(),
            format!("test{}", i)
        );
        assert_eq!(
            config_data.get("value").unwrap().as_i64().unwrap(),
            (i * 10) as i64
        );
    }
}
