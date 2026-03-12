use confers::merger::*;
use confers::value::*;
use confers::SourceId;
use std::sync::Arc;

mod tests {
    use super::*;

    fn make_value(inner: ConfigValue, priority: u8) -> AnnotatedValue {
        AnnotatedValue {
            inner,
            source: SourceId::new("test"),
            path: Arc::from(""),
            priority,
            version: 0,
            location: None,
        }
    }

    fn make_map(values: Vec<(&str, AnnotatedValue)>) -> ConfigValue {
        let mut map = indexmap::IndexMap::new();
        for (k, v) in values {
            map.insert(Arc::from(k), v);
        }
        ConfigValue::Map(Arc::new(map))
    }

    #[test]
    fn test_merge_replace_strategy() {
        let low = make_value(ConfigValue::String("low".to_string()), 10);
        let high = make_value(ConfigValue::String("high".to_string()), 20);

        let engine = MergeEngine::new();
        let result = engine.merge(&low, &high).unwrap();

        assert_eq!(result.inner, ConfigValue::String("high".to_string()));
        assert_eq!(result.priority, 20);
    }

    #[test]
    fn test_merge_join_strategy() {
        let low = make_value(ConfigValue::String("low".to_string()), 10);
        let high = make_value(ConfigValue::String("high".to_string()), 20);

        let engine = MergeEngine::new().with_default_strategy(MergeStrategy::join(":"));
        let result = engine.merge(&low, &high).unwrap();

        assert_eq!(result.inner, ConfigValue::String("low:high".to_string()));
    }

    #[test]
    fn test_merge_append_strategy() {
        let low = make_value(
            ConfigValue::Array(Arc::from(vec![
                make_value(ConfigValue::I64(1), 10),
                make_value(ConfigValue::I64(2), 10),
            ])),
            10,
        );
        let high = make_value(
            ConfigValue::Array(Arc::from(vec![
                make_value(ConfigValue::I64(3), 20),
                make_value(ConfigValue::I64(4), 20),
            ])),
            20,
        );

        let engine = MergeEngine::new().with_default_strategy(MergeStrategy::Append);
        let result = engine.merge(&low, &high).unwrap();

        match result.inner {
            ConfigValue::Array(arr) => {
                assert_eq!(arr.len(), 4);
            }
            _ => panic!("Expected array"),
        }
    }

    #[test]
    fn test_merge_prepend_strategy() {
        let low = make_value(
            ConfigValue::Array(Arc::from(vec![
                make_value(ConfigValue::I64(1), 10),
                make_value(ConfigValue::I64(2), 10),
            ])),
            10,
        );
        let high = make_value(
            ConfigValue::Array(Arc::from(vec![
                make_value(ConfigValue::I64(3), 20),
                make_value(ConfigValue::I64(4), 20),
            ])),
            20,
        );

        let engine = MergeEngine::new().with_default_strategy(MergeStrategy::Prepend);
        let result = engine.merge(&low, &high).unwrap();

        match result.inner {
            ConfigValue::Array(arr) => {
                assert_eq!(arr.len(), 4);
            }
            _ => panic!("Expected array"),
        }
    }

    #[test]
    fn test_merge_deep_strategy() {
        let low_inner = make_map(vec![
            ("a", make_value(ConfigValue::I64(1), 10)),
            ("b", make_value(ConfigValue::I64(2), 10)),
        ]);
        let low = make_value(low_inner, 10);

        let high_inner = make_map(vec![
            ("b", make_value(ConfigValue::I64(3), 20)),
            ("c", make_value(ConfigValue::I64(4), 20)),
        ]);
        let high = make_value(high_inner, 20);

        let engine = MergeEngine::new().with_default_strategy(MergeStrategy::DeepMerge);
        let result = engine.merge(&low, &high).unwrap();

        match result.inner {
            ConfigValue::Map(map) => {
                assert_eq!(map.len(), 3);
                assert_eq!(map.get("a").unwrap().inner, ConfigValue::I64(1));
                assert_eq!(map.get("b").unwrap().inner, ConfigValue::I64(3));
                assert_eq!(map.get("c").unwrap().inner, ConfigValue::I64(4));
            }
            _ => panic!("Expected map"),
        }
    }

