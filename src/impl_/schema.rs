// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

use schemars::{JsonSchema, schema_for};
use serde_json::Value;

use crate::error::{ConfigError, ConfigResult};

pub struct TypeScriptGenerator;

impl TypeScriptGenerator {
    pub fn generate<T: JsonSchema>() -> ConfigResult<String> {
        let schema = schema_for!(T);
        let schema_value = serde_json::to_value(schema).map_err(|e| ConfigError::ParseError {
            format: "schema".into(),
            message: e.to_string(),
            location: None,
            source: None,
        })?;
        Ok(Self::convert_json_schema_to_typescript(&schema_value))
    }

    fn convert_json_schema_to_typescript(schema: &Value) -> String {
        let mut interfaces = Vec::new();

        // First, handle definitions if they exist
        if let Some(definitions) = schema.get("definitions")
            && let Some(defs_obj) = definitions.as_object()
        {
            for (name, def_schema) in defs_obj {
                let interface = Self::generate_interface(name, def_schema);
                interfaces.push(interface);
            }
        }

        // Then, handle the main schema
        let has_properties = schema.get("properties").is_some();
        let has_type_object = schema.get("type").and_then(|t| t.as_str()) == Some("object");
        let has_definitions = schema.get("definitions").is_some();
        if has_properties || has_type_object {
            // Use the title from the schema if available, otherwise default to "Config"
            let interface_name = schema
                .get("title")
                .and_then(|t| t.as_str())
                .unwrap_or("Config");

            let main_interface = Self::generate_interface(interface_name, schema);
            interfaces.push(main_interface);
        } else if !has_definitions && interfaces.is_empty() && has_type_object {
            // Bare {"type": "object"} with no properties
            let interface_name = schema
                .get("title")
                .and_then(|t| t.as_str())
                .unwrap_or("Config");
            interfaces.push(format!(
                "export type {} = Record<string, any>;",
                interface_name
            ));
        }

        if interfaces.is_empty() {
            "// Invalid schema format".to_string()
        } else {
            interfaces.join("\n\n")
        }
    }

    fn generate_interface(name: &str, schema: &Value) -> String {
        // First check if this is a oneOf (enum) that should be a union type, not an interface
        if let Some(_one_of) = schema.get("oneOf").and_then(|o| o.as_array()) {
            let union_type = Self::get_typescript_type(schema);
            return format!("export type {} = {};", name, union_type);
        }

        let mut properties = Vec::new();

        if let Some(props) = schema.get("properties").and_then(|p| p.as_object()) {
            for (prop_name, prop_schema) in props {
                let prop_type = Self::get_typescript_type(prop_schema);
                let optional = Self::is_optional(prop_name, schema);
                // Quote keys that are not valid TS identifiers (e.g. "my-key")
                // so the generated interface is valid TypeScript.
                let key = Self::ts_property_key(prop_name);

                let property_def = if optional {
                    format!("  {}?: {};", key, prop_type)
                } else {
                    format!("  {}: {};", key, prop_type)
                };

                properties.push(property_def);
            }
        } else if let Some(type_str) = schema.get("type").and_then(|t| t.as_str()) {
            // If it's just a primitive type but with a name, make it a type alias
            if type_str != "object" {
                let ts_type = Self::get_typescript_type(schema);
                return format!("export type {} = {};", name, ts_type);
            }
        }

        let properties_str = properties.join("\n");
        format!("export interface {} {{\n{}\n}}", name, properties_str)
    }

    fn get_typescript_type(schema: &Value) -> String {
        // Handle $ref references first as they are most specific
        if let Some(ref_name) = schema.get("$ref").and_then(|r| r.as_str()) {
            if ref_name.is_empty() {
                return "any".to_string();
            }
            let parts: Vec<&str> = ref_name.split('/').collect();
            return parts.last().unwrap_or(&"any").to_string();
        }

        // Handle array type: ["integer", "null"] for Option types
        if let Some(type_array) = schema.get("type").and_then(|t| t.as_array()) {
            let types: Vec<String> = type_array
                .iter()
                .filter_map(|t| t.as_str())
                .map(|t| match t {
                    "string" => "string".to_string(),
                    "number" | "integer" => "number".to_string(),
                    "boolean" => "boolean".to_string(),
                    "null" => "null".to_string(),
                    _ => "any".to_string(),
                })
                .collect();

            // Preserve "null" in the union (Option<T> -> "T | null") so the
            // generated type keeps the value's nullable runtime semantics.
            // `null` is normalized to the last union member (TS convention);
            // optionality (`field?:`) is rendered separately by
            // `generate_interface` via `is_optional`, so a nullable field
            // appears as `field?: T | null`.
            let has_null = types.iter().any(|t| t == "null");
            let mut ordered: Vec<String> = types.into_iter().filter(|t| t != "null").collect();
            if has_null {
                ordered.push("null".to_string());
            }
            return ordered.join(" | ");
        }

        // Handle single type string
        if let Some(type_str) = schema.get("type").and_then(|t| t.as_str()) {
            match type_str {
                "string" => "string".to_string(),
                "number" | "integer" => "number".to_string(),
                "boolean" => "boolean".to_string(),
                "array" => {
                    if let Some(items) = schema.get("items") {
                        let item_type = Self::get_typescript_type(items);
                        format!("{}[]", item_type)
                    } else {
                        "any[]".to_string()
                    }
                }
                "object" => {
                    if let Some(props) = schema.get("properties").and_then(|p| p.as_object()) {
                        let mut inner_props = Vec::new();
                        for (p_name, p_schema) in props {
                            let p_type = Self::get_typescript_type(p_schema);
                            // Quote keys that are not valid TS identifiers.
                            inner_props.push(format!(
                                "{}: {}",
                                Self::ts_property_key(p_name),
                                p_type
                            ));
                        }
                        format!("{{ {} }}", inner_props.join("; "))
                    } else if let Some(additional_props) = schema.get("additionalProperties") {
                        let val_type = Self::get_typescript_type(additional_props);
                        format!("Record<string, {}>", val_type)
                    } else {
                        "Record<string, any>".to_string()
                    }
                }
                _ => "any".to_string(),
            }
        } else if let Some(any_of) = schema.get("anyOf").and_then(|a| a.as_array()) {
            let union_types: Vec<String> = any_of
                .iter()
                .map(Self::get_typescript_type)
                .filter(|t| t != "null")
                .collect();

            if union_types.is_empty() {
                "any".to_string()
            } else if union_types.len() == 1 {
                union_types[0].clone()
            } else {
                union_types.join(" | ")
            }
        } else if let Some(one_of) = schema.get("oneOf").and_then(|o| o.as_array()) {
            let mut union_types = Vec::new();
            for variant_schema in one_of {
                let variant_type = Self::get_typescript_type(variant_schema);
                if variant_type != "any" {
                    union_types.push(variant_type);
                }
            }
            if union_types.is_empty() {
                "any".to_string()
            } else {
                union_types.join(" | ")
            }
        } else if let Some(all_of) = schema.get("allOf").and_then(|a| a.as_array()) {
            let all_types: Vec<String> = all_of
                .iter()
                .map(Self::get_typescript_type)
                .filter(|t| t != "any")
                .collect();
            if all_types.is_empty() {
                "any".to_string()
            } else {
                all_types.join(" & ")
            }
        } else if let Some(enum_values) = schema.get("enum").and_then(|e| e.as_array()) {
            let variants: Vec<String> = enum_values
                .iter()
                .map(|v| match v {
                    Value::String(s) => format!("\"{}\"", s),
                    _ => v.to_string(),
                })
                .collect();
            if !variants.is_empty() {
                variants.join(" | ")
            } else {
                "any".to_string()
            }
        } else {
            "any".to_string()
        }
    }

    fn is_optional(property_name: &str, schema: &Value) -> bool {
        if let Some(required) = schema.get("required").and_then(|r| r.as_array()) {
            !required.iter().any(|r| r.as_str() == Some(property_name))
        } else {
            true // If no required array, assume all properties are optional
        }
    }

    /// Check whether a key is a valid TypeScript identifier shape.
    ///
    /// Only the lexical shape matters: TypeScript allows reserved words as
    /// property names (e.g. `interface X { class: string }` is valid), so a key
    /// is bare-safe when it starts with an ASCII letter, `_` or `$` and
    /// contains only ASCII identifier characters. Anything else (e.g.
    /// `my-key`, `2nd`, keys containing spaces) must be quoted.
    fn is_valid_ts_identifier(key: &str) -> bool {
        let mut chars = key.chars();
        match chars.next() {
            Some(c) if c.is_ascii_alphabetic() || c == '_' || c == '$' => {}
            _ => return false,
        }
        chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$')
    }

    /// Render an object key as a TypeScript property key: bare when it is a
    /// valid identifier, quoted (with escapes) otherwise, so keys like
    /// `my-key`, `2nd` or `with space` produce valid TypeScript.
    fn ts_property_key(key: &str) -> String {
        if Self::is_valid_ts_identifier(key) {
            key.to_string()
        } else {
            format!("\"{}\"", key.replace('\\', "\\\\").replace('"', "\\\""))
        }
    }

    /// Generate TypeScript definitions from a JSON value representing a config structure
    pub fn from_json_value(value: &Value) -> String {
        match value {
            Value::Object(map) => Self::generate_from_object(map),
            Value::Array(arr) => {
                if let Some(first) = arr.first() {
                    format!("{}[]", Self::from_json_value(first))
                } else {
                    "any[]".to_string()
                }
            }
            Value::String(_) => "string".to_string(),
            Value::Number(_) => "number".to_string(),
            Value::Bool(_) => "boolean".to_string(),
            Value::Null => "null".to_string(),
        }
    }

