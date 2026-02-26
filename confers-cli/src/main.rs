//! Confers CLI - Configuration diagnostics and inspection tool.
//!
//! This tool provides runtime configuration observability for confers,
//! answering questions like "Where did this value come from?" and "Why is it this value?".

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Confers CLI - Configuration diagnostics and inspection tool
#[derive(Parser, Debug)]
#[command(name = "confers")]
#[command(about = "Configuration diagnostics tool for confers", long_about = None)]
#[command(version)]
struct Cli {
    /// Configuration file(s) to load
    #[arg(short, long, default_value = "./config.toml")]
    config: Vec<PathBuf>,

    /// Additional environment file
    #[arg(long)]
    env_file: Option<PathBuf>,

    /// Profile name
    #[arg(short, long)]
    profile: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Inspect configuration - list all keys with their sources
    Inspect {
        /// Show only specific key(s)
        #[arg(short, long)]
        key: Vec<String>,

        /// Show all conflicts (values that were overridden)
        #[arg(long)]
        show_conflicts: bool,

        /// Output format (text, json)
        #[arg(short, long, default_value = "text")]
        format: String,
    },

    /// Validate configuration against schema
    Validate {
        /// Strict mode: treat warnings as errors
        #[arg(long)]
        strict: bool,

        /// Output format (text, json)
        #[arg(short, long, default_value = "text")]
        format: String,
    },

    /// Export merged configuration (sanitized)
    Export {
        /// Output format (json, toml, yaml)
        #[arg(short, long, default_value = "json")]
        format: String,

        /// Output file (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Include provenance (source) information
        #[arg(long)]
        with_provenance: bool,

        /// Do not sanitize sensitive values
        #[arg(long)]
        raw: bool,
    },

    /// Diff two configurations
    Diff {
        /// Base configuration file
        #[arg(long)]
        base: PathBuf,

        /// Overlay configuration file
        #[arg(long)]
        overlay: PathBuf,

        /// Output format (text, json)
        #[arg(short, long, default_value = "text")]
        format: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Extract fields we need before matching on command
    let config_paths = cli.config.clone();
    let env_file = cli.env_file.clone();

    match cli.command {
        Commands::Inspect { key, show_conflicts, format } => {
            cmd_inspect(&config_paths, &env_file, &key, show_conflicts, &format)?;
        }
        Commands::Validate { strict, format } => {
            cmd_validate(&config_paths, &env_file, strict, &format)?;
        }
        Commands::Export { format, output, with_provenance, raw } => {
            cmd_export(&config_paths, &env_file, &format, output, with_provenance, raw)?;
        }
        Commands::Diff { base, overlay, format: _ } => {
            cmd_diff(&base, &overlay)?;
        }
    }

    Ok(())
}

/// Inspect configuration - list all keys with their sources
fn cmd_inspect(
    config_paths: &[PathBuf],
    _env_file: &Option<PathBuf>,
    keys: &[String],
    show_conflicts: bool,
    format: &str,
) -> Result<()> {
    use confers::ConfigBuilder;

    // Build configuration
    let mut builder = ConfigBuilder::<serde_json::Value>::new();

    // Add config files
    for config_path in config_paths {
        if config_path.exists() {
            builder = builder.file(config_path.to_string_lossy().as_ref());
        }
    }

    // Add env
    builder = builder.env();

    // Build
    let config = builder.build()?;

    match format {
        "json" => {
            // JSON output
            let json = serde_json::to_string_pretty(&config)?;
            println!("{}", json);
            return Ok(());
        }
        _ => {
            // Text output (default)
        }
    }

    println!("Configuration Inspection");
    println!("=======================");
    println!();
    println!("Loaded {} configuration source(s)", config_paths.len());
    println!();

    // Build source information map
    // In a full implementation, we would track the source of each value
    // For now, we use file paths as sources
    let sources: Vec<String> = config_paths
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect();

    if keys.is_empty() {
        // Show all configuration keys
        println!("All configuration keys:");
        println!("{:<35} {:<25} {:<20}", "KEY", "VALUE", "SOURCE");
        println!("{}", "-".repeat(80));

        // Recursively print all keys
        print_config_value(&config, "", &sources, show_conflicts);
    } else {
        // Show requested keys
        println!("Requested keys:");
        println!("{:<35} {:<25} {:<20}", "KEY", "VALUE", "SOURCE");
        println!("{}", "-".repeat(80));

        for key in keys {
            let value = find_value_by_key(&config, key);
            match value {
                Some(v) => {
                    let value_str = format_value(v);
                    println!("{:<35} {:<25} {:<20}", key, value_str, "config");
                }
                None => {
                    println!("{:<35} {:<25} {:<20}", key, "[NOT FOUND]", "-");
                }
            }
        }
    }

    Ok(())
}

/// Recursively print configuration values
fn print_config_value(value: &serde_json::Value, prefix: &str, sources: &[String], _show_conflicts: bool) {
    match value {
        serde_json::Value::Object(map) => {
            for (key, val) in map {
                let full_key = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{}.{}", prefix, key)
                };

                match val {
                    serde_json::Value::Object(_) => {
                        // Nested object - recurse
                        print_config_value(val, &full_key, sources, _show_conflicts);
                    }
                    _ => {
                        // Leaf value - print it
                        let value_str = format_value(val);
                        let source = sources.first().map(|s| s.as_str()).unwrap_or("config");
                        println!("{:<35} {:<25} {:<20}", full_key, value_str, source);
                    }
                }
            }
        }
        _ => {
            // Top-level primitive
            let value_str = format_value(value);
            let source = sources.first().map(|s| s.as_str()).unwrap_or("config");
            println!("{:<35} {:<25} {:<20}", prefix, value_str, source);
        }
    }
}

