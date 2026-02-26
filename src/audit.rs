#![cfg(feature = "audit")]

use chrono::{DateTime, Utc};

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
    pub durable_wal: bool,
    pub channel_size: usize,
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            log_dir: None,
            durable_wal: false,
            channel_size: 1024,
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
    durable_wal: bool,
    channel_size: usize,
}

impl AuditConfigBuilder {
    pub fn new() -> Self {
        Self {
            enabled: true,
            log_dir: None,
            durable_wal: false,
            channel_size: 1024,
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

    pub fn durable_wal(mut self, enabled: bool) -> Self {
        self.durable_wal = enabled;
        self
    }

    pub fn channel_size(mut self, size: usize) -> Self {
        self.channel_size = size;
        self
    }

    pub fn build(self) -> AuditConfig {
        AuditConfig {
            enabled: self.enabled,
            log_dir: self.log_dir,
            durable_wal: self.durable_wal,
            channel_size: self.channel_size,
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

    pub fn write(&self, event: AuditEvent) {
        if !self.config.enabled {
            return;
        }

        let level = AuditLevel::for_event(&event);

        match level {
            AuditLevel::Durable => self.write_durable(&event),
            AuditLevel::BestEffort => self.write_best_effort(&event),
        }
    }

    fn write_durable(&self, event: &AuditEvent) {
        if let Some(ref dir) = self.config.log_dir {
            let sanitized = self.sanitize(event);
            let filename = format!("audit_{}.log", Utc::now().format("%Y%m%d"));
            let path = dir.join(filename);
            if let Ok(mut file) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
            {
                use std::io::Write;
                let _ = writeln!(file, "{} {:?}", Utc::now(), sanitized);
            }
        }
    }

    fn write_best_effort(&self, event: &AuditEvent) {
        let sanitized = self.sanitize(event);
        tracing::debug!("audit: {:?}", sanitized);
    }

    fn sanitize(&self, event: &AuditEvent) -> AuditEvent {
        // Extended list of sensitive field keywords for redaction
        const SENSITIVE_KEYWORDS: &[&str] = &[
            "password", "secret", "key", "token", "credential", "auth",
            "api_key", "apikey", "access_key", "access_key", "private_key",
            "session_id", "sessionid", "bearer", "refresh_token", "client_secret",
            "encryption_key", "encrypt_key", "master_key", "service_account",
        ];

        match event {
            AuditEvent::Decrypt {
                field,
                success,
                timestamp,
            } => {
                let lower_field = field.to_lowercase();
                let sanitized_field = if SENSITIVE_KEYWORDS.iter().any(|kw| lower_field.contains(kw)) {
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
            other => other.clone(),
        }
    }

    pub fn log_load(&self, source: &str) {
        self.write(AuditEvent::LoadSuccess {
            source: source.to_string(),
            timestamp: Utc::now(),
        });
    }

    pub fn log_key_access(&self, key: &str) {
        self.write(AuditEvent::KeyAccess {
            key: key.to_string(),
            timestamp: Utc::now(),
        });
    }

    pub fn log_decrypt(&self, field: &str, success: bool) {
        self.write(AuditEvent::Decrypt {
            field: field.to_string(),
            success,
            timestamp: Utc::now(),
        });
    }

    pub fn log_key_rotation(&self, old_ver: &str, new_ver: &str) {
        self.write(AuditEvent::KeyRotation {
            old_version: old_ver.to_string(),
            new_version: new_ver.to_string(),
            timestamp: Utc::now(),
        });
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

    pub fn durable_wal(mut self, enabled: bool) -> Self {
        self.config.durable_wal = enabled;
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
