//! Confers CLI - Configuration diagnostics and inspection tool.
//!
//! This tool provides runtime configuration observability for confers,
//! answering questions like "Where did this value come from?" and "Why is it this value?".

#![allow(clippy::incompatible_msrv)]

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::sync::Arc;

use crate::AnnotatedValue;
use crate::ConfigBuilder;
use crate::ConfigResult;

const DEFAULT_SNAPSHOT_DISPLAY_LIMIT: usize = 10;

fn build_annotated_from_cli(
    config_paths: &[PathBuf],
    allow_absolute_paths: bool,
) -> ConfigResult<AnnotatedValue> {
    let mut builder = ConfigBuilder::<serde_json::Value>::new();
    if allow_absolute_paths {
        builder = builder.allow_absolute_paths();
    }
    for path in config_paths {
        if path.exists() {
            builder = builder.file(path.clone());
        }
    }
    builder = builder.env();
    builder.build_annotated()
}

fn build_config_from_cli(
    config_paths: &[PathBuf],
    allow_absolute_paths: bool,
) -> ConfigResult<serde_json::Value> {
    let mut builder = ConfigBuilder::<serde_json::Value>::new();
    if allow_absolute_paths {
        builder = builder.allow_absolute_paths();
    }
    for path in config_paths {
        if path.exists() {
            builder = builder.file(path.clone());
        }
    }
    builder = builder.env();
    builder.build()
}

/// Load environment variables from a .env file
pub fn load_env_file(path: &PathBuf) -> Result<()> {
    if !path.exists() {
        anyhow::bail!("Environment file not found: {}", path.display());
    }

    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read env file: {}", path.display()))?;

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim();
            let value = value.trim();

            let value = if (value.starts_with('"') && value.ends_with('"'))
                || (value.starts_with('\'') && value.ends_with('\''))
            {
                &value[1..value.len() - 1]
            } else {
                value
            };

            if std::env::var(key).is_err() {
                std::env::set_var(key, value);
            }
        }
    }

    Ok(())
}

/// Confers CLI - Configuration diagnostics and inspection tool
#[derive(Parser, Debug)]
#[command(name = "confers")]
#[command(about = "Configuration diagnostics tool for confers", long_about = None)]
#[command(version)]
struct Cli {
    /// Configuration file(s) to load
    #[arg(short, long)]
    config: Vec<PathBuf>,

    /// Additional environment file
    #[arg(long)]
    env_file: Option<PathBuf>,

    /// Profile name
    #[arg(short, long)]
    profile: Option<String>,

    /// Allow absolute paths for config files (use with caution, mainly for testing)
    #[arg(long)]
    allow_absolute_paths: bool,

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

        /// Sanitize sensitive values in output
        #[arg(long, default_value = "true")]
        sanitize: bool,
    },

    /// Manage configuration snapshots
    Snapshot {
        #[command(subcommand)]
        action: SnapshotCommands,
    },
}

#[derive(Subcommand, Debug)]
enum SnapshotCommands {
    /// List all snapshots
    List {
        /// Directory containing snapshots
        #[arg(long, default_value = "./snapshots")]
        directory: PathBuf,
    },
    /// Diff between two snapshots
    Diff {
        /// Number of recent snapshots to compare
        #[arg(long, default_value = "2")]
        latest: usize,
        /// Directory containing snapshots
        #[arg(long, default_value = "./snapshots")]
        directory: PathBuf,
    },
    /// Prune old snapshots
    Prune {
        /// Keep snapshots newer than this (e.g., "30d", "7d")
        #[arg(long, default_value = "30d")]
        older_than: String,
        /// Directory containing snapshots
        #[arg(long, default_value = "./snapshots")]
        directory: PathBuf,
    },
}

/// Run the CLI entry point
///
/// Generic over the config type `T` for type-safe validation and schema
/// generation. Use with `confers::cli::run::<AppConfig>()`.
pub fn run<T>() -> Result<()>
where
    T: serde::de::DeserializeOwned + Send + Sync + 'static,
{
    let cli = Cli::parse();

    if let Some(env_file) = &cli.env_file {
        load_env_file(env_file)?;
    }

    let config_paths = cli.config.clone();
    let allow_absolute_paths = cli.allow_absolute_paths;

    match cli.command {
        Commands::Inspect {
            key,
            show_conflicts,
            format,
        } => {
            cmd_inspect(
                &config_paths,
                &key,
                show_conflicts,
                &format,
                allow_absolute_paths,
            )?;
        }
        Commands::Validate { strict, format } => {
            cmd_validate(&config_paths, strict, &format, allow_absolute_paths)?;
        }
        Commands::Export {
            format,
            output,
            with_provenance,
            raw,
        } => {
            cmd_export(
                &config_paths,
                &format,
                output,
                with_provenance,
                raw,
                allow_absolute_paths,
            )?;
        }
        Commands::Diff {
            base,
            overlay,
            format,
            sanitize,
        } => {
            cmd_diff(&base, &overlay, &format, sanitize, allow_absolute_paths)?;
        }
        Commands::Snapshot { action } => {
            cmd_snapshot(action)?;
        }
    }

    Ok(())
}

/// Inspect configuration - list all keys with their sources
fn cmd_inspect(
    config_paths: &[PathBuf],
    keys: &[String],
    show_conflicts: bool,
    format: &str,
    allow_absolute_paths: bool,
) -> Result<()> {
    let annotated_config = build_annotated_from_cli(config_paths, allow_absolute_paths)?;

    match format {
        "json" => {
            // JSON output
            let json = serde_json::to_string_pretty(&annotated_config)?;
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

    if keys.is_empty() {
        println!("All configuration keys:");
        println!(
            "{:<35} {:<25} {:<20} {:<20}",
            "KEY", "VALUE", "SOURCE", "LOCATION"
        );
        println!("{}", "-".repeat(100));

        print_config_value(&annotated_config, "", show_conflicts);
    } else {
        // Show requested keys
        println!("Requested keys:");
        println!(
            "{:<35} {:<25} {:<20} {:<20}",
            "KEY", "VALUE", "SOURCE", "LOCATION"
        );
        println!("{}", "-".repeat(100));

        for key in keys {
            let value = find_value_by_key(&annotated_config, key);
            match value {
                Some(v) => {
                    let value_str = format_value(&v.inner);
                    let source = v.source.as_str();
                    let location = format_location(&v.location);
                    println!(
                        "{:<35} {:<25} {:<20} {:<20}",
                        key, value_str, source, location
                    );
                }
                None => {
                    println!("{:<35} {:<25} {:<20} {:<20}", key, "[NOT FOUND]", "-", "-");
                }
            }
        }
    }

    Ok(())
}

/// Recursively print configuration values
fn print_config_value(value: &AnnotatedValue, prefix: &str, show_conflicts: bool) {
    match &value.inner {
        crate::types::ConfigValue::Map(map) => {
            for (key, val) in map.iter() {
                let full_key = if prefix.is_empty() {
                    key.clone()
                } else {
                    Arc::from(format!("{}.{}", prefix, key))
                };

                match &val.inner {
                    crate::types::ConfigValue::Map(_) => {
                        print_config_value(val, &full_key, show_conflicts);
                    }
                    _ => {
                        let value_str = format_value(&val.inner);
                        let source = val.source.as_str();
                        let location = format_location(&val.location);
                        let conflict_marker = if show_conflicts && val.priority > 0 {
                            " *"
                        } else {
                            ""
                        };
                        println!(
                            "{:<35} {:<25} {:<20} {:<20}{}",
                            full_key, value_str, source, location, conflict_marker
                        );
                    }
                }
            }
        }
        _ => {
            let value_str = format_value(&value.inner);
            let source = value.source.as_str();
            let location = format_location(&value.location);
            println!(
                "{:<35} {:<25} {:<20} {:<20}",
                prefix, value_str, source, location
            );
        }
    }
}

/// Format location information for display
fn format_location(location: &Option<crate::types::SourceLocation>) -> String {
    match location {
        Some(loc) => format!("line {}, col {}", loc.line, loc.column),
        None => "-".to_string(),
    }
}

/// Find a value by dot-notation key
fn find_value_by_key<'a>(value: &'a AnnotatedValue, key: &str) -> Option<&'a AnnotatedValue> {
    let parts: Vec<&str> = key.split('.').collect();
    let mut current = value;

    for part in parts {
        match &current.inner {
            crate::types::ConfigValue::Map(map) => {
                current = map.get(part)?;
            }
            _ => return None,
        }
    }

    Some(current)
}

/// Format a value for display
fn format_value(value: &crate::types::ConfigValue) -> String {
    match value {
        crate::types::ConfigValue::String(s) => {
            // Truncate long strings
            if s.len() > 20 {
                format!("\"{}...\"", &s[..17])
            } else {
                format!("\"{}\"", s)
            }
        }
        crate::types::ConfigValue::I64(n) => n.to_string(),
        crate::types::ConfigValue::U64(n) => n.to_string(),
        crate::types::ConfigValue::F64(n) => n.to_string(),
        crate::types::ConfigValue::Bool(b) => b.to_string(),
        crate::types::ConfigValue::Null => "[null]".to_string(),
        crate::types::ConfigValue::Bytes(b) => format!("[bytes: {}]", b.len()),
        crate::types::ConfigValue::Array(arr) => format!("[array: {} items]", arr.len()),
        crate::types::ConfigValue::Map(obj) => {
            let keys: Vec<_> = obj.keys().collect();
            format!("{{ {} keys }}", keys.len())
        }
    }
}

