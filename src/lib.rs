// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

pub mod audit;
pub mod commands;
pub mod constants;
pub mod core;
pub mod encryption;
pub mod error;
pub mod error_helpers;
pub mod key;
pub mod providers;
pub mod schema;
pub mod security;
pub mod utils;
pub mod validator;
pub mod validators;
pub mod watcher;

// Re-export common items
pub use audit::Sanitize;
pub use confers_macros::Config;
pub use core::ConfigLoader;
pub use core::OptionalValidate;
pub use error::ConfigError;
pub use error_helpers::{OptionExt, ResultExt};
pub use validator::{
    ParallelValidationConfig, ParallelValidationResult, ParallelValidator, Validate,
    ValidationErrors,
};

// Re-export macro dependencies
pub use clap;
pub use serde::{Deserialize, Serialize};
pub use serde_json;
pub use validator::Validate as ValidatorValidate;

// Create a prelude module for macro use
pub mod prelude {
    pub use crate::Config;
    pub use crate::ConfigError;
    pub use crate::ConfigLoader;
    pub use crate::OptionalValidate;
    pub use crate::ParallelValidationConfig;
    pub use crate::ParallelValidationResult;
    pub use crate::ParallelValidator;
    pub use crate::ResultExt;
    pub use crate::Sanitize;
    pub use crate::Validate;
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
