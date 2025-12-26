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
                            let left = format!("{}{}{}", RED, "".pad_to_width(width), RESET);
                            let right = Self::truncate(&format!("{}", v2_val), width);
                            let right = format!("{}{}{}", GREEN, right, RESET);
                            diffs.push(format!("{} {} {}", left, separator, right));
                        }
                        (Some(v1_val), None) => {
                            let left = Self::truncate(&format!("{}", v1_val), width);
                            let left = format!("{}{}{}", RED, left, RESET);
                            let right = format!("{}{}{}", GREEN, "".pad_to_width(width), RESET);
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
            let mut truncated = s.chars().take(width - 3).collect::<String>();
            truncated.push_str("...");
            truncated
        }
    }

    fn generate_normal_diff(
        v1: &Value,
        v2: &Value,
        path: &str,
        options: &DiffOptions,
    ) -> Vec<String> {
        let mut diffs = Vec::new();

        match (v1, v2) {
            (Value::Object(m1), Value::Object(m2)) => {
                let prefix = if path.is_empty() {
                    "".to_string()
                } else {
                    format!("{}.", path)
                };

                for (k, v) in m1 {
                    if !m2.contains_key(k) {
                        diffs.push(Self::colorize(&format!("{}< {}", prefix, k), RED, options));
                        diffs.extend(
                            Self::indent_value(v, 1)
                                .into_iter()
                                .map(|l| Self::colorize(&format!("< {}", l), RED, options))
                                .collect::<Vec<_>>(),
                        );
                    }
                }
                for (k, v) in m2 {
                    if !m1.contains_key(k) {
                        diffs.push(Self::colorize(
                            &format!("{}> {}", prefix, k),
                            GREEN,
                            options,
                        ));
                        diffs.extend(
                            Self::indent_value(v, 1)
                                .into_iter()
                                .map(|l| Self::colorize(&format!("> {}", l), GREEN, options))
                                .collect::<Vec<_>>(),
                        );
                    }
                }
                for (k, v1_val) in m1 {
                    if let Some(v2_val) = m2.get(k) {
                        if v1_val != v2_val {
                            let new_path = if path.is_empty() {
                                k.clone()
                            } else {
                                format!("{}.{}", path, k)
                            };
                            let sub_diffs =
                                Self::generate_normal_diff(v1_val, v2_val, &new_path, options);
                            if sub_diffs.is_empty() {
                                diffs.push(Self::colorize(
                                    &format!("{}< {}", prefix, new_path),
                                    RED,
                                    options,
                                ));
                                diffs.extend(
                                    Self::indent_value(v1_val, 1)
                                        .into_iter()
                                        .map(|l| Self::colorize(&format!("< {}", l), RED, options))
                                        .collect::<Vec<_>>(),
                                );
                                diffs.push(Self::colorize(
                                    &format!("{}> {}", prefix, new_path),
                                    GREEN,
                                    options,
                                ));
                                diffs.extend(
                                    Self::indent_value(v2_val, 1)
                                        .into_iter()
                                        .map(|l| {
                                            Self::colorize(&format!("> {}", l), GREEN, options)
                                        })
                                        .collect::<Vec<_>>(),
                                );
                            } else {
                                diffs.extend(sub_diffs);
                            }
                        }
                    }
                }
            }
            (Value::Array(a1), Value::Array(a2)) => {
                if a1 != a2 {
                    let prefix = if path.is_empty() { "" } else { path };
                    let mut i = 0;
                    let min_len = std::cmp::min(a1.len(), a2.len());

                    while i < min_len {
                        if a1[i] != a2[i] {
                            let item_path = format!("{}[{}]", prefix, i);
                            let sub_diffs =
                                Self::generate_normal_diff(&a1[i], &a2[i], &item_path, options);
                            if sub_diffs.is_empty() {
                                diffs.push(Self::colorize(
                                    &format!("{}< {}", prefix, item_path),
                                    RED,
                                    options,
                                ));
                                diffs.extend(
                                    Self::indent_value(&a1[i], 1)
                                        .into_iter()
                                        .map(|l| Self::colorize(&format!("< {}", l), RED, options))
                                        .collect::<Vec<_>>(),
                                );
                                diffs.push(Self::colorize(
                                    &format!("{}> {}", prefix, item_path),
                                    GREEN,
                                    options,
                                ));
                                diffs.extend(
                                    Self::indent_value(&a2[i], 1)
                                        .into_iter()
                                        .map(|l| {
                                            Self::colorize(&format!("> {}", l), GREEN, options)
                                        })
                                        .collect::<Vec<_>>(),
                                );
                            } else {
                                diffs.extend(sub_diffs);
                            }
                        }
                        i += 1;
                    }

                    while i < a2.len() {
                        let item_path = format!("{}[{}]", prefix, i);
                        diffs.push(Self::colorize(
                            &format!("{}> {}", prefix, item_path),
                            GREEN,
                            options,
                        ));
                        diffs.extend(
                            Self::indent_value(&a2[i], 1)
                                .into_iter()
                                .map(|l| Self::colorize(&format!("> {}", l), GREEN, options))
                                .collect::<Vec<_>>(),
                        );
                        i += 1;
                    }

                    while i < a1.len() {
                        let item_path = format!("{}[{}]", prefix, i);
                        diffs.push(Self::colorize(
                            &format!("{}< {}", prefix, item_path),
                            RED,
                            options,
                        ));
                        diffs.extend(
                            Self::indent_value(&a1[i], 1)
                                .into_iter()
                                .map(|l| Self::colorize(&format!("< {}", l), RED, options))
                                .collect::<Vec<_>>(),
                        );
                        i += 1;
                    }
                }
            }
            _ if v1 != v2 => {
                diffs.push(Self::colorize(
                    &format!("{}< {}", path, Self::format_value(v1)),
                    RED,
                    options,
                ));
                diffs.push(Self::colorize(
                    &format!("{}> {}", path, Self::format_value(v2)),
                    GREEN,
                    options,
                ));
            }
            _ => {}
        }

        diffs
    }

    fn format_value(v: &Value) -> String {
        match v {
            Value::String(s) => s.clone(),
            Value::Null => "null".to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Number(n) => n.to_string(),
            _ => v.to_string(),
        }
    }

    fn generate_standard_unified_diff(
        v1: &Value,
        v2: &Value,
        path: &str,
        options: &DiffOptions,
    ) -> Vec<String> {
        let mut diffs = Vec::new();
        let context = options.context_lines;

        Self::generate_unified_diff_recursive(v1, v2, path, context, &mut diffs, &mut 1, &mut 1);

        diffs
    }

    fn generate_unified_diff_recursive(
        v1: &Value,
        v2: &Value,
        path: &str,
        _context: usize,
        diffs: &mut Vec<String>,
        src_line: &mut usize,
        dst_line: &mut usize,
    ) {
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
                            let old_start = *src_line;
                            let old_count = 0;
                            let new_start = *dst_line;
                            let new_count = Self::count_value_lines_with_prefix(v2_val, "") + 1;

                            diffs.push(format!(
                                "@@ -{},{} +{},{} @@",
                                old_start, old_count, new_start, new_count
                            ));

                            diffs.push(format!("+{}", k));
                            let indented = Self::indent_value(v2_val, 1);
                            for line in &indented {
                                diffs.push(format!("+{}", line));
                                *dst_line += 1;
                            }
                            *dst_line += 1;
                        }
                        (Some(v1_val), None) => {
                            let old_start = *src_line;
                            let old_count = Self::count_value_lines_with_prefix(v1_val, "") + 1;
                            let new_start = *dst_line;
                            let new_count = 0;

                            diffs.push(format!(
                                "@@ -{},{} +{},{} @@",
                                old_start, old_count, new_start, new_count
                            ));

                            diffs.push(format!("-{}", k));
                            let indented = Self::indent_value(v1_val, 1);
                            for line in &indented {
                                diffs.push(format!("-{}", line));
                                *src_line += 1;
                            }
                            *src_line += 1;
                        }
                        (Some(v1_val), Some(v2_val)) if v1_val != v2_val => {
                            Self::generate_unified_diff_recursive(
                                v1_val, v2_val, &new_path, _context, diffs, src_line, dst_line,
                            );
                        }
                        (Some(_v1_val), Some(_v2_val)) => {
                            diffs.push(format!("  {}", new_path));
                            *src_line += 1;
                            *dst_line += 1;
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
                                let old_start = *src_line;
                                let old_count = 0;
                                let new_start = *dst_line;
                                let new_count = Self::count_value_lines_with_prefix(v2_val, "") + 1;

                                diffs.push(format!(
                                    "@@ -{},{} +{},{} @@",
                                    old_start, old_count, new_start, new_count
                                ));

                                diffs.push(format!("+{}", item_path));
                                let indented = Self::indent_value(v2_val, 1);
                                for line in &indented {
                                    diffs.push(format!("+{}", line));
                                    *dst_line += 1;
                                }
                                *dst_line += 1;
                            }
                            (Some(v1_val), None) => {
                                let old_start = *src_line;
                                let old_count = Self::count_value_lines_with_prefix(v1_val, "") + 1;
                                let new_start = *dst_line;
                                let new_count = 0;

                                diffs.push(format!(
                                    "@@ -{},{} +{},{} @@",
                                    old_start, old_count, new_start, new_count
                                ));

                                diffs.push(format!("-{}", item_path));
                                let indented = Self::indent_value(v1_val, 1);
                                for line in &indented {
                                    diffs.push(format!("-{}", line));
                                    *src_line += 1;
                                }
                                *src_line += 1;
                            }
                            (Some(v1_val), Some(v2_val)) if v1_val != v2_val => {
                                Self::generate_unified_diff_recursive(
                                    v1_val, v2_val, &item_path, _context, diffs, src_line, dst_line,
                                );
                            }
                            (Some(_v1_val), Some(_v2_val)) => {
                                diffs.push(format!("  {}", item_path));
                                *src_line += 1;
                                *dst_line += 1;
                            }
                            _ => {}
                        }
                    }
                }
            }
            (Value::String(s1), Value::String(s2)) if s1 != s2 => {
                let old_start = *src_line;
                let old_count = 1;
                let new_start = *dst_line;
                let new_count = 1;

                diffs.push(format!(
                    "@@ -{},{} +{},{} @@",
                    old_start, old_count, new_start, new_count
                ));
                diffs.push(format!("-{}", s1));
                diffs.push(format!("+{}", s2));
                *src_line += 1;
                *dst_line += 1;
            }
            (Value::Number(n1), Value::Number(n2)) if n1 != n2 => {
                let old_start = *src_line;
                let old_count = 1;
                let new_start = *dst_line;
                let new_count = 1;

                diffs.push(format!(
                    "@@ -{},{} +{},{} @@",
                    old_start, old_count, new_start, new_count
                ));
                diffs.push(format!("-{}", n1));
                diffs.push(format!("+{}", n2));
                *src_line += 1;
                *dst_line += 1;
            }
            (Value::Bool(b1), Value::Bool(b2)) if b1 != b2 => {
                let old_start = *src_line;
                let old_count = 1;
                let new_start = *dst_line;
                let new_count = 1;

                diffs.push(format!(
                    "@@ -{},{} +{},{} @@",
                    old_start, old_count, new_start, new_count
                ));
                diffs.push(format!("-{}", b1));
                diffs.push(format!("+{}", b2));
                *src_line += 1;
                *dst_line += 1;
            }
            _ if v1 != v2 => {
                let old_start = *src_line;
                let old_count = 1;
                let new_start = *dst_line;
                let new_count = 1;

                diffs.push(format!(
                    "@@ -{},{} +{},{} @@",
                    old_start, old_count, new_start, new_count
                ));
                diffs.push(format!("-{}", v1));
                diffs.push(format!("+{}", v2));
                *src_line += 1;
                *dst_line += 1;
            }
            _ => {
                diffs.push(format!("  {}", path));
                *src_line += 1;
                *dst_line += 1;
            }
        }
    }

    fn count_value_lines_with_prefix(v: &Value, prefix: &str) -> usize {
        let s = format!("{}{}", prefix, v);
        s.lines().count()
    }

    fn load_config(path: &str) -> Result<Value, ConfigError> {
        let content = fs::read_to_string(path).map_err(|e| {
            ConfigError::FormatDetectionFailed(format!("Failed to read {}: {}", path, e))
        })?;

        let ext = Path::new(path)
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("");

        match ext {
            "json" => {
                serde_json::from_str(&content).map_err(|e| ConfigError::ParseError(e.to_string()))
            }
            "toml" => toml::from_str(&content).map_err(|e| ConfigError::ParseError(e.to_string())),
            "yaml" | "yml" => {
                serde_yaml::from_str(&content).map_err(|e| ConfigError::ParseError(e.to_string()))
            }
            "ini" => {
                serde_ini::from_str(&content).map_err(|e| ConfigError::ParseError(e.to_string()))
            }
            _ => {
                if let Ok(v) = serde_json::from_str(&content) {
                    Ok(v)
                } else if let Ok(v) = toml::from_str(&content) {
                    Ok(v)
                } else if let Ok(v) = serde_yaml::from_str(&content) {
                    Ok(v)
                } else if let Ok(v) = serde_ini::from_str(&content) {
                    Ok(v)
                } else {
                    Err(ConfigError::FormatDetectionFailed(format!(
                        "Unknown format for file: {}",
                        path
                    )))
                }
            }
        }
    }
}

trait PadToWidth {
    fn pad_to_width(&self, width: usize) -> String;
}

impl PadToWidth for str {
    fn pad_to_width(&self, width: usize) -> String {
        if self.len() >= width {
            self.chars().take(width).collect()
        } else {
            let mut s = self.to_string();
            s.push_str(&" ".repeat(width - self.len()));
            s
        }
    }
}
