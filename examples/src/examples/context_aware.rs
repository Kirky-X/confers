// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

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
    println!("  Context-Aware - 上下文感知配置示例");
    println!("========================================");

    // 1. ContextValue 类型转换与访问
    println!("\n[ContextValue 类型]");
    let s: ContextValue = "production".into();
    let n: ContextValue = 8080i64.into();
    let b: ContextValue = true.into();
    println!("  字符串: {:?} (as_str={:?})", s, s.as_str());
    println!("  数字: {:?} (as_number={:?})", n, n.as_number());
    println!("  布尔: {:?} (as_bool={:?})", b, b.as_bool());

    // 2. 使用 ContextAwareFieldBuilder 构建上下文感知字段
    println!("\n[构建上下文感知字段 - 上传限制]");
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
    println!("  free 计划: 100MB");
    println!("  pro 计划: 1GB");
    println!("  enterprise 计划: 10GB");

    // 3. 不同上下文解析
    println!("\n[按计划解析]");
    let free_ctx = EvaluationContext::new().attr("plan", "free");
    let pro_ctx = EvaluationContext::new().attr("plan", "pro");
    let enterprise_ctx = EvaluationContext::new().attr("plan", "enterprise");
    println!("  free: {} 字节", upload_limit.evaluate(&free_ctx));
    println!("  pro: {} 字节", upload_limit.evaluate(&pro_ctx));
    println!(
        "  enterprise: {} 字节",
        upload_limit.evaluate(&enterprise_ctx)
    );

    // 4. 基于环境属性解析日志级别
    println!("\n[按环境解析日志级别]");
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
    println!("\n[按区域解析 API 端点]");
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
    println!("\n[按目标键解析特性开关]");
    let feature_flag: ContextAwareField<bool> =
        ContextAwareField::new(false).when(|ctx| ctx.targeting_key() == Some("admin"), true);
    let admin_ctx = EvaluationContext::new().with_key("admin");
    let user_ctx = EvaluationContext::new().with_key("user-123");
    println!("  admin: {}", feature_flag.evaluate(&admin_ctx));
    println!("  user: {}", feature_flag.evaluate(&user_ctx));

    // 7. ContextRule 带描述
    println!("\n[ContextRule 描述]");
    let rule: ContextRule<u32> =
        ContextRule::new(|ctx| ctx.attributes().get("tier").is_some(), 100u32)
            .with_description("有 tier 属性时返回 100");
    let ctx_with_tier = EvaluationContext::new().attr("tier", "premium");
    println!("  规则匹配: {}", rule.matches(&ctx_with_tier));
    println!("  规则值: {}", rule.value());

    println!("\n========================================");
    println!("  示例运行完成!");
    println!("========================================");
    Ok(())
}
