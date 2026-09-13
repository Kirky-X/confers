// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Cryptographic operations for Confers.
//!
//! # Encryption Algorithm
//!
//! Confers uses **XChaCha20-Poly1305** as the sole encryption algorithm.
//!
//! ## Migration from AES-256-GCM
//!
//! Versions prior to 0.4.2 used AES-256-GCM. The legacy decryption path
//! (`CryptoError::LegacyDecryptionFailed`) is now deprecated and will be
//! removed in v0.5. To migrate:
//!
//! 1. Re-encrypt existing data with XChaCha20-Poly1305 using [`XChaCha20Crypto`].
//! 2. Remove any code that matches on `CryptoError::LegacyDecryptionFailed`.

use chacha20poly1305::{
    XChaCha20Poly1305, XNonce,
    aead::{Aead, KeyInit, Payload},
};
use getrandom::fill as fill_from_os_rng;
use hkdf::Hkdf;
use sha2::Sha256;

#[derive(Debug, thiserror::Error)]
pub enum CryptoError {
    #[error("encryption failed")]
    EncryptionFailed,
    #[error("decryption failed")]
    DecryptionFailed,
    #[error("invalid key length: expected exactly 32 bytes for XChaCha20-Poly1305, got {0} bytes")]
    InvalidKeyLength(usize),
    #[error("key not found")]
    KeyNotFound,
    #[deprecated(
        since = "0.4.2",
        note = "AES-256-GCM is superseded by XChaCha20-Poly1305; this variant will be removed in v0.5"
    )]
    #[error("legacy decryption failed (AES-256-GCM)")]
    LegacyDecryptionFailed,
}

pub const NONCE_SIZE: usize = 24;

pub struct XChaCha20Crypto;

impl XChaCha20Crypto {
    pub fn new() -> Self {
        Self
    }

    /// Encrypts `plaintext` under `key` without additional authenticated data.
    ///
    /// Equivalent to [`Self::encrypt_with_aad`] with an empty AAD slice.
    pub fn encrypt(&self, plaintext: &[u8], key: &[u8]) -> Result<(Vec<u8>, Vec<u8>), CryptoError> {
        self.encrypt_with_aad(plaintext, key, &[])
    }

    /// Encrypts `plaintext` under `key`, authenticating `aad` as additional
    /// authenticated data.
    ///
    /// The AAD is not encrypted and not part of the ciphertext, but it is
    /// cryptographically bound to it: decryption succeeds only when the exact
    /// same AAD is supplied. Use it to bind context (for example a field path
    /// and key version) so a ciphertext cannot be transplanted into another
    /// context.
    pub fn encrypt_with_aad(
        &self,
        plaintext: &[u8],
        key: &[u8],
        aad: &[u8],
    ) -> Result<(Vec<u8>, Vec<u8>), CryptoError> {
        if key.len() != 32 {
            return Err(CryptoError::InvalidKeyLength(key.len()));
        }

        let cipher =
            XChaCha20Poly1305::new_from_slice(key).map_err(|_| CryptoError::EncryptionFailed)?;

        let mut nonce_bytes = [0u8; NONCE_SIZE];
        // Fill from the OS entropy source (equivalent to the old
        // OsRng::fill_bytes; getrandom 0.4 explicitly returns a Result, and a
        // failure must never silently degrade to a default value).
        fill_from_os_rng(&mut nonce_bytes).map_err(|_| CryptoError::EncryptionFailed)?;
        let nonce =
            XNonce::try_from(&nonce_bytes[..]).map_err(|_| CryptoError::EncryptionFailed)?;

        let ciphertext = cipher
            .encrypt(
                &nonce,
                Payload {
                    msg: plaintext,
                    aad,
                },
            )
            .map_err(|_| CryptoError::EncryptionFailed)?;

        Ok((nonce_bytes.to_vec(), ciphertext))
    }

    /// Decrypts `ciphertext` with `key`, assuming no additional authenticated
    /// data.
    ///
    /// Equivalent to [`Self::decrypt_with_aad`] with an empty AAD slice.
    pub fn decrypt(
        &self,
        nonce: &[u8],
        ciphertext: &[u8],
        key: &[u8],
    ) -> Result<Vec<u8>, CryptoError> {
        self.decrypt_with_aad(nonce, ciphertext, key, &[])
    }

