// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! End-to-end regression test for nested table/object/mapping deserialization.
//!
//! Prior to fix-0.4.1, `convert.rs` used the dotted path (e.g.
//! "database.write_url") as the `ConfigValue::Map` key for nested tables /
//! objects / mappings. This broke `serde::Deserialize` for structs with nested
//! sub-structs because `serde_json::from_value` looks up fields by the bare
//! key ("write_url"), not the dotted path. This test verifies the fix by
//! loading real TOML / JSON / YAML files containing nested tables into a
//! typed `AppConfig` struct.

mod common;

use serde::Deserialize;
use std::io::Write;
use std::path::PathBuf;

use confers::ConfigBuilder;

/// Nested sub-struct — exercises the bug: serde must be able to find `host`
/// and `port` as bare keys inside the inner map, not as `database.host` /
/// `database.port` dotted keys.
#[derive(Debug, Default, PartialEq, Deserialize)]
struct DbConfig {
    host: String,
    port: u16,
}

#[derive(Debug, Default, PartialEq, Deserialize)]
struct AppConfig {
    database: DbConfig,
}

/// Create a temp config file in the current working directory (relative path)
/// so the loader's absolute-path security check does not reject it.
fn create_local_temp_config(content: &str, extension: &str) -> (tempfile::NamedTempFile, PathBuf) {
    let current_dir = std::env::current_dir().unwrap();
    let ext = extension.trim_start_matches('.');
    let mut file = tempfile::Builder::new()
        .suffix(&format!(".{ext}"))
        .tempfile_in(&current_dir)
        .unwrap();
    file.write_all(content.as_bytes()).unwrap();
    file.flush().unwrap();

    let absolute_path = file.path().to_path_buf();
    let relative_path = absolute_path
        .strip_prefix(&current_dir)
        .unwrap_or(&absolute_path)
        .to_path_buf();

    (file, relative_path)
}

fn assert_app_config(config: &AppConfig) {
    assert_eq!(
        config.database.host, "localhost",
        "nested database.host should deserialize to 'localhost'"
    );
    assert_eq!(
        config.database.port, 5432,
        "nested database.port should deserialize to 5432"
    );
}

#[cfg(feature = "toml")]
#[test]
fn test_nested_toml_deserializes_into_struct() {
    let content = r#"
[database]
host = "localhost"
port = 5432
"#;
    let (_file, path) = create_local_temp_config(content, ".toml");

    let config: AppConfig = ConfigBuilder::new()
        .file(&path)
        .build()
        .expect("TOML with nested [database] table should deserialize into AppConfig");

    assert_app_config(&config);
}

#[cfg(feature = "json")]
#[test]
fn test_nested_json_deserializes_into_struct() {
    let content = r#"{
  "database": {
    "host": "localhost",
    "port": 5432
  }
}"#;
    let (_file, path) = create_local_temp_config(content, ".json");

    let config: AppConfig = ConfigBuilder::new()
        .file(&path)
        .build()
        .expect("JSON with nested database object should deserialize into AppConfig");

    assert_app_config(&config);
}

#[cfg(feature = "yaml")]
#[test]
fn test_nested_yaml_deserializes_into_struct() {
    let content = r#"
database:
  host: localhost
  port: 5432
"#;
    let (_file, path) = create_local_temp_config(content, ".yaml");

    let config: AppConfig = ConfigBuilder::new()
        .file(&path)
        .build()
        .expect("YAML with nested database mapping should deserialize into AppConfig");

    assert_app_config(&config);
}
