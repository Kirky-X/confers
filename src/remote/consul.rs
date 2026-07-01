//! Consul remote configuration source.
//!
//! This module provides a Consul-backed implementation of the `PolledSource` trait,
//! using the Consul KV REST API via reqwest.

use super::common::{merge_into_map, try_parse_value};
use crate::error::{ConfigError, ConfigResult};
use crate::loader::Format;
use crate::types::{AnnotatedValue, SourceId};
use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use std::sync::Arc;
use std::time::Duration;

/// Default poll interval for Consul (30 seconds).
pub const DEFAULT_CONSUL_POLL_INTERVAL: Duration = Duration::from_secs(30);

/// Consul KV response entry.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct KvResponse {
    #[serde(default)]
    value: Option<String>,
    #[serde(default)]
    modify_index: Option<u64>,
}

/// Builder for creating Consul configuration sources.
pub struct ConsulSourceBuilder {
    address: String,
    token: Option<String>,
    prefix: String,
    format: Option<Format>,
    interval: Option<Duration>,
    tls_skip_verify: bool,
}

/// TLS configuration for Consul connection.
#[derive(Debug, Clone)]
pub struct ConsulTlsConfig {
    pub ca_file: String,
    pub cert_file: String,
    pub key_file: String,
}

impl ConsulSourceBuilder {
    /// Create a new Consul source builder.
    pub fn new() -> Self {
        Self {
            address: "127.0.0.1:8500".to_string(),
            token: None,
            prefix: "config".to_string(),
            format: None,
            interval: None,
            tls_skip_verify: false,
        }
    }

    /// Set the Consul agent address.
    pub fn address(mut self, address: impl Into<String>) -> Self {
        self.address = address.into();
        self
    }

    /// Set the Consul ACL token.
    pub fn token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(token.into());
        self
    }

    /// Set the KV prefix to watch.
    pub fn prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = prefix.into();
        self
    }

    /// Set the configuration format.
    pub fn format(mut self, format: Format) -> Self {
        self.format = Some(format);
        self
    }

    /// Set the poll interval.
    pub fn interval(mut self, interval: Duration) -> Self {
        self.interval = Some(interval);
        self
    }

    /// Skip TLS verification (for development only).
    ///
    /// This option is only effective in debug builds.
    /// In release builds, TLS verification is always enforced for security.
    pub fn tls_skip_verify(mut self, skip: bool) -> Self {
        #[cfg(debug_assertions)]
        {
            self.tls_skip_verify = skip;
        }
        #[cfg(not(debug_assertions))]
        {
            if skip {
                // TLS skip not allowed in release - silently ignored
            }
            self.tls_skip_verify = false;
        }
        self
    }

    /// Build the Consul source.
    pub fn build(self) -> ConfigResult<ConsulSource> {
        let client = Client::builder()
            .danger_accept_invalid_certs(self.tls_skip_verify)
            .build()
            .map_err(|e| ConfigError::InvalidValue {
                key: "consul".to_string(),
                expected_type: "HTTP client".to_string(),
                message: format!("Failed to create HTTP client: {}", e),
            })?;

        Ok(ConsulSource {
            client: Arc::new(client),
            address: Arc::from(self.address),
            prefix: Arc::from(self.prefix),
            format: self.format,
            interval: self.interval.unwrap_or(DEFAULT_CONSUL_POLL_INTERVAL),
            token: self.token.map(Arc::from),
            last_index: Arc::new(std::sync::Mutex::new(0u64)),
            cached_value: Arc::new(std::sync::RwLock::new(None)),
        })
    }
}

impl Default for ConsulSourceBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Consul-backed configuration source.
pub struct ConsulSource {
    client: Arc<Client>,
    address: Arc<str>,
    prefix: Arc<str>,
    #[allow(dead_code)] // reserved for future format-specific polling
    format: Option<Format>,
    interval: Duration,
    token: Option<Arc<str>>,
    last_index: Arc<std::sync::Mutex<u64>>,
    cached_value: Arc<std::sync::RwLock<Option<AnnotatedValue>>>,
}

impl ConsulSource {
    /// Get the source identifier.
    pub fn source_id(&self) -> SourceId {
        SourceId::new(format!("consul:{}", self.prefix))
    }

