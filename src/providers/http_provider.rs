// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

use crate::error::ConfigError;
use crate::providers::provider::{ConfigProvider, ProviderMetadata, ProviderType};
use figment::value::{Dict, Map, Value as FigmentValue};
use figment::{providers::Serialized, Error, Figment, Profile, Provider};
use serde_json::Value as JsonValue;
use std::path::PathBuf;

#[derive(Clone)]
pub struct HttpConfigProvider {
    url: String,
    auth: Option<HttpAuth>,
    tls_config: Option<TlsConfig>,
    timeout: Option<String>,
    priority: u8,
}

#[derive(Clone)]
pub struct TlsConfig {
    pub ca_cert: Option<PathBuf>,
    pub client_cert: Option<PathBuf>,
    pub client_key: Option<PathBuf>,
}

#[derive(Clone)]
pub struct HttpAuth {
    pub username: String,
    pub password: Option<String>,
    pub bearer_token: Option<String>,
}

impl HttpConfigProvider {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            auth: None,
            tls_config: None,
            timeout: None,
            priority: 30,
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
                let _key_data = std::fs::read(key_path).map_err(|e| {
                    ConfigError::RemoteError(format!("Failed to read client key: {}", e))
                })?;
                let identity = reqwest::Identity::from_pem(&cert_data).map_err(|e| {
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
        let figment: Figment = if content_type.contains("application/json") {
            let json_value: JsonValue = serde_json::from_str(response)
                .map_err(|e| ConfigError::RemoteError(format!("Failed to parse JSON: {}", e)))?;
            let dict: Dict = serde_json::from_value(json_value).map_err(|e| {
                ConfigError::RemoteError(format!("Failed to convert JSON to dict: {}", e))
            })?;
            Figment::new().merge(Serialized::from(dict, Profile::Default))
        } else if content_type.contains("application/toml") || content_type.contains("text/toml") {
            let toml_value: FigmentValue = toml::from_str(response)
                .map_err(|e| ConfigError::RemoteError(format!("Failed to parse TOML: {}", e)))?;
            let dict: Dict = toml_value.deserialize().map_err(|e| {
                ConfigError::RemoteError(format!("Failed to convert TOML to dict: {}", e))
            })?;
            Figment::new().merge(Serialized::from(dict, Profile::Default))
        } else if content_type.contains("application/yaml") || content_type.contains("text/yaml") {
            let yaml_value: FigmentValue = serde_yaml::from_str(response)
                .map_err(|e| ConfigError::RemoteError(format!("Failed to parse YAML: {}", e)))?;
            let dict: Dict = yaml_value.deserialize().map_err(|e| {
                ConfigError::RemoteError(format!("Failed to convert YAML to dict: {}", e))
            })?;
            Figment::new().merge(Serialized::from(dict, Profile::Default))
        } else {
            let json_value: JsonValue = serde_json::from_str(response)
                .map_err(|e| ConfigError::RemoteError(format!("Failed to parse JSON: {}", e)))?;
            let dict: Dict = serde_json::from_value(json_value).map_err(|e| {
                ConfigError::RemoteError(format!("Failed to convert JSON to dict: {}", e))
            })?;
            Figment::new().merge(Serialized::from(dict, Profile::Default))
        };
        Ok(figment)
    }
}

impl ConfigProvider for HttpConfigProvider {
    fn load(&self) -> Result<Figment, ConfigError> {
        let client = self.build_client()?;

        let mut request = client.get(&self.url);

        if let Some(auth) = &self.auth {
            if let Some(token) = &auth.bearer_token {
                request = request.bearer_auth(token);
            } else {
                request = request.basic_auth(&auth.username, auth.password.as_deref());
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
