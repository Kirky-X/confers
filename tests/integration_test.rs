// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

use clap::Parser;
use confers::Config;
use std::fs;
use std::net::SocketAddr;
use tempfile::TempDir;
use validator::Validate;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Config)]
#[config(env_prefix = "APP", validate)]
struct AppConfig {
    #[config(default = "8080", validate = "range(min = 1, max = 65535)")]
    server_port: u32,

    #[config(default = "\"localhost\".to_string()")]
    server_host: String,

    #[config(sensitive = true, default = "\"secret_password\".to_string()")]
    db_password: String,

    #[config(default = "\"postgres://localhost:5432/db\".to_string()")]
    db_url: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Config)]
#[config(env_prefix = "TEST_", validate)]
struct NestedConfig {
    #[config(default = "\"default_val\".to_string()")]
    name: String,

    #[serde(flatten)]
    details: Details,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Config)]
#[config(validate)]
struct Details {
    #[config(default = "10", validate = "range(min = 1, max = 100)")]
    count: u32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Config)]
#[config(strict = true)]
struct StrictConfig {
    #[config(default = "1")]
    val: u32,
}

#[test]
fn test_basic_load_defaults() {
    temp_env::with_vars(
        [
            ("APP_SERVER_PORT", None::<&str>),
            ("APP_SERVER_HOST", None::<&str>),
        ],
        || {
            let config = AppConfig::load_sync().expect("Failed to load config");
            assert_eq!(config.server_port, 8080);
            assert_eq!(config.server_host, "localhost");
        },
    );
}

#[test]
fn test_load_from_toml_file() {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("config.toml");
    fs::write(
        &file_path,
        r#"
        server_port = 9090
        server_host = "127.0.0.1"
    "#,
    )
    .unwrap();

    temp_env::with_vars(
        [
            ("APP_SERVER_PORT", None::<&str>),
            ("APP_SERVER_HOST", None::<&str>),
        ],
        || {
            // Test explicit file loading
            let config = AppConfig::load_file(file_path)
                .load_sync()
                .expect("Failed to load config from explicit file");
            assert_eq!(config.server_port, 9090);
            assert_eq!(config.server_host, "127.0.0.1");
        },
    );
}

#[test]
fn test_load_from_json_file() {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("config.json");
    fs::write(
        &file_path,
        r#"
        {
            "server_port": 9091,
            "server_host": "127.0.0.2"
        }
    "#,
    )
    .unwrap();

    temp_env::with_vars(
        [
            ("APP_SERVER_PORT", None::<&str>),
            ("APP_SERVER_HOST", None::<&str>),
        ],
        || {
            // Test explicit file loading
            let config = AppConfig::load_file(file_path)
                .load_sync()
                .expect("Failed to load config from explicit json file");
            assert_eq!(config.server_port, 9091);
            assert_eq!(config.server_host, "127.0.0.2");
        },
    );
}

#[test]
fn test_load_from_yaml_file() {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("config.yaml");
    fs::write(
        &file_path,
        r#"
        server_port: 9092
        server_host: "127.0.0.3"
    "#,
    )
    .unwrap();

    // Ensure no env vars interfere
    temp_env::with_vars(
        [
            ("APP_SERVER_PORT", None::<&str>),
            ("APP_SERVER_HOST", None::<&str>),
        ],
        || {
            // Test explicit file loading
            let config = AppConfig::load_file(file_path)
                .load_sync()
                .expect("Failed to load config from explicit yaml file");
            assert_eq!(config.server_port, 9092);
            assert_eq!(config.server_host, "127.0.0.3");
        },
    );
}

#[test]
fn test_env_override() {
    // Use temp_env to properly isolate environment variables
    temp_env::with_vars(
        [
            ("APP_SERVER_PORT", Some("9999")),
            ("APP_SERVER_HOST", Some("0.0.0.0")),
        ],
        || {
            let config = AppConfig::load_sync().expect("Failed to load config");
            assert_eq!(config.server_port, 9999);
            assert_eq!(config.server_host, "0.0.0.0");
        },
    );
}

#[test]
fn test_nested_config() {
    temp_env::with_var("TEST_DETAILS__COUNT", None::<&str>, || {
        let config = NestedConfig::load_sync().expect("Failed to load nested config");
        assert_eq!(config.name, "default_val");
        assert_eq!(config.details.count, 10);
    });
}

