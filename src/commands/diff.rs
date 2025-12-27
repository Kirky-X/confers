// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

use crate::error::ConfigError;
use serde_json::Value;
use std::fs;
use std::path::Path;

const GREEN: &str = "\x1b[32m";
const RED: &str = "\x1b[31m";
const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";

pub struct DiffCommand;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DiffFormat {
    Unified,
    Context,
    Normal,
    SideBySide,
    Strict,
}

impl std::str::FromStr for DiffFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "unified" => Ok(DiffFormat::Unified),
            "context" => Ok(DiffFormat::Context),
            "normal" => Ok(DiffFormat::Normal),
            "side-by-side" | "sidebyside" | "side" => Ok(DiffFormat::SideBySide),
            "strict" => Ok(DiffFormat::Strict),
            _ => Err(format!("Unknown diff format: {}. Supported formats: unified, context, normal, side-by-side, strict", s)),
        }
    }
}

#[derive(Debug)]
pub struct DiffOptions {
    pub format: DiffFormat,
    pub context_lines: usize,
    pub show_line_numbers: bool,
    pub ignore_whitespace: bool,
    pub case_insensitive: bool,
    pub strict: bool,
}

impl Clone for DiffOptions {
    fn clone(&self) -> Self {
        DiffOptions {
            format: self.format,
            context_lines: self.context_lines,
            show_line_numbers: self.show_line_numbers,
            ignore_whitespace: self.ignore_whitespace,
            case_insensitive: self.case_insensitive,
            strict: self.strict,
        }
    }
}

impl Default for DiffOptions {
    fn default() -> Self {
        DiffOptions {
            format: DiffFormat::Unified,
            context_lines: 3,
            show_line_numbers: false,
            ignore_whitespace: false,
            case_insensitive: false,
            strict: false,
        }
    }
}

impl DiffCommand {
    pub fn execute(file1: &str, file2: &str, options: DiffOptions) -> Result<(), ConfigError> {
        let v1 = Self::load_config(file1)?;
        let v2 = Self::load_config(file2)?;

        if v1 == v2 {
            println!("Configurations are identical.");
            return Ok(());
        }

        match options.format {
            DiffFormat::Unified => Self::print_unified_diff(file1, file2, &v1, &v2, options),
            DiffFormat::Context => Self::print_context_diff(file1, file2, &v1, &v2, options),
            DiffFormat::Normal => Self::print_normal_diff(file1, file2, &v1, &v2, options),
            DiffFormat::SideBySide => {
                Self::print_side_by_side_diff(file1, file2, &v1, &v2, options)
            }
            DiffFormat::Strict => Self::print_strict_diff(file1, file2, &v1, &v2, options),
        }

        Ok(())
    }

    fn colorize(s: &str, color: &str, options: &DiffOptions) -> String {
        if options.strict {
            s.to_string()
        } else {
            format!("{}{}{}", color, s, RESET)
        }
    }

    fn print_normal_diff(file1: &str, file2: &str, v1: &Value, v2: &Value, options: DiffOptions) {
        println!("{} {} {}", BOLD, file1, RESET);
        println!("{} {} {}", BOLD, file2, RESET);
        let diffs = Self::generate_normal_diff(v1, v2, "", &options);
        for line in diffs {
            println!("{}", line);
        }
    }

    fn print_strict_diff(file1: &str, file2: &str, v1: &Value, v2: &Value, options: DiffOptions) {
        let strict_opts = DiffOptions {
            strict: true,
            format: DiffFormat::Strict,
            ..options
        };

        println!("--- {}", file1);
        println!("+++ {}", file2);
        let diffs = Self::generate_standard_unified_diff(v1, v2, "", &strict_opts);
        for line in diffs {
            println!("{}", line);
        }
    }

    fn print_unified_diff(file1: &str, file2: &str, v1: &Value, v2: &Value, options: DiffOptions) {
        let header = format!("--- {}", file1);
        let header2 = format!("+++ {}", file2);

        println!("{}{}{}", BOLD, header, RESET);
        println!("{}{}{}", BOLD, header2, RESET);

        let diffs = Self::generate_unified_diff(v1, v2, "", &options);
        for line in diffs {
            println!("{}", line);
        }
    }

    fn print_context_diff(file1: &str, file2: &str, v1: &Value, v2: &Value, options: DiffOptions) {
        println!("*** {}", file1);
        println!("--- {}", file2);

        let diffs = Self::generate_context_diff(v1, v2, "", &options);
        for line in diffs {
            println!("{}", line);
        }
    }

