// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! 集成测试：加密环境变量功能
//!
//! 测试从环境变量加载加密密钥的功能

use std::env;

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