#[test]
fn test_nested_config_merge() {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("nested.toml");
    fs::write(
        &file_path,
        r#"
                name = "file_val"
                [details]
                count = 50
            "#,
    )
    .unwrap();

    // For flattened structures, Figment expects environment variables to match the flattened field names
    // So TEST_COUNT maps to the flattened count field, not TEST_DETAILS__COUNT
    temp_env::with_vars(
        [
            ("APP_SERVER_PORT", None::<&str>),
            ("APP_SERVER_HOST", None::<&str>),
            ("TEST_NAME", Some("env_val")),
            ("TEST_COUNT", Some("75")),
        ],
        || {
            let config = NestedConfig::load_file(file_path)
                .load_sync()
                .expect("Failed to load nested config");

            println!("DEBUG: config.name = {:?}", config.name);
            println!("DEBUG: config.details.count = {:?}", config.details.count);

            assert_eq!(config.name, "env_val");
            // The environment variable (75) should override the file (50)
            assert_eq!(config.details.count, 75);
        },
    );
}

#[test]
fn test_performance_load() {
    temp_env::with_vars(
        [
            ("APP_SERVER_PORT", None::<&str>),
            ("APP_SERVER_HOST", None::<&str>),
        ],
        || {
            let start = std::time::Instant::now();
            let iterations = 50;
            for _ in 0..iterations {
                let _ = AppConfig::load_sync();
            }
            let duration = start.elapsed();
            let avg = duration / iterations;
            println!("Average load time: {:?}", avg);

            assert!(
                avg < std::time::Duration::from_millis(500),
                "Average load time {:?} exceeded 500ms in parallel execution",
                avg
            );
        },
    );
}

#[test]
fn test_clap_integration() {
    // Test that ClapShadow struct is generated
    temp_env::with_vars(
        [
            ("APP_SERVER_PORT", None::<&str>),
            ("APP_SERVER_HOST", None::<&str>),
        ],
        || {
            let config = AppConfig::load_from_args(vec![
                "test_program".to_string(),
                "--server_port".to_string(),
                "7777".to_string(),
                "--server_host".to_string(),
                "test.example.com".to_string(),
                "--db_password".to_string(),
                "test_pass".to_string(),
            ])
            .expect("Failed to load config from CLI args");

            assert_eq!(config.server_port, 7777);
            assert_eq!(config.server_host, "test.example.com");
        },
    );
}

#[test]
fn test_debug_clap_args() {
    // Debug test to see what CLI args are generated
    use std::env;

    // Set environment variables to ensure defaults are used
    env::remove_var("APP_SERVER_PORT");
    env::remove_var("APP_SERVER_HOST");

    let args = vec![
        "test_program".to_string(),
        "--server_port".to_string(),
        "7777".to_string(),
        "--server_host".to_string(),
        "test.example.com".to_string(),
    ];

    let clap_args = AppConfigClapShadow::parse_from(&args);
    let cli_args = clap_args.to_cli_args();
    println!("Generated CLI args: {:?}", cli_args);

    let config = AppConfig::load_from_args(args).expect("Failed to load config");
    println!(
        "Loaded config: port={}, host={}",
        config.server_port, config.server_host
    );
}

#[test]
#[cfg(feature = "schema")]
fn test_json_schema_generation() {
    use confers::schema::json::JsonSchemaGenerator;

    let schema = JsonSchemaGenerator::generate::<AppConfig>();

    // Basic validation of the generated schema
    assert!(schema.is_object());
    let schema_obj = schema.as_object().unwrap();

    // Check if properties exist
    let properties = schema_obj
        .get("properties")
        .expect("Schema should have properties")
        .as_object()
        .unwrap();

    assert!(properties.contains_key("server_port"));
    assert!(properties.contains_key("server_host"));
    assert!(properties.contains_key("db_password"));

    // Check type of a field
    let port_prop = properties.get("server_port").unwrap().as_object().unwrap();
    assert_eq!(port_prop.get("type").unwrap(), "integer");
    assert_eq!(port_prop.get("format").unwrap(), "uint32");
}

