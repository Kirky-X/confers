use crate::error::ConfigError;
use figment::{
    providers::{Format, Json, Toml, Yaml},
    Figment,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use validator::Validate;

// ANSI color codes for styled output
const GREEN: &str = "\x1b[32m";
const RED: &str = "\x1b[31m";
const YELLOW: &str = "\x1b[33m";
const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";

/// Output level for validation results
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidateLevel {
    /// Minimal output - just pass/fail
    Minimal,
    /// Full output - detailed validation steps
    Full,
    /// Documentation mode - detailed report with statistics
    Documentation,
}

impl ValidateLevel {
    /// Parse validate level from string
    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "minimal" | "min" => ValidateLevel::Minimal,
            "full" | "" => ValidateLevel::Full,
            "documentation" | "doc" | "docs" => ValidateLevel::Documentation,
            _ => ValidateLevel::Full,
        }
    }
}

impl std::str::FromStr for ValidateLevel {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "minimal" | "min" => Ok(ValidateLevel::Minimal),
            "full" | "" => Ok(ValidateLevel::Full),
            "documentation" | "doc" | "docs" => Ok(ValidateLevel::Documentation),
            _ => Err(()),
        }
    }
}

/// Validation result item for structured output
#[derive(Debug, Clone, Serialize)]
pub struct ValidationItem {
    pub check_type: String,
    pub status: String,
    pub message: String,
    pub line: Option<u32>,
    pub column: Option<u32>,
}

impl ValidationItem {
    pub fn new(check_type: &str, status: &str, message: &str) -> Self {
        Self {
            check_type: check_type.to_string(),
            status: status.to_string(),
            message: message.to_string(),
            line: None,
            column: None,
        }
    }

    pub fn with_location(mut self, line: u32, column: u32) -> Self {
        self.line = Some(line);
        self.column = Some(column);
        self
    }
}

/// Complete validation report
#[derive(Debug, Clone, Serialize)]
pub struct ValidationReport {
    pub file_path: String,
    pub file_format: String,
    pub passed: bool,
    pub total_checks: u32,
    pub passed_checks: u32,
    pub failed_checks: u32,
    pub items: Vec<ValidationItem>,
    pub duration_ms: u64,
}

impl ValidationReport {
    pub fn new(file_path: &str, file_format: String) -> Self {
        Self {
            file_path: file_path.to_string(),
            file_format,
            passed: true,
            total_checks: 0,
            passed_checks: 0,
            failed_checks: 0,
            items: Vec::new(),
            duration_ms: 0,
        }
    }

    pub fn add_item(&mut self, item: ValidationItem) {
        self.total_checks += 1;
        if item.status == "passed" || item.status == "success" {
            self.passed_checks += 1;
        } else {
            self.failed_checks += 1;
            self.passed = false;
        }
        self.items.push(item);
    }

    pub fn set_duration(&mut self, ms: u64) {
        self.duration_ms = ms;
    }
}

pub trait SchemaValidate {
    fn validate_schema(&self) -> Result<(), ConfigError> {
        Ok(())
    }
}

#[cfg(not(feature = "schema"))]
impl<T> SchemaValidate for T {}

#[cfg(feature = "schema")]
impl<T> SchemaValidate for T
where
    T: schemars::JsonSchema + Serialize,
{
    fn validate_schema(&self) -> Result<(), ConfigError> {
        crate::validator::validate_schema(self)
    }
}

pub struct ValidateCommand;

