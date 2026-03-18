//! Key provider implementations for secret management.
//!
//! This module provides various ways to supply encryption keys to the confers library.
//! All key providers are designed to work with XChaCha20-Poly1305 encryption, which
//! requires exactly 32-byte keys.
//!
//! # Key Generation
//!
//! To generate a secure 32-byte key, you can use:
//!
//! ```bash
//! # Generate a random 32-byte key (base64 encoded for text storage)
//! openssl rand -base64 32
//!
//! # Or generate raw bytes (32 bytes = 256 bits)
//! openssl rand 32 | xxd -p -c 32
//! ```
//!
//! # Security Notes
//!
//! - Never hardcode keys in source code
//! - Use environment variables, secrets managers (Vault, AWS Secrets Manager), or
//!   secure file storage with restricted permissions
//! - Rotate keys regularly following your security policy
//! - Ensure key storage has appropriate access controls

use crate::secret::{CryptoError, SecretBytes};

pub trait SecretKeyProvider: Send + Sync {
    fn get_key(&self) -> Result<SecretBytes, CryptoError>;
    fn provider_type(&self) -> &'static str;
}

#[derive(Debug)]
pub struct EnvKeyProvider {
    env_var: String,
}

impl EnvKeyProvider {
    pub fn new(env_var: impl Into<String>) -> Self {
        Self {
            env_var: env_var.into(),
        }
    }

    pub fn builder() -> EnvKeyProviderBuilder {
        EnvKeyProviderBuilder { env_var: None }
    }
}

impl SecretKeyProvider for EnvKeyProvider {
    fn get_key(&self) -> Result<SecretBytes, CryptoError> {
        let key = std::env::var(&self.env_var).map_err(|_| CryptoError::InvalidKeyLength(0))?;

        // XChaCha20-Poly1305 requires exactly 32 bytes key
        if key.len() != 32 {
            return Err(CryptoError::InvalidKeyLength(key.len()));
        }

        Ok(SecretBytes::new(key.into_bytes()))
    }

    fn provider_type(&self) -> &'static str {
        "env"
    }
}

pub struct EnvKeyProviderBuilder {
    env_var: Option<String>,
}

impl EnvKeyProviderBuilder {
    pub fn env_var(mut self, var: impl Into<String>) -> Self {
        self.env_var = Some(var.into());
        self
    }

    pub fn build(self) -> Result<EnvKeyProvider, CryptoError> {
        let env_var = self.env_var.ok_or(CryptoError::InvalidKeyLength(0))?;
        // Validate that the environment variable exists
        std::env::var(&env_var).map_err(|_| CryptoError::InvalidKeyLength(0))?;
        Ok(EnvKeyProvider::new(env_var))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_length_exactly_32_bytes() {
        // 设置正确的 32 字节密钥
        std::env::set_var("TEST_VALID_KEY", "12345678901234567890123456789012");

        let provider = EnvKeyProvider::new("TEST_VALID_KEY");
        let result = provider.get_key();

        assert!(result.is_ok(), "32-byte key should be accepted");
        let key = result.unwrap();
        assert_eq!(key.as_slice().len(), 32);

        std::env::remove_var("TEST_VALID_KEY");
    }

    #[test]
    fn test_key_length_31_bytes_rejected() {
        // 设置 31 字节密钥（太短）
        std::env::set_var("TEST_SHORT_KEY", "1234567890123456789012345678901");

        let provider = EnvKeyProvider::new("TEST_SHORT_KEY");
        let result = provider.get_key();

        assert!(result.is_err());
        match result.unwrap_err() {
            CryptoError::InvalidKeyLength(len) => {
                assert_eq!(len, 31, "Error should report 31 bytes");
            }
            _ => panic!("Expected InvalidKeyLength error"),
        }

        std::env::remove_var("TEST_SHORT_KEY");
    }

    #[test]
    fn test_key_length_33_bytes_rejected() {
        // 设置 33 字节密钥（太长）
        std::env::set_var("TEST_LONG_KEY", "123456789012345678901234567890123");

        let provider = EnvKeyProvider::new("TEST_LONG_KEY");
        let result = provider.get_key();

        assert!(result.is_err());
        match result.unwrap_err() {
            CryptoError::InvalidKeyLength(len) => {
                assert_eq!(len, 33, "Error should report 33 bytes");
            }
            _ => panic!("Expected InvalidKeyLength error"),
        }

        std::env::remove_var("TEST_LONG_KEY");
    }

    #[test]
    fn test_key_provider_type() {
        let provider = EnvKeyProvider::new("TEST_KEY");
        assert_eq!(provider.provider_type(), "env");
    }

    #[test]
    fn test_key_provider_builder_valid() {
        std::env::set_var("BUILDER_TEST_KEY", "12345678901234567890123456789012");

        let provider = EnvKeyProvider::builder()
            .env_var("BUILDER_TEST_KEY")
            .build()
            .unwrap();

        let result = provider.get_key();
        assert!(result.is_ok());

        std::env::remove_var("BUILDER_TEST_KEY");
    }

    #[test]
    fn test_key_provider_builder_missing_env_var() {
        std::env::remove_var("NONEXISTENT_KEY");

        let result = EnvKeyProvider::builder().env_var("NONEXISTENT_KEY").build();

        assert!(result.is_err());
    }

    #[test]
    fn test_key_provider_builder_no_env_var_set() {
        // 不设置 env_var，直接调用 build
        let result = EnvKeyProvider::builder().build();

        assert!(result.is_err());
        match result.unwrap_err() {
            CryptoError::InvalidKeyLength(0) => {}
            _ => panic!("Expected InvalidKeyLength(0) error"),
        }
    }
}
