// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! 高级配置验证示例
//!
//! 展示如何使用增强的配置验证功能，包括范围验证、依赖验证、格式验证和一致性验证。
//!
//! ## 前提条件
//!
//! - 需要启用的特性：`validation`
//!
//! ## 运行方式
//!
//! ```bash
//! cargo run --example 02-validation-advanced_validation --features validation
//! ```

use confers::validator::{
    CachedValidationEngine, ConsistencyValidator, DependencyValidator,
    FormatValidator, RangeFieldValidator, ValidationEngine,
};
use serde_json::{json, Value};

fn main() -> anyhow::Result<()> {
    println!("=== 高级配置验证示例 ===\n");

    // --- 示例 1: 范围验证 ---
    println!("--- 范围验证 ---");
    let mut engine = ValidationEngine::new();
    engine.add_validator(Box::new(RangeFieldValidator::new(
        "port",
        Some(1024.0),
        Some(65535.0),
    )));
    engine.add_validator(Box::new(RangeFieldValidator::new(
        "timeout",
        Some(1.0),
        Some(300.0),
    )));

    let config1 = json!({"port": 8080, "timeout": 30});
    match engine.validate(&config1) {
        Ok(()) => println!("✅ 配置 1 验证通过: {:?}", config1),
        Err(errors) => println!("❌ 配置 1 验证失败: {:?}", errors),
    }

    let config2 = json!({"port": 80, "timeout": 30});
    match engine.validate(&config2) {
        Ok(()) => println!("✅ 配置 2 验证通过: {:?}", config2),
        Err(errors) => {
            println!("❌ 配置 2 验证失败:");
            for error in errors {
                println!("  - {}", error.message);
            }
        }
    }

    // --- 示例 2: 依赖验证 ---
    println!("\n--- 依赖验证 ---");
    let mut engine = ValidationEngine::new();
    engine.add_validator(Box::new(DependencyValidator::new(
        vec!["database.url".to_string(), "database.username".to_string()],
        vec!["database.password".to_string()],
    )));

    let config3 = json!({
        "database": {
            "url": "postgres://localhost:5432/db",
            "username": "admin",
            "password": "secret"
        }
    });
    match engine.validate(&config3) {
        Ok(()) => println!("✅ 配置 3 验证通过: 数据库配置完整"),
        Err(errors) => println!("❌ 配置 3 验证失败: {:?}", errors),
    }

    let config4 = json!({
        "database": {
            "url": "postgres://localhost:5432/db",
            "username": "admin"
        }
    });
    match engine.validate(&config4) {
        Ok(()) => println!("✅ 配置 4 验证通过"),
        Err(errors) => {
            println!("❌ 配置 4 验证失败:");
            for error in errors {
                println!("  - {}", error.message);
            }
        }
    }

    // --- 示例 3: 格式验证 ---
    println!("\n--- 格式验证 ---");
    let mut engine = ValidationEngine::new();
    engine.add_validator(Box::new(FormatValidator::new(
        "email",
        "email".to_string(),
    )));
    engine.add_validator(Box::new(FormatValidator::new(
        "website",
        "url".to_string(),
    )));

    let config5 = json!({
        "email": "admin@example.com",
        "website": "https://example.com"
    });
    match engine.validate(&config5) {
        Ok(()) => println!("✅ 配置 5 验证通过: 格式正确"),
        Err(errors) => println!("❌ 配置 5 验证失败: {:?}", errors),
    }

    let config6 = json!({
        "email": "invalid-email",
        "website": "not-a-url"
    });
    match engine.validate(&config6) {
        Ok(()) => println!("✅ 配置 6 验证通过"),
        Err(errors) => {
            println!("❌ 配置 6 验证失败:");
            for error in errors {
                println!("  - {}", error.message);
            }
        }
    }

    // --- 示例 4: 一致性验证 ---
    println!("\n--- 一致性验证 ---");
    let mut engine = ValidationEngine::new();
    engine.add_validator(Box::new(ConsistencyValidator::new(
        vec!["min_workers".to_string(), "max_workers".to_string()],
        Box::new(|values| {
            let min = values[0].as_f64().unwrap_or(0.0);
            let max = values[1].as_f64().unwrap_or(0.0);
            if min > max {
                return Err("min_workers cannot be greater than max_workers".to_string());
            }
            Ok(())
        }),
    )));

    let config7 = json!({"min_workers": 1, "max_workers": 10});
    match engine.validate(&config7) {
        Ok(()) => println!("✅ 配置 7 验证通过: min <= max"),
        Err(errors) => println!("❌ 配置 7 验证失败: {:?}", errors),
    }

    let config8 = json!({"min_workers": 10, "max_workers": 5});
    match engine.validate(&config8) {
        Ok(()) => println!("✅ 配置 8 验证通过"),
        Err(errors) => {
            println!("❌ 配置 8 验证失败:");
            for error in errors {
                println!("  - {}", error.message);
            }
        }
    }

    // --- 示例 5: 缓存验证 ---
    println!("\n--- 缓存验证 ---");
    let engine = ValidationEngine::new();
    engine.add_validator(Box::new(RangeFieldValidator::new(
        "port",
        Some(1024.0),
        Some(65535.0),
    )));

    let cached_engine = CachedValidationEngine::new(engine);

    let config9 = json!({"port": 8080});
    // 第一次验证
    let start = std::time::Instant::now();
    match cached_engine.validate(&config9) {
        Ok(()) => {
            let duration = start.elapsed();
            println!("✅ 配置 9 验证通过（第一次）: {:?}", duration);
        }
        Err(errors) => println!("❌ 配置 9 验证失败: {:?}", errors),
    }

    // 第二次验证（使用缓存）
    let start = std::time::Instant::now();
    match cached_engine.validate(&config9) {
        Ok(()) => {
            let duration = start.elapsed();
            println!("✅ 配置 9 验证通过（缓存）: {:?}", duration);
        }
        Err(errors) => println!("❌ 配置 9 验证失败: {:?}", errors),
    }

    // --- 示例 6: 综合验证 ---
    println!("\n--- 综合验证 ---");
    let mut engine = ValidationEngine::new();
    engine.add_validator(Box::new(RangeFieldValidator::new(
        "port",
        Some(1024.0),
        Some(65535.0),
    )));
    engine.add_validator(Box::new(FormatValidator::new(
        "admin_email",
        "email".to_string(),
    )));
    engine.add_validator(Box::new(DependencyValidator::new(
        vec!["database.url".to_string()],
        vec!["database.password".to_string()],
    )));

    let config10 = json!({
        "port": 8080,
        "admin_email": "admin@example.com",
        "database": {
            "url": "postgres://localhost:5432/db",
            "password": "secret"
        }
    });
    match engine.validate(&config10) {
        Ok(()) => println!("✅ 配置 10 验证通过: 所有检查通过"),
        Err(errors) => {
            println!("❌ 配置 10 验证失败:");
            for error in errors {
                println!("  - {}", error.message);
            }
        }
    }

    println!("\n=== 高级配置验证示例完成 ===");
    println!("\n安全提示:");
    println!("⚠️  始终验证用户输入");
    println!("⚠️  确保数值在预期范围内");
    println!("⚠️  验证字符串格式（如 URL、邮箱）");
    println!("⚠️  检查配置项之间的依赖关系");
    println!("⚠️  验证配置项之间的一致性");
    println!("⚠️  记录所有验证失败");
    println!("⚠️  将验证失败视为潜在安全事件");

    Ok(())
}