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
//!         10 * 1024 * 1024 * 1024
//!     );
//!
//! let ctx = EvaluationContext::new()
//!     .attr("plan", "enterprise");
//!
//! assert_eq!(upload_limit.evaluate(&ctx), &(10 * 1024 * 1024 * 1024));
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
        let cv: ContextValue = 3.14.into();
        assert!(matches!(cv, ContextValue::Number(n) if (n - 3.14).abs() < 0.001));
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
}
