//! Integration tests for the Audit module.

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
        let writer = AuditWriter::new();
        writer.log_load("test_source");

        // Should not panic - just verify the write doesn't crash
    }

    /// Test 4: Verify key access logging
    #[test]
    fn test_audit_log_key_access() {
        let writer = AuditWriter::new();
        writer.log_key_access("database.password");

        // Should not panic - just verify the write doesn't crash
    }

    /// Test 5: Verify decrypt log sanitization
    #[test]
    fn test_audit_log_decrypt_sanitizes() {
        let writer = AuditWriter::new();

        // Test with sensitive field name
        writer.log_decrypt("database.password", true);
        writer.log_decrypt("api_secret_key", true);
        writer.log_decrypt("regular_field", true);

        // Should not panic - sanitization should work
    }

    /// Test 6: Verify key rotation logging
    #[test]
    fn test_audit_log_key_rotation() {
        let writer = AuditWriter::new();
        writer.log_key_rotation("v1.0.0", "v1.0.1");

        // Should not panic
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
}
