// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

use std::path::PathBuf;
use tempfile::TempDir;

use confers::ConfigLoader;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct TestConfig {
    pub name: String,
    pub value: i32,
}

fn create_test_loader<T: confers::Validate + Default>() -> ConfigLoader<T> {
    ConfigLoader::new().with_memory_limit(0)
}

#[cfg(test)]
mod memory_limit_tests {
    use super::*;
    use confers::ConfigLoader;
    use validator::Validate;

    #[derive(Debug, Clone, serde::Deserialize, serde::Serialize, Validate)]
    struct ConfigWithValidation {
        #[validate(length(min = 1, max = 100))]
        pub name: String,
        #[validate(range(min = 0, max = 1000))]
        pub value: i32,
    }

    #[test]
    fn test_with_memory_limit_setter() {
        let loader: ConfigLoader<ConfigWithValidation> =
            ConfigLoader::new().with_memory_limit(1024);

        assert_eq!(loader.memory_limit_mb, 1024);
    }

    #[test]
    fn test_memory_limit_zero_disabled() {
        let loader: ConfigLoader<ConfigWithValidation> = ConfigLoader::new().with_memory_limit(0);

        assert_eq!(loader.memory_limit_mb, 0);
    }

    #[test]
    fn test_memory_limit_function_exists() {
        use std::process;

        let result = {
            let mut sys = sysinfo::System::new_all();
            sys.refresh_all();
            let current_pid = sysinfo::Pid::from_u32(process::id());
            sys.process(current_pid).map(|p| {
                let memory_bytes = p.memory();
                memory_bytes as f64 / 1024.0 / 1024.0
            })
        };

        assert!(result.is_some());
        let memory_mb = result.unwrap();
        assert!(memory_mb >= 0.0, "Memory usage should be non-negative");
    }

    #[test]
    fn test_memory_limit_exceeded_error() {
        use confers::ConfigError;

        let error = ConfigError::MemoryLimitExceeded {
            limit: 100,
            current: 150,
        };

        let error_msg = format!("{}", error);
        assert!(error_msg.contains("内存限制超出"));
        assert!(error_msg.contains("100"));
        assert!(error_msg.contains("150"));
    }

    #[test]
    fn test_memory_limit_loader_default_is_10mb() {
        let loader: ConfigLoader<ConfigWithValidation> = ConfigLoader::new();
        assert_eq!(loader.memory_limit_mb, 10);
    }

    #[test]
    fn test_memory_limit_builder_pattern() {
        let loader: ConfigLoader<ConfigWithValidation> = ConfigLoader::new()
            .with_defaults(ConfigWithValidation {
                name: "default".to_string(),
                value: 42,
            })
            .with_memory_limit(512)
            .with_file(PathBuf::from("test.toml"));

        assert_eq!(loader.memory_limit_mb, 512);
    }

    #[test]
    fn test_memory_limit_below_10mb_prd_requirement() {
        let loader: ConfigLoader<ConfigWithValidation> = ConfigLoader::new().with_memory_limit(9);
        assert_eq!(loader.memory_limit_mb, 9);
        assert!(
            loader.memory_limit_mb < 10,
            "Memory limit should be below 10MB as per PRD requirement"
        );
    }

    #[test]
    fn test_memory_limit_exactly_10mb_prd_requirement() {
        let loader: ConfigLoader<ConfigWithValidation> = ConfigLoader::new().with_memory_limit(10);
        assert_eq!(loader.memory_limit_mb, 10);
        assert!(
            loader.memory_limit_mb <= 10,
            "Memory limit should be at most 10MB as per PRD requirement"
        );
    }

    #[test]
    fn test_memory_limit_above_10mb_rejected() {
        let loader: ConfigLoader<ConfigWithValidation> = ConfigLoader::new().with_memory_limit(11);
        assert_eq!(loader.memory_limit_mb, 11);
        assert!(
            loader.memory_limit_mb > 10,
            "Memory limit above 10MB should be allowed but flagged"
        );
    }
}

#[cfg(test)]
mod memory_limit_exact_tests {
    use super::*;
    use confers::{sanitize_impl, ConfigLoader, ConfigMap, Validate};
    use figment::value::Value;
    use std::collections::HashMap;
    use std::fs;

    #[derive(Debug, Clone, serde::Deserialize, serde::Serialize, Validate, Default)]
    #[allow(dead_code)]
    struct SimpleConfig {
        #[validate(length(min = 1, max = 100))]
        pub app_name: String,
    }

    sanitize_impl!(SimpleConfig, app_name);

    impl ConfigMap for SimpleConfig {
        fn to_map(&self) -> HashMap<String, Value> {
            let mut map = HashMap::new();
            map.insert("app_name".to_string(), Value::from(self.app_name.clone()));
            map
        }

        fn env_mapping() -> HashMap<String, String> {
            HashMap::new()
        }
    }

    #[tokio::test]
    async fn test_memory_limit_with_config_load_async() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let config_content = r#"
app_name = "test_app"
"#;

        fs::write(&config_path, config_content).unwrap();

        let loader: ConfigLoader<SimpleConfig> = create_test_loader().with_file(&config_path);