#[test]
#[cfg(feature = "schema")]
fn test_schema_generation_methods() {
    // Test JSON Schema generation via the new public API method
    let json_schema = AppConfig::json_schema();
    println!(
        "JSON Schema: {}",
        serde_json::to_string_pretty(&json_schema).unwrap()
    );
    assert!(json_schema.is_object());

    let schema_obj = json_schema.as_object().unwrap();
    let properties = schema_obj
        .get("properties")
        .expect("Schema should have properties")
        .as_object()
        .unwrap();

    // Verify that our config fields are present in the schema
    assert!(properties.contains_key("server_port"));
    assert!(properties.contains_key("server_host"));
    assert!(properties.contains_key("db_password"));
    assert!(properties.contains_key("db_url"));

    // Test TypeScript Schema generation
    let ts_schema = AppConfig::typescript_schema();
    println!("TypeScript Schema: {}", ts_schema);
    assert!(!ts_schema.is_empty());

    // Verify TypeScript schema contains expected interface definitions
    assert!(ts_schema.contains("interface"));
    assert!(ts_schema.contains("AppConfig"));

    // Test schema export to file
    let temp_dir = TempDir::new().unwrap();
    let schema_path = temp_dir.path().join("app_config_schema.json");

    AppConfig::export_schema(&schema_path).expect("Failed to export schema");

    // Verify the file was created and contains valid JSON
    assert!(schema_path.exists());
    let exported_content =
        std::fs::read_to_string(&schema_path).expect("Failed to read exported schema");
    let exported_schema: serde_json::Value =
        serde_json::from_str(&exported_content).expect("Exported schema should be valid JSON");

    assert!(exported_schema.is_object());
    assert!(exported_schema.get("properties").is_some());
}

#[test]
fn test_nested_clap_integration() {
    // NestedConfig has a flattened 'details' field of type Details.
    // Details has 'count'.
    // So we should be able to pass --count 99

    // Debug: Let's see what CLI args are generated
    use std::env;
    env::remove_var("TEST_NAME");
    env::remove_var("TEST_COUNT");

    // First, let's try to parse the args manually
    let args = vec![
        "test_program".to_string(),
        "--name".to_string(),
        "custom_name".to_string(),
        "--count".to_string(),
        "99".to_string(),
    ];

    let clap_args = NestedConfigClapShadow::parse_from(&args);
    let cli_args = clap_args.to_cli_args();
    println!("Generated CLI args: {:?}", cli_args);

    let config = NestedConfig::load_from_args(vec![
        "test_program".to_string(),
        "--name".to_string(),
        "custom_name".to_string(),
        "--count".to_string(),
        "99".to_string(),
    ])
    .expect("Failed to load nested config from CLI args");

    println!(
        "Loaded config: name='{}', count={}",
        config.name, config.details.count
    );
    assert_eq!(config.name, "custom_name");
    assert_eq!(config.details.count, 99);
}

#[test]
fn test_strict_mode_error() {
    let result = StrictConfig::load_from_args(vec![
        "test_program".to_string(),
        "--unknown_arg".to_string(),
        "value".to_string(),
    ]);

    assert!(
        result.is_err(),
        "Should return error for unknown arg in strict mode"
    );
    // We expect clap error wrapped in ConfigError
    println!("Error: {:?}", result.err());
}

#[test]
fn test_user_config_path() {
    let home_dir = TempDir::new().unwrap();
    let pkg_name = env!("CARGO_PKG_NAME");

    temp_env::with_var("HOME", Some(home_dir.path().to_str().unwrap()), || {
        temp_env::with_vars([("APP_SERVER_PORT", None::<&str>)], || {
            let config_dir = dirs::config_dir().expect("Failed to get config dir");
            let app_config_dir = config_dir.join(pkg_name);
            fs::create_dir_all(&app_config_dir).unwrap();

            let config_file_path = app_config_dir.join("config.toml");
            fs::write(
                &config_file_path,
                r#"
                server_port = 6666
                server_host = "localhost"
                db_password = "default_pass"
                db_url = "postgres://localhost:5432/testdb"
            "#,
            )
            .unwrap();

            let config = confers::core::ConfigLoader::<AppConfig>::new()
                .with_app_name(pkg_name)
                .load_sync()
                .expect("Failed to load config from user path");

            assert_eq!(config.server_port, 6666);
        });
    });
}

