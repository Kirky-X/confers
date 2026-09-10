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

use serial_test::serial;
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
#[serial]
fn mac03_name_and_name_env_override_key_mapping() {
    // 基线:文件键仍是 serde 字段名,name 属性不改变文件键。
    let (_file, path) = write_cwd_toml("host = \"file-host\"\nport = 8080\n");
    let cfg = NamedFields::load_file_with_env(&path).expect("load via field-name keys");
    assert_eq!(cfg.host, "file-host");
    assert_eq!(cfg.port, 8080);

    // name_env 声明的 env 名必须与声明一致并真实覆盖 host 字段。
    let (_file, path) = write_cwd_toml("host = \"file-host\"\nport = 8080\n");
    unsafe { std::env::set_var("NAMED_FIELDS_CUSTOM_HOST", "custom-env-host") };
    let cfg = NamedFields::load_file_with_env(&path).expect("load with custom name_env");
    unsafe { std::env::remove_var("NAMED_FIELDS_CUSTOM_HOST") };
    assert_eq!(
        cfg.host, "custom-env-host",
        "name_env override must reach the field"
    );

    // 互不污染:host 声明了 name_env 后,name 派生命名(BIND_HOST)不再生效。
    let (_file, path) = write_cwd_toml("host = \"file-host\"\nport = 8080\n");
    unsafe { std::env::set_var("BIND_HOST", "leaked-host") };
    let cfg = NamedFields::load_file_with_env(&path).expect("load with derived name env");
    unsafe { std::env::remove_var("BIND_HOST") };
    assert_eq!(
        cfg.host, "file-host",
        "fields declaring name_env must not read the name-derived env key"
    );
}

/// MAC-03:仅声明 `name`(无 name_env)时,name 派生的 env 名必须真实覆盖字段。
#[derive(Debug, confers::Config, serde::Deserialize)]
struct NameOnlyFields {
    #[config(name = "bind_host")]
    pub host: String,

    #[config(name = "bind_port")]
    pub port: u16,
}

#[test]
#[serial]
fn mac03_name_derived_env_override_reaches_field() {
    let (_file, path) = write_cwd_toml("host = \"file-host\"\nport = 8080\n");
    unsafe { std::env::set_var("BIND_HOST", "env-host") };
    unsafe { std::env::set_var("BIND_PORT", "9090") };
    let cfg = NameOnlyFields::load_file_with_env(&path).expect("load with derived name envs");
    unsafe { std::env::remove_var("BIND_HOST") };
    unsafe { std::env::remove_var("BIND_PORT") };
    assert_eq!(
        cfg.host, "env-host",
        "name-derived env override must reach the field"
    );
    assert_eq!(cfg.port, 9090, "name-derived env override applies to all fields");
}

/// MAC-03(互不污染):声明 name_env 后,env 键与默认命名规则互不串扰。
#[derive(Debug, confers::Config, serde::Deserialize)]
struct EnvNameIsolation {
    #[config(name_env = "ISOLATION_CUSTOM")]
    pub custom: String,

    pub plain: String,
}

#[test]
#[serial]
fn mac03_name_env_does_not_pollute_default_naming() {
    // 声明了 name_env 的字段只读取声明键:默认派生命名(CUSTOM)不再生效。
    let (_file, path) = write_cwd_toml("custom = \"file-custom\"\nplain = \"file-plain\"\n");
    unsafe { std::env::set_var("CUSTOM", "leaked") };
    unsafe { std::env::set_var("ISOLATION_CUSTOM", "declared-env") };
    let cfg = EnvNameIsolation::load_file_with_env(&path).expect("load isolation struct");
    unsafe { std::env::remove_var("CUSTOM") };
    unsafe { std::env::remove_var("ISOLATION_CUSTOM") };
    assert_eq!(
        cfg.custom, "declared-env",
        "name_env-declared key must drive the override"
    );
    assert_eq!(
        cfg.plain, "file-plain",
        "custom env key must not leak into the plain field"
    );

    // 反向:默认命名规则字段只读取自身派生键,不读取他人声明的 name_env。
    let (_file, path) = write_cwd_toml("custom = \"file-custom\"\nplain = \"file-plain\"\n");
    unsafe { std::env::set_var("PLAIN", "plain-env") };
    let cfg = EnvNameIsolation::load_file_with_env(&path).expect("load isolation struct 2");
    unsafe { std::env::remove_var("PLAIN") };
    assert_eq!(
        cfg.custom, "file-custom",
        "plain field's derived env must not touch the name_env field"
    );
    assert_eq!(cfg.plain, "plain-env");
}

/// MAC-07:skip 字段行为。
#[derive(Debug, confers::Config, serde::Deserialize)]
struct SkippedField {
    pub visible: String,

    #[config(skip, default = "default-value".to_string())]
    pub hidden: String,
}

#[test]
#[serial]
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

    // skip 字段不参与加载:通用 env 源不得把 HIDDEN 注入 skipped 字段。
    unsafe { std::env::set_var("HIDDEN", "from-env") };
    let cfg = SkippedOptional::load_file_with_env(&path).expect("load with env var present");
    unsafe { std::env::remove_var("HIDDEN") };
    assert_eq!(
        cfg.hidden, None,
        "skip must keep the field out of the load pipeline (env bypass fixed)"
    );
}

/// MAC-07:skip 字段从任何来源(env/文件)取得的值都不得覆盖其 default。
#[test]
#[serial]
fn mac07_skip_default_not_overridden_by_env_or_file() {
    // env 同名值不得覆盖 skip 字段 default。
    let (_file, path) = write_cwd_toml("visible = \"from-file\"\n");
    unsafe { std::env::set_var("HIDDEN", "from-env") };
    let cfg = SkippedField::load_file_with_env(&path).expect("load with skipped + default");
    unsafe { std::env::remove_var("HIDDEN") };
    assert_eq!(cfg.visible, "from-file");
    assert_eq!(
        cfg.hidden, "default-value",
        "skip field must materialize its default, not the env value"
    );

    // 文件同名值同样不得覆盖 skip 字段 default。
    let (_file, path) = write_cwd_toml("visible = \"from-file\"\nhidden = \"from-file\"\n");
    let cfg = SkippedField::load_file_with_env(&path).expect("load with file key for skip field");
    assert_eq!(
        cfg.hidden, "default-value",
        "skip field default must not be overridden by the file source"
    );
}

/// MAC-07:skip + default 组合在键缺失时加载成功(default 物化)。
#[test]
#[serial]
fn mac07_skip_materializes_value_instead_of_failing() {
    let (_file, path) = write_cwd_toml("visible = \"from-file\"\n");
    let cfg = SkippedField::load_file_with_env(&path)
        .expect("skip + default must materialize the default instead of failing");
    assert_eq!(cfg.hidden, "default-value");

    // load_file(无 env)同样物化 skip default。
    let (_file, path) = write_cwd_toml("visible = \"from-file\"\n");
    let cfg = SkippedField::load_file(&path).expect("load_file also materializes skip default");
    assert_eq!(cfg.hidden, "default-value");
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
