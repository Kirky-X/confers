use confers::{providers::cli_provider::CliConfigProvider, Config, ConfigError};
use std::fs;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use tempfile::TempDir;
use validator::Validate;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Config)]
#[config(env_prefix = "APP", validate)]
struct AppConfig {
    #[config(default = "8080", validate = "range(min = 1, max = 65535)")]
    server_port: u32,

    #[config(default = "\"localhost\".to_string()")]
    server_host: String,

    #[config(
        default = "\"info\".to_string()",
        validate = "regex(pattern = \"^(debug|info|warn|error)$\")"
    )]
    log_level: String,

    #[config(default = "1000", validate = "range(min = 1, max = 10000)")]
    max_connections: u32,

    #[config(default = "30", validate = "range(min = 1, max = 300)")]
    timeout_seconds: u32,

    #[config(sensitive = true, default = "\"default_secret\".to_string()")]
    api_key: String,

    database: DatabaseConfig,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Config)]
#[config(validate)]
struct DatabaseConfig {
    #[config(default = "\"postgres://localhost:5432/db\".to_string()")]
    url: String,

    #[config(default = "10", validate = "range(min = 1, max = 100)")]
    max_pool_size: u32,

    #[config(default = "true")]
    enable_ssl: bool,
}

#[test]
fn test_end_to_end_config_lifecycle() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("app.toml");

    let initial_config = r#"
        server_port = 8080
        server_host = "localhost"
        log_level = "info"
        max_connections = 1000
        timeout_seconds = 30
        api_key = "initial_secret"
        [database]
        url = "postgres://localhost:5432/mydb"
        max_pool_size = 10
        enable_ssl = true
    "#;

    fs::write(&config_path, initial_config).unwrap();

    temp_env::with_vars(
        [
            ("APP_SERVER_PORT", None::<&str>),
            ("APP_SERVER_HOST", None::<&str>),
            ("APP_LOG_LEVEL", None::<&str>),
        ],
        || {
            let config = AppConfig::load_file(config_path.clone())
                .load_sync()
                .expect("Failed to load initial config");

            assert_eq!(config.server_port, 8080);
            assert_eq!(config.server_host, "localhost");
            assert_eq!(config.log_level, "info");
            assert_eq!(config.max_connections, 1000);
            assert_eq!(config.timeout_seconds, 30);
            assert_eq!(config.api_key, "initial_secret");
            assert_eq!(config.database.url, "postgres://localhost:5432/mydb");
            assert_eq!(config.database.max_pool_size, 10);
            assert!(config.database.enable_ssl);

            config.validate().expect("Config validation should pass");
        },
    );
}

#[test]
fn test_multi_source_priority_chain() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("app.toml");

    let file_config = r#"
        server_port = 8080
        server_host = "localhost"
        log_level = "debug"
        max_connections = 1000
        timeout_seconds = 30
        [database]
        url = "postgres://localhost:5432/filedb"
        max_pool_size = 10
        enable_ssl = false
    "#;

    fs::write(&config_path, file_config).unwrap();

    temp_env::with_vars(
        [
            ("APP_SERVER_PORT", Some("9090")),
            ("APP_SERVER_HOST", Some("0.0.0.0")),
            ("APP_LOG_LEVEL", Some("warn")),
            ("APP_DATABASE__URL", Some("postgres://env-host:5432/envdb")),
            ("APP_DATABASE__MAX_POOL_SIZE", Some("20")),
        ],
        || {
            let config = AppConfig::load_file(config_path.clone())
                .load_sync()
                .expect("Failed to load config from multiple sources");

            assert_eq!(config.server_port, 9090, "Env should override file");
            assert_eq!(config.server_host, "0.0.0.0", "Env should override file");
            assert_eq!(config.log_level, "warn", "Env should override file");
            assert_eq!(config.max_connections, 1000, "File value should be used");
            assert_eq!(config.timeout_seconds, 30, "File value should be used");
            assert_eq!(config.api_key, "default_secret", "Default should be used");
            assert_eq!(
                config.database.url, "postgres://env-host:5432/envdb",
                "Env should override nested file"
            );
            assert_eq!(
                config.database.max_pool_size, 20,
                "Env should override nested file"
            );
            assert!(!config.database.enable_ssl, "File value should be used");
        },
    );
}

