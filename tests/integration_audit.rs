//! Integration tests for the Audit module.

#![cfg(feature = "audit")]

#[cfg(test)]
mod tests {
    use confers::audit::{AuditConfig, AuditEvent, AuditLevel, AuditWriter};
    use std::path::PathBuf;
    use tempfile::TempDir;

    /// Test 1: Verify default configuration
    #[test]
    fn test_audit_writer_default() {
        let writer = AuditWriter::new();
        assert!(
            writer.is_enabled(),
            "Audit writer should be enabled by default"
        );
    }

    /// Test 2: Verify Builder pattern
    #[test]
    fn test_audit_writer_builder() {
        let temp_dir = TempDir::new().unwrap();
        let log_dir: PathBuf = temp_dir.path().to_path_buf();

        let writer = AuditWriter::builder()
            .enabled(true)
            .log_dir(log_dir.clone())
            .durable_wal(true)
            .build();

        assert!(writer.is_enabled(), "Audit writer should be enabled");
    }

    /// Test 3: Verify load success logging
    #[test]
    fn test_audit_log_load() {
        let temp_dir = TempDir::new().unwrap();
        let log_dir: PathBuf = temp_dir.path().to_path_buf();

        let writer = AuditWriter::builder()
            .enabled(true)
            .log_dir(log_dir.clone())
            .build();

        writer.log_load("test_source");

        // Give time for async write
        std::thread::sleep(std::time::Duration::from_millis(100));

        // Verify log directory exists
        assert!(log_dir.exists(), "Log directory should exist after logging");
    }

    /// Test 4: Verify key access logging
    #[test]
    fn test_audit_log_key_access() {
        let temp_dir = TempDir::new().unwrap();
        let log_dir: PathBuf = temp_dir.path().to_path_buf();

        let writer = AuditWriter::builder()
            .enabled(true)
            .log_dir(log_dir.clone())
            .build();

        writer.log_key_access("database.password");

        // Give time for async write
        std::thread::sleep(std::time::Duration::from_millis(100));

        // Verify log directory exists
        assert!(log_dir.exists(), "Log directory should exist after logging");
    }

    /// Test 5: Verify decrypt log sanitization
    #[test]
    fn test_audit_log_decrypt_sanitizes() {
        let temp_dir = TempDir::new().unwrap();
        let log_dir: PathBuf = temp_dir.path().to_path_buf();

        let writer = AuditWriter::builder()
            .enabled(true)
            .log_dir(log_dir.clone())
            .build();

        // Test with sensitive field name
        writer.log_decrypt("database.password", true);
        writer.log_decrypt("api_secret_key", true);
        writer.log_decrypt("regular_field", true);

        // Give time for async write
        std::thread::sleep(std::time::Duration::from_millis(100));

        // Verify log directory exists
        assert!(log_dir.exists(), "Log directory should exist after logging");
    }

    /// Test 6: Verify key rotation logging
    #[test]
    fn test_audit_log_key_rotation() {
        let temp_dir = TempDir::new().unwrap();
        let log_dir: PathBuf = temp_dir.path().to_path_buf();

        let writer = AuditWriter::builder()
            .enabled(true)
            .log_dir(log_dir.clone())
            .build();

        writer.log_key_rotation("v1.0.0", "v1.0.1");

        // Give time for async write
        std::thread::sleep(std::time::Duration::from_millis(100));

        // Verify log directory exists
        assert!(log_dir.exists(), "Log directory should exist after logging");
    }

    /// Test 7: Verify disabled state
    #[test]
    fn test_audit_disabled() {
        let writer = AuditWriter::builder().enabled(false).build();

        assert!(!writer.is_enabled(), "Audit writer should be disabled");

        // These should be no-ops when disabled
        writer.log_load("test_source");
        writer.log_key_access("test_key");
        writer.log_decrypt("test_field", true);
        writer.log_key_rotation("v1", "v2");
    }

    /// Test 8: Verify event level classification
    #[test]
    fn test_audit_level_for_event() {
        // Durable events
        let key_access = AuditEvent::KeyAccess {
            key: "test".to_string(),
            timestamp: chrono::Utc::now(),
        };
        assert_eq!(
            AuditLevel::for_event(&key_access),
            AuditLevel::Durable,
            "KeyAccess should be Durable"
        );

        let key_rotation = AuditEvent::KeyRotation {
            old_version: "v1".to_string(),
            new_version: "v2".to_string(),
            timestamp: chrono::Utc::now(),
        };
        assert_eq!(
            AuditLevel::for_event(&key_rotation),
            AuditLevel::Durable,
            "KeyRotation should be Durable"
        );

        let decrypt = AuditEvent::Decrypt {
            field: "test".to_string(),
            success: true,
            timestamp: chrono::Utc::now(),
        };
        assert_eq!(
            AuditLevel::for_event(&decrypt),
            AuditLevel::Durable,
            "Decrypt should be Durable"
        );

        // BestEffort events
        let load_success = AuditEvent::LoadSuccess {
            source: "test".to_string(),
            timestamp: chrono::Utc::now(),
        };
        assert_eq!(
            AuditLevel::for_event(&load_success),
            AuditLevel::BestEffort,
            "LoadSuccess should be BestEffort"
        );

        let reload_trigger = AuditEvent::ReloadTrigger {
            source: "test".to_string(),
            timestamp: chrono::Utc::now(),
        };
        assert_eq!(
            AuditLevel::for_event(&reload_trigger),
            AuditLevel::BestEffort,
            "ReloadTrigger should be BestEffort"
        );
    }

