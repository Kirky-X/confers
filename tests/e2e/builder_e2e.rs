// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! E2E: 配置构建 / Source 链 / 合并(tests/e2e/builder_e2e.rs)
//!
//! 场景固化(docs/TEST_SCENARIOS.md §2.2):
//! - BLD-10 `.memory_priority(n)` 调整内存源优先级:默认高于文件源,显式调低后被更高优先级文件源覆盖
//! - BLD-14 `ConfigLimits`/`LoaderConfig` 大小上限 → `SizeLimitExceeded`(链路级)
//! - BLD-15 嵌套深度超 `max_nesting_depth` → 明确错误
//! - BLD-16 键总数超 `max_total_fields` → 明确错误
//! - BLD-17 数组长度超 `max_array_length` → 明确错误
//! - BLD-18 字符串超 `max_string_length` → 明确错误
//! - BLD-19 必填字段缺失 → 构建错误(serde "missing field",非 panic)
//! - BLD-20 类型不匹配(port="abc" 对 u16 等)→ 类型错误而非 panic
//! - BLD-21 `build_resilient()`:部分源损坏不中断;全部源损坏 → `Degraded` 携带原因
//! - BLD-22 `build_with_fallback(fallback)`:全部源失败时回退默认实例并携带 `RemoteFallback` warning

use confers::{ConfigBuilder, ConfigLimits, ConfigValue, FileSource, SourceChainBuilder};
use serde::Deserialize;
use std::io::Write;
use std::path::{Path, PathBuf};

fn write_temp_file(dir: &Path, name: &str, content: &str) -> PathBuf {
    let path = dir.join(name);
    let mut file = std::fs::File::create(&path).expect("create temp file");
    file.write_all(content.as_bytes()).expect("write temp file");
    file.flush().expect("flush temp file");
    path
}

#[derive(Debug, Deserialize, Default, PartialEq)]
struct AppConfig {
    name: String,
    port: u16,
}

#[test]
fn bld10_memory_source_wins_over_file_by_default_priority() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_temp_file(dir.path(), "app.toml", "name = \"from-file\"\nport = 80\n");

    let mut memory = std::collections::HashMap::new();
    memory.insert("name".to_string(), ConfigValue::string("from-memory"));
    memory.insert("port".to_string(), ConfigValue::integer(8080));

    let config: AppConfig = ConfigBuilder::new()
        .allow_absolute_paths()
        .file(&file)
        .memory(memory)
        .build()
        .expect("build with default memory priority(50) > file priority(0)");

    assert_eq!(config.name, "from-memory");
}

#[test]
fn bld10_lowered_memory_priority_lets_higher_priority_file_win() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_temp_file(dir.path(), "app.toml", "name = \"from-file\"\n");

    // 链路级:文件源优先级 90,内存源显式降到 0 → 文件源覆盖内存源。
    let mut memory = std::collections::HashMap::new();
    memory.insert("name".to_string(), ConfigValue::string("from-memory"));

    let merged = SourceChainBuilder::new()
        .source(Box::new(
            FileSource::new(&file)
                .allow_absolute_paths()
                .with_priority(90),
        ))
        .memory_with_priority(memory, 0)
        .build()
        .collect()
        .expect("chain collect");

    assert_eq!(merged.to_json()["name"], "from-file");

    // 反向:内存源优先级升到 99 → 内存源覆盖文件源。
    let mut memory = std::collections::HashMap::new();
    memory.insert("name".to_string(), ConfigValue::string("from-memory"));

    let merged = SourceChainBuilder::new()
        .source(Box::new(
            FileSource::new(&file)
                .allow_absolute_paths()
                .with_priority(90),
        ))
        .memory_with_priority(memory, 99)
        .build()
        .collect()
        .expect("chain collect");

    assert_eq!(merged.to_json()["name"], "from-memory");
}

#[test]
fn bld14_file_size_limit_rejected_at_chain_level() {
    let dir = tempfile::tempdir().unwrap();
    let content = format!("a = {}", "1".repeat(200));
    let file = write_temp_file(dir.path(), "big.toml", &content);

    let source = FileSource::new(&file)
        .with_loader_config(confers::LoaderConfig::new().allow_absolute().max_size(16));
    let err = SourceChainBuilder::new()
        .source(Box::new(source))
        .build()
        .collect()
        .expect_err("oversized file must be rejected");

    assert!(
        matches!(err, confers::ConfigError::SizeLimitExceeded { .. }),
        "expected SizeLimitExceeded, got: {err:?}"
    );
}

