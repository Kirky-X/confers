// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! 单元测试：架构配置对齐功能
//!
//! 测试内存对齐、缓存行对齐、页对齐等功能

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
mod alignment_tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_alignment_multiple_of_pointer_width() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let pointer_bytes = if cfg!(target_pointer_width = "64") {
            8
        } else {
            4
        };
        let alignment = pointer_bytes * 8;

        let config_content = format!(
            r#"
        buffer_size = {}
        alignment = {}
        page_size = 4096
        pointer_width = {}
        endianness = "little"
        "#,
            alignment * 2,
            alignment,
            pointer_bytes * 8
        );

        fs::write(&config_path, config_content).unwrap();

        let loader = create_test_loader().with_file(&config_path);
        let config: ArchitectureConfig = loader.load().await.unwrap();

        assert!(config.alignment >= pointer_bytes);
        assert_eq!(config.alignment % pointer_bytes, 0);
    }

    #[tokio::test]
    async fn test_cache_line_alignment() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let cache_line_size = 64;
        let config_content = format!(
            r#"
        buffer_size = {}
        alignment = {}
        page_size = 4096
        pointer_width = 64
        endianness = "little"
        "#,
            cache_line_size * 4,
            cache_line_size
        );

        fs::write(&config_path, config_content).unwrap();

        let loader = create_test_loader().with_file(&config_path);
        let config: ArchitectureConfig = loader.load().await.unwrap();

        assert_eq!(config.alignment, cache_line_size);
        assert!(config.buffer_size >= config.alignment);
    }

    #[tokio::test]
    async fn test_page_aligned_buffer() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let page_size = 4096;
        let config_content = format!(
            r#"
        buffer_size = {}
        alignment = {}
        page_size = {}
        pointer_width = 64
        endianness = "little"
        "#,
            page_size * 4,
            page_size,
            page_size
        );

        fs::write(&config_path, config_content).unwrap();

        let loader = create_test_loader().with_file(&config_path);
        let config: ArchitectureConfig = loader.load().await.unwrap();

        assert_eq!(config.page_size, page_size);
        assert_eq!(config.buffer_size % config.page_size, 0);
        assert!(config.alignment <= config.page_size);
    }

    #[tokio::test]
    async fn test_vector_alignment() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let simd_width = if cfg!(target_arch = "x86_64") || cfg!(target_arch = "aarch64") {
            32
        } else {
            16
        };

        let config_content = format!(
            r#"
        buffer_size = {}
        alignment = {}
        page_size = 4096
        pointer_width = 64
        endianness = "little"
        "#,
            simd_width * 4,
            simd_width
        );

        fs::write(&config_path, config_content).unwrap();

        let loader = create_test_loader().with_file(&config_path);
        let config: ArchitectureConfig = loader.load().await.unwrap();

        assert_eq!(config.alignment, simd_width);
        assert!(config.buffer_size >= config.alignment);
    }
}

#[cfg(test)]
mod memory_layout_tests {
    use super::*;
    use std::fs;
    use std::mem;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_pointer_size_matches_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let expected_pointer_width = mem::size_of::<usize>() * 8;

        let config_content = format!(
            r#"
        buffer_size = 1024
        alignment = 64
        page_size = 4096
        pointer_width = {}
        endianness = "little"
        "#,
            expected_pointer_width
        );

        fs::write(&config_path, config_content).unwrap();

        let loader = create_test_loader().with_file(&config_path);
        let config: ArchitectureConfig = loader.load().await.unwrap();

        assert_eq!(config.pointer_width, expected_pointer_width);
    }

    #[tokio::test]
    async fn test_struct_size_reasonable() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let config_content = r#"
        buffer_size = 1024
        alignment = 64
        page_size = 4096
        pointer_width = 64
        endianness = "little"
        "#;

        fs::write(&config_path, config_content).unwrap();

        let loader = create_test_loader().with_file(&config_path);
        let _config: ArchitectureConfig = loader.load().await.unwrap();

        let config_size = mem::size_of::<ArchitectureConfig>();
        assert!(
            config_size < 4096,
            "Config struct should be less than one page size"
        );
    }

    #[tokio::test]
    async fn test_architecture_info_complete() {
        let config = ArchitectureConfig::default();

        assert!(
            config.pointer_width >= 32,
            "Pointer width should be at least 32 bits"
        );
        assert!(
            config.pointer_width <= 128,
            "Pointer width should be at most 128 bits"
        );

        assert!(config.page_size >= 4096, "Page size should be at least 4KB");
        assert!(
            config.page_size.is_power_of_two(),
            "Page size must be power of two"
        );

        assert!(config.alignment >= 1, "Alignment must be at least 1");
        assert!(
            config.alignment.is_power_of_two() || config.alignment == 0,
            "Alignment should be power of two or zero"
        );

        assert!(config.buffer_size >= 1, "Buffer size must be at least 1");

        assert!(
            config.endianness == "little" || config.endianness == "big",
            "Endianness must be 'little' or 'big'"
        );
    }
}