//! Comprehensive coverage tests for confers library.
//! These tests exercise core APIs across multiple modules.

use confers::config;
use confers::config::Source;
use confers::error;
use confers::interface;
use confers::loader;
#[cfg(feature = "snapshot")]
use confers::snapshot::SnapshotFormat;
use confers::ConfigValue;
#[cfg(feature = "progressive-reload")]
use confers::HealthStatus;

// ============ Loader Module ============

#[test]
fn test_format_try_parse_all() {
    assert_eq!(
        loader::Format::try_parse("toml"),
        Some(loader::Format::Toml)
    );
    assert_eq!(
        loader::Format::try_parse("TOML"),
        Some(loader::Format::Toml)
    );
    assert_eq!(
        loader::Format::try_parse("json"),
        Some(loader::Format::Json)
    );
    assert_eq!(
        loader::Format::try_parse("yaml"),
        Some(loader::Format::Yaml)
    );
    assert_eq!(loader::Format::try_parse("yml"), Some(loader::Format::Yaml));
    assert_eq!(loader::Format::try_parse("ini"), Some(loader::Format::Ini));
    assert_eq!(loader::Format::try_parse("unknown"), None);
}

#[test]
fn test_format_all_contains() {
    assert!(loader::Format::all().contains(&loader::Format::Toml));
    assert!(loader::Format::all().contains(&loader::Format::Json));
    assert!(loader::Format::all().contains(&loader::Format::Yaml));
}

#[test]
fn test_detect_from_path_cases() {
    use std::path::Path;
    assert!(loader::detect_format_from_path(Path::new("config.toml")).is_some());
    assert!(loader::detect_format_from_path(Path::new("config.json")).is_some());
    assert!(loader::detect_format_from_path(Path::new("config.yaml")).is_some());
    assert_eq!(loader::detect_format_from_path(Path::new("config")), None);
}

#[test]
fn test_detect_from_content_formats() {
    assert_eq!(
        loader::detect_format_from_content("name = \"v\""),
        Some(loader::Format::Toml)
    );
    assert_eq!(
        loader::detect_format_from_content("{\"k\":1}"),
        Some(loader::Format::Json)
    );
    assert_eq!(loader::detect_format_from_content(""), None);
}

// ============ Source Module ============

#[test]
fn test_memory_source_basic() {
    let s = config::MemorySource::new();
    let r = s.collect().unwrap();
    assert!(r.is_map());
}

#[test]
fn test_memory_source_with_initial_values() {
    let mut m = std::collections::HashMap::new();
    m.insert("host".into(), ConfigValue::string("localhost"));
    let s = config::MemorySource::with_values(m);
    let r = s.collect().unwrap();
    assert!(r.is_map());
}

#[test]
fn test_default_source_collect_empty() {
    let s = config::DefaultSource::new();
    assert!(s.collect().is_ok());
}

#[test]
fn test_env_source_default_priority() {
    let s = config::EnvSource::new();
    assert_eq!(s.priority(), 50);
}

#[test]
fn test_env_source_custom_priority() {
    let s = config::EnvSource::with_prefix("APP_").with_priority(90);
    assert_eq!(s.priority(), 90);
}

#[test]
fn test_env_source_name() {
    assert_eq!(config::EnvSource::new().name(), "env");
}

#[test]
fn test_file_source_basic() {
    let s = config::FileSource::new("test.toml");
    assert!(s.file_path().is_some());
    assert_eq!(s.name(), "test.toml");
}

#[test]
fn test_file_source_optional_flag() {
    let s = config::FileSource::new("missing.toml").optional();
    assert!(s.is_optional());
}

#[test]
fn test_memory_source_not_optional() {
    assert!(!config::MemorySource::new().is_optional());
}

#[test]
fn test_source_kind_values() {
    assert_eq!(
        config::MemorySource::new().source_kind(),
        config::SourceKind::Memory
    );
    assert_eq!(
        config::DefaultSource::new().source_kind(),
        config::SourceKind::Default
    );
    assert_eq!(
        config::FileSource::new("x.toml").source_kind(),
        config::SourceKind::File
    );
    assert_eq!(
        config::EnvSource::new().source_kind(),
        config::SourceKind::Environment
    );
}

// ============ Snapshot Module ============

#[cfg(feature = "snapshot")]
#[test]
fn test_snapshot_format_basics() {
    assert_eq!(SnapshotFormat::Toml.ext(), "toml");
    assert_eq!(SnapshotFormat::Json.ext(), "json");
    assert_eq!(SnapshotFormat::Yaml.ext(), "yaml");
    assert_eq!(SnapshotFormat::Toml, SnapshotFormat::Toml);
    assert_ne!(SnapshotFormat::Toml, SnapshotFormat::Json);
}

// ============ Error Module ============

#[test]
fn test_error_codes() {
    assert_eq!(error::ErrorCode::FileNotFound as u16, 1);
    assert_eq!(error::ErrorCode::ValidationFailed as u16, 100);
    assert_eq!(error::ErrorCode::DecryptionFailed as u16, 200);
    assert_eq!(error::ErrorCode::RemoteUnavailable as u16, 300);
    assert_eq!(error::ErrorCode::Timeout as u16, 900);
}

