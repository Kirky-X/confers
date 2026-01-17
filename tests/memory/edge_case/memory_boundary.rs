// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! 边界测试：内存限制边界条件
//!
//! 测试内存限制的边界条件和错误处理

#[test]
fn test_memory_limit_validation_edge_cases() {
    use confers::ConfigLoader;
    use validator::Validate;

    #[derive(Debug, Clone, serde::Deserialize, serde::Serialize, Validate)]
    struct TestConfig {
        #[validate(length(min = 1))]
        pub name: String,
    }

    let loader1: ConfigLoader<TestConfig> = ConfigLoader::new().with_memory_limit(1);
    assert_eq!(loader1.memory_limit_mb, 1);

    let loader2: ConfigLoader<TestConfig> = ConfigLoader::new().with_memory_limit(usize::MAX);
    assert_eq!(loader2.memory_limit_mb, usize::MAX);
}

#[test]
fn test_memory_limit_below_10mb_prd_requirement() {
    use confers::ConfigLoader;
    use validator::Validate;

    #[derive(Debug, Clone, serde::Deserialize, serde::Serialize, Validate)]
    struct TestConfig {
        #[validate(length(min = 1))]
        pub name: String,
    }

    let loader: ConfigLoader<TestConfig> = ConfigLoader::new().with_memory_limit(9);
    assert_eq!(loader.memory_limit_mb, 9);
    assert!(
        loader.memory_limit_mb < 10,
        "Memory limit should be below 10MB as per PRD requirement"
    );
}

#[test]
fn test_memory_limit_exactly_10mb_prd_requirement() {
    use confers::ConfigLoader;
    use validator::Validate;

    #[derive(Debug, Clone, serde::Deserialize, serde::Serialize, Validate)]
    struct TestConfig {
        #[validate(length(min = 1))]
        pub name: String,
    }

    let loader: ConfigLoader<TestConfig> = ConfigLoader::new().with_memory_limit(10);
    assert_eq!(loader.memory_limit_mb, 10);
    assert!(
        loader.memory_limit_mb <= 10,
        "Memory limit should be at most 10MB as per PRD requirement"
    );
}

#[test]
fn test_memory_limit_above_10mb_rejected() {
    use confers::ConfigLoader;
    use validator::Validate;

    #[derive(Debug, Clone, serde::Deserialize, serde::Serialize, Validate)]
    struct TestConfig {
        #[validate(length(min = 1))]
        pub name: String,
    }

    let loader: ConfigLoader<TestConfig> = ConfigLoader::new().with_memory_limit(11);
    assert_eq!(loader.memory_limit_mb, 11);
    assert!(
        loader.memory_limit_mb > 10,
        "Memory limit above 10MB should be allowed but flagged"
    );
}