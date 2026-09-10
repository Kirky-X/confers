// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

use chrono::{DateTime, Utc};

use crate::error::{ConfigError, ConfigResult};

/// Length of the random per-file chain salt in bytes. The salt doubles as the
/// HMAC key material and as the genesis `prev_hash` of the chain.
const CHAIN_SALT_LEN: usize = 32;

/// Length of an HMAC-SHA256 output (one chain hash) in bytes.
const CHAIN_HASH_LEN: usize = 32;

/// Top-level `record` field value marking the chain header line.
const CHAIN_HEADER_RECORD: &str = "chain_header";

/// Top-level `record` field value marking the chain footer line.
const CHAIN_FOOTER_RECORD: &str = "chain_footer";

/// HMAC-SHA256 over `prev_hash || canonical_event_bytes`.
fn hmac_sha256(key: &[u8], prev_hash: &[u8], canonical_event: &[u8]) -> [u8; CHAIN_HASH_LEN] {
    use hmac::digest::KeyInit;

    type HmacSha256 = hmac::Hmac<sha2::Sha256>;
    let mut mac = <HmacSha256 as KeyInit>::new_from_slice(key)
        .expect("HMAC accepts keys of any length");
    hmac::Mac::update(&mut mac, prev_hash);
    hmac::Mac::update(&mut mac, canonical_event);
    let out = hmac::Mac::finalize(mac).into_bytes();
    let mut bytes = [0u8; CHAIN_HASH_LEN];
    bytes.copy_from_slice(&out);
    bytes
}

/// Draw a random chain salt from the OS RNG.
fn random_chain_salt() -> ConfigResult<[u8; CHAIN_SALT_LEN]> {
    let mut salt = [0u8; CHAIN_SALT_LEN];
    getrandom::fill(&mut salt).map_err(|e| {
        ConfigError::IoError(std::io::Error::other(format!("chain salt RNG failed: {e}")))
    })?;
    Ok(salt)
}

/// Canonical bytes of an event: the sorted-key JSON serialization of the
/// (already sanitized) event value. Both the writer and
/// [`verify_audit_chain`](crate::audit::verify_audit_chain) derive the same
/// bytes through `serde_json::Value`, which serializes deterministically.
fn canonical_event_bytes(event_value: &serde_json::Value) -> ConfigResult<Vec<u8>> {
    serde_json::to_vec(event_value).map_err(|e| ConfigError::InvalidValue {
        key: "audit.event".into(),
        expected_type: "serializable audit event".into(),
        message: e.to_string(),
    })
}

/// Build the persisted line value: the event's fields at the top level with
/// the chain metadata (`prev_hash`, `hmac`) alongside them.
fn chain_event_line(
    event_value: serde_json::Value,
    prev_hash_hex: &str,
    hmac_hex: &str,
) -> ConfigResult<serde_json::Value> {
    let mut obj = match event_value {
        serde_json::Value::Object(map) => map,
        other => {
            return Err(ConfigError::InvalidValue {
                key: "audit.event".into(),
                expected_type: "object-shaped audit event".into(),
                message: format!("unexpected non-object audit event payload: {other}"),
            })
        }
    };
    obj.insert(
        "prev_hash".to_string(),
        serde_json::Value::String(prev_hash_hex.to_string()),
    );
    obj.insert(
        "hmac".to_string(),
        serde_json::Value::String(hmac_hex.to_string()),
    );
    Ok(serde_json::Value::Object(obj))
}

/// Per-file HMAC chain state: the file's salt and the hash of the last
/// appended event (the salt itself for a fresh chain).
struct ChainState {
    salt: [u8; CHAIN_SALT_LEN],
    prev_hash: [u8; CHAIN_HASH_LEN],
}