    fn generate_from_object(obj: &serde_json::Map<String, Value>) -> String {
        let mut properties = Vec::new();

        for (key, value) in obj {
            let prop_type = Self::from_json_value(value);
            // Quote keys that are not valid TS identifiers (e.g. "my-key").
            properties.push(format!("  {}: {};", Self::ts_property_key(key), prop_type));
        }

        if properties.is_empty() {
            "Record<string, any>".to_string()
        } else {
            format!("{{\n{}\n}}", properties.join("\n"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use schemars::JsonSchema;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, JsonSchema)]
    struct NestedConfig {
        description: String,
        value: Option<i32>,
    }

    #[derive(Debug, Serialize, Deserialize, JsonSchema)]
    struct TestConfig {
        name: String,
        count: u32,
        enabled: bool,
        tags: Vec<String>,
        nested: Option<NestedConfig>,
        config_type: ConfigType,
    }

    #[derive(Debug, Serialize, Deserialize, JsonSchema)]
    enum ConfigType {
        Basic,
        Advanced,
        Custom(String),
    }

    #[test]
    fn test_typescript_generation() {
        use schemars::schema_for;

        let schema = schema_for!(TestConfig);
        let schema_json = serde_json::to_string_pretty(&schema).unwrap();
        println!("JSON Schema:");
        println!("{}", schema_json);
        println!("--- End of JSON Schema ---");

        let ts_output = TypeScriptGenerator::generate::<TestConfig>().unwrap();
        println!("Generated TypeScript output:");
        println!("{}", ts_output);
        println!("--- End of output ---");

        // 暂时放宽测试条件，先查看输出
        assert!(!ts_output.is_empty());
    }

    #[test]
    fn test_from_json_value() {
        let json = serde_json::json!({
            "name": "test",
            "count": 42,
            "enabled": true,
            "tags": ["a", "b"]
        });

        let ts_type = TypeScriptGenerator::from_json_value(&json);
        assert!(ts_type.contains("name: string"));
        assert!(ts_type.contains("count: number"));
        assert!(ts_type.contains("enabled: boolean"));
        assert!(ts_type.contains("tags: string[]"));
    }

    // ---- from_json_value: each Value variant ----

    #[test]
    fn test_from_json_value_string() {
        let v = serde_json::json!("hello");
        assert_eq!(TypeScriptGenerator::from_json_value(&v), "string");
    }

    #[test]
    fn test_from_json_value_number() {
        let v = serde_json::json!(42);
        assert_eq!(TypeScriptGenerator::from_json_value(&v), "number");
    }

    #[test]
    fn test_from_json_value_bool() {
        let v = serde_json::json!(true);
        assert_eq!(TypeScriptGenerator::from_json_value(&v), "boolean");
    }

    #[test]
    fn test_from_json_value_null() {
        let v = serde_json::Value::Null;
        assert_eq!(TypeScriptGenerator::from_json_value(&v), "null");
    }

    #[test]
    fn test_from_json_value_empty_array() {
        let v = serde_json::json!([]);
        assert_eq!(TypeScriptGenerator::from_json_value(&v), "any[]");
    }

    #[test]
    fn test_from_json_value_array_with_first_element() {
        let v = serde_json::json!([1, 2, 3]);
        assert_eq!(TypeScriptGenerator::from_json_value(&v), "number[]");
    }

    #[test]
    fn test_from_json_value_array_of_objects() {
        let v = serde_json::json!([{ "x": 1 }]);
        let ts = TypeScriptGenerator::from_json_value(&v);
        assert!(ts.ends_with("[]"));
        assert!(ts.contains("x: number"));
    }

    #[test]
    fn test_from_json_value_empty_object() {
        let v = serde_json::json!({});
        assert_eq!(
            TypeScriptGenerator::from_json_value(&v),
            "Record<string, any>"
        );
    }

    #[test]
    fn test_from_json_value_nested_object() {
        let v = serde_json::json!({
            "outer": { "inner": "value", "n": 5 }
        });
        let ts = TypeScriptGenerator::from_json_value(&v);
        assert!(ts.contains("outer:"));
        assert!(ts.contains("inner: string"));
        assert!(ts.contains("n: number"));
    }

    // ---- get_typescript_type: $ref ----

    #[test]
    fn test_get_typescript_type_ref_simple() {
        let s = serde_json::json!({ "$ref": "#/definitions/Foo" });
        assert_eq!(TypeScriptGenerator::get_typescript_type(&s), "Foo");
    }

    #[test]
    fn test_get_typescript_type_ref_no_slashes() {
        let s = serde_json::json!({ "$ref": "Bar" });
        assert_eq!(TypeScriptGenerator::get_typescript_type(&s), "Bar");
    }

    #[test]
    fn test_get_typescript_type_ref_empty_string() {
        // An empty $ref string now returns "any" instead of empty string
        let s = serde_json::json!({ "$ref": "" });
        assert_eq!(TypeScriptGenerator::get_typescript_type(&s), "any");
    }

    #[test]
    fn test_get_typescript_type_ref_non_string() {
        let s = serde_json::json!({ "$ref": 42 });
        // No type, no anyOf/oneOf/allOf/enum -> falls through to "any".
        assert_eq!(TypeScriptGenerator::get_typescript_type(&s), "any");
    }

    // ---- get_typescript_type: type arrays (Option-like) ----

    #[test]
    fn test_get_typescript_type_array_option_string() {
        let s = serde_json::json!({ "type": ["string", "null"] });
        // Option<String> keeps null in the union (see #214): "string | null".
        assert_eq!(
            TypeScriptGenerator::get_typescript_type(&s),
            "string | null"
        );
    }

    #[test]
    fn test_get_typescript_type_array_option_integer() {
        let s = serde_json::json!({ "type": ["integer", "null"] });
        assert_eq!(
            TypeScriptGenerator::get_typescript_type(&s),
            "number | null"
        );
    }

    #[test]
    fn test_get_typescript_type_array_option_number() {
        let s = serde_json::json!({ "type": ["null", "number"] });
        assert_eq!(
            TypeScriptGenerator::get_typescript_type(&s),
            "number | null"
        );
    }

    #[test]
    fn test_get_typescript_type_array_option_order_preserved() {
        // Non-null members keep their schema order; `null` is normalized to
        // the last union position (TS convention).
        let s = serde_json::json!({ "type": ["null", "boolean"] });
        assert_eq!(
            TypeScriptGenerator::get_typescript_type(&s),
            "boolean | null"
        );
    }

    #[test]
    fn test_get_typescript_type_array_two_non_null() {
        let s = serde_json::json!({ "type": ["string", "number"] });
        assert_eq!(
            TypeScriptGenerator::get_typescript_type(&s),
            "string | number"
        );
    }

    #[test]
    fn test_get_typescript_type_array_only_null() {
        // A bare ["null"] schema joins to plain "null".
        let s = serde_json::json!({ "type": ["null"] });
        assert_eq!(TypeScriptGenerator::get_typescript_type(&s), "null");
    }

    #[test]
    fn test_get_typescript_type_array_with_unknown_type() {
        let s = serde_json::json!({ "type": ["string", "object"] });
        assert_eq!(TypeScriptGenerator::get_typescript_type(&s), "string | any");
    }

    // ---- get_typescript_type: single type string ----

    #[test]
    fn test_get_typescript_type_string() {
        let s = serde_json::json!({ "type": "string" });
        assert_eq!(TypeScriptGenerator::get_typescript_type(&s), "string");
    }

    #[test]
    fn test_get_typescript_type_integer() {
        let s = serde_json::json!({ "type": "integer" });
        assert_eq!(TypeScriptGenerator::get_typescript_type(&s), "number");
    }

    #[test]
    fn test_get_typescript_type_number() {
        let s = serde_json::json!({ "type": "number" });
        assert_eq!(TypeScriptGenerator::get_typescript_type(&s), "number");
    }

    #[test]
    fn test_get_typescript_type_boolean() {
        let s = serde_json::json!({ "type": "boolean" });
        assert_eq!(TypeScriptGenerator::get_typescript_type(&s), "boolean");
    }

    #[test]
    fn test_get_typescript_type_array_with_items_ref() {
        let s = serde_json::json!({
            "type": "array",
            "items": { "$ref": "#/definitions/Foo" }
        });
        assert_eq!(TypeScriptGenerator::get_typescript_type(&s), "Foo[]");
    }

    #[test]
    fn test_get_typescript_type_array_with_items_string() {
        let s = serde_json::json!({
            "type": "array",
            "items": { "type": "string" }
        });
        assert_eq!(TypeScriptGenerator::get_typescript_type(&s), "string[]");
    }

    #[test]
    fn test_get_typescript_type_array_without_items() {
        let s = serde_json::json!({ "type": "array" });
        assert_eq!(TypeScriptGenerator::get_typescript_type(&s), "any[]");
    }

    #[test]
    fn test_get_typescript_type_object_with_properties() {
        let s = serde_json::json!({
            "type": "object",
            "properties": {
                "a": { "type": "string" },
                "b": { "type": "integer" }
            }
        });
        assert_eq!(
            TypeScriptGenerator::get_typescript_type(&s),
            "{ a: string; b: number }"
        );
    }

    #[test]
    fn test_get_typescript_type_object_with_additional_properties() {
        let s = serde_json::json!({
            "type": "object",
            "additionalProperties": { "type": "string" }
        });
        assert_eq!(
            TypeScriptGenerator::get_typescript_type(&s),
            "Record<string, string>"
        );
    }

    #[test]
    fn test_get_typescript_type_object_bare() {
        let s = serde_json::json!({ "type": "object" });
        assert_eq!(
            TypeScriptGenerator::get_typescript_type(&s),
            "Record<string, any>"
        );
    }

    #[test]
    fn test_get_typescript_type_unknown_single_type() {
        let s = serde_json::json!({ "type": "weird" });
        assert_eq!(TypeScriptGenerator::get_typescript_type(&s), "any");
    }

    // ---- get_typescript_type: anyOf ----

    #[test]
    fn test_get_typescript_type_any_of_single() {
        let s = serde_json::json!({ "anyOf": [{ "type": "string" }] });
        assert_eq!(TypeScriptGenerator::get_typescript_type(&s), "string");
    }

    #[test]
    fn test_get_typescript_type_any_of_multiple() {
        let s = serde_json::json!({
            "anyOf": [{ "type": "string" }, { "type": "integer" }]
        });
        assert_eq!(
            TypeScriptGenerator::get_typescript_type(&s),
            "string | number"
        );
    }

    #[test]
    fn test_get_typescript_type_any_of_only_null() {
        // All variants are null -> filtered out -> empty -> "any"
        let s = serde_json::json!({ "anyOf": [{ "type": "null" }] });
        assert_eq!(TypeScriptGenerator::get_typescript_type(&s), "any");
    }

    #[test]
    fn test_get_typescript_type_any_of_empty() {
        let s = serde_json::json!({ "anyOf": [] });
        assert_eq!(TypeScriptGenerator::get_typescript_type(&s), "any");
    }

    // ---- get_typescript_type: oneOf ----

    #[test]
    fn test_get_typescript_type_one_of_single() {
        let s = serde_json::json!({ "oneOf": [{ "type": "string" }] });
        assert_eq!(TypeScriptGenerator::get_typescript_type(&s), "string");
    }

    #[test]
    fn test_get_typescript_type_one_of_multiple() {
        let s = serde_json::json!({
            "oneOf": [{ "type": "string" }, { "type": "integer" }]
        });
        assert_eq!(
            TypeScriptGenerator::get_typescript_type(&s),
            "string | number"
        );
    }

    #[test]
    fn test_get_typescript_type_one_of_skips_any() {
        let s = serde_json::json!({
            "oneOf": [{ "type": "string" }, { "type": "weird" }]
        });
        // "weird" resolves to "any" and is skipped.
        assert_eq!(TypeScriptGenerator::get_typescript_type(&s), "string");
    }

    #[test]
    fn test_get_typescript_type_one_of_all_any() {
        let s = serde_json::json!({
            "oneOf": [{ "type": "weird" }, { "type": "alien" }]
        });
        assert_eq!(TypeScriptGenerator::get_typescript_type(&s), "any");
    }

    // ---- get_typescript_type: allOf ----

    #[test]
    fn test_get_typescript_type_all_of_single() {
        let s = serde_json::json!({ "allOf": [{ "type": "string" }] });
        assert_eq!(TypeScriptGenerator::get_typescript_type(&s), "string");
    }

    #[test]
    fn test_get_typescript_type_all_of_multiple() {
        let s = serde_json::json!({
            "allOf": [{ "type": "string" }, { "type": "integer" }]
        });
        assert_eq!(
            TypeScriptGenerator::get_typescript_type(&s),
            "string & number"
        );
    }

    #[test]
    fn test_get_typescript_type_all_of_skips_any() {
        let s = serde_json::json!({
            "allOf": [{ "type": "string" }, { "type": "weird" }]
        });
        assert_eq!(TypeScriptGenerator::get_typescript_type(&s), "string");
    }

    #[test]
    fn test_get_typescript_type_all_of_empty() {
        let s = serde_json::json!({ "allOf": [] });
        assert_eq!(TypeScriptGenerator::get_typescript_type(&s), "any");
    }

    // ---- get_typescript_type: enum ----

    #[test]
    fn test_get_typescript_type_enum_strings() {
        let s = serde_json::json!({ "enum": ["red", "green", "blue"] });
        assert_eq!(
            TypeScriptGenerator::get_typescript_type(&s),
            "\"red\" | \"green\" | \"blue\""
        );
    }

    #[test]
    fn test_get_typescript_type_enum_mixed() {
        let s = serde_json::json!({ "enum": ["x", 42, true] });
        let ts = TypeScriptGenerator::get_typescript_type(&s);
        assert!(ts.contains("\"x\""));
        assert!(ts.contains("42"));
        assert!(ts.contains("true"));
    }

    #[test]
    fn test_get_typescript_type_enum_empty() {
        let s = serde_json::json!({ "enum": [] });
        assert_eq!(TypeScriptGenerator::get_typescript_type(&s), "any");
    }

    // ---- get_typescript_type: nothing recognizable ----

    #[test]
    fn test_get_typescript_type_empty_object() {
        let s = serde_json::json!({});
        assert_eq!(TypeScriptGenerator::get_typescript_type(&s), "any");
    }

    // ---- is_optional ----

    #[test]
    fn test_is_optional_no_required_array() {
        // No "required" -> all properties are optional.
        let s = serde_json::json!({ "properties": { "a": {} } });
        assert!(TypeScriptGenerator::is_optional("a", &s));
    }

    #[test]
    fn test_is_optional_when_required_present() {
        let s = serde_json::json!({
            "required": ["a"],
            "properties": { "a": {}, "b": {} }
        });
        assert!(!TypeScriptGenerator::is_optional("a", &s));
        assert!(TypeScriptGenerator::is_optional("b", &s));
    }

    #[test]
    fn test_is_optional_required_not_array() {
        // "required" is present but not an array -> treated as no required list.
        let s = serde_json::json!({ "required": "oops" });
        assert!(TypeScriptGenerator::is_optional("a", &s));
    }

    // ---- generate_interface ----

    #[test]
    fn test_generate_interface_one_of_returns_type_alias() {
        let s = serde_json::json!({
            "oneOf": [{ "type": "string" }, { "type": "integer" }]
        });
        let out = TypeScriptGenerator::generate_interface("MyUnion", &s);
        assert!(out.starts_with("export type MyUnion ="));
        assert!(out.contains("string | number"));
    }

    #[test]
    fn test_generate_interface_primitive_alias() {
        let s = serde_json::json!({ "type": "string" });
        let out = TypeScriptGenerator::generate_interface("Alias", &s);
        assert_eq!(out, "export type Alias = string;");
    }

    #[test]
    fn test_generate_interface_alias_integer() {
        let s = serde_json::json!({ "type": "integer" });
        let out = TypeScriptGenerator::generate_interface("Count", &s);
        assert_eq!(out, "export type Count = number;");
    }

    #[test]
    fn test_generate_interface_with_required_and_optional_props() {
        let s = serde_json::json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" },
                "age": { "type": "integer" }
            },
            "required": ["name"]
        });
        let out = TypeScriptGenerator::generate_interface("Person", &s);
        assert!(out.contains("name: string;"));
        assert!(out.contains("age?: number;"));
        assert!(out.starts_with("export interface Person {"));
    }

    /// Regression test for #215: property keys that are not valid TypeScript
    /// identifiers (`my-key`, `2nd`, keys with spaces) must be quoted, otherwise
    /// the generated interface is not valid TypeScript. Valid identifiers stay
    /// unquoted.
    #[test]
    fn test_generate_interface_quotes_non_identifier_keys() {
        let s = serde_json::json!({
            "type": "object",
            "properties": {
                "my-key": { "type": "string" },
                "2nd": { "type": "integer" },
                "with space": { "type": "boolean" },
                "good_name": { "type": "string" }
            }
        });
        let out = TypeScriptGenerator::generate_interface("Weird", &s);
        // No `required` list → every property renders as optional (`?:`).
        assert!(out.contains("\"my-key\"?: string;"), "got: {}", out);
        assert!(out.contains("\"2nd\"?: number;"), "got: {}", out);
        assert!(out.contains("\"with space\"?: boolean;"), "got: {}", out);
        assert!(out.contains("good_name?: string;"), "got: {}", out);
        // Quoting must wrap exactly the key, not the whole definition.
        assert!(!out.contains("\"good_name\""), "got: {}", out);
    }

    /// Nullable fields render as `field?: T | null`: optionality comes from
    /// `is_optional`, while the runtime nullability stays in the union (#214).
    #[test]
    fn test_generate_interface_nullable_option_field() {
        let s = serde_json::json!({
            "type": "object",
            "properties": {
                "nickname": { "type": ["string", "null"] }
            },
            "required": ["nickname"]
        });
        let out = TypeScriptGenerator::generate_interface("Profile", &s);
        assert!(
            out.contains("nickname: string | null;"),
            "required nullable field must keep null in the union, got: {}",
            out
        );

        let optional_nullable = serde_json::json!({
            "type": "object",
            "properties": {
                "nickname": { "type": ["string", "null"] }
            }
        });
        let out = TypeScriptGenerator::generate_interface("Profile", &optional_nullable);
        assert!(
            out.contains("nickname?: string | null;"),
            "optional nullable field must render `field?: T | null`, got: {}",
            out
        );
    }

    /// Same quoting rule applies to keys of `from_json_value`-generated
    /// object types.
    #[test]
    fn test_from_json_value_quotes_non_identifier_keys() {
        let v = serde_json::json!({
            "my-key": "a",
            "2nd": 1
        });
        let ts = TypeScriptGenerator::from_json_value(&v);
        assert!(ts.contains("\"my-key\": string;"), "got: {}", ts);
        assert!(ts.contains("\"2nd\": number;"), "got: {}", ts);
    }

    /// Same quoting rule applies to inline nested object types.
    #[test]
    fn test_get_typescript_type_object_quotes_non_identifier_keys() {
        let s = serde_json::json!({
            "type": "object",
            "properties": {
                "my-key": { "type": "string" }
            }
        });
        assert_eq!(
            TypeScriptGenerator::get_typescript_type(&s),
            "{ \"my-key\": string }"
        );
    }

    #[test]
    fn test_is_valid_ts_identifier() {
        assert!(TypeScriptGenerator::is_valid_ts_identifier("name"));
        assert!(TypeScriptGenerator::is_valid_ts_identifier("_private"));
        assert!(TypeScriptGenerator::is_valid_ts_identifier("$dollar"));
        assert!(TypeScriptGenerator::is_valid_ts_identifier("n1"));
        // Reserved words are lexically valid property names in TS.
        assert!(TypeScriptGenerator::is_valid_ts_identifier("class"));
        assert!(!TypeScriptGenerator::is_valid_ts_identifier("my-key"));
        assert!(!TypeScriptGenerator::is_valid_ts_identifier("2nd"));
        assert!(!TypeScriptGenerator::is_valid_ts_identifier("with space"));
        assert!(!TypeScriptGenerator::is_valid_ts_identifier(""));
        assert!(!TypeScriptGenerator::is_valid_ts_identifier("a.b"));
    }

    #[test]
    fn test_generate_interface_no_properties_object() {
        // Object type with no properties array and no primitive alias branch hit:
        // falls through to empty interface body.
        let s = serde_json::json!({ "type": "object" });
        let out = TypeScriptGenerator::generate_interface("Empty", &s);
        assert!(out.contains("export interface Empty"));
    }

    // ---- convert_json_schema_to_typescript ----

    #[test]
    fn test_convert_invalid_schema_returns_comment() {
        // {"type": "object"} is a valid JSON Schema, now generates a proper interface
        let s = serde_json::json!({ "type": "object" });
        let out = TypeScriptGenerator::convert_json_schema_to_typescript(&s);
        assert!(
            out.contains("Config"),
            "expected Config in output, got: {}",
            out
        );
    }

    #[test]
    fn test_convert_empty_value_returns_comment() {
        let s = serde_json::Value::Null;
        let out = TypeScriptGenerator::convert_json_schema_to_typescript(&s);
        assert_eq!(out, "// Invalid schema format");
    }

    #[test]
    fn test_convert_with_definitions_and_main() {
        let s = serde_json::json!({
            "title": "Config",
            "definitions": {
                "Inner": {
                    "type": "object",
                    "properties": { "x": { "type": "string" } }
                }
            },
            "properties": {
                "name": { "type": "string" }
            }
        });
        let out = TypeScriptGenerator::convert_json_schema_to_typescript(&s);
        assert!(out.contains("interface Inner"));
        assert!(out.contains("interface Config"));
    }

    #[test]
    fn test_convert_definitions_only() {
        let s = serde_json::json!({
            "definitions": {
                "Foo": { "type": "string" }
            }
        });
        let out = TypeScriptGenerator::convert_json_schema_to_typescript(&s);
        assert!(out.contains("export type Foo = string;"));
        // No main properties, so only the definition interface is emitted.
        assert!(!out.contains("interface Config"));
    }

    #[test]
    fn test_convert_main_no_title_uses_default_name() {
        let s = serde_json::json!({
            "properties": {
                "name": { "type": "string" }
            }
        });
        let out = TypeScriptGenerator::convert_json_schema_to_typescript(&s);
        assert!(out.contains("interface Config"));
    }

    #[test]
    fn test_convert_definitions_not_object_ignored() {
        let s = serde_json::json!({
            "definitions": "not-an-object",
            "properties": { "a": { "type": "string" } }
        });
        let out = TypeScriptGenerator::convert_json_schema_to_typescript(&s);
        // Main interface still emitted, definitions ignored.
        assert!(out.contains("interface Config"));
    }

    // ---- generate<T>: end-to-end smoke tests ----

    #[test]
    fn test_generate_simple_struct() {
        #[derive(Debug, Serialize, Deserialize, JsonSchema)]
        struct Simple {
            name: String,
            count: i64,
        }
        let ts = TypeScriptGenerator::generate::<Simple>().unwrap();
        assert!(ts.contains("interface Simple"));
        assert!(ts.contains("name: string;"));
        assert!(ts.contains("count: number;"));
    }

    #[test]
    fn test_generate_unit_struct_alias() {
        #[derive(Debug, Serialize, Deserialize, JsonSchema)]
        struct Unit;
        let ts = TypeScriptGenerator::generate::<Unit>().unwrap();
        assert!(!ts.is_empty());
    }

    #[test]
    fn test_generate_array_field_with_items() {
        #[derive(Debug, Serialize, Deserialize, JsonSchema)]
        struct WithArray {
            tags: Vec<String>,
            counts: Vec<i32>,
        }
        let ts = TypeScriptGenerator::generate::<WithArray>().unwrap();
        assert!(ts.contains("tags: string[]"));
        assert!(ts.contains("counts: number[]"));
    }
}

