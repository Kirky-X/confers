//! Integration tests for context-aware configuration.
//!
//! These tests verify the context-aware configuration feature described in
//! dev-v2.md Section 3.10 (lines 902-992).

use confers::context::{ContextAwareField, ContextValue, EvaluationContext};

/// Test that EvaluationContext can be created with default values
#[test]
fn test_evaluation_context_default() {
    let ctx = EvaluationContext::new();
    assert!(ctx.targeting_key().is_none());
    assert!(ctx.attributes().is_empty());
}

/// Test that EvaluationContext can be created with a targeting key
#[test]
fn test_evaluation_context_with_key() {
    let ctx = EvaluationContext::new().with_key("user-123");
    assert_eq!(ctx.targeting_key(), Some("user-123"));
}

/// Test that EvaluationContext can have attributes added fluently
#[test]
fn test_evaluation_context_with_attributes() {
    let ctx = EvaluationContext::new()
        .with_key("user-123")
        .attr("plan", ContextValue::String("enterprise".into()))
        .attr("region", ContextValue::String("us-east-1".into()))
        .attr("age", ContextValue::Number(25.0));

    assert_eq!(ctx.attributes().len(), 3);
    assert_eq!(
        ctx.attributes().get("plan"),
        Some(&ContextValue::String("enterprise".into()))
    );
    assert_eq!(
        ctx.attributes().get("region"),
        Some(&ContextValue::String("us-east-1".into()))
    );
    assert_eq!(
        ctx.attributes().get("age"),
        Some(&ContextValue::Number(25.0))
    );
}

/// Test ContextValue conversion from primitive types
#[test]
fn test_context_value_from_primitives() {
    let cv: ContextValue = "test".into();
    assert!(matches!(cv, ContextValue::String(_)));

    let cv: ContextValue = true.into();
    assert!(matches!(cv, ContextValue::Boolean(true)));

    let cv: ContextValue = 42i64.into();
    assert!(matches!(cv, ContextValue::Number(n) if n == 42.0));

    let cv: ContextValue = 3.14.into();
    assert!(matches!(cv, ContextValue::Number(n) if (n - 3.14).abs() < 0.001));
}

/// Test ContextAwareField returns default value when no rules match
#[test]
fn test_context_aware_field_default() {
    let field = ContextAwareField::new(100u64);

    let ctx = EvaluationContext::new();
    let value = field.evaluate(&ctx);

    assert_eq!(value, &100u64);
}

/// Test ContextAwareField with a matching predicate returns the override value
#[test]
fn test_context_aware_field_with_matching_predicate() {
    let field = ContextAwareField::new(100u64).when(
        |ctx: &EvaluationContext| {
            ctx.attributes().get("plan") == Some(&ContextValue::String("enterprise".into()))
        },
        10_737_418_240u64,
    );

    let ctx = EvaluationContext::new().attr("plan", ContextValue::String("enterprise".into()));

    let value = field.evaluate(&ctx);
    assert_eq!(value, &10_737_418_240u64);
}

/// Test ContextAwareField with non-matching predicate returns default
#[test]
fn test_context_aware_field_no_match_returns_default() {
    let field = ContextAwareField::new(100u64).when(
        |ctx: &EvaluationContext| {
            ctx.attributes().get("plan") == Some(&ContextValue::String("enterprise".into()))
        },
        10_737_418_240u64,
    );

    let ctx = EvaluationContext::new().attr("plan", ContextValue::String("basic".into()));

    let value = field.evaluate(&ctx);
    assert_eq!(value, &100u64);
}

/// Test ContextAwareField with multiple rules - first matching rule wins
#[test]
fn test_context_aware_field_multiple_rules() {
    let field = ContextAwareField::new(100u64)
        .when(
            |ctx: &EvaluationContext| {
                ctx.attributes().get("plan") == Some(&ContextValue::String("basic".into()))
            },
            524_288_000u64,
        )
        .when(
            |ctx: &EvaluationContext| {
                ctx.attributes().get("plan") == Some(&ContextValue::String("pro".into()))
            },
            1_073_741_8240u64,
        )
        .when(
            |ctx: &EvaluationContext| {
                ctx.attributes().get("plan") == Some(&ContextValue::String("enterprise".into()))
            },
            10_737_418_240u64,
        );

    let ctx = EvaluationContext::new().attr("plan", ContextValue::String("enterprise".into()));

    let value = field.evaluate(&ctx);
    assert_eq!(value, &10_737_418_240u64);
}

