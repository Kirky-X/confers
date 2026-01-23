// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

use crate::error::ConfigError;
use crate::providers::provider::{ConfigProvider, ProviderMetadata, ProviderType};
#[cfg(feature = "encryption")]
use crate::security::{SecureString, SensitivityLevel};
use crate::utils::file_format::parse_content;
use crate::utils::ssrf::validate_remote_url;
use crate::utils::tls_config::TlsConfig;
use figment::Figment;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone)]
pub struct HttpConfigProvider {
    url: String,
    auth: Option<HttpAuth>,
    tls_config: Option<TlsConfig>,
    timeout: Option<String>,
    priority: u8,
    /// Circuit breaker configuration
    failure_threshold: u32,
    reset_timeout: Duration,
}

#[cfg(feature = "encryption")]
#[derive(Clone)]
pub(crate) struct HttpAuth {
    pub(crate) username: String,
    pub(crate) password: Option<Arc<SecureString>>,
    pub(crate) bearer_token: Option<Arc<SecureString>>,
}

#[cfg(feature = "encryption")]
impl HttpConfigProvider {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            auth: None,
            tls_config: None,
            timeout: None,
            priority: 30,
            failure_threshold: 3,
            reset_timeout: Duration::from_secs(30),
        }
    }

    pub fn from_url(url: impl Into<String>) -> Self {
        Self::new(url)
    }

    pub fn with_auth(mut self, username: impl Into<String>, password: impl Into<String>) -> Self {
        self.auth = Some(HttpAuth {
            username: username.into(),
            password: Some(Arc::new(SecureString::new(
                password.into(),
                SensitivityLevel::Critical,
            ))),
            bearer_token: None,
        });
        self
    }

    pub fn with_auth_secure(
        mut self,
        username: impl Into<String>,
        password: Arc<SecureString>,
    ) -> Self {
        self.auth = Some(HttpAuth {
            username: username.into(),
            password: Some(password),
            bearer_token: None,
        });
        self
    }

    pub fn with_bearer_token(mut self, token: impl Into<String>) -> Self {
        self.auth = Some(HttpAuth {
            username: String::new(),
            password: None,
            bearer_token: Some(Arc::new(SecureString::new(
                token.into(),
                SensitivityLevel::High,
            ))),
        });
        self
    }

    pub fn with_bearer_token_secure(mut self, token: Arc<SecureString>) -> Self {
        self.auth = Some(HttpAuth {
            username: String::new(),
            password: None,
            bearer_token: Some(token),
        });
        self
    }

    pub fn with_tls(
        mut self,
        ca_cert: impl Into<PathBuf>,
        client_cert: Option<impl Into<PathBuf>>,
        client_key: Option<impl Into<PathBuf>>,
    ) -> Self {
        self.tls_config = Some(TlsConfig {
            ca_cert: Some(ca_cert.into()),
            client_cert: client_cert.map(|p| p.into()),
            client_key: client_key.map(|p| p.into()),
        });
        self
    }

    pub fn with_timeout(mut self, timeout: impl Into<String>) -> Self {
        self.timeout = Some(timeout.into());
        self
    }

    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }

    /// Configure circuit breaker failure threshold
    pub fn with_failure_threshold(mut self, threshold: u32) -> Self {
        self.failure_threshold = threshold;
        self
    }

    /// Configure circuit breaker reset timeout
    pub fn with_reset_timeout(mut self, timeout: Duration) -> Self {
        self.reset_timeout = timeout;
        self
    }

    fn build_client(&self) -> Result<reqwest::blocking::Client, ConfigError> {
        let mut builder =
            reqwest::blocking::Client::builder().timeout(std::time::Duration::from_secs(30));

        if let Some(tls) = &self.tls_config {
            if let Some(ca_path) = &tls.ca_cert {
                let cert_data = std::fs::read(ca_path).map_err(|e| {
                    ConfigError::RemoteError(format!("Failed to read CA cert: {}", e))
                })?;
                let cert = reqwest::Certificate::from_pem(&cert_data).map_err(|e| {
                    ConfigError::RemoteError(format!("Failed to parse CA cert: {}", e))
                })?;
                builder = builder.add_root_certificate(cert);
            }

            if let (Some(cert_path), Some(key_path)) = (&tls.client_cert, &tls.client_key) {
                let cert_data = std::fs::read(cert_path).map_err(|e| {
                    ConfigError::RemoteError(format!("Failed to read client cert: {}", e))
                })?;
                let key_data = std::fs::read(key_path).map_err(|e| {
                    ConfigError::RemoteError(format!("Failed to read client key: {}", e))
                })?;
                // Combine cert and key for identity
                let mut combined = cert_data;
                combined.extend_from_slice(b"\n");
                combined.extend_from_slice(&key_data);
                let identity = reqwest::Identity::from_pem(&combined).map_err(|e| {
                    ConfigError::RemoteError(format!("Failed to parse client identity: {}", e))
                })?;
                builder = builder.identity(identity);
            }
        }

        builder
            .build()
            .map_err(|e| ConfigError::RemoteError(format!("Failed to create HTTP client: {}", e)))
    }

    fn parse_response(&self, content_type: &str, response: &str) -> Result<Figment, ConfigError> {
        parse_content(response, Some(content_type))
            .map(|parsed| parsed.figment)
            .map_err(ConfigError::RemoteError)
    }
}

#[cfg(not(feature = "encryption"))]
#[derive(Clone)]
pub(crate) struct HttpAuth {
    pub(crate) username: String,
    pub(crate) password: Option<String>,
    pub(crate) bearer_token: Option<String>,
}