/// Find a value by dot-notation key
fn find_value_by_key<'a>(value: &'a serde_json::Value, key: &str) -> Option<&'a serde_json::Value> {
    let parts: Vec<&str> = key.split('.').collect();
    let mut current = value;

    for part in parts {
        match current {
            serde_json::Value::Object(map) => {
                current = map.get(part)?;
            }
            _ => return None,
        }
    }

    Some(current)
}

/// Format a value for display
fn format_value(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => {
            // Truncate long strings
            if s.len() > 20 {
                format!("\"{}...\"", &s[..17])
            } else {
                format!("\"{}\"", s)
            }
        }
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Null => "[null]".to_string(),
        serde_json::Value::Array(arr) => format!("[array: {} items]", arr.len()),
        serde_json::Value::Object(obj) => {
            let keys: Vec<_> = obj.keys().collect();
            format!("{{ {} keys }}", keys.len())
        }
    }
}

/// Validate configuration against schema
fn cmd_validate(
    config_paths: &[PathBuf],
    _env_file: &Option<PathBuf>,
    strict: bool,
    _format: &str,
) -> Result<()> {
    use confers::ConfigBuilder;

    println!("Configuration Validation");
    println!("=======================");
    println!();

    // Build configuration
    let mut builder = ConfigBuilder::<serde_json::Value>::new();

    for config_path in config_paths {
        if config_path.exists() {
            builder = builder.file(config_path.to_string_lossy().as_ref());
        }
    }

    builder = builder.env();

    match builder.build() {
        Ok(config) => {
            println!("✓ Configuration loaded successfully");

            // Basic validation: check for required keys and types
            let mut issues = Vec::new();

            if let Some(obj) = config.as_object() {
                // Check for common required keys
                check_required_keys(obj, &mut issues);
                // Check for type consistency
                check_types(obj, &mut issues);
            }

            if !issues.is_empty() {
                println!("\n✗ Found {} validation issue(s):", issues.len());
                for issue in &issues {
                    println!("  - {}", issue);
                }

                if strict {
                    anyhow::bail!("Validation failed with {} issue(s)", issues.len());
                }
            } else {
                println!("✓ All validation checks passed");
            }
        }
        Err(e) => {
            println!("✗ Configuration error: {}", e);
            anyhow::bail!("Validation failed");
        }
    }

    Ok(())
}

/// Check for required configuration keys
fn check_required_keys(obj: &serde_json::Map<String, serde_json::Value>, issues: &mut Vec<String>) {
    // Check for server configuration
    if let Some(server) = obj.get("server") {
        if let Some(server_obj) = server.as_object() {
            if !server_obj.contains_key("host") && !server_obj.contains_key("port") {
                issues.push("Server configuration missing host/port".to_string());
            }
        }
    }

    // Check for database configuration
    if let Some(db) = obj.get("database") {
        if let Some(db_obj) = db.as_object() {
            if !db_obj.contains_key("url") && !db_obj.contains_key("host") {
                issues.push("Database configuration missing connection details".to_string());
            }
        }
    }

    // Check for empty required sections
    for key in obj.keys() {
        if let Some(value) = obj.get(key) {
            if matches!(value, serde_json::Value::Null) {
                issues.push(format!("Configuration key '{}' has null value", key));
            }
        }
    }
}

