// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

use crate::error::ConfigError;
use crate::providers::{ConfigProvider, ProviderMetadata, ProviderType};
use crate::security::get_global_validator;
use figment::Figment;

fn insert_nested_value(
    map: &mut figment::value::Dict,
    keys: &[&str],
    value: figment::value::Value,
) {
    if keys.is_empty() {
        return;
    }

    if keys.len() == 1 {
        map.insert(keys[0].to_string(), value);
    } else {
        let entry = map
            .entry(keys[0].to_string())
            .or_insert_with(|| figment::value::Value::from(figment::value::Dict::new()));

        if let figment::value::Value::Dict(_, ref mut inner_map) = entry {
            insert_nested_value(inner_map, &keys[1..], value);
        } else {
            let mut inner_map = figment::value::Dict::new();
            insert_nested_value(&mut inner_map, &keys[1..], value);
            *entry = figment::value::Value::from(inner_map);
        }
    }
}

#[derive(Clone)]
pub struct EnvironmentProvider {
    prefix: String,
    separator: String,
    name: String,
    priority: u8,
    custom_mappings: std::collections::HashMap<String, String>,
}

impl EnvironmentProvider {
    pub fn new(prefix: impl Into<String>) -> Self {
        let prefix = prefix.into();
        Self {
            prefix: if prefix.ends_with('_') {
                prefix
            } else {
                format!("{}_", prefix)
            },
            separator: "__".to_string(),
            name: "environment".to_string(),
            priority: 30,
            custom_mappings: std::collections::HashMap::new(),
        }
    }

    pub fn from_prefix(prefix: impl Into<String>) -> Self {
        let prefix = prefix.into();
        let figment_prefix = if prefix.ends_with('_') {
            prefix.clone()
        } else {
            format!("{}_", prefix)
        };
        Self {
            prefix: figment_prefix,
            separator: "__".to_string(),
            name: "env".to_string(),
            priority: 30,
            custom_mappings: std::collections::HashMap::new(),
        }
    }