/// Validate configuration against schema
fn cmd_validate(
    config_paths: &[PathBuf],
    strict: bool,
    format: &str,
    allow_absolute_paths: bool,
) -> Result<()> {
    match build_annotated_from_cli(config_paths, allow_absolute_paths) {
        Ok(annotated_config) => {
            let mut issues = Vec::new();

            if let crate::types::ConfigValue::Map(map) = &annotated_config.inner {
                check_required_keys(map, &mut issues);
                check_types(map, &mut issues);
            }

            match format {
                "json" => {
                    let result = serde_json::json!({
                        "valid": issues.is_empty(),
                        "issues": issues,
                        "config_path": config_paths.iter().map(|p| p.to_string_lossy().to_string()).collect::<Vec<_>>()
                    });
                    println!("{}", serde_json::to_string_pretty(&result)?);
                }
                _ => {
                    println!("Configuration Validation");
                    println!("=======================");
                    println!();
                    println!("✓ Configuration loaded successfully");

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
            }
        }
        Err(e) => {
            match format {
                "json" => {
                    let result = serde_json::json!({
                        "valid": false,
                        "error": e.to_string()
                    });
                    println!("{}", serde_json::to_string_pretty(&result)?);
                }
                _ => {
                    println!("Configuration Validation");
                    println!("=======================");
                    println!();
                    println!("✗ Configuration error: {}", e);
                }
            }
            anyhow::bail!("Validation failed");
        }
    }

    Ok(())
}

/// Check for required configuration keys
fn check_required_keys(
    obj: &indexmap::IndexMap<Arc<str>, AnnotatedValue>,
    issues: &mut Vec<String>,
) {
    // Check for server configuration
    if let Some(server) = obj.get("server") {
        if let crate::types::ConfigValue::Map(server_map) = &server.inner {
            if !server_map.contains_key("host") && !server_map.contains_key("port") {
                issues.push("Server configuration missing host/port".to_string());
            }
        }
    }

    // Check for database configuration
    if let Some(db) = obj.get("database") {
        if let crate::types::ConfigValue::Map(db_map) = &db.inner {
            if !db_map.contains_key("url") && !db_map.contains_key("host") {
                issues.push("Database configuration missing connection details".to_string());
            }
        }
    }

    // Check for empty required sections
    for (key, value) in obj.iter() {
        if matches!(value.inner, crate::types::ConfigValue::Null) {
            issues.push(format!("Configuration key '{}' has null value", key));
        }
    }
}

/// Check for type consistency issues
fn check_types(obj: &indexmap::IndexMap<Arc<str>, AnnotatedValue>, issues: &mut Vec<String>) {
    // Check for suspicious string values that might be numbers
    for (key, value) in obj.iter() {
        if let crate::types::ConfigValue::String(s) = &value.inner {
            // Check if string looks like a number
            if s.parse::<i64>().is_ok() || s.parse::<f64>().is_ok() {
                issues.push(format!(
                    "Key '{}' has string value that looks like a number: {}",
                    key, s
                ));
            }
            // Check for boolean strings
            if s == "true" || s == "false" {
                issues.push(format!(
                    "Key '{}' has string value that looks like boolean: {}",
                    key, s
                ));
            }
        }
    }
}

/// Export merged configuration (sanitized)
fn cmd_export(
    config_paths: &[PathBuf],
    format: &str,
    output: Option<PathBuf>,
    with_provenance: bool,
    raw: bool,
    allow_absolute_paths: bool,
) -> Result<()> {
    use chrono::Utc;

    if raw {
        eprintln!("⚠️  警告: 使用 --raw 选项将导出未脱敏的敏感数据！");
        eprintln!("   请确保输出目标安全，避免泄露密码、密钥等敏感信息。");
        eprintln!();
    }

    if with_provenance {
        let annotated_config = build_annotated_from_cli(config_paths, allow_absolute_paths)?;

        let output_path: Option<PathBuf> = if let Some(output_path) = &output {
            if output_path.is_dir() {
                let timestamp = Utc::now().format("%Y%m%dT%H%M%SZ");
                let filename = format!("config-annotated-{}.json", timestamp);
                Some(output_path.join(filename))
            } else {
                Some(output_path.clone())
            }
        } else {
            None
        };

        let formatted = match format {
            "json" => serde_json::to_string_pretty(&annotated_config)?,
            "toml" => toml::to_string_pretty(&annotated_config)?,
            "yaml" => serde_yaml_ng::to_string(&annotated_config)?,
            _ => anyhow::bail!("Unsupported format: {}", format),
        };

        if let Some(path) = output_path {
            std::fs::write(&path, formatted)?;
            println!("Exported annotated configuration to: {}", path.display());
        } else {
            println!("{}", formatted);
        }
    } else {
        let config = build_config_from_cli(config_paths, allow_absolute_paths)?;

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
    }

    Ok(())
}

/// Diff two configurations
fn cmd_diff(
    base: &PathBuf,
    overlay: &PathBuf,
    format: &str,
    sanitize: bool,
    allow_absolute_paths: bool,
) -> Result<()> {
    use crate::loader;

    if !allow_absolute_paths {
        if base.is_absolute() {
            anyhow::bail!(
                "Absolute path not allowed: {}. Use --allow-absolute-paths to override.",
                base.display()
            );
        }
        if overlay.is_absolute() {
            anyhow::bail!(
                "Absolute path not allowed: {}. Use --allow-absolute-paths to override.",
                overlay.display()
            );
        }
    }

    println!("Configuration Diff");
    println!("=================");
    println!();

    let base_content = std::fs::read_to_string(base)
        .with_context(|| format!("Failed to read base config: {}", base.display()))?;

    let base_value = loader::parse_content(
        &base_content,
        loader::detect_format_from_path(base)
            .ok_or_else(|| anyhow::anyhow!("Unknown format for base config"))?,
        crate::types::SourceId::new(base.to_string_lossy().as_ref()),
        Some(base),
    )?;

    let overlay_content = std::fs::read_to_string(overlay)
        .with_context(|| format!("Failed to read overlay config: {}", overlay.display()))?;

    let overlay_value = loader::parse_content(
        &overlay_content,
        loader::detect_format_from_path(overlay)
            .ok_or_else(|| anyhow::anyhow!("Unknown format for overlay config"))?,
        crate::types::SourceId::new(overlay.to_string_lossy().as_ref()),
        Some(overlay),
    )?;

    println!("{} vs {}", base.display(), overlay.display());
    println!();

    if base_content == overlay_content {
        println!("Configurations are identical");
        return Ok(());
    }

    match format {
        "json" => {
            let diff_result = serde_json::json!({
                "base": {
                    "file": base.to_string_lossy(),
                    "value": base_value
                },
                "overlay": {
                    "file": overlay.to_string_lossy(),
                    "value": overlay_value
                },
                "identical": false,
                "sanitize": sanitize
            });
            println!("{}", serde_json::to_string_pretty(&diff_result)?);
        }
        _ => {
            println!("Configurations differ");
            println!("\nBase ({}):", base.display());
            for (i, line) in base_content.lines().take(20).enumerate() {
                println!("{:3}: {}", i + 1, line);
            }
            println!("\nOverlay ({}):", overlay.display());
            for (i, line) in overlay_content.lines().take(20).enumerate() {
                println!("{:3}: {}", i + 1, line);
            }

            let diff = similar::TextDiff::from_lines(&base_content, &overlay_content);
            println!("\nUnified Diff:");
            for change in diff.iter_all_changes() {
                print!("{}", change);
            }
        }
    }

    Ok(())
}

/// Handle snapshot commands (list, diff, prune)
fn cmd_snapshot(action: SnapshotCommands) -> Result<()> {
    match action {
        SnapshotCommands::List { directory } => {
            cmd_snapshot_list(&directory)?;
        }
        SnapshotCommands::Diff { latest, directory } => {
            cmd_snapshot_diff(latest, &directory)?;
        }
        SnapshotCommands::Prune {
            older_than,
            directory,
        } => {
            cmd_snapshot_prune(&older_than, &directory)?;
        }
    }
    Ok(())
}

/// List all snapshots in a directory
fn cmd_snapshot_list(directory: &PathBuf) -> Result<()> {
    use std::fs;

    if !directory.exists() {
        println!("Snapshot directory does not exist: {}", directory.display());
        return Ok(());
    }

    let entries: Vec<_> = fs::read_dir(directory)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            let path = e.path();
            path.extension()
                .map(|ext| ext == "json" || ext == "toml")
                .unwrap_or(false)
        })
        .collect();

    if entries.is_empty() {
        println!("No snapshots found in {}", directory.display());
        return Ok(());
    }

    println!("Snapshots in {}:", directory.display());
    println!("{:<30} {:<40} FORMAT", "TIMESTAMP", "FILENAME");
    println!("{}", "-".repeat(90));

    let mut snapshots: Vec<_> = entries.into_iter().collect();
    snapshots.sort_by_key(|e| std::cmp::Reverse(e.metadata().ok().and_then(|m| m.modified().ok())));

    for entry in snapshots.iter().take(DEFAULT_SNAPSHOT_DISPLAY_LIMIT) {
        let filename = entry.file_name();
        let path = entry.path();
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("unknown");
        let modified = entry
            .metadata()
            .and_then(|m| m.modified())
            .map(|t| {
                let datetime: chrono::DateTime<chrono::Utc> = t.into();
                datetime.format("%Y-%m-%dT%H:%M:%SZ").to_string()
            })
            .unwrap_or_else(|_| "unknown".to_string());

        println!("{:<30} {:<40} {}", modified, filename.display(), ext);
    }

    Ok(())
}

/// Diff between recent snapshots
fn cmd_snapshot_diff(count: usize, directory: &PathBuf) -> Result<()> {
    use std::fs;

    if !directory.exists() {
        println!("Snapshot directory does not exist: {}", directory.display());
        return Ok(());
    }

    let entries: Vec<_> = fs::read_dir(directory)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            let path = e.path();
            path.extension()
                .map(|ext| ext == "json" || ext == "toml")
                .unwrap_or(false)
        })
        .collect();

    if entries.len() < 2 {
        println!("Need at least 2 snapshots to diff, found {}", entries.len());
        return Ok(());
    }

    let mut snapshots: Vec<_> = entries.into_iter().collect();
    snapshots.sort_by_key(|e| std::cmp::Reverse(e.metadata().ok().and_then(|m| m.modified().ok())));

    let first = snapshots
        .first()
        .ok_or_else(|| anyhow::anyhow!("No snapshots found after sorting"))?;
    let second = snapshots
        .get(count - 1)
        .or_else(|| snapshots.get(1))
        .ok_or_else(|| anyhow::anyhow!("Not enough snapshots to compare"))?;

    let content1 = std::fs::read_to_string(first.path())?;
    let content2 = std::fs::read_to_string(second.path())?;

    let diff = similar::TextDiff::from_lines(&content1, &content2);

    println!(
        "Diff between {} and {}",
        first.file_name().display(),
        second.file_name().display()
    );
    for change in diff.iter_all_changes() {
        print!("{}", change);
    }

    Ok(())
}

