// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! JSON Schema → Rust 反向生成示例（schema --from-schema）
//!
//! 本示例展示 confers 的反向脚手架能力（与 json_schema 示例的
//! Rust→Schema 正向方向互补）：
//! - 给定 JSON Schema 草稿（2020-12），生成可编译的 Rust struct 起点代码
//! - 映射规则一览：非 required → `Option<T>` + `#[serde(default)]`；
//!   标量 default → 生成的 `_default_*` 函数；字符串枚举 → Rust 枚举；
//!   内联嵌套对象 → 提升为具名 struct；`$defs`/`$ref` → 独立类型
//! - 不支持的结构（oneOf/anyOf/allOf、外部 $ref 等）显性报错而非生成错误代码
//!
//! 对应 CLI 用法：
//!   confers schema --from-schema config-schema.json
//!
//! 运行方式：
//!   cargo run --bin schema_to_rust

use confers::schema::RustScaffoldGenerator;
use serde_json::json;

fn main() {
    println!("========================================");
    println!("  JSON Schema → Rust 脚手架示例");
    println!("========================================\n");

    demo_basic_scaffolding();
    demo_defs_and_refs();
    demo_fail_loud();

    println!("\n========================================");
    println!("  示例运行完成");
    println!("========================================");
}

/// 基础映射：标量、Option、默认值、枚举、数组。
fn demo_basic_scaffolding() {
    let schema = json!({
        "title": "ServiceConfig",
        "type": "object",
        "properties": {
            "name": {"type": "string", "description": "服务名"},
            "port": {"type": "integer", "default": 8080},
            "status": {"type": "string", "enum": ["active", "paused"]},
            "nickname": {"type": "string"},
            "replicas": {"type": "array", "items": {"type": "integer"}},
            "lastDeployed": {"type": "string", "format": "date-time"}
        },
        "required": ["name", "status"]
    });

    let code = RustScaffoldGenerator::generate(&schema).expect("schema is supported");
    println!("--- 基础映射 ---\n{code}");
}

/// $defs + 内部 $ref：共享子结构成为独立类型。
fn demo_defs_and_refs() {
    let schema = json!({
        "title": "StackConfig",
        "type": "object",
        "properties": {
            "primary": {"$ref": "#/$defs/Database"},
            "replica": {"$ref": "#/$defs/Database"}
        },
        "required": ["primary"],
        "$defs": {
            "Database": {
                "type": "object",
                "properties": {
                    "url": {"type": "string"},
                    "maxPool": {"type": "integer", "default": 10}
                },
                "required": ["url"]
            }
        }
    });

    let code = RustScaffoldGenerator::generate(&schema).expect("schema is supported");
    println!("--- $defs 与内部 $ref ---\n{code}");
}

/// 不支持的结构显性报错：脚手架宁缺毋滥，绝不静默生成错误代码。
fn demo_fail_loud() {
    let schema = json!({
        "type": "object",
        "properties": {
            "union": {"oneOf": [{"type": "string"}, {"type": "integer"}]}
        },
        "required": ["union"]
    });

    println!("--- 不支持结构的显性报错 ---");
    match RustScaffoldGenerator::generate(&schema) {
        Ok(_) => println!("（不应到达）"),
        Err(e) => println!("按预期失败：{e}"),
    }
}
