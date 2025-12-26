use crate::error::ConfigError;
use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use std::env;

pub struct ConfigEncryption {
    key: Key<Aes256Gcm>,
}

impl ConfigEncryption {
    /// Create a new encryptor with a 32-byte key
    pub fn new(key_bytes: [u8; 32]) -> Self {
        let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
        Self { key: *key }
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
    pub fn encrypt(&self, plaintext: &str) -> Result<String, ConfigError> {
        let cipher = Aes256Gcm::new(&self.key);
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng); // 96-bits; unique per message

        let ciphertext = cipher
            .encrypt(&nonce, plaintext.as_bytes())
            .map_err(|e| ConfigError::FormatDetectionFailed(format!("Encryption error: {}", e)))?;

        let nonce_b64 = BASE64.encode(nonce);
        let ct_b64 = BASE64.encode(ciphertext);

        Ok(format!("enc:AES256GCM:{}:{}", nonce_b64, ct_b64))
    }

    /// Decrypt a formatted encrypted string
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

        let ciphertext = BASE64.decode(ct_b64).map_err(|e| {
            ConfigError::FormatDetectionFailed(format!("Invalid ciphertext base64: {}", e))
        })?;

        let nonce = Nonce::from_slice(&nonce_bytes);
        let cipher = Aes256Gcm::new(&self.key);

        let plaintext_bytes = cipher
            .decrypt(nonce, ciphertext.as_ref())
            .map_err(|e| ConfigError::FormatDetectionFailed(format!("Decryption error: {}", e)))?;

        let plaintext = String::from_utf8(plaintext_bytes)
            .map_err(|e| ConfigError::FormatDetectionFailed(format!("Invalid UTF-8: {}", e)))?;

        Ok(plaintext)
    }
}
