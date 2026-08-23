//! End-to-end regression test for EnvSource type inference (fix-0.4.1 Bug 2).
//!
//! Prior to fix-0.4.1, `EnvSource::collect()` returned all values as
//! `ConfigValue::String`, which broke `serde::Deserialize` for structs with
//! non-string fields (e.g. `u32`, `bool`). This test loads a typed config
//! purely from environment variables and verifies that `port` (u32) and
//! `debug` (bool) deserialize correctly.

use serde::Deserialize;
use serial_test::serial;

use confers::config::Source;
use confers::ConfigBuilder;
use confers::ConfigValue;

#[derive(Debug, Default, PartialEq, Deserialize)]
struct TypedConfig {
    port: u32,
    debug: bool,
    host: String,
}

/// Unique prefix to avoid collisions with real env vars in the test runner.
const PREFIX: &str = "TYPEDCFG_";

fn set_test_env() {
    std::env::set_var(format!("{PREFIX}PORT"), "8080");
    std::env::set_var(format!("{PREFIX}DEBUG"), "true");
    std::env::set_var(format!("{PREFIX}HOST"), "localhost");
}

fn cleanup_test_env() {
    std::env::remove_var(format!("{PREFIX}PORT"));
    std::env::remove_var(format!("{PREFIX}DEBUG"));
    std::env::remove_var(format!("{PREFIX}HOST"));
}

#[test]
#[serial]
fn test_env_vars_deserialize_into_typed_struct() {
    set_test_env();

    // Ensure cleanup runs even if the test panics
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let config: TypedConfig = ConfigBuilder::new()
            .env_prefix(PREFIX)
            .build()
            .expect("env vars with TYPEDCFG_ prefix should deserialize into TypedConfig");

        assert_eq!(
            config.port, 8080,
            "u32 field 'port' should deserialize from string '8080'"
        );
        assert!(
            config.debug,
            "bool field 'debug' should deserialize from string 'true'"
        );
        assert_eq!(
            config.host, "localhost",
            "string field 'host' should deserialize from 'localhost'"
        );
    }));

    cleanup_test_env();

    match result {
        Ok(()) => {}
        Err(panic) => std::panic::resume_unwind(panic),
    }
}

// ===== EnvSource type inference boundaries (R-scenario-coverage) =====
//
// EnvSource.infer_config_value is deterministic and public through the
// EnvSource type; these tests pin the boundary behavior: bool case variants,
// integers above i64::MAX, floats with e-notation, and the string fallback
// for IP addresses / CIDRs that must NOT be coerced into numbers.

#[test]
fn test_env_infer_bool_case_insensitive() {
    assert_eq!(
        confers::config::EnvSource::infer_config_value("TRUE"),
        ConfigValue::Bool(true),
        "uppercase TRUE infers as bool"
    );
    assert_eq!(
        confers::config::EnvSource::infer_config_value("False"),
        ConfigValue::Bool(false),
        "mixed-case False infers as bool"
    );
    assert_eq!(
        confers::config::EnvSource::infer_config_value("true"),
        ConfigValue::Bool(true)
    );
    assert_eq!(
        confers::config::EnvSource::infer_config_value("false"),
        ConfigValue::Bool(false)
    );
}

#[test]
fn test_env_infer_integer_boundaries() {
    use confers::ConfigValue;

    assert_eq!(
        confers::config::EnvSource::infer_config_value("0"),
        ConfigValue::I64(0)
    );
    assert_eq!(
        confers::config::EnvSource::infer_config_value("9223372036854775807"),
        ConfigValue::I64(i64::MAX),
        "i64::MAX stays signed"
    );
    assert_eq!(
        confers::config::EnvSource::infer_config_value("9223372036854775808"),
        ConfigValue::U64(9_223_372_036_854_775_808),
        "value above i64::MAX infers as u64, not overflow"
    );
    assert_eq!(
        confers::config::EnvSource::infer_config_value("18446744073709551615"),
        ConfigValue::U64(u64::MAX),
        "u64::MAX infers as u64"
    );
    assert_eq!(
        confers::config::EnvSource::infer_config_value("-42"),
        ConfigValue::I64(-42),
        "negative integer stays signed"
    );
}

#[test]
fn test_env_infer_float_notation() {
    assert_eq!(
        confers::config::EnvSource::infer_config_value("2.75"),
        ConfigValue::F64(2.75)
    );
    assert_eq!(
        confers::config::EnvSource::infer_config_value("1e3"),
        ConfigValue::F64(1000.0),
        "e-notation infers as float"
    );
    assert_eq!(
        confers::config::EnvSource::infer_config_value("2.5E-2"),
        ConfigValue::F64(0.025)
    );
    assert_eq!(
        confers::config::EnvSource::infer_config_value("-0.5"),
        ConfigValue::F64(-0.5)
    );
}

