//! Polled source abstraction for remote configuration.
//!
//! This module provides the `PolledSource` trait for sources that require
//! periodic polling (HTTP endpoints, databases, etc.) and implements
//! `HttpPolledSource` for HTTP-based configuration sources with ETag
//! and Last-Modified support.

use crate::error::{ConfigError, ConfigResult};
use crate::loader::{detect_format_from_content, Format};
use crate::value::{AnnotatedValue, SourceId};
use arc_swap::ArcSwap;
use async_trait::async_trait;
use reqwest::Client;
use std::net::IpAddr;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use std::sync::RwLock;
use std::time::Duration;

/// Default poll interval when not specified (60 seconds).
pub const DEFAULT_POLL_INTERVAL: Duration = Duration::from_secs(60);

/// Blocked IP ranges for SSRF protection.
/// Includes private networks, loopback, and link-local addresses.
#[allow(dead_code)]
const BLOCKED_IP_RANGES: &[&str] = &[
    "127.0.0.0/8",     // Loopback
    "10.0.0.0/8",      // Private
    "172.16.0.0/12",   // Private
    "192.168.0.0/16",  // Private
    "169.254.0.0/16",  // Link-local
    "0.0.0.0/8",       // Current network
    "100.64.0.0/10",   // Carrier-grade NAT
    "192.0.0.0/24",    // IETF Protocol assignments
    "192.0.2.0/24",    // Documentation
    "198.51.100.0/24", // Documentation
    "203.0.113.0/24",  // Documentation
    "fc00::/7",        // IPv6 unique local
    "fe80::/10",       // IPv6 link-local
    "::1/128",         // IPv6 loopback
];

/// Check if an IP address is in a blocked range.
fn is_ip_blocked(ip: IpAddr) -> bool {
    // For simplicity, block all non-public IPs
    // In production, implement proper CIDR matching
    match ip {
        IpAddr::V4(ipv4) => {
            let octets = ipv4.octets();
            // Block 127.x.x.x (loopback)
            octets[0] == 127 ||
            // Block 10.x.x.x
            octets[0] == 10 ||
            // Block 172.16-31.x.x
            (octets[0] == 172 && (16..=31).contains(&octets[1])) ||
            // Block 192.168.x.x
            (octets[0] == 192 && octets[1] == 168) ||
            // Block 169.254.x.x (link-local)
            (octets[0] == 169 && octets[1] == 254) ||
            // Block 100.64-127.x.x (carrier-grade NAT)
            (octets[0] == 100 && (64..=127).contains(&octets[1]))
        }
        IpAddr::V6(ipv6) => {
            let segments = ipv6.segments();
            // Block fc00::/7 (unique local)
            (segments[0] & 0xfe00) == 0xfc00 ||
            // Block fe80::/10 (link-local)
            (segments[0] & 0xffc0) == 0xfe80 ||
            // Block ::1 (loopback)
            ipv6.is_loopback()
        }
    }
}

/// Check if a url::Ipv4Addr is blocked.
fn is_ip_blocked_std(ip: std::net::Ipv4Addr) -> bool {
    is_ip_blocked(IpAddr::V4(ip))
}

/// Check if a url::Ipv6Addr is blocked.
fn is_ip_blocked_v6(ip: std::net::Ipv6Addr) -> bool {
    is_ip_blocked(IpAddr::V6(ip))
}

/// Validate URL for security (SSRF protection).
fn validate_url(url: &str) -> ConfigResult<()> {
    let parsed = url::Url::parse(url).map_err(|_| ConfigError::InvalidValue {
        key: "url".to_string(),
        expected_type: "valid URL".to_string(),
        message: "Invalid URL format".to_string(),
    })?;

    // Only allow HTTPS by default for security
    if parsed.scheme() != "https" {
        return Err(ConfigError::InvalidValue {
            key: "url".to_string(),
            expected_type: "https URL".to_string(),
            message: "Only HTTPS URLs are allowed for remote configuration".to_string(),
        });
    }

    // Check if host is a blocked IP
    if let Some(host) = parsed.host() {
        match host {
            url::Host::Domain(_) => {
                // Domain name, not an IP - OK
            }
            url::Host::Ipv4(ip) => {
                if is_ip_blocked_std(ip) {
                    return Err(ConfigError::InvalidValue {
                        key: "url".to_string(),
                        expected_type: "public IP".to_string(),
                        message: "Connection to private/internal IP addresses is not allowed"
                            .to_string(),
                    });
                }
            }
            url::Host::Ipv6(ip) => {
                if is_ip_blocked_v6(ip) {
                    return Err(ConfigError::InvalidValue {
                        key: "url".to_string(),
                        expected_type: "public IP".to_string(),
                        message: "Connection to private/internal IP addresses is not allowed"
                            .to_string(),
                    });
                }
            }
        }
    }

    Ok(())
}

