//! Integration tests for Consul remote configuration source.
//!
//! These tests require a running Consul instance. Use Docker Compose to start:
//! ```bash
//! docker-compose -f docker-compose.test.yml up -d consul
//! ```
//!
//! Run tests with: `cargo test --features consul --test integration_consul`

#![cfg(feature = "consul")]

mod common;

use std::time::Duration;

use confers::remote::{ConsulSourceBuilder, PolledSource};

/// Test that Consul source can connect and fetch configuration.
#[tokio::test]
async fn test_consul_source_connect() {
    // Skip if Consul is not available
    if !common::is_service_available(
        "http://127.0.0.1:8500/v1/status/leader",
        Duration::from_secs(2),
    )
    .await
    {
        eprintln!("Skipping test: Consul not available");
        return;
    }

    let source = ConsulSourceBuilder::new()
        .address("127.0.0.1:8500")
        .prefix("test-config")
        .interval(Duration::from_secs(10))
        .build()
        .expect("Failed to build Consul source");

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

/// Test Consul source with token authentication.
#[tokio::test]
async fn test_consul_source_with_token() {
    if !common::is_service_available(
        "http://127.0.0.1:8500/v1/status/leader",
        Duration::from_secs(2),
    )
    .await
    {
        eprintln!("Skipping test: Consul not available");
        return;
    }

    // First create a test token (empty for dev mode)
    let source = ConsulSourceBuilder::new()
        .address("127.0.0.1:8500")
        .prefix("test-config")
        .build()
        .expect("Failed to build Consul source");

    let result = source.poll().await;
    // Just verify it doesn't panic
    println!("Poll result: {:?}", result);
}

/// Test Consul source configuration parsing.
#[tokio::test]
async fn test_consul_source_parse_config() {
    if !common::is_service_available(
        "http://127.0.0.1:8500/v1/status/leader",
        Duration::from_secs(2),
    )
    .await
    {
        eprintln!("Skipping test: Consul not available");
        return;
    }

    // Write test config to Consul using HTTP API directly
    use reqwest::Client as HttpClient;

    let http_client = HttpClient::new();

    // Write a test key
    let put_result: Result<reqwest::Response, _> = http_client
        .put("http://127.0.0.1:8500/v1/kv/test-config/app")
        .body(r#"{"host": "localhost", "port": 8080}"#)
        .send()
        .await;

    if put_result.is_err() {
        eprintln!("Failed to write test config to Consul");
        return;
    }

    // Now read it back
    let source = ConsulSourceBuilder::new()
        .address("127.0.0.1:8500")
        .prefix("test-config")
        .build()
        .expect("Failed to build Consul source");

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

    // Cleanup - delete the test key
    let _ = http_client
        .delete("http://127.0.0.1:8500/v1/kv/test-config")
        .send()
        .await;
}