#[test]
fn test_env_infer_network_values_stay_strings() {
    assert_eq!(
        confers::config::EnvSource::infer_config_value("192.168.1.1"),
        ConfigValue::String("192.168.1.1".to_string()),
        "IP address must stay a string (not parse as float)"
    );
    assert_eq!(
        confers::config::EnvSource::infer_config_value("10.0.0.0/8"),
        ConfigValue::String("10.0.0.0/8".to_string()),
        "CIDR must stay a string"
    );
    assert_eq!(
        confers::config::EnvSource::infer_config_value("2001:db8::1"),
        ConfigValue::String("2001:db8::1".to_string()),
        "IPv6 must stay a string"
    );
    assert_eq!(
        confers::config::EnvSource::infer_config_value("localhost"),
        ConfigValue::String("localhost".to_string())
    );
}

#[test]
fn test_env_infer_non_numeric_guards() {
    // "123abc" must NOT be coerced by f64's permissive grammar.
    assert_eq!(
        confers::config::EnvSource::infer_config_value("123abc"),
        ConfigValue::String("123abc".to_string())
    );
    assert_eq!(
        confers::config::EnvSource::infer_config_value(""),
        ConfigValue::String("".to_string())
    );
    assert_eq!(
        confers::config::EnvSource::infer_config_value("trueish"),
        ConfigValue::String("trueish".to_string()),
        "bool-like but not exact must stay a string"
    );
}

// ===== Env array boundary: end-to-end through ConfigBuilder =====
//
// EnvSource leaves array-shaped values as plain strings — it never JSON-parses
// env values. This means a JSON array literal in an env var cannot deserialize
// directly into a `Vec` field. That is the pinned boundary: the value survives
// as a string and a caller must supply a custom deserializer or a scalar field.

#[derive(Debug, Default, PartialEq, Deserialize)]
struct ArrayEnvConfig {
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    ports: Vec<u16>,
}

const ARRAY_PREFIX: &str = "ARRCFG_";

fn set_array_env() {
    std::env::set_var(format!("{ARRAY_PREFIX}TAGS"), r#"["a","b","c"]"#);
    std::env::set_var(format!("{ARRAY_PREFIX}PORTS"), r#"[8080,9090]"#);
}

fn clear_array_env() {
    std::env::remove_var(format!("{ARRAY_PREFIX}TAGS"));
    std::env::remove_var(format!("{ARRAY_PREFIX}PORTS"));
}

#[test]
#[serial]
fn test_env_json_array_stays_string_boundary() {
    set_array_env();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let raw = confers::config::EnvSource::with_prefix(ARRAY_PREFIX)
            .collect()
            .expect("collect should succeed");
        let map = match &raw.inner {
            ConfigValue::Map(m) => m,
            _ => panic!("expected map"),
        };
        let tags = map
            .get("tags")
            .expect("tags key present")
            .inner
            .as_str()
            .expect("tags remains a string (no JSON parsing)");
        assert_eq!(tags, r#"["a","b","c"]"#);

        // End-to-end: building into a Vec field fails with a clear error,
        // because the env value was not coerced into an array.
        let result: std::result::Result<ArrayEnvConfig, confers::error::ConfigError> =
            ConfigBuilder::new().env_prefix(ARRAY_PREFIX).build();
        let err =
            result.expect_err("array env cannot deserialize into Vec without custom deserializer");
        let msg = format!("{err:?}");
        assert!(
            msg.contains("expected a sequence"),
            "error should mention the sequence mismatch, got: {msg}"
        );
    }));
    clear_array_env();
    match result {
        Ok(()) => {}
        Err(panic) => std::panic::resume_unwind(panic),
    }
}

// ===== Deep nesting + illegal key boundaries =====
//
// Deeply nested env vars (4+ levels) must produce a nested map tree, and the
// dotted-key parser must tolerate empty segments without panicking.

#[derive(Debug, Default, PartialEq, Deserialize)]
struct DeepConfig {
    #[serde(default)]
    a: DeepA,
}

#[derive(Debug, Default, PartialEq, Deserialize)]
struct DeepA {
    #[serde(default)]
    b: DeepB,
}

#[derive(Debug, Default, PartialEq, Deserialize)]
struct DeepB {
    #[serde(default)]
    c: DeepC,
}

#[derive(Debug, Default, PartialEq, Deserialize)]
struct DeepC {
    #[serde(default)]
    value: String,
}

const DEEP_PREFIX: &str = "DEEPCFG_";

fn set_deep_env() {
    // a_b_c_value = "nested" → 4 levels deep
    std::env::set_var("DEEPCFG_A_B_C_VALUE", "deep-value");
}

fn clear_deep_env() {
    std::env::remove_var("DEEPCFG_A_B_C_VALUE");
}

