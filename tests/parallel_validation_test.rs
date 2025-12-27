// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

#[cfg(test)]
#[cfg(feature = "parallel")]
mod test_parallel_validation {
    use confers::{Config, ParallelValidationConfig, ParallelValidator};
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize, Config)]
    #[config(validate = true)]
    struct TestConfig {
        #[config(validate = "range(min = 0, max = 100)")]
        pub value: i32,
        #[config(validate = "length(min = 1, max = 50)")]
        pub name: String,
    }

    #[test]
    fn test_parallel_validator_basic() {
        let config = ParallelValidationConfig::new()
            .with_struct_validation(true)
            .with_schema_validation(false);

        let validator = ParallelValidator::new(config);

        let configs = vec![
            (
                "config1".to_string(),
                TestConfig {
                    value: 50,
                    name: "test1".to_string(),
                },
            ),
            (
                "config2".to_string(),
                TestConfig {
                    value: 75,
                    name: "test2".to_string(),
                },
            ),
        ];

        let result = validator.validate_many(configs);
        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.is_success());
    }

    #[test]
    fn test_parallel_validator_with_errors() {
        let config = ParallelValidationConfig::new()
            .with_struct_validation(true)
            .with_schema_validation(false);

        let validator = ParallelValidator::new(config);

        let configs = vec![
            (
                "valid_config".to_string(),
                TestConfig {
                    value: 50,
                    name: "test1".to_string(),
                },
            ),
            (
                "invalid_value".to_string(),
                TestConfig {
                    value: 999,
                    name: "test2".to_string(),
                },
            ),
        ];

        let result = validator.validate_many(configs);
        assert!(result.is_ok());
        let validation_result = result.unwrap();
        assert!(
            !validation_result.is_success(),
            "Validation should have failed"
        );
        assert_eq!(
            validation_result.struct_errors.len(),
            1,
            "Should have exactly 1 struct error"
        );
    }

    #[test]
    fn test_parallel_validator_length_error() {
        let config = ParallelValidationConfig::new()
            .with_struct_validation(true)
            .with_schema_validation(false);

        let validator = ParallelValidator::new(config);

        let configs = vec![(
            "invalid_name".to_string(),
            TestConfig {
                value: 50,
                name: "".to_string(),
            },
        )];

        let result = validator.validate_many(configs);
        assert!(result.is_ok());
        let validation_result = result.unwrap();
        assert!(!validation_result.is_success());
    }

    #[test]
    fn test_parallel_validator_empty() {
        let config = ParallelValidationConfig::new()
            .with_struct_validation(true)
            .with_schema_validation(false);

        let validator = ParallelValidator::new(config);

        let configs: Vec<(String, TestConfig)> = Vec::new();
        let result = validator.validate_many(configs);
        assert!(result.is_ok());
        let validation_result = result.unwrap();
        assert!(validation_result.is_success());
    }

    #[test]
    fn test_parallel_validator_single() {
        let config = ParallelValidationConfig::new()
            .with_struct_validation(true)
            .with_schema_validation(false);

        let validator = ParallelValidator::new(config);

        let test_config = TestConfig {
            value: 25,
            name: "single".to_string(),
        };
        let result = validator.validate("single_config", &test_config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parallel_validator_single_with_error() {
        let config = ParallelValidationConfig::new()
            .with_struct_validation(true)
            .with_schema_validation(false);

        let validator = ParallelValidator::new(config);

        let invalid_config = TestConfig {
            value: -1,
            name: "test".to_string(),
        };
        let result = validator.validate("invalid_config", &invalid_config);
        assert!(result.is_err());
    }

    #[test]
    fn test_parallel_validator_with_thread_count() {
        let config = ParallelValidationConfig::new()
            .with_struct_validation(true)
            .with_schema_validation(false)
            .with_num_threads(2);

        let validator = ParallelValidator::new(config);

        let configs = vec![
            (
                "config1".to_string(),
                TestConfig {
                    value: 50,
                    name: "test1".to_string(),
                },
            ),
            (
                "config2".to_string(),
                TestConfig {
                    value: 75,
                    name: "test2".to_string(),
                },
            ),
        ];

        let result = validator.validate_many(configs);
        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.is_success());
    }

    #[test]
    fn test_parallel_validator_with_batch_size() {
        let config = ParallelValidationConfig::new()
            .with_struct_validation(true)
            .with_schema_validation(false)
            .with_batch_size(10);

        let validator = ParallelValidator::new(config);

        let configs: Vec<(String, TestConfig)> = (0..20)
            .map(|i| {
                (
                    format!("config_{}", i),
                    TestConfig {
                        value: i as i32 % 101,
                        name: format!("test_{}", i),
                    },
                )
            })
            .collect();

        let result = validator.validate_many(configs);
        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.is_success());
    }

    #[test]
    fn test_parallel_validator_with_timeout() {
        let config = ParallelValidationConfig::new()
            .with_struct_validation(true)
            .with_schema_validation(false)
            .with_timeout(30000);

        let validator = ParallelValidator::new(config);

        let configs = vec![
            (
                "config1".to_string(),
                TestConfig {
                    value: 50,
                    name: "test1".to_string(),
                },
            ),
            (
                "config2".to_string(),
                TestConfig {
                    value: 75,
                    name: "test2".to_string(),
                },
            ),
        ];

        let result = validator.validate_many(configs);
        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.is_success());
    }

    #[test]
    fn test_parallel_validator_large_batch() {
        let config = ParallelValidationConfig::new()
            .with_struct_validation(true)
            .with_schema_validation(false)
            .with_batch_size(50);

        let validator = ParallelValidator::new(config);

        let configs: Vec<(String, TestConfig)> = (0..200)
            .map(|i| {
                (
                    format!("config_{}", i),
                    TestConfig {
                        value: (i % 101) as i32,
                        name: format!("test_{}", i),
                    },
                )
            })
            .collect();

        let result = validator.validate_many(configs);
        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.is_success());
        assert_eq!(result.struct_errors.len(), 0);
    }

    #[test]
    fn test_parallel_validator_mixed_validity() {
        let config = ParallelValidationConfig::new()
            .with_struct_validation(true)
            .with_schema_validation(false);

        let validator = ParallelValidator::new(config);

        let configs: Vec<(String, TestConfig)> = (0..20)
            .map(|i| {
                let value = if i % 3 == 0 { 999 } else { (i % 100) as i32 };
                let name = if i % 5 == 0 {
                    String::new()
                } else {
                    format!("test_{}", i)
                };
                (format!("config_{}", i), TestConfig { value, name })
            })
            .collect();

        let result = validator.validate_many(configs);
        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(!result.is_success(), "Should have validation errors");
        assert!(
            result.struct_errors.len() > 0,
            "Should have at least one error"
        );
    }

    #[test]
    fn test_parallel_validator_config_access() {
        let config = ParallelValidationConfig::new()
            .with_struct_validation(true)
            .with_num_threads(4)
            .with_batch_size(25)
            .with_timeout(5000);

        let validator = ParallelValidator::new(config);
        let retrieved_config = validator.config();

        assert_eq!(retrieved_config.num_threads(), 4);
        assert_eq!(retrieved_config.batch_size(), 25);
        assert_eq!(retrieved_config.timeout_ms, Some(5000));
    }

    #[test]
    fn test_parallel_validator_default_num_threads() {
        let config = ParallelValidationConfig::new().with_struct_validation(true);

        let validator = ParallelValidator::new(config);
        let retrieved_config = validator.config();

        let threads = retrieved_config.num_threads();
        assert!(threads >= 1, "Should have at least 1 thread available");
    }

    #[test]
    fn test_parallel_validator_error_messages() {
        let config = ParallelValidationConfig::new()
            .with_struct_validation(true)
            .with_schema_validation(false);

        let validator = ParallelValidator::new(config);

        let configs = vec![
            (
                "valid_config".to_string(),
                TestConfig {
                    value: 50,
                    name: "test".to_string(),
                },
            ),
            (
                "invalid_value".to_string(),
                TestConfig {
                    value: 150,
                    name: "test".to_string(),
                },
            ),
            (
                "invalid_name".to_string(),
                TestConfig {
                    value: 25,
                    name: String::new(),
                },
            ),
        ];

        let result = validator.validate_many(configs);
        assert!(result.is_ok());
        let validation_result = result.unwrap();

        assert!(!validation_result.is_success());
        assert_eq!(validation_result.struct_errors.len(), 2);

        let errors = validation_result.errors();
        assert!(errors.len() >= 2, "Should have error messages");
    }
}
