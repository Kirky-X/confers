// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Cloud KMS key providers (`cloud-kms` feature).
//!
//! MVP backend: HashiCorp Vault **transit** — the wrapped data key is stored
//! next to the configuration, and the provider unwraps it through
//! `POST /v1/transit/decrypt/{key}` on every (cache-policy mediated) fetch.
//! This keeps the plaintext master key out of environment variables and
//! files while delegating all key material handling to Vault.
//!
//! AWS KMS / GCP KMS / Azure KeyVault plug into the same [`CloudKmsBackend`]
//! port; they are documented extension points, not implementations yet.

use std::time::Duration;

use crate::error::{ConfigError, ConfigResult};
use crate::types::ZeroizingBytes;

/// Vendor marker for cloud KMS backends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloudKmsVendor {
    /// HashiCorp Vault transit engine (implemented: [`VaultTransitKeyProvider`]).
    Vault,
    /// AWS KMS (extension point, not implemented yet).
    Aws,
    /// GCP KMS (extension point, not implemented yet).
    Gcp,
}

/// Cloud KMS backend port (extension point for AWS/GCP/Azure backends).
///
/// A backend unwraps a ciphertext blob into raw key bytes. The async surface
/// mirrors [`crate::interface::AsyncKeyProvider`]; implement this trait to
/// add a vendor, then adapt it into a key provider.
#[async_trait::async_trait]
pub trait CloudKmsBackend: Send + Sync {
    /// Stable backend identifier (for logs and diagnostics).
    fn vendor(&self) -> CloudKmsVendor;

    /// Unwrap `ciphertext` into raw key bytes.
    async fn decrypt(&self, ciphertext: &str) -> ConfigResult<Vec<u8>>;
}

/// Returns `true` for HTTP statuses worth retrying (429/502/503/504).
fn is_retryable_status(status: u16) -> bool {
    status == 429 || status == 502 || status == 503 || status == 504
}

/// Vault transit key provider (MVP).
///
/// Unwraps the configured ciphertext through the transit engine and returns
/// the first 32 bytes as the master key.
pub struct VaultTransitKeyProvider {
    vault_addr: String,
    /// Transit key name inside the `transit/` engine.
    transit_key: String,
    /// Wrapped key ciphertext (Vault format, `vault:v1:...`).
    ciphertext: String,
    /// Optional Vault namespace header.
    namespace: Option<String>,
    token: Option<String>,
    /// Allow plain HTTP (loopback tests only — production must use TLS).
    allow_http: bool,
    cache_policy: crate::types::KeyCachePolicy,
}

impl VaultTransitKeyProvider {
    /// Create a provider for the transit key `transit_key` unwrapping
    /// `ciphertext`.
    pub fn new(
        vault_addr: impl Into<String>,
        transit_key: impl Into<String>,
        ciphertext: impl Into<String>,
    ) -> ConfigResult<Self> {
        let addr = vault_addr.into();
        if !addr.starts_with("https://") && !addr.starts_with("http://") {
            return Err(ConfigError::KeyError {
                message: "Vault address must be an http(s) URL".to_string(),
            });
        }
        Ok(Self {
            vault_addr: addr,
            transit_key: transit_key.into(),
            ciphertext: ciphertext.into(),
            namespace: None,
            token: None,
            allow_http: false,
            cache_policy: crate::types::KeyCachePolicy::default(),
        })
    }

    /// Builder entry point.
    pub fn builder() -> VaultTransitKeyProviderBuilder {
        VaultTransitKeyProviderBuilder::new()
    }

    /// Set the Vault token (falls back to `VAULT_TOKEN`).
    pub fn with_token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(token.into());
        self
    }

    /// Set the Vault enterprise namespace header.
    pub fn with_namespace(mut self, namespace: impl Into<String>) -> Self {
        self.namespace = Some(namespace.into());
        self
    }

    /// Allow plain HTTP (loopback mock tests only).
    pub fn allow_http(mut self, allow: bool) -> Self {
        self.allow_http = allow;
        self
    }

    /// Override the cache policy.
    pub fn with_cache_policy(mut self, policy: crate::types::KeyCachePolicy) -> Self {
        self.cache_policy = policy;
        self
    }

    fn get_token(&self) -> ConfigResult<String> {
        if let Some(ref token) = self.token {
            return Ok(token.clone());
        }
        std::env::var("VAULT_TOKEN").map_err(|_| ConfigError::KeyError {
            message: "Vault token not provided".to_string(),
        })
    }

    fn validate_addr(&self) -> ConfigResult<()> {
        if !self.allow_http && !self.vault_addr.starts_with("https://") {
            return Err(ConfigError::KeyError {
                message: "Vault address must use HTTPS for security (allow_http overrides this for loopback tests)".to_string(),
            });
        }
        Ok(())
    }
}

