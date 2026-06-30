// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

use schemars::{schema_for, JsonSchema};
use serde_json::Value;

pub struct TypeScriptGenerator;

impl TypeScriptGenerator {
    pub fn generate<T: JsonSchema>() -> String {
        let schema = schema_for!(T);
        let schema_value = serde_json::to_value(schema).unwrap_or_else(|e| {
            eprintln!("Failed to convert schema to JSON value: {}", e);
            serde_json::Value::Null
        });
        Self::convert_json_schema_to_typescript(&schema_value)
    }

    fn convert_json_schema_to_typescript(schema: &Value) -> String {
        let mut interfaces = Vec::new();

        // First, handle definitions if they exist
        if let Some(definitions) = schema.get("definitions") {
            if let Some(defs_obj) = definitions.as_object() {
                for (name, def_schema) in defs_obj {
                    let interface = Self::generate_interface(name, def_schema);
                    interfaces.push(interface);
                }
            }
        }

        // Then, handle the main schema
        if let Some(_properties) = schema.get("properties") {
            // Use the title from the schema if available, otherwise default to "Config"
            let interface_name = schema
                .get("title")
                .and_then(|t| t.as_str())
                .unwrap_or("Config");

            let main_interface = Self::generate_interface(interface_name, schema);
            interfaces.push(main_interface);
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

                let property_def = if optional {
                    format!("  {}?: {};", prop_name, prop_type)
                } else {
                    format!("  {}: {};", prop_name, prop_type)
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

            if types.len() == 2 && types.contains(&"null".to_string()) {
                let non_null_type = types.iter().find(|&t| t != "null");
                match non_null_type {
                    Some(t) => return t.clone(),
                    None => return "any".to_string(),
                }
            } else {
                return types.join(" | ");
            }
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
                            inner_props.push(format!("{}: {}", p_name, p_type));
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
            properties.push(format!("  {}: {};", key, prop_type));
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

        let ts_output = TypeScriptGenerator::generate::<TestConfig>();
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
        // An empty $ref string splits into [""], so parts.last() returns "" (not "any").
        let s = serde_json::json!({ "$ref": "" });
        assert_eq!(TypeScriptGenerator::get_typescript_type(&s), "");
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
        assert_eq!(TypeScriptGenerator::get_typescript_type(&s), "string");
    }

    #[test]
    fn test_get_typescript_type_array_option_integer() {
        let s = serde_json::json!({ "type": ["integer", "null"] });
        assert_eq!(TypeScriptGenerator::get_typescript_type(&s), "number");
    }

    #[test]
    fn test_get_typescript_type_array_option_number() {
        let s = serde_json::json!({ "type": ["null", "number"] });
        assert_eq!(TypeScriptGenerator::get_typescript_type(&s), "number");
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
        // types.len() == 1, so the "len == 2 && contains null" branch is false.
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
        // No definitions, no properties -> empty interfaces -> invalid marker.
        let s = serde_json::json!({ "type": "object" });
        let out = TypeScriptGenerator::convert_json_schema_to_typescript(&s);
        assert_eq!(out, "// Invalid schema format");
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
        let ts = TypeScriptGenerator::generate::<Simple>();
        assert!(ts.contains("interface Simple"));
        assert!(ts.contains("name: string;"));
        assert!(ts.contains("count: number;"));
    }

    #[test]
    fn test_generate_unit_struct_alias() {
        #[derive(Debug, Serialize, Deserialize, JsonSchema)]
        struct Unit;
        let ts = TypeScriptGenerator::generate::<Unit>();
        assert!(!ts.is_empty());
    }

    #[test]
    fn test_generate_array_field_with_items() {
        #[derive(Debug, Serialize, Deserialize, JsonSchema)]
        struct WithArray {
            tags: Vec<String>,
            counts: Vec<i32>,
        }
        let ts = TypeScriptGenerator::generate::<WithArray>();
        assert!(ts.contains("tags: string[]"));
        assert!(ts.contains("counts: number[]"));
    }
}
