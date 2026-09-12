// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! OpenFeature-style flag evaluation (`openfeature` feature).
//!
//! Bridges confers' toggle systems onto OpenFeature semantics: a
//! [`FeatureProvider`] resolves flag keys against an
//! [`EvaluationContext`](crate::context::EvaluationContext) and returns
//! [`EvaluationDetail`] (value + variant + reason), exactly one client
//! provider at a time ([`OpenFeatureClient`]) and deterministic rollout
//! buckets from the targeting key ([`StaticFlagProvider`]).
//!
//! - [`NoOpProvider`] — the required no-op default (always the default
//!   value, reason `Default`).
//! - [`StaticFlagProvider`] — declarative flags with attribute-match rules
//!   and percentage rollouts bucketed by `hash(flag_key, targeting_key)`,
//!   so the same identity always lands in the same bucket.
//! - [`ToggleRegistryProvider`] — adapter exposing the legacy
//!   [`FeatureToggleRegistry`](crate::toggle::FeatureToggleRegistry) booleans
//!   through the same port.

use std::collections::HashMap;
use std::sync::Arc;

use crate::context::EvaluationContext;

/// Why a flag resolved the way it did (OpenFeature `Reason` semantics).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolutionReason {
    /// The value is the flag's static default.
    Static,
    /// A targeting rule matched the context.
    TargetingMatch,
    /// No flag existed; the caller's default was returned.
    Default,
    /// Evaluation errored; the caller's default was returned.
    Error,
}

impl ResolutionReason {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Static => "STATIC",
            Self::TargetingMatch => "TARGETING_MATCH",
            Self::Default => "DEFAULT",
            Self::Error => "ERROR",
        }
    }
}

/// OpenFeature `EvaluationDetail`: resolved value plus evaluation metadata.
#[derive(Debug, Clone, PartialEq)]
pub struct EvaluationDetail<T> {
    /// The resolved value.
    pub value: T,
    /// Variant identifier selected by the provider (if any).
    pub variant: Option<String>,
    /// Why this value was chosen.
    pub reason: ResolutionReason,
}

impl<T> EvaluationDetail<T> {
    /// A detail for the caller's default (nothing resolved).
    pub fn default_reason(value: T) -> Self {
        Self {
            value,
            variant: None,
            reason: ResolutionReason::Default,
        }
    }

    /// A detail carrying an explicit reason.
    pub fn with_reason(value: T, variant: Option<String>, reason: ResolutionReason) -> Self {
        Self {
            value,
            variant,
            reason,
        }
    }
}

/// OpenFeature provider port: resolve flag keys for an evaluation context.
pub trait FeatureProvider: Send + Sync {
    /// Stable provider name (e.g. `noop`, `static`, `toggle-registry`).
    fn name(&self) -> &str;

    /// Resolve a boolean flag.
    fn resolve_bool(
        &self,
        flag_key: &str,
        default: bool,
        ctx: &EvaluationContext,
    ) -> EvaluationDetail<bool>;

    /// Resolve a string flag.
    fn resolve_string(
        &self,
        flag_key: &str,
        default: &str,
        ctx: &EvaluationContext,
    ) -> EvaluationDetail<String>;
}

/// The required no-op provider: always returns the caller's default.
pub struct NoOpProvider;

impl FeatureProvider for NoOpProvider {
    fn name(&self) -> &str {
        "noop"
    }

    fn resolve_bool(
        &self,
        _flag_key: &str,
        default: bool,
        _ctx: &EvaluationContext,
    ) -> EvaluationDetail<bool> {
        EvaluationDetail::default_reason(default)
    }

    fn resolve_string(
        &self,
        _flag_key: &str,
        default: &str,
        _ctx: &EvaluationContext,
    ) -> EvaluationDetail<String> {
        EvaluationDetail::default_reason(default.to_string())
    }
}

/// One attribute constraint: context attribute `attribute` must equal
/// `expected` (string comparison over `ContextValue::as_str`, falling back to
/// the rendered scalar form of numbers/booleans).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttributeMatch {
    pub attribute: String,
    pub expected: String,
}

impl AttributeMatch {
    pub fn new(attribute: impl Into<String>, expected: impl Into<String>) -> Self {
        Self {
            attribute: attribute.into(),
            expected: expected.into(),
        }
    }

    fn matches(&self, ctx: &EvaluationContext) -> bool {
        ctx.attributes()
            .get(self.attribute.as_str())
            .map(|value| {
                value
                    .as_str()
                    .map(|s| s == self.expected)
                    .unwrap_or_else(|| {
                        if let Some(n) = value.as_number() {
                            n.to_string() == self.expected
                        } else if let Some(b) = value.as_bool() {
                            b.to_string() == self.expected
                        } else {
                            false
                        }
                    })
            })
            .unwrap_or(false)
    }
}

