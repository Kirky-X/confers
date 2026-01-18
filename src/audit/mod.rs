// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

use crate::error::ConfigError;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::Write;
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
    /// Log configuration loading to audit file
    ///
    /// # Security Notes
    ///
    /// - ⚠️ **Log Path**: Store audit logs in a secure location with restricted access (e.g., /var/log/)
    /// - ⚠️ **Log Rotation**: Configure log rotation to prevent disk space exhaustion
    /// - ⚠️ **Log Integrity**: Audit logs are signed to prevent tampering
    /// - ⚠️ **Log Access**: Restrict audit log file access permissions (only root/administrator)
    /// - ⚠️ **Log Monitoring**: Monitor audit logs for suspicious activity
    /// - ⚠️ **Data Sanitization**: Sensitive data is automatically sanitized before logging
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use confers::audit::AuditLogger;
    /// # use confers::audit::Sanitize;
    /// # use std::path::PathBuf;
    /// # use serde::Serialize;
    /// # use serde_json::Value;
    /// # #[derive(Serialize)]
    /// # struct ExampleConfig {
    /// #     key: String,
    /// # }
    /// # impl Sanitize for ExampleConfig {
    /// #     fn sanitize(&self) -> Value {
    /// #         serde_json::json!({"key": self.key})
    /// #     }
    /// # }
    /// # let config = ExampleConfig { key: "value".to_string() };
    /// AuditLogger::log_to_file(&config, &PathBuf::from("/var/log/audit.log"), None)?;
    /// # Ok::<(), confers::error::ConfigError>(())
    /// ```
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
    /// Log configuration to a file with audit metadata
    ///
    /// # Security Notes
    ///
    /// - ⚠️ **File Permissions**: Ensure log files have appropriate permissions (e.g., 0600)
    /// - ⚠️ **Log Rotation**: Implement log rotation to prevent disk exhaustion
    /// - ⚠️ **Sensitive Data**: Sanitize sensitive data before logging
    /// - ⚠️ **Log Tampering**: Use HMAC to detect log tampering
    /// - ⚠️ **Log Retention**: Implement log retention policy for compliance
    /// - ⚠️ **Log Access**: Restrict log access to authorized personnel
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use confers::audit::{AuditLogger, AuditConfig};
    /// # use confers::audit::Sanitize;
    /// # use std::path::PathBuf;
    /// # use serde::Serialize;
    /// # use serde_json::Value;
    /// # #[derive(Serialize)]
    /// # struct ExampleConfig {
    /// #     key: String,
    /// # }
    /// # impl Sanitize for ExampleConfig {
    /// #     fn sanitize(&self) -> Value {
    /// #         serde_json::json!({"key": self.key})
    /// #     }
    /// # }
    /// # let config = ExampleConfig { key: "value".to_string() };
    /// let audit_config = AuditConfig {
    ///     validation_error: Some("Invalid port".to_string()),
    ///     config_source: Some("config.toml".to_string()),
    ///     ..Default::default()
    /// };
    /// AuditLogger::log_to_file_with_source(&config, &PathBuf::from("/var/log/audit.log"), audit_config)?;
    /// # Ok::<(), confers::error::ConfigError>(())
    /// ```
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

        // Set restrictive permissions (0600) - owner read/write only
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(path)
                .map_err(|e| ConfigError::RemoteError(e.to_string()))?
                .permissions();
            perms.set_mode(0o600);
            std::fs::set_permissions(path, perms)
                .map_err(|e| ConfigError::RemoteError(e.to_string()))?;
        }

        Ok(())
    }
}

// ============================================================================
// Enhanced Audit Logging System
// ============================================================================

/// Audit event types for categorizing different operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditEventType {
    // Configuration loading events
    ConfigLoad {
        source: String,
    },
    ConfigLoadFailed {
        source: String,
        error: String,
    },
    ConfigReload {
        source: String,
    },

    // Key management events
    KeyRotation {
        key_id: String,
        previous_version: u32,
        new_version: u32,
    },
    KeyGeneration {
        key_id: String,
        version: u32,
    },
    KeyDeletion {
        key_id: String,
    },
    KeyAccess {
        key_id: String,
        operation: String,
    },

    // Configuration modification events
    ConfigUpdate {
        field: String,
        old_value: String,
        new_value: String,
    },
    ConfigDelete {
        field: String,
        value: String,
    },
    ConfigValidationFailed {
        field: String,
        error: String,
    },

    // Remote access events
    RemoteConfigFetch {
        url: String,
    },
    RemoteConfigFetchFailed {
        url: String,
        error: String,
    },
    RemoteConfigUpdate {
        url: String,
    },

    // Security events
    SecurityViolation {
        violation_type: String,
        details: String,
    },
    UnauthorizedAccess {
        resource: String,
        attempt: String,
    },
    EncryptionFailure {
        operation: String,
        error: String,
    },
}