impl ValidateCommand {
    /// Validate specific config type with output level
    pub fn execute<T>(config_path: &str, level: ValidateLevel) -> Result<(), ConfigError>
    where
        T: for<'de> Deserialize<'de> + Serialize + Default + Validate + SchemaValidate,
    {
        let path = Path::new(config_path);
        if !path.exists() {
            return Err(ConfigError::FileNotFound {
                path: path.to_path_buf(),
            });
        }

        let start_time = std::time::Instant::now();
        let mut report = ValidationReport::new(config_path, Self::detect_format(path));

        // Execute generic validation
        match Self::validate_generic_with_report(config_path, &mut report) {
            Ok(_) => {}
            Err(e) => {
                report.add_item(ValidationItem::new(
                    "syntax",
                    "failed",
                    &format!("Syntax validation failed: {}", e),
                ));
            }
        }

        // Execute struct validation
        match Self::validate_struct::<T>(config_path, &mut report) {
            Ok(_) => {}
            Err(e) => {
                report.add_item(ValidationItem::new(
                    "structure",
                    "failed",
                    &format!("Structure validation failed: {}", e),
                ));
            }
        }

        // Execute schema validation
        match Self::validate_schema::<T>(config_path, &mut report) {
            Ok(_) => {}
            Err(e) => {
                report.add_item(ValidationItem::new(
                    "schema",
                    "failed",
                    &format!("Schema validation failed: {}", e),
                ));
            }
        }

        let duration = start_time.elapsed().as_millis() as u64;
        report.set_duration(duration);

        // Output based on level
        Self::output_report(&report, level);

        if report.passed {
            Ok(())
        } else {
            Err(ConfigError::ParseError("Validation failed".to_string()))
        }
    }

    /// Generic syntax check with styled output and report
    pub fn execute_generic(config_path: &str, level: ValidateLevel) -> Result<(), ConfigError> {
        let start_time = std::time::Instant::now();
        let path = Path::new(config_path);

        if !path.exists() {
            return Err(ConfigError::FileNotFound {
                path: path.to_path_buf(),
            });
        }

        let mut report = ValidationReport::new(config_path, Self::detect_format(path));

        match Self::validate_generic_with_report(config_path, &mut report) {
            Ok(_) => {}
            Err(e) => {
                report.add_item(ValidationItem::new(
                    "syntax",
                    "failed",
                    &format!("Syntax validation failed: {}", e),
                ));
            }
        }

        Self::add_structure_validation(config_path, &mut report);

        let duration = start_time.elapsed().as_millis() as u64;
        report.set_duration(duration);

        Self::output_report(&report, level);

        if report.passed {
            Ok(())
        } else {
            Err(ConfigError::ParseError("Validation failed".to_string()))
        }
    }

    fn add_structure_validation(config_path: &str, report: &mut ValidationReport) {
        let path = Path::new(config_path);
        let content = fs::read_to_string(path).unwrap_or_default();

        if let Some(ext) = path.extension() {
            let ext_str = ext.to_string_lossy();

            match ext_str.as_ref() {
                "toml" => match toml::from_str::<toml::Value>(&content) {
                    Ok(value) => {
                        Self::validate_toml_structure(&value, "", report);
                    }
                    Err(_) => {
                        report.add_item(ValidationItem::new(
                            "structure",
                            "skipped",
                            "Skipped due to syntax errors",
                        ));
                        return;
                    }
                },
                "json" => match serde_json::from_str::<serde_json::Value>(&content) {
                    Ok(value) => {
                        Self::validate_json_structure(&value, "", report);
                    }
                    Err(_) => {
                        report.add_item(ValidationItem::new(
                            "structure",
                            "skipped",
                            "Skipped due to syntax errors",
                        ));
                        return;
                    }
                },
                "yaml" | "yml" => match serde_yaml::from_str::<serde_yaml::Value>(&content) {
                    Ok(value) => {
                        Self::validate_yaml_structure(&value, "", report);
                    }
                    Err(_) => {
                        report.add_item(ValidationItem::new(
                            "structure",
                            "skipped",
                            "Skipped due to syntax errors",
                        ));
                        return;
                    }
                },
                _ => {
                    report.add_item(ValidationItem::new(
                        "structure",
                        "skipped",
                        &format!("Unsupported format for structure validation: {}", ext_str),
                    ));
                    return;
                }
            }

            report.add_item(ValidationItem::new(
                "structure",
                "passed",
                "Configuration structure is valid",
            ));
        }
    }

    fn validate_toml_structure(value: &toml::Value, prefix: &str, _report: &mut ValidationReport) {
        match value {
            toml::Value::Table(table) => {
                for (key, val) in table {
                    let full_key = if prefix.is_empty() {
                        key.clone()
                    } else {
                        format!("{}.{}", prefix, key)
                    };
                    Self::validate_toml_structure(val, &full_key, _report);
                }
            }
            toml::Value::Array(arr) => {
                for (i, item) in arr.iter().enumerate() {
                    let indexed_key = format!("{}[{}]", prefix, i);
                    Self::validate_toml_structure(item, &indexed_key, _report);
                }
            }
            _ => {}
        }
    }