/// A targeting rule: when every constraint matches, the flag resolves to
/// `variant`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetingRule {
    /// Attribute constraints (all must match).
    pub conditions: Vec<AttributeMatch>,
    /// Variant selected when the rule matches.
    pub variant: String,
}

impl TargetingRule {
    pub fn when_attribute(
        attribute: impl Into<String>,
        expected: impl Into<String>,
        variant: impl Into<String>,
    ) -> Self {
        Self {
            conditions: vec![AttributeMatch::new(attribute, expected)],
            variant: variant.into(),
        }
    }

    fn matches(&self, ctx: &EvaluationContext) -> bool {
        self.conditions.iter().all(|c| c.matches(ctx))
    }
}

/// Percentage rollout: `rollout` percent of identities (bucketed by the
/// targeting key) receive `variant_in`, the rest `variant_off`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PercentageRollout {
    /// Percentage [0..=100] receiving `variant_in`.
    pub rollout: u8,
    pub variant_in: String,
    pub variant_off: String,
}

/// A declarative flag: static variant + optional targeting rules and
/// percentage rollout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlagConfig {
    /// The variant served when nothing else applies.
    pub default_variant: String,
    /// Variant payloads (variant id → string value).
    pub variants: HashMap<String, String>,
    /// Evaluated in order; first match wins.
    pub targeting: Vec<TargetingRule>,
    /// Percentage rollout applied after targeting.
    pub percentage: Option<PercentageRollout>,
}

impl FlagConfig {
    /// A single-variant boolean-style flag (`on`/`off`).
    pub fn on_off(default_on: bool) -> Self {
        let mut variants = HashMap::new();
        variants.insert("on".to_string(), "true".to_string());
        variants.insert("off".to_string(), "false".to_string());
        Self {
            default_variant: if default_on { "on" } else { "off" }.to_string(),
            variants,
            targeting: Vec::new(),
            percentage: None,
        }
    }

    /// Add a targeting rule.
    pub fn with_rule(mut self, rule: TargetingRule) -> Self {
        self.targeting.push(rule);
        self
    }

    /// Add a percentage rollout.
    pub fn with_rollout(mut self, rollout: PercentageRollout) -> Self {
        self.percentage = Some(rollout);
        self
    }

    fn payload(&self, variant: &str) -> Option<&String> {
        self.variants.get(variant)
    }
}

/// Deterministic bucket for `(flag, targeting key)` in `0..=99`.
///
/// FNV-1a over `flag_key:targeting_key`; a missing targeting key always
/// buckets out (index 0 → only 1% exposure edge — callers should always
/// supply a targeting key for rollouts).
pub fn bucket_index(flag_key: &str, targeting_key: &str) -> u32 {
    let mut hash: u32 = 0x811c9dc5;
    for byte in format!("{flag_key}:{targeting_key}").as_bytes() {
        hash ^= u64::from(*byte) as u32;
        hash = hash.wrapping_mul(0x01000193);
    }
    hash % 100
}

/// Declarative flag provider with percentage and attribute-rule evaluation.
#[derive(Default)]
pub struct StaticFlagProvider {
    flags: HashMap<String, FlagConfig>,
}

impl StaticFlagProvider {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register (or replace) a flag.
    pub fn with_flag(mut self, key: impl Into<String>, flag: FlagConfig) -> Self {
        self.flags.insert(key.into(), flag);
        self
    }

    fn resolve_variant(&self, flag: &FlagConfig, key: &str, ctx: &EvaluationContext) -> (String, ResolutionReason) {
        // 1. Targeting rules, first match wins.
        for rule in &flag.targeting {
            if rule.matches(ctx) {
                return (rule.variant.clone(), ResolutionReason::TargetingMatch);
            }
        }
        // 2. Percentage rollout, bucketed by the targeting key.
        if let Some(rollout) = &flag.percentage {
            let bucket = bucket_index(key, ctx.targeting_key().unwrap_or(""));
            let in_group = bucket < u32::from(rollout.rollout.min(100));
            let variant = if in_group {
                rollout.variant_in.clone()
            } else {
                rollout.variant_off.clone()
            };
            return (variant, ResolutionReason::TargetingMatch);
        }
        // 3. Static default.
        (flag.default_variant.clone(), ResolutionReason::Static)
    }
}

