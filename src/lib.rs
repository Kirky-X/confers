// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

pub mod audit;
#[cfg(feature = "cli")]
pub mod commands;
pub mod constants;
pub mod core;
#[cfg(feature = "encryption")]
pub mod encryption;
pub mod error;
pub mod error_helpers;
#[cfg(feature = "encryption")]
pub mod key;
pub mod providers;
pub mod schema;
pub mod security;
pub mod utils;
#[cfg(feature = "validation")]
pub mod validator;
#[cfg(feature = "validation")]
pub mod validators;
pub mod watcher;

// Re-export common items
pub use audit::Sanitize;
pub use confers_macros::Config;
pub use core::builder::{ConfigBuilder, Environment, File, FileFormat};
pub use core::{ConfigLoader, OptionalValidate};
pub use error::ConfigError;
pub use error_helpers::{OptionExt, ResultExt};
#[cfg(feature = "validation")]
pub use validator::{Validate, ValidationErrors};

// Re-export security module items for convenience
#[cfg(feature = "encryption")]
pub use security::config_injector::{ConfigInjectionError, ConfigInjector};
#[cfg(feature = "encryption")]
pub use security::error_sanitization::{ErrorSanitizer, SafeResult, SecureLogger};
#[cfg(feature = "encryption")]
pub use security::input_validation::{ConfigValidator, InputValidator};
#[cfg(feature = "encryption")]
pub use security::secure_string::{SecureString, SensitivityLevel};

// Re-export macro dependencies
#[cfg(feature = "cli")]
pub use clap;
pub use serde::{Deserialize, Serialize};
pub use serde_json;
#[cfg(feature = "validation")]
pub use validator::Validate as ValidatorValidate;

// Re-export command modules for library users
#[cfg(feature = "cli")]
pub use commands::{
    completions::CompletionsCommand,
    diff::{DiffCommand, DiffFormat, DiffOptions},
    encrypt::EncryptCommand,
    generate::GenerateCommand,
    key::KeyCommand,
    validate::{ValidateCommand, ValidateLevel},
    wizard::ConfigWizard,
};

/// Unified CLI facade for easy integration into other projects
#[cfg(feature = "cli")]
pub struct ConfersCli;

#[cfg(feature = "cli")]
impl ConfersCli {
    /// Generate configuration template
    /// 
    /// # Arguments
    /// * `output` - Optional output file path, if None prints to stdout
    /// * `level` - Template level: "minimal", "full", or "documentation"
    /// 
    /// # Examples
    /// ```
    /// use confers::ConfersCli;
    /// 
    /// // Generate to file
    /// ConfersCli::generate(Some("config.toml"), "full")?;
    /// 
    /// // Generate to stdout
    /// ConfersCli::generate(None, "minimal")?;
    /// ```
    pub fn generate(output: Option<&str>, level: &str) -> Result<(), ConfigError> {
        let output_str = output.map(|s| s.to_string());
        GenerateCommand::execute_placeholder(output_str.as_ref(), level).map(|_| ())
    }

    /// Validate configuration file
    /// 
    /// # Arguments
    /// * `config` - Path to configuration file
    /// * `level` - Validation level: "minimal", "full", or "documentation"
    /// 
    /// # Examples
    /// ```
    /// use confers::ConfersCli;
    /// 
    /// ConfersCli::validate("config.toml", "full")?;
    /// ```
    pub fn validate(config: &str, level: &str) -> Result<(), ConfigError> {
        let validate_level = ValidateLevel::parse(level);
        ValidateCommand::execute_generic(config, validate_level)
    }

    /// Encrypt a configuration value
    /// 
    /// # Arguments
    /// * `value` - Value to encrypt
    /// * `key` - Optional Base64-encoded 32-byte key, if None uses environment variable
    /// 
    /// # Returns
    /// Returns the encrypted value as a Base64-encoded string
    /// 
    /// # Examples
    /// ```
    /// use confers::ConfersCli;
    /// 
    /// // Encrypt with environment key
    /// let encrypted = ConfersCli::encrypt("secret_value", None)?;
    /// 
    /// // Encrypt with provided key
    /// let encrypted = ConfersCli::encrypt("secret_value", Some("base64_key"))?;
    /// ```
    pub fn encrypt(value: &str, key: Option<&str>) -> Result<String, ConfigError> {
        use crate::encryption::ConfigEncryption;
        
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
        Ok(encrypted)
    }

    /// Compare two configuration files
    /// 
    /// # Arguments
    /// * `file1` - Path to first configuration file
    /// * `file2` - Path to second configuration file
    /// * `format` - Optional diff format: "unified", "context", "normal", "side-by-side", or "strict"
    /// 
    /// # Examples
    /// ```
    /// use confers::ConfersCli;
    /// 
    /// ConfersCli::diff("config1.toml", "config2.toml", Some("unified"))?;
    /// ```
    pub fn diff(file1: &str, file2: &str, format: Option<&str>) -> Result<(), ConfigError> {
        let diff_format = format
            .unwrap_or("unified")
            .parse()
            .map_err(ConfigError::ParseError)?;
        let options = DiffOptions {
            format: diff_format,
            ..DiffOptions::default()
        };
        DiffCommand::execute(file1, file2, options)
    }

