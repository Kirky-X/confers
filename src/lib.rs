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

// 重新导出常用项
pub use audit::Sanitize;
pub use confers_macros::Config;
pub use core::ConfigLoader;
pub use core::OptionalValidate;
pub use error::ConfigError;
pub use validator::{
    ParallelValidationConfig, ParallelValidationResult, ParallelValidator, Validate,
    ValidationErrors,
};

pub use validators::{
    register_custom_validator, unregister_custom_validator, validate_with_custom,
    list_custom_validators, CustomValidator,
};

pub use security::{EnvSecurityError, EnvSecurityValidator, EnvironmentValidationConfig};

// 重新导出宏需要的依赖项
pub use clap;
pub use figment;
pub use serde;
pub use serde_json;

// 创建一个 prelude 模块供宏使用
pub mod prelude {
    pub use crate::ConfigError;
}

use figment::value::Value;
use std::collections::HashMap;

pub trait ConfigMap {
    fn to_map(&self) -> HashMap<String, Value>;
    fn env_mapping() -> HashMap<String, String>;
}
