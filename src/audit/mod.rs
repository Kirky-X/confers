// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

use crate::error::ConfigError;
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub trait Sanitize {
    fn sanitize(&self) -> Value;
}

#[derive(Debug, Clone)]
pub struct SanitizeConfig {
    pub mask_char: char,
    pub visible_chars: usize,
    pub max_visible_length: usize,
    pub fixed_mask_length: Option<usize>,
}

impl Default for SanitizeConfig {
    fn default() -> Self {
        Self {
            mask_char: '*',
            visible_chars: 4,
            max_visible_length: 100,
            fixed_mask_length: None,
        }
    }
}

impl SanitizeConfig {
    pub fn new() -> Self {
        Self::default()
    }
}

pub fn sanitize_value(value: &Value, config: &SanitizeConfig) -> Value {
    match value {
        Value::String(s) => {
            if let Some(len) = config.fixed_mask_length {
                json!(std::iter::repeat(config.mask_char)
                    .take(len)
                    .collect::<String>())
            } else if s.len() <= config.visible_chars {
                json!(s.chars().map(|_| config.mask_char).collect::<String>())
            } else {
                let visible_part = &s[..config.visible_chars];
                let masked_part = s[config.visible_chars..]
                    .chars()
                    .map(|_| config.mask_char)
                    .take(config.max_visible_length)
                    .collect::<String>();
                json!(format!("{}...{}", visible_part, masked_part))
            }
        }
        Value::Object(map) => {
            let mut sanitized = serde_json::Map::new();
            for (k, v) in map {
                sanitized.insert(k.clone(), sanitize_value(v, config));
            }
            Value::Object(sanitized)
        }
        Value::Array(arr) => Value::Array(arr.iter().map(|v| sanitize_value(v, config)).collect()),
        _ => value.clone(),
    }
}

pub fn sanitize_string(value: &str, config: &SanitizeConfig) -> String {
    if let Some(len) = config.fixed_mask_length {
        std::iter::repeat(config.mask_char)
            .take(len)
            .collect::<String>()
    } else if value.len() <= config.visible_chars {
        value.chars().map(|_| config.mask_char).collect::<String>()
    } else {
        let visible_part = &value[..config.visible_chars];
        let masked_part = value[config.visible_chars..]
            .chars()
            .map(|_| config.mask_char)
            .take(config.max_visible_length)
            .collect::<String>();
        format!("{}...{}", visible_part, masked_part)
    }
}

#[macro_export]
macro_rules! sanitize_impl {
    ($struct_name:ident, $($field:ident),*) => {
        impl $crate::audit::Sanitize for $struct_name {
            fn sanitize(&self) -> serde_json::Value {
                let mut map = serde_json::Map::new();
                $(
                    let value = serde_json::to_value(&self.$field).unwrap_or(serde_json::Value::Null);
                    map.insert(stringify!($field).to_string(), value);
                )*
                serde_json::Value::Object(map)
            }
        }
    };
}

#[macro_export]
macro_rules! sanitize_impl_with_sensitive {
    ($struct_name:ident, { $($field:ident => $sensitive:expr),+ }) => {
        impl $crate::audit::Sanitize for $struct_name {
            fn sanitize(&self) -> serde_json::Value {
                let mut map = serde_json::Map::new();
                $(
                    let value = serde_json::to_value(&self.$field).unwrap_or(serde_json::Value::Null);
                    let sanitized = if $sensitive {
                        $crate::audit::sanitize_value(&value, &$crate::audit::SanitizeConfig::default())
                    } else {
                        value
                    };
                    map.insert(stringify!($field).to_string(), sanitized);
                )+
                serde_json::Value::Object(map)
            }
        }
    };
}

#[macro_export]
macro_rules! sanitize_sensitive_impl {
    ($struct_name:ident, $($field:ident),*) => {
        impl $crate::audit::Sanitize for $struct_name {
            fn sanitize(&self) -> serde_json::Value {
                let mut map = serde_json::Map::new();
                $(
                    let value = serde_json::to_value(&self.$field).unwrap_or(serde_json::Value::Null);
                    let sanitized = $crate::audit::sanitize_value(&value, &$crate::audit::SanitizeConfig::default());
                    map.insert(stringify!($field).to_string(), sanitized);
                )*
                serde_json::Value::Object(map)
            }
        }
    };
}

#[macro_export]
macro_rules! sanitize_mixed_impl {
    ($struct_name:ident, { $($field:ident => $sensitive:expr),+ }) => {
        impl $crate::audit::Sanitize for $struct_name {
            fn sanitize(&self) -> serde_json::Value {
                let mut map = serde_json::Map::new();
                $(
                    let value = serde_json::to_value(&self.$field).unwrap_or(serde_json::Value::Null);
                    let sanitized = if $sensitive {
                        $crate::audit::sanitize_value(&value, &$crate::audit::SanitizeConfig::default())
                    } else {
                        value
                    };
                    map.insert(stringify!($field).to_string(), sanitized);
                )+
                serde_json::Value::Object(map)
            }
        }
    };
}

