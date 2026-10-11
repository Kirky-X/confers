// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! 本示例展示 confers 的变量插值功能：
//! - 使用 `${VAR}` 语法引用变量
//! - 使用 `${VAR:default}` 提供默认值
//! - 嵌套引用与 URL 默认值
//! - 使用 `interpolate()` 进行基本插值
//! - 使用 `interpolate_tracked()` 跟踪引用来源
//! - 使用 `InterpolationConfig` 与 `InterpolationContext` 管理敏感变量

use confers::i18n::{tr, tr_args};
use confers::interpolation::{
    InterpolationConfig, InterpolationContext, InterpolationWarning, interpolate,
    interpolate_tracked,
};
use std::collections::HashMap;

fn main() -> anyhow::Result<()> {
    println!("========================================");
    println!("  {}", tr("example-interpolation-title"));
    println!("========================================");

    // 准备变量表（resolver 将从中查找变量）
    let mut vars: HashMap<String, String> = HashMap::new();
    vars.insert("HOST".to_string(), "localhost".to_string());
    vars.insert("PORT".to_string(), "8080".to_string());
    vars.insert("DOMAIN".to_string(), "example.com".to_string());
    vars.insert("API_KEY".to_string(), "secret123".to_string());
    vars.insert("DB_HOST".to_string(), "${DOMAIN}".to_string()); // 嵌套引用

    let resolver = |key: &str| vars.get(key).cloned();

    println!("\n{}", tr("example-interpolation-basic"));
    let template = "Server: ${HOST}:${PORT}";
    let result = interpolate(template, &resolver)?;
    println!(
        "  {}",
        tr_args(
            "example-interpolation-template",
            &[("template", template.to_string())]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-interpolation-result",
            &[("result", result.to_string())]
        )
    );

    println!("\n{}", tr("example-interpolation-default-values"));
    let template = "Timeout: ${TIMEOUT:30}";
    let result = interpolate(template, &resolver)?;
    println!(
        "  {}",
        tr_args(
            "example-interpolation-template",
            &[("template", template.to_string())]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-interpolation-result",
            &[("result", result.to_string())]
        )
    );

    println!("\n{}", tr("example-interpolation-nested-references"));
    let template = "Database: ${DB_HOST}";
    let result = interpolate(template, &resolver)?;
    println!(
        "  {}",
        tr_args(
            "example-interpolation-template",
            &[("template", template.to_string())]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-interpolation-result",
            &[("result", result.to_string())]
        )
    );

    println!("\n{}", tr("example-interpolation-nesting-in-defaults"));
    let template = "${URL:http://${HOST}:${PORT}}";
    let result = interpolate(template, &resolver)?;
    println!(
        "  {}",
        tr_args(
            "example-interpolation-template",
            &[("template", template.to_string())]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-interpolation-result",
            &[("result", result.to_string())]
        )
    );

    println!("\n{}", tr("example-interpolation-traced-normal"));
    let result = interpolate_tracked("${HOST}:${PORT}", &resolver, false)?;
    println!(
        "  {}",
        tr_args(
            "example-interpolation-value",
            &[("value", result.value.to_string())]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-interpolation-referenced-variables",
            &[(
                "variables",
                format!("{:?}", result.referenced_vars().collect::<Vec<_>>())
            )]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-interpolation-is-sensitive-field",
            &[("sensitive", result.is_sensitive.to_string())]
        )
    );

    println!("\n{}", tr("example-interpolation-traced-sensitive"));
    let result = interpolate_tracked("${API_KEY}", &resolver, true)?;
    println!(
        "  {}",
        tr_args(
            "example-interpolation-value",
            &[("value", result.value.to_string())]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-interpolation-is-sensitive-field",
            &[("sensitive", result.is_sensitive.to_string())]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-interpolation-contains-sensitive-refs",
            &[("contains", result.has_sensitive_refs().to_string())]
        )
    );

    println!("\n{}", tr("example-interpolation-config"));
    let config = InterpolationConfig::new()
        .with_sensitive_var("API_KEY")
        .with_sensitive_var("DB_PASSWORD")
        .with_warn_sensitive(true);
    println!(
        "  {}",
        tr_args(
            "example-interpolation-api-key-sensitive",
            &[("sensitive", config.is_sensitive("API_KEY").to_string())]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-interpolation-db-password-sensitive",
            &[("sensitive", config.is_sensitive("DB_PASSWORD").to_string())]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-interpolation-host-sensitive",
            &[("sensitive", config.is_sensitive("HOST").to_string())]
        )
    );

    println!("\n{}", tr("example-interpolation-batch-context"));
    let mut ctx = InterpolationContext::new();
    let normal = interpolate_tracked("${HOST}:${PORT}", &resolver, false)?;
    ctx.record("server_url", &normal);
    let sensitive = interpolate_tracked("${API_KEY}", &resolver, true)?;
    ctx.record("api_key", &sensitive);
    println!(
        "  {}",
        tr_args(
            "example-interpolation-api-key-sensitive-ref",
            &[("sensitive", ctx.is_sensitive_ref("API_KEY").to_string())]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-interpolation-host-sensitive-ref",
            &[("sensitive", ctx.is_sensitive_ref("HOST").to_string())]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-interpolation-api-key-ref-field",
            &[("field", format!("{:?}", ctx.sensitive_ref_field("API_KEY")))]
        )
    );

    println!("\n{}", tr("example-interpolation-warnings"));
    let warning = InterpolationWarning::SensitiveFieldInterpolation {
        field: "password".to_string(),
        vars: vec!["DB_PASSWORD".to_string()],
    };
    println!(
        "  {}",
        tr_args(
            "example-interpolation-warning",
            &[("warning", warning.to_string())]
        )
    );

    println!("\n{}", tr("example-interpolation-unresolved-errors"));
    match interpolate("${UNDEFINED}", &resolver) {
        Ok(v) => println!(
            "  {}",
            tr_args(
                "example-interpolation-unreachable",
                &[("value", v.to_string())]
            )
        ),
        Err(e) => println!(
            "  {}",
            tr_args(
                "example-interpolation-expected-error",
                &[("error", format!("{e:?}"))]
            )
        ),
    }

    println!("\n========================================");
    println!("  {}", tr("example-interpolation-completed"));
    println!("========================================");
    Ok(())
}