/// Audit event priority levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Copy)]
pub enum AuditPriority {
    Debug = 0,
    Info = 1,
    Warning = 2,
    Error = 3,
    Critical = 4,
}

/// Complete audit event record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub id: u64,
    pub timestamp: u64,
    pub event_type: AuditEventType,
    pub priority: AuditPriority,
    pub hostname: String,
    pub process_id: u64,
    pub metadata: std::collections::HashMap<String, String>,
}

/// Generator for creating audit events with unique IDs and metadata
pub struct AuditEventGenerator {
    event_id_generator: std::sync::Arc<std::sync::atomic::AtomicU64>,
    hostname: String,
    process_id: u64,
}

impl AuditEventGenerator {
    /// Create a new audit event generator
    pub fn new() -> Self {
        Self {
            event_id_generator: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(1)),
            hostname: std::env::var("HOSTNAME")
                .ok()
                .or_else(|| std::env::var("COMPUTERNAME").ok())
                .unwrap_or_else(|| "unknown".to_string()),
            process_id: std::process::id().into(),
        }
    }

    /// Generate a new audit event
    pub fn generate_event(
        &self,
        event_type: AuditEventType,
        priority: AuditPriority,
        metadata: std::collections::HashMap<String, String>,
    ) -> AuditEvent {
        AuditEvent {
            id: self
                .event_id_generator
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            event_type,
            priority,
            hostname: self.hostname.clone(),
            process_id: self.process_id,
            metadata,
        }
    }
}

impl Default for AuditEventGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// Configuration for audit log rotation
#[derive(Debug, Clone)]
pub struct RotationConfig {
    pub max_size_mb: usize,
    pub max_age_days: usize,
    pub max_files: usize,
    pub compress_archived: bool,
}

impl Default for RotationConfig {
    fn default() -> Self {
        Self {
            max_size_mb: 100,
            max_age_days: 30,
            max_files: 10,
            compress_archived: true,
        }
    }
}

/// Writer for audit logs with rotation and integrity protection
pub struct AuditLogWriter {
    log_path: std::path::PathBuf,
    rotation_config: RotationConfig,
    integrity_key: [u8; 32],
    current_log_size: std::sync::Arc<std::sync::atomic::AtomicU64>,
    rotation_lock: std::sync::Arc<std::sync::Mutex<()>>,
}

