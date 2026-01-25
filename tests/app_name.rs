// Copyright (c) 2025 Kirky.X
//
// Licensed under MIT License
// See LICENSE file in the project root for full license information.

//! Tests for app_name functionality in configuration loading

#[cfg(test)]
mod app_name_tests {
    use confers::Config;
    use serde::{Deserialize, Serialize};
    use std::fs;
    use tempfile::TempDir;

    // Config without app_name
    #[derive(Debug, Clone, Serialize, Deserialize, Config)]
    pub struct NoAppNameConfig {
        pub name: String,
        pub port: u16,
        pub debug: bool,
    }

    // Config with app_name attribute
    #[derive(Debug, Clone, Serialize, Deserialize, Config)]
    #[config(app_name = "test-app")]
    pub struct WithAppNameConfig {
        pub name: String,
        pub port: u16,
    }

    // Multi-format config (reserved for future tests)
    #[derive(Debug, Clone, Serialize, Deserialize, Config)]
    #[allow(dead_code)]
    pub struct MultiFormatConfig {
        pub service: String,
        pub port: u16,
        pub enabled: bool,
    }

    // Config with env_prefix (reserved for future tests)
    #[derive(Debug, Clone, Serialize, Deserialize, Config)]
    #[config(env_prefix = "TEST")]
    #[allow(dead_code)]
    pub struct EnvOverrideConfig {
        pub name: String,
        pub port: u16,
    }

    // Test: load_file works regardless of app_name setting
    #[test]
    fn test_load_file_ignores_app_name() {
        let temp_dir = TempDir::new().unwrap();

        let custom_path = temp_dir.path().join("custom.toml");
        let config_content = r#"name = "custom-path"
port = 7777
debug = true
"#;
        fs::write(&custom_path, config_content).unwrap();

        let config = WithAppNameConfig::load_file(&custom_path)
            .load_sync()
            .unwrap();

        assert_eq!(config.name, "custom-path");
        assert_eq!(config.port, 7777);
    }

    // Test: load_sync equivalent to load
    #[test]
    fn test_load_sync_equivalent() {
        let temp_dir = TempDir::new().unwrap();
        let _original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        let config_content = r#"name = "sync-test"
port = 4444
debug = true
"#;
        fs::write("config.toml", config_content).unwrap();

        let async_config = NoAppNameConfig::load().unwrap();
        let sync_config = NoAppNameConfig::load_sync().unwrap();

        assert_eq!(async_config.name, sync_config.name);
        assert_eq!(async_config.port, sync_config.port);
        assert_eq!(async_config.debug, sync_config.debug);

        std::env::set_current_dir(&_original_dir).unwrap();
    }
}