#[test]
fn test_error_code_display_names() {
    assert_eq!(error::ErrorCode::FileNotFound.to_string(), "FILE_NOT_FOUND");
    assert_eq!(error::ErrorCode::Timeout.to_string(), "TIMEOUT");
    assert_eq!(
        error::ErrorCode::ConcurrencyConflict.to_string(),
        "CONCURRENCY_CONFLICT"
    );
}

#[test]
fn test_build_result_methods() {
    use error::BuildResult;
    let r: BuildResult<i32> = BuildResult::ok(42);
    assert!(!r.degraded);
    assert!(!r.has_warnings());
    let mapped = r.map(|v| v * 2);
    assert_eq!(mapped.config, 84);
}

#[test]
fn test_error_is_retryable() {
    let timeout = error::ConfigError::Timeout { duration_ms: 100 };
    assert!(timeout.is_retryable());
    let io_err = error::ConfigError::IoError(std::io::Error::new(
        std::io::ErrorKind::ConnectionRefused,
        "test",
    ));
    assert!(io_err.is_retryable());
}

// ============ Traits Module ============

#[test]
fn test_filter_sensitive_keys_works() {
    let keys = vec!["a".into(), "password".into(), "b".into()];
    assert_eq!(
        interface::filter_sensitive_keys(keys, &["password"]),
        vec!["a", "b"]
    );
}

#[test]
fn test_filter_sensitive_keys_nested() {
    let keys = vec!["db.host".into(), "db.password".into()];
    let r = interface::filter_sensitive_keys(keys, &["db.password"]);
    assert_eq!(r, vec!["db.host"]);
}

#[test]
fn test_filter_sensitive_keys_all_sensitive() {
    let keys = vec!["pwd".into()];
    let r = interface::filter_sensitive_keys(keys, &["pwd"]);
    assert!(r.is_empty());
}

#[cfg(feature = "progressive-reload")]
#[test]
fn test_health_status_variants() {
    assert!(HealthStatus::Healthy.is_healthy());
    assert!(!HealthStatus::Healthy.requires_rollback());
    let d = HealthStatus::Degraded {
        reason: "slow".into(),
    };
    assert!(!d.is_healthy());
    let c = HealthStatus::Critical {
        reason: "err".into(),
    };
    assert!(c.requires_rollback());
}

// ============ ConfigValue Module ============

#[test]
fn test_config_value_creation() {
    assert!(ConfigValue::null().is_null());
    assert!(ConfigValue::bool(true).is_bool());
    assert!(ConfigValue::integer(42).is_integer());
    assert!(ConfigValue::uint(100).as_u64() == Some(100));
    assert!(ConfigValue::float(std::f64::consts::PI).is_number());
    assert!(ConfigValue::string("hello").is_string());
}

#[test]
fn test_config_value_conversions() {
    let v: ConfigValue = true.into();
    assert_eq!(v.as_bool(), Some(true));
    let v: ConfigValue = 42i32.into();
    assert_eq!(v.as_i64(), Some(42));
    let v: ConfigValue = "test".into();
    assert_eq!(v.as_str(), Some("test"));
}

// ============ Interpolation Module ============

#[cfg(feature = "interpolation")]
#[test]
fn test_interpolate_default() {
    use confers::interpolation::interpolate;
    assert_eq!(
        interpolate("${PORT:8080}", &|_| None::<String>).unwrap(),
        "8080"
    );
    assert_eq!(
        interpolate("${HOST:localhost}", &|_| None::<String>).unwrap(),
        "localhost"
    );
}

#[cfg(feature = "interpolation")]
#[test]
fn test_interpolate_no_vars() {
    use confers::interpolation::interpolate;
    assert_eq!(
        interpolate("hello world", &|_| None::<String>).unwrap(),
        "hello world"
    );
    assert_eq!(interpolate("", &|_| None::<String>).unwrap(), "");
}

// ============ Dynamic Field Module ============

#[cfg(feature = "dynamic")]
#[test]
fn test_dynamic_field_default_value() {
    use confers::dynamic::DynamicField;
    let f: DynamicField<u64> = DynamicField::default();
    assert_eq!(f.get(), 0);
}

#[cfg(feature = "dynamic")]
#[test]
fn test_dynamic_field_builder() {
    use confers::dynamic::DynamicField;
    let f = DynamicField::builder().initial(42u32).build();
    assert_eq!(f.get(), 42);
}

#[cfg(feature = "dynamic")]
#[test]
fn test_dynamic_field_update() {
    use confers::dynamic::DynamicField;
    let f = DynamicField::new(10u32);
    f.update(20);
    assert_eq!(f.get(), 20);
}

#[cfg(feature = "dynamic")]
#[test]
fn test_dynamic_field_no_callbacks() {
    use confers::dynamic::DynamicField;
    let f = DynamicField::new(10u32);
    assert_eq!(f.callback_count(), 0);
}

// ============ Context Module ============

// ============ Merger Module ============
#[test]
fn test_merge_strategies() {
    use confers::merger::MergeStrategy;
    assert_eq!(MergeStrategy::default(), MergeStrategy::Replace);
    assert_eq!(MergeStrategy::Append, MergeStrategy::Append);
    assert_ne!(MergeStrategy::Append, MergeStrategy::Prepend);
}

// ============ Validator Module ============

#[cfg(feature = "validation")]
#[test]
fn test_validation_rule_parse() {
    // Can't call Validate::validate directly without derive,
    // but we can verify the module exports are accessible
    let _validate_type: Option<confers::validator::ValidationRule> = None;
}
