// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! E2E: 派生宏与属性(tests/e2e/macro_e2e.rs)
//!
//! 场景固化(docs/TEST_SCENARIOS.md §2.25):
//! - MAC-03 `name` / `name_env` 字段属性覆盖默认键名/env 名(端到端加载)
//! - MAC-07 `skip = true` 字段不参与 env 加载(端到端)
//! - MAC-09/MAC-17/ENC-23 的宏展开期错误(非法 merge_strategy / env_prefix /
//!   encrypt 算法)固化于 macros crate 的 trybuild 用例(macros/tests/compile_fail.rs)
//!
//! 行为固化说明(见报告):`flatten` / `interpolate` / `dynamic`(结构体与字段级)/
//! `watch` 属性由宏解析但 codegen 未消费 —— MAC-06/08/11/12/18 的"集成级"预期
//! 无法成立,本文件以 flatten+dynamic 组合加载用例固化该真实边界。
//!
//! MAC-01/02/04/05/10/13…16 已有覆盖(tests/core/derive.rs、tests/core/env_types.rs、src 内联)。

use std::io::Write;
use std::path::PathBuf;

/// 在 cwd 内创建唯一临时 toml(derive loader 校验相对路径)。
/// 返回 NamedTempFile 以保住文件生命周期,以及 derive loader 用的相对路径。
fn write_cwd_toml(content: &str) -> (tempfile::NamedTempFile, PathBuf) {
    let cwd = std::env::current_dir().unwrap();
    let mut file = tempfile::Builder::new()
        .suffix(".toml")
        .tempfile_in(&cwd)
        .unwrap();
    file.write_all(content.as_bytes()).unwrap();
    let relative = file.path().strip_prefix(&cwd).unwrap().to_path_buf();
    (file, relative)
}

/// MAC-03:自定义键名 + 自定义 env 名。
#[derive(Debug, confers::Config, serde::Deserialize)]
struct NamedFields {
    #[config(name = "bind_host", name_env = "NAMED_FIELDS_CUSTOM_HOST")]
    pub host: String,

    #[config(name = "bind_port")]
    pub port: u16,
}

#[test]
fn mac03_name_and_name_env_override_key_mapping() {
    // 行为固化:name 属性映射 env 名与默认键,文件键仍是 serde 字段名。
    let (_file, path) = write_cwd_toml("host = \"file-host\"\nport = 8080\n");
    let cfg = NamedFields::load_file_with_env(&path).expect("load via field-name keys");
    assert_eq!(cfg.host, "file-host");
    assert_eq!(cfg.port, 8080);

    // 行为固化(已知缺陷,见报告):name 派生的 env 名写入重命名后的键
    // ("bind_host"),serde 仍按字段名 "host" 反序列化 → env 覆盖不生效。
    let (_file, path) = write_cwd_toml("host = \"file-host\"\nport = 8080\n");
    unsafe { std::env::set_var("BIND_HOST", "env-host") };
    let cfg = NamedFields::load_file_with_env(&path).expect("load with default name env");
    unsafe { std::env::remove_var("BIND_HOST") };
    assert_eq!(
        cfg.host, "file-host",
        "name-derived env override lands under a key serde ignores (fixated defect)"
    );

    // name_env 同理:自定义 env 名的覆盖同样不生效。
    let (_file, path) = write_cwd_toml("host = \"file-host\"\nport = 8080\n");
    unsafe { std::env::set_var("NAMED_FIELDS_CUSTOM_HOST", "custom-env-host") };
    let cfg = NamedFields::load_file_with_env(&path).expect("load with custom name_env");
    unsafe { std::env::remove_var("NAMED_FIELDS_CUSTOM_HOST") };
    assert_eq!(
        cfg.host, "file-host",
        "name_env override ineffective for the same reason (fixated defect)"
    );
}

/// MAC-07:skip 字段行为固化。
#[derive(Debug, confers::Config, serde::Deserialize)]
#[allow(dead_code)] // 仅断言加载失败路径,字段值本身不被读取。
struct SkippedField {
    pub visible: String,

    #[config(skip, default = "default-value".to_string())]
    pub hidden: String,
}

#[test]
fn mac07_skipped_field_behavior() {
    // skip + Option 字段:源未提供键 → None(serde 可选语义)。
    #[derive(Debug, confers::Config, serde::Deserialize)]
    struct SkippedOptional {
        pub visible: String,

        #[config(skip)]
        pub hidden: Option<String>,
    }
    let (_file, path) = write_cwd_toml("visible = \"from-file\"\n");
    let cfg = SkippedOptional::load_file_with_env(&path).expect("load with skipped Option field");
    assert_eq!(cfg.visible, "from-file");
    assert!(cfg.hidden.is_none(), "skipped field stays unset");

    // 行为固化(已知缺陷,见报告):通用 env 源仍会按 HIDDEN → hidden 注入,
    // skip 无法把字段挡在加载管线之外。
    unsafe { std::env::set_var("HIDDEN", "from-env") };
    let cfg = SkippedOptional::load_file_with_env(&path).expect("load with env var present");
    unsafe { std::env::remove_var("HIDDEN") };
    assert_eq!(
        cfg.hidden.as_deref(),
        Some("from-env"),
        "generic env source bypasses skip (fixated defect)"
    );
}

/// 行为固化(已知缺陷):`skip` + `default` 组合 —— default 属性对 skip 字段
/// 不生效(默认值生成同样过滤了 skip 字段)→ 缺键时加载直接失败。
#[test]
fn mac07_skip_with_default_attr_still_fails_when_key_missing() {
    let (_file, path) = write_cwd_toml("visible = \"from-file\"\n");
    let err = SkippedField::load_file_with_env(&path)
        .expect_err("skip + default does not materialize the default");
    assert!(
        err.to_string().contains("missing field `hidden`"),
        "unexpected error: {err}"
    );
}

/// 行为固化:flatten / dynamic / interpolate / watch 属性被解析接受,
/// 配置照常加载(无额外语义 —— 见报告)。
#[derive(Debug, confers::Config, serde::Deserialize)]
#[config(watch = true)]
struct ParsedOnlyAttrs {
    #[config(flatten = true, dynamic = true, interpolate = true, name = "plain_key")]
    pub value: String,
}

#[test]
fn mac06_macro08_macro12_macro18_parsed_only_attrs_do_not_break_loading() {
    let (_file, path) = write_cwd_toml("value = \"loaded\"\n");
    let cfg = ParsedOnlyAttrs::load_file_with_env(&path)
        .expect("parse-only attrs must not affect loading");
    assert_eq!(cfg.value, "loaded");
}
