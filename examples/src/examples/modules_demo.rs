//! 本示例展示 confers 的模块系统功能：
//! - 使用 `ModuleRegistry` 注册配置组
//! - 为不同特性注册 `ModuleConfig`
//! - 根据配置加载与切换模块
//! - 通过环境变量解析活动配置

use confers::loader::LoaderConfig;
use confers::modules::ModuleRegistry;
use std::fs;
use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    println!("========================================");
    println!("  Modules - 配置模块系统示例");
    println!("========================================");

    // 创建临时配置目录与测试文件
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
    println!("\n配置目录: {}", conf_dir.display());

    // 1. 创建 ModuleRegistry
    println!("\n[创建模块注册表]");
    let mut registry = ModuleRegistry::with_capacity(4);
    println!("  注册表大小: {}", registry.len());
    println!("  是否为空: {}", registry.is_empty());

    // 2. 注册配置组（链式调用）
    println!("\n[注册配置组]");
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
    println!("  注册表大小: {}", registry.len());

    // 3. 列出已注册组与活动配置
    println!("\n[已注册组]");
    for group in &registry.list_groups() {
        let active = registry
            .get_active_profile(group)
            .map(|p| p.to_string())
            .unwrap_or_default();
        println!("  组: {} (活动配置: {})", group, active);
    }

    // 4. 查看模块配置详情
    println!("\n[模块配置详情 - database]");
    if let Some(db_module) = registry.get("database") {
        println!("  模块名: {}", db_module.name());
        println!("  活动配置: {}", db_module.active_profile());
        println!("  配置数: {}", db_module.profile_count());
        let profiles: Vec<String> = db_module.profiles().iter().map(|s| s.to_string()).collect();
        println!("  可用配置: {:?}", profiles);
        println!("  含 mysql: {}", db_module.has_profile("mysql"));
    }

    // 5. 加载指定模块配置
    println!("\n[加载模块配置]");
    let loader_config = LoaderConfig::new()
        .no_symlink_check()
        .allow_absolute()
        .allowed_dirs(Vec::<PathBuf>::new());
    let _db_config = registry.load_module("database", "postgresql", &loader_config)?;
    println!("  database/postgresql 加载成功");

    // 6. 加载活动配置
    println!("\n[加载活动配置]");
    let _active_config = registry.load_active("cache", &loader_config)?;
    println!("  cache 活动配置 (redis) 加载成功");

    // 7. 切换活动配置
    println!("\n[切换活动配置]");
    println!(
        "  切换前 database 活动: {:?}",
        registry.get_active_profile("database")
    );
    registry.set_active_profile("database", "postgresql")?;
    println!(
        "  切换后 database 活动: {:?}",
        registry.get_active_profile("database")
    );

    // 8. 所有活动配置映射
    println!("\n[所有活动配置]");
    for (group, profile) in &registry.active_profiles() {
        println!("  {} -> {}", group, profile);
    }

    // 9. 环境变量解析
    println!("\n[环境变量解析]");
    std::env::set_var("APP_DATABASE_PROFILE", "mysql");
    registry.resolve_from_env(Some("APP_"));
    println!(
        "  环境变量解析后 database 活动: {:?}",
        registry.get_active_profile("database")
    );
    std::env::remove_var("APP_DATABASE_PROFILE");

    // 10. 验证活动配置文件存在性
    println!("\n[验证活动配置]");
    match registry.validate_active_profiles() {
        Ok(()) => println!("  所有活动配置文件存在"),
        Err(e) => println!("  验证失败: {}", e),
    }

    // 清理临时目录
    let _ = fs::remove_dir_all(&conf_dir);

    println!("\n========================================");
    println!("  示例运行完成!");
    println!("========================================");
    Ok(())
}
