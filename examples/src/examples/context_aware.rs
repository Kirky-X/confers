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
use std::sync::Arc;

fn main() -> anyhow::Result<()> {
    println!("========================================");
    println!("  Context-Aware - context-aware config demo");
    println!("========================================");

    // 1. ContextValue 类型转换与访问
    println!("\n[ContextValue types]");
    let s: ContextValue = "production".into();
    let n: ContextValue = 8080i64.into();
    let b: ContextValue = true.into();
    println!("  String: {:?} (as_str={:?})", s, s.as_str());
    println!("  Number: {:?} (as_number={:?})", n, n.as_number());
    println!("  Bool: {:?} (as_bool={:?})", b, b.as_bool());

    // 2. 使用 ContextAwareFieldBuilder 构建上下文感知字段
    println!("\n[Build context-aware fields - upload limit]");
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
    println!("  free plan: 100MB");
    println!("  pro plan: 1GB");
    println!("  enterprise plan: 10GB");

    // 3. 不同上下文解析
    println!("\n[Resolve by plan]");
    let free_ctx = EvaluationContext::new().attr("plan", "free");
    let pro_ctx = EvaluationContext::new().attr("plan", "pro");
    let enterprise_ctx = EvaluationContext::new().attr("plan", "enterprise");
    println!("  free: {} bytes", upload_limit.evaluate(&free_ctx));
    println!("  pro: {} bytes", upload_limit.evaluate(&pro_ctx));
    println!(
        "  enterprise: {} bytes",
        upload_limit.evaluate(&enterprise_ctx)
    );

    // 4. 基于环境属性解析日志级别
    println!("\n[Resolve log level by environment]");
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

    // 5. 基于区域解析 API 端点
    println!("\n[Resolve API endpoints by region]");
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

    // 6. 基于 targeting_key 解析特性开关
    println!("\n[Resolve feature flags by target key]");
    let feature_flag: ContextAwareField<bool> =
        ContextAwareField::new(false).when(|ctx| ctx.targeting_key() == Some("admin"), true);
    let admin_ctx = EvaluationContext::new().with_key("admin");
    let user_ctx = EvaluationContext::new().with_key("user-123");
    println!("  admin: {}", feature_flag.evaluate(&admin_ctx));
    println!("  user: {}", feature_flag.evaluate(&user_ctx));

    // 7. ContextRule 带描述
    println!("\n[ContextRule description]");
    let rule: ContextRule<u32> =
        ContextRule::new(|ctx| ctx.attributes().get("tier").is_some(), 100u32)
            .with_description("returns 100 when the tier attribute is present");
    let ctx_with_tier = EvaluationContext::new().attr("tier", "premium");
    println!("  rule matches: {}", rule.matches(&ctx_with_tier));
    println!("  rule value: {}", rule.value());

    println!("\n========================================");
    println!("  Example completed!");
    println!("========================================");
    Ok(())
}