#[test]
fn bld15_nesting_depth_exceeded_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_temp_file(dir.path(), "deep.toml", "a = { b = { c = { d = 1 } } }\n");

    let limits = ConfigLimits::default().with_max_nesting_depth(2);
    let err: confers::ConfigError = ConfigBuilder::<serde_json::Value>::new()
        .allow_absolute_paths()
        .limits(limits)
        .file(&file)
        .build()
        .expect_err("nesting deeper than max_nesting_depth must be rejected");

    let msg = err.to_string();
    assert!(msg.contains("nesting depth"), "unexpected error: {msg}");
    assert_eq!(err.code(), confers::ErrorCode::InvalidValue);
}

#[test]
fn bld15_config_within_nesting_depth_builds() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_temp_file(dir.path(), "shallow.toml", "a = { b = \"ok\" }\n");

    let limits = ConfigLimits::default().with_max_nesting_depth(2);
    let config: serde_json::Value = ConfigBuilder::new()
        .allow_absolute_paths()
        .limits(limits)
        .file(&file)
        .build()
        .expect("nesting within limit must build");

    assert_eq!(config["a"]["b"], "ok");
}

#[test]
fn bld16_total_fields_exceeded_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_temp_file(dir.path(), "wide.toml", "k1 = 1\nk2 = 2\nk3 = 3\nk4 = 4\n");

    let limits = ConfigLimits::default().with_max_total_fields(2);
    let err: confers::ConfigError = ConfigBuilder::<serde_json::Value>::new()
        .allow_absolute_paths()
        .limits(limits)
        .file(&file)
        .build()
        .expect_err("more keys than max_total_fields must be rejected");

    assert!(
        err.to_string().contains("total field count"),
        "unexpected error: {err}"
    );
}

#[test]
fn bld17_array_length_exceeded_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_temp_file(dir.path(), "arr.toml", "items = [1, 2, 3]\n");

    let limits = ConfigLimits::default().with_max_array_length(2);
    let err: confers::ConfigError = ConfigBuilder::<serde_json::Value>::new()
        .allow_absolute_paths()
        .limits(limits)
        .file(&file)
        .build()
        .expect_err("array longer than max_array_length must be rejected");

    assert!(
        err.to_string().contains("array length"),
        "unexpected error: {err}"
    );
}

#[test]
fn bld18_string_length_exceeded_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let long = "x".repeat(64);
    let file = write_temp_file(dir.path(), "str.toml", &format!("name = \"{long}\"\n"));

    let limits = ConfigLimits::default().with_max_string_length(8);
    let err: confers::ConfigError = ConfigBuilder::<serde_json::Value>::new()
        .allow_absolute_paths()
        .limits(limits)
        .file(&file)
        .build()
        .expect_err("string longer than max_string_length must be rejected");

    assert!(
        err.to_string().contains("string length"),
        "unexpected error: {err}"
    );
}

#[test]
fn bld19_missing_required_field_reports_missing_field_not_panic() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_temp_file(dir.path(), "partial.toml", "other = \"value\"\n");

    let err: confers::ConfigError = ConfigBuilder::<AppConfig>::new()
        .allow_absolute_paths()
        .file(&file)
        .build()
        .expect_err("missing required field must fail the build");

    let msg = err.to_string();
    assert!(
        msg.contains("missing field `name`"),
        "expected serde missing-field error, got: {msg}"
    );
}

#[test]
fn bld20_type_mismatch_string_to_u16_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_temp_file(dir.path(), "mismatch.toml", "port = \"abc\"\n");

    let err: confers::ConfigError = ConfigBuilder::<AppConfig>::new()
        .allow_absolute_paths()
        .file(&file)
        .build()
        .expect_err("\"abc\" is not a u16");

    assert!(
        matches!(err, confers::ConfigError::InvalidValue { .. }),
        "expected InvalidValue, got: {err:?}"
    );
}