impl FeatureProvider for StaticFlagProvider {
    fn name(&self) -> &str {
        "static"
    }

    fn resolve_bool(
        &self,
        flag_key: &str,
        default: bool,
        ctx: &EvaluationContext,
    ) -> EvaluationDetail<bool> {
        let detail = self.resolve_string(flag_key, &default.to_string(), ctx);
        if detail.reason == ResolutionReason::Default {
            EvaluationDetail::default_reason(default)
        } else {
            EvaluationDetail {
                value: detail.value == "true",
                variant: detail.variant,
                reason: detail.reason,
            }
        }
    }

    fn resolve_string(
        &self,
        flag_key: &str,
        default: &str,
        ctx: &EvaluationContext,
    ) -> EvaluationDetail<String> {
        let Some(flag) = self.flags.get(flag_key) else {
            return EvaluationDetail::default_reason(default.to_string());
        };
        let (variant, reason) = self.resolve_variant(flag, flag_key, ctx);
        let value = flag
            .payload(&variant)
            .cloned()
            .unwrap_or_else(|| default.to_string());
        EvaluationDetail::with_reason(value, Some(variant), reason)
    }
}

/// Adapter: legacy [`crate::toggle::FeatureToggleRegistry`] booleans through
/// the OpenFeature port.
pub struct ToggleRegistryProvider {
    registry: Arc<crate::toggle::FeatureToggleRegistry>,
}

impl ToggleRegistryProvider {
    pub fn new(registry: Arc<crate::toggle::FeatureToggleRegistry>) -> Self {
        Self { registry }
    }
}

impl FeatureProvider for ToggleRegistryProvider {
    fn name(&self) -> &str {
        "toggle-registry"
    }

    fn resolve_bool(
        &self,
        flag_key: &str,
        default: bool,
        _ctx: &EvaluationContext,
    ) -> EvaluationDetail<bool> {
        // Unknown flags keep the caller's default (fail-safe).
        if self.registry.list().iter().any(|info| info.name == flag_key) {
            EvaluationDetail::with_reason(
                self.registry.is_enabled(flag_key),
                None,
                ResolutionReason::Static,
            )
        } else {
            EvaluationDetail::default_reason(default)
        }
    }

    fn resolve_string(
        &self,
        flag_key: &str,
        default: &str,
        ctx: &EvaluationContext,
    ) -> EvaluationDetail<String> {
        let detail = self.resolve_bool(flag_key, false, ctx);
        if detail.reason == ResolutionReason::Default {
            EvaluationDetail::default_reason(default.to_string())
        } else {
            EvaluationDetail {
                value: detail.value.to_string(),
                variant: detail.variant,
                reason: detail.reason,
            }
        }
    }
}

/// OpenFeature client: one provider at a time, swappable at runtime.
pub struct OpenFeatureClient {
    provider: std::sync::RwLock<Arc<dyn FeatureProvider>>,
}

impl OpenFeatureClient {
    /// A client on the required no-op provider.
    pub fn new() -> Self {
        Self {
            provider: std::sync::RwLock::new(Arc::new(NoOpProvider)),
        }
    }

    /// A client on an explicit provider.
    pub fn with_provider(provider: Arc<dyn FeatureProvider>) -> Self {
        Self {
            provider: std::sync::RwLock::new(provider),
        }
    }

    /// Swap the provider (OpenFeature `set_provider`).
    pub fn set_provider(&self, provider: Arc<dyn FeatureProvider>) {
                // A provider that panicked while evaluated must not wedge the client
        // for good: recover the (possibly inconsistent) lock like the other
        // poisoned-lock call sites in this crate.
        *self
            .provider
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = provider;
    }

    /// The active provider's name.
    pub fn provider_name(&self) -> String {
        self.provider.read().expect("provider lock").name().to_string()
    }

    /// Resolve a boolean flag to a bare value.
    pub fn bool_value(&self, flag_key: &str, default: bool, ctx: &EvaluationContext) -> bool {
        self.provider
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .resolve_bool(flag_key, default, ctx)
            .value
    }

    /// Resolve a boolean flag with full evaluation detail.
    pub fn bool_details(
        &self,
        flag_key: &str,
        default: bool,
        ctx: &EvaluationContext,
    ) -> EvaluationDetail<bool> {
        self.provider
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .resolve_bool(flag_key, default, ctx)
    }

    /// Resolve a string flag to a bare value.
    pub fn string_value(
        &self,
        flag_key: &str,
        default: &str,
        ctx: &EvaluationContext,
    ) -> String {
        self.provider
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .resolve_string(flag_key, default, ctx)
            .value
    }

