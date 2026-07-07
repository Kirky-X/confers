// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Etcd remote configuration source.
//!
//! This module provides an etcd-backed implementation of the `PolledSource` trait,
//! using the etcd-client SDK (gRPC) to interact with etcd's KV store.

use super::common::{merge_into_map, try_parse_value};
use crate::error::{ConfigError, ConfigResult};
use crate::loader::Format;
use crate::types::{AnnotatedValue, SourceId};
use arc_swap::ArcSwap;
use async_trait::async_trait;
use etcd_client::{Client, ConnectOptions};
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// Default poll interval for etcd (30 seconds).
pub const DEFAULT_ETCD_POLL_INTERVAL: Duration = Duration::from_secs(30);

/// Builder for creating etcd configuration sources.
pub struct EtcdSourceBuilder {
    endpoints: Vec<String>,
    username: Option<String>,
    password: Option<String>,
    prefix: String,
    format: Option<Format>,
    interval: Option<Duration>,
    tls: Option<EtcdTlsConfig>,
}

/// TLS configuration for etcd connection.
#[derive(Debug, Clone)]
pub struct EtcdTlsConfig {
    pub ca_file: String,
    pub cert_file: String,
    pub key_file: String,
}

impl EtcdSourceBuilder {
    /// Create a new etcd source builder.
    pub fn new() -> Self {
        Self {
            endpoints: vec!["localhost:2379".to_string()],
            username: None,
            password: None,
            prefix: "config".to_string(),
            format: None,
            interval: None,
            tls: None,
        }
    }

    /// Set the etcd endpoints.
    pub fn endpoints(mut self, endpoints: impl Into<Vec<String>>) -> Self {
        self.endpoints = endpoints.into();
        self
    }

    /// Add a single endpoint.
    pub fn endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoints.push(endpoint.into());
        self
    }

    /// Set the username for authentication.
    pub fn username(mut self, username: impl Into<String>) -> Self {
        self.username = Some(username.into());
        self
    }

    /// Set the password for authentication.
    pub fn password(mut self, password: impl Into<String>) -> Self {
        self.password = Some(password.into());
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

    /// Set TLS configuration.
    pub fn tls(mut self, tls: EtcdTlsConfig) -> Self {
        self.tls = Some(tls);
        self
    }

    /// Build the etcd source.
    pub async fn build(self) -> ConfigResult<EtcdSource> {
        // Build connect options
        let mut options = ConnectOptions::new();

        if let (Some(username), Some(password)) = (self.username, self.password) {
            options = options.with_user(&username, &password);
        }

        // Connect to etcd using the SDK
        let endpoints: Vec<&str> = self.endpoints.iter().map(|s| s.as_str()).collect();
        let client = Client::connect(&endpoints, Some(options))
            .await
            .map_err(|e| ConfigError::InvalidValue {
                key: "etcd".to_string(),
                expected_type: "etcd client".to_string(),
                message: format!("Failed to connect to etcd: {}", e),
            })?;

        Ok(EtcdSource {
            client: Arc::new(client),
            prefix: Arc::from(self.prefix),
            format: self.format,
            interval: self.interval.unwrap_or(DEFAULT_ETCD_POLL_INTERVAL),
            last_revision: AtomicI64::new(0),
            cached_value: ArcSwap::new(Arc::new(None)),
        })
    }
}

impl Default for EtcdSourceBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Etcd-backed configuration source using the etcd-client SDK.
pub struct EtcdSource {
    client: Arc<Client>,
    prefix: Arc<str>,
    #[allow(dead_code)] // reserved for future format-specific polling
    format: Option<Format>,
    interval: Duration,
    last_revision: AtomicI64,
    cached_value: ArcSwap<Option<Arc<AnnotatedValue>>>,
}

impl EtcdSource {
    /// Get the source identifier.
    pub fn source_id(&self) -> SourceId {
        SourceId::new(format!("etcd:{}", self.prefix))
    }

