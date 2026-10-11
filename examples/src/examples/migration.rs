// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! 配置版本迁移示例 - 版本升级与数据迁移
//!
//! 本示例展示如何实现配置版本迁移：
//! - `Versioned` trait 实现
//! - 迁移注册表使用
//! - 版本升级策略
//! - 热重载时的自动迁移

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

    info!("=== Configuration version migration example ===");
    info!("Version: V1 -> V2 -> V3");

    demo_basic_migration();
    demo_path_precomputation();
    demo_multi_step_migration();
    demo_migration_on_reload();

    info!("=== All examples completed ===");
}

fn demo_basic_migration() {
    info!("\n--- Basic migration example ---");

    let mut registry = MigrationRegistry::new();

    registry.register(1, 2, |mut v| {
        info!("Execute migration: v{} -> v{}", 1, 2);
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
            info!("Migration succeeded! version: {} -> {}", 1, 2);
            info!("version field after migration: {}", migrated.version);
        }
        Err(e) => {
            warn!("Migration failed: {:?}", e);
        }
    }
}

fn demo_path_precomputation() {
    info!("\n--- Path precomputation example ---");

    let mut registry = MigrationRegistry::new();

    registry.register(1, 2, |v| {
        info!("Migrate 1 -> 2");
        Ok(v)
    });

    registry.register(2, 3, |v| {
        info!("Migrate 2 -> 3");
        Ok(v)
    });

    registry.register(1, 3, |v| {
        info!("Migrate 1 -> 3 directly");
        Ok(v)
    });

    registry.precompute_paths();

    info!(
        "Migration path (1 -> 3): {:?}",
        registry.get_migration_path(1, 3)
    );
    info!(
        "Migration path (1 -> 2): {:?}",
        registry.get_migration_path(1, 2)
    );
    info!(
        "Migration path (2 -> 3): {:?}",
        registry.get_migration_path(2, 3)
    );

    if let Some(path) = registry.get_migration_path(1, 3) {
        info!("Found the optimal path: {:?}", path);
    }
}

fn demo_multi_step_migration() {
    info!("\n--- Multi-step migration example ---");

    let mut registry = MigrationRegistry::new();

    registry.register(1, 2, |mut v| {
        info!("[Step 1] Add the database configuration");
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
        info!("[Step 2] Add the cache and security configuration");

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

    info!("Initial configuration version: {}", initial_value.version);

    match registry.migrate(initial_value, 1, 3) {
        Ok(migrated) => {
            info!("Multi-step migration succeeded!");
            info!("Final version: {}", migrated.version);

            if let ConfigValue::Map(map) = &migrated.inner {
                info!("Configuration keys: {:?}", map.keys().collect::<Vec<_>>());
            }
        }
        Err(e) => {
            warn!("Migration failed: {:?}", e);
        }
    }
}

fn demo_migration_on_reload() {
    info!("\n--- MigrationOnReload example ---");

    let _always = MigrationOnReload::Always;
    let on_change = MigrationOnReload::OnVersionChange;
    let _disabled = MigrationOnReload::Disabled;

    info!("MigrationOnReload::Always: run the migration on every reload");
    info!("MigrationOnReload::OnVersionChange: migrate only when the version changes (default)");
    info!("MigrationOnReload::Disabled: migrate when reload is disabled");

    match on_change {
        MigrationOnReload::Always => {
            info!("Current configuration: Always");
        }
        MigrationOnReload::OnVersionChange => {
            info!("Current configuration: OnVersionChange (recommended)");
        }
        MigrationOnReload::Disabled => {
            info!("Current configuration: Disabled");
        }
    }

    info!("\nUsed in hot-reload scenarios:");
    info!("  let reload_policy = MigrationOnReload::OnVersionChange;");
    info!("  // migrate automatically when the configuration changes");
}