#[test]
fn test_cli_overrides_all_sources() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("app.toml");

    let file_config = r#"
        server_port = 8080
        server_host = "localhost"
        log_level = "debug"
        [database]
        url = "postgres://localhost:5432/filedb"
        max_pool_size = 10
        enable_ssl = false
    "#;

    fs::write(&config_path, file_config).unwrap();

    temp_env::with_vars(
        [
            ("APP_SERVER_PORT", Some("9090")),
            ("APP_SERVER_HOST", Some("0.0.0.0")),
            ("APP_LOG_LEVEL", Some("warn")),
        ],
        || {
            let cli_provider = CliConfigProvider::from_args(vec![
                "server_port=9999",
                "server_host=cli.example.com",
                "log_level=error",
                "max_connections=2000",
                "database.url=postgres://cli-host:5432/clidb",
                "database.max_pool_size=50",
            ]);

            let config = AppConfig::load_file(config_path.clone())
                .with_cli_provider(cli_provider)
                .load_sync()
                .expect("Failed to load config with CLI args");

            assert_eq!(config.server_port, 9999, "CLI should override env and file");
            assert_eq!(
                config.server_host, "cli.example.com",
                "CLI should override env and file"
            );
            assert_eq!(
                config.log_level, "error",
                "CLI should override env and file"
            );
            assert_eq!(config.max_connections, 2000, "CLI should override file");
            assert_eq!(
                config.database.url, "postgres://cli-host:5432/clidb",
                "CLI should override env and file"
            );
            assert_eq!(
                config.database.max_pool_size, 50,
                "CLI should override env and file"
            );
        },
    );
}

#[test]
fn test_validation_failure_handling() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("app.toml");

    let invalid_config = r#"
        server_port = 70000
        log_level = "invalid_level"
        max_connections = 20000
        [database]
        max_pool_size = 150
    "#;

    fs::write(&config_path, invalid_config).unwrap();

    temp_env::with_vars(
        [
            ("APP_SERVER_PORT", None::<&str>),
            ("APP_SERVER_HOST", None::<&str>),
        ],
        || {
            let load_result = AppConfig::load_file(config_path.clone()).load_sync();

            assert!(
                load_result.is_err(),
                "Loading should fail for invalid values"
            );

            let error = load_result.unwrap_err();
            assert!(
                matches!(error, ConfigError::ValidationError(_)),
                "Should return ValidationError"
            );

            if let ConfigError::ValidationError(msg) = error {
                assert!(
                    msg.contains("server_port")
                        || msg.contains("log_level")
                        || msg.contains("max_connections")
                        || msg.contains("max_pool_size"),
                    "Validation errors should include invalid fields"
                );
            }
        },
    );
}

#[test]
fn test_config_with_missing_optional_fields() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("minimal.toml");

    let minimal_config = r#"
        server_port = 8080
    "#;

    fs::write(&config_path, minimal_config).unwrap();

    temp_env::with_vars(
        [
            ("APP_SERVER_PORT", None::<&str>),
            ("APP_SERVER_HOST", None::<&str>),
        ],
        || {
            let config = AppConfig::load_file(config_path.clone())
                .load_sync()
                .expect("Failed to load minimal config");

            assert_eq!(config.server_port, 8080);
            assert_eq!(config.server_host, "localhost", "Default should be used");
            assert_eq!(config.log_level, "info", "Default should be used");
            assert_eq!(config.max_connections, 1000, "Default should be used");
            assert_eq!(config.timeout_seconds, 30, "Default should be used");
            assert_eq!(config.api_key, "default_secret", "Default should be used");
            assert_eq!(
                config.database.url, "postgres://localhost:5432/db",
                "Default should be used"
            );
            assert_eq!(config.database.max_pool_size, 10, "Default should be used");
            assert!(config.database.enable_ssl, "Default should be used");

            config
                .validate()
                .expect("Config with defaults should validate");
        },
    );
}

