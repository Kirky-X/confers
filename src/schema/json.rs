// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

use schemars::{schema_for, JsonSchema};
use serde_json::Value;

pub struct JsonSchemaGenerator;

impl JsonSchemaGenerator {
    pub fn generate<T: JsonSchema>() -> Value {
        let schema = schema_for!(T);
        serde_json::to_value(schema).unwrap_or_else(|e| {
            eprintln!("Failed to convert schema to JSON value: {}", e);
            serde_json::Value::Null
        })
    }
}