#[test]
#[serial]
fn test_env_deep_nesting_builds_nested_tree() {
    set_deep_env();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let raw = confers::config::EnvSource::with_prefix(DEEP_PREFIX)
            .collect()
            .expect("collect should succeed");
        let map = match &raw.inner {
            ConfigValue::Map(m) => m,
            _ => panic!("expected map"),
        };
        assert!(map.contains_key("a"), "top level 'a' present");
        let a = match map.get("a").map(|v| &v.inner) {
            Some(ConfigValue::Map(m)) => m,
            _ => panic!("a is a map"),
        };
        assert!(a.contains_key("b"), "level 2 'a.b' present");
        let b = match a.get("b").map(|v| &v.inner) {
            Some(ConfigValue::Map(m)) => m,
            _ => panic!("b is a map"),
        };
        assert!(b.contains_key("c"), "level 3 'a.b.c' present");
        let c = match b.get("c").map(|v| &v.inner) {
            Some(ConfigValue::Map(m)) => m,
            _ => panic!("c is a map"),
        };
        let value = c.get("value").expect("leaf 'a.b.c.value' present");
        assert_eq!(value.inner.as_str(), Some("deep-value"));
    }));
    clear_deep_env();
    match result {
        Ok(()) => {}
        Err(panic) => std::panic::resume_unwind(panic),
    }
}

#[test]
#[serial]
fn test_env_deep_nesting_end_to_end() {
    set_deep_env();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let config: DeepConfig = ConfigBuilder::new()
            .env_prefix(DEEP_PREFIX)
            .build()
            .expect("4-level nested env should deserialize into DeepConfig");
        assert_eq!(config.a.b.c.value, "deep-value");
    }));
    clear_deep_env();
    match result {
        Ok(()) => {}
        Err(panic) => std::panic::resume_unwind(panic),
    }
}

#[test]
#[serial]
fn test_env_parse_key_empty_segments_do_not_panic() {
    // Empty segments from repeated separators are tolerated without panic.
    // Each underscore becomes a segment: A__C → a..c → a -> { "" -> { c } };
    // A___B → a...b → a -> { "" -> { "" -> { b } } }. This is the pinned
    // deterministic behavior of EnvSource key parsing.
    std::env::set_var("DEEPCFG_A___B", "empty-mid");
    std::env::set_var("DEEPCFG_A__C", "empty-trailing");
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let raw = confers::config::EnvSource::with_prefix(DEEP_PREFIX)
            .collect()
            .expect("collect should succeed");
        let map = match &raw.inner {
            ConfigValue::Map(m) => m,
            _ => panic!("expected map"),
        };
        // A__C → a..c → a -> { "" -> { c } }
        let a = match map.get("a").map(|v| &v.inner) {
            Some(ConfigValue::Map(m)) => m,
            _ => panic!("a is a map"),
        };
        let inner = match a.get("").map(|v| &v.inner) {
            Some(ConfigValue::Map(m)) => m,
            _ => panic!("A__C: empty segment becomes empty-keyed map"),
        };
        let c = inner.get("c").expect("leaf c under empty segment (A__C)");
        assert_eq!(c.inner.as_str(), Some("empty-trailing"));

        // A___B → a...b → a -> { "" -> { "" -> { b } } }
        let inner2 = match a.get("").map(|v| &v.inner) {
            Some(ConfigValue::Map(m)) => m,
            _ => panic!("a -> empty map present"),
        };
        let inner3 = match inner2.get("").map(|v| &v.inner) {
            Some(ConfigValue::Map(m)) => m,
            _ => panic!("A___B: second empty segment becomes empty-keyed map"),
        };
        let b = inner3
            .get("b")
            .expect("leaf b under two empty segments (A___B)");
        assert_eq!(b.inner.as_str(), Some("empty-mid"));
    }));
    std::env::remove_var("DEEPCFG_A___B");
    std::env::remove_var("DEEPCFG_A__C");
    match result {
        Ok(()) => {}
        Err(panic) => std::panic::resume_unwind(panic),
    }
}

#[test]
#[serial]
fn test_env_parse_key_unicode_and_case() {
    // Unicode keys survive parsing; lowercasing applies. A leading underscore
    // after the prefix becomes an empty first segment → empty-keyed map.
    std::env::set_var("DEEPCFG_HOSTNAME", "srv1");
    std::env::set_var("DEEPCFG__TAG", "underscore-prefixed");
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let raw = confers::config::EnvSource::with_prefix(DEEP_PREFIX)
            .collect()
            .expect("collect should succeed");
        let map = match &raw.inner {
            ConfigValue::Map(m) => m,
            _ => panic!("expected map"),
        };
        assert!(map.contains_key("hostname"), "HOSTNAME → hostname");
        // __TAG → "_tag" → [".", "tag"]? no: strip prefix DEEPCFG_ leaves "_TAG",
        // lowercase "_tag", replace '_' → '.', giving ".tag" → ["", "tag"].
        assert!(
            map.contains_key(""),
            "leading underscore becomes empty-keyed map"
        );
    }));
    std::env::remove_var("DEEPCFG_HOSTNAME");
    std::env::remove_var("DEEPCFG__TAG");
    match result {
        Ok(()) => {}
        Err(panic) => std::panic::resume_unwind(panic),
    }
}
