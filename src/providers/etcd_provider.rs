// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

use crate::error::ConfigError;
use crate::providers::provider::{
    ConfigProvider, ProviderMetadata, ProviderType, WatchableProvider,
};
use crate::security::{SecureString, SensitivityLevel};
use crate::utils::ssrf::validate_remote_url;
use etcd_client::{Client, ConnectOptions, Identity, TlsOptions};
use failsafe::futures::CircuitBreaker;
use figment::{
    value::{Dict, Map},
    Figment, Profile,
};
use std::fs;
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone)]
pub struct EtcdConfigProvider {
    endpoints: Vec<String>,
    key: String,
    username: Option<String>,
    password: Option<Arc<SecureString>>,
    ca_path: Option<String>,
    cert_path: Option<String>,
    key_path: Option<String>,
    priority: u8,
}

impl EtcdConfigProvider {
    pub fn new(endpoints: Vec<String>, key: impl Into<String>) -> Self {
        Self {
            endpoints,
            key: key.into(),
            username: None,
            password: None,
            ca_path: None,
            cert_path: None,
            key_path: None,
            priority: 30,
        }
    }

    pub fn from_endpoints(endpoints: Vec<String>, key: impl Into<String>) -> Self {
        Self::new(endpoints, key)
    }

    pub fn with_auth(mut self, username: impl Into<String>, password: impl Into<String>) -> Self {
        self.username = Some(username.into());
        self.password = Some(Arc::new(SecureString::new(password.into(), SensitivityLevel::Critical)));
        self
    }

    pub fn with_auth_secure(mut self, username: impl Into<String>, password: Arc<SecureString>) -> Self {
        self.username = Some(username.into());
        self.password = Some(password);
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

    fn build_connect_options(&self) -> Result<ConnectOptions, ConfigError> {
        let mut options = ConnectOptions::new();

        if let (Some(username), Some(password)) = (&self.username, &self.password) {
            options = options.with_user(username, password.as_str());
        }

        if let (Some(ca), Some(cert), Some(key_p)) =
            (&self.ca_path, &self.cert_path, &self.key_path)
        {
            let ca_pem = fs::read_to_string(ca)
                .map_err(|e| ConfigError::RemoteError(format!("Failed to read CA file: {}", e)))?;
            let cert_pem = fs::read_to_string(cert).map_err(|e| {
                ConfigError::RemoteError(format!("Failed to read cert file: {}", e))
            })?;
            let key_pem = fs::read_to_string(key_p)
                .map_err(|e| ConfigError::RemoteError(format!("Failed to read key file: {}", e)))?;

            let mut tls =
                TlsOptions::new().ca_certificate(etcd_client::Certificate::from_pem(ca_pem));
            tls = tls.identity(Identity::from_pem(cert_pem, key_pem));
            options = options.with_tls(tls);
        } else if let Some(ca) = &self.ca_path {
            let ca_pem = fs::read_to_string(ca)
                .map_err(|e| ConfigError::RemoteError(format!("Failed to read CA file: {}", e)))?;
            let tls = TlsOptions::new().ca_certificate(etcd_client::Certificate::from_pem(ca_pem));
            options = options.with_tls(tls);
        }

        Ok(options)
    }

    async fn fetch_from_etcd(&self) -> Result<Map<Profile, Dict>, ConfigError> {
        let options = self.build_connect_options()?;

        let mut client = Client::connect(&self.endpoints, Some(options))
            .await
            .map_err(|e| ConfigError::RemoteError(format!("Failed to connect to Etcd: {}", e)))?;

        let resp = client
            .get(self.key.as_bytes(), None)
            .await
            .map_err(|e| ConfigError::RemoteError(format!("Failed to get key from Etcd: {}", e)))?;

        if let Some(kv) = resp.kvs().first() {
            let val_str = kv
                .value_str()
                .map_err(|e| ConfigError::RemoteError(format!("Failed to read value: {}", e)))?;
            let map: Dict = serde_json::from_str(val_str)
                .map_err(|e| ConfigError::RemoteError(format!("Failed to parse JSON: {}", e)))?;

            let mut profiles = Map::new();
            profiles.insert(Profile::Default, map);
            Ok(profiles)
        } else {
            Err(ConfigError::RemoteError(format!(
                "Key {} not found in Etcd",
                self.key
            )))
        }
    }
}

impl ConfigProvider for EtcdConfigProvider {
    fn load(&self) -> Result<Figment, ConfigError> {
        // Validate all endpoints to prevent SSRF attacks
        for endpoint in &self.endpoints {
            validate_remote_url(endpoint)?;
        }

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| {
                ConfigError::RemoteError(format!("Failed to create tokio runtime: {}", e))
            })?;

        let result = rt.block_on(async {
            let circuit_breaker = failsafe::Config::new()
                .failure_policy(failsafe::failure_policy::consecutive_failures(
                    3,
                    failsafe::backoff::constant(Duration::from_secs(10)),
                ))
                .build();

            circuit_breaker
                .call(async { self.fetch_from_etcd().await })
                .await
        });

        match result {
            Ok(data) => {
                let figment = Figment::new().merge(figment::providers::Serialized::from(
                    data,
                    figment::Profile::Default,
                ));
                Ok(figment)
            }
            Err(failsafe::Error::Inner(e)) => Err(e),
            Err(failsafe::Error::Rejected) => Err(ConfigError::RemoteError(
                "Circuit breaker open: Etcd requests rejected".to_string(),
            )),
        }
    }

    fn name(&self) -> &str {
        "etcd"
    }

    fn is_available(&self) -> bool {
        !self.endpoints.is_empty()
    }

    fn priority(&self) -> u8 {
        self.priority
    }

    fn metadata(&self) -> ProviderMetadata {
        ProviderMetadata {
            name: self.name().to_string(),
            description: format!("Etcd provider for key: {}", self.key),
            source_type: ProviderType::Remote,
            requires_network: true,
            supports_watch: true, // Etcd supports watch
            priority: self.priority,
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl WatchableProvider for EtcdConfigProvider {
    fn start_watching(&mut self) -> Result<(), ConfigError> {
        Ok(())
    }

    fn stop_watching(&mut self) -> Result<(), ConfigError> {
        Ok(())
    }

    fn is_watching(&self) -> bool {
        false
    }

    fn poll_interval(&self) -> Option<Duration> {
        None
    }
}

#[deprecated(since = "0.4.0", note = "Use EtcdConfigProvider instead")]
pub type EtcdProvider = EtcdConfigProvider;