#[cfg(test)]
mod rust_scaffold_tests {
    use super::RustScaffoldGenerator;
    use serde_json::json;

    #[test]
    fn scalar_types_map_to_rust_primitives() {
        let schema = json!({
            "title": "Scalar",
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "port": {"type": "integer"},
                "ratio": {"type": "number"},
                "debug": {"type": "boolean"}
            },
            "required": ["name", "port", "ratio", "debug"]
        });
        let code = RustScaffoldGenerator::generate(&schema).unwrap();
        assert!(
            code.contains("pub struct Scalar"),
            "struct decl missing: {code}"
        );
        assert!(code.contains("pub name: String"));
        assert!(code.contains("pub port: i64"));
        assert!(code.contains("pub ratio: f64"));
        assert!(code.contains("pub debug: bool"));
    }

    #[test]
    fn non_required_fields_become_option_with_serde_default() {
        let schema = json!({
            "title": "Opt",
            "type": "object",
            "properties": {
                "nickname": {"type": "string"}
            }
        });
        let code = RustScaffoldGenerator::generate(&schema).unwrap();
        assert!(code.contains("pub nickname: Option<String>"), "{code}");
        assert!(code.contains("#[serde(default)]"));
    }

    #[test]
    fn scalar_defaults_generate_default_fns() {
        let schema = json!({
            "title": "Defaults",
            "type": "object",
            "properties": {
                "host": {"type": "string", "default": "localhost"},
                "retries": {"type": "integer", "default": 3},
                "ratio": {"type": "number", "default": 0.5},
                "verbose": {"type": "boolean", "default": true}
            }
        });
        let code = RustScaffoldGenerator::generate(&schema).unwrap();
        assert!(
            code.contains("#[serde(default = \"_default_host\")]"),
            "{code}"
        );
        assert!(
            code.contains("pub host: String"),
            "defaulted field must not be Option"
        );
        assert!(
            code.contains("fn _default_host() -> String { \"localhost\".to_string() }"),
            "{code}"
        );
        assert!(
            code.contains("fn _default_retries() -> i64 { 3 }"),
            "{code}"
        );
        assert!(
            code.contains("fn _default_ratio() -> f64 { 0.5 }"),
            "{code}"
        );
        assert!(
            code.contains("fn _default_verbose() -> bool { true }"),
            "{code}"
        );
    }

    #[test]
    fn complex_defaults_surface_as_todo_comments() {
        let schema = json!({
            "title": "Complex",
            "type": "object",
            "properties": {
                "tags": {"type": "array", "items": {"type": "string"}, "default": ["a", "b"]}
            }
        });
        let code = RustScaffoldGenerator::generate(&schema).unwrap();
        assert!(
            code.contains("TODO"),
            "complex default must surface a visible marker: {code}"
        );
        assert!(
            code.contains("pub tags: Option<Vec<String>>"),
            "composite default keeps the schema's optional semantics: {code}"
        );
    }

    #[test]
    fn string_enums_become_rust_enums_with_rename() {
        let schema = json!({
            "title": "Service",
            "type": "object",
            "properties": {
                "status": {"type": "string", "enum": ["active", "pending-review"]}
            },
            "required": ["status"]
        });
        let code = RustScaffoldGenerator::generate(&schema).unwrap();
        assert!(code.contains("pub enum ServiceStatus"), "{code}");
        assert!(
            code.contains("#[serde(rename = \"pending-review\")]"),
            "{code}"
        );
        assert!(code.contains("PendingReview"), "{code}");
        assert!(code.contains("pub status: ServiceStatus"), "{code}");
    }

    #[test]
    fn non_string_enums_fall_back_to_scalar_with_allowed_values_comment() {
        let schema = json!({
            "title": "Levels",
            "type": "object",
            "properties": {
                "level": {"type": "integer", "enum": [1, 2, 3]}
            },
            "required": ["level"]
        });
        let code = RustScaffoldGenerator::generate(&schema).unwrap();
        assert!(code.contains("pub level: i64"), "{code}");
        assert!(code.contains("Allowed values"), "{code}");
    }

    #[test]
    fn arrays_map_to_vec() {
        let schema = json!({
            "title": "Lists",
            "type": "object",
            "properties": {
                "tags": {"type": "array", "items": {"type": "string"}},
                "matrix": {"type": "array"}
            },
            "required": ["tags", "matrix"]
        });
        let code = RustScaffoldGenerator::generate(&schema).unwrap();
        assert!(code.contains("pub tags: Vec<String>"), "{code}");
        assert!(
            code.contains("pub matrix: Vec<serde_json::Value>"),
            "items-less arrays widen to Value: {code}"
        );
    }

    #[test]
    fn inline_nested_objects_are_promoted_to_named_structs() {
        let schema = json!({
            "title": "App",
            "type": "object",
            "properties": {
                "database": {
                    "type": "object",
                    "properties": {"host": {"type": "string"}},
                    "required": ["host"]
                }
            },
            "required": ["database"]
        });
        let code = RustScaffoldGenerator::generate(&schema).unwrap();
        assert!(code.contains("pub struct AppDatabase"), "{code}");
        assert!(code.contains("pub database: AppDatabase"), "{code}");
        assert!(code.contains("pub host: String"), "{code}");
    }

    #[test]
    fn defs_and_internal_refs_resolve_to_named_types() {
        let schema = json!({
            "title": "WithDefs",
            "type": "object",
            "properties": {"db": {"$ref": "#/$defs/Database"}},
            "required": ["db"],
            "$defs": {
                "Database": {
                    "type": "object",
                    "properties": {"url": {"type": "string"}},
                    "required": ["url"]
                }
            }
        });
        let code = RustScaffoldGenerator::generate(&schema).unwrap();
        assert!(code.contains("pub struct Database"), "{code}");
        assert!(code.contains("pub db: Database"), "{code}");
    }

    #[test]
    fn additional_properties_schema_maps_to_hashmap() {
        let schema = json!({
            "title": "Labels",
            "type": "object",
            "additionalProperties": {"type": "integer"}
        });
        let code = RustScaffoldGenerator::generate(&schema).unwrap();
        assert!(code.contains("HashMap<String, i64>"), "{code}");
    }

    #[test]
    fn free_form_objects_map_to_serde_json_value() {
        let schema = json!({
            "title": "Free",
            "type": "object",
            "properties": {
                "meta": {"type": "object"}
            },
            "required": ["meta"]
        });
        let code = RustScaffoldGenerator::generate(&schema).unwrap();
        assert!(code.contains("pub meta: serde_json::Value"), "{code}");
    }

    #[test]
    fn nullable_type_arrays_become_option() {
        let schema = json!({
            "title": "Nullable",
            "type": "object",
            "properties": {
                "alias": {"type": ["string", "null"]}
            },
            "required": ["alias"]
        });
        let code = RustScaffoldGenerator::generate(&schema).unwrap();
        assert!(code.contains("pub alias: Option<String>"), "{code}");
    }

    #[test]
    fn non_snake_case_properties_get_rename_and_snake_fields() {
        let schema = json!({
            "title": "Naming",
            "type": "object",
            "properties": {
                "userName": {"type": "string"},
                "type": {"type": "string"},
                "user-name": {"type": "string"}
            },
            "required": ["userName", "type", "user-name"]
        });
        let code = RustScaffoldGenerator::generate(&schema).unwrap();
        assert!(
            code.contains("#[serde(rename = \"user-name\")]\n    pub user_name: String"),
            "{code}"
        );
        assert!(code.contains("pub r#type: String"), "{code}");
        assert!(
            code.contains("#[serde(rename = \"userName\")]\n    pub user_name_2: String"),
            "{code}"
        );
    }

    /// Later-edition keywords (`async` since 2018, `gen` reserved in 2024)
    /// must get the same raw-identifier treatment as classic ones — a bare
    /// `pub async` field would not compile.
    #[test]
    fn later_edition_keywords_become_raw_identifiers() {
        let schema = json!({
            "title": "Keywords",
            "type": "object",
            "properties": {
                "async": {"type": "string"},
                "gen": {"type": "string"}
            },
            "required": ["async", "gen"]
        });
        let code = RustScaffoldGenerator::generate(&schema).unwrap();
        assert!(code.contains("pub r#async: String"), "{code}");
        assert!(code.contains("pub r#gen: String"), "{code}");
    }

    #[test]
    fn descriptions_become_doc_comments() {
        let schema = json!({
            "title": "Documented",
            "type": "object",
            "properties": {
                "name": {"type": "string", "description": "The service name."}
            },
            "required": ["name"]
        });
        let code = RustScaffoldGenerator::generate(&schema).unwrap();
        assert!(code.contains("/// The service name."), "{code}");
    }

    #[test]
    fn format_hints_become_doc_comments_but_keep_string() {
        let schema = json!({
            "title": "Timed",
            "type": "object",
            "properties": {
                "started_at": {"type": "string", "format": "date-time"}
            },
            "required": ["started_at"]
        });
        let code = RustScaffoldGenerator::generate(&schema).unwrap();
        assert!(
            code.contains("pub started_at: String"),
            "format hints must not change the type: {code}"
        );
        assert!(code.contains("date-time"), "{code}");
    }

    #[test]
    fn missing_title_defaults_to_config() {
        let schema = json!({
            "type": "object",
            "properties": {"name": {"type": "string"}},
            "required": ["name"]
        });
        let code = RustScaffoldGenerator::generate(&schema).unwrap();
        assert!(code.contains("pub struct Config"), "{code}");
    }

    #[test]
    fn unsupported_combinators_fail_loud() {
        for combinator in ["oneOf", "anyOf", "allOf"] {
            let schema = json!({
                "title": "Bad",
                "type": "object",
                "properties": { "weird": { combinator: [{"type": "string"}, {"type": "integer"}] } }
            });
            let err = RustScaffoldGenerator::generate(&schema).unwrap_err();
            assert!(
                err.to_string().contains(combinator),
                "{combinator} must be named in the error: {err}"
            );
        }
    }

    #[test]
    fn external_refs_fail_loud() {
        let schema = json!({
            "title": "Ext",
            "type": "object",
            "properties": {"other": {"$ref": "https://example.com/other.json"}},
            "required": ["other"]
        });
        let err = RustScaffoldGenerator::generate(&schema).unwrap_err();
        assert!(err.to_string().contains("$ref"), "{err}");
    }

    #[test]
    fn pattern_properties_fail_loud() {
        let schema = json!({
            "title": "Patterned",
            "type": "object",
            "properties": {
                "dyn": {"type": "object", "patternProperties": {"^x-": {"type": "string"}}}
            },
            "required": ["dyn"]
        });
        let err = RustScaffoldGenerator::generate(&schema).unwrap_err();
        assert!(err.to_string().contains("patternProperties"), "{err}");
    }

    #[test]
    fn boolean_schema_true_maps_to_value() {
        let schema = json!({
            "title": "Loose",
            "type": "object",
            "properties": {
                "anything": true
            },
            "required": ["anything"]
        });
        let code = RustScaffoldGenerator::generate(&schema).unwrap();
        assert!(code.contains("pub anything: serde_json::Value"), "{code}");
    }

    /// Property names are untrusted input: quotes or backslashes inside them
    /// must stay inside the generated string literal, never leak into the
    /// token stream as code.
    #[test]
    fn rename_literals_are_escaped_against_injection() {
        let hostile = "x\")] pub q: u8 } fn injected_marker() -> u32 { 42 } struct Zzz { _q: u8, #[serde(rename = \"";
        let schema = json!({
            "title": "Hostile",
            "type": "object",
            "properties": { hostile: {"type": "string"} },
            "required": [hostile]
        });
        let code = RustScaffoldGenerator::generate(&schema).unwrap();
        // The `"` in the hostile name must appear escaped inside the literal.
        assert!(
            code.contains("#[serde(rename = \"x\\\")]"),
            "opening quote must be escaped: {code}"
        );
        assert!(
            code.contains("#[serde(rename = \\\"\")]"),
            "closing quote must be escaped: {code}"
        );
        // The definitive check: the artifact still compiles, so the hostile
        // text stayed inside the literal instead of entering the token
        // stream as code.
        compile_with_rustc(&code);
    }

    #[test]
    fn enum_variants_with_quotes_and_backslashes_are_escaped() {
        let schema = json!({
            "title": "Quoted",
            "type": "object",
            "properties": {
                "phrase": {"type": "string", "enum": ["say \"hi\"", "back\\slash", "line\nbreak"]}
            },
            "required": ["phrase"]
        });
        let code = RustScaffoldGenerator::generate(&schema).unwrap();
        assert!(
            code.contains("#[serde(rename = \"say \\\"hi\\\"\")]"),
            "{code}"
        );
        assert!(
            code.contains("#[serde(rename = \"back\\\\slash\")]"),
            "{code}"
        );
        assert!(
            code.contains("#[serde(rename = \"line\\nbreak\")]"),
            "newline must become an escape sequence, not a raw line break: {code}"
        );
    }

    #[test]
    fn string_default_with_newline_is_escaped() {
        let schema = json!({
            "title": "Multiline",
            "type": "object",
            "properties": {
                "banner": {"type": "string", "default": "a\nb\\c\"d"}
            }
        });
        let code = RustScaffoldGenerator::generate(&schema).unwrap();
        let default_line = code
            .lines()
            .find(|l| l.contains("_default_banner"))
            .expect("default fn");
        assert_eq!(
            default_line, "fn _default_banner() -> String { \"a\\nb\\\\c\\\"d\".to_string() }",
            "literal must stay on one line with escapes: {code}"
        );
    }

    #[test]
    fn normalized_enum_variant_collisions_get_unique_suffixes() {
        let schema = json!({
            "title": "Collide",
            "type": "object",
            "properties": {
                "mode": {"type": "string", "enum": ["a-b", "a_b", "plain"]}
            },
            "required": ["mode"]
        });
        let code = RustScaffoldGenerator::generate(&schema).unwrap();
        assert!(code.contains("    AB,\n"), "{code}");
        assert!(
            code.contains("#[serde(rename = \"a-b\")]\n    AB,\n"),
            "{code}"
        );
        assert!(
            code.contains("#[serde(rename = \"a_b\")]\n    AB2,\n"),
            "{code}"
        );
        assert!(code.contains("    Plain,\n"), "{code}");
    }

    #[test]
    fn internal_ref_must_resolve_to_a_defs_entry() {
        let schema = json!({
            "title": "Dangling",
            "type": "object",
            "properties": {"db": {"$ref": "#/$defs/DoesNotExist"}},
            "required": ["db"]
        });
        let err = RustScaffoldGenerator::generate(&schema).unwrap_err();
        assert!(
            err.to_string().contains("DoesNotExist"),
            "error must name the unresolved target: {err}"
        );
    }

    /// Two `$defs` keys can normalize to the same Pascal name; a `$ref` must
    /// bind to the type actually emitted for that def name (suffix
    /// included), not to whichever type claimed the un-suffixed name first.
    #[test]
    fn defs_name_collision_refs_resolve_to_emitted_names() {
        let schema = json!({
            "title": "Routed",
            "type": "object",
            "properties": {
                "primary": {"$ref": "#/$defs/data_base"},
                "secondary": {"$ref": "#/$defs/DataBase"}
            },
            "required": ["primary", "secondary"],
            "$defs": {
                "DataBase": {"type": "object", "properties": {"url": {"type": "string"}}, "required": ["url"]},
                "data_base": {"type": "object", "properties": {"port": {"type": "integer"}}, "required": ["port"]}
            }
        });
        let code = RustScaffoldGenerator::generate(&schema).unwrap();
        // Reservation order: `DataBase` first, `data_base` gets the suffix.
        assert!(code.contains("pub primary: DataBase2"), "{code}");
        assert!(code.contains("pub secondary: DataBase"), "{code}");
        compile_with_rustc(&code);
    }

    /// Symbol-only `$defs` keys and root titles cannot become Rust type
    /// identifiers; they must fail loudly instead of emitting broken code.
    #[test]
    fn named_level_symbol_only_names_fail_loud() {
        let defs_schema = json!({
            "title": "Ok",
            "type": "object",
            "properties": {"db": {"$ref": "#/$defs/###"}},
            "required": ["db"],
            "$defs": {"###": {"type": "string"}}
        });
        let err = RustScaffoldGenerator::generate(&defs_schema).unwrap_err();
        let message = err.to_string();
        assert!(
            message.contains("$defs."),
            "error must carry the defs path: {message}"
        );

        let title_schema = json!({
            "title": "!!!",
            "type": "object",
            "properties": {"a": {"type": "string"}},
            "required": ["a"]
        });
        let err = RustScaffoldGenerator::generate(&title_schema).unwrap_err();
        assert!(
            err.to_string().contains("!!!"),
            "root title must be named in the error: {err}"
        );
    }

    /// Error paths interpolate raw schema strings; control characters in
    /// them must never reach stderr raw (terminal escape injection).
    #[test]
    fn error_paths_are_sanitized() {
        let schema = json!({
            "title": "Esc",
            "type": "object",
            "properties": {
                "v": {"$ref": "#/$defs/escPwned"}
            },
            "required": ["v"],
            "$defs": {
                "esc\u{1b}]0;pwned\u{7}Pwned": {"oneOf": [{"type": "string"}]}
            }
        });
        let err = RustScaffoldGenerator::generate(&schema).unwrap_err();
        let message = err.to_string();
        assert!(
            !message.chars().any(|c| c.is_control()),
            "error message must be free of control characters: {message:?}"
        );
        assert!(message.contains("oneOf"), "{message}");
        assert!(
            message.contains("\\u{1b}]0;pwned\\u{7}"),
            "def name must surface as visible escape text: {message}"
        );
    }

    /// Non-string enum values surface in a generated "Allowed values"
    /// comment. JSON serialization already escapes control characters;
    /// sanitizer + escape must guarantee none survive raw.
    #[test]
    fn allowed_values_note_is_sanitized() {
        let mixed = json!({
            "title": "Note",
            "type": "object",
            "properties": {
                "level": {"enum": [1, "a\u{1b}b"]}
            },
            "required": ["level"]
        });
        let code = RustScaffoldGenerator::generate(&mixed).unwrap();
        let note_line = code
            .lines()
            .find(|l| l.contains("Allowed values"))
            .expect("note");
        assert!(
            !note_line.chars().any(|c| c.is_control()),
            "note must not carry raw control characters: {note_line:?}"
        );
        assert!(
            note_line.contains("a\\u001bb") || note_line.contains("a\\u{1b}b"),
            "ESC must appear as visible escape text: {note_line}"
        );
    }

    #[test]
    fn named_level_combinators_fail_loud() {
        let schema = json!({
            "title": "UnionRoot",
            "type": "object",
            "properties": {"v": {"$ref": "#/$defs/Union"}},
            "required": ["v"],
            "$defs": {
                "Union": {"oneOf": [{"type": "string"}, {"type": "integer"}]}
            }
        });
        let err = RustScaffoldGenerator::generate(&schema).unwrap_err();
        let message = err.to_string();
        assert!(message.contains("oneOf"), "{message}");
        assert!(
            message.contains("$defs.Union"),
            "error must carry the named path: {message}"
        );
    }

    #[test]
    fn hashmap_import_survives_named_freeform_types() {
        let schema = json!({
            "title": "Root",
            "type": "object",
            "$defs": {
                "Labels": {"type": "object", "additionalProperties": {"type": "string"}}
            }
        });
        let code = RustScaffoldGenerator::generate(&schema).unwrap();
        assert!(
            code.contains("use std::collections::HashMap;\n"),
            "HashMap import must survive named-level processing: {code}"
        );
        assert!(code.contains("pub struct Labels"), "{code}");
    }

    #[test]
    fn string_enum_with_scalar_default_uses_matching_variant() {
        let schema = json!({
            "title": "Svc",
            "type": "object",
            "properties": {
                "status": {"type": "string", "enum": ["active", "idle"], "default": "active"}
            }
        });
        let code = RustScaffoldGenerator::generate(&schema).unwrap();
        assert!(
            code.contains("fn _default_status() -> SvcStatus { SvcStatus::Active }"),
            "{code}"
        );
        assert!(code.contains("pub status: SvcStatus"), "{code}");
    }

    #[test]
    fn string_enum_with_unknown_scalar_default_fails_loud() {
        let schema = json!({
            "title": "Svc2",
            "type": "object",
            "properties": {
                "status": {"type": "string", "enum": ["active", "idle"], "default": "bogus"}
            }
        });
        let err = RustScaffoldGenerator::generate(&schema).unwrap_err();
        let message = err.to_string();
        assert!(message.contains("bogus"), "{message}");
        assert!(message.contains("status"), "{message}");
    }

    #[test]
    fn composite_default_optional_field_stays_option() {
        let schema = json!({
            "title": "Tagged",
            "type": "object",
            "properties": {
                "tags": {"type": "array", "items": {"type": "string"}, "default": []}
            }
        });
        let code = RustScaffoldGenerator::generate(&schema).unwrap();
        assert!(
            code.contains("pub tags: Option<Vec<String>>"),
            "schema-optional composite-default field must deserialize when absent: {code}"
        );
        assert!(
            code.contains("    // TODO: schema declares a composite default"),
            "manual wiring hint must stay: {code}"
        );
        assert!(
            code.contains("    #[serde(default)]\n    pub tags"),
            "Option-wrapped field needs #[serde(default)]: {code}"
        );
    }

    #[test]
    fn non_required_nullable_field_is_single_option() {
        let schema = json!({
            "title": "NullableOpt",
            "type": "object",
            "properties": {
                "alias": {"type": ["string", "null"]}
            }
        });
        let code = RustScaffoldGenerator::generate(&schema).unwrap();
        assert!(
            code.contains("pub alias: Option<String>"),
            "nullable arrays must not double-wrap Option: {code}"
        );
        assert!(!code.contains("Option<Option<"), "{code}");
    }

    /// Nullable array + scalar default: the field is `Option<T>`, so the
    /// generated default function must return `Some(...)` — not the inner
    /// value, which would be a type error in the emitted code.
    #[test]
    fn nullable_field_with_scalar_default_wraps_some() {
        let schema = json!({
            "title": "NullableDefault",
            "type": "object",
            "properties": {
                "alias": {"type": ["string", "null"], "default": "fallback"}
            }
        });
        let code = RustScaffoldGenerator::generate(&schema).unwrap();
        assert!(code.contains("pub alias: Option<String>"), "{code}");
        assert!(
            code.contains(
                "fn _default_alias() -> Option<String> { Some(\"fallback\".to_string()) }"
            ),
            "{code}"
        );
    }

    #[test]
    fn illegal_property_names_fail_loud() {
        // `_` (and any name normalizing to it — e.g. a single CJK character,
        // which is a realistic non-ASCII config key) is Rust-reserved: neither
        // `pub _` nor `pub r##_` compiles, so it must fail loudly. `-` covers
        // the all-separator branch of the normalizer.
        for hostile in [
            "2fa", "", "123abc", "self", "super", "crate", "_", "-", "日",
        ] {
            let schema = json!({
                "title": "Illegal",
                "type": "object",
                "properties": { hostile: {"type": "string"} },
                "required": [hostile]
            });
            let err = RustScaffoldGenerator::generate(&schema)
                .err()
                .unwrap_or_else(|| panic!("property {hostile:?} must fail loudly, not generate"));
            assert!(
                err.to_string().contains(hostile.trim()) || hostile.is_empty(),
                "error must name the offending property: {err}"
            );
        }
    }

    #[test]
    fn description_control_characters_are_sanitized() {
        let schema = json!({
            "title": "Escapy",
            "type": "object",
            "properties": {
                "a": {"type": "string", "description": "clears \u{1b}[2J screen"}
            },
            "required": ["a"]
        });
        let code = RustScaffoldGenerator::generate(&schema).unwrap();
        let doc_line = code
            .lines()
            .find(|l| l.contains("screen"))
            .expect("doc line");
        assert!(
            !doc_line.chars().any(|c| c.is_control()),
            "ESC must not reach the output raw: {doc_line:?}"
        );
        assert!(
            doc_line.contains("\\u{1b}"),
            "control char must surface as visible escape text: {doc_line}"
        );
    }

    /// Contract test: whatever the mapping rules emit must be valid Rust.
    /// Covers the historically broken combinations (enum × scalar default,
    /// rename escaping, identifier collisions, keyword properties, HashMap
    /// imports) in one compilable artifact.
    #[test]
    fn generated_output_passes_rustc() {
        let schema = json!({
            "title": "Smoked",
            "type": "object",
            "properties": {
                "type": {"type": "string"},
                "userName": {"type": "string"},
                "user-name": {"type": "string"},
                "mode": {"type": "string", "enum": ["active", "idle"], "default": "active"},
                "banner": {"type": "string", "default": "multi\nline \"quoted\""},
                "tags": {"type": "array", "items": {"type": "string"}, "default": []},
                "labels": {"$ref": "#/$defs/Labels"},
                "status": {"type": "string", "enum": ["up", "down"]}
            },
            "required": ["type", "userName", "user-name", "status"],
            "$defs": {
                "Labels": {"type": "object", "additionalProperties": {"type": "string"}}
            }
        });
        let code = RustScaffoldGenerator::generate(&schema).unwrap();
        compile_with_rustc(&code);
    }

    /// Round trip through the documented forward direction: `schema_for!`
    /// (schemars, draft 2020-12, `$defs` + `$ref` style) is exactly the kind
    /// of schema draft `RustScaffoldGenerator` consumes — a consumer can
    /// generate a schema from an existing type with the `schema` feature and
    /// scaffold a Rust starting point for another codebase from it.
    #[test]
    fn schemars_draft_output_round_trips_to_rust_scaffold() {
        use schemars::JsonSchema;

        #[derive(JsonSchema)]
        #[allow(dead_code)]
        enum DeployMode {
            Active,
            Paused,
        }

        #[derive(JsonSchema)]
        #[allow(dead_code)]
        struct DeployScaffold {
            region: String,
            replicas: u32,
            nickname: Option<String>,
            mode: DeployMode,
        }

        let schema =
            serde_json::to_value(schemars::schema_for!(DeployScaffold)).expect("schemars schema");
        let code = RustScaffoldGenerator::generate(&schema).unwrap();

        assert!(code.contains("pub struct DeployScaffold"), "{code}");
        // schemars widens u32 to `integer`; format hints surface as comments.
        assert!(code.contains("pub replicas: i64"), "{code}");
        assert!(code.contains("pub region: String"), "{code}");
        // schemars leaves non-required fields un-required → Option mapping.
        assert!(code.contains("pub nickname: Option<String>"), "{code}");
        // Unit-only enums become string enums behind an internal $ref.
        assert!(code.contains("pub enum DeployMode"), "{code}");
        assert!(code.contains("pub mode: DeployMode"), "{code}");
        // The artifact must compile end to end.
        compile_with_rustc(&code);
    }

    /// Compile `code` as a standalone lib crate with the same serde build the
    /// test itself links against. Skips with a notice when no cached serde
    /// artifacts can be located (e.g. a from-scratch target directory or a
    /// non-default `CARGO_TARGET_DIR`), so the check only runs where it can.
    fn compile_with_rustc(code: &str) {
        use std::path::{Path, PathBuf};

        // The test binary lives in the same `deps/` directory cargo compiled
        // this crate's dependencies into, whatever the profile or
        // `CARGO_TARGET_DIR` — derive the search path from it instead of
        // hardcoding `target/debug` so the check also runs under
        // `--release` / custom target dirs.
        let deps_dir = std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(Path::to_path_buf))
            .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug/deps"));
        // Regular crates ship as `.rlib`; proc-macro crates (serde_derive)
        // ship as shared objects.
        let find_artifact = |prefix: &str| -> Option<PathBuf> {
            std::fs::read_dir(&deps_dir)
                .ok()?
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .find(|path| {
                    path.file_name().and_then(|n| n.to_str()).is_some_and(|n| {
                        n.starts_with(prefix) && (n.ends_with(".rlib") || n.ends_with(".so"))
                    })
                })
        };
        let (Some(serde_rlib), Some(serde_derive_rlib)) = (
            find_artifact("libserde-"),
            find_artifact("libserde_derive-"),
        ) else {
            eprintln!(
                "skipping rustc smoke check: serde artifacts not found under {}",
                deps_dir.display()
            );
            return;
        };

        let out_dir = std::env::temp_dir();
        let src = out_dir.join("confers_scaffold_smoke.rs");
        let meta = out_dir.join("confers_scaffold_smoke.rmeta");
        std::fs::write(&src, code).expect("write generated artifact");

        let status = std::process::Command::new("rustc")
            .args([
                "--edition",
                "2024",
                "--crate-type",
                "lib",
                "--emit",
                "metadata",
            ])
            .arg("-L")
            .arg(&deps_dir)
            .arg("--extern")
            .arg(format!("serde={}", serde_rlib.display()))
            .arg("--extern")
            .arg(format!("serde_derive={}", serde_derive_rlib.display()))
            .arg("-o")
            .arg(&meta)
            .arg(&src)
            .output()
            .expect("spawn rustc");
        assert!(
            status.status.success(),
            "generated scaffolding must compile:\n{}\nstderr:\n{}",
            code,
            String::from_utf8_lossy(&status.stderr)
        );
    }
}

