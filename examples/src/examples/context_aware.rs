// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! 本示例展示 confers 的上下文感知配置功能：
//! - 使用 `ContextAwareFieldBuilder` 构建上下文感知字段
//! - 定义 `ContextRule` 针对不同环境/计划返回不同值
//! - 使用 `EvaluationContext` 携带运行时上下文
//! - 根据上下文解析字段值

use confers::context::{
    ContextAwareField, ContextAwareFieldBuilder, ContextRule, ContextValue, EvaluationContext,
};
use confers::i18n::{tr, tr_args};
use std::sync::Arc;

fn main() -> anyhow::Result<()> {
    println!("========================================");
    println!("  {}", tr("example-context-aware-title"));
    println!("========================================");

    println!("\n{}", tr("example-context-aware-value-types"));
    let s: ContextValue = "production".into();
    let n: ContextValue = 8080i64.into();
    let b: ContextValue = true.into();
    println!(
        "  {}",
        tr_args(
            "example-context-aware-string",
            &[
                ("value", format!("{s:?}")),
                ("as_str", format!("{:?}", s.as_str())),
            ]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-context-aware-number",
            &[
                ("value", format!("{n:?}")),
                ("as_number", format!("{:?}", n.as_number())),
            ]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-context-aware-bool",
            &[
                ("value", format!("{b:?}")),
                ("as_bool", format!("{:?}", b.as_bool())),
            ]
        )
    );

    println!("\n{}", tr("example-context-aware-build-upload-limit"));
    let builder: ContextAwareFieldBuilder<u64> = ContextAwareField::builder();
    let upload_limit: ContextAwareField<u64> = builder
        .default(100 * 1024 * 1024) // 100MB
        .when(
            |ctx| ctx.attributes().get("plan") == Some(&ContextValue::String("pro".into())),
            1024u64 * 1024 * 1024, // 1GB
        )
        .when(
            |ctx| ctx.attributes().get("plan") == Some(&ContextValue::String("enterprise".into())),
            10u64 * 1024 * 1024 * 1024, // 10GB
        )
        .build();
    println!("  {}", tr("example-context-aware-free-plan"));
    println!("  {}", tr("example-context-aware-pro-plan"));
    println!("  {}", tr("example-context-aware-enterprise-plan"));

    println!("\n{}", tr("example-context-aware-resolve-by-plan"));
    let free_ctx = EvaluationContext::new().attr("plan", "free");
    let pro_ctx = EvaluationContext::new().attr("plan", "pro");
    let enterprise_ctx = EvaluationContext::new().attr("plan", "enterprise");
    println!(
        "  {}",
        tr_args(
            "example-context-aware-free-bytes",
            &[("bytes", upload_limit.evaluate(&free_ctx).to_string())]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-context-aware-pro-bytes",
            &[("bytes", upload_limit.evaluate(&pro_ctx).to_string())]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-context-aware-enterprise-bytes",
            &[("bytes", upload_limit.evaluate(&enterprise_ctx).to_string())]
        )
    );

    println!("\n{}", tr("example-context-aware-resolve-log-level"));
    let log_level: ContextAwareField<String> = ContextAwareField::new("info".to_string())
        .when(
            |ctx| *ctx.environment() == Arc::from("production"),
            "warn".to_string(),
        )
        .when(
            |ctx| *ctx.environment() == Arc::from("development"),
            "debug".to_string(),
        );
    let prod_ctx = EvaluationContext::new().with_environment("production");
    let dev_ctx = EvaluationContext::new().with_environment("development");
    let default_ctx = EvaluationContext::new();
    println!("  production: {}", log_level.evaluate(&prod_ctx));
    println!("  development: {}", log_level.evaluate(&dev_ctx));
    println!("  default: {}", log_level.evaluate(&default_ctx));

    println!("\n{}", tr("example-context-aware-resolve-endpoints"));
    let endpoint: ContextAwareField<String> =
        ContextAwareField::new("api.global.example.com".to_string())
            .when(
                |ctx| *ctx.region() == Arc::from("us-east-1"),
                "api.us-east-1.example.com".to_string(),
            )
            .when(
                |ctx| *ctx.region() == Arc::from("eu-west-1"),
                "api.eu-west-1.example.com".to_string(),
            );
    let us_ctx = EvaluationContext::new().with_region("us-east-1");
    let eu_ctx = EvaluationContext::new().with_region("eu-west-1");
    println!("  us-east-1: {}", endpoint.evaluate(&us_ctx));
    println!("  eu-west-1: {}", endpoint.evaluate(&eu_ctx));
    println!("  global: {}", endpoint.evaluate(&EvaluationContext::new()));

    println!("\n{}", tr("example-context-aware-resolve-feature-flags"));
    let feature_flag: ContextAwareField<bool> =
        ContextAwareField::new(false).when(|ctx| ctx.targeting_key() == Some("admin"), true);
    let admin_ctx = EvaluationContext::new().with_key("admin");
    let user_ctx = EvaluationContext::new().with_key("user-123");
    println!("  admin: {}", feature_flag.evaluate(&admin_ctx));
    println!("  user: {}", feature_flag.evaluate(&user_ctx));

    println!("\n{}", tr("example-context-aware-rule-description"));
    let rule: ContextRule<u32> =
        ContextRule::new(|ctx| ctx.attributes().get("tier").is_some(), 100u32)
            .with_description(tr("example-context-aware-rule-desc-text"));
    let ctx_with_tier = EvaluationContext::new().attr("tier", "premium");
    println!(
        "  {}",
        tr_args(
            "example-context-aware-rule-matches",
            &[("matches", rule.matches(&ctx_with_tier).to_string())]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-context-aware-rule-value",
            &[("value", rule.value().to_string())]
        )
    );

    println!("\n========================================");
    println!("  {}", tr("example-context-aware-completed"));
    println!("========================================");
    Ok(())
}