#[cfg(feature = "watch")]
#[tokio::test]
async fn test_config_watcher() {
    use confers::watcher::ConfigWatcher;
    use std::io::Write;
    use std::time::Duration;

    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("watch_config.toml");

    // Initial write
    {
        let mut file = fs::File::create(&file_path).unwrap();
        file.write_all(b"server_port = 1111\nserver_host = \"localhost\"")
            .unwrap();
        file.sync_all().unwrap();
    }

    let watcher = ConfigWatcher::new(vec![file_path.clone()]);
    let (_guard, rx) = watcher.watch().expect("Failed to start watcher");

    // Wait a bit for watcher to initialize
    std::thread::sleep(Duration::from_millis(100));

    // Update file
    {
        let mut file = fs::File::create(&file_path).unwrap();
        file.write_all(b"server_port = 2222\nserver_host = \"localhost\"")
            .unwrap();
        file.sync_all().unwrap();
    }

    // Wait for event
    // The debounce time is 500ms in source, so we wait slightly longer
    let event = rx.recv_timeout(Duration::from_secs(2));

    assert!(event.is_ok(), "Should receive watch event");
    let events = event.unwrap();
    assert!(events.is_ok(), "Watch event should be Ok");

    // Verify we can reload the config
    let config = AppConfig::load_file(file_path)
        .load()
        .await
        .expect("Failed to reload config");
    assert_eq!(config.server_port, 2222);
}

#[test]
fn test_multi_env_automatic_switching() {
    use std::sync::atomic::{AtomicUsize, Ordering};

    // Use a unique app name to avoid conflicts
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let app_name = format!("testapp_{}", COUNTER.fetch_add(1, Ordering::SeqCst));

    let dir = TempDir::new().unwrap();

    // Create base config: config.toml (ConfigLoader looks for config.toml in search paths)
    // We put it in a subdirectory named {app_name} because ConfigLoader searches there too
    // search_paths.push(config_dir.join(app_name));
    let app_config_dir = dir.path().join(&app_name);
    fs::create_dir_all(&app_config_dir).unwrap();

    let base_path = app_config_dir.join("config.toml");
    fs::write(
        &base_path,
        r#"
        server_port = 8080
        server_host = "localhost"
    "#,
    )
    .unwrap();

    // Create prod config: {app_name}.prod.toml
    // ConfigLoader looks for {app_name}.{env}.{ext} in search paths
    // We put it in the base dir so it's found during the same iteration as config.toml (which is found via app_name subdir)
    // Within the same iteration, env files override base files.
    let prod_path = dir.path().join(format!("{}.prod.toml", app_name));
    fs::write(
        &prod_path,
        r#"
        server_port = 8082
        server_host = "0.0.0.0"
    "#,
    )
    .unwrap();

    temp_env::with_vars(
        [
            ("XDG_CONFIG_HOME", Some(dir.path().to_str().unwrap())),
            ("HOME", Some(dir.path().to_str().unwrap())),
            ("APP_SERVER_PORT", None::<&str>),
            ("APP_SERVER_HOST", None::<&str>),
        ],
        || {
            // Test with RUN_ENV=prod
            temp_env::with_vars([("RUN_ENV", Some("prod"))], || {
                let config = AppConfig::new_loader()
                    .with_app_name(&app_name)
                    .load_sync()
                    .expect("Failed to load prod config automatically");

                assert_eq!(config.server_port, 8082);
                assert_eq!(config.server_host, "0.0.0.0");
            });

            // Test with no RUN_ENV (should use base)
            temp_env::with_vars([("RUN_ENV", None::<&str>)], || {
                let config = AppConfig::new_loader()
                    .with_app_name(&app_name)
                    .load_sync()
                    .expect("Failed to load base config automatically");

                assert_eq!(config.server_port, 8080);
                assert_eq!(config.server_host, "localhost");
            });
        },
    );
}

#[test]
fn test_multi_env_switching() {
    // Use temp_env to properly isolate environment variables for parallel execution
    // We use load_sync() inside the closure because with_vars restores env vars
    // immediately after the closure returns, which would be BEFORE the future is polled
    // if we used async/await with the standard with_vars.
    temp_env::with_vars(
        [
            ("APP_SERVER_PORT", None::<&str>),
            ("APP_SERVER_HOST", None::<&str>),
        ],
        || {
            let dir = TempDir::new().unwrap();

            // Create base config
            let base_path = dir.path().join("config.toml");
            fs::write(
                &base_path,
                r#"
                server_port = 8080
                server_host = "localhost"
            "#,
            )
            .unwrap();

            // Create dev config
            let dev_path = dir.path().join("config.dev.toml");
            fs::write(
                &dev_path,
                r#"
                server_port = 8081
            "#,
            )
            .unwrap();

            // Create prod config
            let prod_path = dir.path().join("config.prod.toml");
            fs::write(
                &prod_path,
                r#"
                server_port = 8082
                server_host = "0.0.0.0"
            "#,
            )
            .unwrap();

            // Test Dev Environment
            {
                let config = AppConfig::load_file(dev_path.clone())
                    .load_sync()
                    .expect("Failed to load dev config");
                assert_eq!(config.server_port, 8081);
                assert_eq!(config.server_host, "localhost");
            }

            // Test Prod Environment
            {
                let config = AppConfig::load_file(prod_path.clone())
                    .load_sync()
                    .expect("Failed to load prod config");
                assert_eq!(config.server_port, 8082);
                assert_eq!(config.server_host, "0.0.0.0");
            }
        },
    );
}

