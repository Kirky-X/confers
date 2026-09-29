// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! GCP Cloud KMS key provider (`cloud-kms` feature).
//!
//! Unwraps a base64 ciphertext through
//! `projects/…/locations/…/keyRings/…/cryptoKeys/<key>:decrypt`. The access
//! token comes from the GCE metadata server
//! (`Metadata-Flavor: Google` handshake — required, its absence is exactly
//! how SSRF probes are told apart from real metadata traffic) or is injected
//! directly for loopback mock tests. The metadata and KMS endpoints can be
//! overridden so the contract tests run against a loopback mock server;
//! real-cluster coverage is gated, see the live test at the bottom.

use std::sync::Arc;
use std::time::Duration;

use crate::error::{ConfigError, ConfigResult};
use crate::types::ZeroizingBytes;

use super::providers_cloud::{
    CloudKmsBackend, CloudKmsVendor, endpoint_is_loopback, is_retryable_status,
};

/// Default GCE metadata server.
const METADATA_ENDPOINT: &str = "http://metadata.google.internal";
/// Default Cloud KMS API root.
const KMS_ENDPOINT: &str = "https://cloudkms.googleapis.com";

/// Validate a Cloud KMS key resource path
/// (`projects/<p>/locations/<l>/keyRings/<r>/cryptoKeys/<k>`) before it is
/// interpolated into the request URL: every segment must be a plain GCP
/// resource id (letters, digits, `-`, `_`), which rules out traversal,
/// escapes and query/fragment injection.
fn validate_key_resource(resource: &str) -> ConfigResult<()> {
    const SEGMENT_KEYS: [&str; 4] = ["projects", "locations", "keyRings", "cryptoKeys"];
    let segments: Vec<&str> = resource.split('/').collect();
    let ok = segments.len() == 8
        && segments
            .chunks(2)
            .zip(SEGMENT_KEYS.iter())
            .all(|(pair, prefix)| pair[0] == *prefix && is_plain_id(pair[1]));
    if ok {
        Ok(())
    } else {
        Err(ConfigError::KeyError {
            message: format!(
                "GCP KMS key resource must be `projects/<id>/locations/<id>/keyRings/<id>/cryptoKeys/<id>` with plain [A-Za-z0-9_-] ids, got '{resource}'"
            ),
        })
    }
}

fn is_plain_id(segment: &str) -> bool {
    !segment.is_empty()
        && segment
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-'))
}

/// GCP Cloud KMS key provider.
pub struct GcpKmsKeyProvider {
    /// `projects/<p>/locations/<l>/keyRings/<r>/cryptoKeys/<k>`.
    key_resource: String,
    /// Base64 ciphertext stored next to the configuration.
    ciphertext: String,
    /// Metadata server root (overridable for loopback mock tests).
    metadata_endpoint: String,
    /// Cloud KMS API root (overridable for loopback mock tests).
    kms_endpoint: String,
    /// Directly injected access token (tests / pre-fetched credentials);
    /// when absent the token is fetched from the metadata server and cached
    /// until shortly before its `expires_in` lapses.
    ///
    /// Memory-hygiene note: the token string lives (uncached or in the
    /// cache) for the provider's lifetime and is not zeroized on drop —
    /// tokens are short-lived by design, bounding the exposure window.
    access_token: Option<String>,
    /// Allow plain HTTP on the KMS endpoint (loopback mock tests only — the
    /// metadata server is always plain HTTP by GCP design).
    allow_http: bool,
    cache_policy: crate::types::KeyCachePolicy,
    /// Cached metadata token: `(token, expires_at_unix)` — refreshed before
    /// expiry instead of once per decrypt (mirrors the Vault lease cache).
    token_cache: Arc<std::sync::Mutex<Option<(String, u64)>>>,
}

impl GcpKmsKeyProvider {
    /// Create a provider for the key resource unwrapping `ciphertext`.
    pub fn new(
        project: impl AsRef<str>,
        location: impl AsRef<str>,
        key_ring: impl AsRef<str>,
        key: impl AsRef<str>,
        ciphertext: impl Into<String>,
    ) -> ConfigResult<Self> {
        let resource = format!(
            "projects/{}/locations/{}/keyRings/{}/cryptoKeys/{}",
            project.as_ref(),
            location.as_ref(),
            key_ring.as_ref(),
            key.as_ref()
        );
        validate_key_resource(&resource)?;
        Ok(Self {
            key_resource: resource,
            ciphertext: ciphertext.into(),
            metadata_endpoint: METADATA_ENDPOINT.to_string(),
            kms_endpoint: KMS_ENDPOINT.to_string(),
            access_token: None,
            allow_http: false,
            cache_policy: crate::types::KeyCachePolicy::default(),
            token_cache: Arc::new(std::sync::Mutex::new(None)),
        })
    }

