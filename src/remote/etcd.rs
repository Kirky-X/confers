//! Etcd remote configuration source.
//!
//! This module provides an etcd-backed implementation of the `PolledSource` trait,
//! using the etcd-client SDK (gRPC) to interact with etcd's KV store.

use crate::error::{ConfigError, ConfigResult};
use crate::loader::{detect_format_from_content, Format};
use crate::value::{AnnotatedValue, SourceId};
use async_trait::async_trait;
use etcd_client::{Client, ConnectOptions};
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
            last_revision: Arc::new(tokio::sync::Mutex::new(0i64)),
            cached_value: Arc::new(tokio::sync::RwLock::new(None)),
        })
    }
}

impl Default for EtcdSourceBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Etcd-backed configuration source using the etcd-client SDK.
#[allow(dead_code)]
pub struct EtcdSource {
    client: Arc<Client>,
    prefix: Arc<str>,
    format: Option<Format>,
    interval: Duration,
    last_revision: Arc<tokio::sync::Mutex<i64>>,
    cached_value: Arc<tokio::sync::RwLock<Option<AnnotatedValue>>>,
}

impl EtcdSource {
    /// Get the source identifier.
    pub fn source_id(&self) -> SourceId {
        SourceId::new(format!("etcd:{}", self.prefix))
    }

    /// Poll etcd for configuration.
    async fn poll_internal(&self) -> ConfigResult<AnnotatedValue> {
        let client = self.client.clone();
        let mut kv_client = client.kv_client();

        // Get all keys with the prefix
        let get_response = kv_client
            .get(self.prefix.as_ref(), None)
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
            let last_rev = {
                let guard = self.last_revision.lock().await;
                *guard
            };

            if last_rev == current_revision {
                // No changes, return cached value
                let cached = self.cached_value.read().await;
                if let Some(ref value) = *cached {
                    return Ok(value.clone());
                }
            }

            // Update revision
            {
                let mut guard = self.last_revision.lock().await;
                *guard = current_revision;
            }
        }

        // Build config from KV pairs
        let mut config_map = indexmap::IndexMap::new();

        let kvs = get_response.kvs();
        for kv in kvs.iter() {
            // Get key as bytes and convert to string
            let key_bytes: &[u8] = kv.key();
            let key = String::from_utf8_lossy(key_bytes).to_string();

            // Get value as bytes and convert to string
            let value_bytes: &[u8] = kv.value();
            let value = String::from_utf8_lossy(value_bytes).to_string();

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
            if let Some(parsed) = try_parse_value(&value) {
                merge_into_map(&mut config_map, &relative_key, parsed);
            } else {
                // Treat as simple string value
                config_map.insert(
                    Arc::from(relative_key.clone()),
                    AnnotatedValue::new(
                        crate::value::ConfigValue::String(value),
                        SourceId::new("etcd"),
                        relative_key.as_str(),
                    ),
                );
            }
        }

        let value = if config_map.is_empty() {
            crate::value::ConfigValue::Null
        } else {
            crate::value::ConfigValue::map(config_map.into_iter().collect())
        };

        let result = AnnotatedValue::new(value, SourceId::new("etcd"), "");

        // Cache the result
        {
            let mut cached = self.cached_value.write().await;
            *cached = Some(result.clone());
        }

        Ok(result)
    }
}

/// Try to parse a value as config format.
fn try_parse_value(content: &str) -> Option<AnnotatedValue> {
    // Try to detect format
    let format = detect_format_from_content(content)?;

    match format {
        Format::Toml => {
            let table: toml::Table = toml::from_str(content).ok()?;
            Some(crate::loader::parse_toml_table(
                &table,
                &SourceId::new("etcd"),
                "",
            ))
        }
        Format::Json => {
            let v: serde_json::Value = serde_json::from_str(content).ok()?;
            Some(crate::loader::parse_json_value(
                &v,
                &SourceId::new("etcd"),
                "",
            ))
        }
        Format::Yaml => {
            let v: serde_yaml_ng::Value = serde_yaml_ng::from_str(content).ok()?;
            Some(crate::loader::parse_yaml_value(
                &v,
                &SourceId::new("etcd"),
                "",
            ))
        }
        _ => None,
    }
}

/// Merge a key-value pair into a config map.
fn merge_into_map(
    map: &mut indexmap::IndexMap<Arc<str>, AnnotatedValue>,
    key: &str,
    value: AnnotatedValue,
) {
    map.insert(Arc::from(key.to_string()), value);
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
            .password("secret")
            .prefix("my-app")
            .interval(Duration::from_secs(60));

        assert_eq!(builder.prefix, "my-app");
    }
}