#[async_trait::async_trait]
impl CloudKmsBackend for VaultTransitKeyProvider {
    fn vendor(&self) -> CloudKmsVendor {
        CloudKmsVendor::Vault
    }

    async fn decrypt(&self, ciphertext: &str) -> ConfigResult<Vec<u8>> {
        self.validate_addr()?;
        let token = self.get_token()?;

        let client = reqwest::Client::new();
        let url = format!(
            "{}/v1/transit/decrypt/{}",
            self.vault_addr.trim_end_matches('/'),
            self.transit_key
        );

        let response = client
            .post(&url)
            .header("X-Vault-Token", token)
            .header(
                "X-Vault-Namespace",
                self.namespace.clone().unwrap_or_default(),
            )
            .json(&serde_json::json!({ "ciphertext": ciphertext }))
            .send()
            .await
            .map_err(|e| ConfigError::RemoteUnavailable {
                error_type: format!("vault_transit_request: {e}"),
                retryable: true,
            })?;

        let status = response.status();
        if !status.is_success() {
            return Err(ConfigError::RemoteUnavailable {
                error_type: format!("vault_transit_response: {status}"),
                retryable: is_retryable_status(status.as_u16()),
            });
        }

        let json: serde_json::Value =
            response.json().await.map_err(|e| ConfigError::ParseError {
                format: "json".to_string(),
                message: format!("Failed to parse Vault transit response: {e}"),
                location: None,
                source: None,
            })?;

        let plaintext_b64 = json
            .get("data")
            .and_then(|d| d.get("plaintext"))
            .and_then(|v| v.as_str())
            .ok_or(ConfigError::KeyError {
                message: "Vault transit response missing data.plaintext".to_string(),
            })?;

        use base64::Engine;
        base64::engine::general_purpose::STANDARD
            .decode(plaintext_b64)
            .map_err(|e| ConfigError::KeyError {
                message: format!("Vault transit plaintext is not valid base64: {e}"),
            })
    }
}

#[async_trait::async_trait]
impl crate::interface::AsyncKeyProvider for VaultTransitKeyProvider {
    async fn get_key(&self) -> ConfigResult<ZeroizingBytes> {
        let bytes = CloudKmsBackend::decrypt(self, &self.ciphertext).await?;
        if bytes.len() < 32 {
            return Err(ConfigError::KeyError {
                message: format!(
                    "Vault transit key too short: got {} bytes, need at least 32",
                    bytes.len()
                ),
            });
        }
        Ok(ZeroizingBytes::new(bytes[..32].to_vec()))
    }

    fn provider_type(&self) -> &'static str {
        "vault-transit"
    }

    fn ttl(&self) -> Option<Duration> {
        None
    }

    fn cache_policy(&self) -> crate::types::KeyCachePolicy {
        self.cache_policy
    }
}

/// Builder for [`VaultTransitKeyProvider`].
pub struct VaultTransitKeyProviderBuilder {
    vault_addr: Option<String>,
    transit_key: Option<String>,
    ciphertext: Option<String>,
    token: Option<String>,
    namespace: Option<String>,
    allow_http: bool,
}

impl VaultTransitKeyProviderBuilder {
    pub fn new() -> Self {
        Self {
            vault_addr: None,
            transit_key: None,
            ciphertext: None,
            token: None,
            namespace: None,
            allow_http: false,
        }
    }

    /// Vault server address (`https://vault.internal:8200`).
    pub fn vault_addr(mut self, addr: impl Into<String>) -> Self {
        self.vault_addr = Some(addr.into());
        self
    }

    /// Transit key name.
    pub fn transit_key(mut self, key: impl Into<String>) -> Self {
        self.transit_key = Some(key.into());
        self
    }

    /// Wrapped key ciphertext (`vault:v1:...`).
    pub fn ciphertext(mut self, ciphertext: impl Into<String>) -> Self {
        self.ciphertext = Some(ciphertext.into());
        self
    }

