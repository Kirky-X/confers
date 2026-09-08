// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

use chrono::{DateTime, Utc};

use crate::error::{ConfigError, ConfigResult};

#[derive(Debug, Clone, serde::Serialize)]
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

/// Configuration for audit logging.
///
/// Auditing is **disabled by default**: it must be explicitly enabled and
/// given a `log_dir`. Durable events (key access, key rotation, decryption)
/// return an error when `log_dir` is not configured, so a default of
/// `enabled: true` with no `log_dir` made every durable audit call fail out
/// of the box. Keep auditing off until it is deliberately set up.
#[derive(Debug, Clone, Default)]
pub struct AuditConfig {
    /// Whether audit logging is enabled. Defaults to `false`.
    pub enabled: bool,
    /// Directory the audit log files are written to. Required for durable
    /// events once auditing is enabled.
    pub log_dir: Option<std::path::PathBuf>,
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
        // Mirrors `AuditConfig::default()`: auditing stays off until it is
        // explicitly enabled (and given a log_dir).
        Self {
            enabled: false,
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
    /// Serializes writes so concurrent events never interleave in the log file.
    write_lock: std::sync::Mutex<()>,
}

impl AuditWriter {
    pub fn new() -> Self {
        Self::with_config(AuditConfig::default())
    }

    pub fn builder() -> AuditWriterBuilder {
        AuditWriterBuilder::new()
    }

    pub fn with_config(config: AuditConfig) -> Self {
        Self {
            config,
            write_lock: std::sync::Mutex::new(()),
        }
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
        self.append_event(event, dir)
    }

    fn write_best_effort(&self, event: &AuditEvent) -> ConfigResult<()> {
        // Best-effort: attempt to persist if log_dir is configured.
        // If log_dir is not configured, silently drop the event.
        let Some(ref dir) = self.config.log_dir else {
            return Ok(());
        };
        self.append_event(event, dir)
    }

    /// Shared append path for both Durable and BestEffort events.
    ///
    /// Writes the sanitized event to `audit_YYYYMMDD.log` in `dir`. The file
    /// name date is derived from the event's own timestamp — captured once
    /// per `log_*` call — so the embedded timestamp and the file name can
    /// never disagree across midnight.
    fn append_event(&self, event: &AuditEvent, dir: &std::path::Path) -> ConfigResult<()> {
        let filename = format!("audit_{}.log", event.event_timestamp().format("%Y%m%d"));
        let path = dir.join(filename);
        let line = serde_json::to_string(&self.sanitize(event)).map_err(|e| {
            ConfigError::InvalidValue {
                key: "audit.event".into(),
                expected_type: "serializable audit event".into(),
                message: e.to_string(),
            }
        })?;
        let _guard = self
            .write_lock
            .lock()
            .map_err(|_| ConfigError::LockPoisoned {
                resource: "audit.writer".into(),
            })?;
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .and_then(|mut file| {
                use std::io::Write;
                file.write_all(line.as_bytes())?;
                file.write_all(b"\n")
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
            AuditEvent::KeyAccess { key, timestamp } => {
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

#[cfg(all(test, feature = "audit"))]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_is_disabled() {
        // Regression: the default used to be `enabled: true` with no
        // log_dir, so every Durable event failed out of the box. Auditing
        // must now be explicitly enabled and configured.
        let config = AuditConfig::default();
        assert!(!config.enabled, "audit must be disabled by default");
        assert!(config.log_dir.is_none(), "log_dir starts unset");

        let writer = AuditWriter::new();
        assert!(!writer.is_enabled(), "default writer must be disabled");

        let built = AuditConfig::builder().build();
        assert_eq!(built.enabled, config.enabled);
        assert_eq!(built.log_dir, config.log_dir);
    }

    #[test]
    fn test_disabled_writer_is_silent_without_log_dir() {
        // With the default (disabled) config, log_* calls are silent no-ops
        // even though log_dir is not configured — a Durable event must NOT
        // error.
        let writer = AuditWriter::new();
        writer.log_load("source").unwrap();
        writer.log_key_access("some.key").unwrap();
        writer.log_decrypt("some.field", true).unwrap();
        writer.log_key_rotation("v1", "v2").unwrap();
    }

    #[test]
    fn test_log_file_date_matches_event_timestamp() {
        // Regression: the file name date must come from the event's single
        // timestamp, not from a second Utc::now() taken at write time (the
        // two could straddle midnight).
        let dir = tempfile::tempdir().unwrap();
        let writer = AuditWriter::builder()
            .enabled(true)
            .log_dir(dir.path().to_path_buf())
            .build();

        let timestamp = Utc::now() - chrono::Duration::days(3);
        writer
            .write(AuditEvent::KeyAccess {
                key: "test.key".to_string(),
                timestamp,
            })
            .unwrap();

        let expected = format!("audit_{}.log", timestamp.format("%Y%m%d"));
        assert!(
            dir.path().join(&expected).exists(),
            "expected audit file {} named after the event timestamp",
            expected
        );
    }
}
