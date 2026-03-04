//! Remote Consul Configuration Example
//!
//! This example demonstrates how to use confers to load remote configuration
//! from Consul KV Store.

use std::time::Duration;
use tokio::time::sleep;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .init();

    println!("========================================");
    println!("  Remote Consul Configuration Example");
    println!("========================================");

    // Get configuration from environment or use defaults
    let consul_address =
        std::env::var("CONSUL_ADDRESS").unwrap_or_else(|_| "127.0.0.1:8500".to_string());

    let consul_prefix = std::env::var("CONSUL_PREFIX").unwrap_or_else(|_| "myapp".to_string());

    println!("\nConsul Configuration:");
    println!("  Address: {}", consul_address);
    println!("  Prefix: {}", consul_prefix);

    // Test connection to Consul
    println!("\nTesting Consul connection...");

    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        test_consul_connection(&consul_address).await;
    });

    // If we got here, the example is complete
    println!("\n========================================");
    println!("  Example completed!");
    println!("========================================");

    Ok(())
}

async fn test_consul_connection(address: &str) {
    let url = if address.contains("://") {
        format!("{}/v1/status/leader", address)
    } else {
        format!("http://{}/v1/status/leader", address)
    };

    println!("  Testing URL: {}", url);

    match reqwest::get(&url).await {
        Ok(resp) => {
            if resp.status().is_success() {
                println!("  ✓ Consul connection successful!");

                // Try to read some values
                match read_consul_kv(address, "myapp/config").await {
                    Ok(values) => {
                        if values.is_empty() {
                            println!("  Note: No values found at prefix 'myapp/config'");
                            println!("  You can add values using:");
                            println!("    consul kv put myapp/config/database_url 'postgresql://localhost/mydb'");
                            println!("    consul kv put myapp/config/max_connections '20'");
                        } else {
                            println!("  ✓ Found {} configuration values:", values.len());
                            for (key, value) in &values {
                                println!("    {} = {}", key, value);
                            }
                        }
                    }
                    Err(e) => {
                        println!("  Warning: Could not read KV values: {}", e);
                    }
                }
            } else {
                println!("  ✗ Consul returned error: {}", resp.status());
            }
        }
        Err(e) => {
            println!("  ✗ Failed to connect to Consul: {}", e);
            println!("  Make sure Consul is running at {}", address);
        }
    }
}

async fn read_consul_kv(
    address: &str,
    prefix: &str,
) -> Result<Vec<(String, String)>, Box<dyn std::error::Error>> {
    let url = if address.contains("://") {
        format!("{}/v1/kv/{}?recurse=true", address, prefix)
    } else {
        format!("http://{}/v1/kv/{}?recurse=true", address, prefix)
    };

    let client = reqwest::Client::new();
    let resp = client.get(&url).send().await?;

    if !resp.status().is_success() {
        return Ok(Vec::new());
    }

    let kv_pairs: Vec<serde_json::Value> = resp.json().await?;
    let mut results = Vec::new();

    for item in kv_pairs {
        if let (Some(key), Some(value)) = (item.get("Key"), item.get("Value")) {
            let key_str = key.as_str().unwrap_or("");
            let value_str = value
                .as_str()
                .and_then(|s| base64::decode(s).ok())
                .and_then(|b| String::from_utf8(b).ok())
                .unwrap_or_default();

            // Remove prefix from key
            let short_key = key_str
                .strip_prefix(&format!("{}/", prefix))
                .unwrap_or(key_str)
                .to_string();

            results.push((short_key, value_str));
        }
    }

    Ok(results)
}