    /// Poll etcd for configuration.
    async fn poll_internal(&self) -> ConfigResult<AnnotatedValue> {
        use etcd_client::GetOptions;

        let client = self.client.clone();
        let mut kv_client = client.kv_client();

        // Get all keys with the prefix
        let get_response = kv_client
            .get(self.prefix.as_ref(), Some(GetOptions::new().with_prefix()))
            .await
            .map_err(|e| ConfigError::InvalidValue {
                key: "etcd".to_string(),
                expected_type: "etcd KV response".to_string(),
                message: format!("Failed to fetch from etcd: {}", e),
            })?;

        // Get header with revision
        let header = get_response.header();
        let current_revision: i64 = match header {
            Some(h) => {
                let revision: i64 = h.revision();
                revision
            }
            None => 0,
        };

        // Check if we have cached value and revision hasn't changed
        if current_revision > 0 {
            let last_rev = self.last_revision.load(Ordering::Acquire);

            if last_rev == current_revision {
                // No changes, return cached value
                let cached = self.cached_value.load();
                if let Some(ref value) = **cached {
                    return Ok((**value).clone());
                }
            }

            // Update revision
            self.last_revision
                .store(current_revision, Ordering::Release);
        }

        // Build config from KV pairs
        let mut config_map = indexmap::IndexMap::new();

        let kvs = get_response.kvs();
        for kv in kvs.iter() {
            // Get key as bytes and convert to string.
            // M6: Use String::from_utf8 (not from_utf8_lossy) to surface
            // invalid UTF-8 as an error instead of silently replacing
            // invalid bytes with U+FFFD (Rule 12: Fail Loud).
            let key_bytes: &[u8] = kv.key();
            let key =
                String::from_utf8(key_bytes.to_vec()).map_err(|e| ConfigError::InvalidValue {
                    key: "etcd".to_string(),
                    expected_type: "UTF-8 key".to_string(),
                    message: format!("etcd key is not valid UTF-8: {}", e),
                })?;

            // Get value as bytes and convert to string (same M6 fix).
            let value_bytes: &[u8] = kv.value();
            let value =
                String::from_utf8(value_bytes.to_vec()).map_err(|e| ConfigError::InvalidValue {
                    key: "etcd".to_string(),
                    expected_type: "UTF-8 value".to_string(),
                    message: format!("etcd value for key '{}' is not valid UTF-8: {}", key, e),
                })?;

            // Remove prefix from key
            let relative_key: String = if key.starts_with(&*self.prefix) {
                key.strip_prefix(&*self.prefix)
                    .unwrap_or(&key)
                    .trim_start_matches('/')
                    .to_string()
            } else {
                key.clone()
            };

            // Try to parse as config format
            if let Some(parsed) = try_parse_value(&value, "etcd") {
                merge_into_map(&mut config_map, &relative_key, parsed);
            } else {
                // Treat as simple string value
                config_map.insert(
                    Arc::from(relative_key.clone()),
                    AnnotatedValue::new(
                        crate::types::ConfigValue::String(value),
                        SourceId::new("etcd"),
                        relative_key.as_str(),
                    ),
                );
            }
        }

        let value = if config_map.is_empty() {
            crate::types::ConfigValue::Null
        } else {
            crate::types::ConfigValue::map(config_map.into_iter().collect())
        };

        let result = AnnotatedValue::new(value, SourceId::new("etcd"), "");

        // Cache the result
        self.cached_value
            .store(Arc::new(Some(Arc::new(result.clone()))));

        Ok(result)
    }
}

#[async_trait]
impl crate::remote::PolledSource for EtcdSource {
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
impl crate::interface::AsyncSource for EtcdSource {
    async fn load(&self) -> ConfigResult<AnnotatedValue> {
        self.poll_internal().await
    }

    fn source_id(&self) -> &SourceId {
        static SOURCE_ID: std::sync::OnceLock<SourceId> = std::sync::OnceLock::new();
        SOURCE_ID.get_or_init(|| SourceId::new("etcd"))
    }

