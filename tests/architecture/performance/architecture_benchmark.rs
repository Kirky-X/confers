// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! 性能测试：架构配置加载性能
//!
//! 测试配置加载的性能指标

use confers::{Config, ConfigLoader};
use serde::{Deserialize, Serialize};
use std::fs;
use std::time::Instant;
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

    #[config(default = r#""little".to_string()"#)]
    endianness: String,
}

#[cfg(test)]
mod performance_tests {
    use super::*;

    #[tokio::test]
    async fn test_large_config_loading_performance() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let large_config = format!(
            r#"
        buffer_size = {}
        alignment = {}
        page_size = {}
        pointer_width = {}
        endianness = "{}"
        "#,
            if cfg!(target_pointer_width = "64") {
                1024 * 1024 * 1024
            } else {
                1024 * 1024
            },
            64,
            4096,
            if cfg!(target_pointer_width = "64") {
                64
            } else {
                32
            },
            if cfg!(target_endian = "little") {
                "little"
            } else {
                "big"
            }
        );

        fs::write(&config_path, large_config).unwrap();

        let start = Instant::now();
        let loader = create_test_loader().with_file(&config_path);
        let config: ArchitectureConfig = loader.load().await.unwrap();
        let duration = start.elapsed();

        // 使用更宽松的时间阈值，适应不同测试环境
        assert!(
            duration.as_millis() < 7000,
            "Config loading should complete in less than 7 seconds, took {}ms",
            duration.as_millis()
        );
        assert_eq!(config.alignment, 64);
    }

    #[tokio::test]
    async fn test_multiple_config_loads_performance() {
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

        let start = Instant::now();
        for _ in 0..100 {
            let loader = create_test_loader().with_file(&config_path);
            let _: ArchitectureConfig = loader.load().await.unwrap();
        }
        let duration = start.elapsed();

        // 使用更宽松的时间阈值，适应不同测试环境
        assert!(
            duration.as_millis() < 35000,
            "100 config loads should complete in less than 35 seconds, took {}ms",
            duration.as_millis()
        );
    }
}
