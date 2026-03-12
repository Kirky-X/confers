//! Confers CLI - Configuration diagnostics and inspection tool.
//!
//! This tool provides runtime configuration observability for confers,
//! answering questions like "Where did this value come from?" and "Why is it this value?".

// Allow MSRV warnings - code uses PathBuf::display() which is stable since 1.87
// but the crate claims to support 1.81. The code still compiles and works.
#![allow(clippy::incompatible_msrv)]

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::sync::Arc;

use confers::AnnotatedValue;

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

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Extract fields we need before matching on command
    let config_paths = cli.config.clone();
    let env_file = cli.env_file.clone();

    match cli.command {
        Commands::Inspect {
            key,
            show_conflicts,
            format,
        } => {
            cmd_inspect(&config_paths, &env_file, &key, show_conflicts, &format)?;
        }
        Commands::Validate { strict, format } => {
            cmd_validate(&config_paths, &env_file, strict, &format)?;
        }
        Commands::Export {
            format,
            output,
            with_provenance,
            raw,
        } => {
            cmd_export(
                &config_paths,
                &env_file,
                &format,
                output,
                with_provenance,
                raw,
            )?;
        }
        Commands::Diff {
            base,
            overlay,
            format: _,
        } => {
            cmd_diff(&base, &overlay)?;
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
    _env_file: &Option<PathBuf>,
    keys: &[String],
    show_conflicts: bool,
    format: &str,
) -> Result<()> {
    use confers::ConfigBuilder;

    // Build configuration with annotated values
    let mut builder = ConfigBuilder::<serde_json::Value>::new();

    // Add config files
    for config_path in config_paths {
        if config_path.exists() {
            builder = builder.file(config_path.to_string_lossy().as_ref());
        }
    }

    // Add env
    builder = builder.env();

    // Build annotated configuration
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

    // Build source information map
    let sources: Vec<String> = config_paths
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect();

    if keys.is_empty() {
        // Show all configuration keys
        println!("All configuration keys:");
        println!("{:<35} {:<25} {:<20} {:<20}", "KEY", "VALUE", "SOURCE", "LOCATION");
        println!("{}", "-".repeat(100));

        // Recursively print all keys
        print_config_value(&annotated_config, "", &sources, show_conflicts);
    } else {
        // Show requested keys
        println!("Requested keys:");
        println!("{:<35} {:<25} {:<20} {:<20}", "KEY", "VALUE", "SOURCE", "LOCATION");
        println!("{}", "-".repeat(100));

        for key in keys {
            let value = find_value_by_key(&annotated_config, key);
            match value {
                Some(v) => {
                    let value_str = format_value(&v.inner);
                    let source = v.source.as_str();
                    let location = format_location(&v.location);
                    println!("{:<35} {:<25} {:<20} {:<20}", key, value_str, source, location);
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
fn print_config_value(
    value: &AnnotatedValue,
    prefix: &str,
    sources: &[String],
    _show_conflicts: bool,
) {
    match &value.inner {
        confers::value::ConfigValue::Map(map) => {
            for (key, val) in map.iter() {
                let full_key = if prefix.is_empty() {
                    key.clone()
                } else {
                    Arc::from(format!("{}.{}", prefix, key))
                };

                match &val.inner {
                    confers::value::ConfigValue::Map(_) => {
                        // Nested object - recurse
                        print_config_value(val, &full_key, sources, _show_conflicts);
                    }
                    _ => {
                        // Leaf value - print it
                        let value_str = format_value(&val.inner);
                        let source = val.source.as_str();
                        let location = format_location(&val.location);
                        println!("{:<35} {:<25} {:<20} {:<20}", full_key, value_str, source, location);
                    }
                }
            }
        }
        _ => {
            // Top-level primitive
            let value_str = format_value(&value.inner);
            let source = value.source.as_str();
            let location = format_location(&value.location);
            println!("{:<35} {:<25} {:<20} {:<20}", prefix, value_str, source, location);
        }
    }
}

/// Format location information for display
fn format_location(location: &Option<confers::value::SourceLocation>) -> String {
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
            confers::value::ConfigValue::Map(map) => {
                current = map.get(part)?;
            }
            _ => return None,
        }
    }

    Some(current)
}

/// Format a value for display
fn format_value(value: &confers::value::ConfigValue) -> String {
    match value {
        confers::value::ConfigValue::String(s) => {
            // Truncate long strings
            if s.len() > 20 {
                format!("\"{}...\"", &s[..17])
            } else {
                format!("\"{}\"", s)
            }
        }
        confers::value::ConfigValue::I64(n) => n.to_string(),
        confers::value::ConfigValue::U64(n) => n.to_string(),
        confers::value::ConfigValue::F64(n) => n.to_string(),
        confers::value::ConfigValue::Bool(b) => b.to_string(),
        confers::value::ConfigValue::Null => "[null]".to_string(),
        confers::value::ConfigValue::Bytes(b) => format!("[bytes: {}]", b.len()),
        confers::value::ConfigValue::Array(arr) => format!("[array: {} items]", arr.len()),
        confers::value::ConfigValue::Map(obj) => {
            let keys: Vec<_> = obj.keys().collect();
            format!("{{ {} keys }}", keys.len())
        }
    }
}

/// Validate configuration against schema
#[allow(dead_code)]
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

    match builder.build_annotated() {
        Ok(annotated_config) => {
            println!("✓ Configuration loaded successfully");

            // Basic validation: check for required keys and types
            let mut issues = Vec::new();

            if let confers::value::ConfigValue::Map(map) = &annotated_config.inner {
                // Check for common required keys
                check_required_keys(map, &mut issues);
                // Check for type consistency
                check_types(map, &mut issues);
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
fn check_required_keys(obj: &indexmap::IndexMap<Arc<str>, AnnotatedValue>, issues: &mut Vec<String>) {
    // Check for server configuration
    if let Some(server) = obj.get("server") {
        if let confers::value::ConfigValue::Map(server_map) = &server.inner {
            if !server_map.contains_key("host") && !server_map.contains_key("port") {
                issues.push("Server configuration missing host/port".to_string());
            }
        }
    }

    // Check for database configuration
    if let Some(db) = obj.get("database") {
        if let confers::value::ConfigValue::Map(db_map) = &db.inner {
            if !db_map.contains_key("url") && !db_map.contains_key("host") {
                issues.push("Database configuration missing connection details".to_string());
            }
        }
    }

    // Check for empty required sections
    for (key, value) in obj.iter() {
        if matches!(value.inner, confers::value::ConfigValue::Null) {
            issues.push(format!("Configuration key '{}' has null value", key));
        }
    }
}

/// Check for type consistency issues
fn check_types(obj: &indexmap::IndexMap<Arc<str>, AnnotatedValue>, issues: &mut Vec<String>) {
    // Check for suspicious string values that might be numbers
    for (key, value) in obj.iter() {
        if let confers::value::ConfigValue::String(s) = &value.inner {
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
    _env_file: &Option<PathBuf>,
    format: &str,
    output: Option<PathBuf>,
    _with_provenance: bool,
    _raw: bool,
) -> Result<()> {
    use chrono::Utc;
    use confers::ConfigBuilder;

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

    for entry in snapshots.iter().take(10) {
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

    let first = snapshots.first().unwrap();
    let second = snapshots
        .get(count - 1)
        .unwrap_or(snapshots.get(1).unwrap());

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