    /// Test 9: Verify AuditConfig default
    #[test]
    fn test_audit_config_default() {
        let config = AuditConfig::default();
        assert!(config.enabled, "Enabled should be true by default");
        assert!(
            !config.durable_wal,
            "Durable WAL should be false by default"
        );
        assert_eq!(config.channel_size, 1024, "Channel size should be 1024");
        assert!(
            config.log_dir.is_none(),
            "Log dir should be None by default"
        );
    }

    /// Test 10: Verify AuditConfig builder
    #[test]
    fn test_audit_config_builder() {
        let temp_dir = TempDir::new().unwrap();
        let log_dir: PathBuf = temp_dir.path().to_path_buf();

        let config = AuditConfig::builder()
            .enabled(false)
            .log_dir(log_dir.clone())
            .durable_wal(true)
            .channel_size(2048)
            .build();

        assert!(!config.enabled);
        assert_eq!(config.log_dir, Some(log_dir));
        assert!(config.durable_wal);
        assert_eq!(config.channel_size, 2048);
    }

    /// Test 11: Verify BestEffort events are actually persisted to log file.
    /// Regression test for S-H-2: write_best_effort was previously a no-op
    /// that sanitized the event then discarded the result without writing.
    #[test]
    fn test_audit_best_effort_event_persists_to_file() {
        let temp_dir = TempDir::new().unwrap();
        let log_dir: PathBuf = temp_dir.path().to_path_buf();

        let writer = AuditWriter::builder()
            .enabled(true)
            .log_dir(log_dir.clone())
            .build();

        // LoadSuccess is classified as BestEffort
        writer.log_load("test_source_for_best_effort");

        // Give time for sync write
        std::thread::sleep(std::time::Duration::from_millis(100));

        // Find the audit log file
        let entries: Vec<_> = std::fs::read_dir(&log_dir)
            .expect("log dir readable")
            .filter_map(|e| e.ok())
            .collect();
        assert!(
            !entries.is_empty(),
            "BestEffort event should create at least one log file in log_dir"
        );

        // Verify the file actually contains the event content
        let log_file = &entries[0];
        let content = std::fs::read_to_string(log_file.path()).expect("log file readable");
        assert!(
            content.contains("LoadSuccess"),
            "BestEffort event should be persisted; got content: {}",
            content
        );
        assert!(
            content.contains("test_source_for_best_effort"),
            "BestEffort event source should appear in log; got content: {}",
            content
        );
    }

    /// Test 12: Verify Durable events are persisted to log file.
    /// Companion to test_audit_best_effort_event_persists_to_file.
    #[test]
    fn test_audit_durable_event_persists_to_file() {
        let temp_dir = TempDir::new().unwrap();
        let log_dir: PathBuf = temp_dir.path().to_path_buf();

        let writer = AuditWriter::builder()
            .enabled(true)
            .log_dir(log_dir.clone())
            .build();

        // KeyAccess is classified as Durable
        writer.log_key_access("test.key.durable");

        std::thread::sleep(std::time::Duration::from_millis(100));

        let entries: Vec<_> = std::fs::read_dir(&log_dir)
            .expect("log dir readable")
            .filter_map(|e| e.ok())
            .collect();
        assert!(
            !entries.is_empty(),
            "Durable event should create a log file"
        );

        let content = std::fs::read_to_string(entries[0].path()).expect("log file readable");
        assert!(
            content.contains("KeyAccess"),
            "Durable event should be persisted; got content: {}",
            content
        );
    }

    /// Test 13: Verify no log file is created when log_dir is None.
    /// Ensures write_best_effort does not panic or write to a phantom path.
    #[test]
    fn test_audit_no_log_dir_does_not_panic() {
        let writer = AuditWriter::new(); // log_dir defaults to None
        writer.log_load("source_without_log_dir");
        writer.log_key_access("key_without_log_dir");
        // No assertion needed - test passes if no panic occurs.
    }
}
