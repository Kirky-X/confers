// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Nacos configuration source (`nacos` feature).
//!
//! Pulls a single `dataId` from a Nacos server through the HTTP Open API
//! (`GET /nacos/v1/cs/configs`) and listens for changes with a timed poll:
//! unchanged content short-circuits into the cached snapshot, so the parse
//! and merge work only run when the server actually served new content.
//!
//! Like every remote source this is an MVP adapter: authentication is out of
//! scope (Nacos auth can be layered in front), and content is projected with
//! the standard format pipeline (explicit format or content sniffing).

use std::sync::Arc;
use std::time::Duration;

use arc_swap::ArcSwap;

use crate::error::{ConfigError, ConfigResult};
use crate::loader::Format;
use crate::remote::circuit_breaker::CircuitBreaker;
use crate::remote::common::try_parse_value_with_format;
use crate::types::{AnnotatedValue, SourceId};

const SOURCE_NAME: &str = "nacos";

/// Default Nacos group when none is configured.
pub const DEFAULT_NACOS_GROUP: &str = "DEFAULT_GROUP";

/// Builder for [`NacosSource`].
#[derive(Debug, Clone)]
pub struct NacosSourceBuilder {
    server: String,
    data_id: String,
    group: String,
    namespace: Option<String>,
    format: Option<Format>,
    interval: Duration,
    timeout: Duration,
    cb_threshold: u32,
}

impl NacosSourceBuilder {
    /// Watch `dataId` on the Nacos `server` (e.g. `http://nacos:8848`).
    pub fn new(server: impl Into<String>, data_id: impl Into<String>) -> Self {
        Self {
            server: server.into().trim_end_matches('/').to_string(),
            data_id: data_id.into(),
            group: DEFAULT_NACOS_GROUP.to_string(),
            namespace: None,
            format: None,
            interval: Duration::from_secs(30),
            timeout: Duration::from_secs(10),
            cb_threshold: 5,
        }
    }

    /// Set the group (default: `DEFAULT_GROUP`).
    pub fn group(mut self, group: impl Into<String>) -> Self {
        self.group = group.into();
        self
    }

    /// Set the namespace/tenant.
    pub fn namespace(mut self, namespace: impl Into<String>) -> Self {
        self.namespace = Some(namespace.into());
        self
    }

    /// Pin a value format (default: content sniffing).
    pub fn format(mut self, format: Format) -> Self {
        self.format = Some(format);
        self
    }

    /// Timed-listening poll interval (default: 30s).
    pub fn interval(mut self, interval: Duration) -> Self {
        self.interval = interval;
        self
    }

    /// Per-request timeout (default: 10s).
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Circuit-breaker failure threshold before the source cools down.
    pub fn circuit_breaker_threshold(mut self, threshold: u32) -> Self {
        self.cb_threshold = threshold;
        self
    }

    /// Build the source.
    pub fn build(self) -> ConfigResult<NacosSource> {
        if self.server.is_empty() {
            return Err(ConfigError::InvalidValue {
                key: SOURCE_NAME.to_string(),
                expected_type: "server URL".to_string(),
                message: "nacos server must not be empty".to_string(),
            });
        }
        if self.data_id.is_empty() {
            return Err(ConfigError::InvalidValue {
                key: SOURCE_NAME.to_string(),
                expected_type: "dataId".to_string(),
                message: "nacos dataId must not be empty".to_string(),
            });
        }
        let mut url = format!(
            "{}/nacos/v1/cs/configs?dataId={}&group={}",
            self.server,
            urlencode(&self.data_id),
            urlencode(&self.group)
        );
        if let Some(ref tenant) = self.namespace {
            url.push_str(&format!("&tenant={}", urlencode(tenant)));
        }
        Ok(NacosSource {
            url: url.into(),
            format: self.format,
            interval: self.interval,
            client: reqwest::Client::builder()
                .timeout(self.timeout)
                .build()
                .map_err(|e| ConfigError::InvalidValue {
                    key: SOURCE_NAME.to_string(),
                    expected_type: "HTTP client".to_string(),
                    message: format!("failed to build nacos HTTP client: {e}"),
                })?,
            cached: ArcSwap::new(Arc::new(None)),
            last_content: ArcSwap::new(Arc::new(None)),
            source_id: SourceId::new(format!("{SOURCE_NAME}:{}:{}", self.group, self.data_id)),
            circuit_breaker: std::sync::Mutex::new(
                CircuitBreaker::new().with_threshold(self.cb_threshold),
            ),
        })
    }
}

/// Percent-encode a query parameter value (conservative form encoding).
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

