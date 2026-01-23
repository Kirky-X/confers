// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

use crate::security::secure_string::SecureString;
use crate::utils::file_format::parse_content;
use crate::utils::ssrf::validate_remote_url;
use crate::utils::tls_config::TlsConfig;
use figment::Figment;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::OnceLock;
use std::time::Duration;

// Global HTTP client - lazily initialized to avoid startup panics
// Uses OnceLock for safe initialization

static HTTP_CLIENT: OnceLock<Arc<reqwest::blocking::Client>> = OnceLock::new();

static HTTP_CLIENT_ASYNC: OnceLock<Arc<reqwest::Client>> = OnceLock::new();

/// Get the global blocking HTTP client
pub fn get_http_client() -> &'static Arc<reqwest::blocking::Client> {
    HTTP_CLIENT.get_or_init(|| {
        Arc::new(
            reqwest::blocking::Client::builder()
                .pool_max_idle_per_host(10)
                .pool_idle_timeout(Duration::from_secs(90))
                .timeout(Duration::from_secs(30))
                .build()
                .map_err(|e| {
                    crate::error::ConfigError::RemoteError(format!("Failed to create HTTP client: {}", e))
                })
                .unwrap(),
        )
    })
}

/// Get the global async HTTP client
pub fn get_async_http_client() -> &'static Arc<reqwest::Client> {
    HTTP_CLIENT_ASYNC.get_or_init(|| {
        Arc::new(
            reqwest::Client::builder()
                .pool_max_idle_per_host(10)
                .pool_idle_timeout(Duration::from_secs(90))
                .timeout(Duration::from_secs(30))
                .build()
                .map_err(|e| {
                    crate::error::ConfigError::RemoteError(format!("Failed to create async HTTP client: {}", e))
                })
                .unwrap(),
        )
    })
}

pub struct HttpProvider {
    url: String,
    auth: Option<HttpAuth>,
    tls_config: Option<TlsConfig>,
    timeout: Duration,
}

#[derive(Clone)]
pub(crate) struct HttpAuth {
    pub(crate) username: String,
    pub(crate) password: Option<Arc<SecureString>>,
    pub(crate) bearer_token: Option<Arc<SecureString>>,
}

impl HttpProvider {
    pub fn new(url: impl Into<String>) -> Result<Self, ConfigError> {
        let url_str = url.into();
        // Validate URL to prevent SSRF attacks
        validate_remote_url(&url_str)?;

        Ok(Self {
            url: url_str,
            auth: None,
            tls_config: None,
            timeout: Duration::from_secs(30), // Default 30 seconds
        })
    }

    pub fn from_url(url: impl Into<String>) -> Result<Self, ConfigError> {
        Self::new(url)
    }

    /// Set HTTP request timeout (in seconds)
    pub fn with_timeout_seconds(mut self, timeout_secs: u64) -> Self {
        self.timeout = Duration::from_secs(timeout_secs);
        self
    }

    /// Set HTTP request timeout with Duration
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn with_tls(
        mut self,
        ca_cert: impl Into<std::path::PathBuf>,
        client_cert: Option<impl Into<std::path::PathBuf>>,
        client_key: Option<impl Into<std::path::PathBuf>>,
    ) -> Self {
        self.tls_config = Some(TlsConfig {
            ca_cert: Some(ca_cert.into()),
            client_cert: client_cert.map(|p| p.into()),
            client_key: client_key.map(|p| p.into()),
        });
        self
    }

    pub fn with_auth(mut self, username: impl Into<String>, password: impl Into<String>) -> Self {
        self.auth = Some(HttpAuth {
            username: username.into(),
            password: Some(Arc::new(SecureString::from(password.into()))),
            bearer_token: None,
        });
        self
    }

    pub fn with_auth_secure(mut self, username: String, password: Arc<SecureString>) -> Self {
        self.auth = Some(HttpAuth {
            username,
            password: Some(password),
            bearer_token: None,
        });
        self
    }

