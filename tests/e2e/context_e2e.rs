// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! E2E: 上下文感知(tests/e2e/context_e2e.rs)
//!
//! 场景固化(docs/TEST_SCENARIOS.md §2.16):
//! - CTX-01/06 `EvaluationContext` 默认/with_key/attr/clone/debug 与
//!   environment/region 维度切换
//! - CTX-02/03/04/05 `ContextAwareField` 真实业务流:规则命中返回规则值、
//!   无命中返回 default、多规则按序首条命中(优先序固化)、evaluate 引用语义
//! - CTX-07/08 upload-limit 用例端到端 + `Send+Sync` 跨线程求值
//!
//! CTX-09(context-aware+dynamic 组合)固化于 combo_e2e.rs(CMP-17)。
//! 既有覆盖(tests/core/context.rs)为孤立函数级;本文件固化跨 API 端到端流。

use confers::context::{ContextAwareField, ContextRule, ContextValue, EvaluationContext};
use std::sync::Arc;

#[test]
fn ctx0106_evaluation_context_dimensions() {
    // 默认上下文(CTX-01)。
    let ctx = EvaluationContext::new();
    assert!(ctx.targeting_key().is_none());
    assert!(ctx.attributes().is_empty());
    assert_eq!(ctx.environment().as_ref(), "default");
    assert_eq!(ctx.region().as_ref(), "default");

    // 完整维度构建 + clone/debug(CTX-01)。
    let full = EvaluationContext::new()
        .with_key("user-42")
        .attr("tier", "premium")
        .with_environment("production")
        .with_region("cn-north-1");
    assert_eq!(full.targeting_key(), Some("user-42"));
    assert_eq!(full.environment().as_ref(), "production");
    assert_eq!(full.region().as_ref(), "cn-north-1");
    let cloned = full.clone();
    assert_eq!(cloned.targeting_key(), Some("user-42"));
    assert!(!format!("{full:?}").is_empty());

    // environment/region 维度切换取同一字段不同结论(CTX-06)由 ctx02 验证。
}

#[test]
fn ctx02030405_field_rules_priority_and_default() {
    // 真实业务流:按 region/environment 决定上传上限(CTX-02/06)。
    let upload_limit = ContextAwareField::builder()
        .default(100u64)
        .when(|ctx| ctx.region().as_ref() == "cn-north-1", 1024)
        .when(|ctx| ctx.environment().as_ref() == "dev", 16)
        .build();

    // 规则命中 → 规则值(CTX-02)。
    let cn_prod = EvaluationContext::new().with_region("cn-north-1");
    assert_eq!(*upload_limit.evaluate(&cn_prod), 1024);

    // 无规则命中 → default(CTX-03)。
    let us_prod = EvaluationContext::new()
        .with_region("us-east-1")
        .with_environment("production");
    assert_eq!(*upload_limit.evaluate(&us_prod), 100);

    // 多规则按序求值:第一条命中即返回(优先序固化,CTX-04)。
    // cn-north-1 同时满足第二条(dev),但首条规则先命中 → 1024 而非 16。
    let cn_dev = EvaluationContext::new()
        .with_region("cn-north-1")
        .with_environment("dev");
    assert_eq!(
        *upload_limit.evaluate(&cn_dev),
        1024,
        "first matching rule wins"
    );

    // evaluate 返回引用语义:对 default 路径的引用指向字段内部值(CTX-05)。
    let reference = upload_limit.evaluate(&us_prod);
    let again = upload_limit.evaluate(&us_prod);
    assert!(
        std::ptr::eq(reference, again),
        "same evaluation must yield same reference"
    );

    // ContextRule 直接构造与描述(MIG 文档同构的规则层 API)。
    let rule = ContextRule::new(
        |ctx: &EvaluationContext| ctx.environment().as_ref() == "dev",
        16u64,
    )
    .with_description("dev quota");
    let dev = EvaluationContext::new().with_environment("dev");
    let prod = EvaluationContext::new().with_environment("production");
    assert!(rule.matches(&dev));
    assert!(!rule.matches(&prod));
    assert_eq!(*rule.value(), 16);

    // ContextValue 载体(CTX-01 属性面):attr 存入可读回。
    let attr_ctx = EvaluationContext::new().attr("tenant", "acme");
    let tenant = attr_ctx.attributes().get("tenant").cloned();
    assert!(matches!(tenant, Some(ContextValue::String(s)) if s.as_ref() == "acme"));
}

#[test]
fn ctx0708_upload_limit_use_case_and_send_sync() {
    // CTX-07:按 region 限流上传上限(用例端到端)。
    let upload_limit = Arc::new(
        ContextAwareField::builder()
            .default(50u64)
            .when(|ctx| ctx.region().as_ref() == "cn", 200)
            .when(|ctx| ctx.region().as_ref() == "us", 150)
            .build(),
    );

    let uploads: Vec<(&str, u64)> = vec![("cn", 200), ("us", 150), ("eu", 50)];
    for (region, expected) in uploads {
        let ctx = EvaluationContext::new().with_region(region);
        assert_eq!(*upload_limit.evaluate(&ctx), expected, "region {region}");
    }

    // CTX-08:Send+Sync,多线程并发求值结果一致。
    let handles: Vec<_> = (0..8)
        .map(|i| {
            let field = Arc::clone(&upload_limit);
            std::thread::spawn(move || {
                let region = if i % 2 == 0 { "cn" } else { "eu" };
                let ctx = EvaluationContext::new().with_region(region);
                let expected = if i % 2 == 0 { 200 } else { 50 };
                assert_eq!(*field.evaluate(&ctx), expected);
            })
        })
        .collect();
    for handle in handles {
        handle.join().expect("worker thread must not panic");
    }
}
