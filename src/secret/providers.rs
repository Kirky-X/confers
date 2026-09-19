// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! Additional key providers for secret management.

use std::path::PathBuf;

use crate::error::{ConfigError, ConfigResult};
use crate::interface::KeyProvider;
use crate::types::{KeyCachePolicy, ZeroizingBytes};

#[cfg(feature = "remote")]
use crate::interface::AsyncKeyProvider;

/// Shared HTTP client for the Vault-backed key providers.
///
/// Building a fresh `reqwest::Client` per request defeats connection pooling
/// and inherits reqwest's "no timeout" default, so an unreachable endpoint
/// would hang the calling task forever. One process-wide client with a
/// bounded request timeout is shared instead (`Client` clones are cheap).
#[cfg(feature = "remote")]
pub(crate) fn shared_http_client() -> reqwest::Client {
    static CLIENT: std::sync::OnceLock<reqwest::Client> = std::sync::OnceLock::new();
    CLIENT
        .get_or_init(|| {
            reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_default()
        })
        .clone()
}

/// Key provider that reads a 32-byte key from a local file.
///
/// # Key material semantics
///
/// The file content must be valid UTF-8. Leading and trailing whitespace is
/// trimmed, then the key is taken deterministically as the **first 32 bytes**
/// of the remaining content; any trailing data is ignored. No base64/hex
/// decoding is performed: the raw bytes of the text are used directly as key
/// material. Store the key file as ASCII (for example the base64 or hex
/// output of `openssl rand -base64 32` / `openssl rand -hex 32`) so the first
/// 32 bytes are well-defined and stable across editors and platforms.
pub struct FileKeyProvider {
    path: PathBuf,
    cache_policy: KeyCachePolicy,
}

impl FileKeyProvider {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            cache_policy: KeyCachePolicy::default(),
        }
    }

    pub fn builder() -> FileKeyProviderBuilder {
        FileKeyProviderBuilder::new()
    }

    pub fn with_cache_policy(mut self, policy: KeyCachePolicy) -> Self {
        self.cache_policy = policy;
        self
    }

    fn read_key_from_file(&self) -> ConfigResult<Vec<u8>> {
        let content = std::fs::read(&self.path).map_err(|e| ConfigError::FileNotFound {
            filename: self.path.clone(),
            source: Some(e),
        })?;

        let key_str = String::from_utf8(content).map_err(|_| ConfigError::InvalidValue {
            key: self.path.to_string_lossy().to_string(),
            expected_type: "utf8 string".to_string(),
            message: "Key file contains non-UTF8 content".to_string(),
        })?;

        let key_str = key_str.trim();

        if key_str.len() < 32 {
            return Err(ConfigError::KeyError {
                message: "Key file must contain at least 32 characters".to_string(),
            });
        }

        Ok(key_str.as_bytes()[..32].to_vec())
    }
}

impl KeyProvider for FileKeyProvider {
    fn get_key(&self) -> ConfigResult<ZeroizingBytes> {
        let key = self.read_key_from_file()?;
        Ok(ZeroizingBytes::new(key))
    }

    fn provider_type(&self) -> &'static str {
        "file"
    }

    fn cache_policy(&self) -> KeyCachePolicy {
        self.cache_policy
    }
}

pub struct FileKeyProviderBuilder {
    path: Option<PathBuf>,
    cache_policy: KeyCachePolicy,
}

impl FileKeyProviderBuilder {
    pub fn new() -> Self {
        Self {
            path: None,
            cache_policy: KeyCachePolicy::default(),
        }
    }

    pub fn path(mut self, path: impl Into<PathBuf>) -> Self {
        self.path = Some(path.into());
        self
    }

    pub fn cache_policy(mut self, policy: KeyCachePolicy) -> Self {
        self.cache_policy = policy;
        self
    }

