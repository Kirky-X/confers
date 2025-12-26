// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

use crate::error::ConfigError;
use crate::providers::{ConfigProvider, ProviderMetadata, ProviderType};
use figment::Figment;
use serde::Serialize;

/// Default configuration provider that provides default values
pub struct DefaultConfigProvider<T> {
    defaults: T,
    name: String,
    priority: u8,
}

impl<T> DefaultConfigProvider<T>
where
    T: Serialize + Send + Sync,
{
    pub fn new(defaults: T) -> Self {
        Self {
            defaults,
            name: "default".to_string(),
            priority: 0, // Lowest priority
        }
    }
}

impl<T> ConfigProvider for DefaultConfigProvider<T>
where
    T: Serialize + Send + Sync + 'static,
{
    fn load(&self) -> Result<Figment, ConfigError> {
        Ok(Figment::new().merge(figment::providers::Serialized::from(
            &self.defaults,
            figment::Profile::Default,
        )))
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn is_available(&self) -> bool {
        true // Always available
    }

    fn priority(&self) -> u8 {
        self.priority
    }

    fn metadata(&self) -> ProviderMetadata {
        ProviderMetadata {
            name: self.name.clone(),
            description: "Default configuration values".to_string(),
            source_type: ProviderType::Serialized,
            requires_network: false,
            supports_watch: false,
            priority: self.priority,
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