#[cfg(not(feature = "encryption"))]
impl HttpConfigProvider {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            auth: None,
            tls_config: None,
            timeout: None,
            priority: 30,
            failure_threshold: 3,
            reset_timeout: Duration::from_secs(30),
        }
    }

    pub fn from_url(url: impl Into<String>) -> Self {
        Self::new(url)
    }

    pub fn with_auth(mut self, username: impl Into<String>, password: impl Into<String>) -> Self {
        self.auth = Some(HttpAuth {
            username: username.into(),
            password: Some(password.into()),
            bearer_token: None,
        });
        self
    }

    pub fn with_bearer_token(mut self, token: impl Into<String>) -> Self {
        self.auth = Some(HttpAuth {
            username: String::new(),
            password: None,
            bearer_token: Some(token.into()),
        });
        self
    }

    pub fn with_tls(
        mut self,
        ca_cert: impl Into<PathBuf>,
        client_cert: Option<impl Into<PathBuf>>,
        client_key: Option<impl Into<PathBuf>>,
    ) -> Self {
        self.tls_config = Some(TlsConfig {
            ca_cert: Some(ca_cert.into()),
            client_cert: client_cert.map(|p| p.into()),
            client_key: client_key.map(|p| p.into()),
        });
        self
    }

    pub fn with_timeout(mut self, timeout: impl Into<String>) -> Self {
        self.timeout = Some(timeout.into());
        self
    }

    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }

    /// Configure circuit breaker failure threshold
    pub fn with_failure_threshold(mut self, threshold: u32) -> Self {
        self.failure_threshold = threshold;
        self
    }

    /// Configure circuit breaker reset timeout
    pub fn with_reset_timeout(mut self, timeout: Duration) -> Self {
        self.reset_timeout = timeout;
        self
    }

    fn build_client(&self) -> Result<reqwest::blocking::Client, ConfigError> {
        let mut builder =
            reqwest::blocking::Client::builder().timeout(std::time::Duration::from_secs(30));

        if let Some(tls) = &self.tls_config {
            if let Some(ca_path) = &tls.ca_cert {
                let cert_data = std::fs::read(ca_path).map_err(|e| {
                    ConfigError::RemoteError(format!("Failed to read CA cert: {}", e))
                })?;
                let cert = reqwest::Certificate::from_pem(&cert_data).map_err(|e| {
                    ConfigError::RemoteError(format!("Failed to parse CA cert: {}", e))
                })?;
                builder = builder.add_root_certificate(cert);
            }

            if let (Some(cert_path), Some(key_path)) = (&tls.client_cert, &tls.client_key) {
                let cert_data = std::fs::read(cert_path).map_err(|e| {
                    ConfigError::RemoteError(format!("Failed to read client cert: {}", e))
                })?;
                let key_data = std::fs::read(key_path).map_err(|e| {
                    ConfigError::RemoteError(format!("Failed to read client key: {}", e))
                })?;
                let mut combined = cert_data;
                combined.extend_from_slice(b"\n");
                combined.extend_from_slice(&key_data);
                let identity = reqwest::Identity::from_pem(&combined).map_err(|e| {
                    ConfigError::RemoteError(format!("Failed to parse client identity: {}", e))
                })?;
                builder = builder.identity(identity);
            }
        }

        builder
            .build()
            .map_err(|e| ConfigError::RemoteError(format!("Failed to create HTTP client: {}", e)))
    }

    fn parse_response(&self, content_type: &str, response: &str) -> Result<Figment, ConfigError> {
        parse_content(response, Some(content_type))
            .map(|parsed| parsed.figment)
            .map_err(ConfigError::RemoteError)
    }
}

impl ConfigProvider for HttpConfigProvider {
    fn load(&self) -> Result<Figment, ConfigError> {
        // Validate URL to prevent SSRF attacks
        validate_remote_url(&self.url)?;

        let client = self.build_client()?;

        let mut request = client.get(&self.url);

        if let Some(auth) = &self.auth {
            if let Some(token) = &auth.bearer_token {
                // Both SecureString and String have as_str() method
                request = request.bearer_auth(token.as_str());
            } else {
                request =
                    request.basic_auth(&auth.username, auth.password.as_ref().map(|p| p.as_str()));
            }
        }

        let response = request
            .send()
            .map_err(|e| ConfigError::RemoteError(format!("HTTP request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(ConfigError::RemoteError(format!(
                "HTTP request failed with status: {}",
                response.status()
            )));
        }

        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("")
            .to_string();

        let body = response
            .text()
            .map_err(|e| ConfigError::RemoteError(format!("Failed to read response: {}", e)))?;

        self.parse_response(&content_type, &body)
    }

    fn name(&self) -> &str {
        "http"
    }

    fn is_available(&self) -> bool {
        !self.url.is_empty()
            && (self.url.starts_with("http://") || self.url.starts_with("https://"))
    }

    fn priority(&self) -> u8 {
        self.priority
    }

    fn metadata(&self) -> ProviderMetadata {
        let auth_type = if self
            .auth
            .as_ref()
            .and_then(|a| a.bearer_token.as_ref())
            .is_some()
        {
            "bearer_token"
        } else if self
            .auth
            .as_ref()
            .map(|a| !a.username.is_empty())
            .unwrap_or(false)
        {
            "basic_auth"
        } else {
            "none"
        };

        ProviderMetadata {
            name: self.name().to_string(),
            description: format!("HTTP provider for URL: {} (auth: {})", self.url, auth_type),
            source_type: ProviderType::Remote,
            requires_network: true,
            supports_watch: false,
            priority: self.priority,
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[deprecated(since = "0.4.0", note = "Use HttpConfigProvider instead")]
pub type HttpProvider = HttpConfigProvider;