    fn priority(&self) -> u8 {
        50
    }

    fn name(&self) -> &str {
        "etcd"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_default() {
        let builder = EtcdSourceBuilder::new();
        assert_eq!(builder.prefix, "config");
        assert_eq!(builder.interval, None);
    }

    #[test]
    fn test_builder_chain() {
        let builder = EtcdSourceBuilder::new()
            .endpoint("etcd.example.com:2379")
            .endpoints(vec![
                "etcd1.example.com:2379".to_string(),
                "etcd2.example.com:2379".to_string(),
            ])
            .username("root")
            .password("secret") // pragma: allowlist secret
            .prefix("my-app")
            .interval(Duration::from_secs(60));

        assert_eq!(builder.prefix, "my-app");
    }

    #[test]
    fn test_builder_default_impl() {
        let builder = EtcdSourceBuilder::default();
        assert_eq!(builder.endpoints, vec!["localhost:2379".to_string()]);
        assert_eq!(builder.prefix, "config");
        assert_eq!(builder.username, None);
        assert_eq!(builder.password, None);
        assert_eq!(builder.format, None);
        assert_eq!(builder.interval, None);
        assert!(builder.tls.is_none());
    }

    #[test]
    fn test_builder_endpoints_replace() {
        let builder = EtcdSourceBuilder::new()
            .endpoints(vec!["etcd1:2379".to_string(), "etcd2:2379".to_string()]);
        assert_eq!(builder.endpoints.len(), 2);
        assert_eq!(builder.endpoints[0], "etcd1:2379");
        assert_eq!(builder.endpoints[1], "etcd2:2379");
    }

    #[test]
    fn test_builder_endpoint_appends() {
        let builder = EtcdSourceBuilder::new()
            .endpoint("etcd1:2379")
            .endpoint("etcd2:2379");
        // Default endpoint (localhost:2379) + 2 added
        assert_eq!(builder.endpoints.len(), 3);
        assert!(builder.endpoints.contains(&"etcd1:2379".to_string()));
        assert!(builder.endpoints.contains(&"etcd2:2379".to_string()));
        assert!(builder.endpoints.contains(&"localhost:2379".to_string()));
    }

    #[test]
    fn test_builder_username() {
        let builder = EtcdSourceBuilder::new().username("root");
        assert_eq!(builder.username.as_deref(), Some("root"));
    }

    #[test]
    fn test_builder_password() {
        let builder = EtcdSourceBuilder::new().password("secret"); // pragma: allowlist secret
        assert_eq!(builder.password.as_deref(), Some("secret"));
    }

    #[test]
    fn test_builder_prefix() {
        let builder = EtcdSourceBuilder::new().prefix("my-app/config");
        assert_eq!(builder.prefix, "my-app/config");
    }

    #[test]
    fn test_builder_format() {
        let builder = EtcdSourceBuilder::new().format(Format::Json);
        assert_eq!(builder.format, Some(Format::Json));
    }

    #[test]
    fn test_builder_interval() {
        let interval = Duration::from_secs(90);
        let builder = EtcdSourceBuilder::new().interval(interval);
        assert_eq!(builder.interval, Some(interval));
    }

    #[test]
    fn test_builder_tls() {
        let tls = EtcdTlsConfig {
            ca_file: "/path/ca.pem".to_string(),
            cert_file: "/path/cert.pem".to_string(),
            key_file: "/path/key.pem".to_string(),
        };
        let builder = EtcdSourceBuilder::new().tls(tls.clone());
        assert!(builder.tls.is_some());
        // Verify the TLS config was actually stored by checking a field.
        assert_eq!(builder.tls.as_ref().unwrap().ca_file, "/path/ca.pem");
    }

    #[test]
    fn test_builder_full_chain() {
        let builder = EtcdSourceBuilder::new()
            .endpoints(vec!["etcd1:2379".to_string()])
            .username("admin")
            .password("pass") // pragma: allowlist secret
            .prefix("app")
            .format(Format::Toml)
            .interval(Duration::from_secs(30));
        assert_eq!(builder.endpoints, vec!["etcd1:2379".to_string()]);
        assert_eq!(builder.username.as_deref(), Some("admin"));
        assert_eq!(builder.password.as_deref(), Some("pass"));
        assert_eq!(builder.prefix, "app");
        assert_eq!(builder.format, Some(Format::Toml));
        assert_eq!(builder.interval, Some(Duration::from_secs(30)));
    }

    #[test]
    fn test_tls_config_construction() {
        let tls = EtcdTlsConfig {
            ca_file: "/ca.pem".to_string(),
            cert_file: "/cert.pem".to_string(),
            key_file: "/key.pem".to_string(),
        };
        assert_eq!(tls.ca_file, "/ca.pem");
        assert_eq!(tls.cert_file, "/cert.pem");
        assert_eq!(tls.key_file, "/key.pem");
    }

    #[test]
    fn test_tls_config_clone_debug() {
        let tls = EtcdTlsConfig {
            ca_file: "ca".to_string(),
            cert_file: "cert".to_string(),
            key_file: "key".to_string(),
        };
        let cloned = tls.clone();
        assert_eq!(tls.ca_file, cloned.ca_file);
        assert_eq!(tls.cert_file, cloned.cert_file);
        assert_eq!(tls.key_file, cloned.key_file);
        let debug_str = format!("{:?}", tls);
        assert!(debug_str.contains("EtcdTlsConfig"));
    }

    #[test]
    fn test_default_etcd_poll_interval_constant() {
        assert_eq!(DEFAULT_ETCD_POLL_INTERVAL, Duration::from_secs(30));
    }

    // --- Helpers for live etcd integration tests ---

    /// Check whether a real etcd is reachable on the default gRPC port.
    fn etcd_ready() -> bool {
        std::net::TcpStream::connect("127.0.0.1:2379").is_ok()
    }

    static UNIQUE_PREFIX_COUNTER: std::sync::atomic::AtomicU64 =
        std::sync::atomic::AtomicU64::new(0);

    /// Generate a unique etcd key prefix so parallel tests (and repeated test
    /// runs, since etcd persists keys) don't collide. Combines a nanosecond
    /// timestamp (unique per run) with a monotonic counter (unique within a run).
    fn unique_prefix() -> String {
        let n = UNIQUE_PREFIX_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);
        format!("config/cov_{}_{}", ts, n)
    }

