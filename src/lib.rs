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
pub use core::builder::{ConfigBuilder, ConfigSaveExt, Environment, File};
pub use core::{ConfigLoader, OptionalValidate};
pub use error::ConfigError;
pub use error_helpers::{OptionExt, ResultExt};
pub use utils::FileFormat;
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
    /// * `struct_name` - Optional struct name to generate schema for (currently placeholder)
    /// * `format` - Output format: "toml", "json", "yaml", "ini"
    ///
    /// # Examples
    /// ```
    /// fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     use confers::ConfersCli;
    ///
    ///     // Generate to file
    ///     ConfersCli::generate(Some("config.toml"), "full", None, "toml")?;
    ///
    ///     // Generate to stdout
    ///     ConfersCli::generate(None, "minimal", None, "toml")?;
    ///     Ok(())
    /// }
    /// ```
    pub fn generate(
        output: Option<&str>,
        level: &str,
        struct_name: Option<&str>,
        format: &str,
    ) -> Result<(), ConfigError> {
        let output_str = output.map(|s| s.to_string());
        let struct_str = struct_name.map(|s| s.to_string());
        GenerateCommand::execute_placeholder(
            output_str.as_ref(),
            level,
            struct_str.as_ref(),
            format,
        )
        .map(|_| ())
    }

    /// Validate configuration file
    ///
    /// # Arguments
    /// * `config` - Path to configuration file
    /// * `level` - Validation level: "minimal", "full", or "documentation"
    ///
    /// # Examples
    /// ```
    /// use std::fs::File;
    /// use std::io::Write;
    ///
    /// fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     use confers::ConfersCli;
    ///
    ///     // Create a dummy config file
    ///     let mut file = File::create("config_test.toml")?;
    ///     writeln!(file, "[app]\nname = \"test\"")?;
    ///
    ///     ConfersCli::validate("config_test.toml", "full")?;
    ///
    ///     // Clean up
    ///     std::fs::remove_file("config_test.toml")?;
    ///     Ok(())
    /// }
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
    /// fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     use confers::ConfersCli;
    ///
    ///     // Note: Requires CONFERS_ENCRYPTION_KEY environment variable or valid key parameter
    ///     // Or generate a key using: base64::encode(&rand::random::<[u8; 32]>())
    ///     //
    ///     // Example with key:
    ///     // let encrypted = ConfersCli::encrypt("secret_value", Some("your_base64_key"))?;
    ///
    ///     // For testing, you can set the environment variable:
    ///     // std::env::set_var("CONFERS_ENCRYPTION_KEY", base64::encode(&rand::random::<[u8; 32]>()));
    ///     // let encrypted = ConfersCli::encrypt("secret_value", None)?;
    ///
    ///     Ok(())
    /// }
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
    /// * `output` - Optional output file path
    ///
    /// # Examples
    /// ```
    /// fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     use confers::ConfersCli;
    ///     use std::fs;
    ///
    ///     // Create test config files
    ///     fs::write("config1_test.toml", "name = \"app1\"\nport = 8080\n")?;
    ///     fs::write("config2_test.toml", "name = \"app2\"\nport = 9090\n")?;
    ///
    ///     ConfersCli::diff("config1_test.toml", "config2_test.toml", Some("unified"), None)?;
    ///
    ///     // Cleanup
    ///     fs::remove_file("config1_test.toml")?;
    ///     fs::remove_file("config2_test.toml")?;
    ///     Ok(())
    /// }
    /// ```
    pub fn diff(file1: &str, file2: &str, format: Option<&str>, output: Option<&str>) -> Result<(), ConfigError> {
        let diff_format = format
            .unwrap_or("unified")
            .parse()
            .map_err(ConfigError::ParseError)?;
        let options = DiffOptions {
            format: diff_format,
            output: output.map(|s| s.to_string()),
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
    /// fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     use confers::ConfersCli;
    ///
    ///     // Interactive wizard
    ///     // ConfersCli::wizard(false)?;
    ///
    ///     // Non-interactive with defaults
    ///     ConfersCli::wizard(true)?;
    ///     Ok(())
    /// }
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
    /// fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     use confers::ConfersCli;
    ///
    ///     ConfersCli::completions("bash")?;
    ///     Ok(())
    /// }
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
                    .arg(clap::arg!(--level <LEVEL> "Template level").default_value("full")),
            )
            .subcommand(
                clap::Command::new("validate")
                    .about("Validate configuration file")
                    .arg(clap::arg!(--config <FILE> "Configuration file path").required(true))
                    .arg(clap::arg!(--level <LEVEL> "Validation level").default_value("full")),
            )
            .subcommand(
                clap::Command::new("diff")
                    .about("Compare two configuration files")
                    .arg(clap::arg!(<FILE1> "First file").required(true))
                    .arg(clap::arg!(<FILE2> "Second file").required(true))
                    .arg(clap::arg!(--style <STYLE> "Diff style").default_value("unified")),
            )
            .subcommand(
                clap::Command::new("encrypt")
                    .about("Encrypt a value")
                    .arg(clap::arg!(<VALUE> "Value to encrypt").required(true))
                    .arg(clap::arg!(--key <KEY> "Encryption key")),
            )
            .subcommand(
                clap::Command::new("wizard")
                    .about("Interactive configuration wizard")
                    .arg(clap::arg!(--non_interactive "Skip interactive prompts")),
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
    /// fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     use confers::{ConfersCli, commands::key::KeySubcommand};
    ///
    ///     let subcommand = KeySubcommand::Generate {
    ///         output: None,
    ///         algorithm: "AES256".to_string(),
    ///         size: 256,
    ///     };
    ///     ConfersCli::key(&subcommand)?;
    ///     Ok(())
    /// }
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
