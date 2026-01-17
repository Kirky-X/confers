// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! 单元测试：并行验证功能
//!
//! 测试配置的并行验证功能（仅在启用 parallel feature 时）

#[cfg(feature = "parallel")]
use confers::Config;
#[cfg(feature = "parallel")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "parallel")]
use validator::Validate;
#[cfg(feature = "parallel")]
use std::sync::Arc;

#[cfg(feature = "parallel")]
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
struct ParallelValidationConfig {
    #[validate(length(min = 1, max = 50))]
    name: String,

    #[validate(range(min = 0, max = 100))]
    value: i32,

    #[validate(email)]
    email: String,
}

#[cfg(feature = "parallel")]
#[test]
fn test_parallel_validation_multiple_configs() {
    use rayon::prelude::*;

    let configs = vec![
        ParallelValidationConfig {
            name: "config1".to_string(),
            value: 50,
            email: "user1@example.com".to_string(),
        },
        ParallelValidationConfig {
            name: "config2".to_string(),
            value: 75,
            email: "user2@example.com".to_string(),
        },
        ParallelValidationConfig {
            name: "config3".to_string(),
            value: 25,
            email: "user3@example.com".to_string(),
        },
    ];

    let results: Vec<Result<(), validator::ValidationErrors>> = configs
        .par_iter()
        .map(|config| config.validate())
        .collect();

    assert!(results.len() == 3, "Should have 3 validation results");
    for result in results {
        assert!(result.is_ok(), "All valid configs should pass validation");
    }
}

#[cfg(feature = "parallel")]
#[test]
fn test_parallel_validation_with_errors() {
    use rayon::prelude::*;

    let configs = vec![
        ParallelValidationConfig {
            name: "valid_config".to_string(),
            value: 50,
            email: "user@example.com".to_string(),
        },
        ParallelValidationConfig {
            name: "".to_string(),
            value: 150,
            email: "invalid_email".to_string(),
        },
        ParallelValidationConfig {
            name: "another_valid".to_string(),
            value: 30,
            email: "another@example.com".to_string(),
        },
    ];

    let results: Vec<Result<(), validator::ValidationErrors>> = configs
        .par_iter()
        .map(|config| config.validate())
        .collect();

    assert!(results.len() == 3, "Should have 3 validation results");
    assert!(results[0].is_ok(), "First config should be valid");
    assert!(results[1].is_err(), "Second config should be invalid");
    assert!(results[2].is_ok(), "Third config should be valid");
}

#[cfg(feature = "parallel")]
#[test]
fn test_parallel_validation_large_dataset() {
    use rayon::prelude::*;

    let configs: Vec<ParallelValidationConfig> = (0..1000)
        .map(|i| ParallelValidationConfig {
            name: format!("config_{}", i),
            value: i % 100,
            email: format!("user{}@example.com", i),
        })
        .collect();

    let start = std::time::Instant::now();
    let results: Vec<Result<(), validator::ValidationErrors>> = configs
        .par_iter()
        .map(|config| config.validate())
        .collect();
    let duration = start.elapsed();

    assert!(results.len() == 1000, "Should have 1000 validation results");
    assert!(
        results.iter().all(|r| r.is_ok()),
        "All configs should be valid"
    );
    assert!(
        duration.as_millis() < 5000,
        "Parallel validation of 1000 configs should complete in less than 5 seconds"
    );
}

#[cfg(feature = "parallel")]
#[test]
fn test_parallel_validation_shared_config() {
    use rayon::prelude::*;

    let config = Arc::new(ParallelValidationConfig {
        name: "shared_config".to_string(),
        value: 50,
        email: "shared@example.com".to_string(),
    });

    let results: Vec<Result<(), validator::ValidationErrors>> = (0..10)
        .into_par_iter()
        .map(|_| config.as_ref().validate())
        .collect();

    assert!(results.len() == 10, "Should have 10 validation results");
    for result in results {
        assert!(result.is_ok(), "Shared config should be valid");
    }
}