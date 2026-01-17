// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! 单元测试：内存限制基础功能
//!
//! 测试内存限制的基础功能

#[test]
fn test_memory_limit_setter() {
    use confers::ConfigLoader;
    use validator::Validate;

    #[derive(Debug, Clone, serde::Deserialize, serde::Serialize, Validate)]
    struct TestConfig {
        #[validate(length(min = 1))]
        pub name: String,
    }

    let loader: ConfigLoader<TestConfig> = ConfigLoader::new().with_memory_limit(1024);
    assert_eq!(loader.memory_limit_mb, 1024);
}

#[test]
fn test_memory_limit_zero_disabled() {
    use confers::ConfigLoader;
    use validator::Validate;

    #[derive(Debug, Clone, serde::Deserialize, serde::Serialize, Validate)]
    struct TestConfig {
        #[validate(length(min = 1))]
        pub name: String,
    }

    let loader: ConfigLoader<TestConfig> = ConfigLoader::new().with_memory_limit(0);
    assert_eq!(loader.memory_limit_mb, 0);
}

#[test]
fn test_memory_limit_loader_default_is_10mb() {
    use confers::ConfigLoader;
    use validator::Validate;

    #[derive(Debug, Clone, serde::Deserialize, serde::Serialize, Validate)]
    struct TestConfig {
        #[validate(length(min = 1))]
        pub name: String,
    }

    let loader: ConfigLoader<TestConfig> = ConfigLoader::new();
    assert_eq!(loader.memory_limit_mb, 10);
}

#[test]
fn test_memory_limit_builder_pattern() {
    use confers::ConfigLoader;
    use validator::Validate;

    #[derive(Debug, Clone, serde::Deserialize, serde::Serialize, Validate)]
    struct TestConfig {
        #[validate(length(min = 1))]
        pub name: String,
    }

    let loader: ConfigLoader<TestConfig> = ConfigLoader::new()
        .with_defaults(TestConfig {
            name: "default".to_string(),
        })
        .with_memory_limit(512);

    assert_eq!(loader.memory_limit_mb, 512);
}