    #[tokio::test]
    async fn test_build_success_real_etcd() {
        if !etcd_ready() {
            return;
        }
        let source = EtcdSourceBuilder::new().build().await;
        assert!(source.is_ok(), "build should succeed: {:?}", source.err());
    }

    #[tokio::test]
    async fn test_poll_unreachable_endpoint_returns_error() {
        // gRPC connect is lazy, so build succeeds; the first poll RPC fails
        // because the endpoint is unreachable (exercising poll_internal's
        // error path, lines 162-169).
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        let source = EtcdSourceBuilder::new()
            .endpoints(vec![format!("127.0.0.1:{}", port)])
            .build()
            .await
            .expect("lazy connect should succeed even on a closed port");
        let result = source.poll_internal().await;
        assert!(result.is_err(), "poll on unreachable endpoint should fail");
        let err = match result {
            Err(e) => e.to_string(),
            Ok(_) => unreachable!("expected poll error, got Ok"),
        };
        assert!(
            err.contains("Failed to fetch from etcd"),
            "error should mention fetch failure: {err}"
        );
    }

    #[tokio::test]
    async fn test_build_with_auth_options_rejected_when_auth_disabled() {
        if !etcd_ready() {
            return;
        }
        // Setting both username and password triggers ConnectOptions::with_user
        // (line 107). The dev-mode etcd has auth disabled, so connect's auth
        // RPC is rejected with "authentication is not enabled" — proving the
        // with_user branch was executed.
        let result = EtcdSourceBuilder::new()
            .username("root")
            .password("pass") // pragma: allowlist secret
            .build()
            .await;
        assert!(
            result.is_err(),
            "build with auth should fail on auth-disabled etcd"
        );
        let err = match result {
            Err(e) => e.to_string(),
            Ok(_) => unreachable!("expected build error, got Ok"),
        };
        assert!(
            err.contains("authentication is not enabled"),
            "expected auth-disabled error: {err}"
        );
    }