    pub fn build(self) -> ConfigResult<FileKeyProvider> {
        let path = self.path.ok_or(ConfigError::InvalidValue {
            key: "file_key_provider_path".to_string(),
            expected_type: "PathBuf".to_string(),
            message: "Path is required for FileKeyProvider".to_string(),
        })?;

        Ok(FileKeyProvider {
            path,
            cache_policy: self.cache_policy,
        })
    }
}

impl Default for FileKeyProviderBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Key provider that fetches a 32-byte key from HashiCorp Vault.
///
/// # Token handling
///
/// The Vault token is supplied via [`VaultKeyProvider::with_token`] or, by
/// default, read from the `VAULT_TOKEN` environment variable. Passing the
/// token through the environment is the industry-standard practice (the
/// official `vault` CLI uses the same convention) and keeps the token out of
/// process arguments and config files. This module performs no logging of any
/// kind: tokens, keys, and request details are never written to logs or to
/// stdout/stderr by this code.
/// Vault authentication method.
///
/// `Token` is the static-token flow (unchanged from rc.3). `AppRole` and
/// `Kubernetes` log in against their auth mounts and exchange credentials
/// for a short-lived client token; the resolved token is cached per provider
/// so a fetch does not re-login on every call.
#[cfg(feature = "remote")]
#[derive(Clone)]
pub enum VaultAuth {
    /// Static token (or `VAULT_TOKEN` when empty).
    Token { token: String },
    /// AppRole login: `role_id` + `secret_id` exchanged at
    /// `POST /v1/auth/approle/login`.
    AppRole { role_id: String, secret_id: String },
    /// Kubernetes service-account login: pod JWT + auth role exchanged at
    /// `POST /v1/auth/kubernetes/login`.
    Kubernetes { jwt: String, role: String },
}

#[cfg(feature = "remote")]
impl VaultAuth {
    /// Kubernetes auth from the in-pod service-account token file.
    pub fn kubernetes_from_service_account(role: impl Into<String>) -> ConfigResult<Self> {
        const SA_TOKEN: &str = "/var/run/secrets/kubernetes.io/serviceaccount/token";
        let jwt = std::fs::read_to_string(SA_TOKEN).map_err(|e| ConfigError::KeyError {
            message: format!("cannot read Kubernetes service-account token at {SA_TOKEN}: {e}"),
        })?;
        Ok(Self::Kubernetes {
            jwt: jwt.trim().to_string(),
            role: role.into(),
        })
    }

    fn login_path(&self) -> Option<&'static str> {
        match self {
            Self::Token { .. } => None,
            Self::AppRole { .. } => Some("/v1/auth/approle/login"),
            Self::Kubernetes { .. } => Some("/v1/auth/kubernetes/login"),
        }
    }

    fn login_body(&self) -> serde_json::Value {
        match self {
            Self::Token { .. } => serde_json::json!({}),
            Self::AppRole { role_id, secret_id } => {
                serde_json::json!({ "role_id": role_id, "secret_id": secret_id })
            }
            Self::Kubernetes { jwt, role } => {
                serde_json::json!({ "jwt": jwt, "role": role })
            }
        }
    }

    fn describe(&self) -> &'static str {
        match self {
            Self::Token { .. } => "token",
            Self::AppRole { .. } => "approle",
            Self::Kubernetes { .. } => "kubernetes",
        }
    }

    /// Exchange credentials for a client token (no-op for `Token`).
    async fn resolve(&self, client: &reqwest::Client, vault_addr: &str) -> ConfigResult<String> {
        match self {
            Self::Token { token } => {
                if token.is_empty() {
                    std::env::var("VAULT_TOKEN").map_err(|_| ConfigError::KeyError {
                        message: "Vault token not provided".to_string(),
                    })
                } else {
                    Ok(token.clone())
                }
            }
            _ => {
                let url = format!(
                    "{}{}",
                    vault_addr.trim_end_matches('/'),
                    self.login_path().expect("login path for login method")
                );
                let response = client
                    .post(&url)
                    .json(&self.login_body())
                    .send()
                    .await
                    .map_err(|e| ConfigError::RemoteUnavailable {
                        error_type: format!("vault_login_request: {e}"),
                        retryable: true,
                    })?;
                let status = response.status();
                if !status.is_success() {
                    return Err(ConfigError::RemoteUnavailable {
                        error_type: format!("vault_login_response: {status}"),
                        retryable: is_retryable_status(status.as_u16()),
                    });
                }
                let json: serde_json::Value =
                    response.json().await.map_err(|e| ConfigError::ParseError {
                        format: "json".to_string(),
                        message: format!("Failed to parse Vault login response: {e}"),
                        location: None,
                        source: None,
                    })?;
                json.get("auth")
                    .and_then(|a| a.get("client_token"))
                    .and_then(|t| t.as_str())
                    .map(|t| t.to_string())
                    .ok_or(ConfigError::KeyError {
                        message: "Vault login response missing auth.client_token".to_string(),
                    })
            }
        }
    }
}

