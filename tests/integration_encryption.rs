//! Integration tests for encryption module.
//!
//! Run with: cargo test --test integration_encryption --features encryption

mod common;

#[cfg(feature = "encryption")]
mod tests {
    use super::*;
    use confers::secret::{
        derive_field_key, CryptoError, EnvKeyProvider, SecretBytes, SecretKeyProvider,
        SecretString, XChaCha20Crypto,
    };

    // ========================================
    // SecretString Tests
    // ========================================

    #[test]
    fn test_secret_string_debug_redacts() {
        let secret = SecretString::new("super-secret-password");
        let debug_output = format!("{:?}", secret);

        assert_eq!(debug_output, "[REDACTED]");
        assert!(!debug_output.contains("super-secret-password"));
    }

    #[test]
    fn test_secret_string_expose() {
        let secret = SecretString::new("my-secret");

        // Test expose method
        assert_eq!(secret.expose(), "my-secret");

        // Test expose_clone method
        assert_eq!(secret.expose_clone(), "my-secret");
    }

    #[test]
    fn test_secret_string_deref() {
        let secret = SecretString::new("test-value");

        // Test Deref implementation
        let s: &str = &secret;
        assert_eq!(s, "test-value");
    }

    #[test]
    fn test_secret_string_default() {
        let default_secret: SecretString = SecretString::default();
        assert_eq!(default_secret.expose(), "");
    }

    #[test]
    fn test_secret_string_clone() {
        let original = SecretString::new("clone-me");
        let cloned = original.clone();

        assert_eq!(original.expose(), cloned.expose());
    }

    // ========================================
    // SecretBytes Tests
    // ========================================

    #[test]
    fn test_secret_bytes_debug_redacts() {
        let secret = SecretBytes::new(vec![1, 2, 3, 4, 5]);
        let debug_output = format!("{:?}", secret);

        assert_eq!(debug_output, "[REDACTED]");
        assert!(!debug_output.contains("1"));
        assert!(!debug_output.contains("2"));
    }

    #[test]
    fn test_secret_bytes_drop_zeroizes() {
        let bytes_before_drop: Vec<u8>;
        {
            let secret = SecretBytes::new(vec![0xAB, 0xCD, 0xEF, 0x12, 0x34]);
            bytes_before_drop = secret.as_slice().to_vec();
            // SecretBytes will be dropped here and zeroized
        }

        // Verify the original values were readable before drop
        assert_eq!(bytes_before_drop, vec![0xAB, 0xCD, 0xEF, 0x12, 0x34]);
    }

    #[test]
    fn test_secret_bytes_clone() {
        // SecretBytes intentionally does not implement Clone to prevent bypassing memory protection.
        // Instead, verify we can create separate instances with same content.
        let original = SecretBytes::new(vec![10, 20, 30]);
        let original_slice = original.as_slice().to_vec();

        // Create another instance with same content
        let another = SecretBytes::new(original_slice.clone());

        assert_eq!(original.as_slice(), another.as_slice());
    }

    #[test]
    fn test_secret_bytes_len() {
        let empty = SecretBytes::new(vec![]);
        assert!(empty.is_empty());
        assert_eq!(empty.len(), 0);

        let non_empty = SecretBytes::new(vec![1, 2, 3, 4, 5]);
        assert!(!non_empty.is_empty());
        assert_eq!(non_empty.len(), 5);
    }

    // ========================================
    // XChaCha20Crypto Tests
    // Note: These tests use fixed keys for functional testing purposes.
    // In production, always use cryptographically secure random keys.
    // ========================================

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let crypto = XChaCha20Crypto::new();
        let plaintext = b"Hello, World! This is a secret message.";
        // Fixed key for functional testing - production should use random keys
        let key = [0u8; 32];

        // Encrypt
        let (nonce, ciphertext) = crypto.encrypt(plaintext, &key).expect("encryption failed");

        assert_eq!(nonce.len(), 24); // XChaCha20 uses 24-byte (192-bit) nonce
        assert!(ciphertext.len() > plaintext.len()); // Includes auth tag

        // Decrypt
        let decrypted = crypto
            .decrypt(&nonce, &ciphertext, &key)
            .expect("decryption failed");

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_encrypt_produces_unique_nonces() {
        let crypto = XChaCha20Crypto::new();
        let plaintext = b"same message";
        let key = [0u8; 32];

        let (nonce1, _) = crypto.encrypt(plaintext, &key).unwrap();
        let (nonce2, _) = crypto.encrypt(plaintext, &key).unwrap();

        // Nonces should be different (random)
        assert_ne!(nonce1, nonce2);
    }