/// Trait for polled configuration sources.
///
/// Sources implementing this trait will be polled at regular intervals
/// to fetch the latest configuration values.
#[async_trait]
pub trait PolledSource: Send + Sync {
    /// Poll the source for the latest configuration.
    async fn poll(&self) -> ConfigResult<AnnotatedValue>;

    /// Get the poll interval for this source.
    fn poll_interval(&self) -> Option<Duration>;

    /// Get the source identifier.
    fn source_id(&self) -> SourceId;
}

/// HTTP-polled configuration source.
///
/// Fetches configuration from an HTTP endpoint with support for:
/// - ETag-based conditional requests (If-None-Match)
/// - Last-Modified-based conditional requests (If-Modified-Since)
/// - Configurable poll intervals
/// - Automatic format detection
///
/// # Lock Contention Optimization
///
/// This implementation uses atomics for ETag/Modified tracking to minimize
/// lock contention in high-concurrency scenarios. Only the cached value
/// is protected by a RwLock, which is held for the minimal time necessary.
#[allow(dead_code)]
pub struct HttpPolledSource {
    url: Arc<str>,
    interval: Duration,
    client: Client,
    format: Option<Format>,
    cache_generation: AtomicU64,
    cached: RwLock<Option<AnnotatedValue>>,
    last_etag: ArcSwap<Option<String>>,
    last_modified: ArcSwap<Option<String>>,
    source_id: SourceId,
}

/// Builder for `HttpPolledSource`.
pub struct HttpPolledSourceBuilder {
    url: Option<String>,
    interval: Option<Duration>,
    format: Option<Format>,
    timeout: Option<Duration>,
}

impl HttpPolledSourceBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self {
            url: None,
            interval: None,
            format: None,
            timeout: None,
        }
    }

    /// Set the URL of the remote configuration endpoint.
    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }

    /// Set the poll interval.
    pub fn interval(mut self, interval: Duration) -> Self {
        self.interval = Some(interval);
        self
    }

    /// Set the configuration format (auto-detected if not specified).
    pub fn format(mut self, format: Format) -> Self {
        self.format = Some(format);
        self
    }

    /// Set the request timeout.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Build the `HttpPolledSource`.
    pub fn build(self) -> ConfigResult<HttpPolledSource> {
        let url = self.url.ok_or_else(|| ConfigError::InvalidValue {
            key: "url".to_string(),
            expected_type: "string".to_string(),
            message: "URL is required".to_string(),
        })?;

        // Validate URL for security (SSRF protection)
        validate_url(&url)?;

        let url_arc: Arc<str> = url.clone().into();
        let source_id = SourceId::new(format!("http:{}", url_arc));

        // Build HTTP client with TLS enabled by default
        let mut client_builder = Client::builder().use_rustls_tls(); // Use rustls for TLS (secure by default)

        if let Some(timeout) = self.timeout {
            client_builder = client_builder.timeout(timeout);
        }

        let client = client_builder
            .build()
            .map_err(|_e| ConfigError::RemoteUnavailable {
                error_type: "ClientBuild".to_string(),
                retryable: false,
            })?;

        Ok(HttpPolledSource {
            url: url_arc,
            interval: self.interval.unwrap_or(DEFAULT_POLL_INTERVAL),
            client,
            format: self.format,
            cache_generation: AtomicU64::new(0),
            cached: RwLock::new(None),
            last_etag: ArcSwap::new(Arc::new(None)),
            last_modified: ArcSwap::new(Arc::new(None)),
            source_id,
        })
    }
}

impl Default for HttpPolledSourceBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PolledSource for HttpPolledSource {
    /// Poll the HTTP endpoint for configuration.
    ///
    /// Uses ETag and Last-Modified headers for conditional requests.
    /// Returns cached value on 304 Not Modified responses.
    async fn poll(&self) -> ConfigResult<AnnotatedValue> {
        let mut request = self.client.get(self.url.as_ref());

        if let Some(etag) = self.last_etag.load().as_ref() {
            request = request.header("If-None-Match", etag.as_str());
        }

        if let Some(modified) = self.last_modified.load().as_ref() {
            request = request.header("If-Modified-Since", modified.as_str());
        }

        let response = request
            .send()
            .await
            .map_err(|e| ConfigError::RemoteUnavailable {
                error_type: std::any::type_name::<reqwest::Error>().to_string(),
                retryable: is_retryable_error(&e),
            })?;

        let status = response.status();

        if status == reqwest::StatusCode::NOT_MODIFIED {
            if let Some(cached) = self.cached.read().unwrap().as_ref() {
                return Ok(cached.clone());
            }
            return Err(ConfigError::RemoteUnavailable {
                error_type: "NoCachedValue".to_string(),
                retryable: false,
            });
        }

        if !status.is_success() {
            return Err(ConfigError::RemoteUnavailable {
                error_type: format!("HTTP_{}", status.as_u16()),
                retryable: status.is_server_error() || status.as_u16() == 429,
            });
        }

        if let Some(etag) = response.headers().get("etag") {
            if let Ok(etag_str) = etag.to_str() {
                self.last_etag.store(Arc::new(Some(etag_str.to_string())));
            }
        }

        if let Some(modified) = response.headers().get("last-modified") {
            if let Ok(modified_str) = modified.to_str() {
                self.last_modified
                    .store(Arc::new(Some(modified_str.to_string())));
            }
        }

        let body = response
            .text()
            .await
            .map_err(|e| ConfigError::RemoteUnavailable {
                error_type: std::any::type_name::<reqwest::Error>().to_string(),
                retryable: is_retryable_error(&e),
            })?;

        let format = self
            .format
            .unwrap_or_else(|| detect_format_from_content(&body).unwrap_or(Format::Json));

        let source = self.source_id.clone();
        let value = parse_remote_content(&body, format, source)?;

        *self.cached.write().unwrap() = Some(value.clone());

        Ok(value)
    }

