//! Consul remote configuration source.
//!
//! This module provides a Consul-backed implementation of the `PolledSource` trait,
//! using the Consul KV REST API via reqwest.

use crate::error::{ConfigError, ConfigResult};
use crate::loader::{detect_format_from_content, Format};
use crate::value::{AnnotatedValue, SourceId};
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
#[allow(dead_code)]
pub struct ConsulSource {
    client: Arc<Client>,
    address: Arc<str>,
    prefix: Arc<str>,
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
                if let Some(parsed) = try_parse_value(&decoded) {
                    // Merge into config map
                    merge_into_map(&mut config_map, &key, parsed);
                } else {
                    // Treat as simple string value
                    config_map.insert(
                        Arc::from(key.clone()),
                        AnnotatedValue::new(
                            crate::value::ConfigValue::String(decoded.clone()),
                            SourceId::new("consul"),
                            key.as_str(),
                        ),
                    );
                }
            }
        }

        let value = if config_map.is_empty() {
            crate::value::ConfigValue::Null
        } else {
            crate::value::ConfigValue::map(config_map.into_iter().collect())
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

/// Try to parse a value as config format.
fn try_parse_value(content: &str) -> Option<AnnotatedValue> {
    // Try to detect format
    let format = detect_format_from_content(content)?;

    match format {
        Format::Toml => {
            let table: toml::Table = toml::from_str(content).ok()?;
            Some(crate::loader::parse_toml_table(
                &table,
                &SourceId::new("consul"),
                "",
            ))
        }
        Format::Json => {
            let v: serde_json::Value = serde_json::from_str(content).ok()?;
            Some(crate::loader::parse_json_value(
                &v,
                &SourceId::new("consul"),
                "",
            ))
        }
        #[cfg(feature = "yaml")]
        Format::Yaml => {
            let v: serde_yaml_ng::Value = serde_yaml_ng::from_str(content).ok()?;
            Some(crate::loader::parse_yaml_value(
                &v,
                &SourceId::new("consul"),
                "",
            ))
        }
        #[cfg(not(feature = "yaml"))]
        Format::Yaml => None,
        _ => None,
    }
}

/// Merge a key-value pair into a config map.
fn merge_into_map(
    map: &mut indexmap::IndexMap<Arc<str>, AnnotatedValue>,
    key: &str,
    value: AnnotatedValue,
) {
    // For simplicity, use the full key as-is
    // A more complete implementation would handle nested keys
    map.insert(Arc::from(key), value);
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
impl crate::config::source::AsyncSource for ConsulSource {
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
            .token("my-token")
            .prefix("my-app")
            .interval(Duration::from_secs(60))
            .build();

        assert!(source.is_ok());
    }
}