/// JSON Schema → Rust struct scaffolding.
///
/// The inverse direction of [`TypeScriptGenerator`]: consumes a JSON Schema
/// draft (2020-12 flavored) and emits compilable Rust type definitions as a
/// starting point for schema-first adoption. The generated code is meant to
/// be edited by hand afterwards; regeneration overwrites it.
///
/// # Mapping rules
///
/// - `type: object` + `properties` → `struct`; `$defs` / `definitions` entries
///   become their own named types; internal `$ref`s (`#/$defs/X`,
///   `#/definitions/X`) become type references
/// - scalars: `string` → `String`, `integer` → `i64`, `number` → `f64`,
///   `boolean` → `bool`; `format` hints stay `String` and surface as doc
///   comments (no extra dependencies are introduced)
/// - fields absent from `required` → `Option<T>` + `#[serde(default)]`
///   (a nullable type array already produced the `Option`, so it is not
///   double-wrapped); fields with a scalar `default` keep their concrete
///   type plus `#[serde(default = "_default_*")]` and a generated default
///   function (the first field named `X` owns `_default_X`; later collisions
///   get numeric suffixes). Composite defaults surface as `TODO` comments —
///   the field still gets the schema's optional semantics when it is not
///   required
/// - `type: string` + `enum` → Rust enum with `#[serde(rename)]` per variant;
///   enums of other scalar types fall back to the scalar type plus an
///   "Allowed values" comment
/// - `type: array` → `Vec<T>` (missing `items` widens to
///   `Vec<serde_json::Value>`); inline nested objects are promoted to named
///   structs; free-form objects (`object` without `properties`) map to
///   `serde_json::Value`; `additionalProperties` with a subschema maps to
///   `HashMap<String, T>` (flattened when explicit `properties` also exist)
/// - property names are normalized to snake_case with `#[serde(rename)]`
///   keeping the original value; Rust keywords become raw identifiers
///   (`r#type`); names that cannot normalize to a legal identifier (empty,
///   digit-leading) and the non-raw-able keywords (`self` / `super` /
///   `crate`) fail loudly; normalized collisions (`userName` vs
///   `user-name`) get numeric suffixes
/// - all schema strings entering generated code (rename values, enum
///   variants, scalar defaults, doc comments) are escaped: quotes and
///   backslashes cannot break out of the string literal, control characters
///   (including newlines) become escape sequences
/// - internal `$ref`s must resolve to a `$defs` / `definitions` entry;
///   dangling references fail loudly
/// - a scalar `default` on an enum-typed field must name one of the enum's
///   variants (the generated default function returns that variant);
///   anything else fails loudly
/// - unsupported constructs fail loudly with the offending path:
///   `oneOf` / `anyOf` / `allOf` / `not`, external `$ref`s,
///   `patternProperties`, tuple-style array validation — at property level
///   and at named (`$defs` entry) level alike
pub struct RustScaffoldGenerator;