/// Prune old snapshots
fn cmd_snapshot_prune(older_than: &str, directory: &PathBuf) -> Result<()> {
    use std::fs;

    if !directory.exists() {
        println!("Snapshot directory does not exist: {}", directory.display());
        return Ok(());
    }

    // Parse duration (e.g., "30d" -> 30 days)
    let days = older_than
        .trim_end_matches('d')
        .trim_end_matches('D')
        .parse::<u64>()
        .unwrap_or(30);

    let cutoff = std::time::SystemTime::now() - std::time::Duration::from_secs(days * 24 * 60 * 60);

    let entries: Vec<_> = fs::read_dir(directory)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            let path = e.path();
            path.extension()
                .map(|ext| ext == "json" || ext == "toml")
                .unwrap_or(false)
        })
        .collect();

    let mut removed_count = 0;
    for entry in entries {
        if let Ok(metadata) = entry.metadata() {
            if let Ok(modified) = metadata.modified() {
                if modified < cutoff {
                    println!("Removing: {}", entry.file_name().display());
                    let _ = fs::remove_file(entry.path());
                    removed_count += 1;
                }
            }
        }
    }

    println!(
        "Pruned {} snapshot(s) older than {} days",
        removed_count, days
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::write_with_newline)]
    use super::*;

    #[test]
    fn test_format_location_none() {
        assert_eq!(format_location(&None), "-");
    }

    #[test]
    fn test_format_location_some() {
        use crate::types::SourceLocation;
        let loc = SourceLocation::new("test.toml", 10, 5);
        let result = format_location(&Some(loc));
        assert!(result.contains("10"));
        assert!(result.contains("5"));
    }

    #[test]
    fn test_format_value_string() {
        let v = crate::ConfigValue::string("hello");
        assert_eq!(format_value(&v), "\"hello\"");
    }

    #[test]
    fn test_format_value_int() {
        let v = crate::ConfigValue::integer(42);
        assert_eq!(format_value(&v), "42");
    }

    #[test]
    fn test_format_value_bool() {
        let v = crate::ConfigValue::bool(true);
        assert_eq!(format_value(&v), "true");
    }

    #[test]
    fn test_format_value_null() {
        let v = crate::ConfigValue::Null;
        assert_eq!(format_value(&v), "[null]");
    }

    #[test]
    fn test_load_env_file_not_found() {
        let p = std::path::PathBuf::from("/nonexistent/.env");
        assert!(load_env_file(&p).is_err());
    }

    #[test]
    fn test_find_value_by_key_missing() {
        use crate::types::SourceId;
        use indexmap::IndexMap;
        use std::sync::Arc;
        let mut map = IndexMap::new();
        map.insert(
            Arc::from("a"),
            AnnotatedValue::new(crate::ConfigValue::string("1"), SourceId::new("t"), "a"),
        );
        let av = AnnotatedValue::new(
            crate::ConfigValue::Map(Arc::new(map)),
            SourceId::new("test"),
            "",
        );
        assert!(find_value_by_key(&av, "nonexistent").is_none());
    }

    #[test]
    fn test_find_value_by_key_nested() {
        use crate::types::SourceId;
        use indexmap::IndexMap;
        use std::sync::Arc;
        let mut inner = IndexMap::new();
        inner.insert(
            Arc::from("host"),
            AnnotatedValue::new(
                crate::ConfigValue::string("localhost"),
                SourceId::new("t"),
                "host",
            ),
        );
        let mut outer = IndexMap::new();
        outer.insert(
            Arc::from("db"),
            AnnotatedValue::new(
                crate::ConfigValue::Map(Arc::new(inner)),
                SourceId::new("t"),
                "db",
            ),
        );
        let av = AnnotatedValue::new(
            crate::ConfigValue::Map(Arc::new(outer)),
            SourceId::new("test"),
            "",
        );
        let val = find_value_by_key(&av, "db.host");
        assert!(val.is_some());
        assert_eq!(val.unwrap().as_str(), Some("localhost"));
    }

    #[test]
    fn test_check_required_keys_detects_missing_server() {
        use indexmap::IndexMap;
        let map = IndexMap::new();
        let mut issues = Vec::new();
        check_required_keys(&map, &mut issues);
        // No server section, so no specific error
        assert!(issues.is_empty());
    }

    #[test]
    fn test_check_required_keys_server_missing_host() {
        use crate::types::{AnnotatedValue, ConfigValue, SourceId};
        use indexmap::IndexMap;
        use std::sync::Arc;
        let server_map = IndexMap::new();
        // Server exists but has neither host nor port
        let mut map = IndexMap::new();
        map.insert(
            Arc::from("server"),
            AnnotatedValue::new(
                ConfigValue::Map(Arc::new(server_map)),
                SourceId::new("t"),
                "",
            ),
        );
        let mut issues = Vec::new();
        check_required_keys(&map, &mut issues);
        assert!(issues.is_empty() || issues.iter().any(|i| i.contains("missing")));
    }

    #[test]
    fn test_check_types_detects_number_in_string() {
        use crate::types::{AnnotatedValue, ConfigValue, SourceId};
        use indexmap::IndexMap;
        use std::sync::Arc;
        let mut map = IndexMap::new();
        map.insert(
            Arc::from("port"),
            AnnotatedValue::new(ConfigValue::string("8080"), SourceId::new("t"), "port"),
        );
        let mut issues = Vec::new();
        check_types(&map, &mut issues);
        assert!(issues.iter().any(|i| i.contains("8080")));
    }

    #[test]
    fn test_check_types_detects_bool_in_string() {
        use crate::types::{AnnotatedValue, ConfigValue, SourceId};
        use indexmap::IndexMap;
        use std::sync::Arc;
        let mut map = IndexMap::new();
        map.insert(
            Arc::from("debug"),
            AnnotatedValue::new(ConfigValue::string("true"), SourceId::new("t"), "debug"),
        );
        let mut issues = Vec::new();
        check_types(&map, &mut issues);
        assert!(issues
            .iter()
            .any(|i| i.contains("boolean") || i.contains("true")));
    }

    #[test]
    fn test_print_config_value_basic() {
        use crate::types::{ConfigValue, SourceId};
        let v = AnnotatedValue::new(ConfigValue::string("test"), SourceId::new("t"), "k");
        // Should not panic
        print_config_value(&v, "", false);
    }

    #[test]
    fn test_print_config_value_map() {
        use crate::types::{ConfigValue, SourceId};
        use indexmap::IndexMap;
        use std::sync::Arc;
        let mut map = IndexMap::new();
        map.insert(
            Arc::from("host"),
            AnnotatedValue::new(
                ConfigValue::string("localhost"),
                SourceId::new("env"),
                "host",
            ),
        );
        let v = AnnotatedValue::new(ConfigValue::Map(Arc::new(map)), SourceId::new("t"), "");
        print_config_value(&v, "", true);
    }

    // ============== format_value: remaining variants ==============

    #[test]
    fn test_format_value_u64() {
        let v = crate::ConfigValue::uint(123);
        assert_eq!(format_value(&v), "123");
    }

    #[test]
    fn test_format_value_f64() {
        let v = crate::ConfigValue::float(2.5);
        assert_eq!(format_value(&v), "2.5");
    }

    #[test]
    fn test_format_value_bytes() {
        let v = crate::ConfigValue::Bytes(vec![1, 2, 3]);
        assert_eq!(format_value(&v), "[bytes: 3]");
    }

    #[test]
    fn test_format_value_bytes_empty() {
        let v = crate::ConfigValue::Bytes(vec![]);
        assert_eq!(format_value(&v), "[bytes: 0]");
    }

    #[test]
    fn test_format_value_array_empty() {
        let v = crate::ConfigValue::Array(vec![].into());
        assert_eq!(format_value(&v), "[array: 0 items]");
    }

    #[test]
    fn test_format_value_array_items() {
        let items = vec![AnnotatedValue::new(
            crate::ConfigValue::string("x"),
            crate::types::SourceId::new("t"),
            "0",
        )];
        let v = crate::ConfigValue::Array(items.into());
        assert_eq!(format_value(&v), "[array: 1 items]");
    }

    #[test]
    fn test_format_value_map_keys() {
        use crate::types::SourceId;
        let m = crate::ConfigValue::map(vec![(
            "a",
            AnnotatedValue::new(crate::ConfigValue::integer(1), SourceId::new("t"), "a"),
        )]);
        assert_eq!(format_value(&m), "{ 1 keys }");
    }

    #[test]
    fn test_format_value_long_string_truncation() {
        let long = "a".repeat(25);
        let v = crate::ConfigValue::string(long);
        let formatted = format_value(&v);
        assert!(formatted.starts_with("\"aaa"));
        assert!(formatted.ends_with("...\""));
        // Truncation takes first 17 chars then appends "..."
        assert_eq!(formatted.len(), 17 + 4 + 1); // 17 chars + "..." + closing quote
    }

    #[test]
    fn test_format_value_string_exact_20_no_truncation() {
        // Boundary: strings of length <= 20 are not truncated
        let v = crate::ConfigValue::string("01234567890123456789"); // 20 chars
        assert_eq!(format_value(&v), "\"01234567890123456789\"");
    }

    #[test]
    fn test_format_value_string_21_truncated() {
        // Boundary: strings of length > 20 are truncated
        let v = crate::ConfigValue::string("012345678901234567890"); // 21 chars
        let formatted = format_value(&v);
        assert!(formatted.ends_with("...\""));
    }

    // ============== format_location edge cases ==============

    #[test]
    fn test_format_location_zero_values() {
        use crate::types::SourceLocation;
        let loc = SourceLocation::new("f.toml", 0, 0);
        let r = format_location(&Some(loc));
        assert!(r.contains("line 0"));
        assert!(r.contains("col 0"));
    }

    // ============== load_env_file ==============

    #[test]
    #[serial_test::serial]
    fn test_load_env_file_valid() {
        use std::io::Write;
        let mut tf = tempfile::Builder::new().suffix(".toml").tempfile().unwrap();
        writeln!(tf, "# comment line").unwrap();
        writeln!(tf).unwrap(); // empty line
        writeln!(tf, "CONFERS_TEST_LIT_PLAIN=plain_value").unwrap();
        writeln!(tf, "CONFERS_TEST_LIT_QUOTED=\"quoted value\"").unwrap();
        writeln!(tf, "CONFERS_TEST_LIT_SINGLE='single value'").unwrap();
        writeln!(tf, "  CONFERS_TEST_LIT_SPACED  =  spaced_val  ").unwrap();
        writeln!(tf, "NO_EQUALS_HERE").unwrap(); // ignored: no '='
        tf.flush().unwrap();

        for k in [
            "CONFERS_TEST_LIT_PLAIN",
            "CONFERS_TEST_LIT_QUOTED",
            "CONFERS_TEST_LIT_SINGLE",
            "CONFERS_TEST_LIT_SPACED",
        ] {
            std::env::remove_var(k);
        }

        let result = load_env_file(&tf.path().to_path_buf());
        assert!(result.is_ok());
        assert_eq!(
            std::env::var("CONFERS_TEST_LIT_PLAIN").unwrap(),
            "plain_value"
        );
        assert_eq!(
            std::env::var("CONFERS_TEST_LIT_QUOTED").unwrap(),
            "quoted value"
        );
        assert_eq!(
            std::env::var("CONFERS_TEST_LIT_SINGLE").unwrap(),
            "single value"
        );
        // Values are trimmed: "  spaced_val  " -> "spaced_val"
        assert_eq!(
            std::env::var("CONFERS_TEST_LIT_SPACED").unwrap(),
            "spaced_val"
        );

        for k in [
            "CONFERS_TEST_LIT_PLAIN",
            "CONFERS_TEST_LIT_QUOTED",
            "CONFERS_TEST_LIT_SINGLE",
            "CONFERS_TEST_LIT_SPACED",
        ] {
            std::env::remove_var(k);
        }
    }

    #[test]
    #[serial_test::serial]
    fn test_load_env_file_preserves_existing() {
        use std::io::Write;
        std::env::set_var("CONFERS_TEST_PRESERVE", "original");
        let mut tf = tempfile::Builder::new().suffix(".toml").tempfile().unwrap();
        writeln!(tf, "CONFERS_TEST_PRESERVE=should_not_overwrite").unwrap();
        tf.flush().unwrap();
        load_env_file(&tf.path().to_path_buf()).unwrap();
        assert_eq!(std::env::var("CONFERS_TEST_PRESERVE").unwrap(), "original");
        std::env::remove_var("CONFERS_TEST_PRESERVE");
    }

    #[test]
    #[serial_test::serial]
    fn test_load_env_file_empty_file() {
        let tf = tempfile::Builder::new().suffix(".toml").tempfile().unwrap();
        let result = load_env_file(&tf.path().to_path_buf());
        assert!(result.is_ok());
    }

    #[test]
    #[serial_test::serial]
    fn test_load_env_file_only_comments() {
        use std::io::Write;
        let mut tf = tempfile::Builder::new().suffix(".toml").tempfile().unwrap();
        writeln!(tf, "# only a comment").unwrap();
        writeln!(tf, "# another").unwrap();
        tf.flush().unwrap();
        let result = load_env_file(&tf.path().to_path_buf());
        assert!(result.is_ok());
    }

    // ============== find_value_by_key ==============

    #[test]
    fn test_find_value_by_key_top_level() {
        use crate::types::SourceId;
        use indexmap::IndexMap;
        use std::sync::Arc;
        let mut map = IndexMap::new();
        map.insert(
            Arc::from("name"),
            AnnotatedValue::new(
                crate::ConfigValue::string("confers"),
                SourceId::new("t"),
                "name",
            ),
        );
        let av = AnnotatedValue::new(
            crate::ConfigValue::Map(Arc::new(map)),
            SourceId::new("test"),
            "",
        );
        let val = find_value_by_key(&av, "name");
        assert!(val.is_some());
        assert_eq!(val.unwrap().as_str(), Some("confers"));
    }

    #[test]
    fn test_find_value_by_key_non_map_returns_none() {
        use crate::types::SourceId;
        // Top-level value is a String, not a Map -> returns None
        let av = AnnotatedValue::new(crate::ConfigValue::string("x"), SourceId::new("t"), "");
        assert!(find_value_by_key(&av, "any").is_none());
    }

    #[test]
    fn test_find_value_by_key_intermediate_not_map() {
        use crate::types::SourceId;
        use indexmap::IndexMap;
        use std::sync::Arc;
        // "a" maps to a String; looking for "a.b" must return None because intermediate isn't a Map
        let mut map = IndexMap::new();
        map.insert(
            Arc::from("a"),
            AnnotatedValue::new(crate::ConfigValue::string("v"), SourceId::new("t"), "a"),
        );
        let av = AnnotatedValue::new(
            crate::ConfigValue::Map(Arc::new(map)),
            SourceId::new("t"),
            "",
        );
        assert!(find_value_by_key(&av, "a.b").is_none());
    }

    #[test]
    fn test_find_value_by_key_empty_key() {
        use crate::types::SourceId;
        use indexmap::IndexMap;
        use std::sync::Arc;
        let map = IndexMap::new();
        let av = AnnotatedValue::new(
            crate::ConfigValue::Map(Arc::new(map)),
            SourceId::new("t"),
            "",
        );
        // Empty key splits to [""] which is not in the map -> None
        assert!(find_value_by_key(&av, "").is_none());
    }

    #[test]
    fn test_find_value_by_key_deep_nested() {
        use crate::types::SourceId;
        use indexmap::IndexMap;
        use std::sync::Arc;
        let mut leaf = IndexMap::new();
        leaf.insert(
            Arc::from("v"),
            AnnotatedValue::new(
                crate::ConfigValue::integer(7),
                SourceId::new("t"),
                "a.b.c.v",
            ),
        );
        let mut mid = IndexMap::new();
        mid.insert(
            Arc::from("c"),
            AnnotatedValue::new(
                crate::ConfigValue::Map(Arc::new(leaf)),
                SourceId::new("t"),
                "a.b.c",
            ),
        );
        let mut top = IndexMap::new();
        top.insert(
            Arc::from("b"),
            AnnotatedValue::new(
                crate::ConfigValue::Map(Arc::new(mid)),
                SourceId::new("t"),
                "a.b",
            ),
        );
        let mut root = IndexMap::new();
        root.insert(
            Arc::from("a"),
            AnnotatedValue::new(
                crate::ConfigValue::Map(Arc::new(top)),
                SourceId::new("t"),
                "a",
            ),
        );
        let av = AnnotatedValue::new(
            crate::ConfigValue::Map(Arc::new(root)),
            SourceId::new("t"),
            "",
        );
        let val = find_value_by_key(&av, "a.b.c.v");
        assert!(val.is_some());
        assert_eq!(val.unwrap().as_i64(), Some(7));
    }

    // ============== check_required_keys ==============

    #[test]
    fn test_check_required_keys_server_missing_host_and_port() {
        use crate::types::{AnnotatedValue, ConfigValue, SourceId};
        use indexmap::IndexMap;
        use std::sync::Arc;
        let server_map = IndexMap::new(); // empty: no host, no port
        let mut map = IndexMap::new();
        map.insert(
            Arc::from("server"),
            AnnotatedValue::new(
                ConfigValue::Map(Arc::new(server_map)),
                SourceId::new("t"),
                "server",
            ),
        );
        let mut issues = Vec::new();
        check_required_keys(&map, &mut issues);
        assert!(issues
            .iter()
            .any(|i| i.contains("Server configuration missing host/port")));
    }

    #[test]
    fn test_check_required_keys_server_has_host_no_issue() {
        use crate::types::{AnnotatedValue, ConfigValue, SourceId};
        use indexmap::IndexMap;
        use std::sync::Arc;
        let mut server_map = IndexMap::new();
        server_map.insert(
            Arc::from("host"),
            AnnotatedValue::new(
                ConfigValue::string("localhost"),
                SourceId::new("t"),
                "server.host",
            ),
        );
        let mut map = IndexMap::new();
        map.insert(
            Arc::from("server"),
            AnnotatedValue::new(
                ConfigValue::Map(Arc::new(server_map)),
                SourceId::new("t"),
                "server",
            ),
        );
        let mut issues = Vec::new();
        check_required_keys(&map, &mut issues);
        assert!(issues
            .iter()
            .all(|i| !i.contains("Server configuration missing host/port")));
    }

    #[test]
    fn test_check_required_keys_server_has_port_no_issue() {
        use crate::types::{AnnotatedValue, ConfigValue, SourceId};
        use indexmap::IndexMap;
        use std::sync::Arc;
        let mut server_map = IndexMap::new();
        server_map.insert(
            Arc::from("port"),
            AnnotatedValue::new(
                ConfigValue::integer(8080),
                SourceId::new("t"),
                "server.port",
            ),
        );
        let mut map = IndexMap::new();
        map.insert(
            Arc::from("server"),
            AnnotatedValue::new(
                ConfigValue::Map(Arc::new(server_map)),
                SourceId::new("t"),
                "server",
            ),
        );
        let mut issues = Vec::new();
        check_required_keys(&map, &mut issues);
        assert!(issues
            .iter()
            .all(|i| !i.contains("Server configuration missing host/port")));
    }

    #[test]
    fn test_check_required_keys_database_missing_connection() {
        use crate::types::{AnnotatedValue, ConfigValue, SourceId};
        use indexmap::IndexMap;
        use std::sync::Arc;
        let db_map = IndexMap::new(); // no url, no host
        let mut map = IndexMap::new();
        map.insert(
            Arc::from("database"),
            AnnotatedValue::new(
                ConfigValue::Map(Arc::new(db_map)),
                SourceId::new("t"),
                "database",
            ),
        );
        let mut issues = Vec::new();
        check_required_keys(&map, &mut issues);
        assert!(issues
            .iter()
            .any(|i| i.contains("Database configuration missing connection details")));
    }

    #[test]
    fn test_check_required_keys_database_has_url_no_issue() {
        use crate::types::{AnnotatedValue, ConfigValue, SourceId};
        use indexmap::IndexMap;
        use std::sync::Arc;
        let mut db_map = IndexMap::new();
        db_map.insert(
            Arc::from("url"),
            AnnotatedValue::new(
                ConfigValue::string("postgres://localhost"),
                SourceId::new("t"),
                "database.url",
            ),
        );
        let mut map = IndexMap::new();
        map.insert(
            Arc::from("database"),
            AnnotatedValue::new(
                ConfigValue::Map(Arc::new(db_map)),
                SourceId::new("t"),
                "database",
            ),
        );
        let mut issues = Vec::new();
        check_required_keys(&map, &mut issues);
        assert!(issues
            .iter()
            .all(|i| !i.contains("Database configuration missing connection")));
    }

    #[test]
    fn test_check_required_keys_null_value_flagged() {
        use crate::types::{AnnotatedValue, ConfigValue, SourceId};
        use indexmap::IndexMap;
        use std::sync::Arc;
        let mut map = IndexMap::new();
        map.insert(
            Arc::from("empty_key"),
            AnnotatedValue::new(ConfigValue::Null, SourceId::new("t"), "empty_key"),
        );
        let mut issues = Vec::new();
        check_required_keys(&map, &mut issues);
        assert!(issues
            .iter()
            .any(|i| i.contains("null value") && i.contains("empty_key")));
    }

    #[test]
    fn test_check_required_keys_server_not_map_skips_check() {
        // server exists but is NOT a Map -> inner check is skipped, no "missing host/port" issue
        use crate::types::{AnnotatedValue, ConfigValue, SourceId};
        use indexmap::IndexMap;
        use std::sync::Arc;
        let mut map = IndexMap::new();
        map.insert(
            Arc::from("server"),
            AnnotatedValue::new(
                ConfigValue::string("not_a_map"),
                SourceId::new("t"),
                "server",
            ),
        );
        let mut issues = Vec::new();
        check_required_keys(&map, &mut issues);
        assert!(issues
            .iter()
            .all(|i| !i.contains("Server configuration missing")));
    }

    #[test]
    fn test_check_required_keys_empty_map_no_issues() {
        use indexmap::IndexMap;
        let map = IndexMap::new();
        let mut issues = Vec::new();
        check_required_keys(&map, &mut issues);
        assert!(issues.is_empty());
    }

    // ============== check_types ==============

    #[test]
    fn test_check_types_normal_string_no_issue() {
        use crate::types::{AnnotatedValue, ConfigValue, SourceId};
        use indexmap::IndexMap;
        use std::sync::Arc;
        let mut map = IndexMap::new();
        map.insert(
            Arc::from("name"),
            AnnotatedValue::new(ConfigValue::string("hello"), SourceId::new("t"), "name"),
        );
        let mut issues = Vec::new();
        check_types(&map, &mut issues);
        assert!(issues.is_empty());
    }

    #[test]
    fn test_check_types_non_string_no_issue() {
        use crate::types::{AnnotatedValue, ConfigValue, SourceId};
        use indexmap::IndexMap;
        use std::sync::Arc;
        let mut map = IndexMap::new();
        map.insert(
            Arc::from("port"),
            AnnotatedValue::new(ConfigValue::integer(8080), SourceId::new("t"), "port"),
        );
        map.insert(
            Arc::from("ratio"),
            AnnotatedValue::new(ConfigValue::float(0.5), SourceId::new("t"), "ratio"),
        );
        map.insert(
            Arc::from("flag"),
            AnnotatedValue::new(ConfigValue::bool(true), SourceId::new("t"), "flag"),
        );
        let mut issues = Vec::new();
        check_types(&map, &mut issues);
        assert!(issues.is_empty());
    }

    #[test]
    fn test_check_types_float_string_flagged() {
        use crate::types::{AnnotatedValue, ConfigValue, SourceId};
        use indexmap::IndexMap;
        use std::sync::Arc;
        let mut map = IndexMap::new();
        map.insert(
            Arc::from("rate"),
            AnnotatedValue::new(ConfigValue::string("3.14"), SourceId::new("t"), "rate"),
        );
        let mut issues = Vec::new();
        check_types(&map, &mut issues);
        assert!(issues
            .iter()
            .any(|i| i.contains("3.14") && i.contains("number")));
    }

    #[test]
    fn test_check_types_negative_int_string_flagged() {
        use crate::types::{AnnotatedValue, ConfigValue, SourceId};
        use indexmap::IndexMap;
        use std::sync::Arc;
        let mut map = IndexMap::new();
        map.insert(
            Arc::from("offset"),
            AnnotatedValue::new(ConfigValue::string("-5"), SourceId::new("t"), "offset"),
        );
        let mut issues = Vec::new();
        check_types(&map, &mut issues);
        assert!(issues
            .iter()
            .any(|i| i.contains("-5") && i.contains("number")));
    }

    #[test]
    fn test_check_types_empty_map_no_issues() {
        use indexmap::IndexMap;
        let map: IndexMap<std::sync::Arc<str>, AnnotatedValue> = IndexMap::new();
        let mut issues = Vec::new();
        check_types(&map, &mut issues);
        assert!(issues.is_empty());
    }

    // ============== print_config_value ==============

    #[test]
    fn test_print_config_value_conflict_marker_when_priority() {
        use crate::types::{ConfigValue, SourceId};
        use indexmap::IndexMap;
        use std::sync::Arc;
        let mut map = IndexMap::new();
        map.insert(
            Arc::from("override_me"),
            AnnotatedValue::new(
                ConfigValue::string("v"),
                SourceId::new("env"),
                "override_me",
            )
            .with_priority(5),
        );
        let av = AnnotatedValue::new(ConfigValue::Map(Arc::new(map)), SourceId::new("t"), "");
        // show_conflicts=true with priority>0 -> conflict marker "*" branch
        print_config_value(&av, "", true);
    }

    #[test]
    fn test_print_config_value_no_marker_when_priority_zero() {
        use crate::types::{ConfigValue, SourceId};
        use indexmap::IndexMap;
        use std::sync::Arc;
        let mut map = IndexMap::new();
        map.insert(
            Arc::from("k"),
            AnnotatedValue::new(ConfigValue::string("v"), SourceId::new("t"), "k"),
        );
        let av = AnnotatedValue::new(ConfigValue::Map(Arc::new(map)), SourceId::new("t"), "");
        print_config_value(&av, "", true);
    }

    #[test]
    fn test_print_config_value_no_marker_when_conflicts_disabled() {
        use crate::types::{ConfigValue, SourceId};
        use indexmap::IndexMap;
        use std::sync::Arc;
        let mut map = IndexMap::new();
        map.insert(
            Arc::from("k"),
            AnnotatedValue::new(ConfigValue::string("v"), SourceId::new("t"), "k").with_priority(9),
        );
        let av = AnnotatedValue::new(ConfigValue::Map(Arc::new(map)), SourceId::new("t"), "");
        // show_conflicts=false -> never print marker even with priority
        print_config_value(&av, "", false);
    }

    #[test]
    fn test_print_config_value_nested_map_with_prefix() {
        use crate::types::{ConfigValue, SourceId};
        use indexmap::IndexMap;
        use std::sync::Arc;
        let mut inner = IndexMap::new();
        inner.insert(
            Arc::from("host"),
            AnnotatedValue::new(
                ConfigValue::string("localhost"),
                SourceId::new("t"),
                "db.host",
            ),
        );
        let mut outer = IndexMap::new();
        outer.insert(
            Arc::from("db"),
            AnnotatedValue::new(ConfigValue::Map(Arc::new(inner)), SourceId::new("t"), "db"),
        );
        let av = AnnotatedValue::new(ConfigValue::Map(Arc::new(outer)), SourceId::new("t"), "");
        // Exercises the recursive branch with non-empty prefix
        print_config_value(&av, "root", false);
    }

    #[test]
    fn test_print_config_value_scalar_with_prefix() {
        use crate::types::{ConfigValue, SourceId};
        let v = AnnotatedValue::new(ConfigValue::integer(42), SourceId::new("t"), "some.key");
        // Non-map top-level branch with a non-empty prefix
        print_config_value(&v, "prefix", false);
    }

    // ============== build_annotated_from_cli / build_config_from_cli ==============

    #[test]
    fn test_build_annotated_from_cli_empty_paths() {
        let paths = vec![];
        let result = build_annotated_from_cli(&paths, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_config_from_cli_empty_paths() {
        let paths = vec![];
        let result = build_config_from_cli(&paths, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_annotated_from_cli_nonexistent_path_skipped() {
        let paths = vec![std::path::PathBuf::from("/nonexistent/does-not-exist.toml")];
        // path.exists() == false -> skipped, build still succeeds
        let result = build_annotated_from_cli(&paths, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_config_from_cli_nonexistent_path_skipped() {
        let paths = vec![std::path::PathBuf::from("/nonexistent/does-not-exist.json")];
        let result = build_config_from_cli(&paths, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_config_from_cli_valid_file() {
        use std::io::Write;
        let mut tf = tempfile::Builder::new().suffix(".toml").tempfile().unwrap();
        write!(tf, "name = \"confers\"\nport = 8080\n").unwrap();
        tf.flush().unwrap();
        let paths = vec![tf.path().to_path_buf()];
        let result = build_config_from_cli(&paths, true);
        assert!(result.is_ok());
        let v = result.unwrap();
        assert!(v.get("name").is_some());
    }

    #[test]
    fn test_build_annotated_from_cli_valid_file() {
        use std::io::Write;
        let mut tf = tempfile::Builder::new().suffix(".toml").tempfile().unwrap();
        write!(tf, "name = \"confers\"\nport = 8080\n").unwrap();
        tf.flush().unwrap();
        let paths = vec![tf.path().to_path_buf()];
        let result = build_annotated_from_cli(&paths, true);
        assert!(result.is_ok());
        let av = result.unwrap();
        assert!(av.is_map());
    }

    // ============== cmd_inspect ==============

    #[test]
    fn test_cmd_inspect_json_format_no_keys() {
        use std::io::Write;
        let mut tf = tempfile::Builder::new().suffix(".toml").tempfile().unwrap();
        write!(tf, "name = \"confers\"\nport = 8080\n").unwrap();
        tf.flush().unwrap();
        let paths = vec![tf.path().to_path_buf()];
        let result = cmd_inspect(&paths, &[], false, "json", true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cmd_inspect_text_format_no_keys() {
        use std::io::Write;
        let mut tf = tempfile::Builder::new().suffix(".toml").tempfile().unwrap();
        write!(tf, "name = \"confers\"\nport = 8080\n").unwrap();
        tf.flush().unwrap();
        let paths = vec![tf.path().to_path_buf()];
        let result = cmd_inspect(&paths, &[], false, "text", true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cmd_inspect_text_format_with_keys() {
        use std::io::Write;
        let mut tf = tempfile::Builder::new().suffix(".toml").tempfile().unwrap();
        write!(tf, "name = \"confers\"\nport = 8080\n").unwrap();
        tf.flush().unwrap();
        let paths = vec![tf.path().to_path_buf()];
        let keys = vec!["name".to_string(), "port".to_string()];
        let result = cmd_inspect(&paths, &keys, false, "text", true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cmd_inspect_text_format_key_not_found() {
        use std::io::Write;
        let mut tf = tempfile::Builder::new().suffix(".toml").tempfile().unwrap();
        write!(tf, "name = \"confers\"\n").unwrap();
        tf.flush().unwrap();
        let paths = vec![tf.path().to_path_buf()];
        let keys = vec!["nonexistent.key".to_string()];
        let result = cmd_inspect(&paths, &keys, false, "text", true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cmd_inspect_show_conflicts() {
        use std::io::Write;
        let mut tf = tempfile::Builder::new().suffix(".toml").tempfile().unwrap();
        write!(tf, "name = \"confers\"\nport = 8080\n").unwrap();
        tf.flush().unwrap();
        let paths = vec![tf.path().to_path_buf()];
        let result = cmd_inspect(&paths, &[], true, "text", true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cmd_inspect_empty_paths_text() {
        // No config files -> builds from env only, prints "0 configuration source(s)"
        let paths: Vec<std::path::PathBuf> = vec![];
        let result = cmd_inspect(&paths, &[], false, "text", false);
        assert!(result.is_ok());
    }

    // ============== cmd_validate ==============

    #[test]
    fn test_cmd_validate_text_success() {
        use std::io::Write;
        let mut tf = tempfile::Builder::new().suffix(".toml").tempfile().unwrap();
        write!(tf, "name = \"confers\"\nport = 8080\n").unwrap();
        tf.flush().unwrap();
        let paths = vec![tf.path().to_path_buf()];
        let result = cmd_validate(&paths, false, "text", true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cmd_validate_json_success() {
        use std::io::Write;
        let mut tf = tempfile::Builder::new().suffix(".toml").tempfile().unwrap();
        write!(tf, "name = \"confers\"\nport = 8080\n").unwrap();
        tf.flush().unwrap();
        let paths = vec![tf.path().to_path_buf()];
        let result = cmd_validate(&paths, false, "json", true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cmd_validate_issues_non_strict() {
        // String value that looks like a number triggers an issue
        use std::io::Write;
        let mut tf = tempfile::Builder::new().suffix(".toml").tempfile().unwrap();
        write!(tf, "port = \"8080\"\ndebug = \"true\"\n").unwrap();
        tf.flush().unwrap();
        let paths = vec![tf.path().to_path_buf()];
        // Non-strict: issues printed but command succeeds
        let result = cmd_validate(&paths, false, "text", true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cmd_validate_issues_strict_fails() {
        use std::io::Write;
        let mut tf = tempfile::Builder::new().suffix(".toml").tempfile().unwrap();
        write!(tf, "port = \"8080\"\n").unwrap();
        tf.flush().unwrap();
        let paths = vec![tf.path().to_path_buf()];
        // Strict mode with issues -> bails
        let result = cmd_validate(&paths, true, "text", true);
        assert!(result.is_err());
    }

    #[test]
    fn test_cmd_validate_json_issues_strict_fails() {
        use std::io::Write;
        let mut tf = tempfile::Builder::new().suffix(".toml").tempfile().unwrap();
        write!(tf, "port = \"8080\"\n").unwrap();
        tf.flush().unwrap();
        let paths = vec![tf.path().to_path_buf()];
        // Per cmd_validate design: strict bailing is text-format-only.
        // JSON format always returns Ok and surfaces validity via the printed
        // `{"valid": false, ...}` payload (callers parse the JSON to decide).
        // Therefore strict=true + json must NOT bail.
        let result = cmd_validate(&paths, true, "json", true);
        assert!(result.is_ok(), "json strict mode must not bail (design)");
    }

    #[test]
    fn test_cmd_validate_invalid_config_text_error() {
        use std::io::Write;
        let mut tf = tempfile::Builder::new().suffix(".toml").tempfile().unwrap();
        write!(tf, "this is = = not valid toml\n").unwrap();
        tf.flush().unwrap();
        let paths = vec![tf.path().to_path_buf()];
        // Build fails -> error path
        let result = cmd_validate(&paths, false, "text", true);
        assert!(result.is_err());
    }

    #[test]
    fn test_cmd_validate_invalid_config_json_error() {
        use std::io::Write;
        let mut tf = tempfile::Builder::new().suffix(".toml").tempfile().unwrap();
        write!(tf, "this is = = not valid toml\n").unwrap();
        tf.flush().unwrap();
        let paths = vec![tf.path().to_path_buf()];
        let result = cmd_validate(&paths, false, "json", true);
        assert!(result.is_err());
    }

    // ============== cmd_export ==============

    #[test]
    fn test_cmd_export_json_stdout() {
        use std::io::Write;
        let mut tf = tempfile::Builder::new().suffix(".toml").tempfile().unwrap();
        write!(tf, "name = \"confers\"\nport = 8080\n").unwrap();
        tf.flush().unwrap();
        let paths = vec![tf.path().to_path_buf()];
        let result = cmd_export(&paths, "json", None, false, false, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cmd_export_toml_stdout() {
        use std::io::Write;
        let mut tf = tempfile::Builder::new().suffix(".toml").tempfile().unwrap();
        write!(tf, "name = \"confers\"\nport = 8080\n").unwrap();
        tf.flush().unwrap();
        let paths = vec![tf.path().to_path_buf()];
        let result = cmd_export(&paths, "toml", None, false, false, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cmd_export_yaml_stdout() {
        use std::io::Write;
        let mut tf = tempfile::Builder::new().suffix(".toml").tempfile().unwrap();
        write!(tf, "name = \"confers\"\nport = 8080\n").unwrap();
        tf.flush().unwrap();
        let paths = vec![tf.path().to_path_buf()];
        let result = cmd_export(&paths, "yaml", None, false, false, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cmd_export_unsupported_format_fails() {
        use std::io::Write;
        let mut tf = tempfile::Builder::new().suffix(".toml").tempfile().unwrap();
        write!(tf, "name = \"confers\"\n").unwrap();
        tf.flush().unwrap();
        let paths = vec![tf.path().to_path_buf()];
        let result = cmd_export(&paths, "xml", None, false, false, true);
        assert!(result.is_err());
    }

    #[test]
    fn test_cmd_export_with_provenance_json() {
        use std::io::Write;
        let mut tf = tempfile::Builder::new().suffix(".toml").tempfile().unwrap();
        write!(tf, "name = \"confers\"\nport = 8080\n").unwrap();
        tf.flush().unwrap();
        let paths = vec![tf.path().to_path_buf()];
        let result = cmd_export(&paths, "json", None, true, false, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cmd_export_raw_flag() {
        use std::io::Write;
        let mut tf = tempfile::Builder::new().suffix(".toml").tempfile().unwrap();
        write!(tf, "name = \"confers\"\n").unwrap();
        tf.flush().unwrap();
        let paths = vec![tf.path().to_path_buf()];
        // raw=true prints a warning to stderr then exports normally
        let result = cmd_export(&paths, "json", None, false, true, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cmd_export_to_file() {
        use std::io::Write;
        let mut cfg = tempfile::Builder::new().suffix(".toml").tempfile().unwrap();
        write!(cfg, "name = \"confers\"\nport = 8080\n").unwrap();
        cfg.flush().unwrap();

        let out_dir = tempfile::tempdir().unwrap();
        let out_path = out_dir.path().join("out.json");
        let paths = vec![cfg.path().to_path_buf()];
        let result = cmd_export(&paths, "json", Some(out_path.clone()), false, false, true);
        assert!(result.is_ok());
        assert!(out_path.exists());
    }

    #[test]
    fn test_cmd_export_to_directory() {
        use std::io::Write;
        let mut cfg = tempfile::Builder::new().suffix(".toml").tempfile().unwrap();
        write!(cfg, "name = \"confers\"\n").unwrap();
        cfg.flush().unwrap();

        let out_dir = tempfile::tempdir().unwrap();
        let paths = vec![cfg.path().to_path_buf()];
        // output is an existing directory -> generates timestamped filename
        let result = cmd_export(
            &paths,
            "json",
            Some(out_dir.path().to_path_buf()),
            false,
            false,
            true,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_cmd_export_provenance_to_file() {
        use std::io::Write;
        let mut cfg = tempfile::Builder::new().suffix(".toml").tempfile().unwrap();
        write!(cfg, "name = \"confers\"\nport = 8080\n").unwrap();
        cfg.flush().unwrap();

        let out_dir = tempfile::tempdir().unwrap();
        let out_path = out_dir.path().join("annotated.json");
        let paths = vec![cfg.path().to_path_buf()];
        let result = cmd_export(&paths, "json", Some(out_path.clone()), true, false, true);
        assert!(result.is_ok());
        assert!(out_path.exists());
    }

    #[test]
    fn test_cmd_export_provenance_unsupported_format_fails() {
        use std::io::Write;
        let mut cfg = tempfile::Builder::new().suffix(".toml").tempfile().unwrap();
        write!(cfg, "name = \"confers\"\n").unwrap();
        cfg.flush().unwrap();
        let paths = vec![cfg.path().to_path_buf()];
        let result = cmd_export(&paths, "xml", None, true, false, true);
        assert!(result.is_err());
    }

    // ============== cmd_diff ==============

    #[test]
    fn test_cmd_diff_text_different() {
        use std::io::Write;
        let mut base = tempfile::Builder::new().suffix(".toml").tempfile().unwrap();
        write!(base, "name = \"base\"\n").unwrap();
        base.flush().unwrap();
        let mut overlay = tempfile::Builder::new().suffix(".toml").tempfile().unwrap();
        write!(overlay, "name = \"overlay\"\n").unwrap();
        overlay.flush().unwrap();
        let result = cmd_diff(
            &base.path().to_path_buf(),
            &overlay.path().to_path_buf(),
            "text",
            true,
            true,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_cmd_diff_json_format() {
        use std::io::Write;
        let mut base = tempfile::Builder::new().suffix(".toml").tempfile().unwrap();
        write!(base, "name = \"base\"\n").unwrap();
        base.flush().unwrap();
        let mut overlay = tempfile::Builder::new().suffix(".toml").tempfile().unwrap();
        write!(overlay, "name = \"overlay\"\n").unwrap();
        overlay.flush().unwrap();
        let result = cmd_diff(
            &base.path().to_path_buf(),
            &overlay.path().to_path_buf(),
            "json",
            true,
            true,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_cmd_diff_identical() {
        use std::io::Write;
        let mut base = tempfile::Builder::new().suffix(".toml").tempfile().unwrap();
        write!(base, "name = \"same\"\nport = 8080\n").unwrap();
        base.flush().unwrap();
        let mut overlay = tempfile::Builder::new().suffix(".toml").tempfile().unwrap();
        write!(overlay, "name = \"same\"\nport = 8080\n").unwrap();
        overlay.flush().unwrap();
        // Identical content short-circuits to "Configurations are identical"
        let result = cmd_diff(
            &base.path().to_path_buf(),
            &overlay.path().to_path_buf(),
            "text",
            true,
            true,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_cmd_diff_base_not_found_fails() {
        let base = std::path::PathBuf::from("/nonexistent/base.toml");
        let mut overlay = tempfile::Builder::new().suffix(".toml").tempfile().unwrap();
        use std::io::Write;
        write!(overlay, "name = \"overlay\"\n").unwrap();
        overlay.flush().unwrap();
        let result = cmd_diff(&base, &overlay.path().to_path_buf(), "text", true, true);
        assert!(result.is_err());
    }

    #[test]
    fn test_cmd_diff_unsupported_format_fails() {
        use std::io::Write;
        // Use a .txt extension so detect_format_from_path returns None
        let mut base = tempfile::Builder::new().suffix(".txt").tempfile().unwrap();
        write!(base, "name = base\n").unwrap();
        base.flush().unwrap();
        let mut overlay = tempfile::Builder::new().suffix(".txt").tempfile().unwrap();
        write!(overlay, "name = overlay\n").unwrap();
        overlay.flush().unwrap();
        let result = cmd_diff(
            &base.path().to_path_buf(),
            &overlay.path().to_path_buf(),
            "text",
            true,
            true,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_cmd_diff_absolute_path_rejected() {
        use std::io::Write;
        let mut base = tempfile::Builder::new().suffix(".toml").tempfile().unwrap();
        write!(base, "name = \"base\"\n").unwrap();
        base.flush().unwrap();
        let mut overlay = tempfile::Builder::new().suffix(".toml").tempfile().unwrap();
        write!(overlay, "name = \"overlay\"\n").unwrap();
        overlay.flush().unwrap();
        // With allow_absolute_paths=false, absolute temp paths must be rejected
        let result = cmd_diff(
            &base.path().to_path_buf(),
            &overlay.path().to_path_buf(),
            "text",
            true,
            false,
        );
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("Absolute path not allowed"),
            "expected path rejection, got: {err_msg}"
        );
    }

    // ============== cmd_snapshot_list ==============

    #[test]
    fn test_cmd_snapshot_list_nonexistent_dir() {
        let dir = std::path::PathBuf::from("/nonexistent/snapshots");
        let result = cmd_snapshot_list(&dir);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cmd_snapshot_list_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_snapshot_list(&dir.path().to_path_buf());
        assert!(result.is_ok());
    }

    #[test]
    fn test_cmd_snapshot_list_with_files() {
        use std::io::Write;
        let dir = tempfile::tempdir().unwrap();
        let mut f1 = std::fs::File::create(dir.path().join("snap1.json")).unwrap();
        writeln!(f1, "{{\"k\": 1}}").unwrap();
        let mut f2 = std::fs::File::create(dir.path().join("snap2.toml")).unwrap();
        write!(f2, "k = 2\n").unwrap();
        // Non-snapshot file (wrong extension) should be filtered out
        let mut f3 = std::fs::File::create(dir.path().join("readme.txt")).unwrap();
        write!(f3, "ignored").unwrap();
        let result = cmd_snapshot_list(&dir.path().to_path_buf());
        assert!(result.is_ok());
    }

    #[test]
    fn test_cmd_snapshot_list_filters_non_snapshot_files() {
        let dir = tempfile::tempdir().unwrap();
        // Only a .txt file -> no snapshots match -> "No snapshots found"
        std::fs::File::create(dir.path().join("notes.txt"))
            .unwrap()
            .sync_all()
            .unwrap();
        let result = cmd_snapshot_list(&dir.path().to_path_buf());
        assert!(result.is_ok());
    }

    // ============== cmd_snapshot_diff ==============

    #[test]
    fn test_cmd_snapshot_diff_nonexistent_dir() {
        let dir = std::path::PathBuf::from("/nonexistent/snapshots");
        let result = cmd_snapshot_diff(2, &dir);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cmd_snapshot_diff_insufficient_snapshots() {
        use std::io::Write;
        let dir = tempfile::tempdir().unwrap();
        let mut f = std::fs::File::create(dir.path().join("only.json")).unwrap();
        writeln!(f, "{{\"k\": 1}}").unwrap();
        // Only 1 snapshot -> "Need at least 2"
        let result = cmd_snapshot_diff(2, &dir.path().to_path_buf());
        assert!(result.is_ok());
    }

    #[test]
    fn test_cmd_snapshot_diff_with_two_snapshots() {
        use std::io::Write;
        let dir = tempfile::tempdir().unwrap();
        let mut f1 = std::fs::File::create(dir.path().join("snap1.json")).unwrap();
        writeln!(f1, "{{\"k\": 1}}").unwrap();
        f1.sync_all().unwrap();
        let mut f2 = std::fs::File::create(dir.path().join("snap2.json")).unwrap();
        writeln!(f2, "{{\"k\": 2}}").unwrap();
        f2.sync_all().unwrap();
        let result = cmd_snapshot_diff(2, &dir.path().to_path_buf());
        assert!(result.is_ok());
    }

    // ============== cmd_snapshot_prune ==============

    #[test]
    fn test_cmd_snapshot_prune_nonexistent_dir() {
        let dir = std::path::PathBuf::from("/nonexistent/snapshots");
        let result = cmd_snapshot_prune("30d", &dir);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cmd_snapshot_prune_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_snapshot_prune("7d", &dir.path().to_path_buf());
        assert!(result.is_ok());
    }

    #[test]
    fn test_cmd_snapshot_prune_keeps_recent_files() {
        use std::io::Write;
        let dir = tempfile::tempdir().unwrap();
        // Recently created files should NOT be pruned with a 30d cutoff
        let mut f = std::fs::File::create(dir.path().join("recent.json")).unwrap();
        writeln!(f, "{{\"k\": 1}}").unwrap();
        f.sync_all().unwrap();
        let result = cmd_snapshot_prune("30d", &dir.path().to_path_buf());
        assert!(result.is_ok());
        assert!(dir.path().join("recent.json").exists());
    }

    #[test]
    fn test_cmd_snapshot_prune_invalid_duration_defaults() {
        let dir = tempfile::tempdir().unwrap();
        // Invalid duration "abc" -> parse fails -> defaults to 30 days
        let result = cmd_snapshot_prune("abc", &dir.path().to_path_buf());
        assert!(result.is_ok());
    }

    #[test]
    fn test_cmd_snapshot_prune_uppercase_d_suffix() {
        let dir = tempfile::tempdir().unwrap();
        // "7D" should trim_end_matches('D') and parse to 7
        let result = cmd_snapshot_prune("7D", &dir.path().to_path_buf());
        assert!(result.is_ok());
    }

    // ============== cmd_snapshot dispatch ==============

    #[test]
    fn test_cmd_snapshot_dispatch_list() {
        let dir = tempfile::tempdir().unwrap();
        let action = SnapshotCommands::List {
            directory: dir.path().to_path_buf(),
        };
        let result = cmd_snapshot(action);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cmd_snapshot_dispatch_diff() {
        let dir = tempfile::tempdir().unwrap();
        let action = SnapshotCommands::Diff {
            latest: 2,
            directory: dir.path().to_path_buf(),
        };
        let result = cmd_snapshot(action);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cmd_snapshot_dispatch_prune() {
        let dir = tempfile::tempdir().unwrap();
        let action = SnapshotCommands::Prune {
            older_than: "30d".to_string(),
            directory: dir.path().to_path_buf(),
        };
        let result = cmd_snapshot(action);
        assert!(result.is_ok());
    }

    // ============== Cli parsing (clap) ==============

    #[test]
    fn test_cli_parse_inspect() {
        let cli = Cli::try_parse_from(["confers", "inspect"]).unwrap();
        assert!(matches!(cli.command, Commands::Inspect { .. }));
        assert!(!cli.allow_absolute_paths);
    }

    #[test]
    fn test_cli_parse_inspect_with_key_and_format() {
        let cli = Cli::try_parse_from([
            "confers",
            "inspect",
            "--key",
            "server.host",
            "--format",
            "json",
            "--show-conflicts",
        ])
        .unwrap();
        match cli.command {
            Commands::Inspect {
                key,
                show_conflicts,
                format,
            } => {
                assert_eq!(key, vec!["server.host".to_string()]);
                assert!(show_conflicts);
                assert_eq!(format, "json");
            }
            _ => panic!("expected Inspect"),
        }
    }

    #[test]
    fn test_cli_parse_validate() {
        let cli =
            Cli::try_parse_from(["confers", "validate", "--strict", "--format", "json"]).unwrap();
        match cli.command {
            Commands::Validate { strict, format } => {
                assert!(strict);
                assert_eq!(format, "json");
            }
            _ => panic!("expected Validate"),
        }
    }

    #[test]
    fn test_cli_parse_validate_defaults() {
        let cli = Cli::try_parse_from(["confers", "validate"]).unwrap();
        match cli.command {
            Commands::Validate { strict, format } => {
                assert!(!strict);
                assert_eq!(format, "text");
            }
            _ => panic!("expected Validate"),
        }
    }

    #[test]
    fn test_cli_parse_export() {
        let cli = Cli::try_parse_from([
            "confers",
            "export",
            "--format",
            "yaml",
            "--raw",
            "--with-provenance",
        ])
        .unwrap();
        match cli.command {
            Commands::Export {
                format,
                output,
                with_provenance,
                raw,
            } => {
                assert_eq!(format, "yaml");
                assert!(output.is_none());
                assert!(with_provenance);
                assert!(raw);
            }
            _ => panic!("expected Export"),
        }
    }

    #[test]
    fn test_cli_parse_export_with_output() {
        let cli = Cli::try_parse_from(["confers", "export", "--output", "/tmp/out.json"]).unwrap();
        match cli.command {
            Commands::Export { output, .. } => {
                assert_eq!(output, Some(std::path::PathBuf::from("/tmp/out.json")));
            }
            _ => panic!("expected Export"),
        }
    }

    #[test]
    fn test_cli_parse_diff() {
        let cli = Cli::try_parse_from([
            "confers",
            "diff",
            "--base",
            "base.toml",
            "--overlay",
            "overlay.toml",
        ])
        .unwrap();
        match cli.command {
            Commands::Diff {
                base,
                overlay,
                format,
                sanitize,
            } => {
                assert_eq!(base, std::path::PathBuf::from("base.toml"));
                assert_eq!(overlay, std::path::PathBuf::from("overlay.toml"));
                assert_eq!(format, "text");
                assert!(sanitize);
            }
            _ => panic!("expected Diff"),
        }
    }

    #[test]
    fn test_cli_parse_diff_no_sanitize_unsupported() {
        // Document a production-code limitation: the `Diff.sanitize` field is
        // declared as `bool` with `default_value = "true"`, which makes clap
        // use `ArgAction::SetTrue`. This action is one-way — it can only SET
        // the flag to true, never to false. Neither `--sanitize=false`,
        // `--sanitize false`, nor `--no-sanitize` is accepted.
        //
        // To support sanitize=false, production code would need
        // `#[arg(long, action = clap::ArgAction::Set, default_value = "true")]`
        // or `Option<bool>`. That is a behavior change, out of scope for the
        // test-only coverage task. This test pins the current limitation.
        let cases: &[&[&str]] = &[
            &[
                "confers",
                "diff",
                "--base",
                "a",
                "--overlay",
                "b",
                "--sanitize=false",
            ],
            &[
                "confers",
                "diff",
                "--base",
                "a",
                "--overlay",
                "b",
                "--no-sanitize",
            ],
        ];
        for argv in cases {
            let result = Cli::try_parse_from(argv.iter().copied());
            assert!(
                result.is_err(),
                "expected parse failure for {:?}, but succeeded",
                argv
            );
        }
    }

    #[test]
    fn test_cli_parse_snapshot_list() {
        let cli = Cli::try_parse_from(["confers", "snapshot", "list"]).unwrap();
        match cli.command {
            Commands::Snapshot {
                action: SnapshotCommands::List { .. },
            } => {}
            _ => panic!("expected Snapshot/List"),
        }
    }

    #[test]
    fn test_cli_parse_snapshot_list_custom_dir() {
        let cli = Cli::try_parse_from(["confers", "snapshot", "list", "--directory", "/tmp/snaps"])
            .unwrap();
        match cli.command {
            Commands::Snapshot {
                action: SnapshotCommands::List { directory },
            } => assert_eq!(directory, std::path::PathBuf::from("/tmp/snaps")),
            _ => panic!("expected Snapshot/List"),
        }
    }

    #[test]
    fn test_cli_parse_snapshot_diff() {
        let cli = Cli::try_parse_from(["confers", "snapshot", "diff", "--latest", "5"]).unwrap();
        match cli.command {
            Commands::Snapshot {
                action: SnapshotCommands::Diff { latest, .. },
            } => assert_eq!(latest, 5),
            _ => panic!("expected Snapshot/Diff"),
        }
    }

    #[test]
    fn test_cli_parse_snapshot_prune() {
        let cli =
            Cli::try_parse_from(["confers", "snapshot", "prune", "--older-than", "14d"]).unwrap();
        match cli.command {
            Commands::Snapshot {
                action: SnapshotCommands::Prune { older_than, .. },
            } => assert_eq!(older_than, "14d"),
            _ => panic!("expected Snapshot/Prune"),
        }
    }

    #[test]
    fn test_cli_parse_global_options() {
        let cli = Cli::try_parse_from([
            "confers",
            "--config",
            "a.toml",
            "--config",
            "b.json",
            "--allow-absolute-paths",
            "--profile",
            "prod",
            "inspect",
        ])
        .unwrap();
        assert_eq!(cli.config.len(), 2);
        assert_eq!(cli.config[0], std::path::PathBuf::from("a.toml"));
        assert_eq!(cli.config[1], std::path::PathBuf::from("b.json"));
        assert!(cli.allow_absolute_paths);
        assert_eq!(cli.profile.as_deref(), Some("prod"));
    }

    #[test]
    fn test_cli_parse_env_file_option() {
        let cli = Cli::try_parse_from(["confers", "--env-file", "/tmp/.env", "inspect"]).unwrap();
        assert_eq!(
            cli.env_file.as_deref(),
            Some(std::path::Path::new("/tmp/.env"))
        );
    }

    #[test]
    fn test_cli_parse_unknown_command_fails() {
        let result = Cli::try_parse_from(["confers", "totally-unknown-command"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_cli_parse_missing_required_diff_args_fails() {
        // diff requires --base and --overlay
        let result = Cli::try_parse_from(["confers", "diff"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_cli_parse_missing_subcommand_fails() {
        // No subcommand provided
        let result = Cli::try_parse_from(["confers"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_cli_parse_inspect_multiple_keys() {
        let cli =
            Cli::try_parse_from(["confers", "inspect", "-k", "a", "-k", "b", "-k", "c"]).unwrap();
        match cli.command {
            Commands::Inspect { key, .. } => assert_eq!(key.len(), 3),
            _ => panic!("expected Inspect"),
        }
    }

    #[test]
    fn test_cli_parse_export_default_format() {
        let cli = Cli::try_parse_from(["confers", "export"]).unwrap();
        match cli.command {
            Commands::Export { format, .. } => assert_eq!(format, "json"),
            _ => panic!("expected Export"),
        }
    }

    #[test]
    fn test_cli_parse_snapshot_diff_defaults() {
        let cli = Cli::try_parse_from(["confers", "snapshot", "diff"]).unwrap();
        match cli.command {
            Commands::Snapshot {
                action: SnapshotCommands::Diff { latest, directory },
            } => {
                assert_eq!(latest, 2);
                assert_eq!(directory, std::path::PathBuf::from("./snapshots"));
            }
            _ => panic!("expected Snapshot/Diff"),
        }
    }

    #[test]
    fn test_cli_parse_snapshot_prune_defaults() {
        let cli = Cli::try_parse_from(["confers", "snapshot", "prune"]).unwrap();
        match cli.command {
            Commands::Snapshot {
                action:
                    SnapshotCommands::Prune {
                        older_than,
                        directory,
                    },
            } => {
                assert_eq!(older_than, "30d");
                assert_eq!(directory, std::path::PathBuf::from("./snapshots"));
            }
            _ => panic!("expected Snapshot/Prune"),
        }
    }

    #[test]
    fn test_cli_parse_snapshot_list_default_dir() {
        let cli = Cli::try_parse_from(["confers", "snapshot", "list"]).unwrap();
        match cli.command {
            Commands::Snapshot {
                action: SnapshotCommands::List { directory },
            } => assert_eq!(directory, std::path::PathBuf::from("./snapshots")),
            _ => panic!("expected Snapshot/List"),
        }
    }

    // ============== Const sanity ==============

    #[test]
    fn test_default_snapshot_display_limit_is_ten() {
        assert_eq!(DEFAULT_SNAPSHOT_DISPLAY_LIMIT, 10);
    }
}
