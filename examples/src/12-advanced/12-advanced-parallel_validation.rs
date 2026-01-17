// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! 并行验证示例
//!
//! 展示如何使用并行验证来加速大型配置的验证过程。
//!
//! ## 前提条件
//!
//! - 需要启用的特性：`derive`, `parallel`
//!
//! ## 运行方式
//!
//! ```bash
//! cargo run --bin 12-advanced-parallel_validation
//! ```

#[cfg(feature = "parallel")]
use confers::Config;
#[cfg(feature = "parallel")]
use rayon::prelude::*;
#[cfg(feature = "parallel")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "parallel")]
use std::time::Instant;

#[cfg(feature = "parallel")]
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
#[config(
    parallel_validation = true,
    description = "配置结构，启用并行验证"
)]
pub struct ParallelConfig {
    #[config(
        validate = "length(min = 3, max = 50)",
        description = "应用名称"
    )]
    pub app_name: String,

    #[config(
        validate = "email",
        description = "管理员邮箱"
    )]
    pub admin_email: String,

    #[config(
        validate = "url",
        description = "API 端点"
    )]
    pub api_endpoint: String,

    #[config(
        validate = "range(min = 1, max = 65535)",
        description = "端口号"
    )]
    pub port: u16,

    #[config(
        validate = "regex(^[a-zA-Z0-9_-]+$)",
        description = "环境标识符"
    )]
    pub environment: String,

    #[config(
        validate = "range(min = 1, max = 100)",
        description = "工作线程数"
    )]
    pub workers: u32,
}

#[cfg(not(feature = "parallel"))]
fn main() {
    println!("请使用 --features parallel 运行此示例");
}

#[cfg(feature = "parallel")]
fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    println!("=== 并行验证示例 ===\n");

    // 1. 创建有效配置
    let valid_config = r#"
app_name = "parallel_app"
admin_email = "admin@example.com"
api_endpoint = "https://api.example.com"
port = 8080
environment = "production"
workers = 4
"#;

    std::fs::write("src/12-advanced/configs/parallel.toml", valid_config)?;

    println!("1. 测试并行验证（有效配置）...");
    let start = Instant::now();

    match ParallelConfig::load_file("src/12-advanced/configs/parallel.toml").load() {
        Ok(config) => {
            let duration = start.elapsed();
            println!("   ✅ 验证成功，耗时: {:?}", duration);
            println!("   配置内容: {:#?}", config);
        }
        Err(e) => {
            println!("   ❌ 验证失败: {}", e);
        }
    }

    // 2. 测试无效配置
    println!("\n2. 测试并行验证（无效配置）...");
    let invalid_config = r#"
app_name = "ab"  # 太短
admin_email = "invalid-email"
api_endpoint = "not-a-url"
port = 70000  # 超出范围
environment = "prod env!"  # 包含无效字符
workers = 150  # 超出范围
"#;

    std::fs::write("src/12-advanced/configs/parallel.toml", invalid_config)?;

    let start = Instant::now();

    match ParallelConfig::load_file("src/12-advanced/configs/parallel.toml").load() {
        Ok(_) => {
            println!("   ❌ 意外：无效配置通过了验证");
        }
        Err(e) => {
            let duration = start.elapsed();
            println!("   ✅ 验证正确拒绝无效配置，耗时: {:?}", duration);
            println!("   错误信息: {}", e);
        }
    }

    // 3. 性能对比演示
    println!("\n3. 性能对比演示...");
    let configs = generate_test_configs(100);
    let config_count = configs.len();

    println!("   生成 {} 个测试配置", config_count);

    // 并行验证
    let start = Instant::now();
    let valid_count = configs.par_iter()
        .filter(|config| validate_config(config).is_ok())
        .count();
    let parallel_duration = start.elapsed();

    println!("   并行验证: {:?} (有效: {}/{})", parallel_duration, valid_count, config_count);

    // 4. 清理临时文件
    let _ = std::fs::remove_file("src/12-advanced/configs/parallel.toml");

    println!("\n=== 并行验证的优势 ===");
    println!("- 处理大型配置时性能更好");
    println!("- 充分利用多核 CPU");
    println!("- 验证规则可以并行执行");
    println!("- 适用于配置项数量多的场景");

    Ok(())
}

#[cfg(feature = "parallel")]
/// 生成测试配置
fn generate_test_configs(count: usize) -> Vec<String> {
    (0..count)
        .map(|i| {
            format!(
                r#"
app_name = "app_{}"
admin_email = "user{}@example.com"
api_endpoint = "https://api{}.example.com"
port = {}
environment = "env{}"
workers = {}
"#,
                i % 100,
                i,
                i % 10,
                8000 + (i % 1000),
                i % 5,
                1 + (i % 10)
            )
        })
        .collect()
}

#[cfg(feature = "parallel")]
/// 验证单个配置（简化版）
fn validate_config(config_str: &str) -> anyhow::Result<()> {
    // 这里只是演示，实际应该使用完整的验证逻辑
    if config_str.contains("app_name = \"ab\"") {
        anyhow::bail!("应用名称太短");
    }
    if config_str.contains("invalid-email") {
        anyhow::bail!("无效的邮箱格式");
    }
    Ok(())
}