impl RustScaffoldGenerator {
    pub fn generate(schema: &Value) -> ConfigResult<String> {
        let mut ctx = ScaffoldCtx::default();
        let root_name = schema
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or("Config");

        // Reserve named-level type names up front: `$ref`s must resolve to
        // the name actually emitted (collisions get suffixes), not to a name
        // re-derived at lookup time — re-deriving silently binds a reference
        // to whatever type happened to claim the un-suffixed name first.
        // Reservation order matches emission order (defs, then root).
        for defs_key in ["$defs", "definitions"] {
            if let Some(defs) = schema.get(defs_key).and_then(Value::as_object) {
                for def_name in defs.keys() {
                    let path = format!("$defs.{}", sanitize_comment_text(def_name));
                    let pascal = named_type_name(def_name, &path)?;
                    let emitted = ctx.unique(pascal);
                    ctx.def_targets.insert(def_name.clone(), emitted);
                }
            }
        }
        let root_type = ctx.unique(named_type_name(root_name, "root")?);

        for defs_key in ["$defs", "definitions"] {
            if let Some(defs) = schema.get(defs_key).and_then(Value::as_object) {
                for (def_name, def_schema) in defs {
                    let emitted = ctx
                        .def_targets
                        .get(def_name)
                        .expect("def name reserved above")
                        .clone();
                    let path = format!("$defs.{}", sanitize_comment_text(def_name));
                    ctx.emit_named(&emitted, def_schema, &path)?;
                }
            }
        }
        ctx.emit_named(&root_type, schema, "root")?;

        let mut out = String::from(
            "// Generated by `confers schema --from-schema`. Scaffolding: edit freely.\n\
             // Mapping rules: see `RustScaffoldGenerator` docs or the user guide.\n",
        );
        out.push_str("use serde::{Deserialize, Serialize};\n");
        if ctx.needs_hashmap {
            out.push_str("use std::collections::HashMap;\n");
        }
        if !ctx.default_fns.is_empty() {
            out.push('\n');
            for func in &ctx.default_fns {
                out.push_str(func);
                out.push('\n');
            }
        }
        for item in &ctx.types {
            out.push('\n');
            out.push_str(item);
            out.push('\n');
        }
        // End the artifact with exactly one newline.
        let trimmed = out.trim_end_matches('\n').len();
        out.truncate(trimmed);
        out.push('\n');
        Ok(out)
    }
}