    /// Decrypts `ciphertext` with `key`, authenticating `aad` as additional
    /// authenticated data.
    ///
    /// Fails with [`CryptoError::DecryptionFailed`] unless `aad` is
    /// byte-for-byte identical to the AAD used during encryption.
    ///
    /// Every decryption failure is emitted as a
    /// `confers_secret_decrypt_errors_total` counter through the (optional,
    /// NoOp-by-default) [`MetricsBackend`](crate::interface::MetricsBackend).
    pub fn decrypt_with_aad(
        &self,
        nonce: &[u8],
        ciphertext: &[u8],
        key: &[u8],
        aad: &[u8],
    ) -> Result<Vec<u8>, CryptoError> {
        let result = self.decrypt_with_aad_inner(nonce, ciphertext, key, aad);
        if result.is_err() {
            // Critical-path metric: secret decryption failure.
            crate::metrics::record_counter(crate::metrics::names::SECRET_DECRYPT_ERRORS_TOTAL, &[]);
        }
        result
    }

    fn decrypt_with_aad_inner(
        &self,
        nonce: &[u8],
        ciphertext: &[u8],
        key: &[u8],
        aad: &[u8],
    ) -> Result<Vec<u8>, CryptoError> {
        // Critical-path span: field decryption.
        #[cfg(feature = "tracing")]
        let decrypt_span = tracing::info_span!("confers.decrypt");
        #[cfg(feature = "tracing")]
        let _decrypt_guard = decrypt_span.enter();
        if key.len() != 32 {
            return Err(CryptoError::InvalidKeyLength(key.len()));
        }

        if nonce.len() != NONCE_SIZE {
            return Err(CryptoError::DecryptionFailed);
        }

        let cipher =
            XChaCha20Poly1305::new_from_slice(key).map_err(|_| CryptoError::DecryptionFailed)?;

        let nonce = XNonce::try_from(nonce).map_err(|_| CryptoError::DecryptionFailed)?;

        cipher
            .decrypt(
                &nonce,
                Payload {
                    msg: ciphertext,
                    aad,
                },
            )
            .map_err(|_| CryptoError::DecryptionFailed)
    }
}

impl Default for XChaCha20Crypto {
    fn default() -> Self {
        Self::new()
    }
}