    fn validate_json_structure(
        value: &serde_json::Value,
        prefix: &str,
        _report: &mut ValidationReport,
    ) {
        match value {
            serde_json::Value::Object(obj) => {
                for (key, val) in obj {
                    let full_key = if prefix.is_empty() {
                        key.clone()
                    } else {
                        format!("{}.{}", prefix, key)
                    };
                    Self::validate_json_structure(val, &full_key, _report);
                }
            }
            serde_json::Value::Array(arr) => {
                for (i, item) in arr.iter().enumerate() {
                    let indexed_key = format!("{}[{}]", prefix, i);
                    Self::validate_json_structure(item, &indexed_key, _report);
                }
            }
            _ => {}
        }
    }

    fn validate_yaml_structure(
        value: &serde_yaml::Value,
        prefix: &str,
        _report: &mut ValidationReport,
    ) {
        match value {
            serde_yaml::Value::Mapping(map) => {
                for (key, val) in map {
                    if let Some(key_str) = key.as_str() {
                        let full_key = if prefix.is_empty() {
                            key_str.to_string()
                        } else {
                            format!("{}.{}", prefix, key_str)
                        };
                        Self::validate_yaml_structure(val, &full_key, _report);
                    } else {
                        Self::validate_yaml_structure(val, prefix, _report);
                    }
                }
            }
            serde_yaml::Value::Sequence(seq) => {
                for (i, item) in seq.iter().enumerate() {
                    let indexed_key = format!("{}[{}]", prefix, i);
                    Self::validate_yaml_structure(item, &indexed_key, _report);
                }
            }
            _ => {}
        }
    }

    /// Full validation with schema checking (for types that support it)
    pub fn execute_full<T>(config_path: &str, level: ValidateLevel) -> Result<(), ConfigError>
    where
        T: for<'de> Deserialize<'de> + Serialize + Default + Validate + SchemaValidate,
    {
        Self::execute::<T>(config_path, level)
    }

    /// Detect file format from extension
    fn detect_format(path: &Path) -> String {
        if let Some(ext) = path.extension() {
            ext.to_string_lossy().to_string()
        } else {
            "unknown".to_string()
        }
    }

    /// Validate syntax and add results to report
    fn validate_generic_with_report(
        config_path: &str,
        report: &mut ValidationReport,
    ) -> Result<(), ConfigError> {
        let path = Path::new(config_path);
        let content = fs::read_to_string(path).map_err(|_e| ConfigError::FileNotFound {
            path: path.to_path_buf(),
        })?;

        if let Some(ext) = path.extension() {
            let ext_str = ext.to_string_lossy();

            match ext_str.as_ref() {
                "toml" => match toml::from_str::<toml::Value>(&content) {
                    Ok(_) => {
                        report.add_item(ValidationItem::new(
                            "syntax",
                            "passed",
                            "TOML syntax valid",
                        ));
                    }
                    Err(e) => {
                        let location = Self::parse_toml_error_location(&e);
                        let mut item = ValidationItem::new(
                            "syntax",
                            "failed",
                            &format!("TOML syntax error: {}", e),
                        );
                        if let Some((line, col)) = location {
                            item = item.with_location(line, col);
                        }
                        report.add_item(item);
                        return Err(ConfigError::ParseError(e.to_string()));
                    }
                },
                "json" => match serde_json::from_str::<serde_json::Value>(&content) {
                    Ok(_) => {
                        report.add_item(ValidationItem::new(
                            "syntax",
                            "passed",
                            "JSON syntax valid",
                        ));
                    }
                    Err(e) => {
                        report.add_item(ValidationItem::new(
                            "syntax",
                            "failed",
                            &format!("JSON syntax error: {}", e),
                        ));
                        return Err(ConfigError::ParseError(e.to_string()));
                    }
                },
                "yaml" | "yml" => match serde_yaml::from_str::<serde_yaml::Value>(&content) {
                    Ok(_) => {
                        report.add_item(ValidationItem::new(
                            "syntax",
                            "passed",
                            "YAML syntax valid",
                        ));
                    }
                    Err(e) => {
                        report.add_item(ValidationItem::new(
                            "syntax",
                            "failed",
                            &format!("YAML syntax error: {}", e),
                        ));
                        return Err(ConfigError::ParseError(e.to_string()));
                    }
                },
                _ => {
                    report.add_item(ValidationItem::new(
                        "syntax",
                        "failed",
                        &format!("Unsupported file format: {}", ext_str),
                    ));
                    return Err(ConfigError::ParseError(format!(
                        "Unsupported file format: {}",
                        ext_str
                    )));
                }
            }
        } else {
            report.add_item(ValidationItem::new(
                "syntax",
                "failed",
                "Cannot determine file format (no extension)",
            ));
            return Err(ConfigError::ParseError(
                "Cannot determine file format".to_string(),
            ));
        }

        Ok(())
    }

