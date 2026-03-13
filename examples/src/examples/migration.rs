//! 配置版本迁移示例 - 版本升级与数据迁移
//!
//! 本示例展示如何实现配置版本迁移：
//! - `Versioned` trait 实现
//! - 迁移注册表使用
//! - 版本升级策略
//! - 热重载时的自动迁移

use confers::migration::{MigrationOnReload, MigrationRegistry, Versioned};
use confers::value::{AnnotatedValue, ConfigValue, SourceId};
use indexmap::IndexMap;
use std::sync::Arc;
use tracing::{info, warn};

// These structs are used as type markers for migration examples
#[allow(dead_code)]
struct ConfigV1;
#[allow(dead_code)]
struct ConfigV2;
#[allow(dead_code)]
struct ConfigV3;

impl Versioned for ConfigV1 {
    const VERSION: u32 = 1;
}

impl Versioned for ConfigV2 {
    const VERSION: u32 = 2;
}

impl Versioned for ConfigV3 {
    const VERSION: u32 = 3;
}

fn main() {
    tracing_subscriber::fmt::init();

    info!("=== 配置版本迁移示例 ===");
    info!("版本: V1 -> V2 -> V3");

    demo_basic_migration();
    demo_path_precomputation();
    demo_multi_step_migration();
    demo_migration_on_reload();

    info!("=== 所有示例完成 ===");
}

fn demo_basic_migration() {
    info!("\n--- 基础迁移示例 ---");

    let mut registry = MigrationRegistry::new();

    registry.register(1, 2, |mut v| {
        info!("执行迁移: v{} -> v{}", 1, 2);
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
            info!("迁移成功! 版本: {} -> {}", 1, 2);
            info!("迁移后 version 字段: {}", migrated.version);
        }
        Err(e) => {
            warn!("迁移失败: {:?}", e);
        }
    }
}

fn demo_path_precomputation() {
    info!("\n--- 路径预计算示例 ---");

    let mut registry = MigrationRegistry::new();

    registry.register(1, 2, |v| {
        info!("迁移 1 -> 2");
        Ok(v)
    });

    registry.register(2, 3, |v| {
        info!("迁移 2 -> 3");
        Ok(v)
    });

    registry.register(1, 3, |v| {
        info!("直接迁移 1 -> 3");
        Ok(v)
    });

    registry.precompute_paths();

    info!("迁移路径 (1 -> 3): {:?}", registry.get_migration_path(1, 3));
    info!("迁移路径 (1 -> 2): {:?}", registry.get_migration_path(1, 2));
    info!("迁移路径 (2 -> 3): {:?}", registry.get_migration_path(2, 3));

    if let Some(path) = registry.get_migration_path(1, 3) {
        info!("找到最优路径: {:?}", path);
    }
}

fn demo_multi_step_migration() {
    info!("\n--- 多步迁移示例 ---");

    let mut registry = MigrationRegistry::new();

    registry.register(1, 2, |mut v| {
        info!("[Step 1] 添加 database 配置");
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
        info!("[Step 2] 添加 cache 和 security 配置");

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

    info!("初始配置 version: {}", initial_value.version);

    match registry.migrate(initial_value, 1, 3) {
        Ok(migrated) => {
            info!("多步迁移成功!");
            info!("最终版本: {}", migrated.version);

            if let ConfigValue::Map(map) = &migrated.inner {
                info!("配置键: {:?}", map.keys().collect::<Vec<_>>());
            }
        }
        Err(e) => {
            warn!("迁移失败: {:?}", e);
        }
    }
}

fn demo_migration_on_reload() {
    info!("\n--- MigrationOnReload 示例 ---");

    let _always = MigrationOnReload::Always;
    let on_change = MigrationOnReload::OnVersionChange;
    let _disabled = MigrationOnReload::Disabled;

    info!("MigrationOnReload::Always: 每次重载都执行迁移");
    info!("MigrationOnReload::OnVersionChange: 仅版本变化时迁移 (默认)");
    info!("MigrationOnReload::Disabled: 禁用重载时迁移");

    match on_change {
        MigrationOnReload::Always => {
            info!("当前配置: Always");
        }
        MigrationOnReload::OnVersionChange => {
            info!("当前配置: OnVersionChange (推荐)");
        }
        MigrationOnReload::Disabled => {
            info!("当前配置: Disabled");
        }
    }

    info!("\n在热重载场景中使用:");
    info!("  let reload_policy = MigrationOnReload::OnVersionChange;");
    info!("  // 配置变化时自动迁移");
}
