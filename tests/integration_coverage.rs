use confers::config;
use confers::config::Source;
use confers::error;
use confers::loader;
use confers::snapshot::SnapshotFormat;
use confers::traits;
use confers::traits::HealthStatus;

#[test]
fn test_format_try_parse() {
    assert_eq!(
        loader::Format::try_parse("toml"),
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
    assert_eq!(loader::Format::try_parse("unknown"), None);
}

#[test]
fn test_format_all() {
    let all = loader::Format::all();
    assert!(all.contains(&loader::Format::Toml));
}

#[test]
fn test_detect_format_from_path() {
    use std::path::Path;
    assert_eq!(
        loader::detect_format_from_path(Path::new("config.TOML")),
        Some(loader::Format::Toml)
    );
    assert_eq!(
        loader::detect_format_from_path(Path::new("config.json")),
        Some(loader::Format::Json)
    );
    assert_eq!(loader::detect_format_from_path(Path::new("config")), None);
}

#[test]
fn test_memory_source_with_values() {
    let mut values = std::collections::HashMap::new();
    values.insert("key".to_string(), confers::ConfigValue::string("val"));
    let source = config::MemorySource::with_values(values);
    assert!(source.collect().is_ok());
}

#[test]
fn test_file_source_optional_flag() {
    let source = config::FileSource::new("missing.toml").optional();
    assert!(source.is_optional());
}

#[test]
fn test_snapshot_format_basics() {
    assert_eq!(SnapshotFormat::Toml.ext(), "toml");
    assert_eq!(SnapshotFormat::Json.ext(), "json");
    assert_eq!(SnapshotFormat::Yaml.ext(), "yaml");
}

#[test]
fn test_filter_sensitive_keys() {
    let keys = vec!["host".into(), "password".into()];
    let result = traits::filter_sensitive_keys(keys, &["password"]);
    assert_eq!(result, vec!["host"]);
}

#[test]
fn test_health_status() {
    assert!(HealthStatus::Healthy.is_healthy());
    assert!(!HealthStatus::Healthy.requires_rollback());
    assert!(HealthStatus::Critical { reason: "x".into() }.requires_rollback());
}

#[test]
fn test_error_code_values() {
    assert_eq!(error::ErrorCode::FileNotFound as u16, 1);
    assert_eq!(error::ErrorCode::ValidationFailed as u16, 100);
    assert_eq!(error::ErrorCode::Timeout as u16, 900);
}

#[test]
fn test_interpolate_default() {
    let r = confers::interpolation::interpolate("${X:def}", &|_| None::<String>).unwrap();
    assert_eq!(r, "def");
}

#[test]
fn test_dynamic_field_default() {
    use confers::dynamic::DynamicField;
    let f: DynamicField<u64> = DynamicField::default();
    assert_eq!(f.get(), 0);
}

#[test]
fn test_loader_config_default() {
    use confers::loader::LoaderConfig;
    let cfg = LoaderConfig::new();
    assert!(!cfg.allow_absolute);
}

#[test]
fn test_loader_detect_from_content_json() {
    let r = loader::detect_format_from_content("{\"a\":1}");
    assert_eq!(r, Some(loader::Format::Json));
}

#[test]
fn test_error_code_display() {
    assert_eq!(error::ErrorCode::FileNotFound.to_string(), "FILE_NOT_FOUND");
    assert_eq!(error::ErrorCode::Timeout.to_string(), "TIMEOUT");
}

#[test]
fn test_config_value_string_creation() {
    let v = confers::ConfigValue::string("test");
    assert!(v.is_string());
    assert_eq!(v.as_str(), Some("test"));
}

#[test]
fn test_config_value_type_checks() {
    let b = confers::ConfigValue::bool(true);
    assert!(b.is_bool());
    let n = confers::ConfigValue::null();
    assert!(n.is_null());
    let i = confers::ConfigValue::integer(42);
    assert!(i.is_integer());
}