pub fn derive_field_key(
    master_key: &[u8],
    field_path: &str,
    key_version: &str,
) -> Result<[u8; 32], CryptoError> {
    let hk = Hkdf::<Sha256>::new(None, master_key);
    // NUL cannot appear in key versions or field paths: a `:`-style separator
    // would let ("v1", "a:b") and ("v1:a", "b") collide into the same info
    // string, deriving identical keys for distinct contexts.
    let info = format!("{key_version}\x00{field_path}");
    let mut field_key = [0u8; 32];

    hk.expand(info.as_bytes(), &mut field_key)
        .map_err(|_| CryptoError::InvalidKeyLength(32))?;

    Ok(field_key)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fixed 32-byte key used by tests that need a deterministic master key.
    const TEST_KEY: [u8; 32] = *b"0123456789abcdef0123456789abcdef"; // pragma: allowlist secret

    #[test]
    #[serial_test::serial]
    fn test_decrypt_error_metric_recorded() {
        // Critical-path metric: a failing decryption must increment the
        // confers_secret_decrypt_errors_total counter on the installed
        // backend (and remain a silent no-op without one).
        crate::metrics::clear_metrics_backend();
        let recorder = crate::metrics::test_support::RecordingBackend::installed();

        let cipher = XChaCha20Crypto::new();
        // Wrong nonce length → DecryptionFailed.
        let result = cipher.decrypt(&[0u8; 5], b"data", &TEST_KEY);
        assert!(result.is_err());
        assert!(
            recorder.counter_count(crate::metrics::names::SECRET_DECRYPT_ERRORS_TOTAL) >= 1,
            "decryption failure must be counted"
        );

        // Successful decryption must NOT count as an error.
        let (nonce, ciphertext) = cipher.encrypt(b"ok", &TEST_KEY).unwrap();
        cipher.decrypt(&nonce, &ciphertext, &TEST_KEY).unwrap();

        crate::metrics::clear_metrics_backend();
    }

    #[test]
    fn test_new_returns_instance() {
        let _ = XChaCha20Crypto::new();
    }

    #[test]
    fn test_derive_field_key_separates_contexts() {
        // The NUL info separator must keep ("v1", "a:b") and ("v1:a", "b")
        // distinct: a shared info string would derive the same field key.
        let a = derive_field_key(&TEST_KEY, "a:b", "v1").unwrap();
        let b = derive_field_key(&TEST_KEY, "b", "v1:a").unwrap();
        assert_ne!(a, b, "distinct (version, path) contexts must not collide");
    }

    #[test]
    fn test_default_equals_new() {
        let _: XChaCha20Crypto = Default::default();
    }

    #[test]
    fn test_encrypt_decrypt_round_trip() {
        let cipher = XChaCha20Crypto::new();
        let plaintext = b"the quick brown fox jumps over the lazy dog";
        let (nonce, ciphertext) = cipher.encrypt(plaintext, &TEST_KEY).expect("encrypt");
        let decrypted = cipher
            .decrypt(&nonce, &ciphertext, &TEST_KEY)
            .expect("decrypt");
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_encrypt_empty_plaintext_round_trip() {
        let cipher = XChaCha20Crypto::new();
        let (nonce, ciphertext) = cipher.encrypt(b"", &TEST_KEY).expect("encrypt");
        // Even for empty plaintext, XChaCha20-Poly1305 produces a 16-byte auth tag.
        assert!(!ciphertext.is_empty(), "ciphertext should contain auth tag");
        let decrypted = cipher
            .decrypt(&nonce, &ciphertext, &TEST_KEY)
            .expect("decrypt");
        assert!(decrypted.is_empty());
    }

    #[test]
    fn test_encrypt_large_plaintext_round_trip() {
        let cipher = XChaCha20Crypto::new();
        let plaintext = vec![0xABu8; 100_000];
        let (nonce, ciphertext) = cipher.encrypt(&plaintext, &TEST_KEY).expect("encrypt");
        let decrypted = cipher
            .decrypt(&nonce, &ciphertext, &TEST_KEY)
            .expect("decrypt");
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_encrypt_nonce_is_24_bytes_and_unique() {
        let cipher = XChaCha20Crypto::new();
        let (nonce1, _) = cipher.encrypt(b"a", &TEST_KEY).unwrap();
        let (nonce2, _) = cipher.encrypt(b"a", &TEST_KEY).unwrap();
        assert_eq!(nonce1.len(), NONCE_SIZE);
        assert_eq!(nonce2.len(), NONCE_SIZE);
        // Random nonce generator should produce distinct nonces for the same plaintext.
        assert_ne!(nonce1, nonce2, "nonces must be random and unique");
    }

    #[test]
    fn test_encrypt_ciphertext_differs_from_plaintext() {
        let cipher = XChaCha20Crypto::new();
        let plaintext = b"hello world hello world hello wor";
        let (_, ciphertext) = cipher.encrypt(plaintext, &TEST_KEY).unwrap();
        // Ciphertext length = plaintext length + 16 (Poly1305 tag)
        assert_eq!(ciphertext.len(), plaintext.len() + 16);
        // Ciphertext must not contain the plaintext verbatim
        assert!(!ciphertext.windows(plaintext.len()).any(|w| w == plaintext));
    }

    #[test]
    fn test_encrypt_rejects_short_key() {
        let cipher = XChaCha20Crypto::new();
        let short_key = b"too short"; // pragma: allowlist secret
        let err = cipher.encrypt(b"data", short_key).unwrap_err();
        assert!(matches!(err, CryptoError::InvalidKeyLength(9)));
    }

    #[test]
    fn test_encrypt_rejects_long_key() {
        let cipher = XChaCha20Crypto::new();
        let long_key = [0u8; 64]; // pragma: allowlist secret
        let err = cipher.encrypt(b"data", &long_key).unwrap_err();
        assert!(matches!(err, CryptoError::InvalidKeyLength(64)));
    }

    #[test]
    fn test_decrypt_rejects_short_key() {
        let cipher = XChaCha20Crypto::new();
        let err = cipher
            .decrypt(&[0u8; NONCE_SIZE], b"ciphertext", b"short")
            .unwrap_err();
        assert!(matches!(err, CryptoError::InvalidKeyLength(_)));
    }

    #[test]
    fn test_decrypt_rejects_long_key() {
        let cipher = XChaCha20Crypto::new();
        let long_key = [0u8; 64]; // pragma: allowlist secret
        let err = cipher
            .decrypt(&[0u8; NONCE_SIZE], b"ciphertext", &long_key)
            .unwrap_err();
        assert!(matches!(err, CryptoError::InvalidKeyLength(64)));
    }

    #[test]
    fn test_decrypt_rejects_invalid_nonce_length() {
        let cipher = XChaCha20Crypto::new();
        let bad_nonce = [0u8; NONCE_SIZE - 1]; // 23 bytes instead of 24
        let err = cipher
            .decrypt(&bad_nonce, b"ciphertext", &TEST_KEY)
            .unwrap_err();
        assert!(matches!(err, CryptoError::DecryptionFailed));
    }

    #[test]
    fn test_decrypt_rejects_too_long_nonce() {
        let cipher = XChaCha20Crypto::new();
        let bad_nonce = [0u8; NONCE_SIZE + 1]; // 25 bytes
        let err = cipher
            .decrypt(&bad_nonce, b"ciphertext", &TEST_KEY)
            .unwrap_err();
        assert!(matches!(err, CryptoError::DecryptionFailed));
    }

    #[test]
    fn test_decrypt_with_wrong_key_fails() {
        let cipher = XChaCha20Crypto::new();
        let wrong_key = *b"abcdef0123456789abcdef0123456789"; // pragma: allowlist secret
        let (nonce, ciphertext) = cipher.encrypt(b"secret data", &TEST_KEY).unwrap();
        let err = cipher.decrypt(&nonce, &ciphertext, &wrong_key).unwrap_err();
        assert!(matches!(err, CryptoError::DecryptionFailed));
    }

    #[test]
    fn test_decrypt_with_tampered_ciphertext_fails() {
        let cipher = XChaCha20Crypto::new();
        let (nonce, mut ciphertext) = cipher.encrypt(b"secret data", &TEST_KEY).unwrap();
        // Flip a bit in the ciphertext to break authentication
        ciphertext[0] ^= 0xFF;
        let err = cipher.decrypt(&nonce, &ciphertext, &TEST_KEY).unwrap_err();
        assert!(matches!(err, CryptoError::DecryptionFailed));
    }

    #[test]
    fn test_decrypt_with_wrong_nonce_fails() {
        let cipher = XChaCha20Crypto::new();
        let (_, ciphertext) = cipher.encrypt(b"secret data", &TEST_KEY).unwrap();
        let wrong_nonce = [0u8; NONCE_SIZE]; // all zeros, almost certainly different
        let err = cipher
            .decrypt(&wrong_nonce, &ciphertext, &TEST_KEY)
            .unwrap_err();
        assert!(matches!(err, CryptoError::DecryptionFailed));
    }

    #[test]
    fn test_decrypt_truncated_ciphertext_fails() {
        let cipher = XChaCha20Crypto::new();
        let (nonce, mut ciphertext) = cipher.encrypt(b"secret data", &TEST_KEY).unwrap();
        // Truncate to remove the auth tag
        ciphertext.truncate(ciphertext.len().saturating_sub(16));
        let err = cipher.decrypt(&nonce, &ciphertext, &TEST_KEY).unwrap_err();
        assert!(matches!(err, CryptoError::DecryptionFailed));
    }

    #[test]
    fn test_derive_field_key_returns_32_bytes() {
        let key = derive_field_key(&TEST_KEY, "db.password", "v1").expect("derive");
        assert_eq!(key.len(), 32);
    }

    #[test]
    fn test_derive_field_key_is_deterministic() {
        let k1 = derive_field_key(&TEST_KEY, "db.password", "v1").unwrap();
        let k2 = derive_field_key(&TEST_KEY, "db.password", "v1").unwrap();
        assert_eq!(k1, k2, "same inputs must produce same key");
    }

    #[test]
    fn test_derive_field_key_differs_by_field_path() {
        let k1 = derive_field_key(&TEST_KEY, "db.password", "v1").unwrap();
        let k2 = derive_field_key(&TEST_KEY, "db.user", "v1").unwrap();
        assert_ne!(k1, k2, "different field paths must produce different keys");
    }

    #[test]
    fn test_derive_field_key_differs_by_version() {
        let k1 = derive_field_key(&TEST_KEY, "db.password", "v1").unwrap();
        let k2 = derive_field_key(&TEST_KEY, "db.password", "v2").unwrap();
        assert_ne!(k1, k2, "different key versions must produce different keys");
    }

    #[test]
    fn test_derive_field_key_differs_by_master_key() {
        let other_key = *b"fedcba9876543210fedcba9876543210"; // pragma: allowlist secret
        let k1 = derive_field_key(&TEST_KEY, "db.password", "v1").unwrap();
        let k2 = derive_field_key(&other_key, "db.password", "v1").unwrap();
        assert_ne!(k1, k2, "different master keys must produce different keys");
    }

    #[test]
    fn test_derive_field_key_can_encrypt_decrypt() {
        // The derived field key is a valid 32-byte XChaCha20-Poly1305 key.
        let field_key = derive_field_key(&TEST_KEY, "api.token", "v1").unwrap();
        let cipher = XChaCha20Crypto::new();
        let plaintext = b"super secret field value";
        let (nonce, ciphertext) = cipher.encrypt(plaintext, &field_key).unwrap();
        let decrypted = cipher.decrypt(&nonce, &ciphertext, &field_key).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    #[allow(deprecated)]
    fn test_crypto_error_display_messages() {
        assert_eq!(
            CryptoError::EncryptionFailed.to_string(),
            "encryption failed"
        );
        assert_eq!(
            CryptoError::DecryptionFailed.to_string(),
            "decryption failed"
        );
        assert_eq!(
            CryptoError::InvalidKeyLength(7).to_string(),
            "invalid key length: expected exactly 32 bytes for XChaCha20-Poly1305, got 7 bytes"
        );
        assert_eq!(
            CryptoError::LegacyDecryptionFailed.to_string(),
            "legacy decryption failed (AES-256-GCM)"
        );
    }

    #[test]
    fn test_non_zero_plaintext_round_trip_with_unicode() {
        let cipher = XChaCha20Crypto::new();
        let plaintext = "你好，世界！🌍".as_bytes();
        let (nonce, ciphertext) = cipher.encrypt(plaintext, &TEST_KEY).unwrap();
        let decrypted = cipher.decrypt(&nonce, &ciphertext, &TEST_KEY).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_encrypt_with_aad_round_trip() {
        let cipher = XChaCha20Crypto::new();
        let aad = b"db.password:v1".as_slice();
        let (nonce, ciphertext) = cipher
            .encrypt_with_aad(b"secret data", &TEST_KEY, aad)
            .expect("encrypt with aad");
        let decrypted = cipher
            .decrypt_with_aad(&nonce, &ciphertext, &TEST_KEY, aad)
            .expect("decrypt with aad");
        assert_eq!(decrypted, b"secret data");
    }

    #[test]
    fn test_decrypt_with_wrong_aad_fails() {
        let cipher = XChaCha20Crypto::new();
        let (nonce, ciphertext) = cipher
            .encrypt_with_aad(b"secret data", &TEST_KEY, b"expected-context")
            .unwrap();
        let err = cipher
            .decrypt_with_aad(&nonce, &ciphertext, &TEST_KEY, b"attacker-context")
            .unwrap_err();
        assert!(matches!(err, CryptoError::DecryptionFailed));
    }

    #[test]
    fn test_decrypt_with_missing_aad_fails() {
        let cipher = XChaCha20Crypto::new();
        let (nonce, ciphertext) = cipher
            .encrypt_with_aad(b"secret data", &TEST_KEY, b"context")
            .unwrap();
        // Plain decrypt() authenticates against an empty AAD and must reject
        // ciphertext that was bound to a non-empty AAD.
        let err = cipher.decrypt(&nonce, &ciphertext, &TEST_KEY).unwrap_err();
        assert!(matches!(err, CryptoError::DecryptionFailed));
    }

    #[test]
    fn test_encrypt_and_empty_aad_variants_are_interoperable() {
        let cipher = XChaCha20Crypto::new();
        // The legacy encrypt() output decrypts through the explicit AAD path
        // with an empty AAD...
        let (nonce, ciphertext) = cipher.encrypt(b"secret data", &TEST_KEY).unwrap();
        let decrypted = cipher
            .decrypt_with_aad(&nonce, &ciphertext, &TEST_KEY, &[])
            .expect("empty AAD must match the legacy path");
        assert_eq!(decrypted, b"secret data");

        // ...and the empty-AAD encrypt path decrypts through plain decrypt().
        let (nonce, ciphertext) = cipher
            .encrypt_with_aad(b"secret data", &TEST_KEY, &[])
            .unwrap();
        let decrypted = cipher.decrypt(&nonce, &ciphertext, &TEST_KEY).unwrap();
        assert_eq!(decrypted, b"secret data");
    }

    #[test]
    fn test_encrypt_with_aad_rejects_short_key() {
        let cipher = XChaCha20Crypto::new();
        let short_key = b"too short"; // pragma: allowlist secret
        let err = cipher
            .encrypt_with_aad(b"data", short_key, b"aad")
            .unwrap_err();
        assert!(matches!(err, CryptoError::InvalidKeyLength(9)));
    }

    #[test]
    fn test_decrypt_with_aad_rejects_invalid_nonce_length() {
        let cipher = XChaCha20Crypto::new();
        let bad_nonce = [0u8; NONCE_SIZE - 1];
        let err = cipher
            .decrypt_with_aad(&bad_nonce, b"ciphertext", &TEST_KEY, b"aad")
            .unwrap_err();
        assert!(matches!(err, CryptoError::DecryptionFailed));
    }
}
