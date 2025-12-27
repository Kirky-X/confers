// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

use confers::Config;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Config)]
#[config(env_prefix = "APP_", format_detection = "Auto")]
pub struct ValidationConfig {
    #[config(
        validate = "length(min = 3, max = 20)",
        description = "用户名，长度3-20字符"
    )]
    pub username: String,

    #[config(validate = "email", description = "有效的邮箱地址")]
    pub email: String,

    #[config(
        validate = "range(min = 18, max = 120)",
        description = "年龄，范围18-120"
    )]
    pub age: u32,

    #[config(validate = "url", description = "有效的URL地址")]
    pub website: String,
}

fn main() {
    let config_content = r#"username = "rust_user"
email = "user@example.com"
age = 25
website = "https://github.com/example/confers"
"#;
    std::fs::create_dir_all("confers").ok();
    std::fs::write("confers/config.toml", config_content).unwrap();

    println!("--- Loading valid configuration ---");

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

    println!("\n--- Testing username validation (too short) ---");
    let invalid_content = r#"username = "ru"
email = "user@example.com"
age = 25
website = "https://example.com"
"#;
    std::fs::create_dir_all("confers").ok();
    std::fs::write("confers/config.toml", invalid_content).unwrap();

    match ValidationConfig::load() {
        Ok(_) => println!("Error: Should have failed validation"),
        Err(e) => println!("Caught error: {}", e),
    }

    println!("\n--- Testing email validation (invalid email) ---");
    let invalid_content = r#"username = "rust_user"
email = "not-an-email"
age = 25
website = "https://example.com"
"#;
    std::fs::write("confers/config.toml", invalid_content).unwrap();

    match ValidationConfig::load() {
        Ok(_) => println!("Error: Should have failed validation"),
        Err(e) => println!("Caught error: {}", e),
    }

    println!("\n--- Testing age validation (out of range) ---");
    let invalid_content = r#"username = "rust_user"
email = "user@example.com"
age = 15
website = "https://example.com"
"#;
    std::fs::write("confers/config.toml", invalid_content).unwrap();

    match ValidationConfig::load() {
        Ok(_) => println!("Error: Should have failed validation"),
        Err(e) => println!("Caught error: {}", e),
    }

    println!("\n--- Testing URL validation (invalid URL) ---");
    let invalid_content = r#"username = "rust_user"
email = "user@example.com"
age = 25
website = "not-a-url"
"#;
    std::fs::write("confers/config.toml", invalid_content).unwrap();

    match ValidationConfig::load() {
        Ok(_) => println!("Error: Should have failed validation"),
        Err(e) => println!("Caught error: {}", e),
    }

    let _ = std::fs::remove_file("confers/config.toml");
    let _ = std::fs::remove_dir("confers");
}
