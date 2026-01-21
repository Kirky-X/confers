// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! 基本加密示例
//!
//! 展示 confers 的配置加密功能。
//!
//! ## 前提条件
//!
//! - 需要启用的特性：`derive`, `encryption`
//!
//! ## 运行方式
//!
//! ```bash
//! cargo run --example 06-encryption-basic_encryption --features "derive,encryption"
//! ```

use confers::Config;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Config)]
pub struct EncryptionConfig {
    pub api_key: String,
}

fn main() -> anyhow::Result<()> {
    let config = EncryptionConfig::load()?;
    println!("Encryption Config: {:?}", config);
    Ok(())
}