    fn print_side_by_side_diff(
        file1: &str,
        file2: &str,
        v1: &Value,
        v2: &Value,
        options: DiffOptions,
    ) {
        let sep = if options.show_line_numbers {
            " | "
        } else {
            "   "
        };
        let left_header = format!("{}Left: {}{}", BOLD, file1, RESET);
        let right_header = format!("{}Right: {}{}", BOLD, file2, RESET);

        println!("{}{}{}{}{}", left_header, sep, right_header, sep, RESET);
        println!("{}", "-".repeat(80));

        let diffs = Self::generate_side_by_side_diff(v1, v2, "", &options);
        for line in diffs {
            println!("{}", line);
        }
    }

    fn generate_unified_diff(
        v1: &Value,
        v2: &Value,
        path: &str,
        options: &DiffOptions,
    ) -> Vec<String> {
        let mut diffs = Vec::new();
        let _prefix = if path.is_empty() { "" } else { path };

        match (v1, v2) {
            (Value::Object(m1), Value::Object(m2)) => {
                let all_keys: std::collections::HashSet<_> = m1.keys().chain(m2.keys()).collect();
                let mut sorted_keys: Vec<_> = all_keys.iter().collect();
                sorted_keys.sort();

                for k in sorted_keys {
                    let new_path = if path.is_empty() {
                        k.as_str().to_string()
                    } else {
                        format!("{}.{}", path, k)
                    };

                    match (m1.get(k.as_str()), m2.get(k.as_str())) {
                        (None, Some(v2_val)) => {
                            diffs.push(Self::colorize(&format!("+{}", k), GREEN, options));
                            let indented = Self::indent_value(v2_val, 1);
                            for line in indented {
                                diffs.push(Self::colorize(&format!("+{}", line), GREEN, options));
                            }
                        }
                        (Some(v1_val), None) => {
                            diffs.push(Self::colorize(&format!("-{}", k), RED, options));
                            let indented = Self::indent_value(v1_val, 1);
                            for line in indented {
                                diffs.push(Self::colorize(&format!("-{}", line), RED, options));
                            }
                        }
                        (Some(v1_val), Some(v2_val)) if v1_val != v2_val => {
                            let sub_diffs =
                                Self::generate_unified_diff(v1_val, v2_val, &new_path, options);
                            if sub_diffs.is_empty() {
                                diffs.push(Self::colorize(&format!("-{}", new_path), RED, options));
                                diffs.push(Self::colorize(
                                    &format!("+{}", new_path),
                                    GREEN,
                                    options,
                                ));
                            } else {
                                diffs.extend(sub_diffs);
                            }
                        }
                        (Some(_v1_val), Some(_v2_val)) => {
                            diffs.push(format!("  {}", new_path));
                        }
                        _ => {}
                    }
                }
            }
            (Value::Array(a1), Value::Array(a2)) => {
                if a1 != a2 {
                    let max_len = std::cmp::max(a1.len(), a2.len());
                    for i in 0..max_len {
                        let item_path = format!("{}[{}]", path, i);
                        match (a1.get(i), a2.get(i)) {
                            (None, Some(v2_val)) => {
                                diffs.push(Self::colorize(
                                    &format!("+{}", item_path),
                                    GREEN,
                                    options,
                                ));
                                let indented = Self::indent_value(v2_val, 1);
                                for line in indented {
                                    diffs.push(Self::colorize(
                                        &format!("+{}", line),
                                        GREEN,
                                        options,
                                    ));
                                }
                            }
                            (Some(v1_val), None) => {
                                diffs.push(Self::colorize(
                                    &format!("-{}", item_path),
                                    RED,
                                    options,
                                ));
                                let indented = Self::indent_value(v1_val, 1);
                                for line in indented {
                                    diffs.push(Self::colorize(&format!("-{}", line), RED, options));
                                }
                            }
                            (Some(v1_val), Some(v2_val)) if v1_val != v2_val => {
                                let sub_diffs = Self::generate_unified_diff(
                                    v1_val, v2_val, &item_path, options,
                                );
                                if sub_diffs.is_empty() {
                                    diffs.push(Self::colorize(
                                        &format!("-{}", item_path),
                                        RED,
                                        options,
                                    ));
                                    diffs.push(Self::colorize(
                                        &format!("+{}", item_path),
                                        GREEN,
                                        options,
                                    ));
                                } else {
                                    diffs.extend(sub_diffs);
                                }
                            }
                            (Some(_v1_val), Some(_v2_val)) => {
                                diffs.push(format!("  {}", item_path));
                            }
                            _ => {}
                        }
                    }
                }
            }
            _ if v1 != v2 => {
                diffs.push(Self::colorize(&format!("-{}", v1), RED, options));
                diffs.push(Self::colorize(&format!("+{}", v2), GREEN, options));
            }
            _ => {}
        }

        diffs
    }

