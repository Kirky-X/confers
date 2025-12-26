// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

use confers::Config;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Config)]
pub struct TestConfig {
    pub name: String,
}

fn main() {
    println!("Testing simple macro");
}