#[test]
fn bld20_type_mismatch_integer_to_string_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_temp_file(dir.path(), "int_name.toml", "name = 12345\nport = 1\n");

    let err: confers::ConfigError = ConfigBuilder::<AppConfig>::new()
        .allow_absolute_paths()
        .file(&file)
        .build()
        .expect_err("integer is not a String");

    assert!(
        matches!(err, confers::ConfigError::InvalidValue { .. }),
        "expected InvalidValue, got: {err:?}"
    );
}

#[test]
fn bld20_type_mismatch_object_to_primitive_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_temp_file(dir.path(), "obj_port.toml", "port = { a = 1 }\n");

    let err: confers::ConfigError = ConfigBuilder::<AppConfig>::new()
        .allow_absolute_paths()
        .file(&file)
        .build()
        .expect_err("object is not a u16");

    assert!(
        matches!(err, confers::ConfigError::InvalidValue { .. }),
        "expected InvalidValue, got: {err:?}"
    );
}

/// BLD-21:单源损坏时 build_resilient 不中断构建(其余源照常合并,无降级)。
#[test]
fn bld21_resilient_build_survives_partial_source_failure() {
    let dir = tempfile::tempdir().unwrap();
    let corrupt = write_temp_file(dir.path(), "corrupt.toml", "invalid toml {{{\n");

    let result = ConfigBuilder::<AppConfig>::new()
        .allow_absolute_paths()
        .default("name", ConfigValue::string("fallback-name"))
        .default("port", ConfigValue::integer(8080))
        .file(&corrupt)
        .build_resilient()
        .expect("resilient build must not fail on a single corrupt source");

    assert!(!result.degraded, "partial failure must not degrade");
    assert!(result.warnings.is_empty());
    assert_eq!(result.config.name, "fallback-name");
    assert_eq!(result.config.port, 8080);
}

/// BLD-21:全部源损坏 → Degraded,配置回落到类型默认值并携带原因。
#[test]
fn bld21_resilient_build_degrades_when_all_sources_fail() {
    let dir = tempfile::tempdir().unwrap();
    let corrupt = write_temp_file(dir.path(), "corrupt.toml", "invalid toml {{{\n");

    let result = ConfigBuilder::<AppConfig>::new()
        .allow_absolute_paths()
        .file(&corrupt)
        .build_resilient()
        .expect("resilient build returns a degraded result instead of failing");

    assert!(result.degraded, "total source failure must degrade");
    assert!(
        result.degraded_reason.is_some(),
        "degraded result must carry a reason"
    );
    assert_eq!(result.config, AppConfig::default());
}

/// BLD-22:全部源失败时 build_with_fallback 回退到给定实例并携带 RemoteFallback warning。
#[test]
fn bld22_build_with_fallback_returns_fallback_on_total_failure() {
    let dir = tempfile::tempdir().unwrap();
    let corrupt = write_temp_file(dir.path(), "corrupt.toml", "invalid toml {{{\n");

    let fallback = AppConfig {
        name: "fallback".to_string(),
        port: 9999,
    };

    let result = ConfigBuilder::new()
        .allow_absolute_paths()
        .file(&corrupt)
        .build_with_fallback(fallback);

    assert!(result.degraded);
    assert_eq!(result.config.name, "fallback");
    assert_eq!(result.config.port, 9999);
    assert_eq!(
        result.degraded_reason.as_deref().map(|r| !r.is_empty()),
        Some(true)
    );
    assert!(
        result
            .warnings
            .iter()
            .any(|w| matches!(w.code, confers::error::WarningCode::RemoteFallback)),
        "fallback usage must surface a RemoteFallback warning: {:?}",
        result.warnings
    );
}

/// BLD-22(正向):构建成功时不产生降级与警告。
#[test]
fn bld22_build_with_fallback_ok_when_build_succeeds() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_temp_file(dir.path(), "ok.toml", "name = \"real\"\nport = 80\n");

    let fallback = AppConfig {
        name: "unused".to_string(),
        port: 0,
    };

    let result = ConfigBuilder::new()
        .allow_absolute_paths()
        .file(&file)
        .build_with_fallback(fallback);

    assert!(!result.degraded);
    assert!(result.warnings.is_empty());
    assert_eq!(result.config.name, "real");
    assert_eq!(result.config.port, 80);
}
