// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! E2E: 派生宏与属性(tests/e2e/macro_e2e.rs)
//!
//! 场景固化(docs/TEST_SCENARIOS.md §2.25):
//! - MAC-03 `name` / `name_env` 字段属性覆盖默认键名/env 名(端到端加载)
//! - MAC-07 `skip = true` 字段不参与 env 加载(端到端)
//! - ENC-23 的宏展开期错误(非法 merge_strategy / env_prefix /
//!   encrypt 算法)固化于 macros crate 的 trybuild 用例(macros/tests/compile_fail.rs)
//!
//! 行为固化说明(见报告):`flatten` / `interpolate` / `dynamic` /
//! `watch` 属性曾在 rc.3 前由宏解析但 codegen 未消费 —— rc.4 起四属性
//! codegen 真实生效,08/11/12/18 以集成级语义断言固化于本文件。
//!
//! 02/04/05/10/13…16 已有覆盖(tests/core/derive.rs、tests/core/env_types.rs、src 内联)。

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

/// 自定义键名 + 自定义 env 名。
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

/// 仅声明 `name`(无 name_env)时,name 派生的 env 名必须真实覆盖字段。
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
    assert_eq!(
        cfg.port, 9090,
        "name-derived env override applies to all fields"
    );
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

/// skip 字段行为。
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

/// skip 字段从任何来源(env/文件)取得的值都不得覆盖其 default。
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

/// skip + default 组合在键缺失时加载成功(default 物化)。
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

/// `flatten` 字段并入父命名空间 —— 顶层键提升进嵌套结构。
#[derive(Debug, confers::Config, serde::Deserialize)]
struct FlattenDatabase {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, confers::Config, serde::Deserialize)]
struct FlattenParent {
    pub app_name: String,

    #[config(flatten)]
    pub database: FlattenDatabase,
}

#[test]
#[serial]
fn mac06_flatten_hoists_top_level_keys_into_nested_struct() {
    // 顶层直接写 database 的字段(flat 风格),flatten codegen 负责归位。
    let (_file, path) = write_cwd_toml("app_name = \"orders\"\nhost = \"flat-db\"\nport = 6543\n");
    let cfg = FlattenParent::load_file_with_env(&path).expect("flatten load");
    assert_eq!(cfg.app_name, "orders");
    assert_eq!(
        cfg.database.host, "flat-db",
        "top-level key hoisted into flatten field"
    );
    assert_eq!(cfg.database.port, 6543);

    // 嵌套写法(database.host)依旧原生支持,且显式嵌套值优先于顶层同名键。
    let (_file, path) = write_cwd_toml(
        "app_name = \"orders\"\nhost = \"ignored\"\ndatabase = { host = \"nested-db\", port = 1 }\n",
    );
    let cfg = FlattenParent::load_file_with_env(&path).expect("nested-form flatten load");
    assert_eq!(cfg.database.host, "nested-db", "explicit nested value wins");
}

/// `dynamic` 字段生成 DynamicField 句柄(加载值作为初值)。
#[derive(Debug, confers::Config, serde::Deserialize)]
struct DynamicFieldStruct {
    #[config(dynamic)]
    pub replicas: u32,
}

#[test]
fn mac08_dynamic_field_generates_runtime_handle() {
    let (_file, path) = write_cwd_toml("replicas = 3\n");
    let cfg = DynamicFieldStruct::load_file_with_env(&path).expect("dynamic load");
    let handle = cfg.replicas_handle();
    assert_eq!(handle.get(), 3, "handle seeded with the loaded value");

    // 运行时推送新值,读取侧立即可见(无需重载整个结构体)。
    handle.update(5);
    assert_eq!(handle.get(), 5);
}

/// `interpolate` 字段值模板 ${key} 在加载后按合并树解析。
#[derive(Debug, confers::Config, serde::Deserialize)]
struct InterpolatedStruct {
    #[allow(dead_code)]
    pub host: String,

    #[config(interpolate)]
    pub url: String,
}

#[test]
#[serial]
fn mac18_interpolate_resolves_field_template_against_merged_tree() {
    let (_file, path) = write_cwd_toml("host = \"db.internal\"\nurl = \"http://${host}:8080\"\n");
    let cfg = InterpolatedStruct::load_file_with_env(&path).expect("interpolate load");
    assert_eq!(
        cfg.url, "http://db.internal:8080",
        "${{host}} resolved from merged tree"
    );

    // 不可解析引用 + 默认值回退。
    let (_file, path) = write_cwd_toml("host = \"db.internal\"\nurl = \"${missing:fallback}\"\n");
    let cfg = InterpolatedStruct::load_file_with_env(&path).expect("interpolate default load");
    assert_eq!(cfg.url, "fallback", ":default applies when key missing");
}

/// `watch` 字段生成字段级热重载订阅器(与 watch feature 联动)。
#[derive(Debug, Clone, confers::Config, serde::Deserialize)]
struct WatchedStruct {
    #[config(watch)]
    pub host: String,

    pub port: u16,
}

#[tokio::test]
async fn mac12_watch_generates_field_level_hot_reload_subscription() {
    use std::sync::Arc;
    use std::time::Duration;

    let (tx, rx) = tokio::sync::watch::channel(Arc::new(WatchedStruct {
        host: "a".to_string(),
        port: 1,
    }));
    let mut watcher = WatchedStruct {
        host: "a".to_string(),
        port: 1,
    }
    .field_watcher(rx);

    // port 未被订阅:只推 port 变化不触发。
    tx.send(Arc::new(WatchedStruct {
        host: "a".to_string(),
        port: 2,
    }))
    .unwrap();
    // host 被订阅:host 变化后 changed() 返回且只报告 host。
    tx.send(Arc::new(WatchedStruct {
        host: "b".to_string(),
        port: 2,
    }))
    .unwrap();
    let (snapshot, changed) = tokio::time::timeout(Duration::from_millis(500), watcher.changed())
        .await
        .expect("watcher must wake on watched-field change")
        .expect("channel open");
    assert_eq!(
        changed,
        vec![std::sync::Arc::<str>::from("host")],
        "only the watched field is reported"
    );
    assert_eq!(snapshot.host, "b");
    assert_eq!(snapshot.port, 2);
}

