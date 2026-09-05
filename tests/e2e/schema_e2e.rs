// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! E2E: Schema(tests/e2e/schema_e2e.rs)
//!
//! 场景固化(docs/TEST_SCENARIOS.md §2.23):
//! - SCH-04 嵌套 struct/Option/Vec 字段的 schema 输出正确性
//!   (含 SCH-02 的 TypeScriptGenerator 生成面)
//!
//! SCH-01/02 已有 derive 级覆盖(tests/core/derive.rs);
//! SCH-03(typescript-schema 别名)由本目标的 required-features = ["typescript-schema"]
//! 直接证明:别名开启即编译出全部 schema 能力并运行通过。
//! SCH-05(schema 与 CLI validate 联动)固化于 cli_e2e.rs。
//!
//! 已知实现边界(按真实行为固化,见报告):
//! - `ConfigSchema` 派生的 `json_schema()` 不递归嵌套 struct(映射为 "string"),
//!   `typescript_type()` 输出字段占位接口;
//! - 完整嵌套结构经 `schemars::JsonSchema` + `TypeScriptGenerator::generate::<T>()` 表达。

use confers::{ConfigSchema, schema::TypeScriptGenerator};
use schemars::JsonSchema;

/// 嵌套结构:父 struct 持有嵌套 struct、Option、Vec 字段。
#[derive(Debug, ConfigSchema, JsonSchema)]
#[allow(dead_code)] // 字段仅经 schema 生成面使用。
struct AppSchema {
    #[config(name = "name")]
    name: String,

    #[config(name = "replicas")]
    replicas: u16,

    #[config(name = "tags")]
    tags: Vec<String>,

    #[config(name = "database")]
    database: DatabaseSchema,

    #[config(name = "retry")]
    retry: Option<RetrySchema>,
}

#[derive(Debug, JsonSchema)]
#[allow(dead_code)]
struct DatabaseSchema {
    host: String,
    port: u16,
}

#[derive(Debug, JsonSchema)]
#[allow(dead_code)]
struct RetrySchema {
    attempts: u32,
}

#[test]
fn sch04_json_schema_covers_all_fields_with_primitive_types() {
    let schema = AppSchema::json_schema();
    let obj = schema.as_object().expect("schema must be an object");
    assert_eq!(obj.get("type").and_then(|v| v.as_str()), Some("object"));
    assert_eq!(obj.get("title").and_then(|v| v.as_str()), Some("AppSchema"));

    let properties = obj
        .get("properties")
        .and_then(|v| v.as_object())
        .expect("properties must exist");

    for field in ["name", "replicas", "tags", "database", "retry"] {
        assert!(
            properties.contains_key(field),
            "field `{field}` must appear in schema properties, got: {properties:?}"
        );
    }

    assert_eq!(
        properties["name"].get("type").and_then(|v| v.as_str()),
        Some("string")
    );
    let replicas = &properties["replicas"];
    assert_eq!(
        replicas.get("type").and_then(|v| v.as_str()),
        Some("integer")
    );
    assert_eq!(replicas.get("minimum").and_then(|v| v.as_u64()), Some(0));
    assert_eq!(
        properties["tags"].get("type").and_then(|v| v.as_str()),
        Some("array"),
        "Vec<String> must map to array type"
    );
}

/// 行为固化:`ConfigSchema` 派生对嵌套 struct 的当前映射
/// (struct → "string",Option<struct> → ["string", "null"])。
/// 该边界已在验收报告中记录;完整嵌套语义走 schemars + TypeScriptGenerator。
#[test]
fn sch04_config_schema_derive_nested_struct_current_mapping() {
    let schema = AppSchema::json_schema();
    let properties = schema
        .get("properties")
        .and_then(|v| v.as_object())
        .expect("properties must exist");

    assert_eq!(
        properties["database"].get("type").and_then(|v| v.as_str()),
        Some("string"),
        "nested struct maps to string (fixated current behavior)"
    );
    assert_eq!(
        properties["retry"].get("type").and_then(|v| v
            .as_array()
            .map(|a| a.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>())),
        Some(vec!["string", "null"]),
        "Option<struct> maps to [string, null] (fixated current behavior)"
    );
}

#[test]
fn sch04_typescript_generator_maps_nested_structure() {
    let ts = TypeScriptGenerator::generate::<AppSchema>().expect("TS generation must succeed");

    assert!(
        ts.contains("interface AppSchema"),
        "TS output must name the root interface, got: {ts}"
    );
    assert!(
        ts.contains("database: DatabaseSchema"),
        "nested struct field must map to its type: {ts}"
    );
    assert!(
        ts.contains("retry?:"),
        "Option field must render optional: {ts}"
    );
    assert!(
        ts.contains("tags: string[]"),
        "Vec<String> must render string[]: {ts}"
    );
    assert!(
        ts.contains("replicas: number"),
        "integer field must render number: {ts}"
    );
}

#[test]
fn sch04_typescript_type_static_output_is_an_interface() {
    let ts = AppSchema::typescript_type();
    assert!(
        ts.contains("export interface AppSchema"),
        "typescript_type must emit an exported interface declaration, got: {ts}"
    );
}