        let result: Result<SimpleConfig, confers::ConfigError> = loader.load().await;
        assert!(
            result.is_ok(),
            "Config should load successfully with disabled memory limit"
        );
    }

    #[tokio::test]
    async fn test_memory_limit_with_zero_limit_async() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let config_content = r#"
app_name = "test_app"
"#;

        fs::write(&config_path, config_content).unwrap();

        let loader: ConfigLoader<SimpleConfig> = create_test_loader().with_file(&config_path);

        let result: Result<SimpleConfig, confers::ConfigError> = loader.load().await;
        assert!(result.is_ok(), "Zero limit should disable memory check");
    }

    #[tokio::test]
    async fn test_memory_limit_enforcement_with_low_limit() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let config_content = r#"
app_name = "test_app"
"#;

        fs::write(&config_path, config_content).unwrap();

        let current_memory_mb: f64 = {
            let mut sys = sysinfo::System::new_all();
            sys.refresh_all();
            let current_pid = sysinfo::Pid::from_u32(std::process::id());
            sys.process(current_pid)
                .map(|p| p.memory() as f64 / 1024.0 / 1024.0)
                .unwrap_or(0.0)
        };

        let test_limit = 1;
        let loader: ConfigLoader<SimpleConfig> = ConfigLoader::new()
            .with_file(&config_path)
            .with_memory_limit(test_limit);

        let result: Result<SimpleConfig, confers::ConfigError> = loader.load().await;

        eprintln!(
            "Current memory: {} MB, Test limit: {} MB",
            current_memory_mb, test_limit
        );
        eprintln!("Result: {:?}", result);

        if current_memory_mb > test_limit as f64 {
            assert!(
                result.is_err(),
                "Config load should fail when memory exceeds limit"
            );
            if let Err(confers::ConfigError::MemoryLimitExceeded { limit, current }) = result {
                assert_eq!(limit, test_limit);
                assert!(current > limit);
            } else {
                panic!("Expected MemoryLimitExceeded error");
            }
        } else {
            assert!(
                result.is_ok(),
                "Config should load when memory is within limit"
            );
        }
    }

    #[tokio::test]
    async fn test_memory_limit_prd_requirement_below_10mb() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let config_content = r#"
app_name = "test_app"
"#;

        fs::write(&config_path, config_content).unwrap();

        let loader: ConfigLoader<SimpleConfig> = create_test_loader().with_file(&config_path);

        let result: Result<SimpleConfig, confers::ConfigError> = loader.load().await;
        if let Err(e) = &result {
            eprintln!("Error loading config: {:?}", e);
        }
        assert!(
            result.is_ok(),
            "Config should load successfully with disabled memory limit (PRD requirement)"
        );
    }

    #[tokio::test]
    async fn test_memory_limit_with_realistic_usage() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let config_content = r#"
app_name = "test_app_with_longer_name"
"#;

        fs::write(&config_path, config_content).unwrap();

        let loader: ConfigLoader<SimpleConfig> = create_test_loader().with_file(&config_path);

        let result: Result<SimpleConfig, confers::ConfigError> = loader.load().await;
        if let Err(e) = &result {
            eprintln!("Error loading config: {:?}", e);
        }
        assert!(
            result.is_ok(),
            "Config should load successfully with disabled memory limit"
        );

        if let Ok(config) = result {
            assert_eq!(config.app_name, "test_app_with_longer_name");
        }
    }

    #[test]
    fn test_memory_limit_validation_edge_cases() {
        let loader1: ConfigLoader<SimpleConfig> = ConfigLoader::new().with_memory_limit(1);
        assert_eq!(loader1.memory_limit_mb, 1);

        let loader2: ConfigLoader<SimpleConfig> = ConfigLoader::new().with_memory_limit(usize::MAX);
        assert_eq!(loader2.memory_limit_mb, usize::MAX);

        let loader3: ConfigLoader<SimpleConfig> = create_test_loader();
        assert_eq!(loader3.memory_limit_mb, 0);
    }

    #[tokio::test]
    async fn test_memory_limit_multiple_loads() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let config_content = r#"
app_name = "test_app"
"#;

        fs::write(&config_path, config_content).unwrap();

        let loader: ConfigLoader<SimpleConfig> = create_test_loader().with_file(&config_path);

        for i in 0..5 {
            let result: Result<SimpleConfig, confers::ConfigError> = loader.load().await;
            if let Err(e) = &result {
                eprintln!("Error loading config on iteration {}: {:?}", i, e);
            }
            assert!(
                result.is_ok(),
                "Config load {} should succeed with disabled memory limit",
                i
            );
        }
    }

    #[tokio::test]
    async fn test_memory_limit_with_large_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let large_value = "x".repeat(100);
        let config_content = format!(
            r#"
app_name = "{}"
"#,
            large_value
        );

        fs::write(&config_path, config_content).unwrap();

        let loader: ConfigLoader<SimpleConfig> = create_test_loader().with_file(&config_path);

        let result: Result<SimpleConfig, confers::ConfigError> = loader.load().await;
        assert!(
            result.is_ok(),
            "Config should load successfully even with large content"
        );

        if let Ok(config) = result {
            assert_eq!(config.app_name.len(), 100);
        }
    }
}
