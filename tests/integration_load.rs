//! Integration tests for configuration loading.

mod common;

use confers::value::ConfigValue;
use confers::ConfigBuilder;
use confers::*;

mod tests {
    use super::*;

    #[test]
    fn test_load_file_not_found() {
        let result: Result<serde_json::Value, _> = ConfigBuilder::new()
            .file("/nonexistent/path/config.toml")
            .build();

        assert!(result.is_err());
    }

    #[test]
    fn test_load_invalid_toml() {
        let content = r#"
[invalid
missing_bracket = true
"#;
        let file = common::create_temp_config(content, ".toml");

        let result: Result<serde_json::Value, _> = ConfigBuilder::new().file(file.path()).build();

        assert!(result.is_err());
    }

    #[test]
    fn test_load_invalid_json() {
        let content = r#"{"invalid": json}"#;
        let file = common::create_temp_config(content, ".json");

        let result: Result<serde_json::Value, _> = ConfigBuilder::new().file(file.path()).build();

        assert!(result.is_err());
    }

    #[test]
    fn test_config_builder_creation() {
        let builder: ConfigBuilder<serde_json::Value> = ConfigBuilder::new();
        assert!(builder.build().is_ok());
    }

    #[test]
    fn test_config_value_types() {
        let string_val = ConfigValue::String("test".to_string());
        assert!(matches!(string_val, ConfigValue::String(_)));

        let int_val = ConfigValue::I64(42);
        assert!(matches!(int_val, ConfigValue::I64(_)));

        let bool_val = ConfigValue::Bool(true);
        assert!(matches!(bool_val, ConfigValue::Bool(_)));
    }

    #[test]
    fn test_load_empty_file() {
        let content = "";
        let file = common::create_temp_config(content, ".toml");

        let result: Result<serde_json::Value, _> = ConfigBuilder::new().file(file.path()).build();

        assert!(result.is_ok());
    }

    #[test]
    fn test_load_with_format_detection() {
        let toml_content = r#"
key = "value"
"#;
        let file = common::create_temp_config(toml_content, ".toml");

        let result: Result<serde_json::Value, _> = ConfigBuilder::new().file(file.path()).build();

        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config["key"], "value");
    }

    #[test]
    fn test_load_array_values() {
        let content = r#"
servers = ["localhost:8080", "localhost:8081", "localhost:8082"]
"#;
        let file = common::create_temp_config(content, ".toml");

        let result: Result<serde_json::Value, _> = ConfigBuilder::new().file(file.path()).build();

        assert!(result.is_ok());
        let config = result.unwrap();
        assert!(config["servers"].is_array());
        assert_eq!(config["servers"].as_array().unwrap().len(), 3);
    }
}