#[derive(Serialize)]
struct ConfigSourceStatus {
    source: String,
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    load_time_ms: Option<u64>,
}

/// Configuration for audit logging parameters
#[derive(Debug, Clone, Default)]
pub struct AuditConfig {
    pub validation_error: Option<String>,
    pub config_source: Option<String>,
    pub load_duration: Option<Duration>,
    #[allow(clippy::type_complexity)]
    pub config_sources_status: Option<Vec<(String, String, Option<String>, Option<Duration>)>>,
    pub files_attempted: Option<u32>,
    pub files_loaded: Option<u32>,
    pub format_distribution: Option<HashMap<String, u32>>,
    pub env_vars_count: Option<u32>,
    pub memory_usage_mb: Option<f64>,
}

#[derive(Serialize)]
struct AuditMetadata {
    timestamp: u64,
    // System metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    app_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    hostname: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    user: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    working_directory: Option<String>,
    process_id: u64,

    // Environment details
    #[serde(skip_serializing_if = "Option::is_none")]
    run_env: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    rust_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    target_triple: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    os_info: Option<String>,

    // Config loading metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    load_duration_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    config_sources: Option<Vec<ConfigSourceStatus>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    validation_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    validation_error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    config_source: Option<String>,

    // Load statistics
    #[serde(skip_serializing_if = "Option::is_none")]
    files_attempted: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    files_loaded: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    format_distribution: Option<std::collections::HashMap<String, u32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    env_vars_count: Option<u32>,

    // Performance metrics
    #[serde(skip_serializing_if = "Option::is_none")]
    memory_usage_mb: Option<f64>,
}

pub struct AuditLogger;

impl AuditLogger {
    pub fn log_to_file<T>(
        config: &T,
        path: &Path,
        validation_error: Option<&str>,
    ) -> Result<(), ConfigError>
    where
        T: Serialize + Sanitize,
    {
        let audit_config = AuditConfig {
            validation_error: validation_error.map(String::from),
            config_source: None,
            load_duration: None,
            config_sources_status: None,
            files_attempted: None,
            files_loaded: None,
            format_distribution: None,
            env_vars_count: None,
            memory_usage_mb: None,
        };

        Self::log_to_file_with_source(config, path, audit_config)
    }

    #[allow(clippy::type_complexity)]
    pub fn log_to_file_with_source<T>(
        config: &T,
        path: &Path,
        audit_config: AuditConfig,
    ) -> Result<(), ConfigError>
    where
        T: Serialize + Sanitize,
    {
        let metadata = AuditMetadata {
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),

            // System metadata
            app_name: std::env::var("CARGO_PKG_NAME").ok(),
            version: std::env::var("CARGO_PKG_VERSION").ok(),
            hostname: std::env::var("HOSTNAME")
                .ok()
                .or_else(|| std::env::var("COMPUTERNAME").ok()),
            user: std::env::var("USER")
                .ok()
                .or_else(|| std::env::var("USERNAME").ok()),
            working_directory: std::env::current_dir()
                .ok()
                .and_then(|p| p.to_str().map(String::from)),
            process_id: std::process::id().into(),

            // Environment details
            run_env: std::env::var("RUN_ENV")
                .or_else(|_| std::env::var("APP_ENV"))
                .ok(),
            rust_version: option_env!("RUSTC_VERSION").map(|s| s.to_string()),
            target_triple: option_env!("TARGET").map(|s| s.to_string()),
            os_info: Some(format!(
                "{} {}",
                std::env::consts::OS,
                std::env::consts::ARCH
            )),

            // Config loading metadata
            load_duration_ms: audit_config.load_duration.map(|d| d.as_millis() as u64),
            config_sources: audit_config.config_sources_status.map(|sources| {
                sources
                    .into_iter()
                    .map(|(source, status, error, load_time)| ConfigSourceStatus {
                        source,
                        status,
                        error,
                        load_time_ms: load_time.map(|d| d.as_millis() as u64),
                    })
                    .collect()
            }),
            validation_status: if audit_config.validation_error.is_some() {
                Some("Failed".to_string())
            } else {
                Some("Success".to_string())
            },
            validation_error: audit_config.validation_error,
            config_source: audit_config.config_source,

            // Load statistics
            files_attempted: audit_config.files_attempted,
            files_loaded: audit_config.files_loaded,
            format_distribution: audit_config.format_distribution,
            env_vars_count: audit_config.env_vars_count,

            // Performance metrics
            memory_usage_mb: audit_config.memory_usage_mb,
        };

        let sanitized_config = config.sanitize();
        let audit_entry = json!({
            "metadata": metadata,
            "config": sanitized_config,
        });

        let json_str = serde_json::to_string(&audit_entry)
            .map_err(|e| ConfigError::ParseError(e.to_string()))?;

        use std::fs::OpenOptions;
        use std::io::Write;

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|e| ConfigError::RemoteError(e.to_string()))?;

        writeln!(file, "{}", json_str).map_err(|e| ConfigError::RemoteError(e.to_string()))?;

        Ok(())
    }
}
