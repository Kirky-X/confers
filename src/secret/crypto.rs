use chacha20poly1305::aead::rand_core::RngCore;
use chacha20poly1305::{
    aead::{Aead, KeyInit, OsRng},
    ChaCha20Poly1305, Nonce,
};
use hkdf::Hkdf;
use sha2::Sha256;

#[derive(Debug, thiserror::Error)]
pub enum CryptoError {
    #[error("encryption failed")]
    EncryptionFailed,
    #[error("decryption failed")]
    DecryptionFailed,
    #[error("invalid key length")]
    InvalidKeyLength,
}

pub struct XChaCha20Crypto;

impl XChaCha20Crypto {
    pub fn new() -> Self {
        Self
    }

    pub fn encrypt(&self, plaintext: &[u8], key: &[u8]) -> Result<(Vec<u8>, Vec<u8>), CryptoError> {
        if key.len() != 32 {
            return Err(CryptoError::InvalidKeyLength);
        }

        let cipher =
            ChaCha20Poly1305::new_from_slice(key).map_err(|_| CryptoError::EncryptionFailed)?;

        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

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
            return Err(CryptoError::InvalidKeyLength);
        }

        let cipher =
            ChaCha20Poly1305::new_from_slice(key).map_err(|_| CryptoError::DecryptionFailed)?;

        let nonce = Nonce::from_slice(nonce);

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
        .map_err(|_| CryptoError::InvalidKeyLength)?;

    Ok(field_key)
}
