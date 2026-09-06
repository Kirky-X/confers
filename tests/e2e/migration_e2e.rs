// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! E2E: 版本迁移(tests/e2e/migration_e2e.rs)
//!
//! 场景固化(docs/TEST_SCENARIOS.md §2.13):
//! - MIG-01/02 `Versioned` trait(手写 + `ConfigMigration` derive)与
//!   `MigrationRegistry` builder 链式注册
//! - MIG-03/04 真实 toml v1 文件 → 链式迁移 v1→v2→v3(字段改名 + 默认值补充)
//! - MIG-05 同版本迁移 no-op
//! - MIG-06 未注册路径 → `MigrationError`(MigrationFailed 族)
//! - MIG-07 迁移 fn 内部出错 → 错误传播,不产出半成品
//! - MIG-08 `precompute_paths` 多分支选路:direct 路径优先于链
//! - MIG-09/10 `MigrationOnReload` 三态语义与 derive version 属性透传
//!
//! MIG-11/12(migration+snapshot / migration+watch 组合)固化于 combo_e2e.rs
//! (CMP-02/CMP-05)。

use confers::migration::{MigrationOnReload, MigrationRegistry, Versioned};
use confers::types::{AnnotatedValue, ConfigValue, SourceId};
use confers::{ConfigError, LoaderConfig, load_file};
use std::path::Path;

/// 手写 Versioned 实现(MIG-01)。
struct LegacyConfigV1;
impl Versioned for LegacyConfigV1 {
    const VERSION: u32 = 1;
}

struct CurrentConfigV4;
impl Versioned for CurrentConfigV4 {
    const VERSION: u32 = 4;
}

/// `ConfigMigration` derive:version 属性透传(MIG-01/10)。
#[derive(Debug, confers::ConfigMigration)]
#[config(version = 7)]
#[allow(dead_code)] // 字段仅经 derive 生成物使用(同 tests/core/derive.rs 约定)。
struct E2eDerivedConfig {
    pub name: String,
}

fn write_v1_file(dir: &Path) -> std::path::PathBuf {
    let path = dir.join("app_v1.toml");
    std::fs::write(
        &path,
        "name = \"svc\"\nport = 8080\ndb_host = \"localhost\"\n",
    )
    .expect("write v1 config");
    path
}

fn entry_entries(v: &AnnotatedValue) -> Vec<(String, AnnotatedValue)> {
    match &v.inner {
        ConfigValue::Map(entries) => entries
            .iter()
            .map(|(k, val)| (k.to_string(), val.clone()))
            .collect(),
        _ => panic!("expected map config tree"),
    }
}

#[test]
fn mig0102_versioned_variants_and_registry_builder_chain() {
    // Versioned:不同类型声明不同版本(MIG-01)。
    assert_eq!(LegacyConfigV1::VERSION, 1);
    assert_eq!(CurrentConfigV4::VERSION, 4);
    assert_eq!(<E2eDerivedConfig as Versioned>::VERSION, 7);

    // register 返回 &mut Self 支持 builder 链(MIG-02)。
    let registry = MigrationRegistry::builder()
        .register(1, 2, Ok)
        .register(2, 3, Ok)
        .build();
    assert_eq!(registry.migrations().len(), 2);
    assert!(registry.migrations().contains_key(&(1, 2)));
    assert!(registry.migrations().contains_key(&(2, 3)));

    let mut chained = MigrationRegistry::new();
    let _ = chained
        .register(4, 5, Ok)
        .register(5, 6, Ok)
        .register(6, 7, Ok);
    assert_eq!(chained.migrations().len(), 3);
}

