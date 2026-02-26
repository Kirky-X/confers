use crate::secret::{CryptoError, SecretBytes};

pub trait SecretKeyProvider: Send + Sync {
    fn get_key(&self) -> Result<SecretBytes, CryptoError>;
    fn provider_type(&self) -> &'static str;
}

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
        let key = std::env::var(&self.env_var).map_err(|_| CryptoError::InvalidKeyLength)?;

        if key.len() < 32 {
            return Err(CryptoError::InvalidKeyLength);
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
        let env_var = self.env_var.ok_or(CryptoError::InvalidKeyLength)?;
        Ok(EnvKeyProvider::new(env_var))
    }
}