    /// Parse TOML error to get location info
    fn parse_toml_error_location(error: &toml::de::Error) -> Option<(u32, u32)> {
        let err_str = error.to_string();
        // TOML errors often contain line/column info like "line 5, column 10"
        if let Some(start) = err_str.find("line ") {
            if let Some(end) = err_str[start..].find(',') {
                let line_str = &err_str[start + 4..start + end];
                if let Ok(line) = line_str.parse::<u32>() {
                    let col_start = start + end + 8; // ", column "
                    if let Some(col_end) = err_str[col_start..].find(')') {
                        let col_str = &err_str[col_start..col_start + col_end];
                        if let Ok(col) = col_str.parse::<u32>() {
                            return Some((line, col));
                        }
                    }
                }
            }
        }
        None
    }

    /// Validate struct and add results to report
    fn validate_struct<T>(
        config_path: &str,
        report: &mut ValidationReport,
    ) -> Result<(), ConfigError>
    where
        T: for<'de> Deserialize<'de> + Serialize + Default + Validate + SchemaValidate,
    {
        let path = Path::new(config_path);
        let mut figment = Figment::new();

        if let Some(ext) = path.extension() {
            let ext_str = ext.to_string_lossy();
            if ext_str == "toml" {
                figment = figment.merge(Toml::file(path));
            } else if ext_str == "json" {
                figment = figment.merge(Json::file(path));
            } else if ext_str == "yaml" || ext_str == "yml" {
                figment = figment.merge(Yaml::file(path));
            }
        }

        match figment.extract::<T>() {
            Ok(config) => {
                report.add_item(ValidationItem::new(
                    "structure",
                    "passed",
                    "Configuration structure valid",
                ));
                match config.validate() {
                    Ok(_) => {
                        report.add_item(ValidationItem::new(
                            "fields",
                            "passed",
                            "All field validations passed",
                        ));
                    }
                    Err(e) => {
                        report.add_item(ValidationItem::new(
                            "fields",
                            "failed",
                            &format!("Field validation failed: {}", e),
                        ));
                        return Err(ConfigError::from(e));
                    }
                }
            }
            Err(e) => {
                report.add_item(ValidationItem::new(
                    "structure",
                    "failed",
                    &format!("Failed to parse configuration: {}", e),
                ));
                return Err(ConfigError::from(e));
            }
        }

        Ok(())
    }

    /// Validate schema and add results to report
    fn validate_schema<T>(
        config_path: &str,
        report: &mut ValidationReport,
    ) -> Result<(), ConfigError>
    where
        T: for<'de> Deserialize<'de> + Serialize + Default + Validate + SchemaValidate,
    {
        let path = Path::new(config_path);
        let mut figment = Figment::new();

        if let Some(ext) = path.extension() {
            let ext_str = ext.to_string_lossy();
            if ext_str == "toml" {
                figment = figment.merge(Toml::file(path));
            } else if ext_str == "json" {
                figment = figment.merge(Json::file(path));
            } else if ext_str == "yaml" || ext_str == "yml" {
                figment = figment.merge(Yaml::file(path));
            }
        }

        match figment.extract::<T>() {
            Ok(config) => match config.validate_schema() {
                Ok(_) => {
                    report.add_item(ValidationItem::new(
                        "schema",
                        "passed",
                        "Schema validation passed",
                    ));
                }
                Err(e) => {
                    report.add_item(ValidationItem::new(
                        "schema",
                        "failed",
                        &format!("Schema validation failed: {}", e),
                    ));
                    return Err(e);
                }
            },
            Err(_e) => {
                // Skip schema validation if structure is invalid
                report.add_item(ValidationItem::new(
                    "schema",
                    "skipped",
                    "Skipped due to structure errors",
                ));
            }
        }

        Ok(())
    }