#[test]
fn test_concurrent_config_access() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("concurrent.toml");

    let config_content = r#"
        server_port = 8080
        server_host = "localhost"
        log_level = "info"
        max_connections = 1000
        timeout_seconds = 30
        [database]
        url = "postgres://localhost:5432/testdb"
        max_pool_size = 10
        enable_ssl = true
    "#;

    fs::write(&config_path, config_content).unwrap();

    temp_env::with_vars(
        [
            ("APP_SERVER_PORT", None::<&str>),
            ("APP_SERVER_HOST", None::<&str>),
        ],
        || {
            let config_path = Arc::new(config_path);
            let success_count = Arc::new(AtomicU32::new(0));
            let num_threads = 10;
            let iterations_per_thread = 100;

            let mut handles = vec![];

            for _ in 0..num_threads {
                let config_path = Arc::clone(&config_path);
                let success_count = Arc::clone(&success_count);

                let handle = std::thread::spawn(move || {
                    for _ in 0..iterations_per_thread {
                        let config = AppConfig::load_file(config_path.as_ref()).load_sync();

                        if let Ok(cfg) = config {
                            if cfg.validate().is_ok() {
                                success_count.fetch_add(1, Ordering::Relaxed);
                            }
                        }
                    }
                });

                handles.push(handle);
            }

            for handle in handles {
                handle.join().expect("Thread should complete successfully");
            }

            let total_success = success_count.load(Ordering::Relaxed);
            let expected_success = num_threads * iterations_per_thread;

            assert_eq!(
                total_success, expected_success,
                "All concurrent accesses should succeed: {}/{}",
                total_success, expected_success
            );
        },
    );
}

#[test]
fn test_large_scale_config_performance() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("large.toml");

    let config_content = r#"
        server_port = 8080
        server_host = "localhost"
        log_level = "info"
        max_connections = 1000
        timeout_seconds = 30
        [database]
        url = "postgres://localhost:5432/largedb"
        max_pool_size = 10
        enable_ssl = true
    "#;

    fs::write(&config_path, config_content).unwrap();

    temp_env::with_vars(
        [
            ("APP_SERVER_PORT", None::<&str>),
            ("APP_SERVER_HOST", None::<&str>),
        ],
        || {
            let iterations = 1000;
            let start = std::time::Instant::now();

            for _ in 0..iterations {
                let config = AppConfig::load_file(config_path.clone())
                    .load_sync()
                    .expect("Failed to load config");

                config.validate().expect("Config should validate");
            }

            let duration = start.elapsed();
            let avg_time = duration / iterations;

            println!(
                "Large scale config load time: {:?} per iteration (total: {:?} for {} iterations)",
                avg_time, duration, iterations
            );

            assert!(
                avg_time < std::time::Duration::from_millis(10),
                "Average load time {:?} should be less than 10ms",
                avg_time
            );
        },
    );
}

#[test]
fn test_error_recovery_with_fallback() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("recovery.toml");

    let initial_config = r#"
        server_port = 8080
        log_level = "info"
        [database]
        url = "postgres://localhost:5432/initialdb"
        max_pool_size = 10
        enable_ssl = true
    "#;

    fs::write(&config_path, initial_config).unwrap();

    temp_env::with_vars(
        [
            ("APP_SERVER_PORT", None::<&str>),
            ("APP_SERVER_HOST", None::<&str>),
        ],
        || {
            let config = AppConfig::load_file(config_path.clone())
                .load_sync()
                .expect("Failed to load config");

            assert_eq!(config.server_port, 8080);
            assert_eq!(config.log_level, "info");

            config.validate().expect("Initial config should validate");

            let updated_config = r#"
                server_port = 9090
                log_level = "debug"
                [database]
                url = "postgres://localhost:5432/recoverydb"
                max_pool_size = 20
                enable_ssl = false
            "#;

            fs::write(&config_path, updated_config).unwrap();

            let recovered_config = AppConfig::load_file(config_path.clone())
                .load_sync()
                .expect("Failed to load recovered config");

            assert_eq!(recovered_config.server_port, 9090);
            assert_eq!(recovered_config.log_level, "debug");

            recovered_config
                .validate()
                .expect("Recovered config should validate");
        },
    );
}

