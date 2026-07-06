//! Shared format-to-ConfigValue conversion functions.
//!
//! These functions are used by both the loader and the format converter modules
//! to avoid duplicating the same conversion logic.

#[cfg(any(feature = "toml", feature = "json", feature = "yaml"))]
use crate::types::{AnnotatedValue, ConfigValue, SourceId};

#[cfg(any(feature = "toml", feature = "json", feature = "yaml"))]
use std::sync::Arc;

#[cfg(feature = "toml")]
pub(crate) fn toml_table_to_config_value(
    table: &toml::Table,
    source: &SourceId,
    prefix: &str,
) -> ConfigValue {
    let entries: Vec<(Arc<str>, AnnotatedValue)> = table
        .iter()
        .map(|(k, v)| {
            let path = if prefix.is_empty() {
                k.clone()
            } else {
                format!("{}.{}", prefix, k)
            };
            (
                Arc::from(k.clone()),
                AnnotatedValue::new(
                    toml_value_to_config_value(v, source, &path),
                    source.clone(),
                    k.clone(),
                ),
            )
        })
        .collect();
    ConfigValue::map(entries)
}

#[cfg(feature = "toml")]
pub(crate) fn toml_value_to_config_value(
    value: &toml::Value,
    source: &SourceId,
    prefix: &str,
) -> ConfigValue {
    match value {
        toml::Value::String(s) => ConfigValue::String(s.clone()),
        toml::Value::Integer(i) => ConfigValue::I64(*i),
        toml::Value::Float(f) => ConfigValue::F64(*f),
        toml::Value::Boolean(b) => ConfigValue::Bool(*b),
        toml::Value::Datetime(dt) => ConfigValue::String(dt.to_string()),
        toml::Value::Array(arr) => ConfigValue::Array(
            arr.iter()
                .enumerate()
                .map(|(i, v)| {
                    let path = format!("{}.{}", prefix, i);
                    AnnotatedValue::new(
                        toml_value_to_config_value(v, source, &path),
                        source.clone(),
                        path,
                    )
                })
                .collect::<Vec<_>>()
                .into(),
        ),
        toml::Value::Table(t) => toml_table_to_config_value(t, source, prefix),
    }
}

#[cfg(feature = "json")]
pub(crate) fn json_to_config_value(
    v: &serde_json::Value,
    source: &SourceId,
    prefix: &str,
) -> ConfigValue {
    match v {
        serde_json::Value::Null => ConfigValue::Null,
        serde_json::Value::Bool(b) => ConfigValue::Bool(*b),
        serde_json::Value::Number(n) => n
            .as_i64()
            .map(ConfigValue::I64)
            .or_else(|| n.as_u64().map(ConfigValue::U64))
            .or_else(|| n.as_f64().map(ConfigValue::F64))
            .unwrap_or(ConfigValue::Null),
        serde_json::Value::String(s) => ConfigValue::String(s.clone()),
        serde_json::Value::Array(a) => ConfigValue::Array(
            a.iter()
                .enumerate()
                .map(|(i, v)| {
                    let p = format!("{}.{}", prefix, i);
                    AnnotatedValue::new(json_to_config_value(v, source, &p), source.clone(), p)
                })
                .collect::<Vec<_>>()
                .into(),
        ),
        serde_json::Value::Object(o) => ConfigValue::map(
            o.iter()
                .map(|(k, v)| {
                    let p = if prefix.is_empty() {
                        k.clone()
                    } else {
                        format!("{}.{}", prefix, k)
                    };
                    (
                        Arc::from(k.clone()),
                        AnnotatedValue::new(
                            json_to_config_value(v, source, &p),
                            source.clone(),
                            k.clone(),
                        ),
                    )
                })
                .collect(),
        ),
    }
}