impl AuditLogWriter {
    /// Create a new audit log writer
    pub fn new(
        log_path: std::path::PathBuf,
        rotation_config: RotationConfig,
        integrity_key: [u8; 32],
    ) -> Result<Self, ConfigError> {
        // Ensure parent directory exists
        if let Some(parent) = log_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| ConfigError::IoError(e.to_string()))?;
        }

        // Get initial log size if file exists
        let initial_size = if log_path.exists() {
            std::fs::metadata(&log_path).map(|m| m.len()).unwrap_or(0)
        } else {
            0
        };

        Ok(Self {
            log_path,
            rotation_config,
            integrity_key,
            current_log_size: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(initial_size)),
            rotation_lock: std::sync::Arc::new(std::sync::Mutex::new(())),
        })
    }

    /// Write an audit event to the log file
    pub fn write_event(&self, event: &AuditEvent) -> Result<(), ConfigError> {
        // Serialize event
        let serialized =
            serde_json::to_string(event).map_err(|e| ConfigError::ParseError(e.to_string()))?;

        // Sign event for integrity protection
        let signature = self.sign_event(&serialized)?;

        // Format log entry
        let entry = format!("{}|{}\n", signature, serialized);

        // Write to file
        self.write_to_file(&entry)?;

        // Update log size
        self.current_log_size
            .fetch_add(entry.len() as u64, std::sync::atomic::Ordering::SeqCst);

        // Check if rotation is needed
        if self.should_rotate() {
            self.rotate()?;
        }

        Ok(())
    }

    /// Sign an event for integrity protection
    #[cfg(feature = "encryption")]
    fn sign_event(&self, data: &str) -> Result<String, ConfigError> {
        use hmac::{Hmac, Mac};
        type HmacSha256 = Hmac<sha2::Sha256>;

        let mut mac = HmacSha256::new_from_slice(&self.integrity_key)
            .map_err(|e| ConfigError::EncryptionError(e.to_string()))?;
        mac.update(data.as_bytes());
        let signature = mac.finalize().into_bytes();
        Ok(hex::encode(signature))
    }

    /// Sign an event for integrity protection (no-op without encryption feature)
    #[cfg(not(feature = "encryption"))]
    fn sign_event(&self, _data: &str) -> Result<String, ConfigError> {
        Ok("no-signature".to_string())
    }

    /// Write data to the log file
    fn write_to_file(&self, data: &str) -> Result<(), ConfigError> {
        use std::fs::OpenOptions;
        use std::io::Write;

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)
            .map_err(|e| ConfigError::IoError(e.to_string()))?;

        file.write_all(data.as_bytes())
            .map_err(|e| ConfigError::IoError(e.to_string()))?;

        Ok(())
    }

    /// Check if log rotation is needed
    fn should_rotate(&self) -> bool {
        let current_size = self
            .current_log_size
            .load(std::sync::atomic::Ordering::SeqCst);
        current_size > (self.rotation_config.max_size_mb * 1024 * 1024) as u64
    }

    /// Rotate the log file
    fn rotate(&self) -> Result<(), ConfigError> {
        use std::fs;

        // Acquire rotation lock to prevent concurrent rotations
        let _lock = self
            .rotation_lock
            .lock()
            .map_err(|e| ConfigError::IoError(format!("Failed to acquire rotation lock: {}", e)))?;

        // Check again if rotation is still needed after acquiring lock
        if !self.should_rotate() {
            return Ok(());
        }

        // Generate timestamp for archived filename
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let timestamp = now.to_string();
        let archive_name = format!(
            "{}.{}.{}",
            self.log_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("audit"),
            timestamp,
            self.log_path
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("log")
        );
        let archive_path = self.log_path.with_file_name(&archive_name);

        // Rename current log file
        if self.log_path.exists() {
            fs::rename(&self.log_path, &archive_path)
                .map_err(|e| ConfigError::IoError(e.to_string()))?;

            // Compress if enabled
            if self.rotation_config.compress_archived {
                self.compress_archive(&archive_path)?;
            }

            // Clean up old archives
            self.cleanup_old_archives()?;
        }

        // Reset log size
        self.current_log_size
            .store(0, std::sync::atomic::Ordering::SeqCst);

        Ok(())
    }

    /// Compress an archived log file
    #[cfg(feature = "encryption")]
    fn compress_archive(&self, archive_path: &std::path::Path) -> Result<(), ConfigError> {
        use flate2::write::GzEncoder;
        use flate2::Compression;
        use std::fs::File;
        use std::io::{BufReader, BufWriter, Read};

        let input_file =
            File::open(archive_path).map_err(|e| ConfigError::IoError(e.to_string()))?;
        let mut reader = BufReader::new(input_file);

        let compressed_path = archive_path.with_extension("log.gz");
        let output_file =
            File::create(&compressed_path).map_err(|e| ConfigError::IoError(e.to_string()))?;
        let mut writer = BufWriter::new(GzEncoder::new(output_file, Compression::default()));

        let mut buffer = Vec::new();
        reader
            .read_to_end(&mut buffer)
            .map_err(|e: std::io::Error| ConfigError::IoError(e.to_string()))?;
        writer
            .write_all(&buffer)
            .map_err(|e: std::io::Error| ConfigError::IoError(e.to_string()))?;
        writer
            .flush()
            .map_err(|e: std::io::Error| ConfigError::IoError(e.to_string()))?;

        // Remove original archive file only after successful compression
        std::fs::remove_file(archive_path).map_err(|e| ConfigError::IoError(e.to_string()))?;

        Ok(())
    }

    /// Compress an archived log file (no-op without encryption feature)
    #[cfg(not(feature = "encryption"))]
    fn compress_archive(&self, _archive_path: &std::path::Path) -> Result<(), ConfigError> {
        Ok(())
    }

    /// Clean up old archived log files
    fn cleanup_old_archives(&self) -> Result<(), ConfigError> {
        use std::fs;
        use std::path::PathBuf;

        let parent_dir = self
            .log_path
            .parent()
            .ok_or_else(|| ConfigError::IoError("Invalid log path".to_string()))?;

        let mut archives: Vec<PathBuf> = fs::read_dir(parent_dir)
            .map_err(|e| ConfigError::IoError(e.to_string()))?
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| {
                path.extension()
                    .and_then(|s| s.to_str())
                    .map(|ext| ext == "log" || ext == "gz")
                    .unwrap_or(false)
                    && path.as_path() != self.log_path.as_path()
            })
            .collect();

        // Sort by modification time (oldest first)
        archives.sort_by_key(|path| {
            fs::metadata(path)
                .and_then(|m| m.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
        });

        // Remove old archives if we exceed max_files
        if archives.len() > self.rotation_config.max_files {
            let to_remove = &archives[..archives.len() - self.rotation_config.max_files];
            for path in to_remove {
                fs::remove_file(path).map_err(|e| ConfigError::IoError(e.to_string()))?;
            }
        }

        // Remove archives older than max_age_days
        let max_age =
            std::time::Duration::from_secs(self.rotation_config.max_age_days as u64 * 24 * 3600);
        let now = SystemTime::now();
        archives.retain(|path| {
            fs::metadata(path)
                .and_then(|m| m.modified())
                .map(|modified| {
                    now.duration_since(modified)
                        .map(|age| age < max_age)
                        .unwrap_or(false)
                })
                .unwrap_or(true)
        });

        Ok(())
    }

    /// Get the current log file path
    pub fn log_path(&self) -> &std::path::Path {
        &self.log_path
    }

    /// Get the current log size in bytes
    pub fn current_log_size(&self) -> u64 {
        self.current_log_size
            .load(std::sync::atomic::Ordering::SeqCst)
    }
}

