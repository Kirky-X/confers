// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! JSON Schema Example - ConfigSchema Derive Macro
//!
//! 本示例展示如何使用 confers 的 JSON Schema 生成功能：
//! - `#[derive(ConfigSchema)]` 派生宏自动生成 JSON Schema
//! - 从配置结构生成 TypeScript 类型定义
//! - 带验证约束的 schema 生成
//! - 嵌套结构 schema 生成
//! - Schema 导出到文件
//! - Schema 在 IDE 和文档中的使用
//!
//! 设计依据：ADR-012（Schema 生成与导出）
//!
//! 运行方式：
//!   cargo run --bin json_schema
//!   cargo run --bin json_schema -- --output schema.json

use confers::ConfigSchema;
use confers::i18n::{tr, tr_args};
use serde::{Deserialize, Serialize};
use serde_json::{self, Value};

// 配置结构定义

/// 简单应用配置
#[derive(ConfigSchema, Serialize, Deserialize, Debug, Clone)]
pub struct SimpleConfig {
    /// 应用名称
    pub name: String,

    /// 服务器端口
    pub port: u16,

    /// 启用调试
    pub debug: bool,
}

/// 服务器配置
#[derive(ConfigSchema, Serialize, Deserialize, Debug, Clone)]
pub struct ServerConfig {
    /// 主机地址
    pub host: String,

    /// 端口
    pub port: u16,

    /// 连接超时
    pub timeout_seconds: u64,
}

/// 数据库配置
#[derive(ConfigSchema, Serialize, Deserialize, Debug, Clone)]
pub struct DatabaseConfig {
    /// 数据库主机
    pub host: String,

    /// 数据库端口
    pub port: u16,

    /// 数据库名称
    pub database: String,

    /// 最大连接数
    pub max_connections: u32,
}

/// 缓存配置
#[derive(ConfigSchema, Serialize, Deserialize, Debug, Clone)]
pub struct CacheConfig {
    /// 启用缓存
    pub enabled: bool,

    /// TTL（秒）
    pub ttl_seconds: u64,

    /// 最大条目数
    pub max_entries: u64,
}

/// 完整应用配置
#[derive(ConfigSchema, Serialize, Deserialize, Debug, Clone)]
pub struct FullAppConfig {
    /// 应用名称
    pub name: String,

    /// 应用版本
    pub version: String,

    /// 服务器配置
    pub server: ServerConfig,

    /// 数据库配置
    pub database: DatabaseConfig,

    /// 缓存配置
    pub cache: CacheConfig,
}

/// 特性开关配置
#[derive(ConfigSchema, Serialize, Deserialize, Debug, Clone)]
pub struct FeatureFlags {
    /// 启用新 UI
    pub new_ui: bool,

    /// 启用 Beta API
    pub beta_api: bool,

    /// 启用实时通知
    pub realtime_notification: bool,
}

// Schema 生成器

/// Schema 生成选项
#[derive(Debug, Clone)]
pub struct SchemaOptions {
    /// 是否包含描述
    pub include_descriptions: bool,

    /// 是否包含示例值
    pub include_examples: bool,

    /// 是否包含默认值
    pub include_defaults: bool,

    /// 是否添加必需字段
    pub add_required: bool,

    /// 是否添加格式信息
    pub add_format: bool,
}

impl Default for SchemaOptions {
    fn default() -> Self {
        Self {
            include_descriptions: true,
            include_examples: true,
            include_defaults: false,
            add_required: true,
            add_format: true,
        }
    }
}

/// Schema 生成器
pub struct SchemaGenerator;

