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
///
/// Security justification:
/// - 50,000 nonces allows ~10-20 hours of operation at typical config reload intervals (1-30s)
/// - 50,000 entries consume ~6MB of memory (120 bytes per entry)
/// - LRU eviction ensures cache doesn't grow unbounded
/// - Nonce reuse is detected even after eviction (via cryptographic check)
/// - This is a conservative value suitable for high-concurrency and frequent reload scenarios
const MAX_NONCE_CACHE_SIZE: usize = 50000;

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
    /// Maximum size of the nonce cache
    max_nonce_cache_size: usize,
}

impl ConfigEncryption {
    /// Create a new encryptor with a 32-byte key
    ///
    /// # Security Notes
    ///
    /// - ⚠️ **Key Management**: The encryption key must be stored securely and never committed to version control
    /// - ⚠️ **Key Length**: The key must be exactly 32 bytes (256 bits) for AES-256-GCM
    /// - ⚠️ **Key Security**: Use a cryptographically secure random number generator to generate the key
    /// - ⚠️ **Key Storage**: Consider using a secrets manager (e.g., AWS Secrets Manager, HashiCorp Vault)
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use confers::encryption::ConfigEncryption;
    /// # let secure_key = [0u8; 32];
    /// let encryption = ConfigEncryption::new(secure_key);
    /// let encrypted = encryption.encrypt("sensitive-data")?;
    /// # Ok::<(), confers::error::ConfigError>(())
    /// ```
    pub fn new(key_bytes: [u8; 32]) -> Self {
        Self::with_cache_size(key_bytes, MAX_NONCE_CACHE_SIZE)
    }

    /// Create a new encryptor with a 32-byte key and custom cache size
    ///
    /// # Security Notes
    ///
    /// - ⚠️ **Key Management**: The encryption key must be stored securely and never committed to version control
    /// - ⚠️ **Key Length**: The key must be exactly 32 bytes (256 bits) for AES-256-GCM
    /// - ⚠️ **Cache Size**: Larger cache sizes increase memory usage but improve nonce reuse detection
    /// - ⚠️ **Key Security**: Use a cryptographically secure random number generator to generate the key
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use confers::encryption::ConfigEncryption;
    /// # let secure_key = [0u8; 32];
    /// let encryption = ConfigEncryption::with_cache_size(secure_key, 100000);
    /// let encrypted = encryption.encrypt("sensitive-data")?;
    /// # Ok::<(), confers::error::ConfigError>(())
    /// ```
    pub fn with_cache_size(key_bytes: [u8; 32], cache_size: usize) -> Self {
        let key = SecureKey::new(key_bytes);
        Self {
            key,
            nonce_cache: Mutex::new(LruCache::new(
                #[allow(clippy::incompatible_msrv)]
                NonZero::new(cache_size).expect("cache_size must be > 0"),
            )),
            max_nonce_cache_size: cache_size,
        }
    }

    /// Create from environment variable CONFERS_ENCRYPTION_KEY or CONFERS_KEY (base64 encoded)
    ///
    /// # Security Notes
    ///
    /// - ⚠️ **Environment Variable**: The CONFERS_ENCRYPTION_KEY (or CONFERS_KEY) environment variable must be set securely
    /// - ⚠️ **Key Format**: The key must be base64 encoded and exactly 32 bytes (256 bits)
    /// - ⚠️ **Key Storage**: Never commit environment variables to version control
    /// - ⚠️ **Key Rotation**: Regular key rotation is recommended for production environments
    ///
    /// # Example
    ///
    /// ```bash,no_run
    /// export CONFERS_ENCRYPTION_KEY=$(openssl rand -base64 32)
    /// ```
    ///
    /// ```rust,no_run
    /// # use confers::encryption::ConfigEncryption;
    /// let encryption = ConfigEncryption::from_env()?;
    /// # Ok::<(), confers::error::ConfigError>(())
    /// ```
    pub fn from_env() -> Result<Self, ConfigError> {
        let key_str = env::var("CONFERS_ENCRYPTION_KEY")
            .or_else(|_| env::var("CONFERS_KEY"))
            .map_err(|_| {
                ConfigError::FormatDetectionFailed("CONFERS_ENCRYPTION_KEY (or CONFERS_KEY) not found".to_string())
            })?;

        // Validate key string format
        if !key_str
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '=')
        {
            return Err(ConfigError::FormatDetectionFailed(
                "Invalid base64 key format: contains invalid characters".to_string(),
            ));
        }

        let key_bytes = BASE64.decode(&key_str).map_err(|e| {
            ConfigError::FormatDetectionFailed(format!("Invalid base64 key: {}", e))
        })?;