#[test]
fn test_format_auto_detection() {
    let dir = TempDir::new().unwrap();

    let toml_path = dir.path().join("config.toml");
    fs::write(&toml_path, r#"server_port = 8080"#).unwrap();

    let json_path = dir.path().join("config.json");
    fs::write(&json_path, r#"{"server_port": 8081}"#).unwrap();

    let yaml_path = dir.path().join("config.yaml");
    fs::write(&yaml_path, "server_port: 8082").unwrap();

    temp_env::with_vars(
        [
            ("APP_SERVER_PORT", None::<&str>),
            ("APP_SERVER_HOST", None::<&str>),
        ],
        || {
            let toml_config = AppConfig::load_file(toml_path.clone())
                .load_sync()
                .expect("Failed to load TOML");
            assert_eq!(toml_config.server_port, 8080);

            let json_config = AppConfig::load_file(json_path.clone())
                .load_sync()
                .expect("Failed to load JSON");
            assert_eq!(json_config.server_port, 8081);

            let yaml_config = AppConfig::load_file(yaml_path.clone())
                .load_sync()
                .expect("Failed to load YAML");
            assert_eq!(yaml_config.server_port, 8082);
        },
    );
}

#[cfg(feature = "watch")]
#[tokio::test]
async fn test_hot_reload_with_validation() {
    use std::io::Write;

    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("hot_reload.toml");

    let initial_config = r#"
        server_port = 8080
        server_host = "localhost"
        log_level = "info"
        max_connections = 1000
        [database]
        url = "postgres://localhost:5432/initialdb"
        max_pool_size = 10
        enable_ssl = true
    "#;

    {
        let mut file = fs::File::create(&config_path).unwrap();
        file.write_all(initial_config.as_bytes()).unwrap();
        file.sync_all().unwrap();
    }

    let config_path_clone = config_path.clone();
    tokio::spawn(async move {
        sleep(Duration::from_millis(100)).await;

        let updated_config = r#"
            server_port = 9090
            server_host = "0.0.0.0"
            log_level = "debug"
            max_connections = 2000
            [database]
            url = "postgres://localhost:5432/updateddb"
            max_pool_size = 20
            enable_ssl = false
        "#;

        {
            let mut file = fs::File::create(&config_path_clone).unwrap();
            file.write_all(updated_config.as_bytes()).unwrap();
            file.sync_all().unwrap();
        }
    });

    sleep(Duration::from_millis(200)).await;

    let reloaded_config = AppConfig::load_file(config_path.clone())
        .load()
        .await
        .expect("Failed to reload config");

    assert_eq!(reloaded_config.server_port, 9090);
    assert_eq!(reloaded_config.server_host, "0.0.0.0");
    assert_eq!(reloaded_config.log_level, "debug");
    assert_eq!(reloaded_config.max_connections, 2000);
    assert_eq!(
        reloaded_config.database.url,
        "postgres://localhost:5432/updateddb"
    );
    assert_eq!(reloaded_config.database.max_pool_size, 20);
    assert!(!reloaded_config.database.enable_ssl);

    reloaded_config
        .validate()
        .expect("Reloaded config should validate");
}

#[test]
fn test_sensitive_data_handling() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("sensitive.toml");

    let config_content = r#"
        server_port = 8080
        api_key = "super_secret_key_12345"
        [database]
        url = "postgres://user:password@localhost:5432/sensitivedb"
        max_pool_size = 10
        enable_ssl = true
    "#;

    fs::write(&config_path, config_content).unwrap();

    temp_env::with_vars(
        [
            ("APP_SERVER_PORT", None::<&str>),
            ("APP_SERVER_HOST", None::<&str>),
        ],
        || {
            let config = AppConfig::load_file(config_path.clone())
                .load_sync()
                .expect("Failed to load config");

            assert_eq!(config.api_key, "super_secret_key_12345");
            assert_eq!(
                config.database.url,
                "postgres://user:password@localhost:5432/sensitivedb"
            );

            config
                .validate()
                .expect("Config with sensitive data should validate");
        },
    );
}

#[test]
fn test_nested_config_with_flatten() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("nested.toml");

    let config_content = r#"
        server_port = 8080
        server_host = "localhost"
        log_level = "info"
        max_connections = 1000
        timeout_seconds = 30
        api_key = "test_key"
        [database]
        url = "postgres://localhost:5432/nesteddb"
        max_pool_size = 15
        enable_ssl = true
    "#;

    fs::write(&config_path, config_content).unwrap();

    temp_env::with_vars(
        [
            ("APP_SERVER_PORT", None::<&str>),
            ("APP_SERVER_HOST", None::<&str>),
        ],
        || {
            let config = AppConfig::load_file(config_path.clone())
                .load_sync()
                .expect("Failed to load nested config");

            assert_eq!(config.server_port, 8080);
            assert_eq!(config.database.url, "postgres://localhost:5432/nesteddb");
            assert_eq!(config.database.max_pool_size, 15);
            assert!(config.database.enable_ssl);

            config.validate().expect("Nested config should validate");
        },
    );
}