/// 四属性组合使用不冲突(08/12/18 交叠)。
#[derive(Debug, Clone, confers::Config, serde::Deserialize)]
struct ComboNested {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, confers::Config, serde::Deserialize)]
struct ComboAttrs {
    #[config(flatten)]
    pub database: ComboNested,

    #[config(dynamic)]
    pub replicas: u32,

    #[config(interpolate)]
    pub url: String,

    #[config(watch)]
    pub banner: String,
}

#[test]
#[serial]
fn combined_flatten_dynamic_interpolate_watch_attrs_compose() {
    let (_file, path) = write_cwd_toml(
        "host = \"combo-db\"\nport = 7\nreplicas = 2\nurl = \"pg://${database.host}:${database.port}\"\nbanner = \"v1\"\n",
    );
    let cfg = ComboAttrs::load_file_with_env(&path).expect("combined attrs load");
    // flatten:顶层键归位进嵌套结构。
    assert_eq!(cfg.database.host, "combo-db");
    assert_eq!(cfg.database.port, 7);
    // dynamic:句柄以加载值为初值。
    assert_eq!(cfg.replicas_handle().get(), 2);
    // interpolate:引用解析自 flatten 归位后的树(host/port 已在 database 下)。
    assert_eq!(cfg.url, "pg://combo-db:7");
    // watch:订阅器可从加载后的快照构建。
    let (_tx, rx) = tokio::sync::watch::channel(std::sync::Arc::new(cfg.clone()));
    let watcher = cfg.field_watcher(rx);
    assert_eq!(
        watcher.watched_fields(),
        vec![std::sync::Arc::<str>::from("banner")]
    );
}
/// rename_all 批量命名:文件键使用外部命名,codegen 在反序列化前映射回 serde 名。
#[derive(Debug, confers::Config, serde::Deserialize)]
#[config(rename_all = "camelCase")]
struct CamelCaseNaming {
    user_name: String,
    max_retries: u32,
}

#[test]
#[serial]
fn rename_all_camel_case_maps_file_keys_to_serde_names() {
    let (_file, path) = write_cwd_toml("userName = \"amy\"\nmaxRetries = 5\n");
    let cfg = CamelCaseNaming::load_file_with_env(&path).expect("camelCase load");
    assert_eq!(cfg.user_name, "amy");
    assert_eq!(cfg.max_retries, 5);
}

#[derive(Debug, confers::Config, serde::Deserialize)]
#[config(rename_all = "kebab-case")]
struct KebabCaseNaming {
    log_level: String,
}

#[test]
#[serial]
fn rename_all_kebab_case_maps_file_keys_to_serde_names() {
    let (_file, path) = write_cwd_toml("log-level = \"debug\"\n");
    let cfg = KebabCaseNaming::load_file_with_env(&path).expect("kebab-case load");
    assert_eq!(cfg.log_level, "debug");

    // serde 名显式出现时优先于外部命名(不被外部键覆盖)。
    let (_file, path) = write_cwd_toml("log-level = \"kebab\"\nlog_level = \"explicit\"\n");
    let cfg = KebabCaseNaming::load_file_with_env(&path).expect("explicit serde key load");
    assert_eq!(cfg.log_level, "explicit", "explicit serde-named key wins");
}

#[derive(Debug, confers::Config, serde::Deserialize)]
#[config(rename_all = "snake_case")]
struct SnakeCaseNaming {
    pub host: String,
}

#[test]
#[serial]
fn rename_all_snake_case_is_identity_for_snake_fields() {
    let (_file, path) = write_cwd_toml("host = \"db\"\n");
    let cfg = SnakeCaseNaming::load_file_with_env(&path).expect("snake_case load");
    assert_eq!(cfg.host, "db");
}

/// serde(rename) 字段:merge 空间(defaults/env 覆盖/文件键)必须跟随 serde
/// 字段名,而非 Rust 标识符 —— 否则重命名后的字段永远拿不到源值。
#[derive(Debug, confers::Config, serde::Deserialize)]
struct RenamedBind {
    #[serde(rename = "bind_addr")]
    #[config(default = "fallback".to_string(), name_env = "RENAMED_BIND_CUSTOM")]
    pub bind: String,
}

#[test]
#[serial]
fn serde_rename_field_addresses_serde_name_everywhere() {
    // 文件键使用 serde 名(bind_addr)。
    let (_file, path) = write_cwd_toml("bind_addr = \"from-file\"\n");
    let cfg = RenamedBind::load_file_with_env(&path).expect("file key under serde name");
    assert_eq!(cfg.bind, "from-file");

    // env 覆盖同样落在 serde 名空间。
    let (_file, path) = write_cwd_toml("bind_addr = \"from-file\"\n");
    unsafe { std::env::set_var("RENAMED_BIND_CUSTOM", "from-env") };
    let cfg = RenamedBind::load_file_with_env(&path).expect("env override load");
    unsafe { std::env::remove_var("RENAMED_BIND_CUSTOM") };
    assert_eq!(cfg.bind, "from-env");

    // 无任何来源时 default 生效(default 也注册在 serde 名下)。
    let cfg = RenamedBind::load_sync().expect("default load");
    assert_eq!(cfg.bind, "fallback");
}