    #[tokio::test]
    async fn test_source_id_after_build() {
        if !etcd_ready() {
            return;
        }
        let source = EtcdSourceBuilder::new()
            .prefix("my-app")
            .build()
            .await
            .unwrap();
        assert_eq!(source.source_id().as_str(), "etcd:my-app");
    }

    #[tokio::test]
    async fn test_async_source_trait_methods() {
        if !etcd_ready() {
            return;
        }
        use crate::interface::AsyncSource;
        let source = EtcdSourceBuilder::new().build().await.unwrap();
        assert_eq!(source.name(), "etcd");
        assert_eq!(source.priority(), 50);
        // Use fully-qualified syntax so the TRAIT method (static "etcd",
        // lines 274-277) is called instead of the inherent method
        // (`etcd:{prefix}`).
        assert_eq!(
            <EtcdSource as AsyncSource>::source_id(&source).as_str(),
            "etcd"
        );
    }

    #[tokio::test]
    async fn test_polled_source_poll_interval_after_build() {
        if !etcd_ready() {
            return;
        }
        use crate::remote::PolledSource;
        let source = EtcdSourceBuilder::new()
            .interval(Duration::from_secs(45))
            .build()
            .await
            .unwrap();
        assert_eq!(source.poll_interval(), Some(Duration::from_secs(45)));
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_poll_internal_returns_seeded_data() {
        if !etcd_ready() {
            return;
        }
        let prefix = unique_prefix();
        let put_client = Client::connect(&["127.0.0.1:2379"], None)
            .await
            .expect("etcd connect for seed");
        let mut kv = put_client.kv_client();
        kv.put(format!("{}/key", prefix), "value".to_string(), None)
            .await
            .expect("etcd put seed");
        let source = EtcdSourceBuilder::new()
            .prefix(prefix)
            .build()
            .await
            .unwrap();
        let result = source.poll_internal().await;
        assert!(result.is_ok(), "poll should succeed: {:?}", result.err());
        assert!(
            result.unwrap().is_map(),
            "seeded etcd KV should yield a map"
        );
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_poll_returns_cached_on_same_revision() {
        if !etcd_ready() {
            return;
        }
        let prefix = unique_prefix();
        let put_client = Client::connect(&["127.0.0.1:2379"], None)
            .await
            .expect("etcd connect for seed");
        let mut kv = put_client.kv_client();
        kv.put(format!("{}/key", prefix), "value".to_string(), None)
            .await
            .expect("etcd put seed");
        let source = EtcdSourceBuilder::new()
            .prefix(prefix)
            .build()
            .await
            .unwrap();
        let first = source.poll_internal().await;
        assert!(first.is_ok(), "first poll should succeed");
        assert!(first.as_ref().unwrap().is_map());
        // No writes between polls (serial) → same revision → cached path
        // (lines 185-191) is exercised.
        let second = source.poll_internal().await;
        assert!(second.is_ok(), "second poll should return cached value");
        assert!(second.as_ref().unwrap().is_map());
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_poll_empty_prefix_returns_null() {
        if !etcd_ready() {
            return;
        }
        let prefix = unique_prefix(); // no keys put under this prefix
        let source = EtcdSourceBuilder::new()
            .prefix(prefix)
            .build()
            .await
            .unwrap();
        let result = source.poll_internal().await;
        assert!(result.is_ok(), "poll on empty prefix should succeed");
        assert!(
            result.unwrap().is_null(),
            "prefix with no keys should yield Null"
        );
    }
}
