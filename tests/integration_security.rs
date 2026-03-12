//! Security integration tests.
//!
//! Tests for security features including KeyRegistry, error sanitization.

#![cfg(feature = "encryption")]

use confers::secret::{KeyRegistry, KeyRotationConfig, SecretBytes, XChaCha20Crypto};
use confers::ConfigError;

/// Generate a test key with a specific pattern.
/// NOTE: This is for testing only, never use in production!
fn make_test_key(seed: u8) -> SecretBytes {
    let mut key = vec![seed; 32];
    // Add some variation to make it more realistic
    for (i, byte) in key.iter_mut().enumerate() {
        *byte = byte.wrapping_add(i as u8);
    }
    SecretBytes::new(key)
}

#[test]
fn test_key_registry_rotation() {
    let registry = KeyRegistry::new(KeyRotationConfig::default());

    let key1 = make_test_key(1);
    let key2 = make_test_key(2);

    registry.register_key("v1".to_string(), key1, true).unwrap();
    let old = registry.rotate_to("v2".to_string(), key2).unwrap();

    assert_eq!(old, "v1");

    let (version, _) = registry.get_primary_key().unwrap();
    assert_eq!(version, "v2");
}

#[test]
fn test_key_registry_try_all_keys() {
    let registry = KeyRegistry::new(KeyRotationConfig::default());

    let key1 = make_test_key(1);
    let key2 = make_test_key(2);

    registry.register_key("v1".to_string(), key1, true).unwrap();
    registry
        .register_key("v2".to_string(), key2, false)
        .unwrap();

    let crypto = XChaCha20Crypto::new();
    let plaintext = b"test data";

    // Generate the same key pattern for encryption
    let mut k2_bytes = vec![2u8; 32];
    for (i, byte) in k2_bytes.iter_mut().enumerate() {
        *byte = byte.wrapping_add(i as u8);
    }
    let (nonce, ciphertext) = crypto.encrypt(plaintext, &k2_bytes).unwrap();

    let (version, decrypted) = registry
        .try_decrypt_with_all_keys(&nonce, &ciphertext)
        .unwrap();

    assert_eq!(version, "v2");
    assert_eq!(decrypted, plaintext);
}

#[test]
fn test_error_sanitization() {
    let error = ConfigError::FileNotFound {
        filename: "/tmp/test_config.txt".into(),
        source: None,
    };

    let user_message = error.user_message();
    assert!(user_message.contains("not found") || user_message.contains("file"));
}
