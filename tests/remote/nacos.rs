// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! Integration tests for the Nacos remote configuration source.
//!
//! These tests require a running Nacos instance. Use Docker Compose to start:
//! ```bash
//! docker compose -f docker-compose.test.yml up -d nacos
//! ```
//!
//! Run tests with: `cargo test --features nacos --test remote`

#![cfg(feature = "nacos")]

use super::common;
use std::time::Duration;

use confers::Format;
use confers::remote::{DEFAULT_NACOS_GROUP, NacosSourceBuilder, PolledSource};

const NACOS_URL: &str = "http://127.0.0.1:8848";
const READINESS_URL: &str = "http://127.0.0.1:8848/nacos/v1/console/health/readiness";

/// Unique per-run dataId/tenant: parallel tests never observe or clean up
/// each other's configs.
fn unique_name(base: &str) -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;
    format!("{base}-{nanos}-{}", SEQ.fetch_add(1, Ordering::Relaxed))
}

/// Skip guard shared by every case that needs the server.
async fn nacos_unavailable() -> bool {
    if common::is_service_available(READINESS_URL, Duration::from_secs(2)).await {
        return false;
    }
    eprintln!("Skipping test: Nacos not available");
    true
}

/// Percent-encode a form/query value. The workspace's reqwest is built
/// without the optional `form` feature, so the test bodies are assembled by
/// hand (same conservative encoding as the source's own Open API calls).
fn urlencode(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            other => out.push_str(&format!("%{other:02X}")),
        }
    }
    out
}

/// Publish a config through the Open API (the source reads it back through
/// the same endpoint), returning false when the server rejected the write.
async fn publish(data_id: &str, group: &str, tenant: Option<&str>, content: &str) -> bool {
    let mut body = format!(
        "dataId={}&group={}&content={}",
        urlencode(data_id),
        urlencode(group),
        urlencode(content)
    );
    if let Some(tenant) = tenant {
        body.push_str(&format!("&tenant={}", urlencode(tenant)));
    }
    let response = reqwest::Client::new()
        .post(format!("{NACOS_URL}/nacos/v1/cs/configs"))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await;
    matches!(response, Ok(resp) if resp.status().is_success())
}

/// Delete a config; failures are ignored (cleanup only).
async fn remove(data_id: &str, group: &str, tenant: Option<&str>) {
    let mut url = format!(
        "{NACOS_URL}/nacos/v1/cs/configs?dataId={}&group={}",
        urlencode(data_id),
        urlencode(group)
    );
    if let Some(tenant) = tenant {
        url.push_str(&format!("&tenant={}", urlencode(tenant)));
    }
    let _ = reqwest::Client::new().delete(url).send().await;
}

/// Poll until the source returns a value (Nacos needs a beat to make a
/// freshly published config visible) or the attempts run out.
async fn poll_until_visible(source: &impl PolledSource) -> Option<confers::types::AnnotatedValue> {
    for _ in 0..5 {
        if let Ok(value) = source.poll().await {
            return Some(value);
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    None
}

fn map_get<'a>(value: &'a confers::types::AnnotatedValue, key: &str) -> Option<&'a str> {
    value.inner.as_map()?.get(key)?.as_str()
}

/// `NacosSourceBuilder::new` → `build` → real `poll`: the published TOML body
/// is parsed into config keys and the Open API URL carries group/tenant.
#[tokio::test]
async fn test_nacos_source_connect_and_pull() {
    if nacos_unavailable().await {
        return;
    }

    let data_id = format!("{}.toml", unique_name("confers-it-pull"));
    assert!(
        publish(
            &data_id,
            DEFAULT_NACOS_GROUP,
            None,
            "log_level = \"trace\"\n"
        )
        .await,
        "publishing the test config must succeed"
    );

    let source = NacosSourceBuilder::new(NACOS_URL, &data_id)
        .interval(Duration::from_secs(5))
        .timeout(Duration::from_secs(5))
        .build()
        .expect("Failed to build Nacos source");

    assert!(
        source.url().contains("/nacos/v1/cs/configs?"),
        "Open API path: {}",
        source.url()
    );
    assert!(
        source.url().contains(&format!("dataId={data_id}")),
        "dataId on the query string: {}",
        source.url()
    );
    assert_eq!(source.poll_interval(), Some(Duration::from_secs(5)));

    let value = poll_until_visible(&source)
        .await
        .expect("published config must become visible to the source");
    assert_eq!(
        map_get(&value, "log_level"),
        Some("trace"),
        "TOML body must parse into config keys: {value:?}"
    );

    remove(&data_id, DEFAULT_NACOS_GROUP, None).await;
}

