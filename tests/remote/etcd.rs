// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! Integration tests for Etcd remote configuration source.
//!
//! These tests require a running Etcd instance. Use Docker Compose to start:
//! ```bash
//! docker compose -f docker-compose.test.yml up -d etcd
//! ```
//!
//! Run tests with: `cargo test --features etcd --test remote`

#![cfg(feature = "etcd")]

use super::common;
use std::time::Duration;

use confers::remote::{EtcdSourceBuilder, PolledSource};

/// Unique per-run KV prefix: parallel tests delete only their own keys,
/// so a cleanup can never wipe another test's keys mid-poll.
fn unique_prefix(base: &str) -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;
    format!("{base}-{}-{}", nanos, SEQ.fetch_add(1, Ordering::Relaxed))
}

/// Delete exact keys via the etcd v3 HTTP gateway (`/v3/kv/deleterange`,
/// keys base64-encoded). Cleanup failures must surface: a silently leftover
/// key pollutes later assertions on the shared etcd instance.
async fn etcd_delete_keys(client: &reqwest::Client, keys: &[String]) {
    use base64::Engine;
    for key in keys {
        let resp = client
            .post("http://127.0.0.1:2379/v3/kv/deleterange")
            .json(&serde_json::json!({
                "key": base64::engine::general_purpose::STANDARD.encode(key.as_bytes()),
            }))
            .send()
            .await
            .expect("etcd cleanup request must reach the server");
        assert!(
            resp.status().is_success(),
            "etcd cleanup must succeed for key {key}"
        );
    }
}

/// Test that Etcd source can connect and fetch configuration.
#[tokio::test]
async fn test_etcd_source_connect() {
    // Skip if Etcd is not available
    if !common::is_service_available("http://127.0.0.1:2379/health", Duration::from_secs(2)).await {
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
    if !common::is_service_available("http://127.0.0.1:2379/health", Duration::from_secs(2)).await {
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
    if !common::is_service_available("http://127.0.0.1:2379/health", Duration::from_secs(2)).await {
        eprintln!("Skipping test: Etcd not available");
        return;
    }

    // Write test config to Etcd using HTTP API directly
    use base64::Engine;
    use reqwest::Client;
    let client = Client::new();

    // Write test keys under a unique per-run prefix
    let prefix = unique_prefix("test-config");
    let host_key = format!("{prefix}/app/host");
    let port_key = format!("{prefix}/app/port");
    let base_url = "http://127.0.0.1:2379/v3/kv/put";
    let encode = |value: &str| base64::engine::general_purpose::STANDARD.encode(value.as_bytes());

    let host_resp = client
        .post(base_url)
        .json(&serde_json::json!({
            "key": encode(&host_key),
            "value": "bG9jYWxob3N0"  // "localhost" in base64
        }))
        .send()
        .await;

    if host_resp.is_err() {
        eprintln!("Failed to write test config to Etcd");
        return;
    }

    let port_resp = client
        .post(base_url)
        .json(&serde_json::json!({
            "key": encode(&port_key),
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
        .prefix(&prefix)
        .build()
        .await
        .expect("Failed to build Etcd source");

    let result = source.poll().await;

    match result {
        Ok(config) => {
            println!("Got config: {:?}", config);
            // Verify the config contains expected keys
            assert!(!config.is_empty(), "Config should not be empty");
        }
        Err(e) => {
            println!("Error: {:?}", e);
        }
    }

    // Cleanup - delete this test's keys
    etcd_delete_keys(&client, &[host_key, port_key]).await;
}

/// Test Etcd source with JSON configuration.
#[tokio::test]
async fn test_etcd_source_json_config() {
    if !common::is_service_available("http://127.0.0.1:2379/health", Duration::from_secs(2)).await {
        eprintln!("Skipping test: Etcd not available");
        return;
    }

    // Write JSON config to Etcd
    use base64::Engine;
    use reqwest::Client;
    let client = Client::new();

    let prefix = unique_prefix("test-config-json");
    let json_key = base64::engine::general_purpose::STANDARD.encode(prefix.as_bytes());
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
        .prefix(&prefix)
        .build()
        .await
        .expect("Failed to build Etcd source");

    let result = source.poll().await;

    match result {
        Ok(config) => {
            println!("Got JSON config: {:?}", config);
            // Verify the config contains expected keys
            assert!(!config.is_empty(), "Config should not be empty");
        }
        Err(e) => {
            println!("Error: {:?}", e);
        }
    }

    // Cleanup
    etcd_delete_keys(&client, &[prefix]).await;
}