    /// Builder entry point.
    pub fn builder() -> GcpKmsKeyProviderBuilder {
        GcpKmsKeyProviderBuilder::new()
    }

    fn kms_root(&self) -> ConfigResult<String> {
        let endpoint = self.kms_endpoint.trim_end_matches('/').to_string();
        if endpoint.starts_with("https://") {
            return Ok(endpoint);
        }
        if !self.allow_http {
            return Err(ConfigError::KeyError {
                message:
                    "GCP KMS endpoint must use HTTPS (allow_http overrides this for loopback tests)"
                        .to_string(),
            });
        }
        // allow_http is documented as loopback-mock-only.
        if !endpoint_is_loopback(&endpoint) {
            return Err(ConfigError::KeyError {
                message: format!(
                    "allow_http permits plain HTTP on loopback hosts only, got '{endpoint}'"
                ),
            });
        }
        Ok(endpoint)
    }

    /// Fetch an access token from the metadata server. The
    /// `Metadata-Flavor: Google` header is mandatory — the real metadata
    /// endpoint refuses requests without it, which is what distinguishes
    /// instance metadata traffic from SSRF probes against arbitrary URLs.
    /// Resolve the access token for this decrypt: the injected token wins;
    /// otherwise fetch from the metadata server and cache until shortly
    /// before `expires_in` lapses (mirrors the Vault lease cache).
    async fn access_token(&self) -> ConfigResult<String> {
        if let Some(token) = &self.access_token {
            return Ok(token.clone());
        }
        if let Some((token, expires_at)) = self
            .token_cache
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .clone()
        {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                // A broken clock simply misses the cache and refetches.
                .unwrap_or(0);
            if now + 60 < expires_at {
                return Ok(token);
            }
        }
        let (token, expires_in) = self.fetch_metadata_token().await?;
        let expires_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() + expires_in)
            // Broken clock: cache nothing (refetch next time).
            .unwrap_or(0);
        if expires_at > 0 {
            *self.token_cache.lock().unwrap_or_else(|p| p.into_inner()) =
                Some((token.clone(), expires_at));
        }
        Ok(token)
    }

    async fn fetch_metadata_token(&self) -> ConfigResult<(String, u64)> {
        let client = crate::secret::providers::shared_http_client();
        let url = format!(
            "{}/computeMetadata/v1/instance/service-accounts/default/token",
            self.metadata_endpoint.trim_end_matches('/')
        );
        let response = client
            .get(&url)
            .header("Metadata-Flavor", "Google")
            .send()
            .await
            .map_err(|e| ConfigError::RemoteUnavailable {
                error_type: format!("gcp_metadata_request: {e}"),
                retryable: true,
            })?;
        let status = response.status();
        if !status.is_success() {
            return Err(ConfigError::RemoteUnavailable {
                error_type: format!("gcp_metadata_response: {status}"),
                retryable: is_retryable_status(status.as_u16()),
            });
        }
        let json: serde_json::Value =
            response.json().await.map_err(|e| ConfigError::ParseError {
                format: "json".to_string(),
                message: format!("Failed to parse GCP metadata token response: {e}"),
                location: None,
                source: None,
            })?;
        let token = json
            .get("access_token")
            .and_then(|v| v.as_str())
            .ok_or(ConfigError::KeyError {
                message: "GCP metadata response missing access_token".to_string(),
            })?
            .to_string();
        let expires_in = json.get("expires_in").and_then(|v| v.as_u64()).unwrap_or(0);
        Ok((token, expires_in))
    }
}

#[async_trait::async_trait]
impl CloudKmsBackend for GcpKmsKeyProvider {
    fn vendor(&self) -> CloudKmsVendor {
        CloudKmsVendor::Gcp
    }

