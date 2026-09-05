// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! E2E: 模块 / 配置分组(tests/e2e/modules_e2e.rs)
//!
//! 场景固化(docs/TEST_SCENARIOS.md §2.15):
//! - MOD-11 profile 驱动激活切换端到端:APP_ENV 类环境变量决定激活 profile,
//!   `ModuleRegistry` 按激活 profile 加载对应文件并进入主配置链
//!
//! 实现说明:宏属性 `profile/profile_env` 当前为解析期保留、codegen 未消费
//! (见报告),因此本场景按库层现有能力固化 —— 以 APP_ENV 读值驱动
//! `set_active_profile`,再走 `load_active` 的真实文件加载链路。
//!
//! MOD-01…10 已有覆盖(tests/core/modules.rs、tests/core/derive.rs)。

use confers::LoaderConfig;
use confers::modules::ModuleRegistry;
use serial_test::serial;

fn write_profile_file(dir: &std::path::Path, name: &str, content: &str) -> std::path::PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, content).expect("write profile file");
    path
}

#[test]
#[serial]
fn mod11_app_env_drives_active_profile_and_file_load() {
    let dir = tempfile::tempdir().unwrap();
    let dev_path = write_profile_file(
        dir.path(),
        "dev.toml",
        "host = \"dev.example\"\nport = 1080\n",
    );
    let prod_path = write_profile_file(
        dir.path(),
        "prod.toml",
        "host = \"prod.example\"\nport = 9443\n",
    );

    let mut registry = ModuleRegistry::default();
    registry.register_group(
        "database",
        vec![("dev", dev_path.clone()), ("prod", prod_path.clone())],
        Some("dev"),
    );

    // APP_ENV=prod:按环境值切换激活 profile 并加载。
    unsafe { std::env::set_var("APP_ENV", "prod") };
    let app_env = std::env::var("APP_ENV").unwrap_or_else(|_| "dev".to_string());
    registry
        .set_active_profile("database", &app_env)
        .expect("prod profile exists");

    assert_eq!(
        registry.get_active_profile("database").unwrap().as_ref(),
        "prod"
    );
    let loaded = registry
        .load_active("database", &LoaderConfig::new().allow_absolute())
        .expect("active profile must load");
    let json = loaded.to_json();
    assert_eq!(json["host"], "prod.example");
    assert_eq!(json["port"], 9443);

    // APP_ENV 指回 dev:同一注册表复用,激活与内容随之切换。
    unsafe { std::env::set_var("APP_ENV", "dev") };
    let app_env = std::env::var("APP_ENV").unwrap();
    registry
        .set_active_profile("database", &app_env)
        .expect("dev profile exists");
    let loaded = registry
        .load_active("database", &LoaderConfig::new().allow_absolute())
        .expect("dev must load");
    assert_eq!(loaded.to_json()["host"], "dev.example");
    assert_eq!(
        registry.get_active_profile("database").unwrap().as_ref(),
        "dev"
    );

    unsafe { std::env::remove_var("APP_ENV") };
}

#[test]
fn mod11_unknown_env_value_falls_back_to_error_not_panic() {
    let dir = tempfile::tempdir().unwrap();
    let dev_path = write_profile_file(dir.path(), "dev.toml", "host = \"dev.example\"\n");

    let mut registry = ModuleRegistry::default();
    registry.register_group("database", vec![("dev", dev_path)], Some("dev"));

    let err = registry
        .set_active_profile("database", "staging")
        .expect_err("unknown env value must not activate any profile");
    assert!(
        matches!(err, confers::ConfigError::ModuleNotFound { .. }),
        "expected ModuleNotFound, got: {err:?}"
    );

    // 激活 profile 保持默认,load_active 仍可用。
    let loaded = registry
        .load_active("database", &LoaderConfig::new().allow_absolute())
        .expect("default still active");
    assert_eq!(loaded.to_json()["host"], "dev.example");
}