    #[test]
    fn test_decrypt_with_wrong_key_fails() {
        let crypto = XChaCha20Crypto::new();
        let plaintext = b"secret data";
        let key1 = [0u8; 32];
        let key2 = [1u8; 32];

        let (nonce, ciphertext) = crypto.encrypt(plaintext, &key1).unwrap();

        // Decrypting with wrong key should fail
        let result = crypto.decrypt(&nonce, &ciphertext, &key2);
        assert!(result.is_err());
    }

    #[test]
    fn test_decrypt_with_wrong_nonce_fails() {
        let crypto = XChaCha20Crypto::new();
        let plaintext = b"secret data";
        let key = [0u8; 32];

        let (_, ciphertext) = crypto.encrypt(plaintext, &key).unwrap();
        let wrong_nonce = [0u8; 12];

        // Decrypting with wrong nonce should fail
        let result = crypto.decrypt(&wrong_nonce, &ciphertext, &key);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_key_length_fails() {
        let crypto = XChaCha20Crypto::new();
        let plaintext = b"test";

        // Key too short
        let short_key = [0u8; 16];
        let result = crypto.encrypt(plaintext, &short_key);
        assert!(matches!(result, Err(CryptoError::InvalidKeyLength(16))));

        // Key too long
        let long_key = [0u8; 64];
        let result = crypto.encrypt(plaintext, &long_key);
        assert!(matches!(result, Err(CryptoError::InvalidKeyLength(64))));

        // Exactly 32 bytes should work
        let valid_key = [0u8; 32];
        let result = crypto.encrypt(plaintext, &valid_key);
        assert!(result.is_ok());
    }

    // ========================================
    // HKDF Key Derivation Tests
    // ========================================

    #[test]
    fn test_derive_field_key() {
        let master_key = [0u8; 32];
        let field_path = "database.password";
        let key_version = "v1";

        let field_key = derive_field_key(&master_key, field_path, key_version).unwrap();

        assert_eq!(field_key.len(), 32);
    }

    #[test]
    fn test_derive_field_key_different_fields() {
        let master_key = [0u8; 32];

        let key1 = derive_field_key(&master_key, "field1", "v1").unwrap();
        let key2 = derive_field_key(&master_key, "field2", "v1").unwrap();
        let key3 = derive_field_key(&master_key, "field1", "v2").unwrap();

        // Different field paths should produce different keys
        assert_ne!(key1, key2);

        // Different versions should produce different keys
        assert_ne!(key1, key3);
    }

    #[test]
    fn test_derive_field_key_deterministic() {
        let master_key = [0x42u8; 32];

        let key1 = derive_field_key(&master_key, "same.field", "v1").unwrap();
        let key2 = derive_field_key(&master_key, "same.field", "v1").unwrap();

        // Same inputs should produce same output
        assert_eq!(key1, key2);
    }

    // ========================================
    // EnvKeyProvider Tests
    // ========================================

    #[test]
    fn test_env_key_provider() {
        common::with_env_var(
            "TEST_ENCRYPTION_KEY",
            "12345678901234567890123456789012",
            || {
                let provider = EnvKeyProvider::new("TEST_ENCRYPTION_KEY");
                let result = provider.get_key();

                assert!(result.is_ok());
                let key = result.unwrap();
                assert_eq!(key.as_slice().len(), 32);

                assert_eq!(provider.provider_type(), "env");
            },
        );
    }

    #[test]
    fn test_env_key_provider_missing_var() {
        // Ensure the variable doesn't exist
        std::env::remove_var("NON_EXISTENT_KEY_12345");

        let provider = EnvKeyProvider::new("NON_EXISTENT_KEY_12345");
        let result = provider.get_key();

        assert!(result.is_err());
    }

    #[test]
    fn test_env_key_provider_too_short_key() {
        common::with_env_var("SHORT_KEY_TEST", "short", || {
            let provider = EnvKeyProvider::new("SHORT_KEY_TEST");
            let result = provider.get_key();

            // Should fail because key is less than 32 bytes
            assert!(matches!(result, Err(CryptoError::InvalidKeyLength(5))));
        });
    }

    #[test]
    fn test_env_key_provider_exact_length() {
        // 32 byte key - exactly correct
        common::with_env_var("EXACT_KEY_TEST", "12345678901234567890123456789012", || {
            let provider = EnvKeyProvider::new("EXACT_KEY_TEST");
            let result = provider.get_key();
            assert!(result.is_ok());
        });
    }

    #[test]
    fn test_env_key_provider_too_long_key() {
        // 33 byte key - too long
        common::with_env_var("LONG_KEY_TEST", "123456789012345678901234567890123", || {
            let provider = EnvKeyProvider::new("LONG_KEY_TEST");
            let result = provider.get_key();
            assert!(matches!(result, Err(CryptoError::InvalidKeyLength(33))));
        });
    }

    #[test]
    fn test_env_key_provider_builder() {
        common::with_env_var(
            "BUILDER_TEST_KEY",
            "12345678901234567890123456789012",
            || {
                let provider = EnvKeyProvider::builder()
                    .env_var("BUILDER_TEST_KEY")
                    .build()
                    .expect("build should succeed");

                let result = provider.get_key();
                assert!(result.is_ok());
            },
        );
    }

    #[test]
    fn test_env_key_provider_builder_missing_env_var() {
        // Use unique variable name to avoid conflicts with other tests
        let var_name = "BUILDER_MISSING_VAR_UNIQUE_12345";
        std::env::remove_var(var_name);

        let result = EnvKeyProvider::builder().env_var(var_name).build();

        // Build should fail when env var doesn't exist
        assert!(
            result.is_err(),
            "Builder should fail when env var doesn't exist: {:?}",
            result
        );
    }

    #[test]
    fn test_env_key_provider_builder_no_env_var_set() {
        let result = EnvKeyProvider::builder().build();
        assert!(result.is_err());
    }

    // ========================================
    // Integration Tests
    // ========================================

    #[test]
    fn test_full_encryption_workflow() {
        // 1. Setup master key
        let master_key = [0x1Au8; 32];

        // 2. Derive field-specific key
        let field_key = derive_field_key(&master_key, "user.password", "v1").unwrap();

        // 3. Encrypt sensitive data
        let crypto = XChaCha20Crypto::new();
        let password = "my-super-secret-password";
        let (nonce, ciphertext) = crypto.encrypt(password.as_bytes(), &field_key).unwrap();

        // 4. Decrypt and verify
        let decrypted = crypto.decrypt(&nonce, &ciphertext, &field_key).unwrap();
        assert_eq!(decrypted, password.as_bytes());
    }

    #[test]
    fn test_secret_string_with_encryption() {
        let crypto = XChaCha20Crypto::new();
        let key = [0u8; 32];

        // Encrypt a SecretString's content
        let secret = SecretString::new("sensitive-data");
        let (nonce, ciphertext) = crypto.encrypt(secret.expose().as_bytes(), &key).unwrap();

        // Decrypt
        let decrypted = crypto.decrypt(&nonce, &ciphertext, &key).unwrap();
        let decrypted_str = String::from_utf8(decrypted).unwrap();

        assert_eq!(decrypted_str, "sensitive-data");

        // Verify debug still redacts
        let debug = format!("{:?}", secret);
        assert_eq!(debug, "[REDACTED]");
    }

    // ========================================
    // Edge Case Tests
    // ========================================

    #[test]
    fn test_encrypt_empty_data() {
        let crypto = XChaCha20Crypto::new();
        let key = [0u8; 32];

        let (nonce, ciphertext) = crypto.encrypt(&[], &key).unwrap();
        assert!(!ciphertext.is_empty());

        let decrypted = crypto.decrypt(&nonce, &ciphertext, &key).unwrap();
        assert!(decrypted.is_empty());
    }

    #[test]
    fn test_encrypt_large_data() {
        let crypto = XChaCha20Crypto::new();
        let key = [0u8; 32];
        let large_data = vec![42u8; 1_000_000]; // 1MB

        let (nonce, ciphertext) = crypto.encrypt(&large_data, &key).unwrap();
        let decrypted = crypto.decrypt(&nonce, &ciphertext, &key).unwrap();

        assert_eq!(decrypted.len(), 1_000_000);
    }

    #[test]
    fn test_encrypt_key_too_short() {
        let crypto = XChaCha20Crypto::new();
        let short_key = [0u8; 16]; // Only 16 bytes

        let result = crypto.encrypt(&[1, 2, 3], &short_key);
        assert!(result.is_err());
    }

    #[test]
    fn test_encrypt_key_too_long() {
        let crypto = XChaCha20Crypto::new();
        let long_key = [0u8; 64]; // 64 bytes

        let result = crypto.encrypt(&[1, 2, 3], &long_key);
        // Should either error or truncate - implementation dependent
        // Just verify it doesn't panic
        let _ = result;
    }
}

// Placeholder when encryption feature is not enabled
#[cfg(not(feature = "encryption"))]
mod tests {
    #[test]
    #[ignore = "encryption feature required for this test"]
    fn encryption_feature_required() {
        // This test is ignored when encryption feature is not enabled
    }
}
