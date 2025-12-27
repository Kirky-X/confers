// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

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
        assert_eq!(config.endianness, "little");
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
mod performance_tests {
    use super::*;
    use std::fs;
    use std::time::Instant;
    use tempfile::TempDir;

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

        assert!(
            duration.as_millis() < 1000,
            "Config loading should complete in less than 1 second"
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

        assert!(
            duration.as_millis() < 5000,
            "100 config loads should complete in less than 5 seconds"
        );
    }
}

#[cfg(test)]
mod edge_case_tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

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