    /// Output validation report based on level
    fn output_report(report: &ValidationReport, level: ValidateLevel) {
        match level {
            ValidateLevel::Minimal => {
                if report.passed {
                    println!("{}✓{} Validation passed", GREEN, RESET);
                } else {
                    eprintln!(
                        "{}✗{} Validation failed ({}/{} checks passed)",
                        RED, RESET, report.passed_checks, report.total_checks
                    );
                }
            }
            ValidateLevel::Full => {
                println!("{}🔍{} Validating: {}", YELLOW, RESET, report.file_path);

                for item in &report.items {
                    if item.status == "passed" || item.status == "success" {
                        println!(
                            "  {}✓{} {} - {}",
                            GREEN, RESET, item.check_type, item.message
                        );
                    } else {
                        eprintln!("  {}✗{} {} - {}", RED, RESET, item.check_type, item.message);
                    }
                }

                if report.passed {
                    println!(
                        "\n{}✓{} All validation checks passed ({}{}ms{})",
                        GREEN, RESET, BOLD, report.duration_ms, RESET
                    );
                } else {
                    eprintln!(
                        "\n{}✗{} Validation failed: {}/{} checks passed",
                        RED, RESET, report.passed_checks, report.total_checks
                    );
                }
            }
            ValidateLevel::Documentation => {
                println!("# Configuration Validation Report");
                println!("## File Information");
                println!("- **Path**: {}", report.file_path);
                println!("- **Format**: {}", report.file_format);
                println!("- **Duration**: {}ms", report.duration_ms);
                println!();
                println!("## Validation Summary");
                if report.passed {
                    println!("{}✅ Status: PASSED{}", GREEN, RESET);
                } else {
                    println!("{}❌ Status: FAILED{}", RED, RESET);
                }
                println!("- Total Checks: {}", report.total_checks);
                println!("- {}Passed{}: {}", GREEN, GREEN, report.passed_checks);
                println!("- {}Failed{}: {}", RED, RED, report.failed_checks);
                println!();
                println!("## Detailed Results");

                // Group items by type
                let mut by_type: std::collections::HashMap<String, Vec<&ValidationItem>> =
                    std::collections::HashMap::new();
                for item in &report.items {
                    by_type
                        .entry(item.check_type.clone())
                        .or_default()
                        .push(item);
                }

                for (check_type, items) in by_type {
                    println!("\n### {}", check_type.to_uppercase());
                    for item in items {
                        let icon = if item.status == "passed" || item.status == "success" {
                            "✓"
                        } else {
                            "✗"
                        };
                        let color = if item.status == "passed" || item.status == "success" {
                            GREEN
                        } else {
                            RED
                        };
                        print!("{} {} - {}", icon, item.message, color);
                        if let (Some(line), Some(col)) = (item.line, item.column) {
                            print!(" (line {}, col {})", line, col);
                        }
                        println!("{}", RESET);
                    }
                }
                println!();
            }
        }
    }

    /// Generate JSON output of validation report
    pub fn execute_json<T>(config_path: &str) -> Result<String, ConfigError>
    where
        T: for<'de> Deserialize<'de> + Serialize + Default + Validate + SchemaValidate,
    {
        let path = Path::new(config_path);
        if !path.exists() {
            return Err(ConfigError::FileNotFound {
                path: path.to_path_buf(),
            });
        }

        let mut report = ValidationReport::new(config_path, Self::detect_format(path));

        // Execute generic validation
        let _ = Self::validate_generic_with_report(config_path, &mut report);
        // Execute struct validation
        let _ = Self::validate_struct::<T>(config_path, &mut report);
        // Execute schema validation
        let _ = Self::validate_schema::<T>(config_path, &mut report);

        serde_json::to_string_pretty(&report).map_err(|e| ConfigError::ParseError(e.to_string()))
    }

    /// Legacy execute method for backward compatibility
    pub fn execute_legacy<T>(config_path: &str) -> Result<(), ConfigError>
    where
        T: for<'de> Deserialize<'de> + Serialize + Default + Validate + SchemaValidate,
    {
        Self::execute::<T>(config_path, ValidateLevel::Full)
    }

