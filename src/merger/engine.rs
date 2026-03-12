//! Merge engine for combining configuration values.

use crate::error::{ConfigError, ConfigResult};
use crate::merger::MergeStrategy;
use crate::value::{AnnotatedValue, ConfigValue, ConflictReport, ConflictWinner};
use indexmap::IndexMap;
use std::sync::Arc;

const MAX_MERGE_DEPTH: usize = 100;

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

    pub fn get_strategy(&self, path: &str) -> &MergeStrategy {
        self.field_strategies
            .get(path)
            .unwrap_or(&self.default_strategy)
    }

    pub fn merge(
        &self,
        low: &AnnotatedValue,
        high: &AnnotatedValue,
    ) -> ConfigResult<AnnotatedValue> {
        Self::merge_with_depth(low, high, self.get_strategy(&low.path).clone(), 0)
    }

    fn merge_with_depth(
        low: &AnnotatedValue,
        high: &AnnotatedValue,
        strategy: MergeStrategy,
        depth: usize,
    ) -> ConfigResult<AnnotatedValue> {
        if depth > MAX_MERGE_DEPTH {
            return Err(ConfigError::ParseError {
                format: "merge".to_string(),
                message: format!(
                    "Maximum merge depth ({}) exceeded at path: {}",
                    MAX_MERGE_DEPTH, high.path
                ),
                location: None,
                source: None,
            });
        }

        let merged = match (&low.inner, &high.inner, &strategy) {
            (ConfigValue::Null, _, _) => high.inner.clone(),
            (_, ConfigValue::Null, _) => low.inner.clone(),
            (ConfigValue::Map(l), ConfigValue::Map(_), _) => {
                let l_map = l.as_ref();
                let mut result: IndexMap<Arc<str>, AnnotatedValue> = l_map.clone();

                let r_map = match &high.inner {
                    ConfigValue::Map(m) => m.as_ref(),
                    _ => unreachable!(),
                };

                for (k, v_high) in r_map.iter() {
                    if let Some(v_low) = result.get_mut(k) {
                        let needs_recursive = matches!(
                            (&v_low.inner, &v_high.inner),
                            (&ConfigValue::Map(_), &ConfigValue::Map(_))
                        );

                        if needs_recursive {
                            let merged_inner =
                                Self::merge_with_depth(v_low, v_high, strategy.clone(), depth + 1)?;
                            *v_low = merged_inner;
                        } else {
                            *v_low = Self::apply_strategy(v_low, v_high, &strategy)?;
                        }
                    } else {
                        result.insert(k.clone(), v_high.clone());
                    }
                }
                ConfigValue::Map(Arc::new(result))
            }
            (_, _, MergeStrategy::Custom { func, .. }) => func(&low.inner, &high.inner),
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

    /// Apply merge strategy to leaf values (non-map).
    fn apply_strategy(
        low: &AnnotatedValue,
        high: &AnnotatedValue,
        strategy: &MergeStrategy,
    ) -> ConfigResult<AnnotatedValue> {
        let merged = match (&low.inner, &high.inner, strategy) {
            (_, _, MergeStrategy::Custom { func, .. }) => func(&low.inner, &high.inner),
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