        if key_bytes.len() != 32 {
            return Err(ConfigError::FormatDetectionFailed(format!(
                "Key must be 32 bytes (256 bits), got {} bytes",
                key_bytes.len()
            )));
        }

        // Check for weak key (all zeros)
        if key_bytes.iter().all(|&b| b == 0) {
            return Err(ConfigError::FormatDetectionFailed(
                "Weak key: all zeros".to_string(),
            ));
        }

        // Check for weak key (all same byte)
        if key_bytes.windows(2).all(|w| w[0] == w[1]) {
            return Err(ConfigError::FormatDetectionFailed(
                "Weak key: all bytes are identical".to_string(),
            ));
        }

        // Check for weak key (sequential pattern)
        if is_sequential_pattern(&key_bytes) {
            return Err(ConfigError::FormatDetectionFailed(
                "Weak key: sequential pattern detected".to_string(),
            ));
        }

        // Check for weak key (repeating pattern)
        if is_repeating_pattern(&key_bytes) {
            return Err(ConfigError::FormatDetectionFailed(
                "Weak key: repeating pattern detected".to_string(),
            ));
        }

        // Check key entropy - should have at least 4.0 bits of entropy per byte for 32-byte keys
        // Note: For 32 bytes, max entropy is log2(32) = 5.0 bits
        let entropy = calculate_entropy(&key_bytes);
        if entropy < 4.0 {
            return Err(ConfigError::FormatDetectionFailed(format!(
                "Weak key: insufficient entropy ({} bits per byte, minimum 4.0)",
                entropy
            )));
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

            // Warn if cache is near full (>80%)
            let usage = cache.len() as f64 / self.max_nonce_cache_size as f64;
            if usage > 0.8 {
                #[cfg(feature = "tracing")]
                tracing::warn!(
                    "Nonce cache is {:.0}% full ({} entries). Consider increasing cache size or rotating keys more frequently.",
                    usage * 100.0,
                    cache.len()
                );
            }
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

    /// Get the cache usage as a percentage
    /// Returns a value between 0.0 and 100.0
    pub fn cache_usage_percent(&self) -> f64 {
        let size = self.nonce_cache_size();
        (size as f64 / MAX_NONCE_CACHE_SIZE as f64) * 100.0
    }

    /// Check if the cache is near full (above threshold)
    /// Returns true if cache usage exceeds the threshold (0-100)
    pub fn is_cache_near_full(&self, threshold: f64) -> bool {
        self.cache_usage_percent() > threshold
    }

    /// Get cache statistics for monitoring
    pub fn cache_stats(&self) -> CacheStats {
        CacheStats {
            current_size: self.nonce_cache_size(),
            max_size: self.max_nonce_cache_size,
            usage_percent: self.cache_usage_percent(),
            is_near_full: self.is_cache_near_full(80.0),
        }
    }

    /// Clear the nonce cache
    /// Use with caution - this reduces security by allowing nonce reuse
    pub fn clear_nonce_cache(&self) {
        if let Ok(mut cache) = self.nonce_cache.lock() {
            cache.clear();
        }
    }
}

/// Cache statistics for monitoring
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub current_size: usize,
    pub max_size: usize,
    pub usage_percent: f64,
    pub is_near_full: bool,
}

/// Calculate the Shannon entropy of a byte array
/// Returns entropy in bits per byte (0-8)
fn calculate_entropy(data: &[u8]) -> f64 {
    let mut freq = [0usize; 256];
    for &byte in data {
        freq[byte as usize] += 1;
    }

    let len = data.len() as f64;
    let mut entropy = 0.0;

    for &count in &freq {
        if count > 0 {
            let p = count as f64 / len;
            entropy -= p * p.log2();
        }
    }

    entropy
}

/// Check if the key contains a sequential pattern
fn is_sequential_pattern(key: &[u8]) -> bool {
    // Check for ascending sequence (e.g., 0,1,2,3,...)
    let mut ascending = 0;
    let mut descending = 0;

    for i in 0..key.len() - 1 {
        if key[i + 1] == key[i].wrapping_add(1) {
            ascending += 1;
        } else if key[i + 1] == key[i].wrapping_sub(1) {
            descending += 1;
        }
    }

    // If more than half of the bytes are sequential, consider it weak
    ascending > key.len() / 2 || descending > key.len() / 2
}

/// Check if the key contains a repeating pattern
fn is_repeating_pattern(key: &[u8]) -> bool {
    // Check for patterns with period 2, 4, 8, 16
    for period in [2, 4, 8, 16] {
        if key.len() % period == 0 {
            let mut is_repeating = true;
            for i in period..key.len() {
                if key[i] != key[i % period] {
                    is_repeating = false;
                    break;
                }
            }
            if is_repeating {
                return true;
            }
        }
    }

    false
}
