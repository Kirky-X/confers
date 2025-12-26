#[cfg(feature = "audit")]
use confers::audit::Sanitize;
use confers::ConfigMap;
use figment::value::Value;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, Default, Validate)]
#[allow(dead_code)]
struct TestConfig {
    name: String,
    value: i32,
}

#[cfg(feature = "audit")]
impl Sanitize for TestConfig {
    fn sanitize(&self) -> serde_json::Value {
        serde_json::json!({
            "name": self.name,
            "value": self.value,
            "sanitized": true
        })
    }
}

impl ConfigMap for TestConfig {
    fn to_map(&self) -> HashMap<String, Value> {
        let mut map = HashMap::new();
        map.insert("name".to_string(), Value::from(self.name.clone()));
        map.insert("value".to_string(), Value::from(self.value));
        map
    }

    fn env_mapping() -> HashMap<String, String> {
        HashMap::new()
    }
}

#[cfg(feature = "audit")]
#[tokio::test]
async fn test_explicit_file_format_distribution() {
    use confers::core::ConfigLoader;
    use std::fs;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Create test files with different formats
    let toml_file = temp_path.join("test.toml");
    fs::write(
        &toml_file,
        r#"
name = "toml_test"
value = 42
"#,
    )
    .unwrap();

    let json_file = temp_path.join("test.json");
    fs::write(
        &json_file,
        r#"
{
    "name": "json_test",
    "value": 123
}
"#,
    )
    .unwrap();

    let yaml_file = temp_path.join("test.yaml");
    fs::write(
        &yaml_file,
        r#"
name: yaml_test
value: 999
"#,
    )
    .unwrap();

    // Create audit log file
    let audit_log = temp_path.join("audit.log");

    // Build config loader with explicit files
    let loader: ConfigLoader<TestConfig> = ConfigLoader::new()
        .with_files(vec![
            toml_file.clone(),
            json_file.clone(),
            yaml_file.clone(),
        ])
        .with_audit_log(true)
        .with_audit_log_path(audit_log.to_str().unwrap().to_string());

    // Load configuration
    let result = loader.load().await;
    assert!(result.is_ok());

    // Check audit log was created
    assert!(audit_log.exists());

    let audit_content = fs::read_to_string(&audit_log).unwrap();
    println!("Audit log content:\n{}", audit_content);

    // Verify format distribution is tracked
    assert!(audit_content.contains("format_distribution"));

    // Parse JSON to check format distribution
    if let Ok(audit_json) = serde_json::from_str::<serde_json::Value>(&audit_content) {
        if let Some(metadata) = audit_json.get("metadata") {
            if let Some(format_dist) = metadata.get("format_distribution") {
                println!("Format distribution: {}", format_dist);

                // Should have toml, json, and yaml formats
                let format_dist_obj = format_dist.as_object().unwrap();
                assert!(format_dist_obj.contains_key("toml"));
                assert!(format_dist_obj.contains_key("json"));
                assert!(format_dist_obj.contains_key("yaml"));

                // Each format should have count of 1
                assert_eq!(format_dist_obj.get("toml").unwrap().as_u64().unwrap(), 1);
                assert_eq!(format_dist_obj.get("json").unwrap().as_u64().unwrap(), 1);
                assert_eq!(format_dist_obj.get("yaml").unwrap().as_u64().unwrap(), 1);
            }
        }
    }
}
