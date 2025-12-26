use confers::Config;
use figment::value::Value;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Config)]
#[config(env_prefix = "TEST")]
struct SimpleConfig {
    #[config(default = 5)]
    val: u32,
}

#[test]
fn test_config_basics_creation() {
    let config = SimpleConfig { val: 42 };
    assert_eq!(config.val, 42);
}

#[test]
fn test_config_default_values() {
    let default_config = SimpleConfig::default();
    assert_eq!(default_config.val, 5);
}

#[test]
fn test_config_to_map() {
    let config = SimpleConfig { val: 42 };
    let map = config.to_map();
    assert!(!map.is_empty());
}

#[derive(Debug, Clone, Serialize, Deserialize, Config, PartialEq)]
#[config(env_prefix = "APP")]
struct AppConfig {
    #[config(default = "8080")]
    server_port: u32,

    #[config(default = "\"localhost\".to_string()")]
    server_host: String,

    #[config(sensitive = true, default = "\"secret_password\".to_string()")]
    db_password: String,

    #[config(default = "\"postgres://localhost:5432/db\".to_string()")]
    db_url: String,
}

#[test]
fn test_serde_roundtrip() {
    let config = AppConfig {
        server_port: 8080,
        server_host: "localhost".to_string(),
        db_password: "password".to_string(),
        db_url: "postgres://localhost:5432/db".to_string(),
    };

    let value = Value::serialize(config.clone()).expect("Failed to serialize");
    let deserialized: AppConfig = value.deserialize().expect("Failed to deserialize");
    assert_eq!(config, deserialized);
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Config)]
#[config(env_prefix = "ATTR")]
struct AttributeConfig {
    #[config(name_env = "CUSTOM_VAR_NAME")]
    #[config(default = "1234")]
    custom_field: u32,

    #[config(default = "\"default\".to_string()")]
    normal_field: String,
}

#[test]
fn test_name_env_attribute() {
    temp_env::with_vars(
        [
            ("CUSTOM_VAR_NAME", Some("9999")),
            ("ATTR_CUSTOM_FIELD", Some("1111")),
        ],
        || {
            let config = AttributeConfig::load().expect("Failed to load config");
            assert_eq!(config.custom_field, 9999);
        },
    );
}

#[test]
fn test_name_env_precedence() {
    temp_env::with_vars(
        [
            ("CUSTOM_VAR_NAME", Some("8888")),
            ("ATTR_CUSTOM_FIELD", Some("1111")),
        ],
        || {
            let config = AttributeConfig::load().expect("Failed to load config");
            assert_eq!(config.custom_field, 8888);
        },
    );
}

#[test]
#[cfg(feature = "watch")]
fn test_watch_attribute_generation() {
    use tempfile::tempdir;

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Config)]
    #[config(watch = true)]
    struct WatchConfig {
        #[config(default = "0")]
        val: u32,
    }

    let dir = tempdir().unwrap();
    let file_path = dir.path().join("watch.toml");
    std::fs::write(&file_path, "val = 42").unwrap();

    let (config, watcher) = WatchConfig::load_with_watcher().expect("Failed to load with watcher");
    assert_eq!(config.val, 0);
    assert!(watcher.is_some());
}

#[test]
fn test_validate_attribute() {
    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Config)]
    #[config(env_prefix = "APP")]
    struct ValidateConfig {
        #[config(default = 5)]
        val: u32,
    }

    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("test.toml");
    std::fs::write(&file_path, "val = 5").unwrap();

    let config = ValidateConfig::load_file(&file_path)
        .load_sync()
        .expect("Should load valid config");
    assert_eq!(config.val, 5);

    temp_env::with_vars([("APP_VAL", Some("15"))], || {
        let config = ValidateConfig::load().expect("Should load with env override");
        assert_eq!(config.val, 15);
    });
}
