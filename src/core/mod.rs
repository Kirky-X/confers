// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

pub mod builder;
pub mod loader;

pub use builder::{ConfigBuilder, Environment, File};
pub use loader::{ConfigLoader, OptionalValidate};
pub use crate::utils::FileFormat;