    /// Poll Consul for configuration.
    async fn poll_internal(&self) -> ConfigResult<AnnotatedValue> {
        // Build the KV request URL
        let base_url = if self.address.contains("://") {
            self.address.to_string()
        } else {
            format!("http://{}", self.address)
        };

        let path = if self.prefix.is_empty() {
            format!("{}/v1/kv/?recurse=true", base_url)
        } else {
            format!("{}/v1/kv/{}?recurse=true", base_url, self.prefix)
        };

        // Get the current index
        let current_index = {
            let guard = self
                .last_index
                .lock()
                .map_err(|_| ConfigError::LockPoisoned {
                    resource: "consul_last_index".to_string(),
                })?;
            *guard
        };

        // Build request
        let mut request = self.client.get(&path);

        // Add ACL token if provided
        if let Some(ref token) = self.token {
            request = request.header("X-Consul-Token", token.as_ref());
        }

        // Add index for blocking wait (wait for changes)
        if current_index > 0 {
            let wait_path = format!("{}&wait=30s&index={}", path, current_index);
            request = self.client.get(&wait_path);

            if let Some(ref token) = self.token {
                request = request.header("X-Consul-Token", token.as_ref());
            }
        }

        // Make the request
        let response = request
            .send()
            .await
            .map_err(|e| ConfigError::InvalidValue {
                key: "consul".to_string(),
                expected_type: "Consul KV response".to_string(),
                message: format!("Failed to fetch from Consul: {}", e),
            })?;

        if !response.status().is_success() {
            return Err(ConfigError::InvalidValue {
                key: "consul".to_string(),
                expected_type: "Consul KV response".to_string(),
                message: format!("Consul returned status: {}", response.status()),
            });
        }

        let kv_responses: Vec<KvResponse> =
            response
                .json()
                .await
                .map_err(|e| ConfigError::InvalidValue {
                    key: "consul".to_string(),
                    expected_type: "Consul KV response".to_string(),
                    message: format!("Failed to parse Consul response: {}", e),
                })?;

        if kv_responses.is_empty() {
            // Return cached value if no changes
            let cached = self
                .cached_value
                .read()
                .map_err(|_| ConfigError::LockPoisoned {
                    resource: "consul_cached_value".to_string(),
                })?;
            if let Some(ref value) = *cached {
                return Ok(value.clone());
            }
            return Err(ConfigError::InvalidValue {
                key: "consul".to_string(),
                expected_type: "KV response".to_string(),
                message: "No configuration found in Consul".to_string(),
            });
        }

        // Find the maximum index
        let max_index = kv_responses
            .iter()
            .filter_map(|r| r.modify_index)
            .max()
            .unwrap_or(0);

        // Update index if changed
        if max_index > current_index {
            let mut guard = self
                .last_index
                .lock()
                .map_err(|_| ConfigError::LockPoisoned {
                    resource: "consul_last_index".to_string(),
                })?;
            *guard = max_index;
        }

        // Merge all KV values into a single config
        let mut config_map = indexmap::IndexMap::new();

        for kv in &kv_responses {
            if let Some(value) = &kv.value {
                // Decode base64 value (Consul stores values as base64)
                let decoded = match base64_decode(value) {
                    Ok(d) => d,
                    Err(_) => value.clone(),
                };

                // Extract key name from full path
                let key = if self.prefix.is_empty() {
                    // No prefix, use the full key
                    if decoded.starts_with('/') {
                        decoded.trim_start_matches('/').to_string()
                    } else {
                        decoded.clone()
                    }
                } else {
                    // Remove prefix from key
                    if value.starts_with(&*self.prefix) {
                        value
                            .strip_prefix(&*self.prefix)
                            .unwrap_or(value)
                            .trim_start_matches('/')
                            .to_string()
                    } else {
                        value.clone()
                    }
                };

                // Try to parse as TOML/JSON/YAML
                if let Some(parsed) = try_parse_value(&decoded, "consul") {
                    // Merge into config map
                    merge_into_map(&mut config_map, &key, parsed);
                } else {
                    // Treat as simple string value
                    config_map.insert(
                        Arc::from(key.clone()),
                        AnnotatedValue::new(
                            crate::types::ConfigValue::String(decoded.clone()),
                            SourceId::new("consul"),
                            key.as_str(),
                        ),
                    );
                }
            }
        }

        let value = if config_map.is_empty() {
            crate::types::ConfigValue::Null
        } else {
            crate::types::ConfigValue::map(config_map.into_iter().collect())
        };

        let result = AnnotatedValue::new(value, SourceId::new("consul"), "");

        // Cache the result
        {
            let mut cached = self
                .cached_value
                .write()
                .map_err(|_| ConfigError::LockPoisoned {
                    resource: "consul_cached_value".to_string(),
                })?;
            *cached = Some(result.clone());
        }

        Ok(result)
    }
}