#[cfg(feature = "remote")]
pub struct VaultKeyProvider {
    vault_addr: String,
    secret_path: String,
    secret_key: String,
    auth: VaultAuth,
    /// Resolved client token after the first successful login.
    token_cache: std::sync::Mutex<Option<String>>,
    /// Allow plain HTTP (loopback mock tests only).
    allow_http: bool,
    cache_policy: KeyCachePolicy,
}

#[cfg(feature = "remote")]
impl VaultKeyProvider {
    pub fn new(
        vault_addr: impl Into<String>,
        secret_path: impl Into<String>,
        secret_key: impl Into<String>,
    ) -> ConfigResult<Self> {
        let addr = vault_addr.into();

        if !addr.starts_with("https://") {
            return Err(ConfigError::KeyError {
                message: "Vault address must use HTTPS for security".to_string(),
            });
        }

        Ok(Self {
            vault_addr: addr,
            secret_path: secret_path.into(),
            secret_key: secret_key.into(),
            auth: VaultAuth::Token {
                token: String::new(),
            },
            token_cache: std::sync::Mutex::new(None),
            allow_http: false,
            cache_policy: KeyCachePolicy::default(),
        })
    }

    pub fn builder() -> VaultKeyProviderBuilder {
        VaultKeyProviderBuilder::new()
    }

    pub fn with_token(mut self, token: impl Into<String>) -> Self {
        self.auth = VaultAuth::Token {
            token: token.into(),
        };
        self
    }

    /// Authenticate with AppRole / Kubernetes service-account instead of a
    /// static token (see `VaultAuth`).
    pub fn with_auth(mut self, auth: VaultAuth) -> Self {
        self.auth = auth;
        self
    }

    /// Allow plain HTTP (loopback mock tests only).
    pub fn allow_http(mut self, allow: bool) -> Self {
        self.allow_http = allow;
        self
    }

    pub fn with_cache_policy(mut self, policy: KeyCachePolicy) -> Self {
        self.cache_policy = policy;
        self
    }

    /// The active authentication method descriptor.
    pub fn auth_method(&self) -> &'static str {
        self.auth.describe()
    }

    async fn get_token(&self) -> ConfigResult<String> {
        // Fast path: token already resolved (and cached) by a previous fetch.
        if let Ok(Some(token)) = self.token_cache.lock().map(|c| c.clone()) {
            return Ok(token);
        }
        let client = shared_http_client();
        let token = self.auth.resolve(&client, &self.vault_addr).await?;
        if let Ok(mut cache) = self.token_cache.lock() {
            *cache = Some(token.clone());
        }
        Ok(token)
    }
}

/// Returns `true` for HTTP statuses that are transient and worth retrying:
/// 429 (rate limiting) and 502/503/504 (gateway/server errors). This mirrors
/// the remote polling module's retry policy, which also treats 429 as
/// retryable.
#[cfg(feature = "remote")]
fn is_retryable_status(status: u16) -> bool {
    status == 429 || status == 502 || status == 503 || status == 504
}