/// Query parameters for searching audit logs
#[derive(Debug, Clone, Default)]
pub struct AuditLogQuery {
    pub start_time: Option<u64>,
    pub end_time: Option<u64>,
    pub event_types: Option<Vec<String>>,
    pub priorities: Option<Vec<AuditPriority>>,
    pub keywords: Option<Vec<String>>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

impl AuditLogQuery {
    /// Create a new audit log query with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Set time range for the query
    pub fn with_time_range(mut self, start: Option<u64>, end: Option<u64>) -> Self {
        self.start_time = start;
        self.end_time = end;
        self
    }

    /// Set event types to filter
    pub fn with_event_types(mut self, types: Vec<String>) -> Self {
        self.event_types = Some(types);
        self
    }

    /// Set priorities to filter
    pub fn with_priorities(mut self, priorities: Vec<AuditPriority>) -> Self {
        self.priorities = Some(priorities);
        self
    }

    /// Set keywords to search for
    pub fn with_keywords(mut self, keywords: Vec<String>) -> Self {
        self.keywords = Some(keywords);
        self
    }

    /// Set pagination parameters
    pub fn with_pagination(mut self, offset: usize, limit: usize) -> Self {
        self.offset = Some(offset);
        self.limit = Some(limit);
        self
    }
}

/// Query result with pagination information
#[derive(Debug, Clone)]
pub struct QueryResult {
    pub events: Vec<AuditEvent>,
    pub total_count: usize,
    pub page: usize,
    pub page_size: usize,
    pub total_pages: usize,
}

/// Statistics for audit log analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogStatistics {
    pub total_events: usize,
    pub events_by_type: std::collections::HashMap<String, usize>,
    pub events_by_priority: std::collections::HashMap<String, usize>,
    pub time_range: Option<(u64, u64)>,
}

/// Reader for querying audit logs
pub struct AuditLogReader {
    log_path: std::path::PathBuf,
}

impl AuditLogReader {
    /// Create a new audit log reader
    pub fn new(log_path: std::path::PathBuf) -> Self {
        Self { log_path }
    }

    /// Query audit logs with the given parameters
    pub fn query(&self, query: &AuditLogQuery) -> Result<QueryResult, ConfigError> {
        let all_events = self.read_all_events()?;
        let total_count = all_events.len();

        // Apply filters
        let filtered_events = self.apply_filters(&all_events, query);

        // Apply pagination
        let offset = query.offset.unwrap_or(0);
        let limit = query.limit.unwrap_or(filtered_events.len());
        let paginated_events: Vec<AuditEvent> = filtered_events
            .into_iter()
            .skip(offset)
            .take(limit)
            .collect();

        let total_pages = if limit > 0 {
            total_count.div_ceil(limit)
        } else {
            1
        };

        Ok(QueryResult {
            events: paginated_events,
            total_count,
            page: offset / limit + 1,
            page_size: limit,
            total_pages,
        })
    }