    async fn decrypt(&self, ciphertext: &str) -> ConfigResult<ZeroizingBytes> {
        let kms_root = self.kms_root()?;
        let token = self.access_token().await?;

        let client = crate::secret::providers::shared_http_client();
        let url = format!("{}/v1/{}:decrypt", kms_root, self.key_resource);
        let payload = serde_json::json!({ "ciphertext": ciphertext }).to_string();
        let response = client
            .post(&url)
            .header("Authorization", format!("Bearer {token}"))
            .header("Content-Type", "application/json")
            .body(payload)
            .send()
            .await
            .map_err(|e| ConfigError::RemoteUnavailable {
                error_type: format!("gcp_kms_request: {e}"),
                retryable: true,
            })?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(ConfigError::RemoteUnavailable {
                error_type: format!("gcp_kms_response: {status} {body}"),
                retryable: is_retryable_status(status.as_u16()),
            });
        }

        let json: serde_json::Value =
            response.json().await.map_err(|e| ConfigError::ParseError {
                format: "json".to_string(),
                message: format!("Failed to parse GCP KMS response: {e}"),
                location: None,
                source: None,
            })?;
        let plaintext_b64 =
            json.get("plaintext")
                .and_then(|v| v.as_str())
                .ok_or(ConfigError::KeyError {
                    message: "GCP KMS response missing plaintext".to_string(),
                })?;

        use base64::Engine;
        base64::engine::general_purpose::STANDARD
            .decode(plaintext_b64)
            .map(ZeroizingBytes::new)
            .map_err(|e| ConfigError::KeyError {
                message: format!("GCP KMS plaintext is not valid base64: {e}"),
            })
    }
}

#[async_trait::async_trait]
impl crate::interface::AsyncKeyProvider for GcpKmsKeyProvider {
    async fn get_key(&self) -> ConfigResult<ZeroizingBytes> {
        let bytes = CloudKmsBackend::decrypt(self, &self.ciphertext).await?;
        if bytes.len() < 32 {
            return Err(ConfigError::KeyError {
                message: format!(
                    "GCP KMS key too short: got {} bytes, need at least 32",
                    bytes.len()
                ),
            });
        }
        Ok(ZeroizingBytes::new(bytes.as_slice()[..32].to_vec()))
    }

    fn provider_type(&self) -> &'static str {
        "gcp-kms"
    }

    fn ttl(&self) -> Option<Duration> {
        None
    }

    fn cache_policy(&self) -> crate::types::KeyCachePolicy {
        self.cache_policy
    }
}

/// Builder for [`GcpKmsKeyProvider`].
pub struct GcpKmsKeyProviderBuilder {
    project: Option<String>,
    location: Option<String>,
    key_ring: Option<String>,
    key: Option<String>,
    ciphertext: Option<String>,
    metadata_endpoint: Option<String>,
    kms_endpoint: Option<String>,
    access_token: Option<String>,
    allow_http: bool,
    cache_policy: crate::types::KeyCachePolicy,
}

impl GcpKmsKeyProviderBuilder {
    pub fn new() -> Self {
        Self {
            project: None,
            location: None,
            key_ring: None,
            key: None,
            ciphertext: None,
            metadata_endpoint: None,
            kms_endpoint: None,
            access_token: None,
            allow_http: false,
            cache_policy: crate::types::KeyCachePolicy::default(),
        }
    }

    /// GCP project id.
    pub fn project(mut self, project: impl Into<String>) -> Self {
        self.project = Some(project.into());
        self
    }

    /// Key location (e.g. `global`, `us-east1`).
    pub fn location(mut self, location: impl Into<String>) -> Self {
        self.location = Some(location.into());
        self
    }

    /// Key ring name.
    pub fn key_ring(mut self, key_ring: impl Into<String>) -> Self {
        self.key_ring = Some(key_ring.into());
        self
    }

    /// Crypto key name.
    pub fn key(mut self, key: impl Into<String>) -> Self {
        self.key = Some(key.into());
        self
    }

    /// Base64 ciphertext stored next to the configuration.
    pub fn ciphertext(mut self, ciphertext: impl Into<String>) -> Self {
        self.ciphertext = Some(ciphertext.into());
        self
    }

