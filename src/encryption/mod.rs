// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

use crate::error::ConfigError;
use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use lru::LruCache;
use std::env;
use std::num::NonZero;
use std::sync::Mutex;
use zeroize::ZeroizeOnDrop;

/// Maximum number of nonces to track for reuse detection
/// This balances security (detecting reuse) with memory usage
const MAX_NONCE_CACHE_SIZE: usize = 10000;

/// Secure key container that automatically zeroes memory on drop
#[derive(ZeroizeOnDrop)]
pub struct SecureKey([u8; 32]);

impl SecureKey {
    /// Create a new secure key from bytes
    pub fn new(key_bytes: [u8; 32]) -> Self {
        Self(key_bytes)
    }

    /// Get reference to the key bytes
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Convert to AES-GCM key
    pub fn to_aes_key(&self) -> Key<Aes256Gcm> {
        *Key::<Aes256Gcm>::from_slice(&self.0)
    }
}

pub struct ConfigEncryption {
    key: SecureKey,
    /// Track used nonces to detect reuse
    /// Uses LRU cache to limit memory usage
    nonce_cache: Mutex<LruCache<Vec<u8>, ()>>,
}

impl ConfigEncryption {
    /// Create a new encryptor with a 32-byte key
    pub fn new(key_bytes: [u8; 32]) -> Self {
        let key = SecureKey::new(key_bytes);
        Self {
            key,
            nonce_cache: Mutex::new(LruCache::new(
                #[allow(clippy::incompatible_msrv)]
                NonZero::new(MAX_NONCE_CACHE_SIZE).expect("MAX_NONCE_CACHE_SIZE must be > 0"),
            )),
        }
    }

    /// Create from environment variable CONFERS_ENCRYPTION_KEY (base64 encoded)
    pub fn from_env() -> Result<Self, ConfigError> {
        let key_str = env::var("CONFERS_ENCRYPTION_KEY").map_err(|_| {
            ConfigError::FormatDetectionFailed("CONFERS_ENCRYPTION_KEY not found".to_string())
        })?;

        let key_bytes = BASE64.decode(&key_str).map_err(|e| {
            ConfigError::FormatDetectionFailed(format!("Invalid base64 key: {}", e))
        })?;

        if key_bytes.len() != 32 {
            return Err(ConfigError::FormatDetectionFailed(
                "Key must be 32 bytes (256 bits)".to_string(),
            ));
        }

        let mut key = [0u8; 32];
        key.copy_from_slice(&key_bytes);

        Ok(Self::new(key))
    }

    /// Encrypt a string value. Returns format: "enc:AES256GCM:<nonce_base64>:<ciphertext_base64>"
    ///
    /// This method automatically generates a unique nonce and checks for reuse
    pub fn encrypt(&self, plaintext: &str) -> Result<String, ConfigError> {
        let cipher = Aes256Gcm::new(&self.key.to_aes_key());
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng); // 96-bits; unique per message

        // Convert nonce to Vec<u8> for caching
        let nonce_bytes: Vec<u8> = nonce.to_vec();

        // Check for nonce reuse
        {
            let mut cache = self
                .nonce_cache
                .lock()
                .map_err(|_| ConfigError::RuntimeError("Nonce cache lock poisoned".to_string()))?;

            if cache.contains(&nonce_bytes) {
                return Err(ConfigError::FormatDetectionFailed(
                    "Nonce reuse detected - cryptographic attack prevented".to_string(),
                ));
            }

            // Store this nonce
            cache.put(nonce_bytes.clone(), ());
        }

        let ciphertext = cipher
            .encrypt(&nonce, plaintext.as_bytes())
            .map_err(|e| ConfigError::FormatDetectionFailed(format!("Encryption error: {}", e)))?;

        let nonce_b64 = BASE64.encode(nonce.as_slice());
        let ct_b64 = BASE64.encode(ciphertext);

        Ok(format!("enc:AES256GCM:{}:{}", nonce_b64, ct_b64))
    }

    /// Decrypt a formatted encrypted string
    ///
    /// This method validates the nonce format and checks for reuse
    pub fn decrypt(&self, encrypted_value: &str) -> Result<String, ConfigError> {
        if !encrypted_value.starts_with("enc:AES256GCM:") {
            // Backward compatibility check for AES256 (CBC) could be added here if needed,
            // but for now we enforce GCM or plaintext.
            return Ok(encrypted_value.to_string());
        }

        let parts: Vec<&str> = encrypted_value.split(':').collect();
        if parts.len() != 4 {
            return Err(ConfigError::FormatDetectionFailed(
                "Invalid encrypted value format".to_string(),
            ));
        }

        let nonce_b64 = parts[2];
        let ct_b64 = parts[3];

        let nonce_bytes = BASE64.decode(nonce_b64).map_err(|e| {
            ConfigError::FormatDetectionFailed(format!("Invalid Nonce base64: {}", e))
        })?;

        // Check for nonce reuse during decryption as well
        {
            let cache = self
                .nonce_cache
                .lock()
                .map_err(|_| ConfigError::RuntimeError("Nonce cache lock poisoned".to_string()))?;

            if cache.contains(&nonce_bytes) {
                return Err(ConfigError::FormatDetectionFailed(
                    "Nonce reuse detected - cryptographic attack prevented".to_string(),
                ));
            }
        }

        let ciphertext = BASE64.decode(ct_b64).map_err(|e| {
            ConfigError::FormatDetectionFailed(format!("Invalid ciphertext base64: {}", e))
        })?;

        let nonce = Nonce::from_slice(&nonce_bytes);
        let cipher = Aes256Gcm::new(&self.key.to_aes_key());

        let plaintext_bytes = cipher
            .decrypt(nonce, ciphertext.as_ref())
            .map_err(|e| ConfigError::FormatDetectionFailed(format!("Decryption error: {}", e)))?;

        let plaintext = String::from_utf8(plaintext_bytes)
            .map_err(|e| ConfigError::FormatDetectionFailed(format!("Invalid UTF-8: {}", e)))?;

        Ok(plaintext)
    }

    /// Get the current size of the nonce cache
    /// Useful for monitoring and debugging
    pub fn nonce_cache_size(&self) -> usize {
        self.nonce_cache
            .lock()
            .map(|cache| cache.len())
            .unwrap_or(0)
    }

    /// Clear the nonce cache
    /// Use with caution - this reduces security by allowing nonce reuse
    pub fn clear_nonce_cache(&self) {
        if let Ok(mut cache) = self.nonce_cache.lock() {
            cache.clear();
        }
    }
}
