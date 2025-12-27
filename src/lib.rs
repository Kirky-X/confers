// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

pub mod audit;
pub mod commands;
pub mod core;
pub mod encryption;
pub mod error;
pub mod key;
pub mod providers;
pub mod schema;
pub mod security;
pub mod utils;
pub mod validator;
pub mod validators;
pub mod watcher;

// Re-export commonly used items
pub use audit::Sanitize;
pub use confers_macros::Config;
pub use core::ConfigLoader;
pub use core::OptionalValidate;
pub use error::ConfigError;
pub use validator::{
    ParallelValidationConfig, ParallelValidationResult, ParallelValidator, Validate,
    ValidationErrors,
};

pub use security::{EnvSecurityError, EnvSecurityValidator, EnvironmentValidationConfig};

// Re-export dependencies that macros need
pub use clap;
pub use figment;
pub use serde;
pub use serde_json;

// Create a prelude module for the macro
pub mod prelude {
    pub use crate::ConfigError;
}

use figment::value::Value;
use std::collections::HashMap;

pub trait ConfigMap {
    fn to_map(&self) -> HashMap<String, Value>;
    fn env_mapping() -> HashMap<String, String>;
}