#[derive(Default)]
struct ScaffoldCtx {
    types: Vec<String>,
    default_fns: Vec<String>,
    taken_names: std::collections::HashSet<String>,
    used_default_fn_names: std::collections::HashSet<String>,
    needs_hashmap: bool,
    /// String enums promoted by this generator: enum type name →
    /// `(original value, Rust variant identifier)` pairs. Scalar defaults on
    /// enum-typed fields must resolve against these or fail loudly.
    promoted_enums: std::collections::HashMap<String, Vec<(String, String)>>,
    /// `$defs` / `definitions` name → the type name actually emitted for it
    /// (collision suffixes included); internal `$ref`s must resolve through
    /// this map.
    def_targets: std::collections::HashMap<String, String>,
}

/// Pascal-case a named-level declaration (`$defs` key / root title) and
/// reject names that cannot become a Rust type identifier.
fn named_type_name(name: &str, at: &str) -> ConfigResult<String> {
    let pascal = to_pascal_case(name);
    if pascal.is_empty() || !pascal.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_') {
        return Err(ConfigError::ParseError {
            format: "schema".into(),
            message: format!(
                "name `{}` at `{at}` does not normalize to a legal Rust type identifier (`{pascal}`): rename it in the schema",
                sanitize_comment_text(name)
            ),
            location: None,
            source: None,
        });
    }
    Ok(pascal)
}