#[test]
fn mig030405_real_file_chain_migration_and_noop() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_v1_file(dir.path());

    // 真实文件 → AnnotatedValue 树(MIG-03 输入)。
    let v1 = load_file(&path, &LoaderConfig::new().allow_absolute()).expect("load v1 config");
    assert_eq!(v1.version, 0, "fresh load carries no migration version");

    let mut registry = MigrationRegistry::new();
    // v1→v2:db_host 改名 database_host(MIG-03)。
    registry.register(1, 2, |mut v| {
        let mut entries = entry_entries(&v)
            .into_iter()
            .map(|(k, val)| {
                if k == "db_host" {
                    ("database_host".to_string(), val)
                } else {
                    (k, val)
                }
            })
            .collect::<Vec<_>>();
        entries.push((
            "schema_version".to_string(),
            AnnotatedValue::new(
                ConfigValue::integer(2),
                SourceId::new("migration-v2"),
                "schema_version",
            ),
        ));
        v.inner = ConfigValue::map(entries);
        v.version = 2;
        Ok(v)
    });
    // v2→v3:补充 timeout_ms 默认值(MIG-04 的一步)。
    registry.register(2, 3, |mut v| {
        let mut entries = entry_entries(&v);
        entries.push((
            "timeout_ms".to_string(),
            AnnotatedValue::new(
                ConfigValue::uint(30),
                SourceId::new("migration-v3"),
                "timeout_ms",
            ),
        ));
        v.inner = ConfigValue::map(entries);
        v.version = 3;
        Ok(v)
    });
    registry.precompute_paths();

    // 链式迁移 v1→v3(经 v2)按序执行(MIG-04)。
    let migrated = registry.migrate(v1, 1, 3).expect("chain migration");
    assert_eq!(migrated.version, 3);

    let fields = entry_entries(&migrated);
    let keys: Vec<&str> = fields.iter().map(|(k, _)| k.as_str()).collect();
    assert!(
        keys.contains(&"database_host"),
        "renamed key present: {keys:?}"
    );
    assert!(!keys.contains(&"db_host"), "old key gone: {keys:?}");
    assert!(keys.contains(&"timeout_ms"), "default added: {keys:?}");
    assert!(
        keys.contains(&"name") && keys.contains(&"port"),
        "untouched keys preserved"
    );
    let db_host = fields.iter().find(|(k, _)| k == "database_host").unwrap();
    assert_eq!(db_host.1.inner.as_str(), Some("localhost"));
    let timeout = fields.iter().find(|(k, _)| k == "timeout_ms").unwrap();
    assert_eq!(timeout.1.inner.as_u64(), Some(30));

    // 同版本迁移 no-op 直接返回(MIG-05)。
    let untouched =
        load_file(&path, &LoaderConfig::new().allow_absolute()).expect("reload for noop");
    let noop = registry
        .migrate(untouched, 2, 2)
        .expect("same version is a no-op");
    assert_eq!(
        noop.version, 0,
        "no migration fn ran, version metadata untouched"
    );
}

#[test]
fn mig060708_no_path_error_and_failure_propagation_and_direct_preference() {
    let mut registry = MigrationRegistry::new();

    // MIG-06:未注册路径 → MigrationError。
    let err = registry
        .migrate(
            AnnotatedValue::new(ConfigValue::null(), SourceId::new("e2e"), ""),
            1,
            9,
        )
        .expect_err("no path must fail");
    assert!(
        matches!(err, ConfigError::MigrationFailed { .. }),
        "migration-not-found must surface as MigrationFailed, got {err:?}"
    );

    // MIG-07:迁移 fn 内部出错 → 传播,调用方拿不到半成品。
    registry.register(10, 11, |v| {
        let _ = v;
        Err(ConfigError::migration_failed(
            10,
            11,
            "boom inside migration fn",
        ))
    });
    registry.precompute_paths();
    let err = registry
        .migrate(
            AnnotatedValue::new(ConfigValue::integer(1), SourceId::new("e2e"), "").with_version(10),
            10,
            11,
        )
        .expect_err("failing fn must propagate");
    assert!(
        matches!(err, ConfigError::MigrationFailed { .. }),
        "fn error must surface as MigrationFailed, got {err:?}"
    );

    // MIG-08:direct 路径优先于链。
    let mut multi = MigrationRegistry::new();
    let direct_ran = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let chain_ran = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));

    let direct_flag = direct_ran.clone();
    multi.register(1, 3, move |mut v| {
        direct_flag.store(true, std::sync::atomic::Ordering::SeqCst);
        v.version = 3;
        Ok(v)
    });
    let chain_flag = chain_ran.clone();
    multi.register(1, 2, move |mut v| {
        chain_flag.store(true, std::sync::atomic::Ordering::SeqCst);
        v.version = 2;
        Ok(v)
    });
    multi.register(2, 3, |mut v| {
        v.version = 3;
        Ok(v)
    });
    multi.precompute_paths();

    let input = AnnotatedValue::new(ConfigValue::null(), SourceId::new("e2e"), "").with_version(1);
    let out = multi.migrate(input, 1, 3).expect("direct path");
    assert_eq!(out.version, 3);
    assert!(
        direct_ran.load(std::sync::atomic::Ordering::SeqCst),
        "direct fn executed"
    );
    assert!(
        !chain_ran.load(std::sync::atomic::Ordering::SeqCst),
        "chain fn must not run when direct path exists"
    );
}

#[test]
fn mig0910_on_reload_semantics_and_derive_version_attribute() {
    // MIG-09:三态与默认值。
    assert_eq!(
        MigrationOnReload::default(),
        MigrationOnReload::OnVersionChange
    );
    let variants = [
        MigrationOnReload::Always,
        MigrationOnReload::OnVersionChange,
        MigrationOnReload::Disabled,
    ];
    for v in &variants {
        let cloned = *v;
        assert!(!format!("{v:?}").is_empty(), "Debug must be available");
        assert_eq!(cloned, *v, "Copy must round-trip");
    }

    // MIG-10:derive 的 version 属性透传(非 1 的任意值)。
    assert_eq!(<E2eDerivedConfig as Versioned>::VERSION, 7);
}