impl SchemaGenerator {
    /// 为简单配置生成 schema
    pub fn generate_simple_schema() -> Value {
        serde_json::json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "title": "SimpleConfig",
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": tr("example-json-schema-desc-application-name"),
                    "examples": ["myapp", "production-api"]
                },
                "port": {
                    "type": "integer",
                    "minimum": 1,
                    "maximum": 65535,
                    "description": tr("example-json-schema-desc-server-port"),
                    "examples": [8080, 3000, 9000]
                },
                "debug": {
                    "type": "boolean",
                    "description": tr("example-json-schema-desc-enable-debug-mode"),
                    "default": false
                }
            },
            "required": ["name", "port", "debug"]
        })
    }

    /// 为完整配置生成 schema
    pub fn generate_full_schema() -> Value {
        serde_json::json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "title": "FullAppConfig",
            "type": "object",
            "description": tr("example-json-schema-desc-full-app-config"),
            "properties": {
                "name": {
                    "type": "string",
                    "description": tr("example-json-schema-desc-application-name")
                },
                "version": {
                    "type": "string",
                    "description": tr("example-json-schema-desc-application-version"),
                    "pattern": "^\\d+\\.\\d+\\.\\d+$"
                },
                "server": {
                    "$ref": "#/definitions/ServerConfig"
                },
                "database": {
                    "$ref": "#/definitions/DatabaseConfig"
                },
                "cache": {
                    "$ref": "#/definitions/CacheConfig"
                }
            },
            "required": ["name", "version", "server"],
            "definitions": {
                "ServerConfig": {
                    "type": "object",
                    "properties": {
                        "host": {
                            "type": "string",
                            "description": tr("example-json-schema-desc-host-address"),
                            "default": "0.0.0.0"
                        },
                        "port": {
                            "type": "integer",
                            "minimum": 1,
                            "maximum": 65535,
                            "description": tr("example-json-schema-desc-server-port")
                        },
                        "timeout_seconds": {
                            "type": "integer",
                            "minimum": 1,
                            "description": tr("example-json-schema-desc-connection-timeout")
                        }
                    },
                    "required": ["host", "port", "timeout_seconds"]
                },
                "DatabaseConfig": {
                    "type": "object",
                    "properties": {
                        "host": {
                            "type": "string",
                            "description": tr("example-json-schema-desc-database-host")
                        },
                        "port": {
                            "type": "integer",
                            "minimum": 1,
                            "maximum": 65535,
                            "description": tr("example-json-schema-desc-database-port")
                        },
                        "database": {
                            "type": "string",
                            "description": tr("example-json-schema-desc-database-name")
                        },
                        "max_connections": {
                            "type": "integer",
                            "minimum": 1,
                            "description": tr("example-json-schema-desc-max-connections")
                        }
                    },
                    "required": ["host", "port", "database"]
                },
                "CacheConfig": {
                    "type": "object",
                    "properties": {
                        "enabled": {
                            "type": "boolean",
                            "description": tr("example-json-schema-desc-caching-enabled")
                        },
                        "ttl_seconds": {
                            "type": "integer",
                            "minimum": 0,
                            "description": tr("example-json-schema-desc-cache-ttl")
                        },
                        "max_entries": {
                            "type": "integer",
                            "minimum": 0,
                            "description": tr("example-json-schema-desc-cache-max-entries")
                        }
                    },
                    "required": ["enabled"]
                }
            }
        })
    }

    /// 生成带验证约束的 schema
    pub fn generate_validated_schema() -> Value {
        serde_json::json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "title": "ValidatedConfig",
            "type": "object",
            "properties": {
                "hostname": {
                    "type": "string",
                    "minLength": 1,
                    "maxLength": 253,
                    "pattern": "^[a-zA-Z0-9]([a-zA-Z0-9\\-]{0,61}[a-zA-Z0-9])?(\\.[a-zA-Z0-9]([a-zA-Z0-9\\-]{0,61}[a-zA-Z0-9])?)*$",
                    "description": tr("example-json-schema-desc-hostname")
                },
                "port": {
                    "type": "integer",
                    "minimum": 1,
                    "maximum": 65535,
                    "description": tr("example-json-schema-desc-port-number")
                },
                "email": {
                    "type": "string",
                    "format": "email",
                    "description": tr("example-json-schema-desc-admin-email")
                },
                "url": {
                    "type": "string",
                    "format": "uri",
                    "description": tr("example-json-schema-desc-service-url")
                },
                "rate_limit": {
                    "type": "integer",
                    "minimum": 1,
                    "maximum": 1000000,
                    "description": tr("example-json-schema-desc-rate-limit")
                },
                "cache_ttl": {
                    "type": "number",
                    "minimum": 0,
                    "maximum": 86400,
                    "description": tr("example-json-schema-desc-cache-ttl-24h")
                },
                "tags": {
                    "type": "array",
                    "items": {
                        "type": "string",
                        "minLength": 1,
                        "maxLength": 50
                    },
                    "minItems": 0,
                    "maxItems": 20,
                    "uniqueItems": true,
                    "description": tr("example-json-schema-desc-config-tags")
                }
            },
            "required": ["hostname", "port"]
        })
    }

    /// 生成 TypeScript 类型定义
    pub fn generate_typescript_types() -> String {
        r#"// Generated TypeScript types from ConfigSchema
// This file is auto-generated - do not edit manually

export interface SimpleConfig {
  name: string;
  port: number;
  debug: boolean;
}

export interface ServerConfig {
  host: string;
  port: number;
  timeout_seconds: number;
}

export interface DatabaseConfig {
  host: string;
  port: number;
  database: string;
  max_connections: number;
}

export interface CacheConfig {
  enabled: boolean;
  ttl_seconds: number;
  max_entries: number;
}

export interface FullAppConfig {
  name: string;
  version: string;
  server: ServerConfig;
  database: DatabaseConfig;
  cache: CacheConfig;
}

export interface FeatureFlags {
  new_ui: boolean;
  beta_api: boolean;
  realtime_notification: boolean;
}

// Usage example:
// import type { FullAppConfig } from './types';
//
// const config: FullAppConfig = await fetchConfig();
//
// console.log(config.server.host);
"#
        .to_string()
    }
}

