// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! 单元测试：架构配置基础功能
//!
//! 测试架构配置的基本创建、默认值和属性

use confers::{Config, ConfigLoader};
use serde::{Deserialize, Serialize};

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
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_pointer_width_validation() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let expected_width = if cfg!(target_pointer_width = "64") {
            64
        } else {
            32
        };

        let config_content = format!(
            r#"
        buffer_size = 1024
        alignment = 64
        page_size = 4096
        pointer_width = {}
        endianness = "little"
        "#,
            expected_width
        );

        fs::write(&config_path, config_content).unwrap();

        let loader = create_test_loader().with_file(&config_path);
        let _config: ArchitectureConfig = loader.load().await.unwrap();

        assert_eq!(_config.pointer_width, expected_width);
    }

    #[tokio::test]
    async fn test_pointer_width_mismatch() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let mismatched_width = if cfg!(target_pointer_width = "64") {
            32
        } else {
            64
        };

        let config_content = format!(
            r#"
        buffer_size = 1024
        alignment = 64
        page_size = 4096
        pointer_width = {}
        endianness = "little"
        "#,
            mismatched_width
        );

        fs::write(&config_path, config_content).unwrap();

        let loader = create_test_loader().with_file(&config_path);
        let config: ArchitectureConfig = loader.load().await.unwrap();

        assert_eq!(config.pointer_width, mismatched_width);
    }

    #[tokio::test]
    async fn test_endianness_validation() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let expected_endianness = if cfg!(target_endian = "little") {
            "little"
        } else {
            "big"
        };

        let config_content = format!(
            r#"
        buffer_size = 1024
        alignment = 64
        page_size = 4096
        pointer_width = 64
        endianness = "{}"
        "#,
            expected_endianness
        );

        fs::write(&config_path, config_content).unwrap();

        let loader = create_test_loader().with_file(&config_path);
        let config: ArchitectureConfig = loader.load().await.unwrap();

        assert_eq!(config.endianness, expected_endianness);
    }

    #[tokio::test]
    async fn test_buffer_size_alignment() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let alignment = 64;
        let buffer_size = 1024;

        let config_content = format!(
            r#"
        buffer_size = {}
        alignment = {}
        page_size = 4096
        pointer_width = 64
        endianness = "little"
        "#,
            buffer_size, alignment
        );

        fs::write(&config_path, config_content).unwrap();

        let loader = create_test_loader().with_file(&config_path);
        let config: ArchitectureConfig = loader.load().await.unwrap();

        assert_eq!(config.buffer_size % config.alignment, 0);
    }

    #[tokio::test]
    async fn test_page_size_validation() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let page_size = 4096;

        let config_content = format!(
            r#"
        buffer_size = 1024
        alignment = 64
        page_size = {}
        pointer_width = 64
        endianness = "little"
        "#,
            page_size
        );

        fs::write(&config_path, config_content).unwrap();

        let loader = create_test_loader().with_file(&config_path);
        let config: ArchitectureConfig = loader.load().await.unwrap();

        assert_eq!(config.page_size, page_size);
        assert!(config.page_size.is_power_of_two());
    }

    #[tokio::test]
    async fn test_alignment_power_of_two() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let alignment = 64;

        let config_content = format!(
            r#"
        buffer_size = 1024
        alignment = {}
        page_size = 4096
        pointer_width = 64
        endianness = "little"
        "#,
            alignment
        );

        fs::write(&config_path, config_content).unwrap();

        let loader = create_test_loader().with_file(&config_path);
        let config: ArchitectureConfig = loader.load().await.unwrap();

        assert!(config.alignment.is_power_of_two());
    }

    #[tokio::test]
    async fn test_large_buffer_size() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let large_buffer_size = if cfg!(target_pointer_width = "64") {
            1024 * 1024 * 1024
        } else {
            1024 * 1024
        };

        let config_content = format!(
            r#"
        buffer_size = {}
        alignment = 64
        page_size = 4096
        pointer_width = 64
        endianness = "little"
        "#,
            large_buffer_size
        );

        fs::write(&config_path, config_content).unwrap();

        let loader = create_test_loader().with_file(&config_path);
        let config: ArchitectureConfig = loader.load().await.unwrap();

        assert_eq!(config.buffer_size, large_buffer_size);
    }

    #[tokio::test]
    async fn test_architecture_specific_defaults() {
        let config = ArchitectureConfig::default();

        assert_eq!(config.buffer_size, 1024);
        assert_eq!(config.alignment, 64);
        assert_eq!(config.page_size, 4096);

        #[cfg(target_pointer_width = "64")]
        assert_eq!(config.pointer_width, 64);

        #[cfg(target_pointer_width = "32")]
        assert_eq!(config.pointer_width, 32);

        assert_eq!(config.endianness, "little");
    }
}