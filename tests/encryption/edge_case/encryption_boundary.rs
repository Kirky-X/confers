// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! 边界测试：加密解密边界条件
//!
//! 测试加密解密的边界条件和错误处理

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