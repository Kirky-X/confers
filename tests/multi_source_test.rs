use confers::{providers::cli_provider::CliConfigProvider, Config};
use std::fs;
use tempfile::TempDir;
use validator::Validate;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Config)]
#[config(env_prefix = "MULTI", validate)]
struct MultiSourceConfig {
    #[config(default = "8080", validate = "range(min = 1, max = 65535)")]
    server_port: u32,

    #[config(default = "\"localhost\".to_string()")]
    server_host: String,

    #[config(default = "\"info\".to_string()")]
    log_level: String,

    #[config(default = "1000")]
    max_connections: u32,

    cache: CacheConfig,

    auth: AuthConfig,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Config)]
#[config(validate)]
struct CacheConfig {
    #[config(default = "\"redis://localhost:6379\".to_string()")]
    redis_url: String,

    #[config(default = "60", validate = "range(min = 1, max = 3600)")]
    ttl_seconds: u32,

    #[config(default = "100", validate = "range(min = 1, max = 10000)")]
    max_size: u32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Config)]
#[config(validate)]
struct AuthConfig {
    #[config(default = "\"jwt\".to_string()")]
    auth_type: String,

    #[config(default = "3600", validate = "range(min = 60, max = 86400)")]
    token_expiry_seconds: u32,

    #[config(default = "false")]
    enable_refresh: bool,
}

#[test]
fn test_file_env_cli_priority_order() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("multi.toml");

    let file_config = r#"
        server_port = 8080
        server_host = "file-host"
        log_level = "debug"
        max_connections = 1000
        [cache]
        redis_url = "redis://file-redis:6379"
        ttl_seconds = 60
        max_size = 100
        [auth]
        auth_type = "oauth"
        token_expiry_seconds = 3600
        enable_refresh = false
    "#;

    fs::write(&config_path, file_config).unwrap();

    temp_env::with_vars(
        [
            ("MULTI_SERVER_PORT", Some("9090")),
            ("MULTI_SERVER_HOST", Some("env-host")),
            ("MULTI_LOG_LEVEL", Some("warn")),
            ("MULTI_CACHE__REDIS_URL", Some("redis://env-redis:6379")),
            ("MULTI_CACHE__TTL_SECONDS", Some("120")),
            ("MULTI_AUTH__AUTH_TYPE", Some("jwt")),
        ],
        || {
            let cli_provider = CliConfigProvider::from_args(vec![
                "server_port=9999",
                "server_host=cli-host",
                "log_level=error",
                "max_connections=2000",
                "cache.redis_url=redis://cli-redis:6379",
                "cache.ttl_seconds=180",
                "cache.max_size=200",
                "auth.auth_type=basic",
                "auth.token_expiry_seconds=7200",
                "auth.enable_refresh=true",
            ]);

            let config = MultiSourceConfig::load_file(config_path.clone())
                .with_cli_provider(cli_provider)
                .load_sync()
                .expect("Failed to load config from multiple sources");

            assert_eq!(config.server_port, 9999, "CLI should override env and file");
            assert_eq!(
                config.server_host, "cli-host",
                "CLI should override env and file"
            );
            assert_eq!(
                config.log_level, "error",
                "CLI should override env and file"
            );
            assert_eq!(config.max_connections, 2000, "CLI should override file");
            assert_eq!(
                config.cache.redis_url, "redis://cli-redis:6379",
                "CLI should override env and file"
            );
            assert_eq!(
                config.cache.ttl_seconds, 180,
                "CLI should override env and file"
            );
            assert_eq!(config.cache.max_size, 200, "CLI should override file");
            assert_eq!(
                config.auth.auth_type, "basic",
                "CLI should override env and file"
            );
            assert_eq!(
                config.auth.token_expiry_seconds, 7200,
                "CLI should override file"
            );
            assert!(config.auth.enable_refresh, "CLI should override file");
        },
    );
}

#[test]
fn test_partial_override_from_multiple_sources() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("partial.toml");

    let file_config = r#"
        server_port = 8080
        server_host = "localhost"
        log_level = "info"
        max_connections = 1000
        [cache]
        redis_url = "redis://localhost:6379"
        ttl_seconds = 60
        max_size = 100
        [auth]
        auth_type = "jwt"
        token_expiry_seconds = 3600
        enable_refresh = false
    "#;

    fs::write(&config_path, file_config).unwrap();

    temp_env::with_vars(
        [
            ("MULTI_SERVER_PORT", Some("9090")),
            ("MULTI_CACHE__REDIS_URL", Some("redis://env-redis:6379")),
            ("MULTI_AUTH__TOKEN_EXPIRY_SECONDS", Some("7200")),
        ],
        || {
            let config = MultiSourceConfig::load_file(config_path.clone())
                .load_sync()
                .expect("Failed to load config with partial overrides");

            assert_eq!(config.server_port, 9090, "Env should override file");
            assert_eq!(config.server_host, "localhost", "File value should be used");
            assert_eq!(config.log_level, "info", "File value should be used");
            assert_eq!(config.max_connections, 1000, "File value should be used");
            assert_eq!(
                config.cache.redis_url, "redis://env-redis:6379",
                "Env should override file"
            );
            assert_eq!(config.cache.ttl_seconds, 60, "File value should be used");
            assert_eq!(config.cache.max_size, 100, "File value should be used");
            assert_eq!(config.auth.auth_type, "jwt", "File value should be used");
            assert_eq!(
                config.auth.token_expiry_seconds, 7200,
                "Env should override file"
            );
            assert!(!config.auth.enable_refresh, "File value should be used");
        },
    );
}

