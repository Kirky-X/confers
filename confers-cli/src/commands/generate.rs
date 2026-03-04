// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

use crate::error::ConfigError;
use serde::Serialize;
use std::fs;

#[cfg(feature = "schema")]
use schemars::{schema_for, JsonSchema};

/// 模板生成级别 - 控制生成模板的详细程度
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenerateLevel {
    /// 仅包含必要字段的最小模板
    Minimal,
    /// 包含所有字段和详细注释的完整模板
    Full,
    /// 带解释的文档样式模板
    Documentation,
}

impl GenerateLevel {
    /// 将级别字符串解析为 GenerateLevel 枚举
    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "minimal" | "min" => GenerateLevel::Minimal,
            "doc" | "documentation" => GenerateLevel::Documentation,
            "full" => GenerateLevel::Full,
            _ => GenerateLevel::Full,
        }
    }
}

impl std::str::FromStr for GenerateLevel {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::parse(s))
    }
}

pub struct GenerateCommand;

impl GenerateCommand {
    /// 为特定 Config 类型生成模板
    #[cfg(feature = "schema")]
    pub fn execute<T>(output: Option<&String>, level: &str) -> Result<(), ConfigError>
    where
        T: Serialize + Default + JsonSchema,
    {
        let defaults = T::default();
        let generate_level = GenerateLevel::parse(level);
        let content = match generate_level {
            GenerateLevel::Minimal => toml::to_string(&defaults)
                .map_err(|e| ConfigError::FormatDetectionFailed(e.to_string()))?,
            GenerateLevel::Documentation => generate_documentation_template::<T>(),
            GenerateLevel::Full => toml::to_string_pretty(&defaults)
                .map_err(|e| ConfigError::FormatDetectionFailed(e.to_string()))?,
        };
        Self::write_content(output, &content)
    }

    #[cfg(not(feature = "schema"))]
    pub fn execute<T>(output: Option<&String>, level: &str) -> Result<(), ConfigError>
    where
        T: Serialize + Default,
    {
        let defaults = T::default();
        let generate_level = GenerateLevel::parse(level);
        let content = match generate_level {
            GenerateLevel::Minimal => toml::to_string(&defaults)
                .map_err(|e| ConfigError::FormatDetectionFailed(e.to_string()))?,
            GenerateLevel::Documentation => generate_documentation_template::<T>(),
            GenerateLevel::Full => toml::to_string_pretty(&defaults)
                .map_err(|e| ConfigError::FormatDetectionFailed(e.to_string()))?,
        };
        Self::write_content(output, &content)
    }

    /// 生成通用占位符（用于独立 CLI）
    pub fn execute_placeholder(
        output: Option<&String>,
        level: &str,
        struct_name: Option<&String>,
        format: &str,
    ) -> Result<String, ConfigError> {
        let generate_level = GenerateLevel::parse(level);
        let mut toml_content = match generate_level {
            GenerateLevel::Minimal => Self::minimal_template(),
            GenerateLevel::Documentation => Self::documentation_template(),
            GenerateLevel::Full => Self::full_template(),
        };

        // If struct name is provided, customize the template
        if let Some(name) = struct_name {
            toml_content =
                toml_content.replace("name = \"example\"", &format!("name = \"{}\"", name));
            toml_content = toml_content.replace(
                "# Minimal Config Template",
                &format!("# {} Config Template", name),
            );
            toml_content = toml_content.replace(
                "# Full Config Template",
                &format!("# {} Config Template", name),
            );
            toml_content = toml_content.replace(
                "Configuration Template",
                &format!("{} Configuration Template", name),
            );
        }

        // Convert format
        let content = if format.eq_ignore_ascii_case("toml") {
            toml_content
        } else {
            let value: toml::Value = toml::from_str(&toml_content)
                .map_err(|e| ConfigError::ParseError(format!("Failed to parse template: {}", e)))?;

            match format.to_lowercase().as_str() {
                "json" => serde_json::to_string_pretty(&value)
                    .map_err(|e| ConfigError::SerializationError(e.to_string()))?,
                "yaml" | "yml" => serde_yaml::to_string(&value)
                    .map_err(|e| ConfigError::SerializationError(e.to_string()))?,
                "ini" => {
                    // Check if the value is flat enough for INI
                    // If it's too nested, serde_ini might fail or produce unexpected results.
                    // But for our templates, they are generally section-based (Map<String, Map<String, Value>>)
                    serde_ini::to_string(&value).map_err(|e| {
                        ConfigError::SerializationError(format!(
                            "INI serialization failed (structure might be too deep): {}",
                            e
                        ))
                    })?
                }
                _ => {
                    return Err(ConfigError::FormatDetectionFailed(format!(
                        "Unsupported format: {}",
                        format
                    )))
                }
            }
        };

        Self::write_content(output, &content)?;
        Ok(content)
    }