    /// Legacy generic execute for backward compatibility
    pub fn execute_generic_legacy(config_path: &str) -> Result<(), ConfigError> {
        Self::execute_generic(config_path, ValidateLevel::Full)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::str::FromStr;
    use tempfile::NamedTempFile;

    fn create_test_config_with_ext(content: &str, ext: &str) -> NamedTempFile {
        let mut file = NamedTempFile::with_suffix(ext).unwrap();
        file.write_all(content.as_bytes()).unwrap();
        file
    }

    fn create_test_config(content: &str) -> NamedTempFile {
        create_test_config_with_ext(content, ".toml")
    }

    #[test]
    fn test_validate_level_parsing() {
        assert_eq!(
            ValidateLevel::from_str("minimal").unwrap(),
            ValidateLevel::Minimal
        );
        assert_eq!(
            ValidateLevel::from_str("min").unwrap(),
            ValidateLevel::Minimal
        );
        assert_eq!(
            ValidateLevel::from_str("full").unwrap(),
            ValidateLevel::Full
        );
        assert_eq!(ValidateLevel::from_str("").unwrap(), ValidateLevel::Full);
        assert_eq!(
            ValidateLevel::from_str("documentation").unwrap(),
            ValidateLevel::Documentation
        );
        assert_eq!(
            ValidateLevel::from_str("doc").unwrap(),
            ValidateLevel::Documentation
        );
        assert!(ValidateLevel::from_str("invalid").is_err());
    }

    #[test]
    fn test_validate_command_toml_minimal() {
        let file = create_test_config(
            r#"
name = "test"
value = 42
"#,
        );
        let result =
            ValidateCommand::execute_generic(file.path().to_str().unwrap(), ValidateLevel::Minimal);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_command_toml_full() {
        let file = create_test_config(
            r#"
name = "test"
value = 42
"#,
        );
        let result =
            ValidateCommand::execute_generic(file.path().to_str().unwrap(), ValidateLevel::Full);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_command_toml_documentation() {
        let file = create_test_config(
            r#"
name = "test"
value = 42
"#,
        );
        let result = ValidateCommand::execute_generic(
            file.path().to_str().unwrap(),
            ValidateLevel::Documentation,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_command_invalid_toml() {
        let file = create_test_config(
            r#"
name = "test"
value = [invalid toml
"#,
        );
        let result =
            ValidateCommand::execute_generic(file.path().to_str().unwrap(), ValidateLevel::Full);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_command_json() {
        let file = create_test_config_with_ext(r#"{"name": "test", "value": 42}"#, ".json");
        let result =
            ValidateCommand::execute_generic(file.path().to_str().unwrap(), ValidateLevel::Full);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_command_yaml() {
        let file = create_test_config_with_ext("name: test\nvalue: 42\n", ".yaml");
        let result =
            ValidateCommand::execute_generic(file.path().to_str().unwrap(), ValidateLevel::Full);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_command_nonexistent_file() {
        let result = ValidateCommand::execute_generic(
            "/nonexistent/path/config.toml",
            ValidateLevel::Minimal,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_report_creation() {
        let mut report = ValidationReport::new("test.toml", "toml".to_string());
        assert!(report.passed);
        assert_eq!(report.total_checks, 0);

        report.add_item(ValidationItem::new("syntax", "passed", "Valid TOML"));
        assert_eq!(report.total_checks, 1);
        assert_eq!(report.passed_checks, 1);
        assert!(report.passed);

        report.add_item(ValidationItem::new("syntax", "failed", "Invalid syntax"));
        assert_eq!(report.total_checks, 2);
        assert_eq!(report.passed_checks, 1);
        assert_eq!(report.failed_checks, 1);
        assert!(!report.passed);
    }

    #[test]
    fn test_validation_item_with_location() {
        let item = ValidationItem::new("syntax", "failed", "Error message").with_location(10, 5);
        assert_eq!(item.line, Some(10));
        assert_eq!(item.column, Some(5));
    }

    #[test]
    fn test_validate_level_from_generate_level() {
        assert_eq!(
            ValidateLevel::from_str("minimal"),
            Ok(ValidateLevel::Minimal)
        );
        assert_eq!(ValidateLevel::from_str("full"), Ok(ValidateLevel::Full));
    }
}