#[test]
fn test_multiple_config_files_merge() {
    let dir = TempDir::new().unwrap();

    let base_config = dir.path().join("base.toml");
    let base_content = r#"
        server_port = 8080
        server_host = "localhost"
        log_level = "info"
        max_connections = 1000
        [cache]
        redis_url = "redis://localhost:6379"
        ttl_seconds = 60
        max_size = 100
        [auth]
        auth_type = "jwt"
        token_expiry_seconds = 3600
        enable_refresh = false
    "#;
    fs::write(&base_config, base_content).unwrap();

    let override_config = dir.path().join("override.toml");
    let override_content = r#"
        server_port = 9090
        log_level = "debug"
        redis_url = "redis://override-redis:6379"
        max_size = 200
        enable_refresh = true
    "#;
    fs::write(&override_config, override_content).unwrap();

    temp_env::with_vars(
        [
            ("MULTI_SERVER_PORT", None::<&str>),
            ("MULTI_SERVER_HOST", None::<&str>),
        ],
        || {
            let config = MultiSourceConfig::load_file(base_config.clone())
                .load_sync()
                .expect("Failed to load base config");

            assert_eq!(config.server_port, 8080);
            assert_eq!(config.log_level, "info");
            assert_eq!(config.cache.redis_url, "redis://localhost:6379");
            assert_eq!(config.cache.max_size, 100);
            assert!(!config.auth.enable_refresh);
        },
    );
}

#[test]
fn test_nested_env_override() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("nested.toml");

    let file_config = r#"
        server_port = 8080
        server_host = "localhost"
        log_level = "info"
        max_connections = 1000
        [cache]
        redis_url = "redis://localhost:6379"
        ttl_seconds = 60
        max_size = 100
        [auth]
        auth_type = "jwt"
        token_expiry_seconds = 3600
        enable_refresh = false
    "#;

    fs::write(&config_path, file_config).unwrap();

    temp_env::with_vars(
        [
            ("MULTI_SERVER_PORT", None::<&str>),
            ("MULTI_SERVER_HOST", None::<&str>),
            ("MULTI_CACHE__REDIS_URL", Some("redis://env-redis:6379")),
            ("MULTI_CACHE__TTL_SECONDS", Some("120")),
            ("MULTI_CACHE__MAX_SIZE", Some("200")),
            ("MULTI_AUTH__AUTH_TYPE", Some("oauth")),
            ("MULTI_AUTH__TOKEN_EXPIRY_SECONDS", Some("7200")),
            ("MULTI_AUTH__ENABLE_REFRESH", Some("true")),
        ],
        || {
            let config = MultiSourceConfig::load_file(config_path.clone())
                .load_sync()
                .expect("Failed to load config with nested env overrides");

            assert_eq!(config.server_port, 8080, "File value should be used");
            assert_eq!(config.server_host, "localhost", "File value should be used");
            assert_eq!(config.log_level, "info", "File value should be used");
            assert_eq!(config.max_connections, 1000, "File value should be used");
            assert_eq!(
                config.cache.redis_url, "redis://env-redis:6379",
                "Env should override file"
            );
            assert_eq!(config.cache.ttl_seconds, 120, "Env should override file");
            assert_eq!(config.cache.max_size, 200, "Env should override file");
            assert_eq!(config.auth.auth_type, "oauth", "Env should override file");
            assert_eq!(
                config.auth.token_expiry_seconds, 7200,
                "Env should override file"
            );
            assert!(config.auth.enable_refresh, "Env should override file");
        },
    );
}

