// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Example demonstrating config-rs compatible API
//!
//! This example shows how to use the ConfigBuilder API which is compatible
//! with config-rs, making migration from config-rs to confers much easier.

use confers::{ConfigBuilder, Environment, File};
use serde::{Deserialize, Serialize};
use std::error::Error;

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ServerConfig {
    host: String,
    port: u16,
    enable_port_detection: bool,
}

/// Database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DatabaseConfig {
    url: String,
    max_connections: Option<u32>,
    timeout: Option<u32>,
}

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
struct AppConfig {
    server: ServerConfig,
    database: DatabaseConfig,
    debug: bool,
}

fn main() -> Result<(), Box<dyn Error>> {
    println!("=== Config-rs Compatible API Example ===\n");

    // Example 1: Basic usage with defaults
    println!("Example 1: Basic usage with defaults");
    let config1: AppConfig = ConfigBuilder::new()
        .set_default("server.host", "0.0.0.0")?
        .set_default("server.port", 8899)?
        .set_default("server.enable_port_detection", true)?
        .set_default("database.url", "postgresql://localhost/mydb")?
        .set_default("database.max_connections", 100)?
        .set_default("database.timeout", 30)?
        .set_default("debug", false)?
        .build()?;

    println!("  Server: {}:{}", config1.server.host, config1.server.port);
    println!("  Database: {}", config1.database.url);
    println!("  Debug: {}\n", config1.debug);

    // Example 2: Using file source
    println!("Example 2: Using file source");
    // This will load from config/default.toml if it exists
    let config2: Result<AppConfig, Box<dyn Error>> = ConfigBuilder::new()
        .set_default("server.host", "0.0.0.0")?
        .set_default("server.port", 8899)?
        .set_default("server.enable_port_detection", true)?
        .set_default("database.url", "postgresql://localhost/mydb")?
        .set_default("database.max_connections", 100)?
        .set_default("database.timeout", 30)?
        .set_default("debug", false)?
        .add_source(File::with_name("config/default").required(false))
        .build()
        .map_err(|e| e.into());

    match config2 {
        Ok(config) => {
            println!("  Server: {}:{}", config.server.host, config.server.port);
            println!("  Database: {}", config.database.url);
        }
        Err(e) => {
            println!("  (File not found or error, using defaults)");
            println!("  Error: {}", e);
        }
    }
    println!();

    // Example 3: Using environment variables
    println!("Example 3: Using environment variables");
    // Set environment variables for demonstration
    std::env::set_var("APP_SERVER_PORT", "9000");
    std::env::set_var("APP_DATABASE_URL", "postgresql://remote-host/mydb");

    let config3: AppConfig = ConfigBuilder::new()
        .set_default("server.host", "0.0.0.0")?
        .set_default("server.port", 8899)?
        .set_default("server.enable_port_detection", true)?
        .set_default("database.url", "postgresql://localhost/mydb")?
        .set_default("database.max_connections", 100)?
        .set_default("database.timeout", 30)?
        .set_default("debug", false)?
        .add_source(Environment::with_prefix("APP").separator("_"))
        .build()?;

    println!("  Server: {}:{}", config3.server.host, config3.server.port);
    println!("  Database: {}", config3.database.url);

    // Clean up environment variables
    std::env::remove_var("APP_SERVER_PORT");
    std::env::remove_var("APP_DATABASE_URL");
    println!();

    // Example 4: Combining multiple sources
    println!("Example 4: Combining multiple sources");
    std::env::set_var("CRAWLRS_SERVER_PORT", "8080");

    let config4: AppConfig = ConfigBuilder::new()
        .set_default("server.host", "0.0.0.0")?
        .set_default("server.port", 8899)?
        .set_default("server.enable_port_detection", true)?
        .set_default("database.url", "postgresql://localhost/mydb")?
        .set_default("database.max_connections", 100)?
        .set_default("database.timeout", 30)?
        .set_default("debug", false)?
        .add_source(File::with_name("config/default").required(false))
        .add_source(Environment::with_prefix("CRAWLRS").separator("_"))
        .build()?;

    println!("  Server: {}:{}", config4.server.host, config4.server.port);
    println!("  Database: {}", config4.database.url);

    std::env::remove_var("CRAWLRS_SERVER_PORT");
    println!();

    // Example 5: Using double underscore separator (config-rs style)
    println!("Example 5: Using double underscore separator");
    std::env::set_var("APP__SERVER__PORT", "9999");
    std::env::set_var("APP__DATABASE__URL", "postgresql://double-underscore/mydb");

    let config5: AppConfig = ConfigBuilder::new()
        .set_default("server.host", "0.0.0.0")?
        .set_default("server.port", 8899)?
        .set_default("server.enable_port_detection", true)?
        .set_default("database.url", "postgresql://localhost/mydb")?
        .set_default("database.max_connections", 100)?
        .set_default("database.timeout", 30)?
        .set_default("debug", false)?
        .add_source(Environment::with_prefix("APP").separator("__"))
        .build()?;

    println!("  Server: {}:{}", config5.server.host, config5.server.port);
    println!("  Database: {}", config5.database.url);

    std::env::remove_var("APP__SERVER__PORT");
    std::env::remove_var("APP__DATABASE__URL");
    println!();

    println!("=== All examples completed successfully! ===");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_defaults() {
        let config: AppConfig = ConfigBuilder::new()
            .set_default("server.host", "localhost")?
            .set_default("server.port", 8080)?
            .set_default("database.url", "postgresql://localhost/test")?
            .set_default("debug", true)?
            .build()
            .unwrap();

        assert_eq!(config.server.host, "localhost");
        assert_eq!(config.server.port, 8080);
        assert_eq!(config.database.url, "postgresql://localhost/test");
        assert_eq!(config.debug, true);
    }

    #[test]
    fn test_nested_defaults() {
        let config: AppConfig = ConfigBuilder::new()
            .set_default("server.port", 9000)?
            .set_default("database.max_connections", 200)?
            .build()
            .unwrap();

        assert_eq!(config.server.port, 9000);
        assert_eq!(config.database.max_connections, Some(200));
    }

    #[test]
    fn test_environment_override() {
        std::env::set_var("TEST_SERVER_PORT", "7777");

        let config: AppConfig = ConfigBuilder::new()
            .set_default("server.host", "localhost")?
            .set_default("server.port", 8080)?
            .add_source(Environment::with_prefix("TEST").separator("_"))
            .build()
            .unwrap();

        assert_eq!(config.server.port, 7777);

        std::env::remove_var("TEST_SERVER_PORT");
    }

    #[test]
    fn test_double_underscore_separator() {
        std::env::set_var("TEST__SERVER__PORT", "6666");

        let config: AppConfig = ConfigBuilder::new()
            .set_default("server.port", 8080)?
            .add_source(Environment::with_prefix("TEST").separator("__"))
            .build()
            .unwrap();

        assert_eq!(config.server.port, 6666);

        std::env::remove_var("TEST__SERVER__PORT");
    }
}