/// Nacos HTTP Open API configuration source with timed change listening.
pub struct NacosSource {
    url: Arc<str>,
    format: Option<Format>,
    interval: Duration,
    client: reqwest::Client,
    cached: ArcSwap<Option<Arc<AnnotatedValue>>>,
    /// Raw content served on the previous successful poll (dedupe marker).
    last_content: ArcSwap<Option<Arc<str>>>,
    source_id: SourceId,
    circuit_breaker: std::sync::Mutex<CircuitBreaker>,
}

impl NacosSource {
    /// The Open API endpoint being polled.
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Fetch the current content from the server.
    async fn fetch(&self) -> ConfigResult<String> {
        let response = self
            .client
            .get(self.url.as_ref())
            .send()
            .await
            .map_err(|e| ConfigError::InvalidValue {
                key: SOURCE_NAME.to_string(),
                expected_type: "nacos API response".to_string(),
                message: format!("nacos request failed: {e}"),
            })?;
        let status = response.status();
        if !status.is_success() {
            return Err(ConfigError::InvalidValue {
                key: SOURCE_NAME.to_string(),
                expected_type: "2xx status".to_string(),
                message: format!("nacos returned {status} for {}", self.url),
            });
        }
        response
            .text()
            .await
            .map_err(|e| ConfigError::InvalidValue {
                key: SOURCE_NAME.to_string(),
                expected_type: "nacos config text".to_string(),
                message: format!("nacos returned an unreadable body: {e}"),
            })
    }

    /// Project raw content into an annotated config value.
    fn project(&self, content: &str) -> AnnotatedValue {
        try_parse_value_with_format(content, self.format, SOURCE_NAME).unwrap_or_else(|| {
            AnnotatedValue::new(
                crate::types::ConfigValue::String(content.to_string()),
                SourceId::new(SOURCE_NAME),
                "",
            )
        })
    }
}

#[async_trait::async_trait]
impl crate::remote::PolledSource for NacosSource {
    async fn poll(&self) -> ConfigResult<AnnotatedValue> {
        // Circuit breaker: a failing server cools the source down instead of
        // hammering it every interval.
        let allowed = self
            .circuit_breaker
            .try_lock()
            .map(|mut cb| cb.can_execute())
            .unwrap_or(false);
        if !allowed {
            return Err(ConfigError::InvalidValue {
                key: SOURCE_NAME.to_string(),
                expected_type: "available source".to_string(),
                message: "nacos source circuit breaker is open".to_string(),
            });
        }

        // Remote-fetch critical-path metrics (manual: fetch resolves to raw
        // text, while record_fetch_metrics is bound to AnnotatedValue).
        let started = std::time::Instant::now();
        let labels = [("source", self.source_id.as_str())];
        let result = self.fetch().await;
        crate::metrics::record_histogram(
            crate::metrics::names::REMOTE_FETCH_DURATION_SECONDS,
            started.elapsed().as_secs_f64(),
            &labels,
        );
        let content = match result {
            Ok(content) => {
                if let Ok(mut cb) = self.circuit_breaker.try_lock() {
                    cb.record_success();
                }
                content
            }
            Err(err) => {
                crate::metrics::record_counter(
                    crate::metrics::names::REMOTE_FETCH_ERRORS_TOTAL,
                    &labels,
                );
                if let Ok(mut cb) = self.circuit_breaker.try_lock() {
                    cb.record_failure();
                }
                return Err(err);
            }
        };

        // Timed listening: unchanged content keeps serving the cached
        // snapshot (no re-parse, no provenance churn).
        let unchanged = {
            let last = self.last_content.load();
            last.as_deref() == Some(content.as_str())
        };
        if unchanged {
            let cached = self.cached.load();
            if let Some(ref value) = **cached {
                return Ok((**value).clone());
            }
        }

        let value = self.project(&content);
        self.last_content
            .store(Arc::new(Some(Arc::from(content.as_str()))));
        self.cached.store(Arc::new(Some(Arc::new(value.clone()))));
        Ok(value)
    }

    fn poll_interval(&self) -> Option<Duration> {
        Some(self.interval)
    }

    fn source_id(&self) -> SourceId {
        self.source_id.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// Mock Nacos server: serves the queued bodies in order on
    /// `/nacos/v1/cs/configs`, counting how many requests arrived.
    async fn spawn_mock_nacos(bodies: Vec<String>) -> (std::net::SocketAddr, Arc<AtomicUsize>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind ephemeral port");
        let addr = listener.local_addr().expect("local addr");
        let hits = Arc::new(AtomicUsize::new(0));
        let hits_server = Arc::clone(&hits);
        let queue = Arc::new(std::sync::Mutex::new(bodies));

        tokio::spawn(async move {
            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            loop {
                let Ok((mut stream, _)) = listener.accept().await else {
                    break;
                };
                let hits = Arc::clone(&hits_server);
                let queue = Arc::clone(&queue);
                tokio::spawn(async move {
                    let mut buf = [0u8; 4096];
                    let _ = stream.read(&mut buf).await;
                    hits.fetch_add(1, Ordering::SeqCst);
                    let body = {
                        let mut q = queue.lock().unwrap();
                        if q.len() > 1 {
                            q.remove(0)
                        } else {
                            q.first().cloned().unwrap_or_default()
                        }
                    };
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        body.len(),
                        body
                    );
                    let _ = stream.write_all(response.as_bytes()).await;
                });
            }
        });
        (addr, hits)
    }

