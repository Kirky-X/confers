# Confers - Modern Rust Configuration Management Library

<div align="center">
[Show Image](https://crates.io/crates/confers) [Show Image](https://docs.rs/confers) [Show Image](LICENSE) [Show Image](https://github.com/yourusername/confers/actions)
</div>
<div align="center">
**Zero Boilerplate · Type Safe · Production Ready**
</div>
<div align="center">
[Quick Start](#quick-start) · [Documentation](https://docs.rs/confers) · [Examples](#examples) · [Contributing](#contributing)
</div>



------

## ✨ Features

- 🎯 **Zero Boilerplate** - Define configurations with a single `#[derive(Config)]`
- 🔄 **Smart Merging** - Automatically merge multiple configuration sources by priority
- 🛡️ **Type Safety** - Compile-time type checking, eliminate runtime configuration errors
- 🔥 **Hot Reload** - Configuration changes take effect automatically without restart
- ✅ **Configuration Validation** - Integrated validator with rich validation rules
- 📊 **Audit Logging** - Complete configuration loading process with sensitive field masking
- 🌐 **Multi-format Support** - TOML / JSON / YAML / INI
- ☁️ **Remote Configuration** - Support for Etcd / Consul / HTTP configuration centers
- 🔒 **Encryption Support** - Sensitive field encryption storage (v0.4.0+)
- 🛠️ **CLI Tools** - Template generation, validation, diff comparison

------

## 📦 Installation

Add the following to your `Cargo.toml`:

```toml
[dependencies]
confers = "0.1"
serde = { version = "1.0", features = ["derive"] }

# Optional features
confers = { version = "0.1", features = ["watch", "remote", "cli"] }
```

**Feature Flags**:

- `watch` - Enable configuration hot reload
- `remote` - Enable remote configuration center support
- `audit` - Enable audit logging (enabled by default)
- `schema` - Enable Schema export
- `cli` - Include CLI tools

------

## 🚀 Quick Start

### Basic Usage

```rust
use confers::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Config, Serialize, Deserialize, Debug)]
#[config(env_prefix = "MYAPP_")]
struct AppConfig {
    #[cfg_attr(description = "Server host address", default = "\"localhost\".to_string()")]
    host: String,
    
    #[cfg_attr(description = "Server port", default = "8080")]
    port: u16,
    
    #[cfg_attr(description = "Enable debug mode")]
    debug: Option<bool>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Automatically load configuration from multiple sources
    let config = AppConfig::load()?;
    
    println!("Server will start on {}:{}", config.host, config.port);
    
    Ok(())
}
```

### Configuration File (config.toml)

```toml
# Server host address
host = "0.0.0.0"

# Server port
port = 8080

# Enable debug mode
debug = true
```

### Environment Variable Override

```bash
# Environment variables take priority over configuration files
export MYAPP_PORT=9000
export MYAPP_DEBUG=false

# Run application
cargo run
```

### Command Line Arguments (Highest Priority)

```bash
# Command line arguments have the highest priority
cargo run -- --port 3000 --host 127.0.0.1
```

---

## 📖 Core Concepts

### Configuration Source Priority

Confers automatically merges configurations in the following priority order (from lowest to highest):

```
1. System configuration file      /etc/{app_name}/config.*
2. User configuration file        ~/.config/{app_name}/config.*
3. Remote configuration center   etcd://... / consul://... / http://...
4. Specified configuration file  --config path/to/config.toml
5. Environment variables         {PREFIX}_KEY=value
6. Command line arguments        --key value (highest priority)
```

**Partial Override Strategy**: High-priority configuration sources only override explicitly specified fields, other fields are inherited from lower-priority sources.

### Nested Configuration

```rust
#[derive(Config, Serialize, Deserialize, Debug)]
struct AppConfig {
    server: ServerConfig,
    database: DatabaseConfig,
}

#[derive(Serialize, Deserialize, Debug)]
struct ServerConfig {
    host: String,
    port: u16,
}

#[derive(Serialize, Deserialize, Debug)]
struct DatabaseConfig {
    #[cfg_attr(description = "Database connection URL")]
    url: String,
    
    #[cfg_attr(description = "Connection pool size", default = "10")]
    pool_size: u32,
}
```

**Configuration File**:

```toml
[server]
host = "0.0.0.0"
port = 8080

[database]
url = "postgresql://localhost/mydb"
pool_size = 20
```

**Environment Variable Mapping**:

```bash
export MYAPP_SERVER_HOST=localhost
export MYAPP_SERVER_PORT=9000
export MYAPP_DATABASE_URL=postgresql://prod/db
export MYAPP_DATABASE_POOL_SIZE=50
```

------

## 🎨 Macro Attributes Explained

### Struct Level Attributes

```rust
#[derive(Config)]
#[config(
    env_prefix = "MYAPP_",              // Environment variable prefix (default: empty)
    strict = false,                      // Strict mode (default: false)
    watch = true,                        // Enable hot reload (default: false)
    format_detection = "ByContent",      // Format detection method (default: ByContent)
    audit_log = true,                    // Enable audit logging (default: true)
    audit_log_path = "./config.log",     // Audit log path
    remote = "etcd://localhost:2379/app" // Remote configuration address (optional)
)]
struct AppConfig { }
```

### Field Level Attributes

```rust
#[cfg_attr(
    // Basic attributes
    description = "Field description",           // For documentation and template generation
    default = "default value expression",        // Default value (Rust expression)
    
    // Naming configuration
    name_config = "key name in config file",     // Override default key name
    name_env = "environment variable name",      // Override default env var name
    name_clap_long = "long option",              // CLI long option name
    name_clap_short = 'c',                       // CLI short option
    
    // Validation rules
    validate = "range(min = 1, max = 65535)", // validator syntax
    custom_validate = "my_validator",         // Custom validation function
    
    // Security configuration
    sensitive = true,                   // Sensitive field (masked in audit logs)
    encrypted = true,                   // Encrypted storage (v0.4.0+)
    
    // Special markers
    flatten,                            // Flatten nested structure
    skip                                // Skip this field
)]
```

------

## 💡 Examples

### 1. Basic Configuration

```rust
use confers::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Config, Serialize, Deserialize)]
#[config(env_prefix = "APP_")]
struct Config {
    #[cfg_attr(default = "\"localhost\".to_string()")]
    host: String,
    
    #[cfg_attr(default = "8080")]
    port: u16,
}

fn main() {
    let config = Config::load().unwrap();
    println!("{:?}", config);
}
```

### 2. Configuration Validation

```rust
#[derive(Config, Serialize, Deserialize)]
struct Config {
    #[cfg_attr(
        validate = "range(min = 1, max = 65535)",
        error_msg = "Port must be between 1-65535"
    )]
    port: u16,
    
    #[cfg_attr(
        validate = "email",
        error_msg = "Invalid email address"
    )]
    email: String,
    
    #[cfg_attr(
        validate = "url",
        error_msg = "Invalid URL"
    )]
    website: String,
}

fn main() {
    match Config::load() {
        Ok(config) => println!("Configuration loaded successfully: {:?}", config),
        Err(e) => eprintln!("Configuration validation failed: {}", e),
    }
}
```

### 3. Hot Reload

```rust
use confers::prelude::*;
use tokio;

#[derive(Config, Serialize, Deserialize, Clone)]
#[config(watch = true)]
struct Config {
    port: u16,
    debug: bool,
}

#[tokio::main]
async fn main() {
    // Start configuration monitoring
    let watcher = Config::watch().unwrap();
    
    // Method 1: Channel mode (recommended)
    let mut rx = watcher.subscribe();
    tokio::spawn(async move {
        while rx.changed().await.is_ok() {
            let new_config = rx.borrow().clone();
            println!("Configuration updated: {:?}", new_config);
            // Reload resources, update status, etc.
        }
    });
    
    // Method 2: Callback mode
    watcher.on_change(|config| {
        println!("Configuration changed: {:?}", config);
    });
    
    // Main application logic
    loop {
        let config = watcher.current();
        println!("Current port: {}", config.port);
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    }
}
```

### 4. Remote Configuration

```rust
#[derive(Config, Serialize, Deserialize)]
#[config(
    remote = "etcd://localhost:2379/myapp/config",
    remote_fallback = true  // Fallback to local config when remote fails
)]
struct Config {
    port: u16,
    database_url: String,
}

#[tokio::main]
async fn main() {
    // Automatically load configuration from Etcd
    let config = Config::load().await.unwrap();
    println!("{:?}", config);
}
```

Supported remote configuration centers:

- **Etcd**: `etcd://host:port/key`
- **Consul**: `consul://host:port/key`
- **HTTP**: `http://api.example.com/config` or `https://...`

### 5. Sensitive Field Handling

```rust
#[derive(Config, Serialize, Deserialize)]
struct Config {
    #[cfg_attr(
        sensitive = true,
        description = "Database password"
    )]
    db_password: String,
    
    #[cfg_attr(
        sensitive = true,
        encrypted = true,  // v0.4.0+
        description = "API key"
    )]
    api_key: String,
}

fn main() {
    let config = Config::load().unwrap();
    
    // Export audit log (sensitive fields automatically masked)
    config.export_audit_log().unwrap();
    // In audit log shows as:
    // db_password = "******"
    // api_key = "******"
}
```

### 6. Custom Validation

```rust
use validator::ValidationError;

fn validate_password_strength(password: &str) -> Result<(), ValidationError> {
    if password.len() < 8 {
        return Err(ValidationError::new("password_too_short"));
    }
    if !password.chars().any(|c| c.is_numeric()) {
        return Err(ValidationError::new("password_needs_number"));
    }
    Ok(())
}

#[derive(Config, Serialize, Deserialize)]
struct Config {
    #[cfg_attr(
        custom_validate = "validate_password_strength",
        error_msg = "Password strength insufficient"
    )]
    password: String,
}
```

### 7. Generate Configuration Template

```rust
#[derive(Config, Serialize, Deserialize)]
#[config(env_prefix = "MYAPP_")]
struct Config {
    #[cfg_attr(description = "Server port", default = "8080")]
    port: u16,
    
    #[cfg_attr(description = "Enable debug mode", default = "false")]
    debug: bool,
}

fn main() {
    // Generate complete template (with all fields and comments)
    let template = Config::generate_template(TemplateLevel::Full);
    println!("{}", template);
    
    // Output:
    // # Server port
    // port = 8080
    //
    // # Enable debug mode
    // debug = false
}
```

------

## 🛠️ CLI Tools

### Installation

```bash
cargo install confers-cli
```

### Commands

#### 1. Generate Configuration Template

```bash
# Generate complete template
confers generate --output config.toml --level full

# Generate minimal template (only required fields)
confers generate --output config.toml --level minimal
```

#### 2. Validate Configuration File

```bash
confers validate --config config.toml

# Output:
# ✅ Configuration validation passed
# or
# ❌ Validation failed:
#   - port: Port must be between 1-65535
#   - email: Invalid email address
```

#### 3. Configuration Diff Comparison

```bash
confers diff production.toml staging.toml

# Output:
# - port: 8080
# + port: 9000
#   host: "0.0.0.0"
# - debug: true
# + debug: false
```

#### 4. Export Schema

```bash
# Generate JSON Schema
confers schema --format json --output schema.json

# Generate TypeScript type definitions
confers schema --format typescript --output config.d.ts
```

#### 5. Shell Auto-completion

```bash
# Bash
confers completions bash > /usr/share/bash-completion/completions/myapp

# Zsh
confers completions zsh > ~/.zsh/completion/_myapp

# Fish
confers completions fish > ~/.config/fish/completions/myapp.fish
```

#### 6. Encrypt Configuration (v0.4.0+)

```bash
# Generate encryption key
confers keygen --output ~/.confers/encryption.key

# Encrypt single value
confers encrypt --value "my_secret_password"
# Output: enc:AES256:Zm9vYmFyLi4u

# Batch encrypt configuration file
confers encrypt-file --input config.plain.toml --output config.encrypted.toml
```

---

## 📚 Complete Usage Guide

### Configuration Loading Process

```
1. Initialize application metadata
   ├─ Get application name (from Cargo.toml or environment variables)
   ├─ Get environment variable prefix
   └─ Determine configuration file search paths

2. Load configuration sources by priority
   ├─ System configuration file (/etc/{app}/config.*)
   ├─ User configuration file (~/.config/{app}/config.*)
   ├─ Remote configuration center (etcd/consul/http)
   ├─ Specified configuration file (--config)
   ├─ Environment variables ({PREFIX}_*)
   └─ Command line arguments

3. Configuration merging and validation
   ├─ Merge using Figment by priority
   ├─ Partial override strategy
   ├─ Type conversion and deserialization
   └─ Execute validation rules

4. Generate audit log
   ├─ Record all configuration source statuses
   ├─ Output final configuration (masked)
   └─ Record validation results

5. Return configuration object
```

### Error Handling

#### Strict Mode vs Lenient Mode

```rust
// Strict mode: Any configuration source failure returns error
#[derive(Config)]
#[config(strict = true)]
struct Config { }

// Lenient mode (default): Allow partial configuration source failures
#[derive(Config)]
#[config(strict = false)]
struct Config { }
```

**Lenient Mode Behavior**:

- ✅ System configuration file not found → Skip (common case)
- ✅ User configuration file not found → Skip (common case)
- ❌ Specified configuration file not found → **Error** (user explicitly specified)
- ⚠️ Environment variable format error → Skip variable, log warning
- ❌ Command line argument error → **Error** (Clap handles automatically)

#### Error Types

```rust
use confers::ConfigError;

match Config::load() {
    Ok(config) => { /* ... */ }
    Err(ConfigError::FileNotFound { path }) => {
        eprintln!("Configuration file not found: {:?}", path);
    }
    Err(ConfigError::ParseError { source }) => {
        eprintln!("Configuration parsing failed: {}", source);
    }
    Err(ConfigError::ValidationError(errors)) => {
        eprintln!("Configuration validation failed:");
        for (field, error) in errors.field_errors() {
            eprintln!("  - {}: {}", field, error);
        }
    }
    Err(e) => {
        eprintln!("Unknown error: {}", e);
    }
}
```

### Cross-platform Path Handling

Confers automatically handles Windows and Unix path differences:

```rust
// Windows user configuration file
C:\Users\foo\config.toml

// Automatically converted to Unix style (internal handling)
/c/Users/foo/config.toml

// Path expansion
~/.config/app/config.toml  →  /home/user/.config/app/config.toml
$HOME/config.toml          →  /home/user/config.toml

// Mixed separators (automatic normalization)
C:/Users\foo/config.toml   →  /c/Users/foo/config.toml
```

### Multi-format Configuration Files

#### Format Priority

When multiple format configuration files exist in the same directory:

```
config.toml  ← Highest priority
config.json
config.yaml
config.ini   ← Lowest priority
```

#### Format Detection Mode

```rust
#[derive(Config)]
#[config(format_detection = "ByContent")]  // Default
struct Config { }

#[derive(Config)]
#[config(format_detection = "ByExtension")]  // Extension only
struct Config { }
```

**ByContent Mode** (Recommended):

- Read file content to determine format
- Prevent format mismatch (e.g., JSON content saved as .toml)
- Provide clear error messages

**ByExtension Mode**:

- Determine format only by file extension
- Better performance (no file reading)
- Suitable for scenarios with confirmed correct formats

### Audit Log

#### Audit Log Format

```toml
# Confers Configuration Audit Log
# Generated at: 2025-12-12 10:30:45 UTC

[metadata]
loaded_at = "2025-12-12T10:30:45Z"
app_name = "myapp"
version = "1.0.0"
hostname = "prod-server-01"
load_duration_ms = 125

[sources]
system_config = { status = "loaded", path = "/etc/myapp/config.toml" }
user_config = { status = "not_found", path = "~/.config/myapp/config.toml" }
remote_config = { status = "loaded", url = "etcd://localhost:2379/myapp" }
env_vars = { status = "loaded", count = 3 }
cli_args = { status = "loaded", count = 2 }

[warnings]
# Multiple format configuration files detected
multiple_formats_detected = [
    "/etc/myapp/config.toml",
    "/etc/myapp/config.json"  # Ignored
]

[config]
# Final merged configuration (sensitive fields masked)
host = "0.0.0.0"
port = 8080
debug = false

[config.database]
host = "localhost"
port = 5432
username = "admin"
password = "******"  # Sensitive field masked

[validation]
status = "passed"
errors = []
```

------

## 🔒 Security Best Practices

### 1. Sensitive Information Protection

```rust
#[derive(Config)]
struct Config {
    // ✅ Correct: Mark as sensitive field
    #[cfg_attr(sensitive = true)]
    db_password: String,
    
    #[cfg_attr(sensitive = true)]
    api_key: String,
    
    // ❌ Wrong: Not marked, may leak to logs
    secret_token: String,
}
```

### 2. Path Security

Confers automatically protects against path traversal attacks:

```rust
// ❌ Malicious paths will be rejected
../../../etc/passwd
../../.ssh/id_rsa
/etc/shadow

// ✅ Normal paths allowed
/etc/myapp/config.toml
~/.config/myapp/config.toml
./config.toml
```

### 3. Environment Variable Validation

```rust
// Confers automatically validates environment variables:
// - Key name length ≤ 256 bytes
// - Value length ≤ 4KB
// - Key names only allow alphanumeric and underscore
```

### 4. Configuration Encryption (v0.4.0+)

```rust
#[derive(Config)]
struct Config {
    #[cfg_attr(encrypted = true, sensitive = true)]
    db_password: String,
}
```

**Configuration File**:

```toml
# Use confers encrypt command to encrypt
db_password = "enc:AES256:Zm9vYmFyLi4u"
```

**Key Management**:

```bash
# Method 1: Environment variable
export CONFERS_ENCRYPTION_KEY="base64_encoded_key"

# Method 2: Key file
echo "base64_encoded_key" > ~/.confers/encryption.key
```

------

## ⚡ Performance Optimization

### Configuration Caching

```rust
use once_cell::sync::OnceCell;

static CONFIG: OnceCell<AppConfig> = OnceCell::new();

fn get_config() -> &'static AppConfig {
    CONFIG.get_or_init(|| {
        AppConfig::load().expect("Configuration loading failed")
    })
}

fn main() {
    // First call loads configuration
    let config = get_config();
    
    // Subsequent calls return cached value
    let config2 = get_config();  // Zero overhead
}
```

### Lazy Loading

```rust
#[derive(Config)]
struct Config {
    // Basic configuration loads immediately
    port: u16,
    
    // Complex configuration loads lazily
    #[cfg_attr(skip)]
    database: Option<DatabaseConfig>,
}

impl Config {
    fn database(&mut self) -> &DatabaseConfig {
        self.database.get_or_insert_with(|| {
            DatabaseConfig::load_from_file("database.toml").unwrap()
        })
    }
}
```

---

## 🐛 Troubleshooting

### Common Issues

#### 1. Configuration File Not Found

```
Error: Configuration file not found: /etc/myapp/config.toml
```

**Solutions**:

- Check if file path is correct
- Use `--config` to explicitly specify configuration file
- Enable lenient mode (`strict = false`) to skip missing configuration files

#### 2. Environment Variables Not Taking Effect

```
# Environment variable set but not taking effect
export PORT=9000  # ❌ Missing prefix
export MYAPP_PORT=9000  # ✅ Correct
```

**Checklist**:

- ✅ Does environment variable include correct prefix?
- ✅ Is variable name all uppercase?
- ✅ Are nested fields separated by underscores?

#### 3. Validation Failed

```
Error: Configuration validation failed
  - port: Port must be between 1-65535
```

**Solutions**:

- Check if configuration values meet validation rules
- View `error_msg` for detailed hints
- Use `confers validate` command to check configuration

#### 4. Hot Reload Not Working

**Checklist**:

- ✅ Is `watch = true` enabled?
- ✅ Is `watch` feature enabled? `confers = { features = ["watch"] }`
- ✅ Is file path correct?
- ✅ Do you have file write permissions?

### Debug Mode

```bash
# Enable debug logging
RUST_LOG=confers=debug cargo run

# View configuration loading order
confers debug --show-sources

# Export complete configuration (including source info)
confers debug --dump-config
```

------

## 🤝 Contributing

Contributions are welcome! Please check [CONTRIBUTING.md](CONTRIBUTING.md) for details.

### Development Environment Setup

```bash
# Clone repository
git clone https://github.com/yourusername/confers.git
cd confers

# Install dependencies
cargo build

# Run tests
cargo test --all-features

# Run examples
cargo run --example basic
```

### Commit Conventions

```
feat: New feature
fix: Bug fix
docs: Documentation update
test: Test related
refactor: Refactoring
perf: Performance optimization
```

------

## 📄 License

This project is dual-licensed under MIT or Apache-2.0. See [LICENSE-MIT](LICENSE-MIT) and [LICENSE-APACHE](LICENSE-APACHE) for details.

------

## 🙏 Acknowledgments

Confers is built on the following excellent open source projects:

- [figment](https://github.com/SergioBenitez/Figment) - Configuration merging
- [serde](https://github.com/serde-rs/serde) - Serialization framework
- [clap](https://github.com/clap-rs/clap) - Command line parsing
- [validator](https://github.com/Keats/validator) - Data validation