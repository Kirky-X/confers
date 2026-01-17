// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! 集成测试：架构配置平台特定功能
//!
//! 测试不同平台（x86_64, aarch64, arm）的特定配置

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
mod x86_64_tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[cfg(target_arch = "x86_64")]
    #[tokio::test]
    async fn test_x86_64_specific_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let config_content = r#"
        buffer_size = 2048
        alignment = 64
        page_size = 4096
        pointer_width = 64
        endianness = "little"
        "#;

        fs::write(&config_path, config_content).unwrap();

        let loader = create_test_loader().with_file(&config_path);
        let config: ArchitectureConfig = loader.load().await.unwrap();

        assert_eq!(config.pointer_width, 64);
        assert_eq!(config.endianness, "little");
    }
}

#[cfg(test)]
mod aarch64_tests {
    #[allow(unused_imports)]
    use super::*;
    #[allow(unused_imports)]
    use std::fs;
    #[allow(unused_imports)]
    use tempfile::TempDir;

    #[cfg(target_arch = "aarch64")]
    #[tokio::test]
    async fn test_aarch64_specific_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let config_content = r#"
        buffer_size = 2048
        alignment = 64
        page_size = 4096
        pointer_width = 64
        endianness = "little"
        "#;

        fs::write(&config_path, config_content).unwrap();

        let loader = create_test_loader().with_file(&config_path);
        let config: ArchitectureConfig = loader.load().await.unwrap();

        assert_eq!(config.pointer_width, 64);
        assert!(config.page_size >= 4096);
    }
}

#[cfg(test)]
mod arm_tests {
    #[allow(unused_imports)]
    use super::*;
    #[allow(unused_imports)]
    use std::fs;
    #[allow(unused_imports)]
    use tempfile::TempDir;

    #[cfg(target_arch = "arm")]
    #[tokio::test]
    async fn test_arm_specific_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let config_content = r#"
        buffer_size = 1024
        alignment = 32
        page_size = 4096
        pointer_width = 32
        endianness = "little"
        "#;

        fs::write(&config_path, config_content).unwrap();

        let loader = create_test_loader().with_file(&config_path);
        let config: ArchitectureConfig = loader.load().await.unwrap();

        assert_eq!(config.pointer_width, 32);
    }
}

#[cfg(test)]
mod platform_optimization_tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_x86_64_optimization() {
        #[cfg(target_arch = "x86_64")]
        {
            let temp_dir = TempDir::new().unwrap();
            let config_path = temp_dir.path().join("config.toml");

            let config_content = r#"
            buffer_size = 2048
            alignment = 64
            page_size = 4096
            pointer_width = 64
            endianness = "little"
            "#;

            fs::write(&config_path, config_content).unwrap();

            let loader = create_test_loader().with_file(&config_path);
            let config: ArchitectureConfig = loader.load().await.unwrap();

            assert_eq!(config.pointer_width, 64);
            assert_eq!(config.alignment, 64);
            assert_eq!(config.endianness, "little");
        }
    }

    #[tokio::test]
    async fn test_aarch64_optimization() {
        #[cfg(target_arch = "aarch64")]
        {
            let temp_dir = TempDir::new().unwrap();
            let config_path = temp_dir.path().join("config.toml");

            let config_content = r#"
            buffer_size = 2048
            alignment = 64
            page_size = 4096
            pointer_width = 64
            endianness = "little"
            "#;

            fs::write(&config_path, config_content).unwrap();

            let loader = create_test_loader().with_file(&config_path);
            let config: ArchitectureConfig = loader.load().await.unwrap();

            assert_eq!(config.pointer_width, 64);
            assert!(config.page_size >= 4096);
        }
    }

    #[tokio::test]
    async fn test_sse_alignment_requirements() {
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        {
            let temp_dir = TempDir::new().unwrap();
            let config_path = temp_dir.path().join("config.toml");

            let sse_alignment = 16;
            let config_content = format!(
                r#"
            buffer_size = {}
            alignment = {}
            page_size = 4096
            pointer_width = 64
            endianness = "little"
            "#,
                sse_alignment * 4,
                sse_alignment
            );

            fs::write(&config_path, config_content).unwrap();

            let loader = create_test_loader().with_file(&config_path);
            let config: ArchitectureConfig = loader.load().await.unwrap();

            assert!(config.alignment >= sse_alignment);
        }
    }

    #[tokio::test]
    async fn test_neon_alignment_requirements() {
        #[cfg(target_arch = "aarch64")]
        {
            let temp_dir = TempDir::new().unwrap();
            let config_path = temp_dir.path().join("config.toml");

            let neon_alignment = 16;
            let config_content = format!(
                r#"
            buffer_size = {}
            alignment = {}
            page_size = 4096
            pointer_width = 64
            endianness = "little"
            "#,
                neon_alignment * 4,
                neon_alignment
            );

            fs::write(&config_path, config_content).unwrap();

            let loader = create_test_loader().with_file(&config_path);
            let config: ArchitectureConfig = loader.load().await.unwrap();

            assert!(config.alignment >= neon_alignment);
        }
    }
}

#[cfg(test)]
mod cross_platform_compatibility_tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_network_config_portability() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("network_config.toml");

        let config_content = r#"
        buffer_size = 65536
        alignment = 64
        page_size = 4096
        pointer_width = 64
        endianness = "little"
        "#;

        fs::write(&config_path, config_content).unwrap();

        let loader = create_test_loader().with_file(&config_path);
        let config: ArchitectureConfig = loader.load().await.unwrap();

        assert_eq!(config.buffer_size, 65536);
        assert!(config.alignment <= config.page_size);
    }

    #[tokio::test]
    async fn test_storage_config_portability() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("storage_config.toml");

        let sector_size = 512;
        let config_content = format!(
            r#"
        buffer_size = {}
        alignment = {}
        page_size = 4096
        pointer_width = 64
        endianness = "little"
        "#,
            sector_size * 8,
            sector_size
        );

        fs::write(&config_path, config_content).unwrap();

        let loader = create_test_loader().with_file(&config_path);
        let config: ArchitectureConfig = loader.load().await.unwrap();

        assert_eq!(config.buffer_size % sector_size, 0);
    }

    #[tokio::test]
    async fn test_multithread_config_portability() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("thread_config.toml");

        let cache_line_size = 64;
        let config_content = format!(
            r#"
        buffer_size = {}
        alignment = {}
        page_size = 4096
        pointer_width = 64
        endianness = "little"
        "#,
            cache_line_size * 16,
            cache_line_size
        );

        fs::write(&config_path, config_content).unwrap();

        let loader = create_test_loader().with_file(&config_path);
        let config: ArchitectureConfig = loader.load().await.unwrap();

        assert!(config.alignment >= cache_line_size / 2);
    }

    #[tokio::test]
    async fn test_embedded_config_portability() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("embedded_config.toml");

        let config_content = r#"
        buffer_size = 256
        alignment = 4
        page_size = 4096
        pointer_width = 32
        endianness = "little"
        "#;

        fs::write(&config_path, config_content).unwrap();

        let loader = create_test_loader().with_file(&config_path);
        let config: ArchitectureConfig = loader.load().await.unwrap();

        assert!(config.buffer_size <= 1024);
        assert!(config.alignment <= 32);
        assert_eq!(config.pointer_width, 32);
    }
}