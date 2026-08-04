// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

use chrono::{DateTime, Utc};

use crate::error::{ConfigError, ConfigResult};

#[derive(Debug, Clone)]
pub enum AuditEvent {
    KeyAccess {
        key: String,
        timestamp: DateTime<Utc>,
    },
    KeyRotation {
        old_version: String,
        new_version: String,
        timestamp: DateTime<Utc>,
    },
    Decrypt {
        field: String,
        success: bool,
        timestamp: DateTime<Utc>,
    },
    LoadSuccess {
        source: String,
        timestamp: DateTime<Utc>,
    },
    ReloadTrigger {
        source: String,
        timestamp: DateTime<Utc>,
    },
}

impl AuditEvent {
    /// Return the timestamp embedded in the event.
    pub fn event_timestamp(&self) -> DateTime<Utc> {
        match self {
            AuditEvent::KeyAccess { timestamp, .. } => *timestamp,
            AuditEvent::KeyRotation { timestamp, .. } => *timestamp,
            AuditEvent::Decrypt { timestamp, .. } => *timestamp,
            AuditEvent::LoadSuccess { timestamp, .. } => *timestamp,
            AuditEvent::ReloadTrigger { timestamp, .. } => *timestamp,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditLevel {
    BestEffort,
    Durable,
}

impl AuditLevel {
    pub fn for_event(event: &AuditEvent) -> Self {
        match event {
            AuditEvent::KeyAccess { .. } => AuditLevel::Durable,
            AuditEvent::KeyRotation { .. } => AuditLevel::Durable,
            AuditEvent::Decrypt { .. } => AuditLevel::Durable,
            AuditEvent::LoadSuccess { .. } => AuditLevel::BestEffort,
            AuditEvent::ReloadTrigger { .. } => AuditLevel::BestEffort,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AuditConfig {
    pub enabled: bool,
    pub log_dir: Option<std::path::PathBuf>,
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            log_dir: None,
        }
    }
}

impl AuditConfig {
    pub fn builder() -> AuditConfigBuilder {
        AuditConfigBuilder::new()
    }
}

pub struct AuditConfigBuilder {
    enabled: bool,
    log_dir: Option<std::path::PathBuf>,
}

impl AuditConfigBuilder {
    pub fn new() -> Self {
        Self {
            enabled: true,
            log_dir: None,
        }
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn log_dir(mut self, dir: std::path::PathBuf) -> Self {
        self.log_dir = Some(dir);
        self
    }

    pub fn build(self) -> AuditConfig {
        AuditConfig {
            enabled: self.enabled,
            log_dir: self.log_dir,
        }
    }
}

impl Default for AuditConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

pub struct AuditWriter {
    config: AuditConfig,
}

impl AuditWriter {
    pub fn new() -> Self {
        Self::with_config(AuditConfig::default())
    }

    pub fn builder() -> AuditWriterBuilder {
        AuditWriterBuilder::new()
    }

    pub fn with_config(config: AuditConfig) -> Self {
        Self { config }
    }

    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    pub fn write(&self, event: AuditEvent) -> ConfigResult<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let level = AuditLevel::for_event(&event);

        match level {
            AuditLevel::Durable => self.write_durable(&event),
            AuditLevel::BestEffort => self.write_best_effort(&event),
        }
    }

    fn write_durable(&self, event: &AuditEvent) -> ConfigResult<()> {
        // Durable events MUST be persisted; error if log_dir is not configured.
        let Some(ref dir) = self.config.log_dir else {
            return Err(ConfigError::InvalidValue {
                key: "audit.log_dir".into(),
                expected_type: "path".into(),
                message: "durable audit event requires log_dir to be configured".into(),
            });
        };
        let sanitized = self.sanitize(event);
        let filename = format!("audit_{}.log", Utc::now().format("%Y%m%d"));
        let path = dir.join(filename);
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .and_then(|mut file| {
                use std::io::Write;
                writeln!(file, "{} {:?}", sanitized.event_timestamp(), sanitized)
            })?;
        Ok(())
    }

    fn write_best_effort(&self, event: &AuditEvent) -> ConfigResult<()> {
        // Best-effort: attempt to persist if log_dir is configured.
        // If log_dir is not configured, silently drop the event.
        self.write_to_log(event)
    }

    /// Shared write path for both Durable and BestEffort events.
    /// Writes the sanitized event to `audit_YYYYMMDD.log` in `log_dir` if configured.
    /// Silently returns Ok if `log_dir` is None.
    fn write_to_log(&self, event: &AuditEvent) -> ConfigResult<()> {
        let Some(ref dir) = self.config.log_dir else {
            return Ok(());
        };
        let sanitized = self.sanitize(event);
        let filename = format!("audit_{}.log", Utc::now().format("%Y%m%d"));
        let path = dir.join(filename);
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .and_then(|mut file| {
                use std::io::Write;
                writeln!(file, "{} {:?}", sanitized.event_timestamp(), sanitized)
            })?;
        Ok(())
    }

    fn sanitize(&self, event: &AuditEvent) -> AuditEvent {
        // Extended list of sensitive field keywords for redaction
        const SENSITIVE_KEYWORDS: &[&str] = &[
            "password",
            "secret",
            "key",
            "token",
            "credential",
            "auth",
            "api_key",
            "apikey",
            "access_key",
            "private_key",
            "session_id",
            "sessionid",
            "bearer",
            "refresh_token",
            "client_secret",
            "encryption_key",
            "encrypt_key",
            "master_key",
            "service_account",
        ];

        let is_sensitive_field = |field: &str| {
            let lower = field.to_lowercase();
            SENSITIVE_KEYWORDS.iter().any(|kw| lower.contains(kw))
        };

        match event {
            AuditEvent::Decrypt {
                field,
                success,
                timestamp,
            } => {
                let sanitized_field = if is_sensitive_field(field) {
                    "***REDACTED***".to_string()
                } else {
                    field.clone()
                };
                AuditEvent::Decrypt {
                    field: sanitized_field,
                    success: *success,
                    timestamp: *timestamp,
                }
            }
            AuditEvent::KeyAccess {
                key,
                timestamp,
            } => {
                let sanitized_key = if is_sensitive_field(key) {
                    "***REDACTED***".to_string()
                } else {
                    key.clone()
                };
                AuditEvent::KeyAccess {
                    key: sanitized_key,
                    timestamp: *timestamp,
                }
            }
            other => other.clone(),
        }
    }

    pub fn log_load(&self, source: &str) -> ConfigResult<()> {
        self.write(AuditEvent::LoadSuccess {
            source: source.to_string(),
            timestamp: Utc::now(),
        })
    }

    pub fn log_key_access(&self, key: &str) -> ConfigResult<()> {
        self.write(AuditEvent::KeyAccess {
            key: key.to_string(),
            timestamp: Utc::now(),
        })
    }

    pub fn log_decrypt(&self, field: &str, success: bool) -> ConfigResult<()> {
        self.write(AuditEvent::Decrypt {
            field: field.to_string(),
            success,
            timestamp: Utc::now(),
        })
    }

    pub fn log_key_rotation(&self, old_ver: &str, new_ver: &str) -> ConfigResult<()> {
        self.write(AuditEvent::KeyRotation {
            old_version: old_ver.to_string(),
            new_version: new_ver.to_string(),
            timestamp: Utc::now(),
        })
    }
}

impl Default for AuditWriter {
    fn default() -> Self {
        Self::new()
    }
}

pub struct AuditWriterBuilder {
    config: AuditConfig,
}

impl AuditWriterBuilder {
    pub fn new() -> Self {
        Self {
            config: AuditConfig::default(),
        }
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.config.enabled = enabled;
        self
    }

    pub fn log_dir(mut self, dir: std::path::PathBuf) -> Self {
        self.config.log_dir = Some(dir);
        self
    }

    pub fn build(self) -> AuditWriter {
        AuditWriter::with_config(self.config)
    }
}

impl Default for AuditWriterBuilder {
    fn default() -> Self {
        Self::new()
    }
}
