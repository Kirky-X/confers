// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! 边界测试：架构配置边界条件
//!
//! 测试最小值、最大值、无效值等边界条件

use confers::{Config, ConfigLoader};
use serde::{Deserialize, Serialize};
use std::fs;
use tempfile::TempDir;

fn create_test_loader() -> ConfigLoader<ArchitectureConfig> {
    ConfigLoader::new().with_memory_limit(0)
}

#[derive(Config, Debug, Clone, Serialize, Deserialize)]
struct ArchitectureConfig {
    #[config(default = 1024)]
    buffer_size: usize,

    #[config(default = 64)]
    alignment: usize,

    #[config(default = 4096)]
    page_size: usize,

    #[cfg(target_pointer_width = "64")]
    #[config(default = 64)]
    pointer_width: usize,

    #[cfg(target_pointer_width = "32")]
    #[config(default = 32)]
    pointer_width: usize,

    #[config(default = "\"little\".to_string()")]
    endianness: String,
}

#[cfg(test)]
mod edge_case_tests {
    use super::*;

    #[tokio::test]
    async fn test_minimum_buffer_size() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let config_content = r#"
        buffer_size = 1
        alignment = 1
        page_size = 1
        pointer_width = 64
        endianness = "little"
        "#;

        fs::write(&config_path, config_content).unwrap();

        let loader = create_test_loader().with_file(&config_path);
        let config: ArchitectureConfig = loader.load().await.unwrap();

        assert_eq!(config.buffer_size, 1);
        assert_eq!(config.alignment, 1);
        assert_eq!(config.page_size, 1);
    }

    #[tokio::test]
    async fn test_maximum_buffer_size() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let max_size = usize::MAX / 2;

        let config_content = format!(
            r#"
        buffer_size = {}
        alignment = 64
        page_size = 4096
        pointer_width = 64
        endianness = "little"
        "#,
            max_size
        );

        fs::write(&config_path, config_content).unwrap();

        let loader = create_test_loader().with_file(&config_path);
        let config: ArchitectureConfig = loader.load().await.unwrap();

        assert!(config.buffer_size > 0);
    }

    #[tokio::test]
    async fn test_zero_alignment() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let config_content = r#"
        buffer_size = 1024
        alignment = 0
        page_size = 4096
        pointer_width = 64
        endianness = "little"
        "#;

        fs::write(&config_path, config_content).unwrap();

        let loader = create_test_loader().with_file(&config_path);
        let config: ArchitectureConfig = loader.load().await.unwrap();

        assert_eq!(config.alignment, 0);
    }

    #[tokio::test]
    async fn test_invalid_endianness() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let config_content = r#"
        buffer_size = 1024
        alignment = 64
        page_size = 4096
        pointer_width = 64
        endianness = "invalid"
        "#;

        fs::write(&config_path, config_content).unwrap();

        let loader = create_test_loader().with_file(&config_path);
        let config: ArchitectureConfig = loader.load().await.unwrap();

        assert_eq!(config.endianness, "invalid");
    }
}