// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

#[cfg(test)]
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
}