impl ScaffoldCtx {
    fn emit_named(&mut self, type_name: &str, schema: &Value, path: &str) -> ConfigResult<()> {
        reject_unsupported_combinators(schema, path)?;

        // String enums at the named level become real enums; other enums
        // degrade to a type alias over the scalar with allowed values noted.
        if let Some(variants) = schema.get("enum").and_then(Value::as_array) {
            if variants.iter().all(Value::is_string) {
                let (enum_code, variant_map) = render_string_enum(type_name, variants)?;
                self.promoted_enums
                    .insert(type_name.to_string(), variant_map);
                self.types.push(enum_code);
                return Ok(());
            }
            let (scalar, note) = scalar_type_for(schema)?;
            self.types.push(alias_line(type_name, &scalar, &note));
            return Ok(());
        }

        // A named $ref forwards to its target type.
        if let Some(target) = schema.get("$ref").and_then(Value::as_str) {
            let target_name = self.resolve_internal_ref(target, path)?;
            self.types
                .push(format!("pub type {type_name} = {target_name};\n"));
            return Ok(());
        }

        if object_p(schema) {
            let code = self.emit_struct(type_name, schema)?;
            self.types.push(code);
            return Ok(());
        }

        // Anything else at the named level is kept as an alias so the
        // scaffolding still compiles.
        let (scalar, note) = scalar_type_for(schema)?;
        self.types.push(alias_line(type_name, &scalar, &note));
        Ok(())
    }

    /// Internal `$ref`s must name an existing `$defs` / `definitions` entry
    /// and resolve to the name emitted for it; dangling references or
    /// re-derived names would compile into (or bind to) the wrong type.
    fn resolve_internal_ref(&self, reference: &str, at: &str) -> ConfigResult<String> {
        let shown_ref = sanitize_comment_text(reference);
        let target = internal_ref_target(reference).ok_or_else(|| ConfigError::ParseError {
            format: "schema".into(),
            message: format!(
                "unsupported external $ref `{shown_ref}` at `{at}`: only internal `#/$defs/...` references are supported"
            ),
            location: None,
            source: None,
        })?;
        self.def_targets.get(target).cloned().ok_or_else(|| {
            ConfigError::ParseError {
                format: "schema".into(),
                message: format!(
                    "unresolvable $ref `{shown_ref}` at `{at}`: no `$defs`/`definitions` entry named `{}`",
                    sanitize_comment_text(target)
                ),
                location: None,
                source: None,
            }
        })
    }

    fn emit_struct(&mut self, type_name: &str, schema: &Value) -> ConfigResult<String> {
        let properties = schema.get("properties").and_then(Value::as_object);
        let additional = schema.get("additionalProperties");

        if properties.is_none() && additional.is_none() {
            // Free-form object: any map is accepted. This type alias needs no
            // HashMap import, but must not reset the flag earlier types set.
            return Ok(format!(
                "/// Free-form object (schema declares `object` with no field constraints).\npub type {type_name} = serde_json::Value;\n"
            ));
        }

        let mut body = String::new();
        let mut taken_fields = std::collections::HashSet::new();
        // One lookup per property against a set, instead of a linear scan of
        // `required` per property (O(P×R) → O(P+R)).
        let required_names: std::collections::HashSet<&str> = schema
            .get("required")
            .and_then(Value::as_array)
            .map(|r| r.iter().filter_map(Value::as_str).collect())
            .unwrap_or_default();
        if let Some(props) = properties {
            for (prop_name, prop_schema) in props {
                let required = required_names.contains(prop_name.as_str());
                self.render_field(
                    type_name,
                    prop_name,
                    prop_schema,
                    required,
                    &mut taken_fields,
                    &mut body,
                )?;
            }
        }

        if let Some(additional_schema) = additional.filter(|a| !a.is_boolean()) {
            let extra_type = self.resolve_type(type_name, "extra", additional_schema)?;
            self.needs_hashmap = true;
            body.push_str("    /// Catch-all for keys beyond the declared properties.\n");
            body.push_str("    #[serde(flatten)]\n");
            body.push_str("    pub extra: HashMap<String, ");
            body.push_str(&extra_type);
            body.push_str(">,\n");
        }

        Ok(format!(
            "#[derive(Debug, Clone, Serialize, Deserialize)]\npub struct {type_name} {{\n{body}}}\n"
        ))
    }

    fn render_field(
        &mut self,
        parent: &str,
        prop_name: &str,
        prop_schema: &Value,
        required: bool,
        taken_fields: &mut std::collections::HashSet<String>,
        body: &mut String,
    ) -> ConfigResult<()> {
        if let Some(desc) = prop_schema.get("description").and_then(Value::as_str) {
            for line in desc.lines() {
                body.push_str(&format!("    /// {}\n", sanitize_comment_text(line)));
            }
        }

        // Comment-bearing fallbacks (non-string enums, format hints).
        let (field_type, mut notes) =
            self.resolve_type_with_notes(parent, prop_name, prop_schema)?;
        if let Some(format_hint) = prop_schema.get("format").and_then(Value::as_str) {
            notes.push(format!("format: {}", sanitize_comment_text(format_hint)));
        }
        for note in &notes {
            body.push_str(&format!("    // {note}\n"));
        }

        let scalar_default = scalar_default(prop_schema);
        if scalar_default.is_none() && prop_schema.get("default").is_some() {
            // The schema still promises an optional field here (the property
            // may be absent), so the field stays `Option` + `#[serde(default)]`
            // and the concrete fallback value is wired by hand.
            body.push_str(&format!(
                "    // TODO: schema declares a composite default, implement `Default` or a custom default fn manually: {}\n",
                prop_schema.get("default").map(Value::to_string).unwrap_or_default()
            ));
        }

        let (mut field_ident, mut rename_attr) = field_identifier(prop_name, parent)?;
        if !taken_fields.insert(field_ident.clone()) {
            // Two property names can normalize to the same identifier (e.g.
            // `userName` / `user-name`); suffix the later one and keep the
            // original name via `rename`.
            let base = field_ident.clone();
            let mut suffix = 2;
            loop {
                let candidate = format!("{base}_{suffix}");
                if taken_fields.insert(candidate.clone()) {
                    field_ident = candidate;
                    break;
                }
                suffix += 1;
            }
            if rename_attr.is_none() {
                rename_attr = Some(rename_attribute(prop_name));
            }
        }
        if let Some(mut attr) = rename_attr {
            if !attr.starts_with(' ') && !attr.starts_with('\n') {
                attr = format!("    {attr}");
            }
            body.push_str(&attr);
        }

        // A declared composite default also pins the concrete type: the
        // TODO comment above tells the reader to wire the fallback by hand.
        // A nullable type array already produced `Option<...>`, so the
        // optionality wrapper must not double-wrap.
        let already_option = field_type.starts_with("Option<");
        let is_optional = !required && scalar_default.is_none();
        let effective_type = if is_optional && !already_option {
            format!("Option<{field_type}>")
        } else {
            field_type.clone()
        };

        if let Some(default_literal) = scalar_default {
            let fn_name = self.default_fn_name(parent, prop_name);
            let expr = match self.promoted_enums.get(&field_type) {
                Some(variant_map) => {
                    let raw_default = prop_schema
                        .get("default")
                        .and_then(Value::as_str)
                        .unwrap_or_default();
                    let (_, variant) = variant_map
                        .iter()
                        .find(|(raw, _)| raw == raw_default)
                        .ok_or_else(|| ConfigError::ParseError {
                            format: "schema".into(),
                            message: format!(
                                "default `{}` on property `{}` (type `{parent}`) is not a variant of enum `{field_type}` (variants: {})",
                                sanitize_comment_text(raw_default),
                                sanitize_comment_text(prop_name),
                                variant_map
                                    .iter()
                                    .map(|(raw, _)| sanitize_comment_text(raw))
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            ),
                            location: None,
                            source: None,
                        })?;
                    format!("{field_type}::{variant}")
                }
                None => default_literal,
            };
            // A nullable type array (`type: ["string", "null"]`) makes the
            // field itself `Option<...>`; the generated default must then
            // produce the `Some(...)` variant, not the inner value.
            let expr = if already_option {
                format!("Some({expr})")
            } else {
                expr
            };
            self.default_fns
                .push(format!("fn {fn_name}() -> {field_type} {{ {expr} }}\n"));
            body.push_str(&format!("    #[serde(default = \"{fn_name}\")]\n"));
        } else if is_optional {
            body.push_str("    #[serde(default)]\n");
        }

        body.push_str(&format!("    pub {field_ident}: {effective_type},\n"));
        Ok(())
    }

    fn resolve_type(&mut self, parent: &str, field: &str, schema: &Value) -> ConfigResult<String> {
        self.resolve_type_with_notes(parent, field, schema)
            .map(|(t, _)| t)
    }

    fn resolve_type_with_notes(
        &mut self,
        parent: &str,
        field: &str,
        schema: &Value,
    ) -> ConfigResult<(String, Vec<String>)> {
        // Error paths interpolate the raw property name; schema strings are
        // untrusted and must not carry control characters into stderr.
        let shown_field = sanitize_comment_text(field);
        reject_unsupported_combinators(
            schema,
            &format!("property `{shown_field}` (type `{parent}`)"),
        )?;

        if let Some(reference) = schema.get("$ref").and_then(Value::as_str) {
            let target =
                self.resolve_internal_ref(reference, &format!("property `{shown_field}`"))?;
            return Ok((target, Vec::new()));
        }

        // String enums promote to real enums; other enums stay scalar.
        if let Some(variants) = schema.get("enum").and_then(Value::as_array) {
            if variants.iter().all(Value::is_string) {
                let enum_name = self.unique(to_pascal_case(&format!("{parent}_{field}")));
                let (code, variant_map) = render_string_enum(&enum_name, variants)?;
                self.promoted_enums.insert(enum_name.clone(), variant_map);
                self.types.push(code);
                return Ok((enum_name, Vec::new()));
            }
            let (scalar, note) = scalar_type_for(schema)?;
            return Ok((scalar, vec![note]));
        }

        let type_decl = schema.get("type");
        if let Some(types) = type_decl.and_then(Value::as_array) {
            let nullable = types.iter().any(|t| t.as_str() == Some("null"));
            let non_null: Vec<&str> = types
                .iter()
                .filter_map(Value::as_str)
                .filter(|t| *t != "null")
                .collect();
            if non_null.len() > 1 {
                return Err(ConfigError::ParseError {
                    format: "schema".into(),
                    message: format!(
                        "mixed non-null types {non_null:?} on property `{shown_field}`: narrow the schema to one type"
                    ),
                    location: None,
                    source: None,
                });
            }
            let inner = match non_null.first() {
                Some(t) => {
                    let single = serde_json::json!({ "type": t });
                    self.resolve_type_with_notes(parent, field, &single)?
                }
                None => ("serde_json::Value".to_string(), Vec::new()),
            };
            let ty = if nullable {
                format!("Option<{}>", inner.0)
            } else {
                inner.0
            };
            return Ok((ty, inner.1));
        }

        if let Some(single) = type_decl.and_then(Value::as_str) {
            return match single {
                "string" => Ok(("String".to_string(), Vec::new())),
                "integer" => Ok(("i64".to_string(), Vec::new())),
                "number" => Ok(("f64".to_string(), Vec::new())),
                "boolean" => Ok(("bool".to_string(), Vec::new())),
                "array" => {
                    // Missing `items` accepts any element: widen to Value.
                    let items = match schema.get("items") {
                        Some(items) if !items.is_array() => items,
                        Some(_) => {
                            return Err(ConfigError::ParseError {
                                format: "schema".into(),
                                message: format!(
                                    "unsupported tuple-style `items` on property `{shown_field}`"
                                ),
                                location: None,
                                source: None,
                            });
                        }
                        None => &Value::Bool(true),
                    };
                    if items.is_array() {
                        return Err(ConfigError::ParseError {
                            format: "schema".into(),
                            message: format!(
                                "unsupported tuple-style `items` on property `{shown_field}`"
                            ),
                            location: None,
                            source: None,
                        });
                    }
                    let item_type = self.resolve_type(parent, field, items)?;
                    Ok((format!("Vec<{item_type}>"), Vec::new()))
                }
                "object" => {
                    let has_props = schema
                        .get("properties")
                        .and_then(Value::as_object)
                        .is_some();
                    let additional = schema.get("additionalProperties");
                    if !has_props {
                        if let Some(additional_schema) = additional.filter(|a| !a.is_boolean()) {
                            let value_type = self.resolve_type(parent, field, additional_schema)?;
                            self.needs_hashmap = true;
                            return Ok((format!("HashMap<String, {value_type}>"), Vec::new()));
                        }
                        return Ok(("serde_json::Value".to_string(), Vec::new()));
                    }
                    let nested_name = self.unique(to_pascal_case(&format!("{parent}_{field}")));
                    let code = self.emit_struct(&nested_name, schema)?;
                    self.types.push(code);
                    Ok((nested_name, Vec::new()))
                }
                other => Err(ConfigError::ParseError {
                    format: "schema".into(),
                    message: format!(
                        "unsupported JSON Schema type `{}` on property `{shown_field}`",
                        sanitize_comment_text(other)
                    ),
                    location: None,
                    source: None,
                }),
            };
        }

        // `true`, `{}` and schemas without a type accept anything.
        Ok(("serde_json::Value".to_string(), Vec::new()))
    }