    /// Resolve a string flag with full evaluation detail.
    pub fn string_details(
        &self,
        flag_key: &str,
        default: &str,
        ctx: &EvaluationContext,
    ) -> EvaluationDetail<String> {
        self.provider
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .resolve_string(flag_key, default, ctx)
    }
}

impl Default for OpenFeatureClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::ContextValue;

    fn ctx_with(key: &str, attrs: &[(&str, &str)]) -> EvaluationContext {
        let mut ctx = EvaluationContext::new().with_key(key);
        for (k, v) in attrs {
            ctx = ctx.attr(*k, ContextValue::String(std::sync::Arc::from(*v)));
        }
        ctx
    }

    #[test]
    fn noop_provider_returns_default_with_default_reason() {
        let client = OpenFeatureClient::new();
        assert_eq!(client.provider_name(), "noop");
        let detail = client.bool_details("anything", true, &ctx_with("user", &[]));
        assert!(detail.value);
        assert_eq!(detail.reason, ResolutionReason::Default);
        assert_eq!(client.string_value("anything", "fallback", &ctx_with("u", &[])), "fallback");
    }

    #[test]
    fn static_provider_attribute_rules_match_first() {
        let provider = StaticFlagProvider::new().with_flag(
            "checkout",
            FlagConfig::on_off(false).with_rule(TargetingRule::when_attribute(
                "region",
                "eu",
                "on",
            )),
        );
        let client = OpenFeatureClient::with_provider(Arc::new(provider));

        let eu = client.bool_details(
            "checkout",
            true,
            &ctx_with("user-1", &[("region", "eu")]),
        );
        assert!(eu.value);
        assert_eq!(eu.reason, ResolutionReason::TargetingMatch);

        let us = client.bool_details(
            "checkout",
            true,
            &ctx_with("user-1", &[("region", "us")]),
        );
        assert!(!us.value, "non-matching context falls to the static default");
        assert_eq!(us.reason, ResolutionReason::Static);
    }

    #[test]
    fn percentage_rollout_is_deterministic_and_bounded() {
        let provider = StaticFlagProvider::new().with_flag(
            "beta-ui",
            FlagConfig::on_off(false).with_rollout(PercentageRollout {
                rollout: 50,
                variant_in: "on".to_string(),
                variant_off: "off".to_string(),
            }),
        );
        let client = OpenFeatureClient::with_provider(Arc::new(provider));

        let mut in_group = 0;
        for i in 0..1000 {
            let enabled = client.bool_value(
                "beta-ui",
                false,
                &ctx_with(&format!("user-{i}"), &[]),
            );
            in_group += u32::from(enabled);
        }
        assert!(
            (350..=650).contains(&in_group),
            "50% rollout should land near 500, got {in_group}"
        );

        // Determinism: the same identity always lands in the same bucket.
        let first = client.bool_value("beta-ui", false, &ctx_with("user-42", &[]));
        for _ in 0..5 {
            assert_eq!(
                client.bool_value("beta-ui", false, &ctx_with("user-42", &[])),
                first
            );
        }
    }

    #[test]
    fn bucket_index_is_stable_across_flag_and_key() {
        assert_eq!(bucket_index("f", "user-1"), bucket_index("f", "user-1"));
        assert_ne!(bucket_index("f", "user-1"), bucket_index("g", "user-1"));
    }

    #[test]
    fn toggle_registry_bridges_through_openfeature_semantics() {
        let registry = Arc::new(crate::toggle::FeatureToggleRegistry::new());
        registry.register("dark-mode", "dark theme", false);
        registry.enable("dark-mode");

        let client = OpenFeatureClient::with_provider(Arc::new(
            ToggleRegistryProvider::new(registry),
        ));
        let detail = client.bool_details("dark-mode", false, &ctx_with("u", &[]));
        assert!(detail.value);
        assert_eq!(detail.reason, ResolutionReason::Static);

        // Unknown flags fall back to the caller default.
        let unknown = client.bool_details("nope", true, &ctx_with("u", &[]));
        assert!(unknown.value);
        assert_eq!(unknown.reason, ResolutionReason::Default);
    }

    #[test]
    fn provider_can_be_swapped_at_runtime() {
        let client = OpenFeatureClient::new();
        assert_eq!(client.provider_name(), "noop");
        client.set_provider(Arc::new(
            StaticFlagProvider::new()
                .with_flag("f", FlagConfig::on_off(true)),
        ));
        assert_eq!(client.provider_name(), "static");
        assert!(client.bool_value("f", false, &ctx_with("u", &[])));
    }
}