/// Test ContextAwareField evaluate returns reference to value
#[test]
fn test_context_aware_field_evaluate_returns_reference() {
    let field = ContextAwareField::new(String::from("default"));
    let ctx = EvaluationContext::new();

    let value = field.evaluate(&ctx);

    assert_eq!(value, &String::from("default"));
}

/// Test ContextAwareField is Clone
#[test]
fn test_context_aware_field_clone() {
    let field1 = ContextAwareField::new(100u64).when(
        |ctx: &EvaluationContext| {
            ctx.attributes().get("plan") == Some(&ContextValue::String("enterprise".into()))
        },
        10_737_418_240u64,
    );

    let field2 = field1.clone();

    let ctx = EvaluationContext::new().attr("plan", ContextValue::String("enterprise".into()));

    assert_eq!(field1.evaluate(&ctx), field2.evaluate(&ctx));
}

/// Test EvaluationContext is Clone
#[test]
fn test_evaluation_context_clone() {
    let ctx1 = EvaluationContext::new()
        .with_key("user-123")
        .attr("plan", ContextValue::String("enterprise".into()));

    let ctx2 = ctx1.clone();

    assert_eq!(ctx1.targeting_key(), ctx2.targeting_key());
    assert_eq!(ctx1.attributes().get("plan"), ctx2.attributes().get("plan"));
}

/// Test EvaluationContext Debug implementation
#[test]
fn test_evaluation_context_debug() {
    let ctx = EvaluationContext::new()
        .with_key("user-123")
        .attr("plan", ContextValue::String("enterprise".into()));

    let debug_str = format!("{:?}", ctx);
    assert!(debug_str.contains("EvaluationContext"));
    assert!(debug_str.contains("user-123"));
}

/// Test ContextValue Debug implementation
#[test]
fn test_context_value_debug() {
    let cv_string = ContextValue::String("test".into());
    let cv_number = ContextValue::Number(42.5);
    let cv_bool = ContextValue::Boolean(true);

    assert!(format!("{:?}", cv_string).contains("test"));
    assert!(format!("{:?}", cv_number).contains("42.5"));
    assert!(format!("{:?}", cv_bool).contains("true"));
}

/// Test real-world use case: upload limit based on plan
#[test]
fn test_upload_limit_use_case() {
    let upload_limit: ContextAwareField<u64> = ContextAwareField::new(100 * 1024 * 1024)
        .when(
            |ctx: &EvaluationContext| {
                ctx.attributes().get("plan") == Some(&ContextValue::String("enterprise".into()))
            },
            10 * 1024 * 1024 * 1024,
        )
        .when(
            |ctx: &EvaluationContext| {
                ctx.attributes().get("plan") == Some(&ContextValue::String("pro".into()))
            },
            1 * 1024 * 1024 * 1024,
        )
        .when(
            |ctx: &EvaluationContext| {
                ctx.attributes().get("region") == Some(&ContextValue::String("cn-north".into()))
            },
            500 * 1024 * 1024,
        );

    let ctx = EvaluationContext::new()
        .with_key("user_123")
        .attr("plan", "enterprise")
        .attr("region", "us-east-1");

    assert_eq!(upload_limit.evaluate(&ctx), &(10 * 1024 * 1024 * 1024));

    let ctx = EvaluationContext::new().attr("plan", "pro");
    assert_eq!(upload_limit.evaluate(&ctx), &(1 * 1024 * 1024 * 1024));

    let ctx = EvaluationContext::new().attr("plan", "basic");
    assert_eq!(upload_limit.evaluate(&ctx), &(100 * 1024 * 1024));

    let ctx = EvaluationContext::new()
        .attr("plan", "enterprise")
        .attr("region", "cn-north");
    // First matching rule (enterprise) wins
    assert_eq!(upload_limit.evaluate(&ctx), &(10 * 1024 * 1024 * 1024));
}

/// Test with environment attribute
#[test]
fn test_evaluation_context_environment() {
    let ctx = EvaluationContext::new().with_environment("production");
    assert_eq!(ctx.environment().as_ref(), "production");
}

/// Test with region attribute
#[test]
fn test_evaluation_context_region() {
    let ctx = EvaluationContext::new().with_region("us-west-2");
    assert_eq!(ctx.region().as_ref(), "us-west-2");
}

/// Test ContextAwareField with Send + Sync bounds
#[test]
fn test_context_aware_field_send_sync() {
    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}

    assert_send::<ContextAwareField<u32>>();
    assert_sync::<ContextAwareField<u32>>();
}
