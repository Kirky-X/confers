// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! 配置版本迁移示例 - 版本升级与数据迁移
//!
//! 本示例展示如何实现配置版本迁移：
//! - `Versioned` trait 实现
//! - 迁移注册表使用
//! - 版本升级策略
//! - 热重载时的自动迁移

use confers::i18n::{tr, tr_args};
use confers::migration::{MigrationOnReload, MigrationRegistry};
use confers::types::{AnnotatedValue, ConfigValue, SourceId};
use indexmap::IndexMap;
use std::sync::Arc;
use tracing::{info, warn};

fn main() {
    // 尊重 RUST_LOG；未设置时回落到 info 基线，保证演示输出可见
    // （裸 fmt::init() 在无 RUST_LOG 环境下静默）。
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    info!("{}", tr("example-migration-title"));
    info!("{}", tr("example-migration-version-chain"));

    demo_basic_migration();
    demo_path_precomputation();
    demo_multi_step_migration();
    demo_migration_on_reload();

    info!("{}", tr("example-migration-all-completed"));
}

fn demo_basic_migration() {
    info!("\n{}", tr("example-migration-basic-demo"));

    let mut registry = MigrationRegistry::new();

    registry.register(1, 2, |mut v| {
        info!(
            "{}",
            tr_args(
                "example-migration-execute",
                &[("from", "1".to_string()), ("to", "2".to_string())]
            )
        );
        v.version = 2;
        Ok(v)
    });

    registry.precompute_paths();

    let value = AnnotatedValue::new(
        ConfigValue::String("test".to_string()),
        SourceId::new("demo"),
        "test",
    )
    .with_version(1);

    match registry.migrate(value, 1, 2) {
        Ok(migrated) => {
            info!(
                "{}",
                tr_args(
                    "example-migration-succeeded",
                    &[("from", "1".to_string()), ("to", "2".to_string())]
                )
            );
            info!(
                "{}",
                tr_args(
                    "example-migration-version-field",
                    &[("version", migrated.version.to_string())]
                )
            );
        }
        Err(e) => {
            warn!(
                "{}",
                tr_args("example-migration-failed", &[("error", format!("{e:?}"))])
            );
        }
    }
}

fn demo_path_precomputation() {
    info!("\n{}", tr("example-migration-path-demo"));

    let mut registry = MigrationRegistry::new();

    registry.register(1, 2, |v| {
        info!("{}", tr("example-migration-migrate-1-2"));
        Ok(v)
    });

    registry.register(2, 3, |v| {
        info!("{}", tr("example-migration-migrate-2-3"));
        Ok(v)
    });

    registry.register(1, 3, |v| {
        info!("{}", tr("example-migration-migrate-1-3-direct"));
        Ok(v)
    });

    registry.precompute_paths();

    info!(
        "{}",
        tr_args(
            "example-migration-path-1-3",
            &[("path", format!("{:?}", registry.get_migration_path(1, 3)))]
        )
    );
    info!(
        "{}",
        tr_args(
            "example-migration-path-1-2",
            &[("path", format!("{:?}", registry.get_migration_path(1, 2)))]
        )
    );
    info!(
        "{}",
        tr_args(
            "example-migration-path-2-3",
            &[("path", format!("{:?}", registry.get_migration_path(2, 3)))]
        )
    );

    if let Some(path) = registry.get_migration_path(1, 3) {
        info!(
            "{}",
            tr_args(
                "example-migration-optimal-path",
                &[("path", format!("{path:?}"))]
            )
        );
    }
}

fn demo_multi_step_migration() {
    info!("\n{}", tr("example-migration-multi-step-demo"));

    let mut registry = MigrationRegistry::new();

    registry.register(1, 2, |mut v| {
        info!("{}", tr("example-migration-step-1"));
        let mut map = IndexMap::new();
        map.insert(
            Arc::from("database"),
            AnnotatedValue::new(
                ConfigValue::Map(Arc::new(IndexMap::new())),
                SourceId::new("migration"),
                "database",
            ),
        );
        v.inner = ConfigValue::Map(Arc::new(map));
        v.version = 2;
        Ok(v)
    });

    registry.register(2, 3, |mut v| {
        info!("{}", tr("example-migration-step-2"));

        let mut new_map = IndexMap::new();

        if let ConfigValue::Map(existing) = &v.inner {
            for (k, av) in existing.iter() {
                new_map.insert(k.clone(), av.clone());
            }
        }

        new_map.insert(
            Arc::from("cache"),
            AnnotatedValue::new(
                ConfigValue::Map(Arc::new(IndexMap::new())),
                SourceId::new("migration"),
                "cache",
            ),
        );
        new_map.insert(
            Arc::from("security"),
            AnnotatedValue::new(
                ConfigValue::Map(Arc::new(IndexMap::new())),
                SourceId::new("migration"),
                "security",
            ),
        );

        v.inner = ConfigValue::Map(Arc::new(new_map));
        v.version = 3;
        Ok(v)
    });

    registry.precompute_paths();

    let initial_value = AnnotatedValue::new(
        ConfigValue::Map(Arc::new(IndexMap::new())),
        SourceId::new("demo"),
        "root",
    )
    .with_version(1);

    info!(
        "{}",
        tr_args(
            "example-migration-initial-version",
            &[("version", initial_value.version.to_string())]
        )
    );

    match registry.migrate(initial_value, 1, 3) {
        Ok(migrated) => {
            info!("{}", tr("example-migration-multi-step-succeeded"));
            info!(
                "{}",
                tr_args(
                    "example-migration-final-version",
                    &[("version", migrated.version.to_string())]
                )
            );

            if let ConfigValue::Map(map) = &migrated.inner {
                info!(
                    "{}",
                    tr_args(
                        "example-migration-config-keys",
                        &[("keys", format!("{:?}", map.keys().collect::<Vec<_>>()))]
                    )
                );
            }
        }
        Err(e) => {
            warn!(
                "{}",
                tr_args("example-migration-failed", &[("error", format!("{e:?}"))])
            );
        }
    }
}

fn demo_migration_on_reload() {
    info!("\n{}", tr("example-migration-on-reload-demo"));

    let _always = MigrationOnReload::Always;
    let on_change = MigrationOnReload::OnVersionChange;
    let _disabled = MigrationOnReload::Disabled;

    info!("{}", tr("example-migration-always-desc"));
    info!("{}", tr("example-migration-on-version-change-desc"));
    info!("{}", tr("example-migration-disabled-desc"));

    match on_change {
        MigrationOnReload::Always => {
            info!("{}", tr("example-migration-current-always"));
        }
        MigrationOnReload::OnVersionChange => {
            info!("{}", tr("example-migration-current-on-version-change"));
        }
        MigrationOnReload::Disabled => {
            info!("{}", tr("example-migration-current-disabled"));
        }
    }

    info!("\n{}", tr("example-migration-hot-reload-usage"));
    info!("  let reload_policy = MigrationOnReload::OnVersionChange;");
    info!("  {}", tr("example-migration-auto-migrate-comment"));
}