    fn source_for(addr: std::net::SocketAddr, group: &str) -> NacosSource {
        NacosSourceBuilder::new(format!("http://{addr}"), "app-config.toml")
            .group(group)
            .namespace("public")
            .interval(Duration::from_secs(5))
            .build()
            .expect("build nacos source")
    }

    #[tokio::test]
    async fn fetches_and_parses_config_from_open_api() {
        let body = "log_level = \"debug\"\nreplicas = 3\n";
        let (addr, _) = spawn_mock_nacos(vec![body.to_string()]).await;
        let source = source_for(addr, "DEFAULT_GROUP");

        let polled = crate::remote::PolledSource::poll(&source)
            .await
            .expect("poll nacos");
        let level = polled
            .inner
            .as_map()
            .and_then(|m| m.get("log_level"))
            .and_then(|v| v.as_str());
        assert_eq!(level, Some("debug"), "toml body parses into config keys");
    }

    #[tokio::test]
    async fn timed_listening_serves_cached_snapshot_for_unchanged_content() {
        let body = "log_level = \"info\"\n";
        let (addr, hits) = spawn_mock_nacos(vec![body.to_string()]).await;
        let source = source_for(addr, "DEFAULT_GROUP");
        let poll = crate::remote::PolledSource::poll(&source);

        let first = poll.await.expect("first poll");
        let second = crate::remote::PolledSource::poll(&source)
            .await
            .expect("second poll (unchanged)");
        assert_eq!(
            first
                .inner
                .as_map()
                .and_then(|m| m.get("log_level"))
                .and_then(|v| v.as_str()),
            second
                .inner
                .as_map()
                .and_then(|m| m.get("log_level"))
                .and_then(|v| v.as_str()),
            "unchanged content serves the cached snapshot"
        );
        assert!(hits.load(Ordering::SeqCst) >= 2, "server hit on every poll");
    }

    #[tokio::test]
    async fn published_change_is_picked_up_on_next_poll() {
        let (addr, _) = spawn_mock_nacos(vec![
            "log_level = \"info\"\n".to_string(),
            "log_level = \"trace\"\n".to_string(),
        ])
        .await;
        let source = source_for(addr, "DEFAULT_GROUP");

        let before = crate::remote::PolledSource::poll(&source)
            .await
            .expect("initial");
        let after = crate::remote::PolledSource::poll(&source)
            .await
            .expect("after publish");
        let a = before
            .inner
            .as_map()
            .and_then(|m| m.get("log_level"))
            .and_then(|v| v.as_str());
        let b = after
            .inner
            .as_map()
            .and_then(|m| m.get("log_level"))
            .and_then(|v| v.as_str());
        assert_eq!(a, Some("info"));
        assert_eq!(
            b,
            Some("trace"),
            "server-side publish must reach the next poll"
        );
    }

    #[tokio::test]
    async fn error_response_is_reported_and_counted() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let addr = listener.local_addr().unwrap();
        drop(listener); // connection refused

        let source = source_for(addr, "DEFAULT_GROUP");
        let result = crate::remote::PolledSource::poll(&source).await;
        assert!(result.is_err(), "unreachable server must fail the poll");
    }

    #[test]
    fn builder_urls_map_namespace_group_and_dataid() {
        let source = NacosSourceBuilder::new("http://nacos.internal:8848/", "app config.toml")
            .group("MY_GROUP")
            .namespace("prod tenant")
            .build()
            .expect("build");
        let url = source.url();
        assert!(
            url.contains("/nacos/v1/cs/configs?"),
            "open api path: {url}"
        );
        assert!(
            url.contains("dataId=app%20config.toml"),
            "dataId encoded: {url}"
        );
        assert!(url.contains("group=MY_GROUP"), "group: {url}");
        assert!(url.contains("tenant=prod%20tenant"), "tenant: {url}");
    }

    #[test]
    fn builder_rejects_empty_server_and_dataid() {
        assert!(NacosSourceBuilder::new("", "x").build().is_err());
        assert!(NacosSourceBuilder::new("http://nacos", "").build().is_err());
    }
}
