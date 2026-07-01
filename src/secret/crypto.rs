use chacha20poly1305::aead::rand_core::RngCore;
use chacha20poly1305::{
    aead::{Aead, KeyInit, OsRng},
    XChaCha20Poly1305, XNonce,
};
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
    #[error("legacy decryption failed (AES-256-GCM)")]
    LegacyDecryptionFailed,
}

pub const NONCE_SIZE: usize = 24;

pub struct XChaCha20Crypto;

impl XChaCha20Crypto {
    pub fn new() -> Self {
        Self
    }

    pub fn encrypt(&self, plaintext: &[u8], key: &[u8]) -> Result<(Vec<u8>, Vec<u8>), CryptoError> {
        if key.len() != 32 {
            return Err(CryptoError::InvalidKeyLength(key.len()));
        }

        let cipher =
            XChaCha20Poly1305::new_from_slice(key).map_err(|_| CryptoError::EncryptionFailed)?;

        let mut nonce_bytes = [0u8; NONCE_SIZE];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = XNonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, plaintext)
            .map_err(|_| CryptoError::EncryptionFailed)?;

        Ok((nonce_bytes.to_vec(), ciphertext))
    }

    pub fn decrypt(
        &self,
        nonce: &[u8],
        ciphertext: &[u8],
        key: &[u8],
    ) -> Result<Vec<u8>, CryptoError> {
        if key.len() != 32 {
            return Err(CryptoError::InvalidKeyLength(key.len()));
        }

        if nonce.len() != NONCE_SIZE {
            return Err(CryptoError::DecryptionFailed);
        }

        let cipher =
            XChaCha20Poly1305::new_from_slice(key).map_err(|_| CryptoError::DecryptionFailed)?;

        let nonce = XNonce::from_slice(nonce);

        cipher
            .decrypt(nonce, ciphertext)
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
    let info = format!("{}:{}", key_version, field_path);
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
    fn test_new_returns_instance() {
        let _ = XChaCha20Crypto::new();
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
}
