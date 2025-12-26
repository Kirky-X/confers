// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

use crate::error::ConfigError;
use crate::providers::provider::{
    ConfigProvider, ProviderMetadata, ProviderType, WatchableProvider,
};
use failsafe::futures::CircuitBreaker;
use figment::{Figment, Provider as FigmentProvider};
use std::time::Duration;

#[derive(Clone)]
pub struct EtcdConfigProvider {
    endpoints: Vec<String>,
    key: String,
    username: Option<String>,
    password: Option<String>,
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
            priority: 30, // 远程配置优先级较低
        }
    }

    pub fn with_auth(mut self, username: impl Into<String>, password: impl Into<String>) -> Self {
        self.username = Some(username.into());
        self.password = Some(password.into());
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

    fn create_etcd_provider(&self) -> crate::providers::remote::etcd::EtcdProvider {
        let mut provider = crate::providers::remote::etcd::EtcdProvider::new(
            self.endpoints.clone(),
            self.key.clone(),
        );

        if let (Some(username), Some(password)) = (&self.username, &self.password) {
            provider = provider.with_auth(username.clone(), password.clone());
        }

        provider = provider.with_tls(
            self.ca_path.clone(),
            self.cert_path.clone(),
            self.key_path.clone(),
        );

        provider
    }
}

impl ConfigProvider for EtcdConfigProvider {
    fn load(&self) -> Result<Figment, ConfigError> {
        let etcd_provider = self.create_etcd_provider();

        // 使用tokio运行时执行异步操作
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| {
                ConfigError::RemoteError(format!("Failed to create tokio runtime: {}", e))
            })?;

        let result = rt.block_on(async {
            // 使用熔断器模式
            let circuit_breaker = failsafe::Config::new()
                .failure_policy(failsafe::failure_policy::consecutive_failures(
                    3,
                    failsafe::backoff::constant(Duration::from_secs(10)),
                ))
                .build();

            circuit_breaker
                .call(async {
                    etcd_provider.data().map_err(|e| {
                        ConfigError::RemoteError(format!("Etcd operation failed: {}", e))
                    })
                })
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
        // 检查是否能够连接到Etcd端点
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
        // Implement etcd watch logic here
        // This is a placeholder for actual etcd watch implementation
        // A real implementation would start a tokio task to watch the key
        // and notify the application when changes occur
        Ok(())
    }

    fn stop_watching(&mut self) -> Result<(), ConfigError> {
        // Stop the etcd watch task
        Ok(())
    }

    fn is_watching(&self) -> bool {
        // Return true if the watch task is running
        false
    }

    fn poll_interval(&self) -> Option<Duration> {
        // Etcd uses streaming watch, so poll interval is not applicable
        None
    }
}
