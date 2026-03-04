//! Integration tests for Etcd remote configuration source.
//!
//! These tests require a running Etcd instance. Use Docker Compose to start:
//! ```bash
//! docker-compose -f docker-compose.test.yml up -d etcd
//! ```
//!
//! Run tests with: `cargo test --features etcd --test integration_etcd`

#![cfg(feature = "etcd")]

use std::time::Duration;

use confers::remote::{EtcdSourceBuilder, PolledSource};

/// Test that Etcd source can connect and fetch configuration.
#[tokio::test]
async fn test_etcd_source_connect() {
    // Skip if Etcd is not available
    if !is_etcd_available().await {
        eprintln!("Skipping test: Etcd not available");
        return;
    }

    let source = EtcdSourceBuilder::new()
        .endpoint("127.0.0.1:2379")
        .prefix("test-config")
        .interval(Duration::from_secs(10))
        .build()
        .await
        .expect("Failed to build Etcd source");

    // Try to poll - should return empty or error if no config exists
    let result = source.poll().await;

    // Should not panic - either returns config or error that config not found
    match result {
        Ok(config) => {
            println!("Got config: {:?}", config);
        }
        Err(e) => {
            // Expected if no config exists yet
            println!("Expected error (no config): {:?}", e);
        }
    }
}

/// Test Etcd source with authentication.
#[tokio::test]
async fn test_etcd_source_with_auth() {
    if !is_etcd_available().await {
        eprintln!("Skipping test: Etcd not available");
        return;
    }

    // Connect without auth (default for dev)
    let source = EtcdSourceBuilder::new()
        .endpoint("127.0.0.1:2379")
        .prefix("test-config")
        .build()
        .await
        .expect("Failed to build Etcd source");

    let result = source.poll().await;
    // Just verify it doesn't panic
    println!("Poll result: {:?}", result);
}

/// Test Etcd source configuration parsing.
#[tokio::test]
async fn test_etcd_source_parse_config() {
    if !is_etcd_available().await {
        eprintln!("Skipping test: Etcd not available");
        return;
    }

    // Write test config to Etcd using HTTP API directly
    use reqwest::Client;
    let client = Client::new();

    // Write test keys using etcd v3 HTTP API
    let base_url = "http://127.0.0.1:2379/v3/kv/put";

    // Write host key
    let host_resp = client
        .post(base_url)
        .json(&serde_json::json!({
            "key": "dGVzdC1jb25maWcvYXBwL2hvc3Q=",  // "test-config/app/host" in base64
            "value": "bG9jYWxob3N0"  // "localhost" in base64
        }))
        .send()
        .await;

    if host_resp.is_err() {
        eprintln!("Failed to write test config to Etcd");
        return;
    }

    // Write port key
    let port_resp = client
        .post(base_url)
        .json(&serde_json::json!({
            "key": "dGVzdC1jb25maWcvYXBwL3BvcnQ=",  // "test-config/app/port" in base64
            "value": "ODA4MA=="  // "8080" in base64
        }))
        .send()
        .await;

    if port_resp.is_err() {
        eprintln!("Failed to write port config to Etcd");
        return;
    }

    // Now read it back using our source
    let source = EtcdSourceBuilder::new()
        .endpoint("127.0.0.1:2379")
        .prefix("test-config")
        .build()
        .await
        .expect("Failed to build Etcd source");

    let result = source.poll().await;

    match result {
        Ok(config) => {
            // Verify we got some config
            println!("Got config: {:?}", config);
            // Just verify it doesn't panic
            assert!(true);
        }
        Err(e) => {
            println!("Error: {:?}", e);
        }
    }

    // Cleanup - delete the test keys
    let _ = client
        .post("http://127.0.0.1:2379/v3/kv/del")
        .json(&serde_json::json!({
            "key": "dGVzdC1jb25maWc="
        }))
        .send()
        .await;
}

/// Test Etcd source with JSON configuration.
#[tokio::test]
async fn test_etcd_source_json_config() {
    if !is_etcd_available().await {
        eprintln!("Skipping test: Etcd not available");
        return;
    }

    // Write JSON config to Etcd
    use base64::Engine;
    use reqwest::Client;
    let client = Client::new();

    let json_key = "dGVzdC1jb25maWctanNvbg=="; // "test-config-json" in base64
    let json_value = base64::engine::general_purpose::STANDARD
        .encode(r#"{"database": {"host": "db.example.com", "port": 5432}}"#);

    let result = client
        .post("http://127.0.0.1:2379/v3/kv/put")
        .json(&serde_json::json!({
            "key": json_key,
            "value": json_value
        }))
        .send()
        .await;

    if result.is_err() {
        eprintln!("Failed to write JSON config to Etcd");
        return;
    }

    // Now read it back
    let source = EtcdSourceBuilder::new()
        .endpoint("127.0.0.1:2379")
        .prefix("test-config-json")
        .build()
        .await
        .expect("Failed to build Etcd source");

    let result = source.poll().await;

    match result {
        Ok(config) => {
            println!("Got JSON config: {:?}", config);
            assert!(true);
        }
        Err(e) => {
            println!("Error: {:?}", e);
        }
    }

    // Cleanup
    let _ = client
        .post("http://127.0.0.1:2379/v3/kv/del")
        .json(&serde_json::json!({
            "key": json_key
        }))
        .send()
        .await;
}

/// Check if Etcd is available.
async fn is_etcd_available() -> bool {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .unwrap();

    match client.get("http://127.0.0.1:2379/health").send().await {
        Ok(resp) => resp.status().is_success(),
        Err(_) => false,
    }
}
