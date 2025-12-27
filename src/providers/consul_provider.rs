// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

use crate::error::ConfigError;
use crate::providers::provider::{ConfigProvider, ProviderMetadata, ProviderType};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use failsafe::{
    backoff, failure_policy, CircuitBreaker, Config as CircuitBreakerConfig, Error as FailsafeError,
};
use figment::{
    providers::Serialized,
    value::{Dict, Map},
    Figment, Profile,
};
use std::time::Duration;
use url::Url;

#[derive(Clone)]
pub struct ConsulConfigProvider {
    address: String,
    key: String,
    token: Option<String>,
    ca_path: Option<String>,
    cert_path: Option<String>,
    key_path: Option<String>,
    priority: u8,
}

#[derive(serde::Deserialize)]
#[allow(non_snake_case)]
struct ConsulKvPair {
    Value: String,
}

impl ConsulConfigProvider {
    pub fn new(address: impl Into<String>, key: impl Into<String>) -> Self {
        Self {
            address: address.into(),
            key: key.into(),
            token: None,
            ca_path: None,
            cert_path: None,
            key_path: None,
            priority: 30,
        }
    }

    pub fn from_address(address: impl Into<String>, key: impl Into<String>) -> Self {
        Self::new(address, key)
    }

    pub fn with_token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(token.into());
        self
    }

    pub fn with_auth(self, _username: impl Into<String>, _password: impl Into<String>) -> Self {
        self
    }

    pub fn with_tls(
        mut self,
        ca_path: Option<String>,
        cert_path: Option<String>,
        key_path: Option<String>,
    ) -> Self {
        self.ca_path = ca_path;
        self.cert_path = cert_path;
        self.key_path = key_path;
        self
    }

    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }

    fn build_url(&self) -> Result<Url, ConfigError> {
        let mut url = Url::parse(&self.address)
            .map_err(|e| ConfigError::RemoteError(format!("Invalid Consul URL: {}", e)))?;

        let path = url.path();
        let key = &self.key;

        if path == "/" || path.is_empty() {
            url.set_path(&format!("/v1/kv/{}", key));
        } else if path.ends_with("/v1/kv/") {
            url.set_path(&format!("{}{}", path, key));
        } else if path.contains("/v1/kv") {
            let new_path = format!("{}/{}", path.trim_end_matches('/'), key);
            url.set_path(&new_path);
        } else {
            let new_path = format!("{}/v1/kv/{}", path.trim_end_matches('/'), key);
            url.set_path(&new_path);
        }

        Ok(url)
    }

    fn build_client(&self) -> Result<reqwest::blocking::Client, ConfigError> {
        let mut client_builder = reqwest::blocking::Client::builder();

        if let (Some(ca_path), Some(cert_path), Some(_key_path)) =
            (&self.ca_path, &self.cert_path, &self.key_path)
        {
            client_builder = client_builder.add_root_certificate(
                reqwest::Certificate::from_pem(&std::fs::read(ca_path).map_err(|e| {
                    ConfigError::RemoteError(format!("Failed to read CA cert: {}", e))
                })?)
                .map_err(|e| ConfigError::RemoteError(format!("Failed to parse CA cert: {}", e)))?,
            );

            client_builder = client_builder.identity(
                reqwest::Identity::from_pem(&std::fs::read(cert_path).map_err(|e| {
                    ConfigError::RemoteError(format!("Failed to read client cert: {}", e))
                })?)
                .map_err(|e| {
                    ConfigError::RemoteError(format!("Failed to parse client cert: {}", e))
                })?,
            );
        } else if let Some(ca_path) = &self.ca_path {
            client_builder = client_builder.add_root_certificate(
                reqwest::Certificate::from_pem(&std::fs::read(ca_path).map_err(|e| {
                    ConfigError::RemoteError(format!("Failed to read CA cert: {}", e))
                })?)
                .map_err(|e| ConfigError::RemoteError(format!("Failed to parse CA cert: {}", e)))?,
            );
        }

        client_builder
            .build()
            .map_err(|e| ConfigError::RemoteError(format!("Failed to build client: {}", e)))
    }

    fn fetch_data(&self) -> Result<Map<Profile, Dict>, ConfigError> {
        let url = self.build_url()?;
        let client = self.build_client()?;

        let mut req = client.get(url);

        if let Some(token) = &self.token {
            req = req.header("X-Consul-Token", token);
        }

        let resp = req
            .send()
            .map_err(|e| ConfigError::RemoteError(format!("Failed to connect to Consul: {}", e)))?;

        if resp.status().is_success() {
            let kvs: Vec<ConsulKvPair> = resp.json().map_err(|e| {
                ConfigError::RemoteError(format!("Failed to parse Consul response: {}", e))
            })?;

            if let Some(kv) = kvs.first() {
                let val_str = &kv.Value;
                let decoded = BASE64.decode(val_str).map_err(|e| {
                    ConfigError::RemoteError(format!("Base64 decode failed: {}", e))
                })?;

                let json_str = String::from_utf8(decoded)
                    .map_err(|e| ConfigError::RemoteError(format!("UTF-8 error: {}", e)))?;

                let map: Dict = serde_json::from_str(&json_str).map_err(|e| {
                    ConfigError::RemoteError(format!("Failed to parse JSON: {}", e))
                })?;

                let mut profiles = Map::new();
                profiles.insert(Profile::Default, map);
                Ok(profiles)
            } else {
                Err(ConfigError::RemoteError(format!(
                    "Key {} not found in Consul (empty response)",
                    self.key
                )))
            }
        } else if resp.status() == reqwest::StatusCode::NOT_FOUND {
            Err(ConfigError::RemoteError(format!(
                "Key {} not found in Consul",
                self.key
            )))
        } else {
            Err(ConfigError::RemoteError(format!(
                "Consul returned error: {}",
                resp.status()
            )))
        }
    }
}

impl ConfigProvider for ConsulConfigProvider {
    fn load(&self) -> Result<Figment, ConfigError> {
        let circuit_breaker = CircuitBreakerConfig::new()
            .failure_policy(failure_policy::consecutive_failures(
                3,
                backoff::constant(Duration::from_secs(10)),
            ))
            .build();

        let result = circuit_breaker.call(|| self.fetch_data());

        match result {
            Ok(data) => {
                let figment = Figment::new().merge(Serialized::from(data, Profile::Default));
                Ok(figment)
            }
            Err(FailsafeError::Inner(e)) => Err(e),
            Err(FailsafeError::Rejected) => Err(ConfigError::RemoteError(
                "Circuit breaker open: Consul requests rejected".to_string(),
            )),
        }
    }

    fn name(&self) -> &str {
        "consul"
    }

    fn is_available(&self) -> bool {
        !self.address.is_empty() && self.address.starts_with("http")
    }

    fn priority(&self) -> u8 {
        self.priority
    }

    fn metadata(&self) -> ProviderMetadata {
        ProviderMetadata {
            name: self.name().to_string(),
            description: format!("Consul provider for key: {}", self.key),
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

#[deprecated(since = "0.4.0", note = "Use ConsulConfigProvider instead")]
pub type ConsulProvider = ConsulConfigProvider;
