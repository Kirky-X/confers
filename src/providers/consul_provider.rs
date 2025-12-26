use crate::error::ConfigError;
use crate::providers::provider::{ConfigProvider, ProviderMetadata, ProviderType};
use failsafe::CircuitBreaker;
use figment::{Figment, Provider as FigmentProvider};
use std::time::Duration;

pub struct ConsulConfigProvider {
    address: String,
    key: String,
    token: Option<String>,
    priority: u8,
}

impl ConsulConfigProvider {
    pub fn new(address: impl Into<String>, key: impl Into<String>) -> Self {
        Self {
            address: address.into(),
            key: key.into(),
            token: None,
            priority: 30, // 远程配置优先级较低
        }
    }

    pub fn with_token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(token.into());
        self
    }

    pub fn with_auth(self, _username: impl Into<String>, _password: impl Into<String>) -> Self {
        // Consul primarily uses tokens. Basic auth is rarely used or configured differently.
        // We implement this no-op to satisfy the common remote interface.
        // The username/password could be used for basic auth if the Consul server is configured for it.
        self
    }

    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }

    fn create_consul_provider(&self) -> crate::providers::remote::consul::ConsulProvider {
        let mut provider = crate::providers::remote::consul::ConsulProvider::new(
            self.address.clone(),
            self.key.clone(),
        );

        if let Some(token) = &self.token {
            provider = provider.with_token(token.clone());
        }

        provider
    }
}

impl ConfigProvider for ConsulConfigProvider {
    fn load(&self) -> Result<Figment, ConfigError> {
        let consul_provider = self.create_consul_provider();

        // 使用熔断器模式
        let circuit_breaker = failsafe::Config::new()
            .failure_policy(failsafe::failure_policy::consecutive_failures(
                3,
                failsafe::backoff::constant(Duration::from_secs(10)),
            ))
            .build();

        let result = circuit_breaker.call(|| {
            consul_provider
                .data()
                .map_err(|e| ConfigError::RemoteError(format!("Consul operation failed: {}", e)))
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
                "Circuit breaker open: Consul requests rejected".to_string(),
            )),
        }
    }

    fn name(&self) -> &str {
        "consul"
    }

    fn is_available(&self) -> bool {
        // 检查地址是否有效
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
            supports_watch: false, // Consul支持watch，但这里简化处理
            priority: self.priority,
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
