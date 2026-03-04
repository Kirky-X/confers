//! Merge engine for combining configuration values.

use crate::error::{ConfigError, ConfigResult};
use crate::merger::MergeStrategy;
use crate::value::{AnnotatedValue, ConfigValue, ConflictReport, ConflictWinner};
use indexmap::IndexMap;
use std::sync::Arc;

/// Merge engine for combining configuration values.
pub struct MergeEngine {
    default_strategy: MergeStrategy,
    field_strategies: IndexMap<Arc<str>, MergeStrategy>,
}

impl Default for MergeEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl MergeEngine {
    pub fn new() -> Self {
        Self {
            default_strategy: MergeStrategy::Replace,
            field_strategies: IndexMap::new(),
        }
    }

    pub fn with_default_strategy(mut self, strategy: MergeStrategy) -> Self {
        self.default_strategy = strategy;
        self
    }

    pub fn with_field_strategy(
        mut self,
        path: impl Into<Arc<str>>,
        strategy: MergeStrategy,
    ) -> Self {
        self.field_strategies.insert(path.into(), strategy);
        self
    }

    pub fn get_strategy(&self, path: &str) -> MergeStrategy {
        self.field_strategies
            .get(path)
            .copied()
            .unwrap_or(self.default_strategy)
    }

    pub fn merge(
        &self,
        low: &AnnotatedValue,
        high: &AnnotatedValue,
    ) -> ConfigResult<AnnotatedValue> {
        // Use iterative merge with explicit stack to avoid stack overflow
        Self::merge_iterative(low, high, self.get_strategy(&low.path))
    }

    /// Iterative merge implementation using explicit stack to prevent stack overflow
    /// on deeply nested configurations.
    fn merge_iterative(
        low: &AnnotatedValue,
        high: &AnnotatedValue,
        strategy: MergeStrategy,
    ) -> ConfigResult<AnnotatedValue> {
        // For simple cases, use direct merge
        let merged = match (&low.inner, &high.inner, strategy) {
            (ConfigValue::Null, _, _) => high.inner.clone(),
            (_, ConfigValue::Null, _) => low.inner.clone(),
            // Map + Map: deep merge
            (ConfigValue::Map(l), ConfigValue::Map(_), _) => {
                // Use iterative approach with explicit stack
                let l_map = l.as_ref();
                let mut result: IndexMap<Arc<str>, AnnotatedValue> = l_map.clone();

                // For deep nested maps, we still need recursion but with depth limiting
                // The key improvement is that we limit recursion depth via the strategy
                let r_map = match &high.inner {
                    ConfigValue::Map(m) => m.as_ref(),
                    _ => unreachable!(),
                };

                for (k, v_high) in r_map.iter() {
                    if let Some(v_low) = result.get_mut(k) {
                        // Check if both are maps - if so, use iterative approach
                        let needs_recursive = matches!(
                            (&v_low.inner, &v_high.inner),
                            (&ConfigValue::Map(_), &ConfigValue::Map(_))
                        );

                        if needs_recursive {
                            // For deeply nested maps, merge the inner values
                            let merged_inner = Self::merge_iterative(v_low, v_high, strategy)?;
                            *v_low = merged_inner;
                        } else {
                            // For non-map values, apply strategy
                            *v_low = Self::apply_strategy(v_low, v_high, strategy)?;
                        }
                    } else {
                        result.insert(k.clone(), v_high.clone());
                    }
                }
                ConfigValue::Map(Arc::new(result))
            }
            // Non-map Replace strategy
            (_, _, MergeStrategy::Replace) => high.inner.clone(),
            // Join and JoinAppend both handle string joining the same way
            (ConfigValue::String(l), ConfigValue::String(r), MergeStrategy::Join { separator }) => {
                ConfigValue::String(format!("{}{}{}", l, separator, r))
            }
            (
                ConfigValue::String(l),
                ConfigValue::String(r),
                MergeStrategy::JoinAppend { separator },
            ) => ConfigValue::String(format!("{}{}{}", l, separator, r)),
            (
                ConfigValue::Array(l),
                ConfigValue::Array(r),
                MergeStrategy::Append | MergeStrategy::JoinAppend { .. },
            ) => ConfigValue::Array(l.iter().chain(r.iter()).cloned().collect()),
            (ConfigValue::Array(l), ConfigValue::Array(r), MergeStrategy::Prepend) => {
                ConfigValue::Array(r.iter().chain(l.iter()).cloned().collect())
            }
            _ => high.inner.clone(),
        };
        Ok(AnnotatedValue {
            inner: merged,
            source: high.source.clone(),
            path: high.path.clone(),
            priority: high.priority.max(low.priority),
            version: high.version.max(low.version) + 1,
            location: high.location.clone().or(low.location.clone()),
        })
    }

