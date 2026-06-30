//! Context-aware configuration module.
//!
//! This module provides the ability to evaluate configuration values based on
//! runtime context (user plan, region, experiment group, etc.), supporting
//! multi-tenant, progressive rollout, and A/B testing scenarios.
//!
//! # Example
//!
//! ```
//! use confers::context::{ContextAwareField, ContextValue, EvaluationContext};
//!
//! let upload_limit = ContextAwareField::new(100 * 1024 * 1024)
//!     .when(
//!         |ctx| ctx.attributes().get("plan") == Some(&ContextValue::String("enterprise".into())),
//!         10i64 * 1024 * 1024 * 1024
//!     );
//!
//! let ctx = EvaluationContext::new()
//!     .attr("plan", "enterprise");
//!
//! assert_eq!(upload_limit.evaluate(&ctx), &(10i64 * 1024 * 1024 * 1024));
//! ```

use std::collections::HashMap;
use std::sync::Arc;

/// Context value types supported in evaluation.
#[derive(Debug, Clone, PartialEq)]
pub enum ContextValue {
    String(Arc<str>),
    Number(f64),
    Boolean(bool),
}

impl ContextValue {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            ContextValue::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_number(&self) -> Option<f64> {
        match self {
            ContextValue::Number(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            ContextValue::Boolean(b) => Some(*b),
            _ => None,
        }
    }
}

impl From<&str> for ContextValue {
    fn from(s: &str) -> Self {
        ContextValue::String(s.into())
    }
}

impl From<String> for ContextValue {
    fn from(s: String) -> Self {
        ContextValue::String(s.into())
    }
}

impl From<Arc<str>> for ContextValue {
    fn from(s: Arc<str>) -> Self {
        ContextValue::String(s)
    }
}

impl From<i64> for ContextValue {
    fn from(n: i64) -> Self {
        ContextValue::Number(n as f64)
    }
}

impl From<i32> for ContextValue {
    fn from(n: i32) -> Self {
        ContextValue::Number(n as f64)
    }
}

impl From<f64> for ContextValue {
    fn from(n: f64) -> Self {
        ContextValue::Number(n)
    }
}

impl From<bool> for ContextValue {
    fn from(b: bool) -> Self {
        ContextValue::Boolean(b)
    }
}

/// Evaluation context containing request-level attributes.
///
/// This context is used to evaluate context-aware configuration fields.
#[derive(Debug, Clone)]
pub struct EvaluationContext {
    targeting_key: Option<String>,
    attributes: HashMap<Arc<str>, ContextValue>,
    environment: Arc<str>,
    region: Arc<str>,
}

impl Default for EvaluationContext {
    fn default() -> Self {
        Self::new()
    }
}

impl EvaluationContext {
    pub fn new() -> Self {
        Self {
            targeting_key: None,
            attributes: HashMap::new(),
            environment: Arc::from("default"),
            region: Arc::from("default"),
        }
    }

    pub fn with_key(mut self, key: impl Into<String>) -> Self {
        self.targeting_key = Some(key.into());
        self
    }

    pub fn targeting_key(&self) -> Option<&str> {
        self.targeting_key.as_deref()
    }

    pub fn attr(mut self, k: impl Into<Arc<str>>, v: impl Into<ContextValue>) -> Self {
        self.attributes.insert(k.into(), v.into());
        self
    }

    pub fn attributes(&self) -> &HashMap<Arc<str>, ContextValue> {
        &self.attributes
    }

    pub fn with_environment(mut self, env: impl Into<Arc<str>>) -> Self {
        self.environment = env.into();
        self
    }

    pub fn environment(&self) -> &Arc<str> {
        &self.environment
    }

    pub fn with_region(mut self, region: impl Into<Arc<str>>) -> Self {
        self.region = region.into();
        self
    }

