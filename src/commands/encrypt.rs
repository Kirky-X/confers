use crate::encryption::ConfigEncryption;
use crate::error::ConfigError;

pub struct EncryptCommand;

impl EncryptCommand {
    pub fn execute(value: &str, key: Option<&String>) -> Result<(), ConfigError> {
        let encryptor = if let Some(k) = key {
            // Parse key from arg
            use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
            let key_bytes = BASE64.decode(k).map_err(|e| {
                ConfigError::FormatDetectionFailed(format!("Invalid base64 key: {}", e))
            })?;

            if key_bytes.len() != 32 {
                return Err(ConfigError::FormatDetectionFailed(
                    "Key must be 32 bytes".to_string(),
                ));
            }

            let mut key_arr = [0u8; 32];
            key_arr.copy_from_slice(&key_bytes);
            ConfigEncryption::new(key_arr)
        } else {
            ConfigEncryption::from_env()?
        };

        let encrypted = encryptor.encrypt(value)?;
        println!("{}", encrypted);

        Ok(())
    }
}