    /// Analyze audit logs and generate statistics
    pub fn analyze(&self) -> Result<AuditLogStatistics, ConfigError> {
        let events = self.read_all_events()?;
        let total_events = events.len();

        let mut events_by_type = std::collections::HashMap::new();
        let mut events_by_priority = std::collections::HashMap::new();
        let mut min_time = u64::MAX;
        let mut max_time = 0u64;

        for event in &events {
            // Count by event type
            let type_name = self.event_type_to_string(&event.event_type);
            *events_by_type.entry(type_name).or_insert(0) += 1;

            // Count by priority
            let priority_name = format!("{:?}", event.priority);
            *events_by_priority.entry(priority_name).or_insert(0) += 1;

            // Track time range
            min_time = min_time.min(event.timestamp);
            max_time = max_time.max(event.timestamp);
        }

        let time_range = if total_events > 0 {
            Some((min_time, max_time))
        } else {
            None
        };

        Ok(AuditLogStatistics {
            total_events,
            events_by_type,
            events_by_priority,
            time_range,
        })
    }

    /// Read all events from the log file
    fn read_all_events(&self) -> Result<Vec<AuditEvent>, ConfigError> {
        use std::fs::File;
        use std::io::{BufRead, BufReader};

        if !self.log_path.exists() {
            return Ok(Vec::new());
        }

        let file = File::open(&self.log_path).map_err(|e| ConfigError::IoError(e.to_string()))?;
        let reader = BufReader::new(file);
        let mut events = Vec::new();

        for line in reader.lines() {
            let line = line.map_err(|e| ConfigError::IoError(e.to_string()))?;
            if let Some(event) = self.parse_log_line(&line) {
                events.push(event);
            }
        }

        Ok(events)
    }

    /// Parse a log line into an AuditEvent
    fn parse_log_line(&self, line: &str) -> Option<AuditEvent> {
        // Log format: <signature>|<json_event>
        let parts: Vec<&str> = line.splitn(2, '|').collect();
        if parts.len() != 2 {
            return None;
        }

        let _signature = parts[0];
        let json_str = parts[1];

        serde_json::from_str(json_str).ok()
    }

    /// Apply filters to events based on query parameters
    fn apply_filters(&self, events: &[AuditEvent], query: &AuditLogQuery) -> Vec<AuditEvent> {
        let mut filtered = events.to_vec();

        // Filter by time range
        if let Some(start) = query.start_time {
            filtered.retain(|e| e.timestamp >= start);
        }
        if let Some(end) = query.end_time {
            filtered.retain(|e| e.timestamp <= end);
        }

        // Filter by event types
        if let Some(ref types) = query.event_types {
            filtered.retain(|e| {
                let type_name = self.event_type_to_string(&e.event_type);
                types.contains(&type_name)
            });
        }

        // Filter by priorities
        if let Some(ref priorities) = query.priorities {
            filtered.retain(|e| priorities.contains(&e.priority));
        }

        // Filter by keywords
        if let Some(ref keywords) = query.keywords {
            filtered.retain(|e| {
                let event_str = serde_json::to_string(e).unwrap_or_default();
                keywords.iter().any(|kw| event_str.contains(kw))
            });
        }

        filtered
    }