    fn poll_interval(&self) -> Option<Duration> {
        Some(self.interval)
    }

    fn source_id(&self) -> SourceId {
        self.source_id.clone()
    }
}

/// Parse content from a remote source.
fn parse_remote_content(
    content: &str,
    format: Format,
    source: SourceId,
) -> ConfigResult<AnnotatedValue> {
    match format {
        Format::Toml => parse_toml_remote(content, source),
        Format::Json => parse_json_remote(content, source),
        Format::Yaml => parse_yaml_remote(content, source),
        Format::Ini => parse_ini_remote(content, source),
    }
}

#[cfg(feature = "toml")]
fn parse_toml_remote(content: &str, source: SourceId) -> ConfigResult<AnnotatedValue> {
    use crate::loader::parse_toml;
    parse_toml(content, source, None)
}

#[cfg(not(feature = "toml"))]
fn parse_toml_remote(_content: &str, _source: SourceId) -> ConfigResult<AnnotatedValue> {
    Err(ConfigError::RemoteUnavailable {
        error_type: "TOML_NotSupported".to_string(),
        retryable: false,
    })
}

#[cfg(feature = "json")]
fn parse_json_remote(content: &str, source: SourceId) -> ConfigResult<AnnotatedValue> {
    use crate::loader::parse_json;
    parse_json(content, source, None)
}

#[cfg(not(feature = "json"))]
fn parse_json_remote(_content: &str, _source: SourceId) -> ConfigResult<AnnotatedValue> {
    Err(ConfigError::RemoteUnavailable {
        error_type: "JSON_NotSupported".to_string(),
        retryable: false,
    })
}

#[cfg(feature = "yaml")]
fn parse_yaml_remote(content: &str, source: SourceId) -> ConfigResult<AnnotatedValue> {
    use crate::loader::parse_yaml;
    parse_yaml(content, source, None)
}

#[cfg(not(feature = "yaml"))]
fn parse_yaml_remote(_content: &str, _source: SourceId) -> ConfigResult<AnnotatedValue> {
    Err(ConfigError::RemoteUnavailable {
        error_type: "YAML_NotSupported".to_string(),
        retryable: false,
    })
}

fn parse_ini_remote(content: &str, source: SourceId) -> ConfigResult<AnnotatedValue> {
    use crate::loader::parse_ini;
    parse_ini(content, source, None)
}

/// Check if a reqwest error is likely retryable.
fn is_retryable_error(error: &reqwest::Error) -> bool {
    if error.is_timeout() || error.is_connect() {
        return true;
    }
    if let Some(url) = error.url() {
        if url.host_str() == Some("localhost") || url.host_str() == Some("127.0.0.1") {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_polled_source_builder() {
        let source = HttpPolledSourceBuilder::new()
            .url("https://example.com/config.json")
            .interval(Duration::from_secs(30))
            .build()
            .unwrap();

        assert_eq!(source.poll_interval(), Some(Duration::from_secs(30)));
        assert_eq!(
            source.source_id().as_str(),
            "http:https://example.com/config.json"
        );
    }

    #[test]
    fn test_http_polled_source_builder_requires_url() {
        let result = HttpPolledSourceBuilder::new().build();
        assert!(result.is_err());
    }

    #[test]
    fn test_default_poll_interval() {
        let source = HttpPolledSourceBuilder::new()
            .url("https://example.com/config.json")
            .build()
            .unwrap();

        assert_eq!(source.poll_interval(), Some(DEFAULT_POLL_INTERVAL));
    }

    #[test]
    fn test_builder_pattern() {
        use crate::loader::Format;

        let source = HttpPolledSourceBuilder::new()
            .url("https://example.com/config.yaml")
            .interval(Duration::from_secs(30))
            .format(Format::Yaml)
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap();

        assert_eq!(source.poll_interval(), Some(Duration::from_secs(30)));
    }
}