    pub fn region(&self) -> &Arc<str> {
        &self.region
    }
}

/// Context rule for conditional configuration.
#[derive(Clone)]
pub struct ContextRule<T> {
    predicate: Arc<dyn Fn(&EvaluationContext) -> bool + Send + Sync>,
    value: T,
    description: Option<String>,
}

impl<T> ContextRule<T> {
    pub fn new<F>(predicate: F, value: T) -> Self
    where
        F: Fn(&EvaluationContext) -> bool + Send + Sync + 'static,
    {
        Self {
            predicate: Arc::new(predicate),
            value,
            description: None,
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn matches(&self, ctx: &EvaluationContext) -> bool {
        (self.predicate)(ctx)
    }

    pub fn value(&self) -> &T {
        &self.value
    }
}

/// Context-aware configuration field that evaluates to different values
/// based on the evaluation context.
#[derive(Clone)]
pub struct ContextAwareField<T> {
    rules: Vec<ContextRule<T>>,
    default: T,
}

/// Builder for ContextAwareField.
pub struct ContextAwareFieldBuilder<T> {
    rules: Vec<ContextRule<T>>,
    default: Option<T>,
}

impl<T: Clone + Send + Sync + 'static> ContextAwareFieldBuilder<T> {
    pub fn default(mut self, default: T) -> Self {
        self.default = Some(default);
        self
    }

    pub fn when<F>(mut self, predicate: F, value: T) -> Self
    where
        F: Fn(&EvaluationContext) -> bool + Send + Sync + 'static,
    {
        self.rules.push(ContextRule::new(predicate, value));
        self
    }

    pub fn build(self) -> ContextAwareField<T> {
        ContextAwareField {
            rules: self.rules,
            default: self
                .default
                .unwrap_or_else(|| panic!("Default value required")),
        }
    }
}

impl<T: Clone + Send + Sync + 'static> ContextAwareField<T> {
    pub fn new(default: T) -> Self {
        Self {
            rules: Vec::new(),
            default,
        }
    }

    pub fn builder() -> ContextAwareFieldBuilder<T> {
        ContextAwareFieldBuilder {
            rules: Vec::new(),
            default: None,
        }
    }

