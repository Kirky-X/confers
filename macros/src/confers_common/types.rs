// Copyright (c) 2025 Kirky.X
//
// Licensed under MIT License
// See LICENSE file in the project root for full license information.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConfigSource {
    File(String),
    Environment,
    Cli,
    Remote(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigMetadata {
    pub source: ConfigSource,
    pub last_modified: Option<String>,
    pub version: Option<String>,
}