    /// 最小模板 - 仅包含必要字段
    fn minimal_template() -> String {
        "# Minimal Config Template\n# Generated by confers\n\n[app]\nname = \"example\"\nversion = \"1.0.0\"\n".to_string()
    }

    /// 完整模板 - 包含所有字段和结构
    fn full_template() -> String {
        "# Full Config Template\n# Generated by confers\n\n# Basic configuration\n[app]\nname = \"example\"\nversion = \"1.0.0\"\n\n# Server settings\n[server]\nhost = \"localhost\"\nport = 8080\n\n# Database configuration\n[database]\nurl = \"postgres://localhost/mydb\"\npool_size = 10\n\n# Logging configuration\n[logging]\nlevel = \"info\"\nformat = \"json\"\n".to_string()
    }

    /// 文档模板 - 包含详细解释
    #[cfg(feature = "schema")]
    fn documentation_template() -> String {
        #[derive(Default, Serialize, JsonSchema)]
        struct GenericPlaceholder {
            name: String,
            version: String,
            description: String,
        }
        generate_documentation_template::<GenericPlaceholder>()
    }

    #[cfg(not(feature = "schema"))]
    fn documentation_template() -> String {
        generate_documentation_template::<()>()
    }

    fn write_content(output: Option<&String>, content: &str) -> Result<(), ConfigError> {
        if let Some(path) = output {
            fs::write(path, content)
                .map_err(|e| ConfigError::FormatDetectionFailed(e.to_string()))?;
        } else {
            println!("{}", content);
        }
        Ok(())
    }
}

/// 生成带详细注释的文档样式模板
#[cfg(feature = "schema")]
fn generate_documentation_template<T: Default + JsonSchema + serde::Serialize>() -> String {
    let defaults = T::default();
    let defaults_str = toml::to_string_pretty(&defaults)
        .map_err(|_| ConfigError::FormatDetectionFailed("Failed to serialize defaults".to_string()))
        .unwrap_or_default();

    let schema = schema_for!(T);
    let schema_value = serde_json::to_value(&schema)
        .map_err(|_| ConfigError::FormatDetectionFailed("Failed to serialize schema".to_string()))
        .unwrap_or_default();

    let struct_name = schema_value
        .get("title")
        .and_then(|t| t.as_str())
        .unwrap_or("Config");

    let mut doc = format!(
        r#"# Documentation Configuration Template for {}
# Generated by confers - Configuration Management Tool
# ============================================================
# This template includes detailed comments to help you understand
# each configuration option and its purpose.
# -----------------------------------------------------------

"#,
        struct_name
    );

    if let Some(properties) = schema_value.get("properties").and_then(|p| p.as_object()) {
        generate_property_documentation(properties, "", &mut doc);
    }

    doc.push_str(&format!(
        "\n# ============================================================\n# Default Values Reference\n# -----------------------------------------------------------\n{}\n# -----------------------------------------------------------\n",
        defaults_str
    ));

    doc
}

#[cfg(not(feature = "schema"))]
fn generate_documentation_template<T: Default + serde::Serialize>() -> String {
    let defaults = T::default();
    let defaults_str = toml::to_string_pretty(&defaults)
        .map_err(|_| ConfigError::FormatDetectionFailed("Failed to serialize defaults".to_string()))
        .unwrap_or_default();

    let mut doc = r#"# Configuration Template (Documentation Mode)
# Generated by confers - Configuration Management Tool
# ============================================================
# This template includes detailed comments to help you understand
# each configuration option and its purpose.

"#
    .to_string();

    doc.push_str(&format!(
        "# Default Values\n# -----------------------------------------------------------\n{}\n# -----------------------------------------------------------\n",
        defaults_str
    ));

    doc
}

