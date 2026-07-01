//! Merge engine for combining configuration values.

use crate::error::{ConfigError, ConfigResult};
use crate::impl_::merger::MergeStrategy;
use crate::types::{AnnotatedValue, ConfigValue, ConflictReport, ConflictWinner};
use indexmap::IndexMap;
use std::sync::Arc;

const MAX_MERGE_DEPTH: usize = 100;

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
        Self::merge_with_depth(low, high, *self.get_strategy(&low.path), 0)
    }

    fn merge_with_depth(
        low: &AnnotatedValue,
        high: &AnnotatedValue,
        strategy: MergeStrategy,
        depth: usize,
    ) -> ConfigResult<AnnotatedValue> {
        check_merge_depth(depth, &high.path)?;

        let merged = match (&low.inner, &high.inner, &strategy) {
            (ConfigValue::Null, _, _) => high.inner.clone(),
            (_, ConfigValue::Null, _) => low.inner.clone(),
            (ConfigValue::Map(l), ConfigValue::Map(r), _) => {
                merge_maps_with_cow(l, r, &strategy, depth)?
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

        Ok(build_annotated_value(merged, high, low))
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

#[inline]
fn check_merge_depth(depth: usize, path: &Arc<str>) -> ConfigResult<()> {
    if depth > MAX_MERGE_DEPTH {
        return Err(ConfigError::ParseError {
            format: "merge".to_string(),
            message: format!(
                "Maximum merge depth ({}) exceeded at path: {}",
                MAX_MERGE_DEPTH, path
            ),
            location: None,
            source: None,
        });
    }
    Ok(())
}

#[inline]
fn build_annotated_value(
    merged: ConfigValue,
    high: &AnnotatedValue,
    low: &AnnotatedValue,
) -> AnnotatedValue {
    AnnotatedValue {
        inner: merged,
        source: high.source.clone(),
        path: high.path.clone(),
        priority: high.priority.max(low.priority),
        version: high.version.max(low.version) + 1,
        location: high.location.clone().or(low.location.clone()),
    }
}

/// Optimized COW map merge.
///
/// Key optimizations:
/// 1. Arc::ptr_eq fast path: if low_map and high_map share the same allocation,
///    return immediately (zero allocation, zero copy)
/// 2. Empty fast paths: if either map is empty, return the other's Arc
/// 3. Single-pass change detection: scan high keys against low to detect no-op case
/// 4. Single clone: only clone low_map once when modifications are actually needed
fn merge_maps_with_cow(
    low_map: &Arc<IndexMap<Arc<str>, AnnotatedValue>>,
    high_map: &Arc<IndexMap<Arc<str>, AnnotatedValue>>,
    strategy: &MergeStrategy,
    depth: usize,
) -> ConfigResult<ConfigValue> {
    // Fast path: identical Arc → no merge work needed at all
    if Arc::ptr_eq(low_map, high_map) {
        return Ok(ConfigValue::Map(Arc::clone(low_map)));
    }

    let low = low_map.as_ref();
    let high = high_map.as_ref();

    // Fast path: if high is empty, no modifications possible
    if high.is_empty() {
        return Ok(ConfigValue::Map(Arc::clone(low_map)));
    }

    // Fast path: if low is empty, high wins completely
    if low.is_empty() {
        return Ok(ConfigValue::Map(Arc::clone(high_map)));
    }

    // Determine if any modifications are needed by checking high keys against low
    // This is a quick scan to detect the no-modification case
    let mut has_changes = false;

    for (k, v_high) in high.iter() {
        match low.get(k) {
            Some(v_low) => {
                // Check if values are actually different
                if !values_equal(&v_low.inner, &v_high.inner, strategy) {
                    has_changes = true;
                    break;
                }
            }
            None => {
                has_changes = true;
                break;
            }
        }
    }

    // No modifications needed - return original Arc (zero allocation, zero copy)
    if !has_changes {
        return Ok(ConfigValue::Map(Arc::clone(low_map)));
    }

    // There are modifications - clone the low map once and apply all changes
    // This is the key COW optimization: only one clone instead of per-key clones
    let mut result = (**low_map).clone();

    for (k, v_high) in high.iter() {
        if let Some(v_low) = low.get(k) {
            // Both maps have this key - merge or replace
            let needs_recursive = matches!(
                (&v_low.inner, &v_high.inner),
                (ConfigValue::Map(_), ConfigValue::Map(_))
            );

            if needs_recursive {
                let merged_inner =
                    MergeEngine::merge_with_depth(v_low, v_high, *strategy, depth + 1)?;
                result.insert(
                    k.clone(),
                    build_annotated_value(merged_inner.inner.clone(), v_high, v_low),
                );
            } else {
                let merged = apply_leaf_strategy(&v_low.inner, &v_high.inner, strategy);
                result.insert(k.clone(), build_annotated_value(merged, v_high, v_low));
            }
        } else {
            // Key only in high - add it
            result.insert(k.clone(), v_high.clone());
        }
    }

    Ok(ConfigValue::Map(Arc::new(result)))
}

/// Check if two values are equal after applying a merge strategy.
/// Used for the fast-path detection of no modifications.
///
/// Returns true when applying `strategy` to `(low, high)` yields a result equal
/// to `low` (i.e., no modification needed). For Map/Map with non-Replace strategies,
/// uses `Arc::ptr_eq` for O(1) pointer comparison; falls back to false (conservative)
/// when the maps are different allocations, since a full deep comparison would
/// defeat the purpose of this fast-path check.
#[inline]
fn values_equal(low: &ConfigValue, high: &ConfigValue, strategy: &MergeStrategy) -> bool {
    match (low, high, strategy) {
        // Replace strategy: result == high, so no change iff low == high
        (_, _, MergeStrategy::Replace) => low == high,
        // For Map/Map with non-Replace strategies: use Arc::ptr_eq for O(1) check.
        // If the maps share the same allocation, merge is a no-op.
        // If they don't, conservatively report "may differ" to avoid O(n) comparison.
        (ConfigValue::Map(l), ConfigValue::Map(r), _) => Arc::ptr_eq(l, r),
        // String join: result differs from low
        (ConfigValue::String(_), ConfigValue::String(_), MergeStrategy::Join { .. }) => false,
        (ConfigValue::String(_), ConfigValue::String(_), MergeStrategy::JoinAppend { .. }) => false,
        // Array strategies: result differs from low
        (
            ConfigValue::Array(_),
            ConfigValue::Array(_),
            MergeStrategy::Append | MergeStrategy::JoinAppend { .. },
        ) => false,
        (ConfigValue::Array(_), ConfigValue::Array(_), MergeStrategy::Prepend) => false,
        // Default: high wins, so result always differs from low
        _ => low == high,
    }
}

#[inline]
fn apply_leaf_strategy(
    low: &ConfigValue,
    high: &ConfigValue,
    strategy: &MergeStrategy,
) -> ConfigValue {
    match (low, high, strategy) {
        (_, _, MergeStrategy::Replace) => high.clone(),
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
        (_, _, MergeStrategy::Custom { func, .. }) => func(low, high),
        _ => high.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::SourceId;

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

    #[test]
    fn test_cow_no_modification() {
        let e = MergeEngine::new();
        let inner = IndexMap::from_iter(vec![(
            Arc::from("key"),
            AnnotatedValue::new(
                ConfigValue::String("value".into()),
                SourceId::new("s"),
                "key",
            ),
        )]);
        let l = AnnotatedValue::new(
            ConfigValue::Map(Arc::new(inner.clone())),
            SourceId::new("l"),
            "t",
        );
        let h = AnnotatedValue::new(
            ConfigValue::Map(Arc::new(IndexMap::new())),
            SourceId::new("h"),
            "t",
        );
        let result = e.merge(&l, &h).unwrap();
        assert!(matches!(result.inner, ConfigValue::Map(_)));
    }

    #[test]
    fn test_engine_default_impl() {
        let e = MergeEngine::default();
        assert_eq!(e.default_strategy, MergeStrategy::Replace);
    }

    #[test]
    fn test_with_default_strategy() {
        let e = MergeEngine::new().with_default_strategy(MergeStrategy::Append);
        assert_eq!(e.default_strategy, MergeStrategy::Append);
    }

    #[test]
    fn test_with_field_strategy_and_get() {
        let e = MergeEngine::new()
            .with_default_strategy(MergeStrategy::Replace)
            .with_field_strategy("special", MergeStrategy::Append);
        assert_eq!(e.get_strategy("special"), &MergeStrategy::Append);
        assert_eq!(e.get_strategy("other"), &MergeStrategy::Replace);
    }

    #[test]
    fn test_merge_null_low_returns_high() {
        let e = MergeEngine::new();
        let l = AnnotatedValue::new(ConfigValue::Null, SourceId::new("l"), "t");
        let h = AnnotatedValue::new(ConfigValue::string("high"), SourceId::new("h"), "t");
        let result = e.merge(&l, &h).unwrap();
        assert_eq!(result.as_str(), Some("high"));
    }

    #[test]
    fn test_merge_join_append_string() {
        let e = MergeEngine::new().with_default_strategy(MergeStrategy::join_append(","));
        let l = AnnotatedValue::new(ConfigValue::string("a"), SourceId::new("l"), "t");
        let h = AnnotatedValue::new(ConfigValue::string("b"), SourceId::new("h"), "t");
        assert_eq!(e.merge(&l, &h).unwrap().as_str(), Some("a,b"));
    }

    #[test]
    fn test_merge_prepend_arrays() {
        let e = MergeEngine::new().with_default_strategy(MergeStrategy::Prepend);
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
        let merged = e.merge(&l, &h).unwrap();
        let arr = merged.inner.as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0].as_i64(), Some(2));
        assert_eq!(arr[1].as_i64(), Some(1));
    }

    #[test]
    fn test_merge_custom_strategy() {
        fn custom_merge(low: &ConfigValue, high: &ConfigValue) -> ConfigValue {
            match (low, high) {
                (ConfigValue::String(l), ConfigValue::String(h)) => {
                    ConfigValue::String(format!("{}+{}", l, h))
                }
                _ => high.clone(),
            }
        }
        let e =
            MergeEngine::new().with_default_strategy(MergeStrategy::custom("test", custom_merge));
        let l = AnnotatedValue::new(ConfigValue::string("low"), SourceId::new("l"), "t");
        let h = AnnotatedValue::new(ConfigValue::string("high"), SourceId::new("h"), "t");
        assert_eq!(e.merge(&l, &h).unwrap().as_str(), Some("low+high"));
    }

    #[test]
    fn test_merge_type_mismatch_falls_to_high() {
        let e = MergeEngine::new().with_default_strategy(MergeStrategy::Append);
        let l = AnnotatedValue::new(ConfigValue::string("low"), SourceId::new("l"), "t");
        let h = AnnotatedValue::new(ConfigValue::I64(42), SourceId::new("h"), "t");
        let result = e.merge(&l, &h).unwrap();
        assert_eq!(result.as_i64(), Some(42));
    }

    #[test]
    fn test_merge_nested_maps_with_changes() {
        let e = MergeEngine::new();
        let low_inner = IndexMap::from_iter(vec![
            (
                Arc::from("a"),
                AnnotatedValue::new(ConfigValue::string("low_a"), SourceId::new("l"), "t.a"),
            ),
            (
                Arc::from("b"),
                AnnotatedValue::new(ConfigValue::string("low_b"), SourceId::new("l"), "t.b"),
            ),
        ]);
        let high_inner = IndexMap::from_iter(vec![
            (
                Arc::from("a"),
                AnnotatedValue::new(ConfigValue::string("high_a"), SourceId::new("h"), "t.a"),
            ),
            (
                Arc::from("c"),
                AnnotatedValue::new(ConfigValue::string("high_c"), SourceId::new("h"), "t.c"),
            ),
        ]);
        let l = AnnotatedValue::new(
            ConfigValue::Map(Arc::new(low_inner)),
            SourceId::new("l"),
            "t",
        );
        let h = AnnotatedValue::new(
            ConfigValue::Map(Arc::new(high_inner)),
            SourceId::new("h"),
            "t",
        );
        let result = e.merge(&l, &h).unwrap();
        let map = result.inner.as_map().unwrap();
        assert_eq!(map.len(), 3);
        assert_eq!(map.get("a").unwrap().as_str(), Some("high_a"));
        assert_eq!(map.get("b").unwrap().as_str(), Some("low_b"));
        assert_eq!(map.get("c").unwrap().as_str(), Some("high_c"));
    }

    #[test]
    fn test_merge_maps_low_empty() {
        let e = MergeEngine::new();
        let l = AnnotatedValue::new(
            ConfigValue::Map(Arc::new(IndexMap::new())),
            SourceId::new("l"),
            "t",
        );
        let high_inner = IndexMap::from_iter(vec![(
            Arc::from("k"),
            AnnotatedValue::new(ConfigValue::string("v"), SourceId::new("h"), "t.k"),
        )]);
        let h = AnnotatedValue::new(
            ConfigValue::Map(Arc::new(high_inner)),
            SourceId::new("h"),
            "t",
        );
        let result = e.merge(&l, &h).unwrap();
        let map = result.inner.as_map().unwrap();
        assert_eq!(map.len(), 1);
        assert_eq!(map.get("k").unwrap().as_str(), Some("v"));
    }

    #[test]
    fn test_merge_maps_no_modification_equal_values() {
        let e = MergeEngine::new();
        let inner = IndexMap::from_iter(vec![(
            Arc::from("k"),
            AnnotatedValue::new(ConfigValue::string("v"), SourceId::new("s"), "t.k"),
        )]);
        let l = AnnotatedValue::new(
            ConfigValue::Map(Arc::new(inner.clone())),
            SourceId::new("l"),
            "t",
        );
        let h = AnnotatedValue::new(ConfigValue::Map(Arc::new(inner)), SourceId::new("h"), "t");
        let result = e.merge(&l, &h).unwrap();
        let map = result.inner.as_map().unwrap();
        assert_eq!(map.get("k").unwrap().as_str(), Some("v"));
    }

    #[test]
    fn test_report_conflict_equal_returns_none() {
        let e = MergeEngine::new();
        let l = AnnotatedValue::new(ConfigValue::string("same"), SourceId::new("l"), "t");
        let h = AnnotatedValue::new(ConfigValue::string("same"), SourceId::new("h"), "t");
        assert!(e.report_conflict(&l, &h).is_none());
    }

    #[test]
    fn test_report_conflict_low_wins() {
        let e = MergeEngine::new();
        let l = AnnotatedValue::new(ConfigValue::string("low"), SourceId::new("l"), "t")
            .with_priority(100);
        let h = AnnotatedValue::new(ConfigValue::string("high"), SourceId::new("h"), "t")
            .with_priority(1);
        let report = e.report_conflict(&l, &h).unwrap();
        assert_eq!(report.winner, ConflictWinner::Low);
        assert_eq!(report.path.as_ref(), "t");
    }

    #[test]
    fn test_report_conflict_with_locations() {
        let e = MergeEngine::new();
        let loc = crate::types::SourceLocation::new("f.toml", 1, 1);
        let l = AnnotatedValue::new(ConfigValue::string("a"), SourceId::new("l"), "t")
            .with_location(loc.clone());
        let h = AnnotatedValue::new(ConfigValue::string("b"), SourceId::new("h"), "t")
            .with_location(loc);
        let report = e.report_conflict(&l, &h).unwrap();
        assert!(report.low_location.is_some());
        assert!(report.high_location.is_some());
        assert_eq!(report.low_source.as_str(), "l");
        assert_eq!(report.high_source.as_str(), "h");
    }

    #[test]
    fn test_check_merge_depth_exceeded() {
        let path: Arc<str> = Arc::from("deep.path");
        let result = check_merge_depth(MAX_MERGE_DEPTH + 1, &path);
        assert!(result.is_err());
        match result.unwrap_err() {
            ConfigError::ParseError {
                format, message, ..
            } => {
                assert_eq!(format, "merge");
                assert!(message.contains("Maximum merge depth"));
                assert!(message.contains(&format!("{}", MAX_MERGE_DEPTH)));
            }
            other => panic!("expected ParseError, got {:?}", other),
        }
    }

    #[test]
    fn test_check_merge_depth_ok() {
        let path: Arc<str> = Arc::from("ok.path");
        assert!(check_merge_depth(0, &path).is_ok());
        assert!(check_merge_depth(MAX_MERGE_DEPTH, &path).is_ok());
    }

    #[test]
    fn test_merge_deep_merge_strategy_on_maps() {
        let e = MergeEngine::new().with_default_strategy(MergeStrategy::DeepMerge);
        let low_inner = IndexMap::from_iter(vec![(
            Arc::from("a"),
            AnnotatedValue::new(ConfigValue::string("low_a"), SourceId::new("l"), "t.a"),
        )]);
        let high_inner = IndexMap::from_iter(vec![(
            Arc::from("a"),
            AnnotatedValue::new(ConfigValue::string("high_a"), SourceId::new("h"), "t.a"),
        )]);
        let l = AnnotatedValue::new(
            ConfigValue::Map(Arc::new(low_inner)),
            SourceId::new("l"),
            "t",
        );
        let h = AnnotatedValue::new(
            ConfigValue::Map(Arc::new(high_inner)),
            SourceId::new("h"),
            "t",
        );
        let result = e.merge(&l, &h).unwrap();
        let map = result.inner.as_map().unwrap();
        // DeepMerge on leaf strings falls to _ => high.clone()
        assert_eq!(map.get("a").unwrap().as_str(), Some("high_a"));
    }

    #[test]
    fn test_merge_deep_merge_strategy_on_non_maps() {
        let e = MergeEngine::new().with_default_strategy(MergeStrategy::DeepMerge);
        let l = AnnotatedValue::new(ConfigValue::string("low"), SourceId::new("l"), "t");
        let h = AnnotatedValue::new(ConfigValue::string("high"), SourceId::new("h"), "t");
        // DeepMerge on non-map falls to _ => high.inner.clone()
        assert_eq!(e.merge(&l, &h).unwrap().as_str(), Some("high"));
    }

    #[test]
    fn test_merge_nested_maps_recursive() {
        let e = MergeEngine::new();
        // Create two-level nested maps: {outer: {inner: "value"}}
        let low_inner = IndexMap::from_iter(vec![(
            Arc::from("inner"),
            AnnotatedValue::new(
                ConfigValue::string("low_inner"),
                SourceId::new("l"),
                "t.outer.inner",
            ),
        )]);
        let low_outer = IndexMap::from_iter(vec![(
            Arc::from("outer"),
            AnnotatedValue::new(
                ConfigValue::Map(Arc::new(low_inner)),
                SourceId::new("l"),
                "t.outer",
            ),
        )]);
        let high_inner = IndexMap::from_iter(vec![(
            Arc::from("inner"),
            AnnotatedValue::new(
                ConfigValue::string("high_inner"),
                SourceId::new("h"),
                "t.outer.inner",
            ),
        )]);
        let high_outer = IndexMap::from_iter(vec![(
            Arc::from("outer"),
            AnnotatedValue::new(
                ConfigValue::Map(Arc::new(high_inner)),
                SourceId::new("h"),
                "t.outer",
            ),
        )]);
        let l = AnnotatedValue::new(
            ConfigValue::Map(Arc::new(low_outer)),
            SourceId::new("l"),
            "t",
        );
        let h = AnnotatedValue::new(
            ConfigValue::Map(Arc::new(high_outer)),
            SourceId::new("h"),
            "t",
        );
        let result = e.merge(&l, &h).unwrap();
        let outer_map = result.inner.as_map().unwrap();
        let inner_av = outer_map.get("outer").unwrap();
        let inner_map = inner_av.inner.as_map().unwrap();
        assert_eq!(inner_map.get("inner").unwrap().as_str(), Some("high_inner"));
    }

    #[test]
    fn test_merge_join_append_arrays() {
        let e = MergeEngine::new().with_default_strategy(MergeStrategy::join_append(","));
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
        let merged = e.merge(&l, &h).unwrap();
        let arr = merged.inner.as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0].as_i64(), Some(1));
        assert_eq!(arr[1].as_i64(), Some(2));
    }

    #[test]
    fn test_values_equal_replace_equal() {
        let s = MergeStrategy::Replace;
        assert!(values_equal(
            &ConfigValue::string("a"),
            &ConfigValue::string("a"),
            &s
        ));
        assert!(!values_equal(
            &ConfigValue::string("a"),
            &ConfigValue::string("b"),
            &s
        ));
    }

    #[test]
    fn test_values_equal_map_different_arc_false() {
        // Two maps with different Arc allocations: conservatively report "may differ"
        // (avoids O(n) deep comparison in the fast-path check)
        let m1 = ConfigValue::Map(Arc::new(IndexMap::new()));
        let m2 = ConfigValue::Map(Arc::new(IndexMap::new()));
        assert!(!values_equal(&m1, &m2, &MergeStrategy::Append));
        assert!(!values_equal(&m1, &m2, &MergeStrategy::DeepMerge));
    }

    #[test]
    fn test_values_equal_map_same_arc_true() {
        // S-C-5 regression: same Arc allocation means merge is a no-op
        let shared = Arc::new(IndexMap::from_iter([(
            Arc::from("k"),
            AnnotatedValue::new(ConfigValue::string("v"), SourceId::new("s"), "k"),
        )]));
        let m1 = ConfigValue::Map(Arc::clone(&shared));
        let m2 = ConfigValue::Map(Arc::clone(&shared));
        assert!(values_equal(&m1, &m2, &MergeStrategy::Append));
        assert!(values_equal(&m1, &m2, &MergeStrategy::DeepMerge));
    }

    #[test]
    fn test_merge_maps_with_cow_ptr_eq_fast_path() {
        // S-C-5 regression: merging a map with itself (same Arc) must reuse the
        // same Arc allocation without cloning
        let e = MergeEngine::new();
        let shared = Arc::new(IndexMap::from_iter([(
            Arc::from("k"),
            AnnotatedValue::new(ConfigValue::string("v"), SourceId::new("s"), "k"),
        )]));
        let l = AnnotatedValue::new(
            ConfigValue::Map(Arc::clone(&shared)),
            SourceId::new("l"),
            "t",
        );
        let h = AnnotatedValue::new(
            ConfigValue::Map(Arc::clone(&shared)),
            SourceId::new("h"),
            "t",
        );
        let result = e.merge(&l, &h).unwrap();
        if let ConfigValue::Map(result_arc) = &result.inner {
            assert!(
                Arc::ptr_eq(result_arc, &shared),
                "merge of Arc::ptr_eq maps must return the same Arc (zero allocation)"
            );
        } else {
            panic!("expected ConfigValue::Map, got {:?}", result.inner);
        }
    }

    #[test]
    fn test_values_equal_string_join_false() {
        let s1 = ConfigValue::string("a");
        let s2 = ConfigValue::string("a");
        assert!(!values_equal(&s1, &s2, &MergeStrategy::join(":")));
        assert!(!values_equal(&s1, &s2, &MergeStrategy::join_append(",")));
    }

    #[test]
    fn test_values_equal_array_strategies_false() {
        let a1 = ConfigValue::array(vec![]);
        let a2 = ConfigValue::array(vec![]);
        assert!(!values_equal(&a1, &a2, &MergeStrategy::Append));
        assert!(!values_equal(&a1, &a2, &MergeStrategy::join_append(",")));
        assert!(!values_equal(&a1, &a2, &MergeStrategy::Prepend));
    }

    #[test]
    fn test_values_equal_deep_merge_non_map() {
        // DeepMerge on non-map types falls to _ => low == high
        assert!(values_equal(
            &ConfigValue::I64(1),
            &ConfigValue::I64(1),
            &MergeStrategy::DeepMerge
        ));
        assert!(!values_equal(
            &ConfigValue::I64(1),
            &ConfigValue::I64(2),
            &MergeStrategy::DeepMerge
        ));
    }

    #[test]
    fn test_apply_leaf_strategy_replace() {
        let result = apply_leaf_strategy(
            &ConfigValue::string("low"),
            &ConfigValue::string("high"),
            &MergeStrategy::Replace,
        );
        assert_eq!(result.as_str(), Some("high"));
    }

    #[test]
    fn test_apply_leaf_strategy_join_and_join_append() {
        let result = apply_leaf_strategy(
            &ConfigValue::string("a"),
            &ConfigValue::string("b"),
            &MergeStrategy::join("-"),
        );
        assert_eq!(result.as_str(), Some("a-b"));

        let result = apply_leaf_strategy(
            &ConfigValue::string("x"),
            &ConfigValue::string("y"),
            &MergeStrategy::join_append("+"),
        );
        assert_eq!(result.as_str(), Some("x+y"));
    }

    #[test]
    fn test_apply_leaf_strategy_append_prepend() {
        let low = ConfigValue::array(vec![AnnotatedValue::new(
            ConfigValue::I64(1),
            SourceId::new("l"),
            "t.0",
        )]);
        let high = ConfigValue::array(vec![AnnotatedValue::new(
            ConfigValue::I64(2),
            SourceId::new("h"),
            "t.0",
        )]);

        let appended = apply_leaf_strategy(&low, &high, &MergeStrategy::Append);
        assert_eq!(appended.as_array().unwrap().len(), 2);

        let prepended = apply_leaf_strategy(&low, &high, &MergeStrategy::Prepend);
        assert_eq!(prepended.as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_apply_leaf_strategy_custom() {
        fn custom_fn(low: &ConfigValue, _high: &ConfigValue) -> ConfigValue {
            low.clone()
        }
        let result = apply_leaf_strategy(
            &ConfigValue::string("keep_low"),
            &ConfigValue::string("ignore_high"),
            &MergeStrategy::custom("test", custom_fn),
        );
        assert_eq!(result.as_str(), Some("keep_low"));
    }

    #[test]
    fn test_apply_leaf_strategy_fallback() {
        // Type mismatch with non-Replace strategy falls to _ => high.clone()
        let result = apply_leaf_strategy(
            &ConfigValue::string("low"),
            &ConfigValue::I64(42),
            &MergeStrategy::Append,
        );
        assert_eq!(result.as_i64(), Some(42));
    }

    #[test]
    fn test_report_conflict_high_equal_priority() {
        // When high.priority >= low.priority, high wins
        let e = MergeEngine::new();
        let l = AnnotatedValue::new(ConfigValue::string("low"), SourceId::new("l"), "t")
            .with_priority(5);
        let h = AnnotatedValue::new(ConfigValue::string("high"), SourceId::new("h"), "t")
            .with_priority(5);
        let report = e.report_conflict(&l, &h).unwrap();
        assert_eq!(report.winner, ConflictWinner::High);
    }

    #[test]
    fn test_merge_preserves_priority_and_version() {
        let e = MergeEngine::new();
        let l = AnnotatedValue::new(ConfigValue::string("low"), SourceId::new("l"), "t")
            .with_priority(10)
            .with_version(3);
        let h = AnnotatedValue::new(ConfigValue::string("high"), SourceId::new("h"), "t")
            .with_priority(20)
            .with_version(5);
        let merged = e.merge(&l, &h).unwrap();
        assert_eq!(merged.priority, 20); // max(10, 20)
        assert_eq!(merged.version, 6); // max(3, 5) + 1
        assert_eq!(merged.source.as_str(), "h"); // high source
    }

    #[test]
    fn test_merge_with_location_preserved() {
        let e = MergeEngine::new();
        let loc = crate::types::SourceLocation::new("f.toml", 1, 1);
        let l = AnnotatedValue::new(ConfigValue::string("low"), SourceId::new("l"), "t");
        let h = AnnotatedValue::new(ConfigValue::string("high"), SourceId::new("h"), "t")
            .with_location(loc.clone());
        let merged = e.merge(&l, &h).unwrap();
        assert_eq!(merged.location, Some(loc));
    }

    #[test]
    fn test_merge_with_location_from_low_when_high_none() {
        let e = MergeEngine::new();
        let loc = crate::types::SourceLocation::new("low.toml", 2, 3);
        let l = AnnotatedValue::new(ConfigValue::string("low"), SourceId::new("l"), "t")
            .with_location(loc.clone());
        let h = AnnotatedValue::new(ConfigValue::string("high"), SourceId::new("h"), "t");
        let merged = e.merge(&l, &h).unwrap();
        assert_eq!(merged.location, Some(loc));
    }
}
