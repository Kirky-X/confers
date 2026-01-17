// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

use confers::Config;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Config)]
#[config(remote = "http://localhost:2379")]
pub struct EtcdConfig {
    pub key: String,
}

fn main() -> anyhow::Result<()> {
    let config = EtcdConfig::load()?;
    println!("Etcd Config: {:?}", config);
    Ok(())
}