/// Decode base64 string.
fn base64_decode(input: &str) -> Result<String, base64::DecodeError> {
    use base64::Engine;
    let engine = base64::engine::general_purpose::STANDARD;
    let decoded = engine.decode(input)?;
    Ok(String::from_utf8(decoded).unwrap_or_default())
}

#[async_trait]
impl crate::remote::PolledSource for ConsulSource {
    async fn poll(&self) -> ConfigResult<AnnotatedValue> {
        self.poll_internal().await
    }

    fn poll_interval(&self) -> Option<Duration> {
        Some(self.interval)
    }

    fn source_id(&self) -> SourceId {
        Self::source_id(self)
    }
}

#[async_trait]
impl crate::interface::AsyncSource for ConsulSource {
    async fn load(&self) -> ConfigResult<AnnotatedValue> {
        self.poll_internal().await
    }

    fn source_id(&self) -> &SourceId {
        static SOURCE_ID: std::sync::OnceLock<SourceId> = std::sync::OnceLock::new();
        SOURCE_ID.get_or_init(|| SourceId::new("consul"))
    }

    fn priority(&self) -> u8 {
        50
    }

    fn name(&self) -> &str {
        "consul"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_default() {
        let builder = ConsulSourceBuilder::new();
        assert_eq!(builder.prefix, "config");
        assert_eq!(builder.interval, None);
    }

    #[test]
    fn test_builder_chain() {
        let source = ConsulSourceBuilder::new()
            .address("consul.example.com:8500")
            .token("my-token") // pragma: allowlist secret
            .prefix("my-app")
            .interval(Duration::from_secs(60))
            .build();

        assert!(source.is_ok());
    }

    #[test]
    fn test_builder_default_impl() {
        let builder = ConsulSourceBuilder::default();
        assert_eq!(builder.address, "127.0.0.1:8500");
        assert_eq!(builder.prefix, "config");
        assert_eq!(builder.token, None);
        assert_eq!(builder.format, None);
        assert_eq!(builder.interval, None);
        assert!(!builder.tls_skip_verify);
    }

    #[test]
    fn test_builder_address() {
        let builder = ConsulSourceBuilder::new().address("consul.local:8500");
        assert_eq!(builder.address, "consul.local:8500");
    }

    #[test]
    fn test_builder_token() {
        let builder = ConsulSourceBuilder::new().token("secret-token"); // pragma: allowlist secret
        assert_eq!(builder.token.as_deref(), Some("secret-token"));
    }

    #[test]
    fn test_builder_prefix() {
        let builder = ConsulSourceBuilder::new().prefix("my-app/config");
        assert_eq!(builder.prefix, "my-app/config");
    }

    #[test]
    fn test_builder_format() {
        let builder = ConsulSourceBuilder::new().format(Format::Toml);
        assert_eq!(builder.format, Some(Format::Toml));
    }

    #[test]
    fn test_builder_interval() {
        let interval = Duration::from_secs(120);
        let builder = ConsulSourceBuilder::new().interval(interval);
        assert_eq!(builder.interval, Some(interval));
    }

    #[test]
    fn test_builder_tls_skip_verify_debug() {
        let builder = ConsulSourceBuilder::new().tls_skip_verify(true);
        #[cfg(debug_assertions)]
        assert!(builder.tls_skip_verify);
        #[cfg(not(debug_assertions))]
        assert!(!builder.tls_skip_verify);
    }

    #[test]
    fn test_builder_tls_skip_verify_false() {
        let builder = ConsulSourceBuilder::new().tls_skip_verify(false);
        assert!(!builder.tls_skip_verify);
    }

    #[test]
    fn test_build_success_with_all_options() {
        let source = ConsulSourceBuilder::new()
            .address("consul.example.com:8500")
            .token("my-token") // pragma: allowlist secret
            .prefix("my-app")
            .format(Format::Json)
            .interval(Duration::from_secs(60))
            .build();
        assert!(source.is_ok());
        let source = source.unwrap();
        assert_eq!(source.source_id().as_str(), "consul:my-app");
    }

    #[test]
    fn test_build_with_tls_skip_verify_debug() {
        let source = ConsulSourceBuilder::new().tls_skip_verify(true).build();
        assert!(source.is_ok());
    }

    #[test]
    fn test_source_id_format() {
        let source = ConsulSourceBuilder::new().prefix("my-app").build().unwrap();
        assert_eq!(source.source_id().as_str(), "consul:my-app");
    }

    #[test]
    fn test_source_id_default_prefix() {
        let source = ConsulSourceBuilder::new().build().unwrap();
        assert_eq!(source.source_id().as_str(), "consul:config");
    }

    #[test]
    fn test_polled_source_poll_interval() {
        use crate::remote::PolledSource;
        let source = ConsulSourceBuilder::new()
            .interval(Duration::from_secs(45))
            .build()
            .unwrap();
        assert_eq!(source.poll_interval(), Some(Duration::from_secs(45)));
    }

    #[test]
    fn test_polled_source_poll_interval_default() {
        use crate::remote::PolledSource;
        let source = ConsulSourceBuilder::new().build().unwrap();
        assert_eq!(source.poll_interval(), Some(DEFAULT_CONSUL_POLL_INTERVAL));
    }

    #[test]
    fn test_polled_source_source_id() {
        let source = ConsulSourceBuilder::new().prefix("app").build().unwrap();
        assert_eq!(source.source_id().as_str(), "consul:app");
    }

    #[test]
    fn test_async_source_name() {
        use crate::interface::AsyncSource;
        let source = ConsulSourceBuilder::new().build().unwrap();
        assert_eq!(source.name(), "consul");
    }

    #[test]
    fn test_async_source_priority() {
        use crate::interface::AsyncSource;
        let source = ConsulSourceBuilder::new().build().unwrap();
        assert_eq!(source.priority(), 50);
    }

    #[test]
    fn test_async_source_source_id() {
        // Default prefix is "config", so source_id is "consul:config".
        let source = ConsulSourceBuilder::new().build().unwrap();
        assert_eq!(source.source_id().as_str(), "consul:config");
    }

    #[test]
    fn test_tls_config_construction() {
        let tls = ConsulTlsConfig {
            ca_file: "/path/to/ca.pem".to_string(),
            cert_file: "/path/to/cert.pem".to_string(),
            key_file: "/path/to/key.pem".to_string(),
        };
        assert_eq!(tls.ca_file, "/path/to/ca.pem");
        assert_eq!(tls.cert_file, "/path/to/cert.pem");
        assert_eq!(tls.key_file, "/path/to/key.pem");
    }

    #[test]
    fn test_tls_config_clone_debug() {
        let tls = ConsulTlsConfig {
            ca_file: "ca".to_string(),
            cert_file: "cert".to_string(),
            key_file: "key".to_string(),
        };
        let cloned = tls.clone();
        assert_eq!(tls.ca_file, cloned.ca_file);
        assert_eq!(tls.cert_file, cloned.cert_file);
        assert_eq!(tls.key_file, cloned.key_file);
        let debug_str = format!("{:?}", tls);
        assert!(debug_str.contains("ConsulTlsConfig"));
    }

    #[test]
    fn test_base64_decode_valid() {
        let result = base64_decode("aGVsbG8=").unwrap();
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_base64_decode_empty() {
        let result = base64_decode("").unwrap();
        assert_eq!(result, "");
    }

    #[test]
    fn test_base64_decode_invalid() {
        let result = base64_decode("!!!not base64!!!");
        assert!(result.is_err());
    }

    #[test]
    fn test_base64_decode_complex_string() {
        // "config value" in base64
        let result = base64_decode("Y29uZmlnIHZhbHVl").unwrap();
        assert_eq!(result, "config value");
    }

    #[test]
    fn test_kv_response_deserialize_full() {
        let json = r#"{"Value":"aGVsbG8=","ModifyIndex":42}"#;
        let resp: KvResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.value, Some("aGVsbG8=".to_string()));
        assert_eq!(resp.modify_index, Some(42));
    }

    #[test]
    fn test_kv_response_deserialize_empty() {
        let json = r#"{}"#;
        let resp: KvResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.value, None);
        assert_eq!(resp.modify_index, None);
    }

