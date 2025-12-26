use confers::Config;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Config)]
#[config(env_prefix = "APP_", format_detection = "Auto")]
pub struct ValidationConfig {
    pub username: String,
    pub email: String,
    pub age: u32,
    pub website: String,
}

#[tokio::main]
async fn main() {
    // Create a temporary config file for demonstration
    let config_content = r#"
username = "rust_user"
email = "user@example.com"
age = 25
website = "https://github.com/example/confers"
tags = ["rust", "config"]
"#;
    std::fs::write("examples/config_validation.toml", config_content).unwrap();

    println!("--- Loading configuration with validation ---");

    // Load configuration - this will automatically perform validation
    match ValidationConfig::load() {
        Ok(config) => {
            println!("Configuration loaded successfully:");
            println!("  Username: {}", config.username);
            println!("  Email:    {}", config.email);
            println!("  Age:      {}", config.age);
            println!("  Website:  {}", config.website);
        }
        Err(e) => {
            eprintln!("Failed to load configuration: {}", e);
        }
    }

    // Example of invalid configuration
    println!("\n--- Loading invalid configuration ---");
    let invalid_content = r#"
username = "ru" # Too short
email = "not-an-email"
age = 15 # Too young
website = "not-a-url"
tags = []
"#;
    std::fs::write("examples/config_validation.toml", invalid_content).unwrap();

    match ValidationConfig::load() {
        Ok(_) => println!("Error: Invalid configuration should not have loaded successfully"),
        Err(e) => {
            println!("Successfully caught validation error:");
            println!("{}", e);
        }
    }

    // Cleanup
    let _ = std::fs::remove_file("examples/config_validation.toml");
}