/// Scan an existing audit file for its chain header salt and the last event
/// hash, so a reopened file continues the same chain instead of forking it.
/// Returns `None` when the file has no recognizable chain (yet).
fn scan_chain_state(path: &std::path::Path) -> Option<ChainState> {
    let content = std::fs::read_to_string(path).ok()?;
    let mut salt: Option<[u8; CHAIN_SALT_LEN]> = None;
    let mut last_hash: Option<[u8; CHAIN_HASH_LEN]> = None;

    for line in content.lines().map(str::trim).filter(|l| !l.is_empty()) {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        let Some(obj) = value.as_object() else {
            continue;
        };
        match obj.get("record").and_then(|r| r.as_str()) {
            Some(CHAIN_HEADER_RECORD) => {
                let salt_hex = obj.get("salt").and_then(|s| s.as_str())?;
                let mut decoded = [0u8; CHAIN_SALT_LEN];
                hex::decode_to_slice(salt_hex, &mut decoded).ok()?;
                salt = Some(decoded);
            }
            Some(_) => {}
            None => {
                if let Some(hash_hex) = obj.get("hmac").and_then(|h| h.as_str()) {
                    let mut decoded = [0u8; CHAIN_HASH_LEN];
                    if hex::decode_to_slice(hash_hex, &mut decoded).is_ok() {
                        last_hash = Some(decoded);
                    }
                }
            }
        }
    }

    Some(ChainState {
        salt: salt?,
        prev_hash: last_hash.unwrap_or(salt?),
    })
}

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
    /// HMAC chain state per audit file this writer has extended. Restored from
    /// the file header when a same-day file is reopened by a new writer.
    chain_states: std::sync::Mutex<std::collections::HashMap<std::path::PathBuf, ChainState>>,
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
            chain_states: std::sync::Mutex::new(std::collections::HashMap::new()),
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
    /// Writes the sanitized event to `audit_YYYYMMDD.log` in `dir` as one line
    /// of a keyed hash chain: before the event is persisted the writer
    /// computes `HMAC-SHA256(prev_hash || canonical_event_bytes)` and embeds
    /// both `prev_hash` and the resulting `hmac` alongside the event. The
    /// file's chain starts from a random salt stored in a `chain_header` line
    /// (written when the file is first created), and a `chain_footer` line is
    /// appended when the writer is dropped. Any tampering, deletion, or
    /// reordering breaks the chain and is caught by
    /// [`verify_audit_chain`](crate::audit::verify_audit_chain).
    ///
    /// The file name date is derived from the event's own timestamp — captured
    /// once per `log_*` call — so the embedded timestamp and the file name can
    /// never disagree across midnight.
    fn append_event(&self, event: &AuditEvent, dir: &std::path::Path) -> ConfigResult<()> {
        let filename = format!("audit_{}.log", event.event_timestamp().format("%Y%m%d"));
        let path = dir.join(filename);
        let sanitized = self.sanitize(event);
        let event_value = serde_json::to_value(&sanitized).map_err(|e| {
            ConfigError::InvalidValue {
                key: "audit.event".into(),
                expected_type: "serializable audit event".into(),
                message: e.to_string(),
            }
        })?;
        let canonical = canonical_event_bytes(&event_value)?;

        let _guard = self
            .write_lock
            .lock()
            .map_err(|_| ConfigError::LockPoisoned {
                resource: "audit.writer".into(),
            })?;
        let mut states = self
            .chain_states
            .lock()
            .map_err(|_| ConfigError::LockPoisoned {
                resource: "audit.chain_states".into(),
            })?;

        let state = match states.get_mut(&path) {
            Some(state) => state,
            None => {
                let restored = if path.exists() {
                    scan_chain_state(&path)
                } else {
                    None
                };
                let state = match restored {
                    Some(state) => state,
                    None => self.init_chain(&path)?,
                };
                states.insert(path.clone(), state);
                states
                    .get_mut(&path)
                    .expect("chain state was just inserted")
            }
        };

        let entry_hash = hmac_sha256(&state.salt, &state.prev_hash, &canonical);
        let line = chain_event_line(event_value, &hex::encode(state.prev_hash), &hex::encode(
            entry_hash,
        ))?;
        let mut line_str = serde_json::to_string(&line).map_err(|e| {
            ConfigError::InvalidValue {
                key: "audit.event".into(),
                expected_type: "serializable audit event".into(),
                message: e.to_string(),
            }
        })?;
        line_str.push('\n');
        state.prev_hash = entry_hash;

        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .and_then(|mut file| {
                use std::io::Write;
                file.write_all(line_str.as_bytes())
            })?;
        Ok(())
    }

    /// Start a fresh chain for `path`: draw a random salt and persist it in a
    /// `chain_header` line. The salt doubles as the genesis `prev_hash`.
    fn init_chain(&self, path: &std::path::Path) -> ConfigResult<ChainState> {
        let salt = random_chain_salt()?;
        let header = serde_json::json!({
            "record": CHAIN_HEADER_RECORD,
            "version": 1,
            "algorithm": "hmac-sha256",
            "salt": hex::encode(salt),
        });
        let mut header_str = serde_json::to_string(&header).map_err(|e| {
            ConfigError::InvalidValue {
                key: "audit.chain_header".into(),
                expected_type: "serializable chain header".into(),
                message: e.to_string(),
            }
        })?;
        header_str.push('\n');
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .and_then(|mut file| {
                use std::io::Write;
                file.write_all(header_str.as_bytes())
            })?;
        Ok(ChainState {
            salt,
            prev_hash: salt,
        })
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

impl Drop for AuditWriter {
    fn drop(&mut self) {
        // Best-effort chain footers: a dropped writer must never panic nor
        // mask the original outcome, so append errors are swallowed here by
        // design. The footer is pure metadata (it is not part of the hash
        // chain) and is skipped by `verify_audit_chain`.
        let Ok(states) = self.chain_states.lock() else {
            return;
        };
        if states.is_empty() {
            return;
        }
        let _guard = self.write_lock.lock();
        for (path, state) in states.iter() {
            let footer = serde_json::json!({
                "record": CHAIN_FOOTER_RECORD,
                "last_hash": hex::encode(state.prev_hash),
            });
            let Ok(mut line) = serde_json::to_string(&footer) else {
                continue;
            };
            line.push('\n');
            let _ = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
                .and_then(|mut file| {
                    use std::io::Write;
                    file.write_all(line.as_bytes())
                });
        }
    }
}

/// Verify the HMAC hash chain of an audit file.
///
/// Returns `Ok(true)` only when the file starts with a `chain_header` record
/// and every event line's stored `prev_hash` matches the previous entry's
/// `hmac` (or the header salt for the first event) and its stored `hmac`
/// equals `HMAC-SHA256(salt, prev_hash || canonical_event_bytes)`. Any
/// tampered byte, deleted event, reordered events, or truncated chain makes
/// the verification fail with `Ok(false)`; `chain_footer` lines are metadata
/// and are skipped. `Ok(false)` is also returned for a missing file;
/// `Err` is reserved for IO failures other than a missing file.
pub fn verify_audit_chain(path: &std::path::Path) -> ConfigResult<bool> {
    use std::io::ErrorKind;

    let content = match std::fs::read_to_string(path) {
        Ok(content) => content,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(false),
        Err(e) => return Err(ConfigError::IoError(e)),
    };

    let parse_failure = || ConfigError::InvalidValue {
        key: "audit.chain".into(),
        expected_type: "well-formed audit chain file".into(),
        message: "line is not valid JSON".into(),
    };

    let mut lines = content
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(serde_json::from_str::<serde_json::Value>);

    // The chain must begin with a header carrying the salt.
    let header = match lines.next() {
        Some(Ok(v)) => v,
        Some(Err(_)) => return Err(parse_failure()),
        None => return Ok(false),
    };
    let header_ok = header.get("record").and_then(|r| r.as_str()) == Some(CHAIN_HEADER_RECORD);
    if !header_ok {
        return Ok(false);
    }
    let Some(salt_hex) = header.get("salt").and_then(|s| s.as_str()) else {
        return Ok(false);
    };
    let Ok(salt) = hex::decode(salt_hex) else {
        return Ok(false);
    };
    if salt.len() != CHAIN_SALT_LEN {
        return Ok(false);
    }
    let mut prev_hash = salt.clone();

    for line in lines {
        let value = match line {
            Ok(v) => v,
            Err(_) => return Err(parse_failure()),
        };
        let Some(obj) = value.as_object() else {
            return Ok(false);
        };

        // Metadata records (chain footer, possible future extensions) are not
        // part of the hash chain.
        if obj.contains_key("record") {
            continue;
        }

        let Some(prev_hash_hex) = obj.get("prev_hash").and_then(|p| p.as_str()) else {
            return Ok(false);
        };
        let Some(hmac_hex) = obj.get("hmac").and_then(|h| h.as_str()) else {
            return Ok(false);
        };

        // Deletion / reordering detection: every entry must continue the
        // exact chain position it claims.
        if prev_hash_hex != hex::encode(&prev_hash) {
            return Ok(false);
        }

        let mut event_obj = obj.clone();
        event_obj.remove("prev_hash");
        event_obj.remove("hmac");
        let canonical = canonical_event_bytes(&serde_json::Value::Object(event_obj))?;
        if hmac_hex != hex::encode(hmac_sha256(&salt, &prev_hash, &canonical)) {
            return Ok(false);
        }

        prev_hash = match hex::decode(hmac_hex) {
            Ok(h) if h.len() == CHAIN_HASH_LEN => h,
            _ => return Ok(false),
        };
    }

    Ok(true)
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

    /// Read all non-empty lines of the single audit file in `dir`.
    fn audit_lines(dir: &std::path::Path) -> Vec<String> {
        let mut files: Vec<_> = std::fs::read_dir(dir)
            .expect("log dir readable")
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .collect();
        assert!(!files.is_empty(), "audit file must exist");
        files.sort();
        std::fs::read_to_string(&files[0])
            .expect("audit file readable")
            .lines()
            .map(str::to_string)
            .filter(|l| !l.trim().is_empty())
            .collect()
    }

    /// Write `sources` as LoadSuccess events into a fresh audit dir.
    fn write_events(dir: &std::path::Path, sources: &[&str]) {
        let writer = AuditWriter::builder()
            .enabled(true)
            .log_dir(dir.to_path_buf())
            .build();
        for source in sources {
            writer.log_load(source).expect("event must write");
        }
    }

    fn event_line_indices(lines: &[String]) -> Vec<usize> {
        lines
            .iter()
            .enumerate()
            .filter(|(_, l)| {
                serde_json::from_str::<serde_json::Value>(l)
                    .ok()
                    .and_then(|v| {
                        let obj = v.as_object()?;
                        Some(obj.contains_key("hmac") && !obj.contains_key("record"))
                    })
                    .unwrap_or(false)
            })
            .map(|(i, _)| i)
            .collect()
    }

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

    #[test]
    fn test_chain_header_and_event_metadata_present() {
        // Every audit file starts with a chain_header line carrying a random
        // salt, and every event line carries prev_hash + hmac.
        let dir = tempfile::tempdir().unwrap();
        write_events(dir.path(), &["chain-src-a"]);

        let lines = audit_lines(dir.path());
        let header: serde_json::Value = serde_json::from_str(&lines[0]).unwrap();
        assert_eq!(
            header.get("record").and_then(|r| r.as_str()),
            Some(CHAIN_HEADER_RECORD),
            "first line must be the chain header: {}",
            lines[0]
        );
        assert!(header.get("salt").is_some(), "header must carry the salt");

        let events = event_line_indices(&lines);
        assert_eq!(events.len(), 1);
        let event: serde_json::Value = serde_json::from_str(&lines[events[0]]).unwrap();
        assert!(
            event.get("LoadSuccess").is_some(),
            "event fields stay at the top level: {}",
            lines[events[0]]
        );
        assert!(event.get("prev_hash").is_some(), "event carries prev_hash");
        assert!(event.get("hmac").is_some(), "event carries hmac");
    }

    #[test]
    fn test_verify_audit_chain_accepts_intact_chain() {
        let dir = tempfile::tempdir().unwrap();
        write_events(dir.path(), &["src-a", "src-b", "src-c"]);

        let path = &std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .next()
            .unwrap();
        assert!(
            verify_audit_chain(path).unwrap(),
            "an intact chain must verify"
        );
    }

    #[test]
    fn test_verify_audit_chain_detects_tampering() {
        let dir = tempfile::tempdir().unwrap();
        write_events(dir.path(), &["src-a", "src-b", "src-c"]);

        let path = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .next()
            .unwrap();
        let lines = audit_lines(dir.path());
        let events = event_line_indices(&lines);
        // Flip one byte inside the middle event's payload.
        let tampered = lines[events[1]].replace("src-b", "src-X");
        assert_ne!(tampered, lines[events[1]], "tamper must change the line");
        let rewritten: Vec<String> = lines
            .iter()
            .enumerate()
            .map(|(i, l)| if i == events[1] { tampered.clone() } else { l.clone() })
            .collect();
        std::fs::write(&path, rewritten.join("\n") + "\n").unwrap();

        assert!(
            !verify_audit_chain(&path).unwrap(),
            "a tampered event byte must break verification"
        );
    }

    #[test]
    fn test_verify_audit_chain_detects_deletion() {
        let dir = tempfile::tempdir().unwrap();
        write_events(dir.path(), &["src-a", "src-b", "src-c"]);

        let path = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .next()
            .unwrap();
        let lines = audit_lines(dir.path());
        let events = event_line_indices(&lines);
        // Drop the middle event line.
        let rewritten: Vec<String> = lines
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != events[1])
            .map(|(_, l)| l.clone())
            .collect();
        std::fs::write(&path, rewritten.join("\n") + "\n").unwrap();

        assert!(
            !verify_audit_chain(&path).unwrap(),
            "a deleted event must break verification"
        );
    }

    #[test]
    fn test_verify_audit_chain_detects_reordering() {
        let dir = tempfile::tempdir().unwrap();
        write_events(dir.path(), &["src-a", "src-b", "src-c"]);

        let path = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .next()
            .unwrap();
        let mut lines = audit_lines(dir.path());
        let events = event_line_indices(&lines);
        lines.swap(events[0], events[1]);
        std::fs::write(&path, lines.join("\n") + "\n").unwrap();

        assert!(
            !verify_audit_chain(&path).unwrap(),
            "reordered events must break verification"
        );
    }

    #[test]
    fn test_verify_audit_chain_missing_or_garbage_file() {
        // Missing file → not a valid chain.
        assert!(!verify_audit_chain(std::path::Path::new("/nonexistent/audit.log")).unwrap());

        // A file that never had a chain (legacy plain JSONL) must not verify.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("audit_legacy.log");
        std::fs::write(&path, "{\"LoadSuccess\":{\"source\":\"old\"}}\n").unwrap();
        assert!(!verify_audit_chain(&path).unwrap());
    }

    #[test]
    fn test_chain_salt_is_random_per_file() {
        let dir_a = tempfile::tempdir().unwrap();
        let dir_b = tempfile::tempdir().unwrap();
        write_events(dir_a.path(), &["x"]);
        write_events(dir_b.path(), &["x"]);

        let salt = |dir: &std::path::Path| {
            let lines = audit_lines(dir);
            let header: serde_json::Value = serde_json::from_str(&lines[0]).unwrap();
            header
                .get("salt")
                .and_then(|s| s.as_str())
                .expect("header salt")
                .to_string()
        };
        assert_ne!(
            salt(dir_a.path()),
            salt(dir_b.path()),
            "each chain must start from a fresh random salt"
        );
    }

    #[test]
    fn test_chain_continues_when_file_reopened() {
        // Writer A writes one event and is dropped (footer appended); writer B
        // reopening the same-day file must continue A's chain, and the file
        // must still verify.
        let dir = tempfile::tempdir().unwrap();
        {
            let writer = AuditWriter::builder()
                .enabled(true)
                .log_dir(dir.path().to_path_buf())
                .build();
            writer.log_load("first-session").unwrap();
        }
        {
            let writer = AuditWriter::builder()
                .enabled(true)
                .log_dir(dir.path().to_path_buf())
                .build();
            writer.log_load("second-session").unwrap();
        }

        let path = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .next()
            .unwrap();
        assert!(
            verify_audit_chain(&path).unwrap(),
            "a reopened file must continue the same chain and verify"
        );
    }
}