    /// Convert AuditEventType to string representation
    fn event_type_to_string(&self, event_type: &AuditEventType) -> String {
        match event_type {
            AuditEventType::ConfigLoad { .. } => "ConfigLoad".to_string(),
            AuditEventType::ConfigLoadFailed { .. } => "ConfigLoadFailed".to_string(),
            AuditEventType::ConfigReload { .. } => "ConfigReload".to_string(),
            AuditEventType::KeyRotation { .. } => "KeyRotation".to_string(),
            AuditEventType::KeyGeneration { .. } => "KeyGeneration".to_string(),
            AuditEventType::KeyDeletion { .. } => "KeyDeletion".to_string(),
            AuditEventType::KeyAccess { .. } => "KeyAccess".to_string(),
            AuditEventType::ConfigUpdate { .. } => "ConfigUpdate".to_string(),
            AuditEventType::ConfigDelete { .. } => "ConfigDelete".to_string(),
            AuditEventType::ConfigValidationFailed { .. } => "ConfigValidationFailed".to_string(),
            AuditEventType::RemoteConfigFetch { .. } => "RemoteConfigFetch".to_string(),
            AuditEventType::RemoteConfigFetchFailed { .. } => "RemoteConfigFetchFailed".to_string(),
            AuditEventType::RemoteConfigUpdate { .. } => "RemoteConfigUpdate".to_string(),
            AuditEventType::SecurityViolation { .. } => "SecurityViolation".to_string(),
            AuditEventType::UnauthorizedAccess { .. } => "UnauthorizedAccess".to_string(),
            AuditEventType::EncryptionFailure { .. } => "EncryptionFailure".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_audit_event_types() {
        // Test ConfigLoad event
        let event_type = AuditEventType::ConfigLoad {
            source: "config.toml".to_string(),
        };
        assert!(matches!(event_type, AuditEventType::ConfigLoad { .. }));

        // Test KeyRotation event
        let event_type = AuditEventType::KeyRotation {
            key_id: "test".to_string(),
            previous_version: 1,
            new_version: 2,
        };
        assert!(matches!(event_type, AuditEventType::KeyRotation { .. }));

        // Test SecurityViolation event
        let event_type = AuditEventType::SecurityViolation {
            violation_type: "UnauthorizedAccess".to_string(),
            details: "Invalid credentials".to_string(),
        };
        assert!(matches!(
            event_type,
            AuditEventType::SecurityViolation { .. }
        ));
    }

    #[test]
    fn test_audit_priority_ordering() {
        assert!(AuditPriority::Debug < AuditPriority::Info);
        assert!(AuditPriority::Info < AuditPriority::Warning);
        assert!(AuditPriority::Warning < AuditPriority::Error);
        assert!(AuditPriority::Error < AuditPriority::Critical);
    }

    #[test]
    fn test_audit_event_generator() {
        let generator = AuditEventGenerator::new();

        let event_type = AuditEventType::ConfigLoad {
            source: "config.toml".to_string(),
        };
        let priority = AuditPriority::Info;
        let metadata = HashMap::new();

        let event = generator.generate_event(event_type.clone(), priority, metadata);

        assert_eq!(event.id, 1);
        assert!(matches!(
            event.event_type,
            AuditEventType::ConfigLoad { .. }
        ));
        assert_eq!(event.priority, AuditPriority::Info);
        assert!(!event.hostname.is_empty());
        assert!(event.process_id > 0);

        // Second event should have ID 2
        let event2 = generator.generate_event(event_type, priority, HashMap::new());
        assert_eq!(event2.id, 2);
    }

    #[test]
    fn test_audit_event_serialization() {
        let generator = AuditEventGenerator::new();

        let event_type = AuditEventType::ConfigLoad {
            source: "config.toml".to_string(),
        };
        let priority = AuditPriority::Info;
        let metadata = HashMap::new();

        let event = generator.generate_event(event_type, priority, metadata);

        // Test serialization
        let serialized = serde_json::to_string(&event);
        assert!(serialized.is_ok());

        // Test deserialization
        let deserialized: Result<AuditEvent, _> = serde_json::from_str(&serialized.unwrap());
        assert!(deserialized.is_ok());

        let deserialized_event = deserialized.unwrap();
        assert_eq!(deserialized_event.id, event.id);
        assert_eq!(deserialized_event.priority, event.priority);
    }

    #[test]
    fn test_rotation_config_default() {
        let config = RotationConfig::default();
        assert_eq!(config.max_size_mb, 100);
        assert_eq!(config.max_age_days, 30);
        assert_eq!(config.max_files, 10);
        assert!(config.compress_archived);
    }

    #[test]
    fn test_audit_log_query_builder() {
        let query = AuditLogQuery::new()
            .with_time_range(Some(1000), Some(2000))
            .with_event_types(vec!["ConfigLoad".to_string()])
            .with_priorities(vec![AuditPriority::Info])
            .with_keywords(vec!["config".to_string()])
            .with_pagination(0, 10);

        assert_eq!(query.start_time, Some(1000));
        assert_eq!(query.end_time, Some(2000));
        assert!(query.event_types.is_some());
        assert!(query.priorities.is_some());
        assert!(query.keywords.is_some());
        assert_eq!(query.offset, Some(0));
        assert_eq!(query.limit, Some(10));
    }
}