#[cfg(feature = "remote")]
#[async_trait::async_trait]
impl AsyncKeyProvider for VaultKeyProvider {
    async fn get_key(&self) -> ConfigResult<ZeroizingBytes> {
        if !self.allow_http && !self.vault_addr.starts_with("https://") {
            return Err(ConfigError::KeyError {
                message: "Vault address must use HTTPS for security (allow_http overrides this for loopback tests)".to_string(),
            });
        }
        let token = self.get_token().await?;

        let client = shared_http_client();
        let url = format!(
            "{}/v1/{}",
            self.vault_addr.trim_end_matches('/'),
            self.secret_path
        );

        let response = client
            .get(&url)
            .header("X-Vault-Token", token)
            .send()
            .await
            .map_err(|e| ConfigError::RemoteUnavailable {
                error_type: format!("vault_request: {}", e),
                retryable: true,
            })?;

        if !response.status().is_success() {
            let status = response.status();
            // 429 (rate limit) and 502/503/504 are transient errors that may
            // succeed on retry.
            let retryable = is_retryable_status(status.as_u16());
            return Err(ConfigError::RemoteUnavailable {
                error_type: format!("vault_response: {}", status),
                retryable,
            });
        }

        let json: serde_json::Value =
            response.json().await.map_err(|e| ConfigError::ParseError {
                format: "json".to_string(),
                message: format!("Failed to parse Vault response: {}", e),
                location: None,
                source: None,
            })?;

        let key_value = json
            .get("data")
            .and_then(|d| d.get(&self.secret_key))
            .and_then(|v| v.as_str())
            .ok_or(ConfigError::KeyError {
                message: format!("Key '{}' not found in Vault secret", self.secret_key),
            })?;

        if key_value.len() < 32 {
            return Err(ConfigError::KeyError {
                message: "Vault key must be at least 32 characters".to_string(),
            });
        }

        Ok(ZeroizingBytes::new(key_value.as_bytes()[..32].to_vec()))
    }

    fn provider_type(&self) -> &'static str {
        "vault"
    }

    fn cache_policy(&self) -> KeyCachePolicy {
        self.cache_policy
    }
}

#[cfg(feature = "remote")]
pub struct VaultKeyProviderBuilder {
    vault_addr: Option<String>,
    secret_path: Option<String>,
    secret_key: Option<String>,
    auth: Option<VaultAuth>,
    allow_http: bool,
    cache_policy: KeyCachePolicy,
}

#[cfg(feature = "remote")]
impl VaultKeyProviderBuilder {
    pub fn new() -> Self {
        Self {
            vault_addr: None,
            secret_path: None,
            secret_key: None,
            auth: None,
            allow_http: false,
            cache_policy: KeyCachePolicy::default(),
        }
    }

    pub fn vault_addr(mut self, addr: impl Into<String>) -> Self {
        self.vault_addr = Some(addr.into());
        self
    }

    pub fn secret_path(mut self, path: impl Into<String>) -> Self {
        self.secret_path = Some(path.into());
        self
    }

    pub fn secret_key(mut self, key: impl Into<String>) -> Self {
        self.secret_key = Some(key.into());
        self
    }

    pub fn token(mut self, token: impl Into<String>) -> Self {
        self.auth = Some(VaultAuth::Token {
            token: token.into(),
        });
        self
    }

    /// Authenticate with AppRole / Kubernetes service-account (see `VaultAuth`).
    pub fn auth(mut self, auth: VaultAuth) -> Self {
        self.auth = Some(auth);
        self
    }

    /// Allow plain HTTP (loopback mock tests only).
    pub fn allow_http(mut self, allow: bool) -> Self {
        self.allow_http = allow;
        self
    }

    pub fn cache_policy(mut self, policy: KeyCachePolicy) -> Self {
        self.cache_policy = policy;
        self
    }