    #[test]
    fn test_kv_response_deserialize_partial() {
        let json = r#"{"Value":"dGVzdA=="}"#;
        let resp: KvResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.value, Some("dGVzdA==".to_string()));
        assert_eq!(resp.modify_index, None);
    }

    #[test]
    fn test_default_consul_poll_interval_constant() {
        assert_eq!(DEFAULT_CONSUL_POLL_INTERVAL, Duration::from_secs(30));
    }

    // --- Helpers for poll_internal coverage ---

    /// Check whether a real Consul agent is reachable on the default port.
    fn consul_ready() -> bool {
        std::net::TcpStream::connect("127.0.0.1:8500").is_ok()
    }

    /// Spawn a minimal HTTP/1.1 server that serves `responses` in order, one per
    /// connection, then stops. Each entry is `(status_code, body)`. Returns the
    /// `host:port` address clients should use. Lets us exercise `poll_internal`
    /// branches deterministically without depending on a live Consul.
    fn mock_http_server(responses: Vec<(u16, String)>) -> String {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        std::thread::spawn(move || {
            for (status, body) in responses {
                let Ok((mut stream, _)) = listener.accept() else {
                    continue;
                };
                use std::io::{Read, Write};
                let mut buf = [0u8; 4096];
                let _ = stream.read(&mut buf);
                let response = format!(
                    "HTTP/1.1 {status} OK\r\nContent-Type: application/json\r\nConnection: close\r\nContent-Length: {len}\r\n\r\n{body}",
                    status = status,
                    len = body.len(),
                    body = body,
                );
                let _ = stream.write_all(response.as_bytes());
                let _ = stream.flush();
            }
        });
        format!("127.0.0.1:{}", addr.port())
    }

    #[tokio::test]
    async fn test_poll_internal_success_returns_map() {
        let body = r#"[{"Value":"aGVsbG8=","ModifyIndex":10}]"#.to_string();
        let addr = mock_http_server(vec![(200, body)]);
        let source = ConsulSourceBuilder::new()
            .address(addr)
            .prefix("config")
            .build()
            .unwrap();
        let result = source.poll_internal().await;
        assert!(result.is_ok(), "poll should succeed: {:?}", result.err());
        assert!(
            result.unwrap().is_map(),
            "non-empty KV response should yield a map"
        );
    }

    #[tokio::test]
    async fn test_poll_internal_non_200_returns_error() {
        let addr = mock_http_server(vec![(500, "internal error".to_string())]);
        let source = ConsulSourceBuilder::new().address(addr).build().unwrap();
        let err = source.poll_internal().await.unwrap_err().to_string();
        assert!(
            err.contains("status"),
            "error should mention response status: {err}"
        );
    }

    #[tokio::test]
    async fn test_poll_internal_connection_refused() {
        // Reserve a port then drop it to get a guaranteed-closed port.
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        let source = ConsulSourceBuilder::new()
            .address(format!("127.0.0.1:{}", port))
            .build()
            .unwrap();
        let err = source.poll_internal().await.unwrap_err().to_string();
        assert!(
            err.contains("Failed to fetch"),
            "error should mention fetch failure: {err}"
        );
    }

    #[tokio::test]
    async fn test_poll_internal_unsupported_url_scheme() {
        // reqwest only supports http/https; "ftp://" is rejected at send time,
        // exercising both the `contains("://")` URL branch and a fetch error.
        let source = ConsulSourceBuilder::new()
            .address("ftp://invalid-host")
            .build()
            .unwrap();
        let err = source.poll_internal().await.unwrap_err().to_string();
        assert!(
            err.contains("Failed to fetch"),
            "unsupported scheme should produce a fetch error: {err}"
        );
    }

    #[tokio::test]
    async fn test_poll_internal_empty_no_cache_returns_error() {
        let addr = mock_http_server(vec![(200, "[]".to_string())]);
        let source = ConsulSourceBuilder::new().address(addr).build().unwrap();
        let err = source.poll_internal().await.unwrap_err().to_string();
        assert!(
            err.contains("No configuration found"),
            "empty response without cache should error: {err}"
        );
    }

    #[tokio::test]
    async fn test_poll_internal_empty_returns_cached_value() {
        // First response caches a value (ModifyIndex=10); second response is
        // empty, exercising the "return cached value" branch (lines 230-246).
        let non_empty = r#"[{"Value":"aGVsbG8=","ModifyIndex":10}]"#.to_string();
        let addr = mock_http_server(vec![(200, non_empty), (200, "[]".to_string())]);
        let source = ConsulSourceBuilder::new().address(addr).build().unwrap();
        let first = source.poll_internal().await;
        assert!(first.is_ok(), "first poll should succeed");
        assert!(first.as_ref().unwrap().is_map());
        let second = source.poll_internal().await;
        assert!(second.is_ok(), "second poll should return cached value");
        assert!(
            second.as_ref().unwrap().is_map(),
            "cached value should be a map"
        );
    }

    #[tokio::test]
    async fn test_poll_internal_token_and_blocking_wait() {
        // Two polls: the second has current_index > 0, exercising the blocking
        // wait URL path and the token header re-attachment (lines 193-200).
        let body = r#"[{"Value":"aGVsbG8=","ModifyIndex":7}]"#.to_string();
        let addr = mock_http_server(vec![(200, body.clone()), (200, body)]);
        let source = ConsulSourceBuilder::new()
            .address(addr)
            .token("test-token") // pragma: allowlist secret
            .build()
            .unwrap();
        let first = source.poll_internal().await;
        assert!(first.is_ok());
        let second = source.poll_internal().await;
        assert!(
            second.is_ok(),
            "blocking-wait poll with token should succeed"
        );
    }

    #[tokio::test]
    async fn test_poll_internal_url_with_scheme() {
        // Address containing "://" exercises the scheme-preserving URL branch.
        let body = r#"[{"Value":"aGVsbG8=","ModifyIndex":1}]"#.to_string();
        let addr = format!("http://{}", mock_http_server(vec![(200, body)]));
        let source = ConsulSourceBuilder::new().address(addr).build().unwrap();
        let result = source.poll_internal().await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_map());
    }

    #[tokio::test]
    async fn test_poll_internal_empty_prefix() {
        // prefix="" exercises the empty-prefix URL and key-extraction branches.
        let body = r#"[{"Value":"aGVsbG8=","ModifyIndex":3}]"#.to_string();
        let addr = mock_http_server(vec![(200, body)]);
        let source = ConsulSourceBuilder::new()
            .address(addr)
            .prefix("")
            .build()
            .unwrap();
        let result = source.poll_internal().await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_map());
    }

    #[tokio::test]
    async fn test_poll_internal_real_consul_success() {
        assert!(
            consul_ready(),
            "Consul must be running on 127.0.0.1:8500 for this integration test"
        );
        let source = ConsulSourceBuilder::new()
            .address("127.0.0.1:8500")
            .prefix("config/app")
            .build()
            .unwrap();
        let result = source.poll_internal().await;
        assert!(
            result.is_ok(),
            "real consul poll should succeed: {:?}",
            result.err()
        );
        assert!(
            result.unwrap().is_map(),
            "seeded consul KV (config/app/*) should yield a map"
        );
    }

    #[tokio::test]
    async fn test_poll_internal_invalid_json_returns_error() {
        let addr = mock_http_server(vec![(200, "this is not valid json".to_string())]);
        let source = ConsulSourceBuilder::new()
            .address(addr)
            .prefix("config")
            .build()
            .unwrap();
        let result = source.poll_internal().await;
        assert!(result.is_err(), "invalid JSON should produce an error");
        let err = result.unwrap_err();
        assert!(
            matches!(err, ConfigError::InvalidValue { .. }),
            "expected InvalidValue, got {:?}",
            err
        );
        assert!(
            err.to_string().contains("Failed to parse Consul response"),
            "unexpected error message: {err}"
        );
    }

    #[tokio::test]
    async fn test_poll_internal_non_base64_value_fallback() {
        // Value "!!!notbase64!!!" cannot be decoded as base64 (contains '!'),
        // so the code falls back to using the raw value string (line 274).
        let body = r#"[{"Value":"!!!notbase64!!!","ModifyIndex":1}]"#.to_string();
        let addr = mock_http_server(vec![(200, body)]);
        let source = ConsulSourceBuilder::new()
            .address(addr)
            .prefix("")
            .build()
            .unwrap();
        let result = source.poll_internal().await;
        assert!(result.is_ok(), "poll should succeed: {:?}", result.err());
        let value = result.unwrap();
        assert!(value.is_map(), "non-empty KV response should yield a map");
    }

    #[tokio::test]
    async fn test_poll_internal_value_starts_with_prefix() {
        // Value "config/!data" fails base64 decode (contains '!') and starts
        // with the prefix "config/", exercising the prefix-stripping branch
        // (lines 288-292).
        let body = r#"[{"Value":"config/!data","ModifyIndex":1}]"#.to_string();
        let addr = mock_http_server(vec![(200, body)]);
        let source = ConsulSourceBuilder::new()
            .address(addr)
            .prefix("config/")
            .build()
            .unwrap();
        let result = source.poll_internal().await;
        assert!(result.is_ok(), "poll should succeed: {:?}", result.err());
        assert!(
            result.unwrap().is_map(),
            "non-empty KV response should yield a map"
        );
    }

    #[tokio::test]
    async fn test_poll_internal_null_values_returns_null_config() {
        // All KV entries have null Value, so config_map stays empty and the
        // result is ConfigValue::Null (line 317).
        let body = r#"[{"Value":null,"ModifyIndex":1}]"#.to_string();
        let addr = mock_http_server(vec![(200, body)]);
        let source = ConsulSourceBuilder::new()
            .address(addr)
            .prefix("config")
            .build()
            .unwrap();
        let result = source.poll_internal().await;
        assert!(result.is_ok(), "poll should succeed: {:?}", result.err());
        let value = result.unwrap();
        assert!(
            value.is_null(),
            "KV response with all-null values should yield Null config"
        );
    }

    #[test]
    fn test_polled_source_trait_source_id() {
        use crate::remote::PolledSource;
        let source = ConsulSourceBuilder::new().build().unwrap();
        // Call the TRAIT method (which delegates to the inherent method),
        // not the inherent method directly.
        let id = <ConsulSource as PolledSource>::source_id(&source);
        assert_eq!(id.as_str(), "consul:config");
    }

    #[tokio::test]
    async fn test_async_source_trait_load() {
        use crate::interface::AsyncSource;
        let body = r#"[{"Value":"aGVsbG8=","ModifyIndex":5}]"#.to_string();
        let addr = mock_http_server(vec![(200, body)]);
        let source = ConsulSourceBuilder::new()
            .address(addr)
            .prefix("config")
            .build()
            .unwrap();
        // Call the TRAIT method AsyncSource::load (delegates to poll_internal).
        let result = <ConsulSource as AsyncSource>::load(&source).await;
        assert!(
            result.is_ok(),
            "trait load should succeed: {:?}",
            result.err()
        );
        assert!(
            result.unwrap().is_map(),
            "non-empty KV response should yield a map"
        );
    }

    #[test]
    fn test_async_source_trait_source_id_ref() {
        use crate::interface::AsyncSource;
        let source = ConsulSourceBuilder::new().build().unwrap();
        // Call the TRAIT method AsyncSource::source_id (returns &SourceId to
        // a static "consul"), not the inherent source_id.
        let id = <ConsulSource as AsyncSource>::source_id(&source);
        assert_eq!(id.as_str(), "consul");
    }
}
