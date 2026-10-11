// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! 本示例展示 confers 的模块系统功能：
//! - 使用 `ModuleRegistry` 注册配置组
//! - 为不同特性注册 `ModuleConfig`
//! - 根据配置加载与切换模块
//! - 通过环境变量解析活动配置

use confers::i18n::{tr, tr_args};
use confers::loader::LoaderConfig;
use confers::modules::ModuleRegistry;
use std::fs;
use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    println!("========================================");
    println!("  {}", tr("example-modules-demo-title"));
    println!("========================================");

    let conf_dir = std::env::temp_dir().join("confers_modules_example");
    fs::create_dir_all(conf_dir.join("database"))?;
    fs::create_dir_all(conf_dir.join("cache"))?;
    fs::write(
        conf_dir.join("database").join("mysql.toml"),
        "host = \"localhost\"\nport = 3306\nengine = \"mysql\"\n",
    )?;
    fs::write(
        conf_dir.join("database").join("postgresql.toml"),
        "host = \"localhost\"\nport = 5432\nengine = \"postgresql\"\n",
    )?;
    fs::write(
        conf_dir.join("cache").join("redis.toml"),
        "host = \"redis-host\"\nport = 6379\ndriver = \"redis\"\n",
    )?;
    fs::write(
        conf_dir.join("cache").join("memory.toml"),
        "driver = \"memory\"\nsize = 1024\n",
    )?;
    println!(
        "\n{}",
        tr_args(
            "example-modules-demo-conf-dir",
            &[("dir", conf_dir.display().to_string())]
        )
    );

    println!("\n{}", tr("example-modules-demo-create-registry"));
    let mut registry = ModuleRegistry::with_capacity(4);
    println!(
        "  {}",
        tr_args(
            "example-modules-demo-registry-size",
            &[("size", registry.len().to_string())]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-modules-demo-is-empty",
            &[("empty", registry.is_empty().to_string())]
        )
    );

    println!("\n{}", tr("example-modules-demo-register-groups"));
    registry
        .register_group(
            "database",
            vec![
                ("mysql", conf_dir.join("database").join("mysql.toml")),
                (
                    "postgresql",
                    conf_dir.join("database").join("postgresql.toml"),
                ),
            ],
            Some("mysql"),
        )
        .register_group(
            "cache",
            vec![
                ("redis", conf_dir.join("cache").join("redis.toml")),
                ("memory", conf_dir.join("cache").join("memory.toml")),
            ],
            Some("redis"),
        );
    println!(
        "  {}",
        tr_args(
            "example-modules-demo-registry-size",
            &[("size", registry.len().to_string())]
        )
    );

    println!("\n{}", tr("example-modules-demo-registered-groups"));
    for group in &registry.list_groups() {
        let active = registry
            .get_active_profile(group)
            .map(|p| p.to_string())
            .unwrap_or_default();
        println!(
            "  {}",
            tr_args(
                "example-modules-demo-group-row",
                &[("group", group.to_string()), ("active", active)]
            )
        );
    }

    println!("\n{}", tr("example-modules-demo-details-database"));
    if let Some(db_module) = registry.get("database") {
        println!(
            "  {}",
            tr_args(
                "example-modules-demo-module-name",
                &[("name", db_module.name().to_string())]
            )
        );
        println!(
            "  {}",
            tr_args(
                "example-modules-demo-active-profile",
                &[("profile", db_module.active_profile().to_string())]
            )
        );
        println!(
            "  {}",
            tr_args(
                "example-modules-demo-profile-count",
                &[("count", db_module.profile_count().to_string())]
            )
        );
        let profiles: Vec<String> = db_module.profiles().iter().map(|s| s.to_string()).collect();
        println!(
            "  {}",
            tr_args(
                "example-modules-demo-available-profiles",
                &[("profiles", format!("{profiles:?}"))]
            )
        );
        println!(
            "  {}",
            tr_args(
                "example-modules-demo-has-mysql",
                &[("has", db_module.has_profile("mysql").to_string())]
            )
        );
    }

    println!("\n{}", tr("example-modules-demo-load-module"));
    let loader_config = LoaderConfig::new()
        .no_symlink_check()
        .allow_absolute()
        .allowed_dirs(Vec::<PathBuf>::new());
    let _db_config = registry.load_module("database", "postgresql", &loader_config)?;
    println!("  {}", tr("example-modules-demo-loaded-postgresql"));

    println!("\n{}", tr("example-modules-demo-load-active"));
    let _active_config = registry.load_active("cache", &loader_config)?;
    println!("  {}", tr("example-modules-demo-loaded-redis"));

    println!("\n{}", tr("example-modules-demo-switch-active"));
    println!(
        "  {}",
        tr_args(
            "example-modules-demo-active-before-switch",
            &[(
                "profile",
                format!("{:?}", registry.get_active_profile("database"))
            )]
        )
    );
    registry.set_active_profile("database", "postgresql")?;
    println!(
        "  {}",
        tr_args(
            "example-modules-demo-active-after-switch",
            &[(
                "profile",
                format!("{:?}", registry.get_active_profile("database"))
            )]
        )
    );

    println!("\n{}", tr("example-modules-demo-all-active"));
    for (group, profile) in &registry.active_profiles() {
        println!("  {} -> {}", group, profile);
    }

    println!("\n{}", tr("example-modules-demo-env-parsing"));
    // FIXME: Audit that the environment access only happens in single-threaded code.
    unsafe { std::env::set_var("APP_DATABASE_PROFILE", "mysql") };
    registry.resolve_from_env(Some("APP_"));
    println!(
        "  {}",
        tr_args(
            "example-modules-demo-active-after-env",
            &[(
                "profile",
                format!("{:?}", registry.get_active_profile("database"))
            )]
        )
    );
    // FIXME: Audit that the environment access only happens in single-threaded code.
    unsafe { std::env::remove_var("APP_DATABASE_PROFILE") };

    println!("\n{}", tr("example-modules-demo-validate-active"));
    match registry.validate_active_profiles() {
        Ok(()) => println!("  {}", tr("example-modules-demo-all-files-exist")),
        Err(e) => println!(
            "  {}",
            tr_args(
                "example-modules-demo-validation-failed",
                &[("error", e.to_string())]
            )
        ),
    }

    let _ = fs::remove_dir_all(&conf_dir);

    println!("\n========================================");
    println!("  {}", tr("example-modules-demo-completed"));
    println!("========================================");
    Ok(())
}