    pub fn with_separator(mut self, separator: impl Into<String>) -> Self {
        self.separator = separator.into();
        self
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_custom_mappings(
        mut self,
        mappings: std::collections::HashMap<String, String>,
    ) -> Self {
        self.custom_mappings = mappings;
        self
    }

    /// Check if any environment variables with the given prefix exist
    pub fn has_env_vars(&self) -> bool {
        std::env::vars().any(|(key, _)| key.starts_with(&self.prefix))
    }

    /// Get all environment variables with the given prefix
    pub fn get_env_vars(&self) -> std::collections::HashMap<String, String> {
        std::env::vars()
            .filter(|(key, _)| key.starts_with(&self.prefix))
            .collect()
    }
}

impl ConfigProvider for EnvironmentProvider {
    fn load(&self) -> Result<Figment, ConfigError> {
        let mut figment = Figment::new();

        let validator = get_global_validator();

        let env_vars = self.get_env_vars();
        // Reduced logging to avoid performance issues in tests
        if cfg!(debug_assertions) {
            tracing::debug!(
                "Found {} env vars with prefix '{}'",
                env_vars.len(),
                self.prefix
            );
        }

        if !env_vars.is_empty() {
            let mut nested_env_map = figment::value::Dict::new();
            let mut validation_warnings = Vec::new();

            for (key, value) in env_vars {
                if let Err(e) = validator.validate_env_name(&key, Some(&value)) {
                    validation_warnings.push(format!("环境变量名称 '{}' 验证失败: {}", key, e));
                    continue;
                }

                if value.is_empty() {
                    continue;
                }

                if let Err(e) = validator.validate_env_value(&value) {
                    validation_warnings.push(format!("环境变量值 '{}' 验证失败: {}", key, e));
                    continue;
                }

                let field_key = key.strip_prefix(&self.prefix).unwrap_or(&key);
                let field_key = field_key.to_lowercase();
                let field_key = if self.separator == "__" {
                    field_key.replace("__", ".")
                } else {
                    field_key.replace(&self.separator, ".")
                };

                let parsed_value = if let Ok(int_val) = value.parse::<i64>() {
                    figment::value::Value::from(int_val)
                } else if let Ok(bool_val) = value.parse::<bool>() {
                    figment::value::Value::from(bool_val)
                } else {
                    figment::value::Value::from(value.clone())
                };

                let keys: Vec<&str> = field_key.split('.').collect();
                insert_nested_value(&mut nested_env_map, &keys, parsed_value);
            }

            if !validation_warnings.is_empty() {
                tracing::warn!(
                    "Environment variable validation warnings: {}",
                    validation_warnings.join("; ")
                );
            }

            if !nested_env_map.is_empty() {
                figment = figment.merge(figment::providers::Serialized::from(
                    nested_env_map,
                    figment::Profile::Default,
                ));
            }
        }

        if !self.custom_mappings.is_empty() {
            if let Err(e) = validator.validate_env_mapping(&self.custom_mappings) {
                tracing::warn!("Failed to validate env mapping: {}", e);
            }

            let mut custom_env_map = figment::value::Dict::new();

            for (field_name, env_name) in &self.custom_mappings {
                if let Ok(value) = std::env::var(env_name) {
                    if let Err(e) = validator.validate_env_value(&value) {
                        tracing::warn!(
                            "Failed to validate custom mapping env value for '{}': {}",
                            field_name,
                            e
                        );
                        continue;
                    }

                    let parsed_value = if let Ok(int_val) = value.parse::<i64>() {
                        figment::value::Value::from(int_val)
                    } else if let Ok(bool_val) = value.parse::<bool>() {
                        figment::value::Value::from(bool_val)
                    } else {
                        figment::value::Value::from(value)
                    };

                    let keys: Vec<&str> = field_name.split('.').collect();
                    insert_nested_value(&mut custom_env_map, &keys, parsed_value);
                }
            }

            if !custom_env_map.is_empty() {
                figment = figment.merge(figment::providers::Serialized::from(
                    custom_env_map,
                    figment::Profile::Default,
                ));
            }
        }

        Ok(figment)
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn is_available(&self) -> bool {
        self.has_env_vars() || !self.custom_mappings.is_empty()
    }

    fn priority(&self) -> u8 {
        self.priority
    }

    fn metadata(&self) -> ProviderMetadata {
        ProviderMetadata {
            name: self.name.clone(),
            description: format!(
                "Environment variable provider with prefix '{}' and {} custom mappings",
                self.prefix,
                self.custom_mappings.len()
            ),
            source_type: ProviderType::Environment,
            requires_network: false,
            supports_watch: false,
            priority: self.priority,
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[deprecated(since = "0.4.0", note = "Use EnvironmentProvider instead")]
pub type EnvProvider = EnvironmentProvider;

/// Standard environment provider with common conventions
pub struct StandardEnvironmentProvider {
    app_name: String,
    base_provider: EnvironmentProvider,
}

impl StandardEnvironmentProvider {
    pub fn new(app_name: impl Into<String>) -> Self {
        let app_name = app_name.into();
        let env_prefix = app_name.to_uppercase().replace('-', "_");

        Self {
            app_name: app_name.clone(),
            base_provider: EnvironmentProvider::new(&env_prefix),
        }
    }

    pub fn with_custom_mappings(
        mut self,
        mappings: std::collections::HashMap<String, String>,
    ) -> Self {
        self.base_provider = self.base_provider.with_custom_mappings(mappings);
        self
    }
}

impl ConfigProvider for StandardEnvironmentProvider {
    fn load(&self) -> Result<Figment, ConfigError> {
        self.base_provider.load()
    }

    fn name(&self) -> &str {
        "standard_environment"
    }

    fn is_available(&self) -> bool {
        self.base_provider.is_available()
    }

    fn priority(&self) -> u8 {
        self.base_provider.priority()
    }

    fn metadata(&self) -> ProviderMetadata {
        ProviderMetadata {
            name: "standard_environment".to_string(),
            description: format!("Standard environment provider for app '{}'", self.app_name),
            source_type: ProviderType::Environment,
            requires_network: false,
            supports_watch: false,
            priority: self.base_provider.priority(),
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
