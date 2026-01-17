// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! 单元测试：加密解密基础功能
//!
//! 测试加密解密的基本功能和正确性

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