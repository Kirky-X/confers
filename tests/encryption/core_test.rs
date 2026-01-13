// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Core encryption tests

use std::env;

/// Test basic encryption and decryption functionality
#[test]
fn test_encrypt_decrypt_basic() {
    let key = [0u8; 32];
    let encryption = confers::encryption::ConfigEncryption::new(key);

    let plaintext = "Hello, World!";
    let encrypted = encryption
        .encrypt(plaintext)
        .expect("Encryption should succeed");

    assert_ne!(encrypted, plaintext);

    let decrypted = encryption
        .decrypt(&encrypted)
        .expect("Decryption should succeed");
    assert_eq!(decrypted, plaintext);
}

/// Test encryption produces different outputs for same input
#[test]
fn test_encrypt_produces_different_outputs() {
    let key = [0u8; 32];
    let encryption = confers::encryption::ConfigEncryption::new(key);

    let plaintext = "Same input";

    let encrypted1 = encryption
        .encrypt(plaintext)
        .expect("First encryption should succeed");
    let encrypted2 = encryption
        .encrypt(plaintext)
        .expect("Second encryption should succeed");

    assert_ne!(encrypted1, encrypted2);

    let decrypted1 = encryption
        .decrypt(&encrypted1)
        .expect("First decryption should succeed");
    let decrypted2 = encryption
        .decrypt(&encrypted2)
        .expect("Second decryption should succeed");

    assert_eq!(decrypted1, plaintext);
    assert_eq!(decrypted2, plaintext);
}

/// Test encryption with empty string
#[test]
fn test_encrypt_empty_string() {
    let key = [0u8; 32];
    let encryption = confers::encryption::ConfigEncryption::new(key);

    let plaintext = "";
    let encrypted = encryption
        .encrypt(plaintext)
        .expect("Encryption should succeed");

    let decrypted = encryption
        .decrypt(&encrypted)
        .expect("Decryption should succeed");
    assert_eq!(decrypted, plaintext);
}

/// Test encryption with special characters
#[test]
fn test_encrypt_special_characters() {
    let key = [0xFFu8; 32];
    let encryption = confers::encryption::ConfigEncryption::new(key);

    let test_cases = vec!["Hello\tWorld\n", "Special: !@#$%^&*()", "Newlines:\n\n\n"];

    for plaintext in test_cases {
        let encrypted = encryption
            .encrypt(plaintext)
            .expect("Encryption should succeed");
        let decrypted = encryption
            .decrypt(&encrypted)
            .expect("Decryption should succeed");
        assert_eq!(decrypted, plaintext);
    }
}

/// Test encryption with long strings
#[test]
fn test_encrypt_long_string() {
    let key = [0xAB; 32];
    let encryption = confers::encryption::ConfigEncryption::new(key);

    let plaintext = "A".repeat(10000);
    let encrypted = encryption
        .encrypt(&plaintext)
        .expect("Encryption should succeed");
    let decrypted = encryption
        .decrypt(&encrypted)
        .expect("Decryption should succeed");
    assert_eq!(decrypted, plaintext);
}

/// Test decryption with wrong key fails
#[test]
fn test_decrypt_wrong_key_fails() {
    let key1 = [0u8; 32];
    let key2 = [1u8; 32];

    let encryption1 = confers::encryption::ConfigEncryption::new(key1);
    let encryption2 = confers::encryption::ConfigEncryption::new(key2);

    let plaintext = "Secret message";
    let encrypted = encryption1
        .encrypt(plaintext)
        .expect("Encryption should succeed");

    let result = encryption2.decrypt(&encrypted);
    assert!(result.is_err());
}

/// Test decryption with invalid hex string
#[test]
fn test_decrypt_invalid_hex() {
    let key = [0u8; 32];
    let encryption = confers::encryption::ConfigEncryption::new(key);

    let invalid_hex = "not-valid-hex!!!";
    let result = encryption.decrypt(invalid_hex);
    assert!(result.is_err());
}

/// Test decryption with too short data
#[test]
fn test_decrypt_too_short() {
    let key = [0u8; 32];
    let encryption = confers::encryption::ConfigEncryption::new(key);

    let too_short = hex::encode([0u8; 11]);
    let result = encryption.decrypt(&too_short);
    assert!(result.is_err());
}

/// Test from_env with valid key
#[test]
fn test_from_env_valid_key() {
    let key = [0xDEu8; 32];
    let key_hex = hex::encode(key);

    env::set_var("CONFERS_ENCRYPTION_KEY", &key_hex);

    let encryption = confers::encryption::ConfigEncryption::from_env()
        .expect("Should create encryption from env");

    let plaintext = "Environment test";
    let encrypted = encryption
        .encrypt(plaintext)
        .expect("Encryption should succeed");
    let decrypted = encryption
        .decrypt(&encrypted)
        .expect("Decryption should succeed");
    assert_eq!(decrypted, plaintext);

    env::remove_var("CONFERS_ENCRYPTION_KEY");
}

/// Test from_env with missing environment variable
#[test]
fn test_from_env_missing() {
    env::remove_var("CONFERS_ENCRYPTION_KEY");

    let result = confers::encryption::ConfigEncryption::from_env();
    assert!(result.is_err());
}

/// Test from_env with invalid hex format
#[test]
fn test_from_env_invalid_hex() {
    env::set_var("CONFERS_ENCRYPTION_KEY", "not-hex-string");

    let result = confers::encryption::ConfigEncryption::from_env();
    assert!(result.is_err());

    env::remove_var("CONFERS_ENCRYPTION_KEY");
}

/// Test from_env with wrong key length
#[test]
fn test_from_env_wrong_length() {
    env::set_var("CONFERS_ENCRYPTION_KEY", hex::encode([0u8; 16]));

    let result = confers::encryption::ConfigEncryption::from_env();
    assert!(result.is_err());

    env::remove_var("CONFERS_ENCRYPTION_KEY");
}

/// Test encryption output format is valid hex
#[test]
fn test_encrypt_output_is_valid_hex() {
    let key = [0x12u8; 32];
    let encryption = confers::encryption::ConfigEncryption::new(key);

    let encrypted = encryption
        .encrypt("test")
        .expect("Encryption should succeed");

    let decoded = hex::decode(&encrypted).expect("Output should be valid hex");
    assert!(decoded.len() >= 12);
}

/// Test multiple encrypt/decrypt cycles
#[test]
fn test_multiple_cycles() {
    let key = [0x42u8; 32];
    let encryption = confers::encryption::ConfigEncryption::new(key);

    let plaintexts = vec!["First message", "Second message", "Third message"];

    for plaintext in plaintexts {
        let encrypted = encryption
            .encrypt(plaintext)
            .expect("Encryption should succeed");
        let decrypted = encryption
            .decrypt(&encrypted)
            .expect("Decryption should succeed");
        assert_eq!(decrypted, plaintext);
    }
}

/// Test encryption with unicode content
#[test]
fn test_encrypt_unicode() {
    let key = [0x55u8; 32];
    let encryption = confers::encryption::ConfigEncryption::new(key);

    let test_cases = vec!["English", "中文", "한국어", "🎉🎊🎈"];

    for plaintext in test_cases {
        let encrypted = encryption
            .encrypt(plaintext)
            .expect("Encryption should succeed");
        let decrypted = encryption
            .decrypt(&encrypted)
            .expect("Decryption should succeed");
        assert_eq!(decrypted, plaintext);
    }
}