    /// Vault token (falls back to `VAULT_TOKEN`).
    pub fn token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(token.into());
        self
    }

    /// Vault enterprise namespace.
    pub fn namespace(mut self, namespace: impl Into<String>) -> Self {
        self.namespace = Some(namespace.into());
        self
    }

    /// Allow plain HTTP (loopback mock tests only).
    pub fn allow_http(mut self, allow: bool) -> Self {
        self.allow_http = allow;
        self
    }

    pub fn build(self) -> ConfigResult<VaultTransitKeyProvider> {
        let addr = self.vault_addr.ok_or(ConfigError::KeyError {
            message: "vault_addr is required".to_string(),
        })?;
        let key = self.transit_key.ok_or(ConfigError::KeyError {
            message: "transit_key is required".to_string(),
        })?;
        let ct = self.ciphertext.ok_or(ConfigError::KeyError {
            message: "ciphertext is required".to_string(),
        })?;
        let mut provider = VaultTransitKeyProvider::new(addr, key, ct)?;
        provider.token = self.token;
        provider.namespace = self.namespace;
        provider.allow_http = self.allow_http;
        Ok(provider)
    }
}

impl Default for VaultTransitKeyProviderBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interface::AsyncKeyProvider;

    fn wrapped_key() -> String {
        // Simulates a wrapped 32-byte key: `vault:v1:<base64>`.
        use base64::Engine;
        let key = [0x42u8; 32];
        format!(
            "vault:v1:{}",
            base64::engine::general_purpose::STANDARD.encode(key)
        )
    }

    #[tokio::test]
    async fn transit_decrypt_unwraps_key_against_mock_server() {
        use base64::Engine;

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let addr = listener.local_addr().unwrap();

        // The mock answers the transit decrypt call with the base64 key.
        let plaintext_b64 = base64::engine::general_purpose::STANDARD.encode([0x42u8; 32]);
        let body = serde_json::json!({"data": {"plaintext": plaintext_b64}}).to_string();

        let server = tokio::spawn(async move {
            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            let (mut stream, _) = listener.accept().await.expect("accept");
            let mut buf = [0u8; 4096];
            let _ = stream.read(&mut buf).await;
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            stream.write_all(response.as_bytes()).await.expect("write");
        });

        let provider = VaultTransitKeyProvider::builder()
            .vault_addr(format!("http://{addr}"))
            .transit_key("confers-master")
            .ciphertext(wrapped_key())
            .token("test-token")
            .allow_http(true)
            .build()
            .expect("build provider");

        let key = provider.get_key().await.expect("unwrap key");
        server.abort();

        assert_eq!(key.as_slice(), &[0x42u8; 32], "unwrapped key bytes");
        assert_eq!(provider.provider_type(), "vault-transit");
    }

    #[tokio::test]
    async fn transit_error_status_maps_to_retryable_error() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let addr = listener.local_addr().unwrap();

        let server = tokio::spawn(async move {
            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            let (mut stream, _) = listener.accept().await.expect("accept");
            let mut buf = [0u8; 1024];
            let _ = stream.read(&mut buf).await;
            let body = "{\"errors\": [\"permission denied\"]}";
            let response = format!(
                "HTTP/1.1 403 Forbidden\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            stream.write_all(response.as_bytes()).await.expect("write");
        });

        let provider = VaultTransitKeyProvider::builder()
            .vault_addr(format!("http://{addr}"))
            .transit_key("confers-master")
            .ciphertext(wrapped_key())
            .token("test-token")
            .allow_http(true)
            .build()
            .expect("build");

        let err = provider.get_key().await.expect_err("must fail");
        server.abort();
        assert!(matches!(err, ConfigError::RemoteUnavailable { retryable: false, .. }));
    }

    #[tokio::test]
    async fn https_is_enforced_outside_loopback_mode() {
        let provider = VaultTransitKeyProvider::new(
            "http://vault.internal:8200",
            "confers-master",
            "vault:v1:abc",
        )
        .expect("construct");
        let err = provider
            .decrypt("vault:v1:abc")
            .await
            .expect_err("http must be rejected without allow_http");
        assert!(matches!(err, ConfigError::KeyError { .. }));
    }

    #[test]
    fn builder_requires_all_fields() {
        assert!(VaultTransitKeyProviderBuilder::new().build().is_err());
        assert!(
            VaultTransitKeyProviderBuilder::new()
                .vault_addr("https://vault:8200")
                .build()
                .is_err()
        );
    }

    #[test]
    fn aws_and_gcp_are_documented_extension_points() {
        // Marker port check: vendors enumerate; only Vault is implemented.
        let vendors = [CloudKmsVendor::Vault, CloudKmsVendor::Aws, CloudKmsVendor::Gcp];
        assert_eq!(vendors.len(), 3);
        assert_ne!(vendors[0], CloudKmsVendor::Aws);
    }
}