    /// Run interactive configuration wizard
    /// 
    /// # Arguments
    /// * `non_interactive` - If true, uses default values without prompting
    /// 
    /// # Examples
    /// ```
    /// use confers::ConfersCli;
    /// 
    /// // Interactive wizard
    /// ConfersCli::wizard(false)?;
    /// 
    /// // Non-interactive with defaults
    /// ConfersCli::wizard(true)?;
    /// ```
    pub fn wizard(non_interactive: bool) -> Result<(), ConfigError> {
        let wizard = ConfigWizard::new();
        if non_interactive {
            let values = &["", "", "", "", "", "", ""];
            let config = wizard.run_with_values(values)?;
            config.save()?;
        } else {
            let config = wizard.run()?;
            config.save()?;
        }
        Ok(())
    }

    /// Generate shell completion scripts
    /// 
    /// # Arguments
    /// * `shell` - Shell type: "bash", "fish", "zsh", "powershell", or "elvish"
    /// 
    /// # Examples
    /// ```
    /// use confers::ConfersCli;
    /// 
    /// ConfersCli::completions("bash")?;
    /// ```
    pub fn completions(shell: &str) -> Result<(), ConfigError> {
        // For library usage, we provide a simplified completion generation
        // that doesn't depend on the CLI structure from main.rs
        use clap_complete::{generate, Shell};
        use std::io;
        
        let shell_enum = match shell {
            "bash" => Shell::Bash,
            "zsh" => Shell::Zsh,
            "fish" => Shell::Fish,
            "powershell" => Shell::PowerShell,
            "elvish" => Shell::Elvish,
            _ => {
                return Err(ConfigError::FormatDetectionFailed(format!(
                    "Unsupported shell: {}",
                    shell
                )))
            }
        };

        // Create a minimal command structure for completion generation
        let mut cmd = clap::Command::new("confers")
            .about("Configuration management tool")
            .subcommand_required(true)
            .arg_required_else_help(true)
            .subcommand(
                clap::Command::new("generate")
                    .about("Generate configuration template")
                    .arg(clap::arg!(--output <FILE> "Output file path"))
                    .arg(clap::arg!(--level <LEVEL> "Template level").default_value("full"))
            )
            .subcommand(
                clap::Command::new("validate")
                    .about("Validate configuration file")
                    .arg(clap::arg!(--config <FILE> "Configuration file path").required(true))
                    .arg(clap::arg!(--level <LEVEL> "Validation level").default_value("full"))
            )
            .subcommand(
                clap::Command::new("diff")
                    .about("Compare two configuration files")
                    .arg(clap::arg!(<FILE1> "First file").required(true))
                    .arg(clap::arg!(<FILE2> "Second file").required(true))
                    .arg(clap::arg!(--style <STYLE> "Diff style").default_value("unified"))
            )
            .subcommand(
                clap::Command::new("encrypt")
                    .about("Encrypt a value")
                    .arg(clap::arg!(<VALUE> "Value to encrypt").required(true))
                    .arg(clap::arg!(--key <KEY> "Encryption key"))
            )
            .subcommand(
                clap::Command::new("wizard")
                    .about("Interactive configuration wizard")
                    .arg(clap::arg!(--non_interactive "Skip interactive prompts"))
            );

        generate(shell_enum, &mut cmd, "confers", &mut io::stdout());
        Ok(())
    }

    /// Execute key management operations
    /// 
    /// # Arguments
    /// * `subcommand` - Key subcommand to execute
    /// 
    /// # Examples
    /// ```
    /// use confers::{ConfersCli, commands::key::KeySubcommand};
    /// 
    /// let subcommand = KeySubcommand::Generate;
    /// ConfersCli::key(&subcommand)?;
    /// ```
    pub fn key(subcommand: &crate::commands::key::KeySubcommand) -> Result<(), ConfigError> {
        KeyCommand::execute(subcommand, None, None)
    }
}

// Create a prelude module for macro use
pub mod prelude {
    pub use crate::Config;
    pub use crate::ConfigError;
    pub use crate::ConfigLoader;
    pub use crate::OptionalValidate;
    pub use crate::ResultExt;
    pub use crate::Sanitize;
    #[cfg(feature = "validation")]
    pub use crate::Validate;
    #[cfg(feature = "validation")]
    pub use crate::ValidationErrors;
    pub use serde::Deserialize;
    pub use serde::Serialize;
    
    // Re-export CLI facade when feature is enabled
    #[cfg(feature = "cli")]
    pub use crate::ConfersCli;
}

/// Trait for types that can be converted to a configuration map
pub trait ConfigMap {
    fn to_map(&self) -> std::collections::HashMap<String, serde_json::Value>;
    fn env_mapping() -> std::collections::HashMap<String, String> {
        std::collections::HashMap::new()
    }
}
