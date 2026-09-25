// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! E2E: FMT-22 —— 格式 feature 未启用时的占位错误(cfg stub 分支)。
//!
//! 本文件无 required-features,专供 `--no-default-features` 编译运行:
//! toml/json 关闭时,`load_file`/`parse_content` 走 impl_/loader.rs 的
//! cfg(not(feature)) stub 分支,返回「Add '<format>' feature」占位 ParseError。
//!
//! 行为固化说明:facade `confers::loader` 仅在 feature 开启时重导出
//! `parse_toml/parse_json`(loader.rs 顶部 cfg 门),关闭态的 stub 函数不可从
//! 外部直接调用;可观测入口是 `load_file`/`parse_content`。

// 两个测试分别在 not(toml)/not(json) 下编译；共享项在「至少一个格式关闭」时才编译，
// 避免全特性组合下 unused 告警。
#[cfg(not(all(feature = "toml", feature = "json")))]
use confers::{ConfigError, Format, LoaderConfig, SourceId, load_file, parse_content};

#[cfg(not(all(feature = "toml", feature = "json")))]
fn write_temp_file(dir: &std::path::Path, name: &str, content: &str) -> std::path::PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, content).expect("write temp file");
    path
}

#[cfg(not(all(feature = "toml", feature = "json")))]
fn assert_placeholder(err: ConfigError, format: &str, expected_message: &str) {
    match err {
        ConfigError::ParseError {
            format: f,
            message,
            location,
            source,
        } => {
            assert_eq!(f, format, "format must name the disabled format");
            assert_eq!(message, expected_message, "placeholder message expected");
            assert!(location.is_none(), "stub has no location");
            assert!(source.is_none(), "stub has no source error");
        }
        other => panic!("expected placeholder ParseError({format}), got: {other:?}"),
    }
}

#[cfg(not(feature = "toml"))]
#[test]
fn s22_toml_disabled_reports_placeholder_error() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_temp_file(dir.path(), "x.toml", "a = 1\n");

    let err = load_file(&path, &LoaderConfig::new().allow_absolute())
        .expect_err("toml feature disabled: load must return the placeholder error");
    assert_placeholder(err, "TOML", "Add 'toml' feature");

    let err2 = parse_content("a = 1\n", Format::Toml, SourceId::new("x.toml"), None)
        .expect_err("parse_content must hit the same stub");
    assert!(
        err2.to_string().contains("Add 'toml' feature"),
        "got: {}",
        err2
    );
    println!("s22 evidence(toml off): placeholder ParseError, message=\"Add 'toml' feature\"");
}

#[cfg(not(feature = "json"))]
#[test]
fn s22_json_disabled_reports_placeholder_error() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_temp_file(dir.path(), "x.json", "{\"a\": 1}\n");

    let err = load_file(&path, &LoaderConfig::new().allow_absolute())
        .expect_err("json feature disabled: load must return the placeholder error");
    assert_placeholder(err, "JSON", "Add 'json' feature");

    let err2 = parse_content("{\"a\": 1}\n", Format::Json, SourceId::new("x.json"), None)
        .expect_err("parse_content must hit the same stub");
    assert!(
        err2.to_string().contains("Add 'json' feature"),
        "got: {}",
        err2
    );
    println!("s22 evidence(json off): placeholder ParseError, message=\"Add 'json' feature\"");
}