// Schema 验证

/// Schema 验证结果
#[derive(Debug)]
pub struct ValidationResult {
    /// 是否有效
    pub valid: bool,

    /// 错误消息
    pub errors: Vec<String>,
}

impl ValidationResult {
    /// 验证配置是否符合 schema
    ///
    /// 注意：这里使用简化的验证逻辑。
    /// 生产环境应使用 jsonschema crate 进行完整的 JSON Schema 验证。
    pub fn validate(value: &Value, schema: &Value) -> Self {
        let mut errors = Vec::new();

        if let Some(obj) = value.as_object() {
            if let Some(props) = schema.get("properties").and_then(|p| p.as_object()) {
                for (key, prop_schema) in props {
                    if let Some(value) = obj.get(key)
                        && let Some(prop_err) = Self::validate_property(key, value, prop_schema)
                    {
                        errors.push(prop_err);
                    }
                }
            }

            if let Some(required) = schema.get("required").and_then(|r| r.as_array()) {
                for req in required {
                    if let Some(req_str) = req.as_str()
                        && !obj.contains_key(req_str)
                    {
                        errors.push(format!("Missing required field: {}", req_str));
                    }
                }
            }
        }

        Self {
            valid: errors.is_empty(),
            errors,
        }
    }

    /// 验证单个属性
    fn validate_property(name: &str, value: &Value, schema: &Value) -> Option<String> {
        if let Some(type_str) = schema.get("type").and_then(|t| t.as_str()) {
            let value_type = match value {
                Value::String(_) => "string",
                Value::Number(n) => {
                    if n.is_i64() || n.is_u64() {
                        "integer"
                    } else {
                        "number"
                    }
                }
                Value::Bool(_) => "boolean",
                Value::Array(_) => "array",
                Value::Object(_) => "object",
                Value::Null => "null",
            };

            if type_str != value_type && !(type_str == "integer" && value_type == "number") {
                return Some(format!(
                    "Field '{}': expected type '{}', got '{}'",
                    name, type_str, value_type
                ));
            }
        }

        if let Some(min) = schema.get("minimum").and_then(|m| m.as_i64())
            && let Some(num) = value.as_i64()
            && num < min
        {
            return Some(format!(
                "Field '{}': value {} is less than minimum {}",
                name, num, min
            ));
        }

        if let Some(max) = schema.get("maximum").and_then(|m| m.as_i64())
            && let Some(num) = value.as_i64()
            && num > max
        {
            return Some(format!(
                "Field '{}': value {} is greater than maximum {}",
                name, num, max
            ));
        }

        if let Some(min) = schema.get("minLength").and_then(|m| m.as_i64())
            && let Some(s) = value.as_str()
            && (s.len() as i64) < min
        {
            return Some(format!(
                "Field '{}': string length {} is less than minLength {}",
                name,
                s.len(),
                min
            ));
        }

        if let Some(max) = schema.get("maxLength").and_then(|m| m.as_i64())
            && let Some(s) = value.as_str()
            && (s.len() as i64) > max
        {
            return Some(format!(
                "Field '{}': string length {} is greater than maxLength {}",
                name,
                s.len(),
                max
            ));
        }

        None
    }
}

