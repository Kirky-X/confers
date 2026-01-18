// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! 模板生成示例
//!
//! 展示如何使用模板来快速生成配置文件。
//!
//! ## 前提条件
//!
//! - 需要启用的特性：`derive`
//!
//! ## 运行方式
//!
//! ```bash
//! cargo run --bin 13-wizard-template_generation
//! ```

use confers::Config;
use serde::{Deserialize, Serialize};

// === Structs ===

#[derive(Debug, Clone, Serialize, Deserialize, Config)]
pub struct TemplateConfig {
    pub app_name: String,
    pub version: String,
    pub environment: String,
    pub database_url: String,
    pub redis_url: String,
    pub api_port: u16,
    pub admin_port: u16,
}

// === Main ===

fn main() -> anyhow::Result<()> {
    println!("=== 模板生成示例 ===\n");

    // 1. 定义模板变量
    let app_name = "my-webapp";
    let version = "1.0.0";
    let environment = "development";
    let db_host = "localhost";
    let db_port = "5432";
    let db_name = "myapp";
    let redis_host = "localhost";
    let redis_port = "6379";
    let api_port = "8080";
    let admin_port = "8081";

    // 2. 生成配置文件
    println!("生成配置文件...\n");

    let config_content = format!(
        r#"# {} Configuration
# Version: {}
# Environment: {}

[database]
url = "postgres://{}:{}/{}"
pool_size = 10
timeout = 30

[redis]
url = "redis://{}:{}"
pool_size = 5

[server]
api_port = {}
admin_port = {}
workers = 4

[logging]
level = "info"
format = "json"
"#,
        app_name,
        version,
        environment,
        db_host,
        db_port,
        db_name,
        redis_host,
        redis_port,
        api_port,
        admin_port
    );

    println!("=== 生成的配置内容 ===\n");
    println!("{}", config_content);

    // 3. 保存配置文件
    let config_path = "src/13-wizard/configs/generated.toml";
    std::fs::write(config_path, config_content)?;

    println!("\n✅ 配置文件已生成: {}", config_path);

    // 4. 验证配置
    println!("\n=== 验证配置 ===");

    let config_content = r#"
app_name = "my-webapp"
version = "1.0.0"
environment = "development"

[database]
url = "postgres://localhost:5432/myapp"
pool_size = 10
timeout = 30

[redis]
url = "redis://localhost:6379"
pool_size = 5

[server]
api_port = 8080
admin_port = 8081
workers = 4

[logging]
level = "info"
format = "json"
"#;

    std::fs::write(config_path, config_content)?;

    println!("应用名称: {}", app_name);
    println!("版本号: {}", version);
    println!("环境: {}", environment);
    println!("数据库 URL: postgres://{}:{}/{}", db_host, db_port, db_name);
    println!("Redis URL: redis://{}:{}", redis_host, redis_port);
    println!("API 端口: {}", api_port);
    println!("管理端口: {}", admin_port);

    println!("\n=== 模板变量 ===");
    println!("可用的模板变量:");
    println!("  {{app_name}} - 应用名称");
    println!("  {{version}} - 版本号");
    println!("  {{environment}} - 运行环境");
    println!("  {{db_host}} - 数据库主机");
    println!("  {{db_port}} - 数据库端口");
    println!("  {{db_name}} - 数据库名称");
    println!("  {{redis_host}} - Redis 主机");
    println!("  {{redis_port}} - Redis 端口");
    println!("  {{api_port}} - API 端口");
    println!("  {{admin_port}} - 管理端口");

    // 清理临时文件
    let _ = std::fs::remove_file(config_path);

    println!("\n=== 模板最佳实践 ===");
    println!("- 使用有意义的变量名");
    println!("- 提供合理的默认值");
    println!("- 添加注释说明配置项的用途");
    println!("- 按功能分组配置项");
    println!("- 使用环境特定的模板");

    Ok(())
}