    /// Metadata server root override (loopback mock tests).
    pub fn metadata_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.metadata_endpoint = Some(endpoint.into());
        self
    }

    /// Cloud KMS API root override (plain HTTP on loopback hosts only).
    pub fn kms_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.kms_endpoint = Some(endpoint.into());
        self
    }

    /// Directly inject an access token (skips the metadata server).
    pub fn access_token(mut self, token: impl Into<String>) -> Self {
        self.access_token = Some(token.into());
        self
    }

    /// Allow plain HTTP on the KMS endpoint (loopback mock tests only).
    pub fn allow_http(mut self, allow: bool) -> Self {
        self.allow_http = allow;
        self
    }

    /// Override the cache policy.
    pub fn with_cache_policy(mut self, policy: crate::types::KeyCachePolicy) -> Self {
        self.cache_policy = policy;
        self
    }

    pub fn build(self) -> ConfigResult<GcpKmsKeyProvider> {
        let project = self.project.ok_or(ConfigError::KeyError {
            message: "project is required".to_string(),
        })?;
        let location = self.location.ok_or(ConfigError::KeyError {
            message: "location is required".to_string(),
        })?;
        let key_ring = self.key_ring.ok_or(ConfigError::KeyError {
            message: "key_ring is required".to_string(),
        })?;
        let key = self.key.ok_or(ConfigError::KeyError {
            message: "key is required".to_string(),
        })?;
        let ciphertext = self.ciphertext.ok_or(ConfigError::KeyError {
            message: "ciphertext is required".to_string(),
        })?;
        let mut provider = GcpKmsKeyProvider::new(project, location, key_ring, key, ciphertext)?;
        if let Some(endpoint) = self.metadata_endpoint {
            provider.metadata_endpoint = endpoint;
        }
        if let Some(endpoint) = self.kms_endpoint {
            provider.kms_endpoint = endpoint;
        }
        provider.access_token = self.access_token;
        provider.allow_http = self.allow_http;
        provider.cache_policy = self.cache_policy;
        Ok(provider)
    }
}

impl Default for GcpKmsKeyProviderBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interface::AsyncKeyProvider;

    #[test]
    fn key_resource_injection_is_rejected() {
        // Every malformed shape fails at construction time, before any URL
        // is assembled: traversal segments, escapes, empty ids, wrong depth.
        for (project, location, ring, key) in [
            ("p", "l", "r", "k/../x"), // traversal in the key id
            ("p%2f", "l", "r", "k"),   // escape in the project id
            ("", "l", "r", "k"),       // empty project id
            ("p", "l", "r/../r", "k"), // traversal in the ring
        ] {
            assert!(
                GcpKmsKeyProvider::new(project, location, ring, key, "AAECAw==").is_err(),
                "resource {project}/{location}/{ring}/{key} must be rejected"
            );
        }
        assert!(
            validate_key_resource(
                "projects/my-proj/locations/global/keyRings/confers/cryptoKeys/master-key_1"
            )
            .is_ok(),
            "plain resource ids remain accepted"
        );
    }

    /// Full decrypt flow against loopback mocks: the metadata server must
    /// see the `Metadata-Flavor` header (and nothing else hits it), the KMS
    /// endpoint must see the Bearer token from the metadata answer.
    #[tokio::test]
    async fn decrypt_fetches_metadata_token_and_unwraps_key() {
        use base64::Engine;
        use std::sync::{Arc, Mutex};

        let metadata_listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind metadata");
        let kms_listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind kms");
        let metadata_addr = metadata_listener.local_addr().unwrap();
        let kms_addr = kms_listener.local_addr().unwrap();

        let saw_flavor = Arc::new(Mutex::new(None::<String>));
        let flavor_capture = Arc::clone(&saw_flavor);
        let metadata_server = tokio::spawn(async move {
            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            let (mut stream, _) = metadata_listener.accept().await.expect("accept");
            let mut buf = [0u8; 4096];
            let n = stream.read(&mut buf).await.expect("read");
            let request = String::from_utf8_lossy(&buf[..n]).to_string();
            for line in request.lines() {
                // reqwest lowercases header names on the wire.
                if line.to_lowercase().starts_with("metadata-flavor: ") {
                    *flavor_capture.lock().unwrap() =
                        Some(line["metadata-flavor: ".len()..].to_string());
                }
            }
            let body = r#"{"access_token":"mock-access-token","expires_in":3599}"#;
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            stream.write_all(response.as_bytes()).await.expect("write");
        });

        let seen_bearer = Arc::new(Mutex::new(None::<String>));
        let bearer_capture = Arc::clone(&seen_bearer);
        let plaintext_b64 = base64::engine::general_purpose::STANDARD.encode([0x5au8; 32]);
        let kms_body = serde_json::json!({ "plaintext": plaintext_b64 }).to_string();
        let kms_server = tokio::spawn(async move {
            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            let (mut stream, _) = kms_listener.accept().await.expect("accept");
            let mut buf = vec![0u8; 8192];
            let n = stream.read(&mut buf).await.expect("read");
            let request = String::from_utf8_lossy(&buf[..n]).to_string();
            for line in request.lines() {
                if line.to_lowercase().starts_with("authorization: ") {
                    *bearer_capture.lock().unwrap() =
                        Some(line["authorization: ".len()..].to_string());
                }
            }
            assert!(
                request.contains("/v1/projects/my-proj/locations/global/keyRings/confers/cryptoKeys/master-key_1:decrypt"),
                "decrypt must hit the assembled key resource path"
            );
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                kms_body.len(),
                kms_body
            );
            stream.write_all(response.as_bytes()).await.expect("write");
        });

        let provider = GcpKmsKeyProvider::builder()
            .project("my-proj")
            .location("global")
            .key_ring("confers")
            .key("master-key_1")
            .ciphertext(base64::engine::general_purpose::STANDARD.encode([0x33u8; 40]))
            .metadata_endpoint(format!("http://{metadata_addr}"))
            .kms_endpoint(format!("http://{kms_addr}"))
            .allow_http(true)
            .build()
            .expect("build provider");

        let key = provider.get_key().await.expect("unwrap key");
        metadata_server.abort();
        kms_server.abort();

        assert_eq!(key.as_slice(), &[0x5au8; 32]);
        assert_eq!(provider.provider_type(), "gcp-kms");
        assert_eq!(
            saw_flavor.lock().unwrap().as_deref(),
            Some("Google"),
            "metadata handshake requires the flavor header"
        );
        assert_eq!(
            seen_bearer.lock().unwrap().as_deref(),
            Some("Bearer mock-access-token"),
            "KMS call must carry the token from metadata"
        );
    }

    #[tokio::test]
    async fn injected_token_skips_metadata_server() {
        use base64::Engine;

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let addr = listener.local_addr().unwrap();
        let plaintext_b64 = base64::engine::general_purpose::STANDARD.encode([0x11u8; 32]);
        let body = serde_json::json!({ "plaintext": plaintext_b64 }).to_string();

        let server = tokio::spawn(async move {
            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            let (mut stream, _) = listener.accept().await.expect("accept");
            let mut buf = [0u8; 4096];
            let _ = stream.read(&mut buf).await;
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            stream.write_all(response.as_bytes()).await.expect("write");
        });

        let provider = GcpKmsKeyProvider::builder()
            .project("p")
            .location("l")
            .key_ring("r")
            .key("k")
            .ciphertext("AAECAw==")
            .access_token("injected-token")
            .kms_endpoint(format!("http://{addr}"))
            .allow_http(true)
            .build()
            .expect("build");

        let key = provider.get_key().await.expect("decrypt");
        server.abort();
        assert_eq!(key.as_slice(), &[0x11u8; 32]);
    }

    #[tokio::test]
    async fn kms_access_denied_maps_to_non_retryable() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let addr = listener.local_addr().unwrap();

        let server = tokio::spawn(async move {
            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            let (mut stream, _) = listener.accept().await.expect("accept");
            let mut buf = [0u8; 1024];
            let _ = stream.read(&mut buf).await;
            let body = "{\"error\":{\"code\":403}}";
            let response = format!(
                "HTTP/1.1 403 Forbidden\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            stream.write_all(response.as_bytes()).await.expect("write");
        });

        let provider = GcpKmsKeyProvider::builder()
            .project("p")
            .location("l")
            .key_ring("r")
            .key("k")
            .ciphertext("AAECAw==")
            .access_token("t")
            .kms_endpoint(format!("http://{addr}"))
            .allow_http(true)
            .build()
            .expect("build");

        let err = provider.get_key().await.expect_err("must fail");
        server.abort();
        assert!(matches!(
            err,
            ConfigError::RemoteUnavailable {
                retryable: false,
                ..
            }
        ));
    }

    #[tokio::test]
    async fn http_kms_endpoint_is_rejected_outside_loopback_mode() {
        let provider = GcpKmsKeyProvider::builder()
            .project("p")
            .location("l")
            .key_ring("r")
            .key("k")
            .ciphertext("AAECAw==")
            .access_token("t")
            .kms_endpoint("http://cloudkms.example.com")
            .build()
            .expect("construct");
        let err = CloudKmsBackend::decrypt(&provider, "AAECAw==")
            .await
            .expect_err("http rejected");
        assert!(matches!(err, ConfigError::KeyError { .. }));
    }

    /// A rate-limit answer must keep the retryable flag on (the caller is
    /// expected to back off and retry, not give up).
    #[tokio::test]
    async fn kms_rate_limit_maps_to_retryable() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let addr = listener.local_addr().unwrap();

        let server = tokio::spawn(async move {
            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            let (mut stream, _) = listener.accept().await.expect("accept");
            let mut buf = [0u8; 1024];
            let _ = stream.read(&mut buf).await;
            let response =
                "HTTP/1.1 429 Too Many Requests\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
            stream.write_all(response.as_bytes()).await.expect("write");
        });

        let provider = GcpKmsKeyProvider::builder()
            .project("p")
            .location("l")
            .key_ring("r")
            .key("k")
            .ciphertext("AAECAw==")
            .access_token("t")
            .kms_endpoint(format!("http://{addr}"))
            .allow_http(true)
            .build()
            .expect("build");

        let err = provider.get_key().await.expect_err("must fail");
        server.abort();
        assert!(matches!(
            err,
            ConfigError::RemoteUnavailable {
                retryable: true,
                ..
            }
        ));
    }

    /// allow_http does not open the door to remote plain-HTTP hosts.
    #[tokio::test]
    async fn allow_http_rejects_remote_plain_http_hosts() {
        let provider = GcpKmsKeyProvider::builder()
            .project("p")
            .location("l")
            .key_ring("r")
            .key("k")
            .ciphertext("AAECAw==")
            .access_token("t")
            .kms_endpoint("http://cloudkms.internal.example.com")
            .allow_http(true)
            .build()
            .expect("construct");
        let err = CloudKmsBackend::decrypt(&provider, "AAECAw==")
            .await
            .expect_err("remote plain HTTP must be rejected");
        assert!(
            matches!(&err, ConfigError::KeyError { message } if message.contains("loopback")),
            "{err:?}"
        );
    }

    /// Live-cluster integration: gated behind `CONFERS_GCP_KMS_LIVE=1` plus
    /// real `GOOGLE_APPLICATION_CREDENTIALS`/ADC and real key material —
    /// sandboxed CI has neither, so it stays skipped by default
    /// (`#[ignore]` keeps it out of normal runs even when the env is set by
    /// accident).
    #[tokio::test]
    #[ignore = "live GCP KMS: requires CONFERS_GCP_KMS_LIVE=1, ADC credentials and a real cryptoKey"]
    async fn live_gcp_kms_decrypt() {
        if std::env::var("CONFERS_GCP_KMS_LIVE").as_deref() != Ok("1") {
            println!("skipping live GCP KMS decrypt: CONFERS_GCP_KMS_LIVE != 1");
            return;
        }
        let (project, location, ring, key) = (
            std::env::var("CONFERS_GCP_KMS_PROJECT").expect("CONFERS_GCP_KMS_PROJECT"),
            std::env::var("CONFERS_GCP_KMS_LOCATION").expect("CONFERS_GCP_KMS_LOCATION"),
            std::env::var("CONFERS_GCP_KMS_KEY_RING").expect("CONFERS_GCP_KMS_KEY_RING"),
            std::env::var("CONFERS_GCP_KMS_KEY").expect("CONFERS_GCP_KMS_KEY"),
        );
        let ciphertext =
            std::env::var("CONFERS_GCP_KMS_CIPHERTEXT").expect("CONFERS_GCP_KMS_CIPHERTEXT");
        let provider = GcpKmsKeyProvider::new(project, location, ring, key, ciphertext)
            .expect("valid key resource");
        let bytes = CloudKmsBackend::decrypt(&provider, &provider.ciphertext)
            .await
            .expect("live decrypt");
        assert!(bytes.len() >= 32, "live key must be usable master material");
    }
}
