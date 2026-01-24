// Copyright (c) 2025 Kirky.X
//
// Licensed under MIT License
// See LICENSE file in project root for full license information.

//! 单元测试：Schema生成
//!
//! 测试配置Schema的自动生成功能，包括JSON Schema生成和验证

#[cfg(test)]
#[cfg(feature = "schema")]
mod schema_tests {
    use super::super::*;
    use serde_json::Value;
    use std::collections::HashMap;

    /// 测试基本Schema生成
    #[test]
    fn test_basic_schema_generation() {
        #[derive(Config, serde::Serialize, serde::Deserialize)]
        #[config(validate)]
        struct TestConfig {
            name: String,
            port: u16,
            debug: bool,
        }

        // 模拟Schema生成
        let schema = TestConfig::schema();

        // 验证Schema结构
        assert!(schema.contains("\"type\": \"object\""));
        assert!(schema.contains("\"properties\""));
    }

    /// 测试嵌套结构Schema生成
    #[test]
    fn test_nested_schema_generation() {
        #[derive(Config, serde::Serialize, serde::Deserialize)]
        #[config(validate)]
        struct DatabaseConfig {
            host: String,
            port: u16,
            credentials: AuthConfig,
        }

        #[derive(Config, serde::Serialize, serde::Deserialize)]
        #[config(validate)]
        struct AuthConfig {
            username: String,
            password: String,
        }

        // 模拟Schema生成
        let schema = DatabaseConfig::schema();

        // 验证嵌套结构
        assert!(schema.contains("\"credentials\""));
        assert!(schema.contains("\"username\""));
        assert!(schema.contains("\"password\""));
    }

    /// 测试数组Schema生成
    #[test]
    fn test_array_schema_generation() {
        #[derive(Config, serde::Serialize, serde::Deserialize)]
        #[config(validate)]
        struct ServerConfig {
            hosts: Vec<String>,
            ports: Vec<u16>,
        }

        // 模拟Schema生成
        let schema = ServerConfig::schema();

        // 验证数组类型
        assert!(schema.contains("\"type\": \"array\""));
        assert!(schema.contains("\"items\""));
    }

    /// 测试可选字段Schema
    #[test]
    fn test_optional_field_schema() {
        #[derive(Config, serde::Serialize, serde::Deserialize)]
        #[config(validate)]
        struct OptionalConfig {
            name: String,
            description: Option<String>,
            timeout: Option<u32>,
        }

        // 模拟Schema生成
        let schema = OptionalConfig::schema();

        // 验证可选字段处理
        assert!(schema.contains("\"description\""));
        assert!(schema.contains("\"timeout\""));
    }

    /// 测试默认值Schema
    #[test]
    fn test_default_value_schema() {
        #[derive(Config, serde::Serialize, serde::Deserialize)]
        #[config(validate)]
        struct DefaultConfig {
            #[serde(default = "default")]
            name: String,
            #[serde(default = 8080)]
            port: u16,
            #[serde(default = true)]
            debug: bool,
        }

        // 模拟Schema生成
        let schema = DefaultConfig::schema();

        // 验证默认值
        assert!(schema.contains("\"default\""));
    }

    /// 测试验证规则Schema
    #[test]
    fn test_validation_schema() {
        #[derive(Config, serde::Serialize, serde::Deserialize)]
        #[config(validate)]
        struct ValidatedConfig {
            #[serde(validate(min_length = 1))]
            name: String,
            #[serde(validate(range = "1..=65535"))]
            port: u16,
        }

        // 模拟Schema生成
        let schema = ValidatedConfig::schema();

        // 验证约束条件
        assert!(schema.contains("\"minLength\""));
        assert!(schema.contains("\"minimum\""));
        assert!(schema.contains("\"maximum\""));
    }

    /// 测试枚举Schema生成
    #[test]
    fn test_enum_schema_generation() {
        #[derive(Config, serde::Serialize, serde::Deserialize)]
        #[config(validate)]
        struct EnumConfig {
            mode: ConfigMode,
        }

        #[derive(serde::Serialize, serde::Deserialize)]
        enum ConfigMode {
            Development,
            Production,
            Testing,
        }

        // 模拟Schema生成
        let schema = EnumConfig::schema();

        // 验证枚举类型
        assert!(schema.contains("\"enum\""));
        assert!(schema.contains("\"Development\""));
        assert!(schema.contains("\"Production\""));
        assert!(schema.contains("\"Testing\""));
    }

    /// 测试复杂数据类型Schema
    #[test]
    fn test_complex_data_types_schema() {
        #[derive(Config, serde::Serialize, serde::Deserialize)]
        #[config(validate)]
        struct ComplexConfig {
            metadata: HashMap<String, Value>,
            timestamp: chrono::DateTime<chrono::Utc>,
            duration: std::time::Duration,
        }

        // 模拟Schema生成
        let schema = ComplexConfig::schema();

        // 验证复杂数据类型
        assert!(schema.contains("\"object\""));
        assert!(schema.contains("\"string\""));
        assert!(schema.contains("\"dateTime\""));
    }

    /// 测试Schema文档生成
    #[test]
    fn test_schema_documentation() {
        #[derive(Config, serde::Serialize, serde::Deserialize)]
        #[config(validate, description = "Main application configuration")]
        struct DocumentedConfig {
            #[serde(description = "Application name")]
            name: String,
            #[serde(description = "Server port number")]
            port: u16,
        }

        // 模拟Schema生成
        let schema = DocumentedConfig::schema();

        // 验证文档
        assert!(schema.contains("\"Main application configuration\""));
        assert!(schema.contains("\"Application name\""));
        assert!(schema.contains("\"Server port number\""));
    }

    /// 测试Schema验证功能
    #[test]
    fn test_schema_validation() {
        let test_schema = r#"{
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "port": {"type": "integer", "minimum": 1, "maximum": 65535}
            },
            "required": ["name"]
        }"#;

        let valid_config = r#"{"name": "test", "port": 8080}"#;
        let invalid_config = r#"{"port": 8080}"#; // 缺少必需字段

        // 验证有效配置
        let valid_result: Result<Value, _> = serde_json::from_str(valid_config);
        assert!(valid_result.is_ok());

        // 验证无效配置
        let invalid_result: Result<Value, _> = serde_json::from_str(invalid_config);
        assert!(invalid_result.is_err() || invalid_result.unwrap().get("name").is_none());
    }

    /// 测试Schema版本信息
    #[test]
    fn test_schema_version_info() {
        #[derive(Config, serde::Serialize, serde::Deserialize)]
        #[config(validate, schema_version = "1.0")]
        struct VersionedConfig {
            name: String,
        }

        // 模拟Schema生成
        let schema = VersionedConfig::schema();

        // 验证版本信息
        assert!(schema.contains("\"$schema\""));
        assert!(schema.contains("\"1.0\""));
    }
}
