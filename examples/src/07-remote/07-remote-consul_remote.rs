// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

use confers::Config;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Config)]
#[config(remote = "consul://localhost:8500")]
pub struct ConsulConfig {
    pub key: String,
}

fn main() -> anyhow::Result<()> {
    let config = ConsulConfig::load()?;
    println!("Consul Config: {:?}", config);
    Ok(())
}
