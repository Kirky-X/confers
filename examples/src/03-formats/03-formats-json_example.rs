// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

use confers::Config;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Config)]
pub struct JsonConfig {
    pub name: String,
    pub port: u16,
}

fn main() -> anyhow::Result<()> {
    let config_content = r#"{"name": "json-example", "port": 8080}"#;
    std::fs::write("src/03-formats/configs/config.json", config_content)?;
    let config = JsonConfig::load()?;
    println!("JSON Config: {:?}", config);
    Ok(())
}
