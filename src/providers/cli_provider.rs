// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

use crate::error::ConfigError;
use crate::providers::provider::{ConfigProvider, ProviderMetadata, ProviderType};
use figment::{
    value::{Dict, Value},
    Figment,
};
use std::collections::HashMap;

#[derive(Clone)]
pub struct CliConfigProvider {
    overrides: HashMap<String, String>,
    priority: u8,
}

impl CliConfigProvider {
    pub fn from_args<I, S>(args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut overrides = HashMap::new();

        for arg in args {
            let s = arg.as_ref();
            if let Some((key, value)) = s.split_once('=') {
                overrides.insert(key.to_string(), value.to_string());
            }
        }

        Self {
            overrides,
            priority: 50,
        }
    }

    pub fn from_env_var(env_var: &str) -> Self {
        let mut overrides = HashMap::new();

        if let Ok(value) = std::env::var(env_var) {
            for pair in value.split(',') {
                if let Some((key, val)) = pair.split_once('=') {
                    overrides.insert(key.trim().to_string(), val.trim().to_string());
                }
            }
        }

        Self {
            overrides,
            priority: 50,
        }
    }

    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }

    fn parse_value(v: &str) -> Option<Value> {
        if v == "null" || v == "None" {
            return None;
        }
        if let Ok(b) = v.parse::<bool>() {
            return Some(Value::from(b));
        }
        if let Ok(i) = v.parse::<i64>() {
            return Some(Value::from(i));
        }
        if let Ok(f) = v.parse::<f64>() {
            return Some(Value::from(f));
        }
        Some(Value::from(v))
    }

    fn insert_nested(map: &mut Dict, key: &str, value: Value) {
        if let Some((head, tail)) = key.split_once('.') {
            let entry = map
                .entry(head.to_string())
                .or_insert_with(|| Value::from(Dict::new()));

            if let Value::Dict(_, ref mut inner_map) = entry {
                Self::insert_nested(inner_map, tail, value);
            } else {
                let mut inner_map = Dict::new();
                Self::insert_nested(&mut inner_map, tail, value);
                *entry = Value::from(inner_map);
            }
        } else {
            map.insert(key.to_string(), value);
        }
    }
}

impl ConfigProvider for CliConfigProvider {
    fn load(&self) -> Result<Figment, ConfigError> {
        let mut data = Dict::new();

        for (key, value) in &self.overrides {
            if let Some(val) = Self::parse_value(value) {
                Self::insert_nested(&mut data, key, val);
            }
        }

        let figment = Figment::new().merge(figment::providers::Serialized::from(
            data,
            figment::Profile::Default,
        ));

        Ok(figment)
    }

    fn name(&self) -> &str {
        "cli"
    }

    fn is_available(&self) -> bool {
        !self.overrides.is_empty()
    }

    fn priority(&self) -> u8 {
        self.priority
    }

    fn metadata(&self) -> ProviderMetadata {
        ProviderMetadata {
            name: self.name().to_string(),
            description: format!("CLI provider with {} overrides", self.overrides.len()),
            source_type: ProviderType::Cli,
            requires_network: false,
            supports_watch: false,
            priority: self.priority,
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[deprecated(since = "0.4.0", note = "Use CliConfigProvider instead")]
pub type CliProvider = CliConfigProvider;
