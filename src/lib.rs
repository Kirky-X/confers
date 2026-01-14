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
}

/// Trait for types that can be converted to a configuration map
pub trait ConfigMap {
    fn to_map(&self) -> std::collections::HashMap<String, serde_json::Value>;
    fn env_mapping() -> std::collections::HashMap<String, String> {
        std::collections::HashMap::new()
    }
}