    pub fn with_bearer_token(mut self, token: impl Into<String>) -> Self {
        self.auth = Some(HttpAuth {
            username: String::new(),
            password: None,
            bearer_token: Some(Arc::new(SecureString::from(token.into()))),
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

    pub fn load_sync(&self) -> Result<Figment, ConfigError> {
        // Validate URL again before use to prevent SSRF if struct was mutated
        validate_remote_url(&self.url)?;

        let client = reqwest::blocking::Client::builder()
            .pool_max_idle_per_host(10)
            .pool_idle_timeout(Duration::from_secs(90))
            .timeout(self.timeout)
            .build()
            .map_err(|e| {
                ConfigError::RemoteError(format!("Failed to create HTTP client: {}", e))
            })?;

        let mut request = client.get(&self.url);

        // Apply authentication if configured
        if let Some(auth) = &self.auth {
            if let Some(token) = &auth.bearer_token {
                request = request.bearer_auth(token.as_str());
            } else {
                request = request.basic_auth(
                    &auth.username,
                    auth.password.as_ref().map(|p| p.as_str()),
                );
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
            .unwrap_or("");

        let body = response
            .text()
            .map_err(|e| ConfigError::RemoteError(format!("Failed to read response body: {}", e)))?;

        let figment = parse_content(&body, Some(content_type))
            .map_err(|e| ConfigError::RemoteError(e))?
            .figment;

        Ok(figment)
    }

    pub async fn load(&self) -> Result<Figment, ConfigError> {
        // Validate URL again before use to prevent SSRF if struct was mutated
        validate_remote_url(&self.url)?;

        let client = reqwest::Client::builder()
            .pool_max_idle_per_host(10)
            .pool_idle_timeout(Duration::from_secs(90))
            .timeout(self.timeout)
            .build()
            .map_err(|e| {
                ConfigError::RemoteError(format!("Failed to create async HTTP client: {}", e))
            })?;

        let mut request = client.get(&self.url);

        if let Some(auth) = &self.auth {
            if let Some(token) = &auth.bearer_token {
                request = request.bearer_auth(token.as_str());
            } else {
                request = request.basic_auth(
                    &auth.username,
                    auth.password.as_ref().map(|p| p.as_str()),
                );
            }
        }

        let response = request
            .send()
            .await
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
            .unwrap_or("");

        let body = response
            .text()
            .await
            .map_err(|e| ConfigError::RemoteError(format!("Failed to read response body: {}", e)))?;

        let figment = parse_content(&body, Some(content_type))
            .map_err(|e| ConfigError::RemoteError(e))?
            .figment;

        Ok(figment)
    }
}

impl Provider for HttpProvider {
    fn metadata(&self) -> figment::Metadata {
        figment::Metadata::named(format!("HTTP ({})", self.url))
    }

    fn data(&self) -> Result<Map<Profile, Dict>, Error> {
        // For Provider trait implementation, we need to avoid blocking operations
        // We'll use reqwest::blocking but handle it carefully to avoid runtime conflicts
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| Error::from(format!("Failed to create HTTP client: {}", e)))?;

        let mut request = client.get(&self.url);

        // Apply authentication if configured
        if let Some(auth) = &self.auth {
            if let Some(token) = &auth.bearer_token {
                request = request.bearer_auth(token);
            } else {
                request = request.basic_auth(&auth.username, auth.password.as_deref());
            }
        }

        let response = request
            .send()
            .map_err(|e| Error::from(format!("HTTP request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(Error::from(format!(
                "HTTP request failed with status: {}",
                response.status()
            )));
        }

        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("");

        let dict: Dict = if content_type.contains("application/json") {
            let json_str = response
                .text()
                .map_err(|e| Error::from(format!("Failed to read JSON response: {}", e)))?;
            serde_json::from_str(&json_str)
                .map_err(|e| Error::from(format!("Failed to parse JSON: {}", e)))?
        } else if content_type.contains("application/toml") || content_type.contains("text/toml") {
            let toml_str = response
                .text()
                .map_err(|e| Error::from(format!("Failed to read TOML response: {}", e)))?;
            toml::from_str(&toml_str)
                .map_err(|e| Error::from(format!("Failed to parse TOML: {}", e)))?
        } else if content_type.contains("application/yaml") || content_type.contains("text/yaml") {
            let yaml_str = response
                .text()
                .map_err(|e| Error::from(format!("Failed to read YAML response: {}", e)))?;
            serde_yaml::from_str(&yaml_str)
                .map_err(|e| Error::from(format!("Failed to parse YAML: {}", e)))?
        } else {
            // Default to JSON parsing
            let json_str = response
                .text()
                .map_err(|e| Error::from(format!("Failed to read JSON response: {}", e)))?;
            serde_json::from_str(&json_str)
                .map_err(|e| Error::from(format!("Failed to parse JSON: {}", e)))?
        };

        let mut profiles = Map::new();
        profiles.insert(Profile::Default, dict);
        Ok(profiles)
    }
}
