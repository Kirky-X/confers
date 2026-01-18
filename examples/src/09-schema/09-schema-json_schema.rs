// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

use confers::Config;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Config)]
pub struct SchemaConfig {
    pub name: String,
    pub port: u16,
}

fn main() -> anyhow::Result<()> {
    let _config = SchemaConfig::load()?;

    // 注意：Schema 生成功能需要 schema 特性
    // 示例代码（需要启用 schema 特性）：
    // let schema = generate_schema(&config)?;
    // println!("JSON Schema: {}", schema);

    println!("Schema generation example requires the 'schema' feature.");
    println!("Run with: cargo run --bin 09-schema-json_schema --features schema");

    Ok(())
}
