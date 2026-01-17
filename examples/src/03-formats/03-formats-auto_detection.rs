// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

use confers::Config;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Config)]
pub struct AutoConfig {
    pub name: String,
    pub port: u16,
}

fn main() -> anyhow::Result<()> {
    let config_content = "name = auto-example\nport = 8080\n";
    std::fs::write("src/03-formats/configs/config.toml", config_content)?;
    let config = AutoConfig::load()?;
    println!("Auto-detected Config: {:?}", config);
    Ok(())
}