#[test]
fn test_encryption_auto_decryption() {
    use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
    use confers::encryption::ConfigEncryption;

    // 1. Setup encryption key (32 bytes)
    let key = [0u8; 32];
    let key_b64 = BASE64.encode(key);

    // Use temp_env::with_vars to properly isolate environment variables
    temp_env::with_vars(
        [
            ("CONFERS_ENCRYPTION_KEY", Some(key_b64.as_str())),
            ("APP_SERVER_PORT", None::<&str>), // Ensure no interference
            ("APP_SERVER_HOST", None::<&str>),
        ],
        || {
            let encryptor = ConfigEncryption::new(key);
            let secret = "my_ultra_secret_password";
            let encrypted = encryptor.encrypt(secret).expect("Encryption failed");

            // 2. Test auto-decryption from environment variable
            temp_env::with_var("APP_DB_PASSWORD", Some(encrypted.as_str()), || {
                let config =
                    AppConfig::load().expect("Failed to load config with encrypted env var");
                assert_eq!(
                    config.db_password, secret,
                    "Environment variable should be auto-decrypted"
                );
            });

            // 3. Test auto-decryption from configuration file
            let dir = TempDir::new().unwrap();
            let file_path = dir.path().join("encrypted.toml");
            fs::write(&file_path, format!(r#"db_password = "{}""#, encrypted)).unwrap();

            let config = AppConfig::load_file(file_path)
                .load_sync()
                .expect("Failed to load config from encrypted file");
            assert_eq!(
                config.db_password, secret,
                "File value should be auto-decrypted"
            );
        },
    );
}

#[test]
fn test_load_from_encrypted_file() {
    use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
    use confers::encryption::ConfigEncryption;

    // Setup encryption key (32 bytes)
    let key = [1u8; 32]; // Use different key for this test
    let key_b64 = BASE64.encode(key);

    temp_env::with_vars([("CONFERS_ENCRYPTION_KEY", Some(key_b64.as_str()))], || {
        let encryptor = ConfigEncryption::new(key);
        let _secret_password = "ENC(BASE64_AQIDBAUGBwgJCgsMDQ4PEA==)"; // Simulate encrypted format
        let encrypted_value = encryptor
            .encrypt("my_secret_password")
            .expect("Encryption failed");

        // Create encrypted.toml file as specified in the documentation
        let dir = TempDir::new().unwrap();
        let encrypted_file_path = dir.path().join("encrypted.toml");

        let config_content = format!(
            r#"server_port = 8080
server_host = "localhost"  
db_password = "{}"
db_url = "postgres://localhost:5432/db"
"#,
            encrypted_value
        );

        fs::write(&encrypted_file_path, config_content)
            .expect("Failed to write encrypted config file");

        // Test AppConfig::load_from("encrypted.toml") as specified in documentation
        let config = AppConfig::load_file(encrypted_file_path.to_str().unwrap())
            .load_sync()
            .expect("Failed to load from encrypted.toml");

        assert_eq!(config.server_port, 8080);
        assert_eq!(config.server_host, "localhost");
        assert_eq!(
            config.db_password, "my_secret_password",
            "Password should be auto-decrypted"
        );
        assert_eq!(config.db_url, "postgres://localhost:5432/db");
    });
}

#[test]
fn test_validation_business_logic_errors() {
    temp_env::with_vars(
        [
            ("APP_SERVER_PORT", Some("65537")), // Exceeds u32 range (1-65535)
            ("APP_SERVER_HOST", Some("localhost")),
        ],
        || {
            println!(
                "DEBUG TEST: APP_SERVER_PORT={:?}",
                std::env::var("APP_SERVER_PORT")
            );
            let loader = AppConfig::new_loader().with_env_prefix("APP");
            let result = loader.load_sync();

            println!("DEBUG TEST: Raw result = {:?}", result);

            if let Ok(ref config) = result {
                println!("DEBUG TEST: Loaded config: {:?}", config);
                println!("DEBUG TEST: server_port value = {}", config.server_port);
            }

            // Range validation should fail because 65537 > 65535
            assert!(
                result.is_err(),
                "Loading should fail due to port value out of range (65537). Got: {:?}",
                result
            );
        },
    );

    temp_env::with_vars(
        [
            ("APP_SERVER_PORT", Some("-1")), // Invalid for u16
        ],
        || {
            let result = AppConfig::load_with_strict(true).load_sync();
            assert!(
                result.is_err(),
                "Loading should fail due to invalid port value (-1)"
            );
        },
    );

    temp_env::with_vars(
        [
            ("APP_SERVER_PORT", Some("0")), // Port 0 is invalid by range(min=1)
        ],
        || {
            let result = AppConfig::load_with_strict(true).load_sync();
            println!("DEBUG TEST: Port 0 result = {:?}", result);
            assert!(
                result.is_err(),
                "Loading should fail due to validation error (port 0)"
            );

            if let Err(confers::error::ConfigError::ValidationError(e)) = result {
                let err_str = format!("{:?}", e);
                assert!(
                    err_str.contains("server_port"),
                    "Error should mention server_port"
                );
            } else {
                panic!("Expected ValidationError, got {:?}", result.err());
            }
        },
    );

    temp_env::with_vars(
        [
            ("APP_SERVER_PORT", Some("8080")),
            ("TEST_COUNT", Some("150")), // For flattened structures, use flat field name
        ],
        || {
            println!("DEBUG TEST: TEST_COUNT={:?}", std::env::var("TEST_COUNT"));
            // Nested validation with strict mode
            let result = NestedConfig::load_with_strict(true)
                .with_env_prefix("TEST")
                .load_sync();
            println!("DEBUG TEST: Nested result = {:?}", result);
            assert!(
                result.is_err(),
                "Loading should fail due to nested validation error"
            );

            // Verify the error type and content for nested validation
            if let Err(confers::error::ConfigError::ValidationError(e)) = result {
                let err_str = format!("{:?}", e);
                assert!(err_str.contains("count"), "Error should mention count");
            } else {
                panic!(
                    "Expected ValidationError for nested field, got {:?}",
                    result.err()
                );
            }
        },
    );
}

#[test]
fn test_explicit_validator_triggered_errors() {
    // Test Case 1: Port validation - too low (below min=1)
    temp_env::with_vars([("APP_SERVER_PORT", Some("0"))], || {
        println!("DEBUG: Testing port 0 validation...");
        let result = AppConfig::load_sync();
        println!("DEBUG: Load result: {:?}", result);

        if let Ok(config) = &result {
            println!("DEBUG: Loaded config: {:?}", config);
            // Try manual validation
            match config.validate() {
                Ok(_) => println!("DEBUG: Manual validation passed"),
                Err(e) => println!("DEBUG: Manual validation failed: {:?}", e),
            }
        }

        assert!(
            result.is_err(),
            "Should fail due to port validation (port 0 < min=1)"
        );

        match result {
            Err(confers::error::ConfigError::ValidationError(ref e)) => {
                let error_msg = e.to_string();
                println!("Validation error message: {}", error_msg);
                assert!(
                    error_msg.contains("server_port") || error_msg.contains("port"),
                    "Error message should mention the field name"
                );
            }
            Err(other) => panic!("Expected ValidationError, got: {:?}", other),
            Ok(_) => panic!("Expected validation to fail but it succeeded"),
        }
    });

    // Test Case 2: Port validation - too high (above max=65535)
    temp_env::with_vars([("APP_SERVER_PORT", Some("70000"))], || {
        let result = AppConfig::load_sync();
        assert!(
            result.is_err(),
            "Should fail due to port validation (port 70000 > max=65535)"
        );

        match result {
            Err(confers::error::ConfigError::ValidationError(ref e)) => {
                let error_msg = e.to_string();
                println!("Validation error message: {}", error_msg);
                assert!(
                    error_msg.contains("server_port") || error_msg.contains("port"),
                    "Error message should mention the field name"
                );
            }
            Err(other) => panic!("Expected ValidationError, got: {:?}", other),
            Ok(_) => panic!("Expected validation to fail but it succeeded"),
        }
    });

    // Test Case 3: Nested validation - count out of range
    temp_env::with_vars(
        [
            ("TEST_COUNT", Some("200")), // Above max=100 - use flattened field name
            ("TEST_NAME", Some("test")),
        ],
        || {
            let loader = NestedConfig::new_loader()
                .with_env_prefix("TEST")
                .with_strict(true);
            println!("DEBUG: Loading config with env vars: TEST_COUNT=200, TEST_NAME=test");
            let result = loader.load_sync();
            println!("DEBUG: Nested validation result = {:?}", result);

            // Let's also try to validate manually
            let manual_validation_result = if let Ok(ref config) = result {
                println!("DEBUG: Loaded config: {:?}", config);
                match config.validate() {
                    Ok(_) => {
                        println!("DEBUG: Manual validation passed (should have failed!)");
                        None
                    }
                    Err(e) => {
                        println!("DEBUG: Manual validation failed: {:?}", e);
                        Some(e)
                    }
                }
            } else {
                None
            };

            assert!(
                result.is_err() || manual_validation_result.is_some(),
                "Should fail due to nested count validation (count 200 > max=100)"
            );

            match result {
                Err(confers::error::ConfigError::ValidationError(ref e)) => {
                    let error_msg = e.to_string();
                    println!("Nested validation error message: {}", error_msg);
                    assert!(
                        error_msg.contains("count"),
                        "Error message should mention the count field"
                    );
                }
                Err(other) => panic!("Expected ValidationError, got: {:?}", other),
                Ok(_) => panic!("Expected validation to fail but it succeeded"),
            }
        },
    );

    // Test Case 4: Nested validation - count at boundary (should succeed)
    temp_env::with_vars(
        [
            ("TEST_COUNT", Some("1")), // At min=1 boundary - should be valid
            ("TEST_NAME", Some("test")),
        ],
        || {
            let result = NestedConfig::new_loader()
                .with_env_prefix("TEST")
                .with_strict(true)
                .load_sync();
            assert!(
                result.is_ok(),
                "Should succeed with count at boundary (count 1 >= min=1)"
            );

            match result {
                Ok(config) => {
                    println!("Config loaded successfully: {:?}", config);
                    assert_eq!(config.details.count, 1);
                }
                Err(e) => panic!("Expected validation to succeed but it failed: {:?}", e),
            }
        },
    );
}

#[cfg(not(feature = "audit"))]
#[test]
fn test_audit_logs() {
    // This test simulates audit behavior by checking the Sanitize trait manually
    // Since 'audit' feature might be off, we rely on the trait being available
    // (which we saw in lib.rs is available even without 'audit' feature, just limited)

    use confers::audit::Sanitize;

    temp_env::with_vars(
        [
            ("APP_SERVER_PORT", None::<&str>),
            ("APP_SERVER_HOST", None::<&str>),
            ("APP_DB_PASSWORD", None::<&str>),
            ("APP_DB_URL", None::<&str>),
        ],
        || {
            let config = AppConfig::load_sync().expect("Failed to load config");
            let sanitized = config.sanitize();

            println!("Sanitized config: {:?}", sanitized);

            // Check if sensitive field is masked (partial masking with visible prefix)
            if let Some(obj) = sanitized.as_object() {
                if let Some(val) = obj.get("db_password") {
                    // Sanitize implementation uses visible_chars=4, so we check for partial mask
                    let val_str = val.as_str().expect("db_password should be a string");
                    assert!(
                        val_str.starts_with("secr") && val_str.contains('*'),
                        "Sensitive field 'db_password' should be partially masked, got: {}",
                        val_str
                    );
                } else {
                    panic!("db_password field missing in sanitized output");
                }

                // Check if non-sensitive field is present
                if let Some(val) = obj.get("server_port") {
                    assert_eq!(
                        val.as_u64(),
                        Some(8080),
                        "Non-sensitive field 'server_port' should be visible"
                    );
                } else {
                    panic!("server_port field missing in sanitized output");
                }
            } else {
                panic!("Sanitized output should be an object");
            }
        },
    );
}

#[test]
fn test_dynamic_template_injection() {
    use std::sync::atomic::{AtomicUsize, Ordering};

    // Use a unique file path to avoid conflicts in parallel execution
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let file_path = format!(
        "test_template_{}.toml",
        COUNTER.fetch_add(1, Ordering::SeqCst)
    );

    let config_content = r#"
        server_host = "127.0.0.1"
        server_port = 8080
        db_password = "password"
        db_url = "postgres://${DB_HOST}:5432/${DB_NAME}"
    "#;

    std::fs::write(&file_path, config_content).expect("Failed to write test file");

    // Use temp_env::with_vars to handle environment variables safely for parallel tests
    temp_env::with_vars(
        [
            ("APP_SERVER_PORT", None::<&str>),
            ("APP_SERVER_HOST", None::<&str>),
            ("DB_HOST", Some("localhost")),
            ("DB_NAME", Some("users")),
        ],
        || {
            // Use AppConfig::load_file to test template expansion
            let config = AppConfig::load_file(&file_path)
                .load_sync()
                .expect("Failed to load config");
            assert_eq!(config.db_url, "postgres://localhost:5432/users");
        },
    );

    std::fs::remove_file(&file_path).ok();
}

#[cfg(feature = "remote")]
#[tokio::test]
async fn test_remote_config_with_real_server() {
    use confers::providers::remote::http::HttpProvider;
    use serde_json::json;
    use warp::Filter;

    let port = 18710;
    let config_response = json!({
        "server_port": 9000,
        "server_host": "config-server.example",
        "db_password": "secure_db_password",
        "db_url": "postgres://production-db:5432/app"
    });

    let addr: SocketAddr = format!("127.0.0.1:{}", port).parse().unwrap();
    let routes = warp::path("config.json").map(move || warp::reply::json(&config_response));

    tokio::spawn(warp::serve(routes).run(addr));
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let server_url = format!("http://127.0.0.1:{}", port);

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let provider = HttpProvider::new(format!("{}/config.json", server_url));
    let figment = provider
        .load()
        .await
        .expect("Failed to load config from real server");

    let config = figment
        .extract::<AppConfig>()
        .expect("Failed to extract AppConfig");

    assert_eq!(config.server_port, 9000);
    assert_eq!(config.server_host, "config-server.example");
    assert_eq!(config.db_password, "secure_db_password");
    assert_eq!(config.db_url, "postgres://production-db:5432/app");
}

#[cfg(feature = "remote")]
#[tokio::test]
async fn test_remote_config_validation_error() {
    use confers::providers::remote::http::HttpProvider;
    use serde_json::json;
    use warp::Filter;

    let port = 18711;
    let invalid_config = json!({
        "server_port": 70000,
        "server_host": "test-server.example",
        "db_password": "test_password",
        "db_url": "postgres://test-db:5432/test"
    });

    let addr: SocketAddr = format!("127.0.0.1:{}", port).parse().unwrap();
    let routes = warp::path("invalid-config.json").map(move || warp::reply::json(&invalid_config));

    tokio::spawn(warp::serve(routes).run(addr));
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    let server_url = format!("http://127.0.0.1:{}", port);

    let provider = HttpProvider::new(format!("{}/invalid-config.json", server_url));

    let figment_result = provider.load().await;
    assert!(
        figment_result.is_ok(),
        "Should successfully load config from real server"
    );

    let figment = figment_result.unwrap();
    let config = figment
        .extract::<AppConfig>()
        .expect("Should extract AppConfig");

    let validation_result = config.validate();
    assert!(
        validation_result.is_err(),
        "Should fail due to invalid port validation (port 70000 > 65535)"
    );
}

#[cfg(feature = "remote")]
#[tokio::test]
async fn test_remote_config_server_error() {
    use confers::providers::remote::http::HttpProvider;
    use warp::Filter;

    let port = 18712;
    let addr: SocketAddr = format!("127.0.0.1:{}", port).parse().unwrap();
    let routes = warp::path("error-config.json").map(|| {
        warp::reply::with_status(
            "Internal Server Error",
            warp::http::StatusCode::INTERNAL_SERVER_ERROR,
        )
    });

    tokio::spawn(warp::serve(routes).run(addr));

    let server_url = format!("http://127.0.0.1:{}", port);

    let provider = HttpProvider::new(format!("{}/error-config.json", server_url));

    let result = provider.load().await;
    assert!(result.is_err(), "Should fail due to server error");
}