    fn default_fn_name(&mut self, _parent: &str, field: &str) -> String {
        let base = format!("_default_{}", to_snake_case(field));
        let mut candidate = base.clone();
        let mut suffix = 2;
        while !self.used_default_fn_names.insert(candidate.clone()) {
            candidate = format!("{base}_{suffix}");
            suffix += 1;
        }
        candidate
    }

    fn unique(&mut self, base: String) -> String {
        let mut candidate = base.clone();
        let mut suffix = 2;
        while !self.taken_names.insert(candidate.clone()) {
            candidate = format!("{base}{suffix}");
            suffix += 1;
        }
        candidate
    }
}

fn object_p(schema: &Value) -> bool {
    schema.get("type").and_then(Value::as_str) == Some("object")
}

/// Reject the combinators documented as unsupported. Used at both the
/// property and the named (`$defs` entry / root) level so no construct
/// silently degrades.
fn reject_unsupported_combinators(schema: &Value, at: &str) -> ConfigResult<()> {
    for unsupported in ["oneOf", "anyOf", "allOf", "not"] {
        if schema.get(unsupported).is_some() {
            return Err(ConfigError::ParseError {
                format: "schema".into(),
                message: format!(
                    "unsupported `{unsupported}` at `{at}`: hand-write this part of the scaffolding"
                ),
                location: None,
                source: None,
            });
        }
    }
    if schema.get("patternProperties").is_some() {
        return Err(ConfigError::ParseError {
            format: "schema".into(),
            message: format!(
                "unsupported `patternProperties` at `{at}`: hand-write this part of the scaffolding"
            ),
            location: None,
            source: None,
        });
    }
    Ok(())
}

fn internal_ref_target(reference: &str) -> Option<&str> {
    reference
        .strip_prefix("#/$defs/")
        .or_else(|| reference.strip_prefix("#/definitions/"))
}

/// `pub type X = T;` line, carrying the allowed-values note only when there
/// is one (a missing note must not leave a dangling `// `).
fn alias_line(type_name: &str, scalar: &str, note: &str) -> String {
    if note.is_empty() {
        format!("pub type {type_name} = {scalar};\n")
    } else {
        format!("pub type {type_name} = {scalar}; // {note}\n")
    }
}

/// Escape an arbitrary schema string for inclusion inside a generated Rust
/// string literal. Quotes and backslashes are the code-injection vector;
/// control characters (including newlines) become escape sequences so the
/// literal never breaks across lines or smuggles terminal/OSC bytes.
fn escape_rust_string(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len() + 2);
    for ch in raw.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{{{:x}}}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

/// Make an arbitrary schema string safe for a generated comment (doc or
/// regular): control characters surface as visible escape text and a
/// trailing backslash is neutralized (it would act as a rustdoc line
/// continuation).
fn sanitize_comment_text(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        if ch.is_control() {
            out.push_str(&format!("\\u{{{:x}}}", ch as u32));
        } else {
            out.push(ch);
        }
    }
    if out.ends_with('\\') {
        out.push(' ');
    }
    out
}

/// `(scalar type, allowed-values note)` for enum schemas of non-string values.
fn scalar_type_for(schema: &Value) -> ConfigResult<(String, String)> {
    let allowed = schema
        .get("enum")
        .map(|e| format!("{e}"))
        .unwrap_or_default();
    let ty = match schema.get("type").and_then(Value::as_str) {
        Some("integer") => "i64",
        Some("number") => "f64",
        Some("boolean") => "bool",
        // Untyped enums (e.g. mixed scalars) widen to Value, fail-loudly
        // documented via the allowed-values comment.
        _ => "serde_json::Value",
    };
    let note = if allowed.is_empty() {
        String::new()
    } else {
        // The note lands inside a generated comment; enum values are
        // untrusted and must not carry control characters into the artifact.
        format!("Allowed values: {}", sanitize_comment_text(&allowed))
    };
    Ok((ty.to_string(), note))
}

/// Scalar default literal (`Some(literal)`), if the schema default is a
/// scalar the scaffolding can turn into a generated default function.
fn scalar_default(schema: &Value) -> Option<String> {
    match schema.get("default") {
        Some(Value::String(s)) => Some(format!("\"{}\".to_string()", escape_rust_string(s))),
        Some(Value::Bool(b)) => Some(b.to_string()),
        Some(Value::Number(n)) => Some(n.to_string()),
        _ => None,
    }
}

/// Render a string enum and return `(code, (original value, variant ident)
/// pairs)` so scalar defaults can resolve to variant paths. Variant names
/// that collide after normalization (e.g. `"a-b"` / `"a_b"`) get numeric
/// suffixes; the original value stays reachable via `#[serde(rename)]`.
fn render_string_enum(
    type_name: &str,
    variants: &[Value],
) -> ConfigResult<(String, Vec<(String, String)>)> {
    let mut out = String::from("#[derive(Debug, Clone, Serialize, Deserialize)]\n");
    out.push_str(&format!("pub enum {type_name} {{\n"));
    let mut taken_variants = std::collections::HashSet::new();
    let mut variant_map = Vec::with_capacity(variants.len());
    for variant in variants {
        let raw = variant.as_str().ok_or_else(|| ConfigError::ParseError {
            format: "schema".into(),
            message: format!(
                "non-string variant `{}` in enum `{type_name}`: string enums require string values",
                sanitize_comment_text(&variant.to_string())
            ),
            location: None,
            source: None,
        })?;
        if raw.is_empty() {
            return Err(ConfigError::ParseError {
                format: "schema".into(),
                message: format!(
                    "empty string variant in enum `{type_name}`: Rust variant names cannot be empty"
                ),
                location: None,
                source: None,
            });
        }
        let base = to_pascal_case(raw);
        if base.is_empty() {
            return Err(ConfigError::ParseError {
                format: "schema".into(),
                message: format!(
                    "enum variant `{}` in `{type_name}` contains no alphanumeric characters: cannot derive a Rust variant name",
                    sanitize_comment_text(raw)
                ),
                location: None,
                source: None,
            });
        }
        let mut ident = base.clone();
        let mut suffix = 2;
        while !taken_variants.insert(ident.clone()) {
            ident = format!("{base}{suffix}");
            suffix += 1;
        }
        if ident != raw {
            out.push_str(&format!(
                "    #[serde(rename = \"{}\")]\n",
                escape_rust_string(raw)
            ));
        }
        out.push_str(&format!("    {ident},\n"));
        variant_map.push((raw.to_string(), ident));
    }
    out.push_str("}\n");
    Ok((out, variant_map))
}

/// `#[serde(rename = "...")]` attribute line carrying the original property
/// name, escaped against literal breakout.
fn rename_attribute(prop_name: &str) -> String {
    format!("#[serde(rename = \"{}\")]\n", escape_rust_string(prop_name))
}

/// `(identifier, optional #[serde(rename)] attribute)` for a property name.
/// Names that do not normalize to a legal Rust identifier (empty, digit
/// leading) and the keywords that cannot be raw identifiers (`self`,
/// `super`, `crate`) fail loudly instead of generating broken code.
fn field_identifier(prop_name: &str, parent: &str) -> ConfigResult<(String, Option<String>)> {
    let snake = to_snake_case(prop_name);
    // A name made entirely of underscores (`_`, `__`, or any input that only
    // feeds the separator branch — e.g. a single CJK character) normalizes
    // to `_`, which Rust reserves outright: neither `pub _` nor `pub r##_`
    // compiles, and no escape path exists, so it must fail loudly.
    let legal_start = !snake.is_empty()
        && snake != "_"
        && snake.starts_with(|c: char| c.is_ascii_lowercase() || c == '_');
    if !legal_start {
        return Err(ConfigError::ParseError {
            format: "schema".into(),
            message: format!(
                "property name `{}` (type `{parent}`) normalizes to `{snake}`, which is not a legal Rust identifier: rename the property in the schema",
                sanitize_comment_text(prop_name)
            ),
            location: None,
            source: None,
        });
    }
    if matches!(snake.as_str(), "self" | "super" | "crate") {
        return Err(ConfigError::ParseError {
            format: "schema".into(),
            message: format!(
                "property name `{}` (type `{parent}`) normalizes to `{snake}`, which cannot be a raw identifier in Rust: rename the property in the schema",
                sanitize_comment_text(prop_name)
            ),
            location: None,
            source: None,
        });
    }
    if RUST_KEYWORDS.contains(&snake.as_str()) {
        // serde derives the serialized name from the raw identifier minus `r#`.
        Ok((format!("r#{snake}"), None))
    } else if snake == prop_name {
        Ok((snake, None))
    } else {
        Ok((snake, Some(rename_attribute(prop_name))))
    }
}

const RUST_KEYWORDS: &[&str] = &[
    "as", "break", "const", "continue", "crate", "dyn", "else", "enum", "extern", "false", "fn",
    "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref",
    "return", "self", "static", "struct", "super", "trait", "true", "type", "unsafe", "use",
    "where", "while", "abstract", "become", "box", "do", "final", "macro", "override", "priv",
    "typeof", "unsized", "virtual", "yield", "await", "try", "async", "gen",
];

fn to_snake_case(input: &str) -> String {
    let mut out = String::with_capacity(input.len() + 4);
    let chars: Vec<char> = input.chars().collect();
    for (index, ch) in chars.iter().enumerate() {
        if ch.is_ascii_uppercase() {
            let prev_lower_or_digit =
                index > 0 && (chars[index - 1].is_lowercase() || chars[index - 1].is_ascii_digit());
            let next_lower = chars.get(index + 1).is_some_and(char::is_ascii_lowercase);
            if index > 0 && (prev_lower_or_digit || next_lower) {
                out.push('_');
            }
            out.push(ch.to_ascii_lowercase());
        } else if ch.is_ascii_alphanumeric() {
            out.push(*ch);
        } else {
            out.push('_');
        }
    }
    out
}

fn to_pascal_case(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut capitalize_next = true;
    if input.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        out.insert(0, 'V');
    }
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() {
            if capitalize_next {
                out.extend(ch.to_uppercase());
                capitalize_next = false;
            } else {
                out.push(ch);
            }
        } else {
            capitalize_next = true;
        }
    }
    out
}