#[cfg(feature = "yaml")]
pub(crate) fn yaml_to_config_value(
    v: &serde_yaml_ng::Value,
    source: &SourceId,
    prefix: &str,
) -> ConfigValue {
    match v {
        serde_yaml_ng::Value::Null => ConfigValue::Null,
        serde_yaml_ng::Value::Bool(b) => ConfigValue::Bool(*b),
        serde_yaml_ng::Value::Number(n) => n
            .as_i64()
            .map(ConfigValue::I64)
            .or_else(|| n.as_u64().map(ConfigValue::U64))
            .or_else(|| n.as_f64().map(ConfigValue::F64))
            .unwrap_or(ConfigValue::Null),
        serde_yaml_ng::Value::String(s) => ConfigValue::String(s.clone()),
        serde_yaml_ng::Value::Sequence(s) => ConfigValue::Array(
            s.iter()
                .enumerate()
                .map(|(i, v)| {
                    let p = format!("{}.{}", prefix, i);
                    AnnotatedValue::new(yaml_to_config_value(v, source, &p), source.clone(), p)
                })
                .collect::<Vec<_>>()
                .into(),
        ),
        serde_yaml_ng::Value::Mapping(m) => ConfigValue::map(
            m.iter()
                .filter_map(|(k, v)| {
                    k.as_str().map(|key| {
                        let p = if prefix.is_empty() {
                            key.to_string()
                        } else {
                            format!("{}.{}", prefix, key)
                        };
                        (
                            Arc::from(key),
                            AnnotatedValue::new(
                                yaml_to_config_value(v, source, &p),
                                source.clone(),
                                key,
                            ),
                        )
                    })
                })
                .collect(),
        ),
        serde_yaml_ng::Value::Tagged(t) => yaml_to_config_value(&t.value, source, prefix),
    }
}

#[cfg(test)]
mod tests {
    #[cfg(any(feature = "toml", feature = "json", feature = "yaml"))]
    use super::*;
    use crate::types::SourceId;

    fn src() -> SourceId {
        SourceId::new("test")
    }

    // ===== toml_value_to_config_value =====

    #[cfg(feature = "toml")]
    #[test]
    fn test_toml_value_string() {
        let v: toml::Value = "hello".into();
        let cv = toml_value_to_config_value(&v, &src(), "p");
        assert_eq!(cv.as_str(), Some("hello"));
    }

    #[cfg(feature = "toml")]
    #[test]
    fn test_toml_value_integer() {
        let v: toml::Value = 42i64.into();
        let cv = toml_value_to_config_value(&v, &src(), "p");
        assert_eq!(cv.as_i64(), Some(42));
    }

    #[cfg(feature = "toml")]
    #[test]
    fn test_toml_value_negative_integer() {
        let v: toml::Value = (-7i64).into();
        let cv = toml_value_to_config_value(&v, &src(), "p");
        assert_eq!(cv.as_i64(), Some(-7));
    }

    #[cfg(feature = "toml")]
    #[test]
    fn test_toml_value_float() {
        let v: toml::Value = 2.5f64.into();
        let cv = toml_value_to_config_value(&v, &src(), "p");
        assert_eq!(cv.as_f64(), Some(2.5));
    }

    #[cfg(feature = "toml")]
    #[test]
    fn test_toml_value_boolean() {
        let v: toml::Value = true.into();
        let cv = toml_value_to_config_value(&v, &src(), "p");
        assert_eq!(cv.as_bool(), Some(true));
    }

    #[cfg(feature = "toml")]
    #[test]
    fn test_toml_value_datetime() {
        let v: toml::Value = toml::Value::Datetime("2024-01-01".parse().unwrap());
        let cv = toml_value_to_config_value(&v, &src(), "p");
        assert!(cv.as_str().unwrap().contains("2024-01-01"));
    }

