// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

use confers::Config;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Config)]
pub struct WatchConfig {
    pub message: String,
}

fn main() -> anyhow::Result<()> {
    let config = WatchConfig::load()?;
    println!("Watch Config: {:?}", config);
    Ok(())
}
