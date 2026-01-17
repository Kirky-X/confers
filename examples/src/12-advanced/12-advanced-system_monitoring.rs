// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! 系统监控示例
//!
//! 展示如何使用系统监控功能来跟踪配置加载和使用的性能指标。
//!
//! ## 前提条件
//!
//! - 需要启用的特性：`derive`, `monitoring`
//!
//! ## 运行方式
//!
//! ```bash
//! cargo run --bin 12-advanced-system_monitoring
//! ```

#[cfg(feature = "monitoring")]
use confers::{Config, metrics::ConfigMetrics};
#[cfg(feature = "monitoring")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "monitoring")]
use std::time::Instant;

#[cfg(feature = "monitoring")]
#[derive(Debug, Clone, Serialize, Deserialize, Config)]
#[config(
    enable_monitoring = true,
    description = "配置结构，启用系统监控"
)]
pub struct MonitoringConfig {
    pub app_name: String,
    pub version: String,
    pub environment: String,
    pub max_connections: u32,
    pub timeout: u64,
}

#[cfg(not(feature = "monitoring"))]
fn main() {
    println!("请使用 --features monitoring 运行此示例");
}

#[cfg(feature = "monitoring")]
fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    println!("=== 系统监控示例 ===\n");

    // 1. 创建配置文件
    let config_content = r#"
app_name = "monitored_app"
version = "1.0.0"
environment = "production"
max_connections = 100
timeout = 30
"#;

    std::fs::write("src/12-advanced/configs/monitoring.toml", config_content)?;

    // 2. 启用监控并加载配置
    println!("1. 加载配置并收集监控指标...");
    let start = Instant::now();

    let config = MonitoringConfig::load_file("src/12-advanced/configs/monitoring.toml")
        .with_monitoring()
        .load()?;

    let load_duration = start.elapsed();

    println!("   ✅ 配置加载成功");
    println!("   配置内容: {:#?}", config);

    // 3. 获取监控指标
    println!("\n2. 系统监控指标:");
    let metrics = ConfigMetrics::current();

    println!("   加载时间: {:?}", load_duration);
    println!("   文件大小: {} bytes", metrics.file_size());
    println!("   解析时间: {:?}", metrics.parse_time());
    println!("   验证时间: {:?}", metrics.validate_time());
    println!("   内存使用: {} KB", metrics.memory_usage() / 1024);

    // 4. 性能分析
    println!("\n3. 性能分析:");
    let total_time = metrics.parse_time() + metrics.validate_time();

    println!("   解析耗时占比: {:.1}%",
        (metrics.parse_time().as_millis() as f64 / total_time.as_millis() as f64) * 100.0
    );
    println!("   验证耗时占比: {:.1}%",
        (metrics.validate_time().as_millis() as f64 / total_time.as_millis() as f64) * 100.0
    );

    // 5. 多次加载测试
    println!("\n4. 多次加载性能测试:");
    let iterations = 10;
    let mut total_duration = std::time::Duration::ZERO;

    for i in 0..iterations {
        let start = Instant::now();
        let _ = MonitoringConfig::load_file("src/12-advanced/configs/monitoring.toml")
            .load()?;
        total_duration += start.elapsed();

        if (i + 1) % 5 == 0 {
            println!("   已加载 {} 次，平均耗时: {:?}", i + 1, total_duration / (i + 1) as u32);
        }
    }

    let avg_duration = total_duration / iterations;
    println!("   平均加载时间: {:?}", avg_duration);

    // 6. 清理临时文件
    let _ = std::fs::remove_file("src/12-advanced/configs/monitoring.toml");

    println!("\n=== 系统监控的最佳实践 ===");
    println!("- 定期检查配置加载性能");
    println!("- 监控内存使用情况，防止内存泄漏");
    println!("- 优化慢速配置文件（考虑拆分或缓存）");
    println!("- 设置性能阈值，超过时发出告警");
    println!("- 记录历史指标，分析性能趋势");

    Ok(())
}