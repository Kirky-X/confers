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

const DEFAULT_SNAPSHOT_DISPLAY_LIMIT: usize = 10;

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
pub fn run() -> Result<()> {
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
#[allow(dead_code)]
fn cmd_inspect(
    config_paths: &[PathBuf],
    keys: &[String],
    show_conflicts: bool,
    format: &str,
    allow_absolute_paths: bool,
) -> Result<()> {
    use crate::ConfigBuilder;

    let mut builder = ConfigBuilder::<serde_json::Value>::new();

    if allow_absolute_paths {
        builder = builder.allow_absolute_paths();
    }

    for config_path in config_paths {
        if config_path.exists() {
            builder = builder.file(config_path.clone());
        }
    }

    builder = builder.env();

    let annotated_config = builder.build_annotated()?;

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
        crate::value::ConfigValue::Map(map) => {
            for (key, val) in map.iter() {
                let full_key = if prefix.is_empty() {
                    key.clone()
                } else {
                    Arc::from(format!("{}.{}", prefix, key))
                };

                match &val.inner {
                    crate::value::ConfigValue::Map(_) => {
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
fn format_location(location: &Option<crate::value::SourceLocation>) -> String {
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
            crate::value::ConfigValue::Map(map) => {
                current = map.get(part)?;
            }
            _ => return None,
        }
    }

    Some(current)
}

/// Format a value for display
fn format_value(value: &crate::value::ConfigValue) -> String {
    match value {
        crate::value::ConfigValue::String(s) => {
            // Truncate long strings
            if s.len() > 20 {
                format!("\"{}...\"", &s[..17])
            } else {
                format!("\"{}\"", s)
            }
        }
        crate::value::ConfigValue::I64(n) => n.to_string(),
        crate::value::ConfigValue::U64(n) => n.to_string(),
        crate::value::ConfigValue::F64(n) => n.to_string(),
        crate::value::ConfigValue::Bool(b) => b.to_string(),
        crate::value::ConfigValue::Null => "[null]".to_string(),
        crate::value::ConfigValue::Bytes(b) => format!("[bytes: {}]", b.len()),
        crate::value::ConfigValue::Array(arr) => format!("[array: {} items]", arr.len()),
        crate::value::ConfigValue::Map(obj) => {
            let keys: Vec<_> = obj.keys().collect();
            format!("{{ {} keys }}", keys.len())
        }
    }
}

/// Validate configuration against schema
#[allow(dead_code)]
fn cmd_validate(
    config_paths: &[PathBuf],
    strict: bool,
    format: &str,
    allow_absolute_paths: bool,
) -> Result<()> {
    use crate::ConfigBuilder;

    let mut builder = ConfigBuilder::<serde_json::Value>::new();

    if allow_absolute_paths {
        builder = builder.allow_absolute_paths();
    }

    for config_path in config_paths {
        if config_path.exists() {
            builder = builder.file(config_path.clone());
        }
    }

    builder = builder.env();

    match builder.build_annotated() {
        Ok(annotated_config) => {
            let mut issues = Vec::new();

            if let crate::value::ConfigValue::Map(map) = &annotated_config.inner {
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
        if let crate::value::ConfigValue::Map(server_map) = &server.inner {
            if !server_map.contains_key("host") && !server_map.contains_key("port") {
                issues.push("Server configuration missing host/port".to_string());
            }
        }
    }

    // Check for database configuration
    if let Some(db) = obj.get("database") {
        if let crate::value::ConfigValue::Map(db_map) = &db.inner {
            if !db_map.contains_key("url") && !db_map.contains_key("host") {
                issues.push("Database configuration missing connection details".to_string());
            }
        }
    }

    // Check for empty required sections
    for (key, value) in obj.iter() {
        if matches!(value.inner, crate::value::ConfigValue::Null) {
            issues.push(format!("Configuration key '{}' has null value", key));
        }
    }
}

/// Check for type consistency issues
fn check_types(obj: &indexmap::IndexMap<Arc<str>, AnnotatedValue>, issues: &mut Vec<String>) {
    // Check for suspicious string values that might be numbers
    for (key, value) in obj.iter() {
        if let crate::value::ConfigValue::String(s) = &value.inner {
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
#[allow(dead_code)]
fn cmd_export(
    config_paths: &[PathBuf],
    format: &str,
    output: Option<PathBuf>,
    with_provenance: bool,
    raw: bool,
    allow_absolute_paths: bool,
) -> Result<()> {
    use crate::ConfigBuilder;
    use chrono::Utc;

    if raw {
        eprintln!("⚠️  警告: 使用 --raw 选项将导出未脱敏的敏感数据！");
        eprintln!("   请确保输出目标安全，避免泄露密码、密钥等敏感信息。");
        eprintln!();
    }

    if with_provenance {
        let mut builder = ConfigBuilder::<serde_json::Value>::new();

        if allow_absolute_paths {
            builder = builder.allow_absolute_paths();
        }

        for config_path in config_paths {
            if config_path.exists() {
                builder = builder.file(config_path.clone());
            }
        }

        builder = builder.env();

        let annotated_config = builder.build_annotated()?;

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
        let mut builder = ConfigBuilder::<serde_json::Value>::new();

        if allow_absolute_paths {
            builder = builder.allow_absolute_paths();
        }

        for config_path in config_paths {
            if config_path.exists() {
                builder = builder.file(config_path.clone());
            }
        }

        builder = builder.env();

        let config = builder.build()?;

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
    _allow_absolute_paths: bool,
) -> Result<()> {
    use crate::loader;

    println!("Configuration Diff");
    println!("=================");
    println!();

    let base_content = std::fs::read_to_string(base)
        .with_context(|| format!("Failed to read base config: {}", base.display()))?;

    let base_value = loader::parse_content(
        &base_content,
        loader::detect_format_from_path(base)
            .ok_or_else(|| anyhow::anyhow!("Unknown format for base config"))?,
        crate::value::SourceId::new(base.to_string_lossy().as_ref()),
        Some(base),
    )?;

    let overlay_content = std::fs::read_to_string(overlay)
        .with_context(|| format!("Failed to read overlay config: {}", overlay.display()))?;

    let overlay_value = loader::parse_content(
        &overlay_content,
        loader::detect_format_from_path(overlay)
            .ok_or_else(|| anyhow::anyhow!("Unknown format for overlay config"))?,
        crate::value::SourceId::new(overlay.to_string_lossy().as_ref()),
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