/// `group`/`namespace` actually scope the read: the same dataId under a
/// custom group + tenant is served, while a default-namespace source misses it.
#[tokio::test]
async fn test_nacos_source_group_and_namespace_scope() {
    if nacos_unavailable().await {
        return;
    }

    let data_id = format!("{}.toml", unique_name("confers-it-ns"));
    let group = "CONFERS_TEST_GROUP";
    let tenant = unique_name("confers-it-tenant");
    assert!(
        publish(&data_id, group, Some(&tenant), "scoped = true\n").await,
        "publishing into group+tenant must succeed"
    );

    let scoped = NacosSourceBuilder::new(NACOS_URL, &data_id)
        .group(group)
        .namespace(&tenant)
        .format(Format::Toml)
        .interval(Duration::from_secs(5))
        .timeout(Duration::from_secs(5))
        .build()
        .expect("Failed to build Nacos source");
    assert!(
        scoped.url().contains(&format!("group={group}")),
        "group on the query string: {}",
        scoped.url()
    );
    assert!(
        scoped.url().contains(&format!("tenant={tenant}")),
        "tenant on the query string: {}",
        scoped.url()
    );

    let value = poll_until_visible(&scoped)
        .await
        .expect("group+tenant scoped config must be visible");
    assert_eq!(
        value
            .inner
            .as_map()
            .and_then(|m| m.get("scoped"))
            .and_then(|v| v.as_bool()),
        Some(true),
        "scoped config parses: {value:?}"
    );

    // The same dataId read without the tenant must NOT surface the value:
    // proves the tenant segment is really applied (not silently ignored).
    let unscoped = NacosSourceBuilder::new(NACOS_URL, &data_id)
        .group(group)
        .timeout(Duration::from_secs(5))
        .build()
        .expect("Failed to build unscoped source");
    assert!(
        unscoped.poll().await.is_err(),
        "a config published under a tenant must not resolve from the public namespace"
    );

    remove(&data_id, group, Some(&tenant)).await;
}

/// Timed listening: a server-side publish is observed on the next poll.
#[tokio::test]
async fn test_nacos_source_picks_up_published_change() {
    if nacos_unavailable().await {
        return;
    }

    let data_id = format!("{}.toml", unique_name("confers-it-change"));
    assert!(publish(&data_id, DEFAULT_NACOS_GROUP, None, "replicas = 1\n").await);

    let source = NacosSourceBuilder::new(NACOS_URL, &data_id)
        .interval(Duration::from_secs(5))
        .timeout(Duration::from_secs(5))
        .build()
        .expect("Failed to build Nacos source");

    let before = poll_until_visible(&source)
        .await
        .expect("initial config must be visible");
    assert_eq!(
        before
            .inner
            .as_map()
            .and_then(|m| m.get("replicas"))
            .and_then(|v| v.as_i64()),
        Some(1)
    );

    assert!(publish(&data_id, DEFAULT_NACOS_GROUP, None, "replicas = 7\n").await);
    let mut updated = false;
    for _ in 0..5 {
        if let Ok(value) = source.poll().await {
            let replicas = value
                .inner
                .as_map()
                .and_then(|m| m.get("replicas"))
                .and_then(|v| v.as_i64());
            if replicas == Some(7) {
                updated = true;
                break;
            }
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    assert!(
        updated,
        "the update must reach the source on a later poll (timed listening)"
    );

    remove(&data_id, DEFAULT_NACOS_GROUP, None).await;
}

/// Explicit unreachable address → the poll reports an error instead of hanging
/// or inventing content.
#[tokio::test]
async fn test_nacos_source_unreachable_reports_error() {
    let source = NacosSourceBuilder::new("http://127.0.0.1:1", "app.toml")
        .timeout(Duration::from_secs(1))
        .build()
        .expect("Failed to build Nacos source");

    assert!(
        source.poll().await.is_err(),
        "an unreachable server must fail the poll"
    );
}
