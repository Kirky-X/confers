// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! E2E: 校验(tests/e2e/validation_e2e.rs)
//!
//! 场景固化(docs/TEST_SCENARIOS.md §2.3):
//! - VAL-01 `#[derive(Config)] #[config(validate)]` + garde 派生:合法配置加载并通过校验
//! - VAL-02 非法值(范围越界)→ garde Report;经 `ConfigError::validation_error` 转换后
//!   保留字段路径,`user_message` 输出可读文案
//! - VAL-07 多字段同时违规 → 全部字段聚合在一份 Report 中
//!
//! 行为固化说明:`#[config(validate)]` 当前不把校验自动挂进加载管线
//! (见 macros/src/codegen/validate.rs,codegen 为显式 no-op),
//! 校验由使用方以 `garde::Validate::validate` 显式触发 —— 本文件按该真实行为固化。

use confers::validator::Validate;
use confers::{Config, ConfigBuilder, ConfigError};
use garde::Validate as GardeValidate;
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

#[derive(Debug, Config, Deserialize, GardeValidate)]
#[config(validate)]
struct ServerSettings {
    #[garde(length(min = 1, max = 253))]
    host: String,

    #[garde(range(min = 1, max = 10000))]
    port: u16,
}

#[test]
fn val01_valid_config_loads_and_passes_validation() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_temp_file(
        dir.path(),
        "valid.toml",
        "host = \"localhost\"\nport = 8080\n",
    );

    let settings: ServerSettings = ConfigBuilder::new()
        .allow_absolute_paths()
        .file(&file)
        .build()
        .expect("valid config must build");

    settings
        .validate()
        .expect("valid config must pass garde validation");
}

#[test]
fn val01_derive_load_helper_and_validation_roundtrip() {
    // derive 生成的 loader 校验相对路径,因此临时文件落在 cwd 内并以相对路径引用。
    let cwd = std::env::current_dir().unwrap();
    let mut file = tempfile::Builder::new()
        .suffix(".toml")
        .tempfile_in(&cwd)
        .unwrap();
    std::io::Write::write_all(&mut file, b"host = \"example.org\"\nport = 443\n").unwrap();
    let relative = file.path().strip_prefix(&cwd).unwrap().to_path_buf();

    let settings = ServerSettings::load_file_with_env(&relative).expect("derive load must succeed");
    assert_eq!(settings.host, "example.org");
    assert_eq!(settings.port, 443);
    settings.validate().expect("must satisfy garde rules");
}

#[test]
fn val02_out_of_range_value_fails_validation_with_field_path() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_temp_file(dir.path(), "invalid.toml", "host = \"h\"\nport = 65535\n");

    let settings: ServerSettings = ConfigBuilder::new()
        .allow_absolute_paths()
        .file(&file)
        .build()
        .expect("65535 is a valid u16, so the build succeeds (garde is a separate step)");

    let report = settings
        .validate()
        .expect_err("port 65535 violates range(1..=10000)");

    let mut fields = Vec::new();
    for (path, _error) in report.iter() {
        fields.push(path.to_string());
    }
    assert!(
        fields.iter().any(|f| f.contains("port")),
        "report must reference the offending field path, got: {fields:?}"
    );

    // garde Report → ConfigError 转换保留字段信息,user_message 可读。
    let err = ConfigError::validation_error("validation failed", report);
    assert!(
        matches!(err, ConfigError::ValidationFailed { ref field, .. } if field.contains("port")),
        "conversion must keep the field path, got: {err:?}"
    );
    let msg = err.user_message();
    assert!(!msg.is_empty(), "user_message must be readable");
    assert!(msg.contains("port"), "message should mention field: {msg}");
}

#[test]
fn val07_multiple_field_violations_aggregate_into_one_report() {
    let dir = tempfile::tempdir().unwrap();
    // host 超长(>253)与 port 越界(>65535)同时违规。
    let long_host = "h".repeat(300);
    let file = write_temp_file(
        dir.path(),
        "multi_bad.toml",
        &format!("host = \"{long_host}\"\nport = 65535\n"),
    );

    let settings: ServerSettings = ConfigBuilder::new()
        .allow_absolute_paths()
        .file(&file)
        .build()
        .expect("both fields load; violations surface at validation");

    let report = settings
        .validate()
        .expect_err("two violating fields must produce a report");

    let mut bad_fields = 0;
    let mut saw_host = false;
    let mut saw_port = false;
    for (path, _) in report.iter() {
        bad_fields += 1;
        let p = path.to_string();
        if p.contains("host") {
            saw_host = true;
        }
        if p.contains("port") {
            saw_port = true;
        }
    }
    assert!(saw_host, "host violation must be reported");
    assert!(saw_port, "port violation must be reported");
    assert!(
        bad_fields >= 2,
        "both violations aggregate, got {bad_fields}"
    );
}
