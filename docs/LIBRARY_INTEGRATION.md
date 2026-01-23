# Library Integration Guide

This guide explains how to integrate confers CLI functionality into your own Rust projects using the unified `ConfersCli` API.

## Quick Start

### 1. Add Dependency

Add confers to your `Cargo.toml` with the `cli` feature enabled:

```toml
[dependencies]
confers = { version = "0.2.0", features = ["cli"] }
```

### 2. Basic Usage

```rust
use confers::ConfersCli;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Generate a configuration template
    ConfersCli::generate(Some("config.toml"), "full")?;
    
    // Validate a configuration file
    ConfersCli::validate("config.toml", "full")?;
    
    // Compare two configurations
    ConfersCli::diff("config1.toml", "config2.toml", Some("unified"))?;
    
    // Encrypt a value (requires CONFERS_ENCRYPTION_KEY env var)
    let encrypted = ConfersCli::encrypt("secret_value", None)?;
    println!("Encrypted: {}", encrypted);
    
    Ok(())
}
```

**Note:** The CLI feature automatically includes `derive`, `validation`, and `encryption` dependencies.

## API Reference

### ConfersCli

A unified facade for all confers CLI operations.

#### Methods

##### `generate(output, level)`

Generate configuration templates.

**Parameters:**
- `output: Option<&str>` - Output file path (None prints to stdout)
- `level: &str` - Template level: "minimal", "full", or "documentation"

**Returns:**
- `Result<(), ConfigError>`

**Example:**
```rust
ConfersCli::generate(Some("app.toml"), "minimal")?;
ConfersCli::generate(None, "documentation")?; // Prints to stdout
```

**Note:** This uses `GenerateCommand::execute_placeholder()` internally.

##### `validate(config, level)`

Validate configuration files.

**Parameters:**
- `config: &str` - Path to configuration file
- `level: &str` - Validation level: "minimal", "full", or "documentation"

**Returns:**
- `Result<(), ConfigError>`

**Example:**
```rust
ConfersCli::validate("config.toml", "full")?;
```

**Note:** This uses `ValidateCommand::execute_generic()` internally.

##### `diff(file1, file2, format)`

Compare two configuration files.

**Parameters:**
- `file1: &str` - Path to first configuration file
- `file2: &str` - Path to second configuration file
- `format: Option<&str>` - Diff format: "unified", "context", "normal", "side-by-side", or "strict"

**Returns:**
- `Result<(), ConfigError>`

**Example:**
```rust
ConfersCli::diff("old.toml", "new.toml", Some("side-by-side"))?;
```

**Note:** This creates `DiffOptions` with default values and calls `DiffCommand::execute()`.

##### `encrypt(value, key)`

Encrypt configuration values.

**Parameters:**
- `value: &str` - Value to encrypt
- `key: Option<&str>` - Optional Base64-encoded 32-byte key

**Returns:**
- `Result<String, ConfigError>` - Base64-encoded encrypted value

**Example:**
```rust
let encrypted = ConfersCli::encrypt("secret_password", None)?;
let encrypted_with_key = ConfersCli::encrypt("secret", Some("base64_key_here"))?;
```

##### `wizard(non_interactive)`

Run the interactive configuration wizard.

**Parameters:**
- `non_interactive: bool` - If true, uses default values without prompting

**Example:**
```rust
ConfersCli::wizard(false)?;  // Interactive mode
ConfersCli::wizard(true)?;   // Non-interactive mode
```

##### `completions(shell)`

Generate shell completion scripts.

**Parameters:**
- `shell: &str` - Shell type: "bash", "fish", "zsh", "powershell", or "elvish"

**Example:**
```rust
ConfersCli::completions("bash")?;
ConfersCli::completions("zsh")?;
```

##### `key(subcommand)`

Execute key management operations.

**Parameters:**
- `subcommand: &KeySubcommand` - Key subcommand to execute

**Example:**
```rust
use confers::commands::key::KeySubcommand;

ConfersCli::key(&KeySubcommand::Generate)?;
```

## Advanced Usage

### Custom Configuration Types

You can use confers with your own configuration types:

```rust
use confers::{Config, ConfersCli};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Config)]
#[config(validate)]
pub struct MyAppConfig {
    pub name: String,
    pub port: u16,
    pub debug: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Generate template for your config type
    // This requires the schema feature
    #[cfg(feature = "schema")]
    {
        confers::commands::GenerateCommand::execute::<MyAppConfig>(
            Some("my_app.toml".into()),
            "full"
        )?;
    }
    
    // Validate your config type
    #[cfg(feature = "validation")]
    {
        use confers::commands::validate::{ValidateCommand, ValidateLevel};
        ValidateCommand::execute::<MyAppConfig>("my_app.toml", ValidateLevel::Full)?;
    }
    
    Ok(())
}
```

### Error Handling

All methods return `Result<(), ConfigError>` (or `Result<String, ConfigError>` for encrypt):

```rust
use confers::{ConfersCli, ConfigError};

fn handle_config_operations() -> Result<(), ConfigError> {
    match ConfersCli::validate("config.toml", "full") {
        Ok(_) => println!("Configuration is valid"),
        Err(ConfigError::FileNotFound { path }) => {
            eprintln!("Configuration file not found: {:?}", path);
        }
        Err(ConfigError::ParseError(msg)) => {
            eprintln!("Parse error: {}", msg);
        }
        Err(e) => {
            eprintln!("Validation failed: {}", e);
        }
    }
    
    Ok(())
}
```

## Feature Flags

The library integration requires the `cli` feature, but you can enable additional features:

```toml
[dependencies]
confers = { 
    version = "0.2.0", 
    features = [
        "cli",           # Required for ConfersCli
        "validation",    # For config validation
        "encryption",    # For value encryption
        "schema",       # For schema generation
        "watch",        # For file watching
        "remote",       # For remote configuration
    ]
}
```

## Examples

See the `examples/library_usage.rs` file for a complete working example that demonstrates all features.

## Migration from Direct Command Usage

If you were previously using the command modules directly:

**Before:**
```rust
use confers::commands::{GenerateCommand, ValidateCommand};

GenerateCommand::execute_placeholder(Some("config.toml".into()), "full")?;
ValidateCommand::execute_generic("config.toml", ValidateLevel::Full)?;
```

**After:**
```rust
use confers::ConfersCli;

ConfersCli::generate(Some("config.toml"), "full")?;
ConfersCli::validate("config.toml", "full")?;
```

The new API provides:
- ✅ Simpler method signatures
- ✅ Better error handling
- ✅ Consistent parameter naming
- ✅ Comprehensive documentation
- ✅ Type safety where possible

## Troubleshooting

### Feature Not Enabled

If you get "feature not enabled" errors, make sure you have the `cli` feature in your `Cargo.toml`:

```toml
[dependencies]
confers = { version = "0.2.0", features = ["cli"] }
```

### Encryption Key Issues

For encryption operations, you can either:
1. Set the `CONFERS_ENCRYPTION_KEY` environment variable
2. Provide a key directly to the `encrypt` method

```bash
# Set environment variable
export CONFERS_ENCRYPTION_KEY=$(base64 <<< "32-byte-key-here-123456789012")
```

### Validation Failures

If validation fails, use the "documentation" level to get detailed information:

```rust
ConfersCli::validate("config.toml", "documentation")?;
```

This will provide a comprehensive report of all validation checks.
