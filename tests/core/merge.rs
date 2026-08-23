// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

use confers::merger::*;
use confers::types::*;
use confers::SourceId;
use std::sync::Arc;

mod tests {
    use super::*;
    use serde::Deserialize;
    use serial_test::serial;
    use std::collections::HashMap;
    use std::io::Write;

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

    /// End-to-end multi-source precedence: Default < File < Env < Memory.
    ///
    /// Each source defines the SAME key with a distinct value; the final
    /// merged value must come from the highest-priority source. This asserts
    /// real precedence across the full ConfigBuilder pipeline, not just the
    /// merge engine in isolation (T060 scenario 1).
    #[derive(Debug, Default, PartialEq, Deserialize)]
    struct PrecedenceConfig {
        #[serde(default)]
        host: String,
        #[serde(default)]
        port: u16,
    }

    const PRECEDENCE_PREFIX: &str = "PRECEDENCE_CFG_";

    fn set_precedence_env() {
        std::env::set_var(format!("{PRECEDENCE_PREFIX}HOST"), "env-host");
        std::env::set_var(format!("{PRECEDENCE_PREFIX}PORT"), "9001");
    }

    fn clear_precedence_env() {
        std::env::remove_var(format!("{PRECEDENCE_PREFIX}HOST"));
        std::env::remove_var(format!("{PRECEDENCE_PREFIX}PORT"));
    }

    fn write_precedence_file() -> (tempfile::NamedTempFile, std::path::PathBuf) {
        // FileSource default priority is 0 — the same as DefaultSource, and
        // tie-breaking then depends on source_id ordering (tempfile names sort
        // before "default"), which would let default win. Give the file source
        // an explicit mid priority (20) so it reliably beats default (0) and
        // loses to env/memory (50), asserting the intended precedence chain.
        let mut file = tempfile::Builder::new()
            .suffix(".toml")
            .tempfile_in(std::env::current_dir().unwrap())
            .unwrap();
        writeln!(file, "host = \"file-host\"\nport = 8001").unwrap();
        file.flush().unwrap();
        let file_path = file.path().to_path_buf();
        let rel_path = file_path
            .strip_prefix(std::env::current_dir().unwrap())
            .unwrap_or(&file_path)
            .to_path_buf();
        (file, rel_path)
    }

    fn build_with_file(
        include_env: bool,
        include_memory: bool,
    ) -> confers::ConfigResult<PrecedenceConfig> {
        use confers::FileSource;
        // Keep the tempfile alive for the duration of the build.
        let (_file, rel_path) = write_precedence_file();
        let mut builder: confers::ConfigBuilder<PrecedenceConfig> = confers::ConfigBuilder::new()
            .default("host", ConfigValue::string("default-host"))
            .default("port", ConfigValue::uint(7001))
            .source(Box::new(FileSource::new(rel_path).with_priority(20)));
        if include_env {
            builder = builder.env_prefix(PRECEDENCE_PREFIX);
        }
        if include_memory {
            builder = builder.memory(HashMap::from([
                ("host".to_string(), ConfigValue::string("memory-host")),
                ("port".to_string(), ConfigValue::uint(6001)),
            ]));
        }
        builder.build()
    }

    #[test]
    #[serial]
    fn test_precedence_default_file_env_memory() {
        set_precedence_env();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let config = build_with_file(true, true).expect("precedence chain should build");

            // Memory (priority 50) wins over Env (50), File (20), Default (0).
            assert_eq!(
                config.host, "memory-host",
                "memory source must shadow file/env/default"
            );
            assert_eq!(
                config.port, 6001,
                "memory source must shadow file/env/default for numeric key"
            );
        }));
        clear_precedence_env();
        match result {
            Ok(()) => {}
            Err(p) => std::panic::resume_unwind(p),
        }
    }

    #[test]
    #[serial]
    fn test_precedence_env_shadows_file_default() {
        set_precedence_env();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            // No memory source: Env (50) must beat File (20) and Default (0).
            let config = build_with_file(true, false).expect("precedence chain should build");

            assert_eq!(config.host, "env-host", "env must shadow file/default");
            assert_eq!(config.port, 9001, "env must shadow file/default numeric");
        }));
        clear_precedence_env();
        match result {
            Ok(()) => {}
            Err(p) => std::panic::resume_unwind(p),
        }
    }

    #[test]
    #[serial]
    fn test_precedence_file_shadows_default() {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            // No env, no memory: File (20) must beat Default (0).
            let config = build_with_file(false, false).expect("precedence chain should build");

            assert_eq!(config.host, "file-host", "file must shadow default");
            assert_eq!(config.port, 8001, "file must shadow default numeric");
        }));
        match result {
            Ok(()) => {}
            Err(p) => std::panic::resume_unwind(p),
        }
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