    pub fn build(self) -> ConfigResult<VaultKeyProvider> {
        let vault_addr = self.vault_addr.ok_or(ConfigError::InvalidValue {
            key: "vault_addr".to_string(),
            expected_type: "string".to_string(),
            message: "Vault address is required".to_string(),
        })?;

        let secret_path = self.secret_path.ok_or(ConfigError::InvalidValue {
            key: "secret_path".to_string(),
            expected_type: "string".to_string(),
            message: "Secret path is required".to_string(),
        })?;

        let secret_key = self.secret_key.ok_or(ConfigError::InvalidValue {
            key: "secret_key".to_string(),
            expected_type: "string".to_string(),
            message: "Secret key is required".to_string(),
        })?;

        Ok(VaultKeyProvider {
            vault_addr,
            secret_path,
            secret_key,
            auth: self.auth.unwrap_or(VaultAuth::Token {
                token: String::new(),
            }),
            token_cache: std::sync::Mutex::new(None),
            allow_http: self.allow_http,
            cache_policy: self.cache_policy,
        })
    }
}

#[cfg(feature = "remote")]
impl Default for VaultKeyProviderBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::sync::Arc;
    use tempfile::NamedTempFile;

    #[test]
    fn test_file_key_provider() {
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file
            .write_all(b"this-is-a-test-key-with-32-chars-minimum")
            .unwrap();

        let provider = FileKeyProvider::new(temp_file.path());
        let key = provider.get_key().unwrap();

        assert_eq!(key.len(), 32);
        assert_eq!(provider.provider_type(), "file");
    }

    #[test]
    fn test_file_key_provider_builder() {
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file
            .write_all(b"this-is-a-test-key-with-32-chars-minimum")
            .unwrap();

        let provider = FileKeyProvider::builder()
            .path(temp_file.path())
            .cache_policy(KeyCachePolicy::NoCache)
            .build()
            .unwrap();

        let key = provider.get_key().unwrap();
        assert_eq!(key.len(), 32);
    }

    #[test]
    fn test_file_key_provider_short_key() {
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"short").unwrap();

        let provider = FileKeyProvider::new(temp_file.path());
        let result = provider.get_key();

        assert!(result.is_err());
    }

    #[test]
    fn test_file_key_provider_whitespace_trim() {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(b"  this-is-a-test-key-with-32-chars-min  \n")
            .unwrap();
        let p = FileKeyProvider::new(f.path());
        let key = p.get_key().unwrap();
        assert_eq!(key.len(), 32);
    }

    #[test]
    fn test_file_key_provider_builder_no_path() {
        let result = FileKeyProvider::builder().build();
        assert!(result.is_err());
    }

    #[test]
    fn test_file_key_provider_builder_cache_policy() {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(b"test-key-12345678901234567890").unwrap();
        let p = FileKeyProvider::builder()
            .path(f.path())
            .cache_policy(KeyCachePolicy::NoCache)
            .build()
            .unwrap();
        assert_eq!(p.provider_type(), "file");
    }

    #[test]
    fn test_file_key_provider_file_not_found() {
        let provider = FileKeyProvider::new("/nonexistent/path/does-not-exist-key.txt");
        let result = provider.get_key();
        assert!(result.is_err());
    }

    #[test]
    fn test_file_key_provider_non_utf8_content() {
        let mut f = NamedTempFile::new().unwrap();
        // Invalid UTF-8 bytes — from_utf8 conversion must fail before length check.
        f.write_all(&[0xFF, 0xFE, 0xFD, 0xFC, 0xFB, 0xFA]).unwrap();
        let provider = FileKeyProvider::new(f.path());
        let result = provider.get_key();
        assert!(result.is_err());
    }

    #[test]
    fn test_file_key_provider_empty_file() {
        let f = NamedTempFile::new().unwrap();
        let provider = FileKeyProvider::new(f.path());
        // Empty content → trimmed length 0 < 32 → KeyError
        let result = provider.get_key();
        assert!(result.is_err());
    }

    #[test]
    fn test_file_key_provider_with_cache_policy() {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(b"this-is-a-test-key-with-32-chars-minimum")
            .unwrap();
        let provider = FileKeyProvider::new(f.path()).with_cache_policy(KeyCachePolicy::NoCache);
        assert_eq!(provider.cache_policy(), KeyCachePolicy::NoCache);
    }

    #[test]
    fn test_file_key_provider_default_cache_policy() {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(b"this-is-a-test-key-with-32-chars-minimum")
            .unwrap();
        let provider = FileKeyProvider::new(f.path());
        assert_eq!(
            provider.cache_policy(),
            KeyCachePolicy::CacheWithTtl(std::time::Duration::from_secs(3600))
        );
    }

    #[test]
    fn test_file_key_provider_cache_indefinitely_policy() {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(b"this-is-a-test-key-with-32-chars-minimum")
            .unwrap();
        let provider =
            FileKeyProvider::new(f.path()).with_cache_policy(KeyCachePolicy::CacheIndefinitely);
        assert_eq!(provider.cache_policy(), KeyCachePolicy::CacheIndefinitely);
    }

    #[test]
    fn test_file_key_provider_extracts_exactly_32_bytes() {
        let mut f = NamedTempFile::new().unwrap();
        let key = b"0123456789abcdef0123456789abcdefEXTRA_TRAILING_DATA"; // pragma: allowlist secret
        f.write_all(key).unwrap();
        let provider = FileKeyProvider::new(f.path());
        let result = provider.get_key().unwrap();
        assert_eq!(result.len(), 32);
        assert_eq!(&*result, &key[..32]);
    }

    #[test]
    fn test_file_key_provider_exactly_32_chars_boundary() {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(b"0123456789abcdef0123456789abcdef").unwrap(); // pragma: allowlist secret
        let provider = FileKeyProvider::new(f.path());
        let result = provider.get_key();
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 32);
    }

    #[test]
    fn test_file_key_provider_builder_default_impl() {
        let builder = FileKeyProviderBuilder::default();
        // Default builder has no path set — build must fail.
        assert!(builder.build().is_err());
    }

    #[test]
    fn test_file_key_provider_as_trait_object() {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(b"this-is-a-test-key-with-32-chars-minimum")
            .unwrap();
        let provider = FileKeyProvider::new(f.path());
        let provider_ref: &dyn KeyProvider = &provider;
        assert_eq!(provider_ref.provider_type(), "file");
        let key = provider_ref.get_key().unwrap();
        assert_eq!(key.len(), 32);
    }

    #[test]
    fn test_file_key_provider_builder_with_cache_indefinitely() {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(b"this-is-a-test-key-with-32-chars-minimum")
            .unwrap();
        let provider = FileKeyProvider::builder()
            .path(f.path())
            .cache_policy(KeyCachePolicy::CacheIndefinitely)
            .build()
            .unwrap();
        assert_eq!(provider.cache_policy(), KeyCachePolicy::CacheIndefinitely);
    }

    // ===== VaultKeyProvider tests (only when `remote` feature is enabled) =====

    #[cfg(feature = "remote")]
    #[test]
    fn test_vault_key_provider_new_https() {
        let provider =
            VaultKeyProvider::new("https://vault.example.com", "secret/data/path", "my_key")
                .unwrap();
        assert_eq!(provider.provider_type(), "vault");
    }

    #[cfg(feature = "remote")]
    #[test]
    fn test_vault_key_provider_new_non_https() {
        let result = VaultKeyProvider::new("http://vault.example.com", "secret/path", "key");
        assert!(result.is_err());
    }

    #[cfg(feature = "remote")]
    #[test]
    fn test_vault_key_provider_with_token() {
        let provider = VaultKeyProvider::new("https://vault.example.com", "secret/path", "key")
            .unwrap()
            .with_token("my-token");
        assert_eq!(provider.provider_type(), "vault");
    }

    #[cfg(feature = "remote")]
    #[test]
    fn test_vault_key_provider_with_cache_policy() {
        let provider = VaultKeyProvider::new("https://vault.example.com", "secret/path", "key")
            .unwrap()
            .with_cache_policy(KeyCachePolicy::NoCache);
        assert_eq!(provider.cache_policy(), KeyCachePolicy::NoCache);
    }

    #[cfg(feature = "remote")]
    #[test]
    fn test_vault_key_provider_cache_policy_default() {
        let provider =
            VaultKeyProvider::new("https://vault.example.com", "secret/path", "key").unwrap();
        assert_eq!(
            provider.cache_policy(),
            KeyCachePolicy::CacheWithTtl(std::time::Duration::from_secs(3600))
        );
    }

    #[cfg(feature = "remote")]
    #[test]
    fn test_vault_key_provider_builder_default_impl() {
        let builder = VaultKeyProviderBuilder::default();
        assert!(builder.build().is_err());
    }

    #[cfg(feature = "remote")]
    #[test]
    fn test_vault_key_provider_builder_success() {
        let provider = VaultKeyProvider::builder()
            .vault_addr("https://vault.example.com")
            .secret_path("secret/data/path")
            .secret_key("my_key")
            .token("my-token")
            .cache_policy(KeyCachePolicy::NoCache)
            .build()
            .unwrap();
        assert_eq!(provider.provider_type(), "vault");
        assert_eq!(provider.cache_policy(), KeyCachePolicy::NoCache);
    }

    #[cfg(feature = "remote")]
    #[test]
    fn test_vault_key_provider_builder_no_vault_addr() {
        let result = VaultKeyProvider::builder()
            .secret_path("secret/path")
            .secret_key("key")
            .build();
        assert!(result.is_err());
    }

    #[cfg(feature = "remote")]
    #[tokio::test]
    async fn test_vault_key_provider_builder_non_https() {
        // Plain HTTP builds (mock-test support) but fails closed on fetch
        // unless allow_http is explicitly enabled.
        let provider = VaultKeyProvider::builder()
            .vault_addr("http://vault.example.com")
            .secret_path("secret/path")
            .secret_key("key")
            .build()
            .expect("http builds with runtime enforcement");
        let err = provider.get_key().await.expect_err("http must be rejected");
        assert!(matches!(err, ConfigError::KeyError { .. }));
    }

    #[cfg(feature = "remote")]
    #[test]
    fn test_vault_key_provider_builder_no_secret_path() {
        let result = VaultKeyProvider::builder()
            .vault_addr("https://vault.example.com")
            .secret_key("key")
            .build();
        assert!(result.is_err());
    }

    #[cfg(feature = "remote")]
    #[test]
    fn test_vault_key_provider_builder_no_secret_key() {
        let result = VaultKeyProvider::builder()
            .vault_addr("https://vault.example.com")
            .secret_path("secret/path")
            .build();
        assert!(result.is_err());
    }

    #[cfg(feature = "remote")]
    #[test]
    fn test_is_retryable_status_includes_429_and_gateway_errors() {
        // 429 must be treated as retryable, consistent with the remote
        // polling module's retry policy.
        assert!(is_retryable_status(429));
        assert!(is_retryable_status(502));
        assert!(is_retryable_status(503));
        assert!(is_retryable_status(504));
        // Everything else is terminal and must not be retried.
        assert!(!is_retryable_status(200));
        assert!(!is_retryable_status(400));
        assert!(!is_retryable_status(401));
        assert!(!is_retryable_status(403));
        assert!(!is_retryable_status(404));
        assert!(!is_retryable_status(500));
    }

    /// Mock Vault: serves queued (status, body) responses in order, records
    /// the requests it received.
    async fn spawn_mock_vault(responses: Vec<(u16, String)>) -> std::net::SocketAddr {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let addr = listener.local_addr().unwrap();
        let queue = Arc::new(std::sync::Mutex::new(responses));
        tokio::spawn(async move {
            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            loop {
                let Ok((mut stream, _)) = listener.accept().await else {
                    break;
                };
                let queue = Arc::clone(&queue);
                tokio::spawn(async move {
                    let mut buf = [0u8; 8192];
                    let _ = stream.read(&mut buf).await;
                    let (status, body) = {
                        let mut q = queue.lock().unwrap();
                        if q.len() > 1 {
                            q.remove(0)
                        } else {
                            q.first().cloned().unwrap_or((404, "{}".to_string()))
                        }
                    };
                    let status_line = if status == 200 {
                        "200 OK"
                    } else {
                        "403 Forbidden"
                    };
                    let response = format!(
                        "HTTP/1.1 {status_line}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        body.len(),
                        body
                    );
                    let _ = stream.write_all(response.as_bytes()).await;
                });
            }
        });
        addr
    }

    #[cfg(feature = "remote")]
    #[tokio::test]
    async fn approle_login_exchanges_credentials_for_token() {
        let addr = spawn_mock_vault(vec![
            (
                200,
                serde_json::json!({"auth": {"client_token": "approle-token"}}).to_string(),
            ),
            (
                200,
                serde_json::json!({"data": {"key": "0123456789012345678901234567890123456789"}})
                    .to_string(),
            ),
        ])
        .await;

        let provider = VaultKeyProvider::builder()
            .vault_addr(format!("http://{addr}"))
            .secret_path("secret/data/confers")
            .secret_key("key")
            .auth(VaultAuth::AppRole {
                role_id: "role-1".to_string(),
                secret_id: "secret-1".to_string(),
            })
            .allow_http(true)
            .build()
            .expect("build");

        assert_eq!(provider.auth_method(), "approle");
        let key = provider.get_key().await.expect("key via approle");
        assert!(!key.as_slice().is_empty());
    }

    #[cfg(feature = "remote")]
    #[tokio::test]
    async fn kubernetes_login_exchanges_jwt_for_token() {
        let addr = spawn_mock_vault(vec![
            (
                200,
                serde_json::json!({"auth": {"client_token": "k8s-token"}}).to_string(),
            ),
            (
                200,
                serde_json::json!({"data": {"key": "0123456789012345678901234567890123456789"}})
                    .to_string(),
            ),
        ])
        .await;

        let provider = VaultKeyProvider::builder()
            .vault_addr(format!("http://{addr}"))
            .secret_path("secret/data/confers")
            .secret_key("key")
            .auth(VaultAuth::Kubernetes {
                jwt: "fake-pod-jwt".to_string(),
                role: "confers-role".to_string(),
            })
            .allow_http(true)
            .build()
            .expect("build");

        assert_eq!(provider.auth_method(), "kubernetes");
        let key = provider.get_key().await.expect("key via kubernetes auth");
        assert!(!key.as_slice().is_empty());
    }

    #[cfg(feature = "remote")]
    #[tokio::test]
    async fn failed_login_maps_to_retryable_unavailable() {
        let addr = spawn_mock_vault(vec![(403, "{\"errors\":[\"bad creds\"]}".to_string())]).await;

        let provider = VaultKeyProvider::builder()
            .vault_addr(format!("http://{addr}"))
            .secret_path("secret/data/confers")
            .secret_key("key")
            .auth(VaultAuth::AppRole {
                role_id: "bad".to_string(),
                secret_id: "worse".to_string(),
            })
            .allow_http(true)
            .build()
            .expect("build");

        let err = provider.get_key().await.expect_err("login must fail");
        assert!(
            matches!(
                err,
                ConfigError::RemoteUnavailable {
                    retryable: false,
                    ..
                }
            ),
            "403 is permanent, not retryable"
        );
    }

    #[cfg(feature = "remote")]
    #[test]
    fn kubernetes_auth_can_be_built_from_inline_jwt() {
        let auth = VaultAuth::Kubernetes {
            jwt: "jwt".to_string(),
            role: "role".to_string(),
        };
        assert_eq!(auth.describe(), "kubernetes");
        assert_eq!(auth.login_path(), Some("/v1/auth/kubernetes/login"));
        let approle = VaultAuth::AppRole {
            role_id: "r".to_string(),
            secret_id: "s".to_string(),
        };
        assert_eq!(approle.login_path(), Some("/v1/auth/approle/login"));
    }
}