#[test]
fn test_validation_with_multi_source() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("validation.toml");

    let file_config = r#"
        server_port = 8080
        server_host = "localhost"
        log_level = "info"
        max_connections = 1000
        [cache]
        redis_url = "redis://localhost:6379"
        ttl_seconds = 60
        max_size = 100
        [auth]
        auth_type = "jwt"
        token_expiry_seconds = 3600
        enable_refresh = false
    "#;

    fs::write(&config_path, file_config).unwrap();

    temp_env::with_vars(
        [
            ("MULTI_SERVER_PORT", Some("70000")),
            ("MULTI_CACHE__TTL_SECONDS", Some("5000")),
            ("MULTI_CACHE__MAX_SIZE", Some("20000")),
            ("MULTI_AUTH__TOKEN_EXPIRY_SECONDS", Some("100000")),
        ],
        || {
            let load_result = MultiSourceConfig::load_file(config_path.clone()).load_sync();

            assert!(
                load_result.is_err(),
                "Load should fail due to validation errors"
            );

            let error = load_result.unwrap_err();
            let error_str = error.to_string();

            assert!(
                error_str.contains("server_port")
                    || error_str.contains("ttl_seconds")
                    || error_str.contains("max_size")
                    || error_str.contains("token_expiry_seconds"),
                "Validation error should include invalid fields. Error: {}",
                error_str
            );
        },
    );
}

#[test]
fn test_default_values_with_partial_config() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("defaults.toml");

    let minimal_config = r#"
        server_port = 8080
    "#;

    fs::write(&config_path, minimal_config).unwrap();

    temp_env::with_vars(
        [
            ("MULTI_SERVER_PORT", None::<&str>),
            ("MULTI_SERVER_HOST", None::<&str>),
        ],
        || {
            let config = MultiSourceConfig::load_file(config_path.clone())
                .load_sync()
                .expect("Failed to load config with defaults");

            assert_eq!(config.server_port, 8080);
            assert_eq!(config.server_host, "localhost", "Default should be used");
            assert_eq!(config.log_level, "info", "Default should be used");
            assert_eq!(config.max_connections, 1000, "Default should be used");
            assert_eq!(
                config.cache.redis_url, "redis://localhost:6379",
                "Default should be used"
            );
            assert_eq!(config.cache.ttl_seconds, 60, "Default should be used");
            assert_eq!(config.cache.max_size, 100, "Default should be used");
            assert_eq!(config.auth.auth_type, "jwt", "Default should be used");
            assert_eq!(
                config.auth.token_expiry_seconds, 3600,
                "Default should be used"
            );
            assert!(!config.auth.enable_refresh, "Default should be used");

            config
                .validate()
                .expect("Config with defaults should validate");
        },
    );
}

#[test]
fn test_complex_nested_structure() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("complex.toml");

    let complex_config = r#"
        server_port = 8080
        server_host = "localhost"
        log_level = "info"
        max_connections = 1000
        [cache]
        redis_url = "redis://localhost:6379"
        ttl_seconds = 60
        max_size = 100
        [auth]
        auth_type = "jwt"
        token_expiry_seconds = 3600
        enable_refresh = false
    "#;

    fs::write(&config_path, complex_config).unwrap();

    temp_env::with_vars(
        [
            ("MULTI_SERVER_PORT", None::<&str>),
            ("MULTI_SERVER_HOST", None::<&str>),
            ("MULTI_CACHE__REDIS_URL", Some("redis://env-redis:6379")),
            ("MULTI_CACHE__TTL_SECONDS", Some("120")),
            ("MULTI_AUTH__AUTH_TYPE", Some("oauth")),
        ],
        || {
            let config = MultiSourceConfig::load_file(config_path.clone())
                .load_sync()
                .expect("Failed to load complex nested config");

            assert_eq!(config.server_port, 8080);
            assert_eq!(config.server_host, "localhost");
            assert_eq!(config.log_level, "info");
            assert_eq!(config.max_connections, 1000);
            assert_eq!(config.cache.redis_url, "redis://env-redis:6379");
            assert_eq!(config.cache.ttl_seconds, 120);
            assert_eq!(config.cache.max_size, 100);
            assert_eq!(config.auth.auth_type, "oauth");
            assert_eq!(config.auth.token_expiry_seconds, 3600);
            assert!(!config.auth.enable_refresh);

            config
                .validate()
                .expect("Complex nested config should validate");
        },
    );
}

#[test]
fn test_env_override_with_boolean() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("boolean.toml");

    let file_config = r#"
        server_port = 8080
        server_host = "localhost"
        log_level = "info"
        max_connections = 1000
        [cache]
        redis_url = "redis://localhost:6379"
        ttl_seconds = 60
        max_size = 100
        [auth]
        auth_type = "jwt"
        token_expiry_seconds = 3600
        enable_refresh = false
    "#;

    fs::write(&config_path, file_config).unwrap();

    temp_env::with_vars(
        [
            ("MULTI_SERVER_PORT", None::<&str>),
            ("MULTI_SERVER_HOST", None::<&str>),
            ("MULTI_AUTH__ENABLE_REFRESH", Some("true")),
        ],
        || {
            let config = MultiSourceConfig::load_file(config_path.clone())
                .load_sync()
                .expect("Failed to load config with boolean env override");

            assert_eq!(config.server_port, 8080);
            assert_eq!(config.server_host, "localhost");
            assert_eq!(config.log_level, "info");
            assert_eq!(config.max_connections, 1000);
            assert_eq!(config.cache.redis_url, "redis://localhost:6379");
            assert_eq!(config.cache.ttl_seconds, 60);
            assert_eq!(config.cache.max_size, 100);
            assert_eq!(config.auth.auth_type, "jwt");
            assert_eq!(config.auth.token_expiry_seconds, 3600);
            assert!(
                config.auth.enable_refresh,
                "Env should override file boolean"
            );
        },
    );
}