#[cfg(feature = "schema")]
fn generate_property_documentation(
    properties: &serde_json::Map<String, serde_json::Value>,
    prefix: &str,
    doc: &mut String,
) {
    for (prop_name, prop_schema) in properties {
        let full_name = if prefix.is_empty() {
            prop_name.clone()
        } else {
            format!("{}.{}", prefix, prop_name)
        };

        if let Some(obj_props) = prop_schema.get("properties").and_then(|p| p.as_object()) {
            doc.push_str(&format!("# [{}]\n", full_name));
            if let Some(desc) = prop_schema.get("description").and_then(|d| d.as_str()) {
                doc.push_str(&format!("# {}\n", desc));
            }
            doc.push_str("#\n");
            generate_property_documentation(obj_props, &full_name, doc);
            doc.push('\n');
        } else {
            let type_str = prop_schema
                .get("type")
                .and_then(|t| t.as_str())
                .unwrap_or("unknown");

            if let Some(desc) = prop_schema.get("description").and_then(|d| d.as_str()) {
                doc.push_str(&format!("# {}\n", desc));
            }

            let default_val = prop_schema
                .get("default")
                .map(|v| format!(" (default: {})", v));

            let enum_vals = prop_schema
                .get("enum")
                .and_then(|e| e.as_array())
                .map(|vals| {
                    let vals_str: Vec<String> = vals
                        .iter()
                        .filter_map(|v| v.as_str().map(|s| format!("'{}'", s)))
                        .collect();
                    format!(" (options: {})", vals_str.join(", "))
                });

            let example = match type_str {
                "string" | "integer" | "number" | "boolean" => {
                    format!("{} = \"{}\"", prop_name, "<value>")
                }
                _ => format!("{} = <{}>", prop_name, type_str),
            };

            if let Some(dv) = default_val {
                doc.push_str(&format!("{}{}\n", example, dv));
            } else if let Some(ev) = enum_vals {
                doc.push_str(&format!("{}{}\n", example, ev));
            } else {
                doc.push_str(&format!("{}\n", example));
            }
            doc.push('\n');
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "schema")]
    mod with_schema {
        use super::*;
        use schemars::JsonSchema;

        #[derive(Debug, Default, Serialize, JsonSchema)]
        struct TestConfig {
            name: String,
            port: u16,
            #[schemars(skip)]
            secret: String,
        }

        #[test]
        fn test_documentation_with_schema() {
            let result = GenerateCommand::execute::<TestConfig>(None, "documentation");
            assert!(result.is_ok());

            let result = GenerateCommand::execute::<TestConfig>(None, "doc");
            assert!(result.is_ok());
        }
    }

    #[test]
    fn test_generate_level_parsing() {
        assert_eq!(GenerateLevel::parse("minimal"), GenerateLevel::Minimal);
        assert_eq!(GenerateLevel::parse("min"), GenerateLevel::Minimal);
        assert_eq!(GenerateLevel::parse("doc"), GenerateLevel::Documentation);
        assert_eq!(
            GenerateLevel::parse("documentation"),
            GenerateLevel::Documentation
        );
        assert_eq!(GenerateLevel::parse("full"), GenerateLevel::Full);
        assert_eq!(GenerateLevel::parse("unknown"), GenerateLevel::Full);
        assert_eq!(GenerateLevel::parse("FULL"), GenerateLevel::Full);
        assert_eq!(GenerateLevel::parse("DOC"), GenerateLevel::Documentation);
    }

    #[test]
    fn test_generate_command_minimal() {
        let result = GenerateCommand::execute_placeholder(None, "minimal", None, "toml");
        assert!(result.is_ok());
        let content = result.unwrap();
        assert!(content.contains("name = \"example\""));
        assert!(content.contains("# Minimal Config Template"));
    }

    #[test]
    fn test_generate_command_full() {
        let result = GenerateCommand::execute_placeholder(None, "full", None, "toml");
        assert!(result.is_ok());
        let content = result.unwrap();
        assert!(content.contains("[server]"));
        assert!(content.contains("[database]"));
        assert!(content.contains("# Full Config Template"));
    }

    #[test]
    fn test_generate_command_documentation_fallback() {
        let result = GenerateCommand::execute_placeholder(None, "doc", None, "toml");
        assert!(result.is_ok());
        let content = result.unwrap();
        assert!(content.contains("Configuration Template"));
        assert!(content.contains("Generated by confers"));
    }

    #[test]
    fn test_generate_level_affects_output() {
        let minimal = GenerateCommand::execute_placeholder(None, "minimal", None, "toml").unwrap();
        let full = GenerateCommand::execute_placeholder(None, "full", None, "toml").unwrap();
        let doc = GenerateCommand::execute_placeholder(None, "doc", None, "toml").unwrap();

        assert_ne!(minimal, full);
        assert_ne!(full, doc);
        assert_ne!(minimal, doc);

        assert!(minimal.contains("Minimal"));
        assert!(full.contains("Full"));
        assert!(doc.contains("Documentation"));
    }
}