    #[cfg(feature = "toml")]
    #[test]
    fn test_toml_value_array() {
        let v: toml::Value = toml::Value::Array(vec![1i64.into(), 2i64.into()]);
        let cv = toml_value_to_config_value(&v, &src(), "p");
        let arr = cv.as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0].inner.as_i64(), Some(1));
        assert_eq!(arr[1].inner.as_i64(), Some(2));
        // Path is "{prefix}.{index}"
        assert_eq!(arr[0].path.as_ref(), "p.0");
        assert_eq!(arr[1].path.as_ref(), "p.1");
        assert_eq!(arr[0].source.as_str(), "test");
    }

    #[cfg(feature = "toml")]
    #[test]
    fn test_toml_value_array_nested() {
        let v: toml::Value = toml::Value::Array(vec![toml::Value::Array(vec![1i64.into()])]);
        let cv = toml_value_to_config_value(&v, &src(), "p");
        let outer = cv.as_array().unwrap();
        let inner = outer[0].inner.as_array().unwrap();
        assert_eq!(inner[0].inner.as_i64(), Some(1));
        assert_eq!(inner[0].path.as_ref(), "p.0.0");
    }

    #[cfg(feature = "toml")]
    #[test]
    fn test_toml_value_table_delegates_to_table_fn() {
        let mut tbl = toml::value::Table::new();
        tbl.insert("a".to_string(), 1i64.into());
        tbl.insert("b".to_string(), "x".into());
        let v: toml::Value = tbl.into();
        let cv = toml_value_to_config_value(&v, &src(), "p");
        let map = cv.as_map().unwrap();
        assert_eq!(map.len(), 2);
        // Map key is the bare key from the table; AnnotatedValue.path
        // also stores the bare key (not the prefixed dotted path).
        let a = map.get("a").unwrap();
        assert_eq!(a.inner.as_i64(), Some(1));
        assert_eq!(a.path.as_ref(), "a");
        let b = map.get("b").unwrap();
        assert_eq!(b.inner.as_str(), Some("x"));
        assert_eq!(b.path.as_ref(), "b");
    }

    #[cfg(feature = "toml")]
    #[test]
    fn test_toml_value_table_key_with_dot_in_path() {
        let mut tbl = toml::value::Table::new();
        tbl.insert("name".to_string(), "v".into());
        let v: toml::Value = tbl.into();
        let cv = toml_value_to_config_value(&v, &src(), "prefix");
        let map = cv.as_map().unwrap();
        // Map key is bare "name"; prefix no longer contributes to the map key.
        let n = map.get("name").unwrap();
        assert_eq!(n.inner.as_str(), Some("v"));
    }

    // ===== toml_table_to_config_value =====

    #[cfg(feature = "toml")]
    #[test]
    fn test_toml_table_empty_prefix() {
        let mut tbl = toml::value::Table::new();
        tbl.insert("k".to_string(), 5i64.into());
        let cv = toml_table_to_config_value(&tbl, &src(), "");
        let map = cv.as_map().unwrap();
        let v = map.get("k").unwrap();
        assert_eq!(v.inner.as_i64(), Some(5));
        assert_eq!(v.path.as_ref(), "k");
    }

    #[cfg(feature = "toml")]
    #[test]
    fn test_toml_table_with_prefix() {
        let mut tbl = toml::value::Table::new();
        tbl.insert("k".to_string(), 5i64.into());
        let cv = toml_table_to_config_value(&tbl, &src(), "root");
        let map = cv.as_map().unwrap();
        // Map key is bare "k" (prefix only affects AnnotatedValue.path,
        // which here is also "k" since path stores the bare key).
        let v = map.get("k").unwrap();
        assert_eq!(v.inner.as_i64(), Some(5));
    }

    #[cfg(feature = "toml")]
    #[test]
    fn test_toml_table_empty_table() {
        let tbl = toml::value::Table::new();
        let cv = toml_table_to_config_value(&tbl, &src(), "");
        assert!(cv.is_map());
        assert_eq!(cv.as_map().unwrap().len(), 0);
    }

    #[cfg(feature = "toml")]
    #[test]
    fn test_toml_table_nested_path_construction() {
        let mut inner = toml::value::Table::new();
        inner.insert("x".to_string(), 1i64.into());
        let mut outer = toml::value::Table::new();
        outer.insert("inner".to_string(), inner.into());
        let cv = toml_table_to_config_value(&outer, &src(), "root");
        let map = cv.as_map().unwrap();
        // Outer key: bare "inner" (not "root.inner")
        let inner_av = map.get("inner").unwrap();
        let inner_map = inner_av.inner.as_map().unwrap();
        // Inner key: bare "x" (not "root.inner.x")
        let x_av = inner_map.get("x").unwrap();
        assert_eq!(x_av.inner.as_i64(), Some(1));
    }

    // ===== Regression: bare key for nested tables/objects/mappings (fix-0.4.1) =====

    #[cfg(feature = "toml")]
    #[test]
    fn test_toml_nested_table_uses_bare_key() {
        // TOML equivalent of [database]\nwrite_url = "postgres://x"
        let mut db = toml::value::Table::new();
        db.insert(
            "write_url".to_string(),
            toml::Value::String("postgres://x".into()),
        );
        let mut root = toml::value::Table::new();
        root.insert("database".to_string(), db.into());
        let cv = toml_table_to_config_value(&root, &src(), "");
        let map = cv.as_map().unwrap();
        // Outer key is bare "database"
        let db_av = map
            .get("database")
            .expect("outer key should be bare 'database'");
        let db_map = db_av.inner.as_map().unwrap();
        // Inner key is bare "write_url", NOT "database.write_url"
        let wu = db_map
            .get("write_url")
            .expect("inner key should be bare 'write_url', not 'database.write_url'");
        assert_eq!(wu.inner.as_str(), Some("postgres://x"));
        // Dotted key must NOT exist
        assert!(
            db_map.get("database.write_url").is_none(),
            "dotted-key should not exist as map key"
        );
    }

    #[cfg(feature = "json")]
    #[test]
    fn test_json_nested_object_uses_bare_key() {
        let v = serde_json::json!({ "database": { "write_url": "postgres://x" } });
        let cv = json_to_config_value(&v, &src(), "");
        let map = cv.as_map().unwrap();
        let db_av = map
            .get("database")
            .expect("outer key should be bare 'database'");
        let db_map = db_av.inner.as_map().unwrap();
        let wu = db_map
            .get("write_url")
            .expect("inner key should be bare 'write_url', not 'database.write_url'");
        assert_eq!(wu.inner.as_str(), Some("postgres://x"));
        assert!(
            db_map.get("database.write_url").is_none(),
            "dotted-key should not exist as map key"
        );
    }

    #[cfg(feature = "yaml")]
    #[test]
    fn test_yaml_nested_mapping_uses_bare_key() {
        let v: serde_yaml_ng::Value =
            serde_yaml_ng::from_str("database:\n  write_url: postgres://x\n").unwrap();
        let cv = yaml_to_config_value(&v, &src(), "");
        let map = cv.as_map().unwrap();
        let db_av = map
            .get("database")
            .expect("outer key should be bare 'database'");
        let db_map = db_av.inner.as_map().unwrap();
        let wu = db_map
            .get("write_url")
            .expect("inner key should be bare 'write_url', not 'database.write_url'");
        assert_eq!(wu.inner.as_str(), Some("postgres://x"));
        assert!(
            db_map.get("database.write_url").is_none(),
            "dotted-key should not exist as map key"
        );
    }

    // ===== json_to_config_value =====

    #[cfg(feature = "json")]
    #[test]
    fn test_json_null() {
        let v = serde_json::Value::Null;
        let cv = json_to_config_value(&v, &src(), "p");
        assert!(cv.is_null());
    }

    #[cfg(feature = "json")]
    #[test]
    fn test_json_bool() {
        let v = serde_json::json!(true);
        let cv = json_to_config_value(&v, &src(), "p");
        assert_eq!(cv.as_bool(), Some(true));
    }

    #[cfg(feature = "json")]
    #[test]
    fn test_json_i64() {
        let v = serde_json::json!(-42);
        let cv = json_to_config_value(&v, &src(), "p");
        assert_eq!(cv.as_i64(), Some(-42));
    }

    #[cfg(feature = "json")]
    #[test]
    fn test_json_u64_large() {
        // A value above i64::MAX should land in U64.
        let v = serde_json::json!(u64::MAX);
        let cv = json_to_config_value(&v, &src(), "p");
        assert_eq!(cv.as_u64(), Some(u64::MAX));
    }

    #[cfg(feature = "json")]
    #[test]
    fn test_json_f64() {
        let v = serde_json::json!(2.5);
        let cv = json_to_config_value(&v, &src(), "p");
        assert_eq!(cv.as_f64(), Some(2.5));
    }

    #[cfg(feature = "json")]
    #[test]
    fn test_json_string() {
        let v = serde_json::json!("hi");
        let cv = json_to_config_value(&v, &src(), "p");
        assert_eq!(cv.as_str(), Some("hi"));
    }

    #[cfg(feature = "json")]
    #[test]
    fn test_json_array_paths() {
        let v = serde_json::json!([1, 2]);
        let cv = json_to_config_value(&v, &src(), "p");
        let arr = cv.as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0].path.as_ref(), "p.0");
        assert_eq!(arr[1].path.as_ref(), "p.1");
        assert_eq!(arr[0].source.as_str(), "test");
    }

    #[cfg(feature = "json")]
    #[test]
    fn test_json_array_nested() {
        let v = serde_json::json!([[1]]);
        let cv = json_to_config_value(&v, &src(), "p");
        let outer = cv.as_array().unwrap();
        let inner = outer[0].inner.as_array().unwrap();
        assert_eq!(inner[0].inner.as_i64(), Some(1));
        assert_eq!(inner[0].path.as_ref(), "p.0.0");
    }

    #[cfg(feature = "json")]
    #[test]
    fn test_json_object_empty_prefix() {
        let v = serde_json::json!({ "a": 1 });
        let cv = json_to_config_value(&v, &src(), "");
        let map = cv.as_map().unwrap();
        let a = map.get("a").unwrap();
        assert_eq!(a.inner.as_i64(), Some(1));
        assert_eq!(a.path.as_ref(), "a");
    }

    #[cfg(feature = "json")]
    #[test]
    fn test_json_object_with_prefix() {
        let v = serde_json::json!({ "a": 1 });
        let cv = json_to_config_value(&v, &src(), "root");
        let map = cv.as_map().unwrap();
        let a = map.get("a").unwrap();
        assert_eq!(a.inner.as_i64(), Some(1));
    }

    #[cfg(feature = "json")]
    #[test]
    fn test_json_object_nested_path() {
        let v = serde_json::json!({ "outer": { "inner": 1 } });
        let cv = json_to_config_value(&v, &src(), "");
        let map = cv.as_map().unwrap();
        let outer_av = map.get("outer").unwrap();
        let inner_map = outer_av.inner.as_map().unwrap();
        let inner_av = inner_map.get("inner").unwrap();
        assert_eq!(inner_av.inner.as_i64(), Some(1));
    }

    #[cfg(feature = "json")]
    #[test]
    fn test_json_object_preserves_key_name() {
        let v = serde_json::json!({ "my_key": "val" });
        let cv = json_to_config_value(&v, &src(), "");
        let map = cv.as_map().unwrap();
        let av = map.get("my_key").unwrap();
        // Source path is the constructed path, but original key is preserved separately.
        assert_eq!(av.path.as_ref(), "my_key");
    }

    // ===== yaml_to_config_value =====

    #[cfg(feature = "yaml")]
    #[test]
    fn test_yaml_null() {
        let v: serde_yaml_ng::Value = serde_yaml_ng::from_str("null").unwrap();
        let cv = yaml_to_config_value(&v, &src(), "p");
        assert!(cv.is_null());
    }

    #[cfg(feature = "yaml")]
    #[test]
    fn test_yaml_bool_true() {
        let v: serde_yaml_ng::Value = serde_yaml_ng::from_str("true").unwrap();
        let cv = yaml_to_config_value(&v, &src(), "p");
        assert_eq!(cv.as_bool(), Some(true));
    }

    #[cfg(feature = "yaml")]
    #[test]
    fn test_yaml_bool_false() {
        let v: serde_yaml_ng::Value = serde_yaml_ng::from_str("false").unwrap();
        let cv = yaml_to_config_value(&v, &src(), "p");
        assert_eq!(cv.as_bool(), Some(false));
    }

    #[cfg(feature = "yaml")]
    #[test]
    fn test_yaml_integer_i64() {
        let v: serde_yaml_ng::Value = serde_yaml_ng::from_str("-99").unwrap();
        let cv = yaml_to_config_value(&v, &src(), "p");
        assert_eq!(cv.as_i64(), Some(-99));
    }

    #[cfg(feature = "yaml")]
    #[test]
    fn test_yaml_integer_u64_large() {
        let v: serde_yaml_ng::Value = serde_yaml_ng::from_str(&u64::MAX.to_string()).unwrap();
        let cv = yaml_to_config_value(&v, &src(), "p");
        assert_eq!(cv.as_u64(), Some(u64::MAX));
    }

    #[cfg(feature = "yaml")]
    #[test]
    fn test_yaml_float() {
        let v: serde_yaml_ng::Value = serde_yaml_ng::from_str("1.5").unwrap();
        let cv = yaml_to_config_value(&v, &src(), "p");
        assert_eq!(cv.as_f64(), Some(1.5));
    }

    #[cfg(feature = "yaml")]
    #[test]
    fn test_yaml_string() {
        let v: serde_yaml_ng::Value = serde_yaml_ng::from_str("hello").unwrap();
        let cv = yaml_to_config_value(&v, &src(), "p");
        assert_eq!(cv.as_str(), Some("hello"));
    }

    #[cfg(feature = "yaml")]
    #[test]
    fn test_yaml_sequence_paths() {
        let v: serde_yaml_ng::Value = serde_yaml_ng::from_str("- 1\n- 2").unwrap();
        let cv = yaml_to_config_value(&v, &src(), "p");
        let arr = cv.as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0].inner.as_i64(), Some(1));
        assert_eq!(arr[0].path.as_ref(), "p.0");
        assert_eq!(arr[1].path.as_ref(), "p.1");
    }

    #[cfg(feature = "yaml")]
    #[test]
    fn test_yaml_mapping_empty_prefix() {
        let v: serde_yaml_ng::Value = serde_yaml_ng::from_str("a: 1").unwrap();
        let cv = yaml_to_config_value(&v, &src(), "");
        let map = cv.as_map().unwrap();
        let a = map.get("a").unwrap();
        assert_eq!(a.inner.as_i64(), Some(1));
        assert_eq!(a.path.as_ref(), "a");
    }

    #[cfg(feature = "yaml")]
    #[test]
    fn test_yaml_mapping_with_prefix() {
        let v: serde_yaml_ng::Value = serde_yaml_ng::from_str("a: 1").unwrap();
        let cv = yaml_to_config_value(&v, &src(), "root");
        let map = cv.as_map().unwrap();
        let a = map.get("a").unwrap();
        assert_eq!(a.inner.as_i64(), Some(1));
    }

    #[cfg(feature = "yaml")]
    #[test]
    fn test_yaml_mapping_nested_path() {
        let v: serde_yaml_ng::Value = serde_yaml_ng::from_str("outer:\n  inner: 1").unwrap();
        let cv = yaml_to_config_value(&v, &src(), "");
        let map = cv.as_map().unwrap();
        let outer_av = map.get("outer").unwrap();
        let inner_map = outer_av.inner.as_map().unwrap();
        let inner_av = inner_map.get("inner").unwrap();
        assert_eq!(inner_av.inner.as_i64(), Some(1));
    }

    #[cfg(feature = "yaml")]
    #[test]
    fn test_yaml_mapping_non_string_key_filtered() {
        // Numeric keys cannot be converted to &str and are silently filtered out.
        let v: serde_yaml_ng::Value = serde_yaml_ng::from_str("1: foo\nname: bar").unwrap();
        let cv = yaml_to_config_value(&v, &src(), "");
        let map = cv.as_map().unwrap();
        assert_eq!(map.len(), 1);
        assert!(map.get("name").is_some());
    }

    #[cfg(feature = "yaml")]
    #[test]
    fn test_yaml_tagged_unwraps_inner_value() {
        // A YAML tag like !!str on a numeric value should unwrap to the inner value.
        let v: serde_yaml_ng::Value = serde_yaml_ng::from_str("!!str 42").unwrap();
        let cv = yaml_to_config_value(&v, &src(), "p");
        // The Tagged wrapper is unwrapped, and the inner string "42" is used.
        assert_eq!(cv.as_str(), Some("42"));
    }

    #[cfg(feature = "yaml")]
    #[test]
    fn test_yaml_empty_sequence() {
        let v: serde_yaml_ng::Value = serde_yaml_ng::from_str("[]").unwrap();
        let cv = yaml_to_config_value(&v, &src(), "p");
        let arr = cv.as_array().unwrap();
        assert_eq!(arr.len(), 0);
    }

    #[cfg(feature = "yaml")]
    #[test]
    fn test_yaml_empty_mapping() {
        let v: serde_yaml_ng::Value = serde_yaml_ng::from_str("{}").unwrap();
        let cv = yaml_to_config_value(&v, &src(), "p");
        let map = cv.as_map().unwrap();
        assert_eq!(map.len(), 0);
    }
}