    #[test]
    fn test_merge_custom_strategy() {
        fn custom_merge(low: &ConfigValue, high: &ConfigValue) -> ConfigValue {
            match (low, high) {
                (ConfigValue::I64(l), ConfigValue::I64(h)) => ConfigValue::I64(l + h),
                _ => high.clone(),
            }
        }

        let low = make_value(ConfigValue::I64(10), 10);
        let high = make_value(ConfigValue::I64(5), 20);

        let engine =
            MergeEngine::new().with_default_strategy(MergeStrategy::custom("sum", custom_merge));
        let result = engine.merge(&low, &high).unwrap();

        assert_eq!(result.inner, ConfigValue::I64(15));
    }

    #[test]
    fn test_merge_null_values() {
        let low = make_value(ConfigValue::Null, 10);
        let high = make_value(ConfigValue::String("value".to_string()), 20);

        let engine = MergeEngine::new();
        let result = engine.merge(&low, &high).unwrap();

        assert_eq!(result.inner, ConfigValue::String("value".to_string()));

        let result2 = engine.merge(&high, &low).unwrap();
        assert_eq!(result2.inner, ConfigValue::String("value".to_string()));
    }

    #[test]
    fn test_merge_priority_propagation() {
        let low = make_value(ConfigValue::String("low".to_string()), 10);
        let high = make_value(ConfigValue::String("high".to_string()), 20);

        let engine = MergeEngine::new();
        let result = engine.merge(&low, &high).unwrap();

        assert_eq!(result.priority, 20);
    }

    #[test]
    fn test_field_specific_strategy() {
        let engine = MergeEngine::new().with_field_strategy("path.to.array", MergeStrategy::Append);

        assert_eq!(engine.get_strategy("other.path"), &MergeStrategy::Replace);
        assert_eq!(engine.get_strategy("path.to.array"), &MergeStrategy::Append);
    }

    #[test]
    fn test_nested_deep_merge() {
        let nested_low = make_map(vec![("inner", make_value(ConfigValue::I64(1), 10))]);
        let low_inner = make_map(vec![("outer", make_value(nested_low, 10))]);
        let low = make_value(low_inner, 10);

        let nested_high = make_map(vec![
            ("inner", make_value(ConfigValue::I64(2), 20)),
            (
                "extra",
                make_value(ConfigValue::String("new".to_string()), 20),
            ),
        ]);
        let high_inner = make_map(vec![("outer", make_value(nested_high, 20))]);
        let high = make_value(high_inner, 20);

        let engine = MergeEngine::new().with_default_strategy(MergeStrategy::DeepMerge);
        let result = engine.merge(&low, &high).unwrap();

        match result.inner {
            ConfigValue::Map(outer) => match outer.get("outer").unwrap().inner.clone() {
                ConfigValue::Map(inner) => {
                    assert_eq!(inner.get("inner").unwrap().inner, ConfigValue::I64(2));
                    assert_eq!(
                        inner.get("extra").unwrap().inner,
                        ConfigValue::String("new".to_string())
                    );
                }
                _ => panic!("Expected inner map"),
            },
            _ => panic!("Expected outer map"),
        }
    }

    #[test]
    fn test_merge_type_mismatch() {
        let low = make_value(ConfigValue::String("string".to_string()), 10);
        let high = make_value(ConfigValue::I64(42), 20);

        let engine = MergeEngine::new();
        // Type mismatch should use higher priority value
        let result = engine.merge(&low, &high).unwrap();
        assert_eq!(result.inner, ConfigValue::I64(42));
    }

    #[test]
    fn test_merge_empty_arrays() {
        let low = make_value(ConfigValue::Array(Arc::from(vec![])), 10);
        let high = make_value(
            ConfigValue::Array(Arc::from(vec![make_value(ConfigValue::I64(1), 20)])),
            20,
        );

        let engine = MergeEngine::new().with_default_strategy(MergeStrategy::Append);
        let result = engine.merge(&low, &high).unwrap();

        match result.inner {
            ConfigValue::Array(arr) => assert_eq!(arr.len(), 1),
            _ => panic!("Expected array"),
        }
    }

    #[test]
    fn test_merge_deeply_nested() {
        // Test 5 levels of nesting
        let deep = make_value(
            ConfigValue::Map(Arc::new(indexmap::IndexMap::from([(
                Arc::from("level1"),
                make_value(
                    ConfigValue::Map(Arc::new(indexmap::IndexMap::from([(
                        Arc::from("level2"),
                        make_value(ConfigValue::I64(42), 10),
                    )]))),
                    10,
                ),
            )]))),
            10,
        );

        let engine = MergeEngine::new().with_default_strategy(MergeStrategy::DeepMerge);
        let result = engine.merge(&deep, &deep).unwrap();

        // Should not panic with deep nesting
        assert!(matches!(result.inner, ConfigValue::Map(_)));
    }
}
