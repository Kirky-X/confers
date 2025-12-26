use confers::Config;
use serde::{Deserialize, Serialize};

// === Structs ===

#[derive(Debug, Clone, Serialize, Deserialize, Config)]
#[config(validate)]
#[config(env_prefix = "APP_", format_detection = "Auto")]
pub struct BasicConfig {
    pub name: String,
    pub port: u16,
    pub debug: bool,
}

// === Main ===

fn main() -> anyhow::Result<()> {
    // 1. Initialize logging
    tracing_subscriber::fmt::init();

    // 2. Create example config file if not exists
    let config_content = r#"
name = "basic-example"
port = 8080
debug = true
tags = ["rust", "config", "example"]
"#;
    std::fs::write("examples/config.toml", config_content)?;

    // 3. Load configuration
    println!("Loading configuration...");
    let config = BasicConfig::load()?;

    // 4. Print configuration
    println!("Loaded configuration: {:#?}", config);

    // 5. Configuration is validated during load when validate = true
    println!("Configuration loaded successfully!");

    // 6. Demonstrate environment variable override
    println!("\nDemonstrating environment variable override...");
    std::env::set_var("APP_PORT", "9090");

    let config_with_env = BasicConfig::load()?;
    println!("Port after env override: {}", config_with_env.port);

    Ok(())
}
