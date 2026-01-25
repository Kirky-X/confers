// Copyright (c) 2025 Kirky.X
//
// Licensed under MIT License
// See LICENSE file in project root for full license information.

//! 测试模块入口
//!
//! 组织所有测试模块，包括安全、CLI、提供者、schema和密钥管理测试

pub mod architecture;
#[cfg(test)]
pub mod commands;
#[cfg(test)]
pub mod common;
#[cfg(feature = "encryption")]
#[cfg(test)]
#[cfg(test)]
pub mod providers;
#[cfg(feature = "schema")]
#[cfg(test)]
pub mod schema;
#[cfg(test)]
pub mod security;
