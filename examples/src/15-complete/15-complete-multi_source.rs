// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

use confers::Config;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Config)]
#[config(validate)]
pub struct MultiSourceConfig {
    pub name: String,
    pub port: u16,
}

fn main() -> anyhow::Result<()> {
    let config = MultiSourceConfig::load()?;
    println!("Multi Source Config: {:?}", config);
    Ok(())
}