    fn generate_context_diff(
        v1: &Value,
        v2: &Value,
        path: &str,
        _options: &DiffOptions,
    ) -> Vec<String> {
        let mut diffs = Vec::new();
        let _prefix = if path.is_empty() { "" } else { path };

        match (v1, v2) {
            (Value::Object(m1), Value::Object(m2)) => {
                for k in m1
                    .keys()
                    .chain(m2.keys())
                    .collect::<std::collections::HashSet<_>>()
                {
                    let new_path = if path.is_empty() {
                        k.clone()
                    } else {
                        format!("{}.{}", path, k)
                    };

                    match (m1.get(k), m2.get(k)) {
                        (None, Some(v2_val)) => {
                            diffs.push(format!("{}{}*{}*{}", GREEN, BOLD, k, RESET));
                            let indented = Self::indent_value(v2_val, 1);
                            diffs.extend(
                                indented
                                    .iter()
                                    .map(|s| format!("+{}", s))
                                    .collect::<Vec<_>>(),
                            );
                        }
                        (Some(v1_val), None) => {
                            diffs.push(format!("{}{}*{}*{}", RED, BOLD, k, RESET));
                            let indented = Self::indent_value(v1_val, 1);
                            diffs.extend(
                                indented
                                    .iter()
                                    .map(|s| format!("-{}", s))
                                    .collect::<Vec<_>>(),
                            );
                        }
                        (Some(v1_val), Some(v2_val)) if v1_val != v2_val => {
                            let sub_diffs =
                                Self::generate_context_diff(v1_val, v2_val, &new_path, _options);
                            if sub_diffs.is_empty() {
                                diffs.push(format!("{}{}{}- ", RED, new_path, RESET));
                                diffs.push(format!("{}{}{}+ ", GREEN, new_path, RESET));
                            } else {
                                diffs.extend(sub_diffs);
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ if v1 != v2 => {
                diffs.push(format!("{}{}*{}*{}", RED, BOLD, path, RESET));
            }
            _ => {}
        }

        diffs
    }

    fn generate_side_by_side_diff(
        v1: &Value,
        v2: &Value,
        path: &str,
        options: &DiffOptions,
    ) -> Vec<String> {
        let width = 35;
        let separator = if options.show_line_numbers {
            " | "
        } else {
            "   "
        };
        let mut diffs = Vec::new();

        match (v1, v2) {
            (Value::Object(m1), Value::Object(m2)) => {
                for k in m1
                    .keys()
                    .chain(m2.keys())
                    .collect::<std::collections::HashSet<_>>()
                {
                    let new_path = if path.is_empty() {
                        k.clone()
                    } else {
                        format!("{}.{}", path, k)
                    };

                    match (m1.get(k), m2.get(k)) {
                        (None, Some(v2_val)) => {
                            let left = format!("{}{}{}", RED, " ".repeat(width), RESET);
                            let right = Self::truncate(&format!("{}", v2_val), width);
                            let right = format!("{}{}{}", GREEN, right, RESET);
                            diffs.push(format!("{} {} {}", left, separator, right));
                        }
                        (Some(v1_val), None) => {
                            let left = Self::truncate(&format!("{}", v1_val), width);
                            let left = format!("{}{}{}", RED, left, RESET);
                            let right = format!("{}{}{}", GREEN, " ".repeat(width), RESET);
                            diffs.push(format!("{} {} {}", left, separator, right));
                        }
                        (Some(v1_val), Some(v2_val)) if v1_val != v2_val => {
                            let sub_diffs = Self::generate_side_by_side_diff(
                                v1_val, v2_val, &new_path, options,
                            );
                            diffs.extend(sub_diffs);
                        }
                        (Some(v1_val), Some(v2_val)) => {
                            let left = Self::truncate(&format!("{}", v1_val), width);
                            let right = Self::truncate(&format!("{}", v2_val), width);
                            diffs.push(format!("{} {} {}", left, separator, right));
                        }
                        _ => {}
                    }
                }
            }
            _ if v1 != v2 => {
                let left = Self::truncate(&format!("{}", v1), width);
                let left = format!("{}{}{}", RED, left, RESET);
                let right = Self::truncate(&format!("{}", v2), width);
                let right = format!("{}{}{}", GREEN, right, RESET);
                diffs.push(format!("{} {} {}", left, separator, right));
            }
            _ => {
                let left = Self::truncate(&format!("{}", v1), width);
                let right = Self::truncate(&format!("{}", v2), width);
                diffs.push(format!("{} {} {}", left, separator, right));
            }
        }

        diffs
    }

    fn indent_value(v: &Value, indent: usize) -> Vec<String> {
        let s = format!("{}", v);
        let indent_str = "  ".repeat(indent);
        s.lines()
            .map(|line| format!("{}{}", indent_str, line))
            .collect()
    }

    fn truncate(s: &str, width: usize) -> String {
        if s.len() <= width {
            s.to_string()
        } else {
            s.chars().take(width.saturating_sub(3)).collect::<String>() + "..."
        }
    }

    fn load_config(file: &str) -> Result<Value, ConfigError> {
        let path = Path::new(file);
        if !path.exists() {
            return Err(ConfigError::FileNotFound {
                path: path.to_path_buf(),
            });
        }

        let content = fs::read_to_string(path)
            .map_err(|e| ConfigError::IoError(format!("Failed to read config file: {}", e)))?;

        let ext = path.extension().and_then(|e| e.to_str());

        match ext {
            Some("json") | Some("jsonc") => {
                serde_json::from_str(&content).map_err(|e| ConfigError::ParseError(e.to_string()))
            }
            Some("yaml") | Some("yml") => {
                serde_yaml::from_str(&content).map_err(|e| ConfigError::ParseError(e.to_string()))
            }
            Some("toml") => {
                toml::from_str(&content).map_err(|e| ConfigError::ParseError(e.to_string()))
            }
            _ => Err(ConfigError::ParseError(format!(
                "Unsupported config format: {:?}",
                ext
            ))),
        }
    }

    fn generate_normal_diff(
        v1: &Value,
        v2: &Value,
        path: &str,
        _options: &DiffOptions,
    ) -> Vec<String> {
        let mut diffs = Vec::new();

        match (v1, v2) {
            (Value::Object(m1), Value::Object(m2)) => {
                for k in m1
                    .keys()
                    .chain(m2.keys())
                    .collect::<std::collections::HashSet<_>>()
                {
                    let new_path = if path.is_empty() {
                        k.clone()
                    } else {
                        format!("{}.{}", path, k)
                    };

                    match (m1.get(k), m2.get(k)) {
                        (None, Some(v2_val)) => {
                            diffs.push(format!("{}: {}", new_path, v2_val));
                        }
                        (Some(v1_val), None) => {
                            diffs.push(format!("{}: {}", new_path, v1_val));
                        }
                        (Some(v1_val), Some(v2_val)) if v1_val != v2_val => {
                            diffs.extend(Self::generate_normal_diff(
                                v1_val, v2_val, &new_path, _options,
                            ));
                        }
                        _ => {}
                    }
                }
            }
            (Value::Array(a1), Value::Array(a2)) => {
                if a1 != a2 {
                    let max_len = std::cmp::max(a1.len(), a2.len());
                    for i in 0..max_len {
                        let item_path = format!("{}[{}]", path, i);
                        match (a1.get(i), a2.get(i)) {
                            (None, Some(v2_val)) => {
                                diffs.push(format!("{}: {}", item_path, v2_val));
                            }
                            (Some(v1_val), None) => {
                                diffs.push(format!("{}: {}", item_path, v1_val));
                            }
                            (Some(v1_val), Some(v2_val)) if v1_val != v2_val => {
                                diffs.extend(Self::generate_normal_diff(
                                    v1_val, v2_val, &item_path, _options,
                                ));
                            }
                            _ => {}
                        }
                    }
                }
            }
            _ if v1 != v2 => {
                diffs.push(format!("{}: {} => {}", path, v1, v2));
            }
            _ => {}
        }

        diffs
    }

    fn generate_standard_unified_diff(
        v1: &Value,
        v2: &Value,
        path: &str,
        _options: &DiffOptions,
    ) -> Vec<String> {
        let mut diffs = Vec::new();

        match (v1, v2) {
            (Value::Object(m1), Value::Object(m2)) => {
                let all_keys: std::collections::HashSet<_> = m1.keys().chain(m2.keys()).collect();
                let mut sorted_keys: Vec<_> = all_keys.iter().collect();
                sorted_keys.sort();

                for k in sorted_keys {
                    let new_path = if path.is_empty() {
                        k.as_str().to_string()
                    } else {
                        format!("{}.{}", path, k)
                    };

                    match (m1.get(k.as_str()), m2.get(k.as_str())) {
                        (None, Some(v2_val)) => {
                            diffs.push(format!("+[{}]", new_path));
                            let indented = Self::indent_value(v2_val, 1);
                            for line in indented {
                                diffs.push(format!("+{}", line));
                            }
                        }
                        (Some(v1_val), None) => {
                            diffs.push(format!("-[{}]", new_path));
                            let indented = Self::indent_value(v1_val, 1);
                            for line in indented {
                                diffs.push(format!("-{}", line));
                            }
                        }
                        (Some(_v1_val), Some(_v2_val)) if _v1_val != _v2_val => {
                            diffs.extend(Self::generate_standard_unified_diff(
                                _v1_val, _v2_val, &new_path, _options,
                            ));
                        }
                        (Some(_v1_val), Some(_v2_val)) => {
                            diffs.push(format!(" [{}]", new_path));
                        }
                        _ => {}
                    }
                }
            }
            (Value::Array(a1), Value::Array(a2)) => {
                if a1 != a2 {
                    let max_len = std::cmp::max(a1.len(), a2.len());
                    for i in 0..max_len {
                        let item_path = format!("{}[{}]", path, i);
                        match (a1.get(i), a2.get(i)) {
                            (None, Some(v2_val)) => {
                                diffs.push(format!("+{}", item_path));
                                let indented = Self::indent_value(v2_val, 1);
                                for line in indented {
                                    diffs.push(format!("+{}", line));
                                }
                            }
                            (Some(v1_val), None) => {
                                diffs.push(format!("-{}", item_path));
                                let indented = Self::indent_value(v1_val, 1);
                                for line in indented {
                                    diffs.push(format!("-{}", line));
                                }
                            }
                            (Some(v1_val), Some(v2_val)) if v1_val != v2_val => {
                                diffs.extend(Self::generate_standard_unified_diff(
                                    v1_val, v2_val, &item_path, _options,
                                ));
                            }
                            _ => {}
                        }
                    }
                }
            }
            _ if v1 != v2 => {
                diffs.push(format!("-{}", v1));
                diffs.push(format!("+{}", v2));
            }
            _ => {}
        }

        diffs
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[allow(dead_code)]
    fn create_test_config() -> (Value, Value) {
        let config1 = json!({
            "name": "test_app",
            "version": "1.0.0",
            "server": {
                "host": "localhost",
                "port": 8080
            },
            "database": {
                "url": "postgres://localhost/db",
                "pool_size": 5
            }
        });

        let config2 = json!({
            "name": "test_app",
            "version": "1.1.0",
            "server": {
                "host": "0.0.0.0",
                "port": 8080
            },
            "database": {
                "url": "postgres://localhost/db",
                "pool_size": 10
            },
            "new_feature": {
                "enabled": true
            }
        });

        (config1, config2)
    }

    #[test]
    fn test_execute_with_identical_configs() {
        let config_content = r#"{"name": "test", "value": "same"}"#;

        let temp_dir = std::env::temp_dir();
        let file1_path = temp_dir.join("test_config_1.json");
        let file2_path = temp_dir.join("test_config_2.json");

        std::fs::write(&file1_path, config_content).unwrap();
        std::fs::write(&file2_path, config_content).unwrap();

        let result = DiffCommand::execute(
            file1_path.to_str().unwrap(),
            file2_path.to_str().unwrap(),
            DiffOptions::default(),
        );

        let _ = std::fs::remove_file(&file1_path);
        let _ = std::fs::remove_file(&file2_path);

        assert!(result.is_ok());
    }

    #[test]
    fn test_diff_format_parsing() {
        assert_eq!(
            "unified".parse::<DiffFormat>().unwrap(),
            DiffFormat::Unified
        );
        assert_eq!(
            "side-by-side".parse::<DiffFormat>().unwrap(),
            DiffFormat::SideBySide
        );
        assert_eq!("strict".parse::<DiffFormat>().unwrap(), DiffFormat::Strict);
        assert!("invalid".parse::<DiffFormat>().is_err());
    }

    #[test]
    fn test_diff_options_default() {
        let options = DiffOptions::default();

        assert_eq!(options.format, DiffFormat::Unified);
        assert_eq!(options.context_lines, 3);
        assert!(!options.show_line_numbers);
        assert!(!options.ignore_whitespace);
        assert!(!options.case_insensitive);
        assert!(!options.strict);
    }

    #[test]
    fn test_colorize_strict_mode() {
        let options = DiffOptions {
            strict: true,
            ..Default::default()
        };

        let result = DiffCommand::colorize("test", GREEN, &options);
        assert_eq!(result, "test");
    }

    #[test]
    fn test_colorize_non_strict_mode() {
        let options = DiffOptions {
            strict: false,
            ..Default::default()
        };

        let result = DiffCommand::colorize("test", GREEN, &options);
        assert!(result.contains("test"));
        assert!(result.contains(GREEN));
        assert!(result.contains(RESET));
    }
}
