// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

use crate::error::ConfigError;
use crate::utils::ssrf::validate_remote_url;
use figment::value::Value as FigmentValue;
use figment::{
    providers::Serialized,
    value::{Dict, Map},
    Error, Figment, Profile, Provider,
};
use lazy_static::lazy_static;
use serde_json::Value as JsonValue;
use std::sync::Arc;
use std::time::Duration;

/// Global HTTP client with connection pooling
/// Reuses connections across multiple requests to improve performance
static HTTP_CLIENT: lazy_static::lazy_static! {
    Arc::new(
            reqwest::blocking::Client::builder()
                .pool_max_idle_per_host(10)
                .pool_idle_timeout(Duration::from_secs(90))
                .timeout(Duration::from_secs(30))
                .build()
                .expect("Failed to create HTTP client"),
        )
    };

/// Global async HTTP client with connection pooling
static HTTP_CLIENT_ASYNC: lazy_static::lazy_static! {
    Arc::new(
            reqwest::Client::builder()
                .pool_max_idle_per_host(10)
                .pool_idle_timeout(Duration::from_secs(90))
                .timeout(Duration::from_secs(30))
                .build()
                .expect("Failed to create async HTTP client"),
        )
    };

pub struct HttpProvider {
    url: String,
    auth: Option<HttpAuth>,
    tls_config: Option<TlsConfig>,
    timeout: Option<String>,
}

pub struct TlsConfig {
    pub ca_cert: Option<std::path::PathBuf>,
    pub client_cert: Option<std::path::PathBuf>,
    pub client_key: Option<std::path::PathBuf>,
}

#[derive(Clone)]
pub struct HttpAuth {
    pub username: String,
    pub password: Option<String>,
    pub bearer_token: Option<String>,
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
            timeout: None,
        })
    }

    pub fn from_url(url: impl Into<String>) -> Result<Self, ConfigError> {
        Self::new(url)
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

    pub fn with_timeout(mut self, timeout: impl Into<String>) -> Self {
        self.timeout = Some(timeout.into());
        self
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

    pub fn load_sync(&self) -> Result<Figment, ConfigError> {
        let client = (*HTTP_CLIENT).clone();

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

        let figment: Figment = if content_type.contains("application/json") {
            let json_value: JsonValue = response
                .json()
                .map_err(|e| ConfigError::RemoteError(format!("Failed to parse JSON: {}", e)))?;
            let dict: Dict = serde_json::from_value(json_value).map_err(|e| {
                ConfigError::RemoteError(format!("Failed to convert JSON to dict: {}", e))
            })?;
            Figment::new().merge(Serialized::from(dict, Profile::Default))
        } else if content_type.contains("application/toml") || content_type.contains("text/toml") {
            let toml_str = response.text().map_err(|e| {
                ConfigError::RemoteError(format!("Failed to read TOML response: {}", e))
            })?;
            let toml_value: FigmentValue = toml::from_str(&toml_str)
                .map_err(|e| ConfigError::RemoteError(format!("Failed to parse TOML: {}", e)))?;
            let dict: Dict = toml_value.deserialize().map_err(|e| {
                ConfigError::RemoteError(format!("Failed to convert TOML to dict: {}", e))
            })?;
            Figment::new().merge(Serialized::from(dict, Profile::Default))
        } else if content_type.contains("application/yaml") || content_type.contains("text/yaml") {
            let yaml_str = response.text().map_err(|e| {
                ConfigError::RemoteError(format!("Failed to read YAML response: {}", e))
            })?;
            let yaml_value: FigmentValue = serde_yaml::from_str(&yaml_str)
                .map_err(|e| ConfigError::RemoteError(format!("Failed to parse YAML: {}", e)))?;
            let dict: Dict = yaml_value.deserialize().map_err(|e| {
                ConfigError::RemoteError(format!("Failed to convert YAML to dict: {}", e))
            })?;
            Figment::new().merge(Serialized::from(dict, Profile::Default))
        } else {
            let json_value: JsonValue = response
                .json()
                .map_err(|e| ConfigError::RemoteError(format!("Failed to parse JSON: {}", e)))?;
            let dict: Dict = serde_json::from_value(json_value).map_err(|e| {
                ConfigError::RemoteError(format!("Failed to convert JSON to dict: {}", e))
            })?;
            Figment::new().merge(Serialized::from(dict, Profile::Default))
        };

        Ok(figment)
    }

    pub async fn load(&self) -> Result<Figment, ConfigError> {
        let client = (*HTTP_CLIENT_ASYNC).clone();

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

        let figment: Figment = if content_type.contains("application/json") {
            let json_value: JsonValue = response
                .json()
                .await
                .map_err(|e| ConfigError::RemoteError(format!("Failed to parse JSON: {}", e)))?;
            let dict: Dict = serde_json::from_value(json_value).map_err(|e| {
                ConfigError::RemoteError(format!("Failed to convert JSON to dict: {}", e))
            })?;
            Figment::new().merge(Serialized::from(dict, Profile::Default))
        } else if content_type.contains("application/toml") || content_type.contains("text/toml") {
            let toml_str = response.text().await.map_err(|e| {
                ConfigError::RemoteError(format!("Failed to read TOML response: {}", e))
            })?;
            let toml_value: FigmentValue = toml::from_str(&toml_str)
                .map_err(|e| ConfigError::RemoteError(format!("Failed to parse TOML: {}", e)))?;
            let dict: Dict = toml_value.deserialize().map_err(|e| {
                ConfigError::RemoteError(format!("Failed to convert TOML to dict: {}", e))
            })?;
            Figment::new().merge(Serialized::from(dict, Profile::Default))
        } else if content_type.contains("application/yaml") || content_type.contains("text/yaml") {
            let yaml_str = response.text().await.map_err(|e| {
                ConfigError::RemoteError(format!("Failed to read YAML response: {}", e))
            })?;
            let yaml_value: FigmentValue = serde_yaml::from_str(&yaml_str)
                .map_err(|e| ConfigError::RemoteError(format!("Failed to parse YAML: {}", e)))?;
            let dict: Dict = yaml_value.deserialize().map_err(|e| {
                ConfigError::RemoteError(format!("Failed to convert YAML to dict: {}", e))
            })?;
            Figment::new().merge(Serialized::from(dict, Profile::Default))
        } else {
            let json_value: JsonValue = response
                .json()
                .await
                .map_err(|e| ConfigError::RemoteError(format!("Failed to parse JSON: {}", e)))?;
            let dict: Dict = serde_json::from_value(json_value).map_err(|e| {
                ConfigError::RemoteError(format!("Failed to convert JSON to dict: {}", e))
            })?;
            Figment::new().merge(Serialized::from(dict, Profile::Default))
        };

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