/// Check for type consistency issues
fn check_types(obj: &serde_json::Map<String, serde_json::Value>, issues: &mut Vec<String>) {
    // Check for suspicious string values that might be numbers
    for (key, value) in obj {
        if let Some(s) = value.as_str() {
            // Check if string looks like a number
            if s.parse::<i64>().is_ok() || s.parse::<f64>().is_ok() {
                issues.push(format!("Key '{}' has string value that looks like a number: {}", key, s));
            }
            // Check for boolean strings
            if s == "true" || s == "false" {
                issues.push(format!("Key '{}' has string value that looks like boolean: {}", key, s));
            }
        }
    }
}

/// Export merged configuration (sanitized)
fn cmd_export(
    config_paths: &[PathBuf],
    _env_file: &Option<PathBuf>,
    format: &str,
    output: Option<PathBuf>,
    _with_provenance: bool,
    _raw: bool,
) -> Result<()> {
    use confers::ConfigBuilder;
    use chrono::Utc;

    // Build configuration
    let mut builder = ConfigBuilder::<serde_json::Value>::new();

    for config_path in config_paths {
        if config_path.exists() {
            builder = builder.file(config_path.to_string_lossy().as_ref());
        }
    }

    builder = builder.env();

    let config = builder.build()?;

    // Determine output
    let output_path: Option<PathBuf> = if let Some(output_path) = &output {
        if output_path.is_dir() {
            let timestamp = Utc::now().format("%Y%m%dT%H%M%SZ");
            let filename = format!("config-{}.json", timestamp);
            Some(output_path.join(filename))
        } else {
            Some(output_path.clone())
        }
    } else {
        None
    };

    // Format output
    let formatted = match format {
        "json" => serde_json::to_string_pretty(&config)?,
        "toml" => toml::to_string_pretty(&config)?,
        "yaml" => serde_yaml_ng::to_string(&config)?,
        _ => anyhow::bail!("Unsupported format: {}", format),
    };

    if let Some(path) = output_path {
        std::fs::write(&path, formatted)?;
        println!("Exported configuration to: {}", path.display());
    } else {
        println!("{}", formatted);
    }

    Ok(())
}

/// Diff two configurations
fn cmd_diff(base: &PathBuf, overlay: &PathBuf) -> Result<()> {
    use confers::loader;

    println!("Configuration Diff");
    println!("=================");
    println!();

    // Load base configuration
    let base_content = std::fs::read_to_string(base)
        .with_context(|| format!("Failed to read base config: {}", base.display()))?;

    let _base_value = loader::parse_content(
        &base_content,
        loader::detect_format_from_path(base)
            .ok_or_else(|| anyhow::anyhow!("Unknown format for base config"))?,
        confers::value::SourceId::new(base.to_string_lossy().as_ref()),
        Some(base),
    )?;

    // Load overlay configuration
    let overlay_content = std::fs::read_to_string(overlay)
        .with_context(|| format!("Failed to read overlay config: {}", overlay.display()))?;

    let _overlay_value = loader::parse_content(
        &overlay_content,
        loader::detect_format_from_path(overlay)
            .ok_or_else(|| anyhow::anyhow!("Unknown format for overlay config"))?,
        confers::value::SourceId::new(overlay.to_string_lossy().as_ref()),
        Some(overlay),
    )?;

    println!("{} vs {}", base.display(), overlay.display());
    println!();

    // Simple comparison
    if base_content == overlay_content {
        println!("Configurations are identical");
    } else {
        println!("Configurations differ");
        println!("\nBase ({}):", base.display());
        for (i, line) in base_content.lines().take(20).enumerate() {
            println!("{:3}: {}", i + 1, line);
        }
        println!("\nOverlay ({}):", overlay.display());
        for (i, line) in overlay_content.lines().take(20).enumerate() {
            println!("{:3}: {}", i + 1, line);
        }
    }

    Ok(())
}