#[test]
fn test_empty_env_vars_fallback_to_file() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("empty_env.toml");

    let file_config = r#"
        server_port = 8080
        server_host = "localhost"
        log_level = "info"
        max_connections = 1000
        [cache]
        redis_url = "redis://localhost:6379"
        ttl_seconds = 60
        max_size = 100
        [auth]
        auth_type = "jwt"
        token_expiry_seconds = 3600
        enable_refresh = false
    "#;

    fs::write(&config_path, file_config).unwrap();

    temp_env::with_vars(
        [
            ("MULTI_SERVER_PORT", Some("")),
            ("MULTI_SERVER_HOST", Some("")),
            ("MULTI_LOG_LEVEL", Some("")),
            ("MULTI_CACHE__REDIS_URL", Some("")),
            ("MULTI_CACHE__TTL_SECONDS", Some("")),
            ("MULTI_CACHE__MAX_SIZE", Some("")),
            ("MULTI_AUTH__AUTH_TYPE", Some("")),
            ("MULTI_AUTH__TOKEN_EXPIRY_SECONDS", Some("")),
            ("MULTI_AUTH__ENABLE_REFRESH", None::<&str>),
        ],
        || {
            let config = MultiSourceConfig::load_file(config_path.clone())
                .load_sync()
                .expect("Failed to load config with empty env vars");

            assert_eq!(
                config.server_port, 8080,
                "File value should be used when env is empty"
            );
            assert_eq!(
                config.server_host, "localhost",
                "File value should be used when env is empty"
            );
            assert_eq!(
                config.log_level, "info",
                "File value should be used when env is empty"
            );
            assert_eq!(
                config.max_connections, 1000,
                "File value should be used when env is empty"
            );
            assert_eq!(
                config.cache.redis_url, "redis://localhost:6379",
                "File value should be used when env is empty"
            );
            assert_eq!(
                config.cache.ttl_seconds, 60,
                "File value should be used when env is empty"
            );
            assert_eq!(
                config.cache.max_size, 100,
                "File value should be used when env is empty"
            );
            assert_eq!(
                config.auth.auth_type, "jwt",
                "File value should be used when env is empty"
            );
            assert_eq!(
                config.auth.token_expiry_seconds, 3600,
                "File value should be used when env is empty"
            );
            assert!(
                !config.auth.enable_refresh,
                "File value should be used when env is empty"
            );
        },
    );
}

#[test]
fn test_special_characters_in_env_values() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("special.toml");

    let file_config = r#"
        server_port = 8080
        server_host = "localhost"
        log_level = "info"
        max_connections = 1000
        [cache]
        redis_url = "redis://localhost:6379"
        ttl_seconds = 60
        max_size = 100
        [auth]
        auth_type = "jwt"
        token_expiry_seconds = 3600
        enable_refresh = false
    "#;

    fs::write(&config_path, file_config).unwrap();

    temp_env::with_vars(
        [
            ("MULTI_SERVER_PORT", None::<&str>),
            ("MULTI_SERVER_HOST", None::<&str>),
            (
                "MULTI_CACHE__REDIS_URL",
                Some("redis://user:pass@host:6379/0"),
            ),
            ("MULTI_AUTH__AUTH_TYPE", Some("oauth2")),
        ],
        || {
            let config = MultiSourceConfig::load_file(config_path.clone())
                .load_sync()
                .expect("Failed to load config with special characters in env values");

            assert_eq!(config.server_port, 8080);
            assert_eq!(config.server_host, "localhost");
            assert_eq!(config.log_level, "info");
            assert_eq!(config.max_connections, 1000);
            assert_eq!(
                config.cache.redis_url, "redis://user:pass@host:6379/0",
                "Special characters should be preserved"
            );
            assert_eq!(config.cache.ttl_seconds, 60);
            assert_eq!(config.cache.max_size, 100);
            assert_eq!(
                config.auth.auth_type, "oauth2",
                "Special characters should be preserved"
            );
            assert_eq!(config.auth.token_expiry_seconds, 3600);
            assert!(!config.auth.enable_refresh);
        },
    );
}