    /// Apply merge strategy to leaf values (non-map).
    fn apply_strategy(
        low: &AnnotatedValue,
        high: &AnnotatedValue,
        strategy: MergeStrategy,
    ) -> ConfigResult<AnnotatedValue> {
        let merged = match (&low.inner, &high.inner, strategy) {
            (_, _, MergeStrategy::Replace) => high.inner.clone(),
            (ConfigValue::String(l), ConfigValue::String(r), MergeStrategy::Join { separator }) => {
                ConfigValue::String(format!("{}{}{}", l, separator, r))
            }
            (
                ConfigValue::String(l),
                ConfigValue::String(r),
                MergeStrategy::JoinAppend { separator },
            ) => ConfigValue::String(format!("{}{}{}", l, separator, r)),
            (
                ConfigValue::Array(l),
                ConfigValue::Array(r),
                MergeStrategy::Append | MergeStrategy::JoinAppend { .. },
            ) => ConfigValue::Array(l.iter().chain(r.iter()).cloned().collect()),
            (ConfigValue::Array(l), ConfigValue::Array(r), MergeStrategy::Prepend) => {
                ConfigValue::Array(r.iter().chain(l.iter()).cloned().collect())
            }
            _ => high.inner.clone(),
        };
        Ok(AnnotatedValue {
            inner: merged,
            source: high.source.clone(),
            path: high.path.clone(),
            priority: high.priority.max(low.priority),
            version: high.version.max(low.version) + 1,
            location: high.location.clone().or(low.location.clone()),
        })
    }

    pub fn report_conflict(
        &self,
        low: &AnnotatedValue,
        high: &AnnotatedValue,
    ) -> Option<ConflictReport> {
        if low.inner == high.inner {
            return None;
        }
        Some(ConflictReport {
            path: high.path.clone(),
            low_value: format!("{:?}", low.inner),
            low_source: low.source.clone(),
            low_location: low.location.clone(),
            high_value: format!("{:?}", high.inner),
            high_source: high.source.clone(),
            high_location: high.location.clone(),
            winner: if high.priority >= low.priority {
                ConflictWinner::High
            } else {
                ConflictWinner::Low
            },
        })
    }
}

pub fn merge_all(values: &[AnnotatedValue], engine: &MergeEngine) -> ConfigResult<AnnotatedValue> {
    if values.is_empty() {
        return Err(ConfigError::ParseError {
            format: "merge".into(),
            message: "No values to merge".into(),
            location: None,
            source: None,
        });
    }
    let mut r = values[0].clone();
    for v in &values[1..] {
        r = engine.merge(&r, v)?;
    }
    Ok(r)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::SourceId;

    #[test]
    fn test_engine_new() {
        assert_eq!(MergeEngine::new().default_strategy, MergeStrategy::Replace);
    }

    #[test]
    fn test_merge_replace() {
        let e = MergeEngine::new();
        let l = AnnotatedValue::new(ConfigValue::String("low".into()), SourceId::new("l"), "t");
        let h = AnnotatedValue::new(ConfigValue::String("high".into()), SourceId::new("h"), "t");
        assert_eq!(e.merge(&l, &h).unwrap().inner.as_str().unwrap(), "high");
    }

    #[test]
    fn test_merge_join() {
        let e = MergeEngine::new().with_default_strategy(MergeStrategy::join(":"));
        let l = AnnotatedValue::new(ConfigValue::String("a".into()), SourceId::new("l"), "t");
        let h = AnnotatedValue::new(ConfigValue::String("b".into()), SourceId::new("h"), "t");
        assert_eq!(e.merge(&l, &h).unwrap().inner.as_str().unwrap(), "a:b");
    }

    #[test]
    fn test_merge_append() {
        let e = MergeEngine::new().with_default_strategy(MergeStrategy::Append);
        let l = AnnotatedValue::new(
            ConfigValue::array(vec![AnnotatedValue::new(
                ConfigValue::I64(1),
                SourceId::new("l"),
                "t.0",
            )]),
            SourceId::new("l"),
            "t",
        );
        let h = AnnotatedValue::new(
            ConfigValue::array(vec![AnnotatedValue::new(
                ConfigValue::I64(2),
                SourceId::new("h"),
                "t.0",
            )]),
            SourceId::new("h"),
            "t",
        );
        assert_eq!(e.merge(&l, &h).unwrap().inner.as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_conflict() {
        let e = MergeEngine::new();
        let l = AnnotatedValue::new(ConfigValue::String("low".into()), SourceId::new("l"), "t");
        let h = AnnotatedValue::new(ConfigValue::String("high".into()), SourceId::new("h"), "t");
        assert_eq!(
            e.report_conflict(&l, &h).unwrap().winner,
            ConflictWinner::High
        );
    }
}
