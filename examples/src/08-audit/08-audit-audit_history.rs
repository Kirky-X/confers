// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

use confers::Config;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Config)]
pub struct AuditHistoryConfig {
    pub name: String,
}

fn main() -> anyhow::Result<()> {
    let config = AuditHistoryConfig::load()?;
    println!("Audit History Config: {:?}", config);
    Ok(())
}
