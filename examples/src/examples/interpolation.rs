// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! 本示例展示 confers 的变量插值功能：
//! - 使用 `${VAR}` 语法引用变量
//! - 使用 `${VAR:default}` 提供默认值
//! - 嵌套引用与 URL 默认值
//! - 使用 `interpolate()` 进行基本插值
//! - 使用 `interpolate_tracked()` 跟踪引用来源
//! - 使用 `InterpolationConfig` 与 `InterpolationContext` 管理敏感变量

use confers::interpolation::{
    InterpolationConfig, InterpolationContext, InterpolationWarning, interpolate,
    interpolate_tracked,
};
use std::collections::HashMap;

fn main() -> anyhow::Result<()> {
    println!("========================================");
    println!("  Interpolation - variable interpolation demo");
    println!("========================================");

    // 准备变量表（resolver 将从中查找变量）
    let mut vars: HashMap<String, String> = HashMap::new();
    vars.insert("HOST".to_string(), "localhost".to_string());
    vars.insert("PORT".to_string(), "8080".to_string());
    vars.insert("DOMAIN".to_string(), "example.com".to_string());
    vars.insert("API_KEY".to_string(), "secret123".to_string());
    vars.insert("DB_HOST".to_string(), "${DOMAIN}".to_string()); // 嵌套引用

    let resolver = |key: &str| vars.get(key).cloned();

    // 1. 基本插值
    println!("\n[Basic interpolation]");
    let template = "Server: ${HOST}:${PORT}";
    let result = interpolate(template, &resolver)?;
    println!("  template: {}", template);
    println!("  result: {}", result);

    // 2. 默认值
    println!("\n[default values]");
    let template = "Timeout: ${TIMEOUT:30}";
    let result = interpolate(template, &resolver)?;
    println!("  template: {}", template);
    println!("  result: {}", result);

    // 3. 嵌套引用（DB_HOST -> ${DOMAIN} -> example.com）
    println!("\n[nested references]");
    let template = "Database: ${DB_HOST}";
    let result = interpolate(template, &resolver)?;
    println!("  template: {}", template);
    println!("  result: {}", result);

    // 4. URL 默认值中嵌套
    println!("\n[Nesting in default values]");
    let template = "${URL:http://${HOST}:${PORT}}";
    let result = interpolate(template, &resolver)?;
    println!("  template: {}", template);
    println!("  result: {}", result);

    // 5. 跟踪插值 - 普通字段
    println!("\n[Traced interpolation - normal field]");
    let result = interpolate_tracked("${HOST}:${PORT}", &resolver, false)?;
    println!("  value: {}", result.value);
    println!(
        "  referenced variables: {:?}",
        result.referenced_vars().collect::<Vec<_>>()
    );
    println!("  is sensitive field: {}", result.is_sensitive);

    // 6. 跟踪插值 - 敏感字段
    println!("\n[Traced interpolation - sensitive field]");
    let result = interpolate_tracked("${API_KEY}", &resolver, true)?;
    println!("  value: {}", result.value);
    println!("  is sensitive field: {}", result.is_sensitive);
    println!(
        "  contains sensitive references: {}",
        result.has_sensitive_refs()
    );

    // 7. InterpolationConfig 配置敏感变量
    println!("\n[Interpolation config]");
    let config = InterpolationConfig::new()
        .with_sensitive_var("API_KEY")
        .with_sensitive_var("DB_PASSWORD")
        .with_warn_sensitive(true);
    println!("  API_KEY is sensitive: {}", config.is_sensitive("API_KEY"));
    println!(
        "  DB_PASSWORD is sensitive: {}",
        config.is_sensitive("DB_PASSWORD")
    );
    println!("  HOST is sensitive: {}", config.is_sensitive("HOST"));

    // 8. InterpolationContext 批量跟踪
    println!("\n[Batch interpolation context]");
    let mut ctx = InterpolationContext::new();
    let normal = interpolate_tracked("${HOST}:${PORT}", &resolver, false)?;
    ctx.record("server_url", &normal);
    let sensitive = interpolate_tracked("${API_KEY}", &resolver, true)?;
    ctx.record("api_key", &sensitive);
    println!(
        "  API_KEY is sensitive reference: {}",
        ctx.is_sensitive_ref("API_KEY")
    );
    println!(
        "  HOST is sensitive reference: {}",
        ctx.is_sensitive_ref("HOST")
    );
    println!(
        "  API_KEY referenced from field: {:?}",
        ctx.sensitive_ref_field("API_KEY")
    );

    // 9. 插值警告展示
    println!("\n[Interpolation warnings]");
    let warning = InterpolationWarning::SensitiveFieldInterpolation {
        field: "password".to_string(),
        vars: vec!["DB_PASSWORD".to_string()],
    };
    println!("  warning: {}", warning);

    // 10. 错误处理 - 未解析变量
    println!("\n[Unresolved variable errors]");
    match interpolate("${UNDEFINED}", &resolver) {
        Ok(v) => println!("  unreachable here: {}", v),
        Err(e) => println!("  expected error: {:?}", e),
    }

    println!("\n========================================");
    println!("  Example completed!");
    println!("========================================");
    Ok(())
}