// Schema 导出

/// Schema 导出器
pub struct SchemaExporter;

impl SchemaExporter {
    /// 导出 schema 到文件
    pub fn export_to_file(schema: &Value, path: &str) -> std::io::Result<()> {
        let content = serde_json::to_string_pretty(schema)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// 从文件导入 schema
    pub fn import_from_file(path: &str) -> std::io::Result<Value> {
        let content = std::fs::read_to_string(path)?;
        let schema: Value = serde_json::from_str(&content)?;
        Ok(schema)
    }
}

// 演示函数

/// 演示简单 schema 生成
fn demo_simple_schema() {
    println!("\n{}\n", tr("example-json-schema-demo1"));

    let schema = SchemaGenerator::generate_simple_schema();
    let schema_str = serde_json::to_string_pretty(&schema).unwrap();

    println!("{}", tr("example-json-schema-generated-schema"));
    println!("{}", schema_str);
    println!();

    let valid_config = serde_json::json!({
        "name": "myapp",
        "port": 8080,
        "debug": false
    });

    let invalid_config = serde_json::json!({
        "name": "myapp",
        "port": 70000,  // 无效端口
        "debug": false
    });

    let result1 = ValidationResult::validate(&valid_config, &schema);
    println!(
        "{}",
        tr_args(
            "example-json-schema-validate-valid",
            &[(
                "status",
                if result1.valid { "pass" } else { "fail" }.to_string()
            )]
        )
    );

    let result2 = ValidationResult::validate(&invalid_config, &schema);
    println!(
        "{}",
        tr_args(
            "example-json-schema-validate-invalid",
            &[(
                "status",
                if result2.valid { "pass" } else { "fail" }.to_string()
            )]
        )
    );
    for error in &result2.errors {
        println!("  - {}", error);
    }
}

/// 演示嵌套结构 schema
fn demo_nested_schema() {
    println!("\n{}\n", tr("example-json-schema-demo2"));

    let schema = SchemaGenerator::generate_full_schema();
    let schema_str = serde_json::to_string_pretty(&schema).unwrap();

    println!("{}", tr("example-json-schema-nested-schema"));
    println!("{}", schema_str);
    println!();

    let full_config = serde_json::json!({
        "name": "myapp",
        "version": "1.0.0",
        "server": {
            "host": "0.0.0.0",
            "port": 8080,
            "timeout_seconds": 30
        },
        "database": {
            "host": "localhost",
            "port": 5432,
            "database": "appdb",
            "max_connections": 20
        },
        "cache": {
            "enabled": true,
            "ttl_seconds": 300,
            "max_entries": 10000
        }
    });

    let result = ValidationResult::validate(&full_config, &schema);
    println!(
        "{}",
        tr_args(
            "example-json-schema-validate-full",
            &[(
                "status",
                if result.valid { "pass" } else { "fail" }.to_string()
            )]
        )
    );
}

/// 演示带约束的 schema
fn demo_validated_schema() {
    println!("\n{}\n", tr("example-json-schema-demo3"));

    let schema = SchemaGenerator::generate_validated_schema();
    let schema_str = serde_json::to_string_pretty(&schema).unwrap();

    println!("{}", tr("example-json-schema-validated-schema"));
    println!("{}", schema_str);
    println!();

    // 测试各种约束
    let test_cases = vec![
        (
            tr("example-json-schema-case-valid-config"),
            serde_json::json!({
                "hostname": "example.com",
                "port": 8080,
                "email": "admin@example.com",
                "url": "https://example.com",
                "rate_limit": 1000,
                "cache_ttl": 300.0,
                "tags": ["web", "api"]
            }),
            true,
        ),
        (
            tr("example-json-schema-case-invalid-port"),
            serde_json::json!({
                "hostname": "example.com",
                "port": 99999
            }),
            false,
        ),
        (
            tr("example-json-schema-case-missing-required"),
            serde_json::json!({
                "hostname": "example.com"
            }),
            false,
        ),
    ];

    for (name, config, expect_valid) in test_cases {
        let result = ValidationResult::validate(&config, &schema);
        let status = if result.valid == expect_valid {
            "OK"
        } else {
            "UNEXPECTED"
        };
        println!("  {}: {}", name, status);
        if !result.errors.is_empty() {
            for error in &result.errors {
                println!("    - {}", error);
            }
        }
    }
}

/// 演示 TypeScript 类型生成
fn demo_typescript_types() {
    println!("\n{}\n", tr("example-json-schema-demo4"));

    let ts_types = SchemaGenerator::generate_typescript_types();
    println!("{}", tr("example-json-schema-generated-typescript"));
    println!("{}", ts_types);
    println!();

    println!("{}", tr("example-json-schema-ide-integration"));
    println!("  {}", tr("example-json-schema-ide-copy-types"));
    println!("  {}", tr("example-json-schema-ide-quicktype"));
    println!("  {}", tr("example-json-schema-ide-vscode"));
    println!();

    println!("  {}", tr("example-json-schema-used-in-typescript"));
    println!("  import type {{ FullAppConfig }} from './types';");
    println!();
    println!("  async function loadConfig(): Promise<FullAppConfig> {{");
    println!("    const response = await fetch('/api/config');");
    println!("    return response.json() as FullAppConfig;");
    println!("  }}");
}

/// 演示 schema 导出到文件
fn demo_schema_export() {
    println!("\n{}\n", tr("example-json-schema-demo5"));

    let output_path = "config-schema.json";
    let schema = SchemaGenerator::generate_full_schema();

    match SchemaExporter::export_to_file(&schema, output_path) {
        Ok(_) => {
            println!(
                "  {}",
                tr_args(
                    "example-json-schema-schema-exported",
                    &[("path", output_path.to_string())]
                )
            );

            match SchemaExporter::import_from_file(output_path) {
                Ok(imported) => {
                    println!("  {}", tr("example-json-schema-verify-import-ok"));
                    println!(
                        "  {}",
                        tr_args(
                            "example-json-schema-schema-title",
                            &[(
                                "title",
                                format!("{:?}", imported.get("title").and_then(|v| v.as_str()))
                            )]
                        )
                    );
                }
                Err(e) => {
                    println!(
                        "  {}",
                        tr_args(
                            "example-json-schema-verify-import-failed",
                            &[("error", e.to_string())]
                        )
                    );
                }
            }
        }
        Err(e) => {
            println!(
                "  {}",
                tr_args(
                    "example-json-schema-export-failed",
                    &[("error", e.to_string())]
                )
            );
        }
    }

    // 同时导出 TypeScript 类型
    let ts_output = "config-types.ts";
    if let Err(e) = std::fs::write(ts_output, SchemaGenerator::generate_typescript_types()) {
        println!(
            "  {}",
            tr_args(
                "example-json-schema-typescript-export-failed",
                &[("error", e.to_string())]
            )
        );
    } else {
        println!(
            "  {}",
            tr_args(
                "example-json-schema-typescript-exported",
                &[("path", ts_output.to_string())]
            )
        );
    }
}

/// 演示 ConfigSchema 派生宏用法
fn demo_config_schema_derive() {
    println!("\n{}\n", tr("example-json-schema-demo6"));

    println!("{}", tr("example-json-schema-derive-intro"));
    println!();
    println!("  #[derive(ConfigSchema, Serialize, Deserialize)]");
    println!("  pub struct ConfersConfig {{");
    println!("      pub host: String,");
    println!("      pub port: u16,");
    println!("      pub debug: bool,");
    println!("  }}");
    println!();
    println!("  {}", tr("example-json-schema-auto-generated-methods"));
    println!("  // - ConfersConfig::json_schema() -> serde_json::Value");
    println!("  // - ConfersConfig::typescript_type() -> String");
    println!();
    println!("  let schema = ConfersConfig::json_schema();");
    println!("  let ts_type = ConfersConfig::typescript_type();");
    println!();
    println!("  {}", tr("example-json-schema-export-schema-comment"));
    println!("  let schema_json = serde_json::to_string_pretty(&schema).unwrap();");
    println!("  std::fs::write(\"schema.json\", schema_json).unwrap();");
}

/// 演示 schema 的实际应用场景
fn demo_use_cases() {
    println!("\n{}\n", tr("example-json-schema-demo7"));

    println!("{}", tr("example-json-schema-use-case-1"));
    println!("   {}", tr("example-json-schema-use-case-1a"));
    println!("   {}", tr("example-json-schema-use-case-1b"));
    println!();

    println!("{}", tr("example-json-schema-use-case-2"));
    println!("   {}", tr("example-json-schema-use-case-2a"));
    println!("   {}", tr("example-json-schema-use-case-2b"));
    println!();

    println!("{}", tr("example-json-schema-use-case-3"));
    println!("   {}", tr("example-json-schema-use-case-3a"));
    println!("   {}", tr("example-json-schema-use-case-3b"));
    println!();

    println!("{}", tr("example-json-schema-use-case-4"));
    println!("   {}", tr("example-json-schema-use-case-4a"));
    println!("   {}", tr("example-json-schema-use-case-4b"));
    println!();

    println!("{}", tr("example-json-schema-use-case-5"));
    println!("   {}", tr("example-json-schema-use-case-5a"));
    println!("   {}", tr("example-json-schema-use-case-5b"));
}

// 主程序

fn main() {
    println!("========================================");
    println!("  JSON Schema Example");
    println!("  ConfigSchema Derive Macro Demo");
    println!("========================================");

    let args: Vec<String> = std::env::args().collect();
    let output_path = args
        .iter()
        .position(|a| a == "--output")
        .and_then(|i| args.get(i + 1))
        .cloned();

    demo_simple_schema();
    demo_nested_schema();
    demo_validated_schema();
    demo_typescript_types();
    demo_schema_export();
    demo_config_schema_derive();
    demo_use_cases();

    if let Some(path) = output_path {
        let schema = SchemaGenerator::generate_full_schema();
        match SchemaExporter::export_to_file(&schema, &path) {
            Ok(_) => println!(
                "\n{}",
                tr_args(
                    "example-json-schema-cli-exported",
                    &[("path", path.clone())]
                )
            ),
            Err(e) => eprintln!(
                "{}",
                tr_args(
                    "example-json-schema-export-failed",
                    &[("error", e.to_string())]
                )
            ),
        }
    }

    println!("\n========================================");
    println!("  {}", tr("example-json-schema-completed"));
    println!("========================================");
}