    /// Construct with a default value. This is a thin wrapper around [`Self::new`].
    #[deprecated(
        since = "0.1.0",
        note = "Use `ContextAwareFieldBuilder::new(default)` directly instead."
    )]
    pub fn with_default(default: T) -> Self {
        Self::new(default)
    }

    pub fn when<F>(mut self, predicate: F, value: T) -> Self
    where
        F: Fn(&EvaluationContext) -> bool + Send + Sync + 'static,
    {
        self.rules.push(ContextRule::new(predicate, value));
        self
    }

    pub fn evaluate(&self, ctx: &EvaluationContext) -> &T {
        self.rules
            .iter()
            .find(|r| r.matches(ctx))
            .map(|r| r.value())
            .unwrap_or(&self.default)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_value_from_string() {
        let cv: ContextValue = "test".into();
        assert!(matches!(cv, ContextValue::String(s) if s.as_ref() == "test"));
    }

    #[test]
    fn test_context_value_from_bool() {
        let cv: ContextValue = true.into();
        assert!(matches!(cv, ContextValue::Boolean(true)));
    }

    #[test]
    fn test_context_value_from_i64() {
        let cv: ContextValue = 42i64.into();
        assert!(matches!(cv, ContextValue::Number(n) if n == 42.0));
    }

    #[test]
    fn test_context_value_from_f64() {
        let cv: ContextValue = std::f64::consts::PI.into();
        assert!(matches!(cv, ContextValue::Number(n) if (n - std::f64::consts::PI).abs() < 0.001));
    }

    #[test]
    fn test_evaluation_context_default() {
        let ctx = EvaluationContext::new();
        assert!(ctx.targeting_key().is_none());
        assert!(ctx.attributes().is_empty());
        assert_eq!(*ctx.environment(), Arc::from("default"));
        assert_eq!(*ctx.region(), Arc::from("default"));
    }

    #[test]
    fn test_evaluation_context_with_key() {
        let ctx = EvaluationContext::new().with_key("user-123");
        assert_eq!(ctx.targeting_key(), Some("user-123"));
    }

    #[test]
    fn test_evaluation_context_fluent_attr() {
        let ctx = EvaluationContext::new()
            .with_key("user-123")
            .attr("plan", ContextValue::String("enterprise".into()))
            .attr("age", ContextValue::Number(25.0));

        assert_eq!(ctx.targeting_key(), Some("user-123"));
        assert_eq!(ctx.attributes().len(), 2);
    }

    #[test]
    fn test_evaluation_context_environment_region() {
        let ctx = EvaluationContext::new()
            .with_environment("production")
            .with_region("us-west-2");

        assert_eq!(*ctx.environment(), Arc::from("production"));
        assert_eq!(*ctx.region(), Arc::from("us-west-2"));
    }

    #[test]
    fn test_context_aware_field_default() {
        let field = ContextAwareField::new(100u64);
        let ctx = EvaluationContext::new();
        assert_eq!(field.evaluate(&ctx), &100u64);
    }

    #[test]
    fn test_context_aware_field_with_rule() {
        let field = ContextAwareField::new(100u64).when(
            |ctx| ctx.attributes().get("plan") == Some(&ContextValue::String("enterprise".into())),
            1000u64,
        );

        let ctx = EvaluationContext::new().attr("plan", ContextValue::String("enterprise".into()));

        assert_eq!(field.evaluate(&ctx), &1000u64);
    }

    #[test]
    fn test_context_aware_field_no_match() {
        let field = ContextAwareField::new(100u64).when(
            |ctx| ctx.attributes().get("plan") == Some(&ContextValue::String("enterprise".into())),
            1000u64,
        );

        let ctx = EvaluationContext::new().attr("plan", ContextValue::String("basic".into()));

        assert_eq!(field.evaluate(&ctx), &100u64);
    }

    #[test]
    fn test_context_aware_field_first_rule_wins() {
        let field = ContextAwareField::new(100u64)
            .when(
                |ctx| {
                    ctx.attributes().get("plan") == Some(&ContextValue::String("enterprise".into()))
                },
                1000u64,
            )
            .when(
                |ctx| {
                    ctx.attributes().get("plan") == Some(&ContextValue::String("enterprise".into()))
                },
                2000u64,
            );

        let ctx = EvaluationContext::new().attr("plan", ContextValue::String("enterprise".into()));

        assert_eq!(field.evaluate(&ctx), &1000u64);
    }

    #[test]
    fn test_context_aware_field_clone() {
        let field1 = ContextAwareField::new(100u64).when(
            |ctx| ctx.attributes().get("plan") == Some(&ContextValue::String("enterprise".into())),
            1000u64,
        );

        let field2 = field1.clone();

        let ctx = EvaluationContext::new().attr("plan", ContextValue::String("enterprise".into()));

        assert_eq!(field1.evaluate(&ctx), field2.evaluate(&ctx));
    }

    // =============================================================================
    // ContextValue accessor methods
    // =============================================================================

    #[test]
    fn test_context_value_as_str_string() {
        let cv = ContextValue::String("hello".into());
        assert_eq!(cv.as_str(), Some("hello"));
    }

    #[test]
    fn test_context_value_as_str_non_string_returns_none() {
        let cv = ContextValue::Number(42.0);
        assert_eq!(cv.as_str(), None);

        let cv = ContextValue::Boolean(true);
        assert_eq!(cv.as_str(), None);
    }

    #[test]
    fn test_context_value_as_number() {
        let cv = ContextValue::Number(2.5);
        assert_eq!(cv.as_number(), Some(2.5));
    }

    #[test]
    fn test_context_value_as_number_non_number_returns_none() {
        let cv = ContextValue::String("x".into());
        assert_eq!(cv.as_number(), None);

        let cv = ContextValue::Boolean(false);
        assert_eq!(cv.as_number(), None);
    }

    #[test]
    fn test_context_value_as_bool() {
        let cv = ContextValue::Boolean(true);
        assert_eq!(cv.as_bool(), Some(true));

        let cv = ContextValue::Boolean(false);
        assert_eq!(cv.as_bool(), Some(false));
    }

    #[test]
    fn test_context_value_as_bool_non_bool_returns_none() {
        let cv = ContextValue::String("x".into());
        assert_eq!(cv.as_bool(), None);

        let cv = ContextValue::Number(1.0);
        assert_eq!(cv.as_bool(), None);
    }

    // =============================================================================
    // ContextValue From impls
    // =============================================================================

    #[test]
    fn test_context_value_from_owned_string() {
        let cv: ContextValue = "hello".to_string().into();
        assert!(matches!(cv, ContextValue::String(s) if s.as_ref() == "hello"));
    }

    #[test]
    fn test_context_value_from_arc_str() {
        let arc: Arc<str> = Arc::from("arc_value");
        let cv: ContextValue = arc.into();
        assert!(matches!(cv, ContextValue::String(s) if s.as_ref() == "arc_value"));
    }

    #[test]
    fn test_context_value_from_i32() {
        let cv: ContextValue = 100i32.into();
        assert!(matches!(cv, ContextValue::Number(n) if n == 100.0));
    }

    // =============================================================================
    // ContextValue Debug and PartialEq
    // =============================================================================

    #[test]
    fn test_context_value_debug_format() {
        let cv = ContextValue::String("test".into());
        let dbg = format!("{:?}", cv);
        assert!(dbg.contains("String"));
        assert!(dbg.contains("test"));
    }

    #[test]
    fn test_context_value_partial_eq() {
        let a = ContextValue::Number(1.0);
        let b = ContextValue::Number(1.0);
        assert_eq!(a, b);

        let c = ContextValue::Number(2.0);
        assert_ne!(a, c);

        let d = ContextValue::String("x".into());
        assert_ne!(a, d);
    }

    // =============================================================================
    // EvaluationContext Default trait
    // =============================================================================

    #[test]
    fn test_evaluation_context_default_trait() {
        let ctx = EvaluationContext::default();
        assert!(ctx.targeting_key().is_none());
        assert_eq!(*ctx.environment(), Arc::from("default"));
        assert_eq!(*ctx.region(), Arc::from("default"));
        assert!(ctx.attributes().is_empty());
    }

    #[test]
    fn test_evaluation_context_debug_format() {
        let ctx = EvaluationContext::new().with_key("user-1");
        let dbg = format!("{:?}", ctx);
        assert!(dbg.contains("EvaluationContext"));
    }

    #[test]
    fn test_evaluation_context_clone() {
        let ctx1 = EvaluationContext::new()
            .with_key("user-1")
            .attr("plan", "enterprise")
            .with_environment("prod")
            .with_region("us-east-1");
        let ctx2 = ctx1.clone();
        assert_eq!(ctx1.targeting_key(), ctx2.targeting_key());
        assert_eq!(ctx1.attributes().len(), ctx2.attributes().len());
        assert_eq!(*ctx1.environment(), *ctx2.environment());
        assert_eq!(*ctx1.region(), *ctx2.region());
    }

    // =============================================================================
    // ContextRule
    // =============================================================================

    #[test]
    fn test_context_rule_with_description() {
        let rule: ContextRule<u32> =
            ContextRule::new(|_| true, 42).with_description("enterprise plan rule");
        assert_eq!(*rule.value(), 42);
    }

    #[test]
    fn test_context_rule_matches_true() {
        let rule: ContextRule<u32> =
            ContextRule::new(|ctx| ctx.attributes().get("tier").is_some(), 100);
        let ctx = EvaluationContext::new().attr("tier", "premium");
        assert!(rule.matches(&ctx));
    }

    #[test]
    fn test_context_rule_matches_false() {
        let rule: ContextRule<u32> =
            ContextRule::new(|ctx| ctx.attributes().get("tier").is_some(), 100);
        let ctx = EvaluationContext::new();
        assert!(!rule.matches(&ctx));
    }

    #[test]
    fn test_context_rule_clone() {
        let rule1: ContextRule<u32> = ContextRule::new(|_| true, 42);
        let rule2 = rule1.clone();
        let ctx = EvaluationContext::new();
        assert_eq!(*rule1.value(), *rule2.value());
        assert_eq!(rule1.matches(&ctx), rule2.matches(&ctx));
    }

    // =============================================================================
    // ContextAwareFieldBuilder
    // =============================================================================

    #[test]
    fn test_context_aware_field_builder_with_default() {
        let field: ContextAwareField<u32> = ContextAwareField::builder()
            .default(10)
            .when(|_| true, 20)
            .build();
        let ctx = EvaluationContext::new();
        // Rule matches (always true), so should return rule value
        assert_eq!(field.evaluate(&ctx), &20);
    }

    #[test]
    fn test_context_aware_field_builder_default_only() {
        let field: ContextAwareField<u32> = ContextAwareField::builder().default(99).build();
        let ctx = EvaluationContext::new();
        assert_eq!(field.evaluate(&ctx), &99);
    }

    #[test]
    fn test_context_aware_field_builder_no_rules_uses_default() {
        let field: ContextAwareField<String> = ContextAwareField::builder()
            .default("fallback".into())
            .build();
        let ctx = EvaluationContext::new().attr("plan", "enterprise");
        assert_eq!(field.evaluate(&ctx), "fallback");
    }

    #[test]
    #[should_panic(expected = "Default value required")]
    fn test_context_aware_field_builder_build_without_default_panics() {
        let _: ContextAwareField<u32> = ContextAwareField::builder().build();
    }

    #[test]
    fn test_context_aware_field_builder_multiple_rules_first_wins() {
        let field: ContextAwareField<u32> = ContextAwareField::builder()
            .default(0)
            .when(
                |ctx| {
                    ctx.attributes().get("plan") == Some(&ContextValue::String("enterprise".into()))
                },
                100,
            )
            .when(
                |ctx| {
                    ctx.attributes().get("plan") == Some(&ContextValue::String("enterprise".into()))
                },
                200,
            )
            .build();

        let ctx = EvaluationContext::new().attr("plan", ContextValue::String("enterprise".into()));
        assert_eq!(field.evaluate(&ctx), &100);
    }

    // =============================================================================
    // ContextAwareField::with_default (deprecated)
    // =============================================================================

    #[test]
    fn test_context_aware_field_with_default_deprecated() {
        #[allow(deprecated)]
        let field: ContextAwareField<u32> = ContextAwareField::with_default(42);
        let ctx = EvaluationContext::new();
        assert_eq!(field.evaluate(&ctx), &42);
    }

    // =============================================================================
    // ContextAwareField with multiple rule types
    // =============================================================================

    #[test]
    fn test_context_aware_field_with_string_values() {
        let field: ContextAwareField<String> = ContextAwareField::new("default".into())
            .when(
                |ctx| ctx.attributes().get("env") == Some(&ContextValue::String("prod".into())),
                "production".into(),
            )
            .when(
                |ctx| ctx.attributes().get("env") == Some(&ContextValue::String("staging".into())),
                "staging".into(),
            );

        let prod_ctx = EvaluationContext::new().attr("env", ContextValue::String("prod".into()));
        assert_eq!(field.evaluate(&prod_ctx), "production");

        let staging_ctx =
            EvaluationContext::new().attr("env", ContextValue::String("staging".into()));
        assert_eq!(field.evaluate(&staging_ctx), "staging");

        let dev_ctx = EvaluationContext::new().attr("env", ContextValue::String("dev".into()));
        assert_eq!(field.evaluate(&dev_ctx), "default");
    }

    #[test]
    fn test_context_aware_field_with_boolean_values() {
        let field: ContextAwareField<bool> = ContextAwareField::new(false).when(
            |ctx| ctx.attributes().get("feature_enabled") == Some(&ContextValue::Boolean(true)),
            true,
        );

        let enabled_ctx =
            EvaluationContext::new().attr("feature_enabled", ContextValue::Boolean(true));
        assert!(*field.evaluate(&enabled_ctx));

        let disabled_ctx =
            EvaluationContext::new().attr("feature_enabled", ContextValue::Boolean(false));
        assert!(!*field.evaluate(&disabled_ctx));
    }

    #[test]
    fn test_context_aware_field_evaluate_uses_environment() {
        let field: ContextAwareField<u32> = ContextAwareField::new(100)
            .when(|ctx| *ctx.environment() == Arc::from("production"), 1000);

        let prod_ctx = EvaluationContext::new().with_environment("production");
        assert_eq!(field.evaluate(&prod_ctx), &1000);

        let dev_ctx = EvaluationContext::new().with_environment("development");
        assert_eq!(field.evaluate(&dev_ctx), &100);
    }

    #[test]
    fn test_context_aware_field_evaluate_uses_region() {
        let field: ContextAwareField<u32> =
            ContextAwareField::new(100).when(|ctx| *ctx.region() == Arc::from("us-east-1"), 200);

        let us_ctx = EvaluationContext::new().with_region("us-east-1");
        assert_eq!(field.evaluate(&us_ctx), &200);

        let eu_ctx = EvaluationContext::new().with_region("eu-west-1");
        assert_eq!(field.evaluate(&eu_ctx), &100);
    }

    #[test]
    fn test_context_aware_field_evaluate_uses_targeting_key() {
        let field: ContextAwareField<u32> =
            ContextAwareField::new(0).when(|ctx| ctx.targeting_key() == Some("admin"), 999);

        let admin_ctx = EvaluationContext::new().with_key("admin");
        assert_eq!(field.evaluate(&admin_ctx), &999);

        let user_ctx = EvaluationContext::new().with_key("user");
        assert_eq!(field.evaluate(&user_ctx), &0);
    }

    #[test]
    fn test_context_aware_field_no_rules_returns_default() {
        let field: ContextAwareField<u32> = ContextAwareField::new(42);
        let ctx = EvaluationContext::new()
            .attr("plan", "enterprise")
            .with_key("admin")
            .with_environment("prod");
        // No rules, should return default
        assert_eq!(field.evaluate(&ctx), &42);
    }

    // =============================================================================
    // EvaluationContext attr with different value types
    // =============================================================================

    #[test]
    fn test_evaluation_context_attr_with_string_value() {
        let ctx = EvaluationContext::new().attr("name", ContextValue::String("test".into()));
        assert_eq!(ctx.attributes().len(), 1);
        assert!(matches!(
            ctx.attributes().get("name"),
            Some(ContextValue::String(_))
        ));
    }

    #[test]
    fn test_evaluation_context_attr_with_bool_value() {
        let ctx = EvaluationContext::new().attr("enabled", true);
        assert_eq!(ctx.attributes().len(), 1);
        assert!(matches!(
            ctx.attributes().get("enabled"),
            Some(ContextValue::Boolean(true))
        ));
    }

    #[test]
    fn test_evaluation_context_attr_with_number_value() {
        let ctx = EvaluationContext::new().attr("count", 42i64);
        assert_eq!(ctx.attributes().len(), 1);
        assert!(matches!(
            ctx.attributes().get("count"),
            Some(ContextValue::Number(n)) if *n == 42.0
        ));
    }

    #[test]
    fn test_evaluation_context_attr_overwrites() {
        let ctx = EvaluationContext::new()
            .attr("key", "first")
            .attr("key", "second");
        assert_eq!(ctx.attributes().len(), 1);
        assert_eq!(
            ctx.attributes().get("key"),
            Some(&ContextValue::String("second".into()))
        );
    }

    #[test]
    fn test_evaluation_context_multiple_attributes() {
        let ctx = EvaluationContext::new()
            .attr("a", 1i64)
            .attr("b", "two")
            .attr("c", true)
            .attr("d", 2.5f64);
        assert_eq!(ctx.attributes().len(), 4);
    }
}
