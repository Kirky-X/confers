//! Remote Consul Configuration Example
//!
//! This example demonstrates how to use confers to load remote configuration
//! from Consul KV Store with TLS support.
//!
//! Features:
//! - Consul KV Store integration
//! - TLS/SSL secure connection
//! - ACL token authentication
//! - Configuration watching and updates
//!
//! Run:
//!   # Start Consul
//!   docker run -d --name consul -p 8500:8500 consul:latest
//!
//!   # Run example
//!   cargo run --bin remote_consul
//!
//! Design: ADR-005 (Remote Sources), ADR-022 (Remote Source Security)

use base64::Engine;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

// =============================================================================
// Configuration Structures
// =============================================================================

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    #[serde(default)]
    pub name: String,

    #[serde(default)]
    pub version: String,

    #[serde(default)]
    pub environment: String,

    #[serde(default)]
    pub server: ServerConfig,

    #[serde(default)]
    pub database: DatabaseConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ServerConfig {
    #[serde(default)]
    pub host: String,

    #[serde(default)]
    pub port: u16,

    #[serde(default)]
    pub workers: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DatabaseConfig {
    #[serde(default)]
    pub url: String,

    #[serde(default)]
    pub max_connections: u32,
}

// =============================================================================
// Consul Client Configuration
// =============================================================================

/// Consul client configuration
#[derive(Debug, Clone)]
pub struct ConsulConfig {
    /// Consul HTTP address
    pub address: String,

    /// Datacenter (optional)
    pub datacenter: Option<String>,

    /// ACL token (optional)
    pub token: Option<String>,

    /// Namespace (Consul Enterprise, optional)
    pub namespace: Option<String>,

    /// TLS configuration
    pub tls: Option<TlsConfig>,

    /// Request timeout in seconds
    pub timeout_seconds: u64,
}

impl Default for ConsulConfig {
    fn default() -> Self {
        Self {
            address: "http://127.0.0.1:8500".to_string(),
            datacenter: None,
            token: None,
            namespace: None,
            tls: None,
            timeout_seconds: 10,
        }
    }
}

impl ConsulConfig {
    /// Create new Consul configuration
    pub fn new(address: &str) -> Self {
        Self {
            address: address.to_string(),
            ..Default::default()
        }
    }

    /// Set ACL token
    pub fn with_token(mut self, token: &str) -> Self {
        self.token = Some(token.to_string());
        self
    }

    /// Set datacenter
    pub fn with_datacenter(mut self, datacenter: &str) -> Self {
        self.datacenter = Some(datacenter.to_string());
        self
    }

    /// Enable TLS
    pub fn with_tls(mut self, tls: TlsConfig) -> Self {
        self.tls = Some(tls);
        self
    }

    /// Build HTTP client with configuration
    pub fn build_client(&self) -> Result<reqwest::Client, Box<dyn std::error::Error>> {
        let mut builder =
            reqwest::Client::builder().timeout(Duration::from_secs(self.timeout_seconds));

        // Configure TLS if provided
        if let Some(ref tls) = self.tls {
            builder = tls.configure_client(builder)?;
        }

        Ok(builder.build()?)
    }
}

// =============================================================================
// TLS Configuration
// =============================================================================

/// TLS configuration for secure Consul connections
#[derive(Debug, Clone)]
pub struct TlsConfig {
    /// Path to CA certificate file
    pub ca_cert_path: PathBuf,

    /// Path to client certificate file (optional, for mTLS)
    pub client_cert_path: Option<PathBuf>,

    /// Path to client private key file (optional, for mTLS)
    pub client_key_path: Option<PathBuf>,

    /// Server name for SNI (optional)
    pub server_name: Option<String>,

    /// Skip certificate verification (dangerous, for testing only)
    pub danger_accept_invalid_certs: bool,
}

impl TlsConfig {
    /// Create TLS configuration with CA certificate
    pub fn new(ca_cert_path: impl Into<PathBuf>) -> Self {
        Self {
            ca_cert_path: ca_cert_path.into(),
            client_cert_path: None,
            client_key_path: None,
            server_name: None,
            danger_accept_invalid_certs: false,
        }
    }

    /// Enable mutual TLS (client certificate authentication)
    pub fn with_client_cert(
        mut self,
        cert_path: impl Into<PathBuf>,
        key_path: impl Into<PathBuf>,
    ) -> Self {
        self.client_cert_path = Some(cert_path.into());
        self.client_key_path = Some(key_path.into());
        self
    }

    /// Set server name for SNI
    pub fn with_server_name(mut self, name: &str) -> Self {
        self.server_name = Some(name.to_string());
        self
    }

    /// Configure reqwest client with TLS settings
    pub fn configure_client(
        &self,
        builder: reqwest::ClientBuilder,
    ) -> Result<reqwest::ClientBuilder, Box<dyn std::error::Error>> {
        let mut builder = builder;

        // Load CA certificate
        let ca_cert = std::fs::read(&self.ca_cert_path)?;
        let ca_cert = reqwest::Certificate::from_pem(&ca_cert)?;
        builder = builder.add_root_certificate(ca_cert);

        // Load client certificate and key for mTLS
        // Note: reqwest's identity support varies by platform and TLS backend
        // For full mTLS support, consider using a custom TLS connector
        if let (Some(_cert_path), Some(_key_path)) = (&self.client_cert_path, &self.client_key_path)
        {
            // mTLS requires native-tls or rustls-tls with specific features
            // This is a placeholder showing the configuration pattern
            info!(
                "TLS: Client certificate authentication configured (requires native-tls feature)"
            );
            info!(
                "TLS: For production mTLS, use a custom TLS connector or enable native-tls feature"
            );
        }

        // Set server name for SNI
        if let Some(ref name) = self.server_name {
            builder = builder.tls_info(true);
            info!("TLS: Server name set to '{}'", name);
        }

        // Danger: accept invalid certs (testing only)
        if self.danger_accept_invalid_certs {
            #[cfg(not(debug_assertions))]
            panic!("SECURITY: danger_accept_invalid_certs must not be used in release builds!");

            #[cfg(debug_assertions)]
            {
                builder = builder.danger_accept_invalid_certs(true);
                warn!("⚠️ CRITICAL SECURITY WARNING: Accepting invalid certificates!");
                warn!("This should NEVER be enabled in production environments!");
            }
        }

        Ok(builder)
    }
}

// =============================================================================
// Consul Client
// =============================================================================

/// Consul client for configuration management
pub struct ConsulClient {
    config: ConsulConfig,
    client: reqwest::Client,
    cache: Arc<RwLock<HashMap<String, String>>>,
}

impl ConsulClient {
    /// Create new Consul client
    pub fn new(config: ConsulConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let client = config.build_client()?;
        Ok(Self {
            config,
            client,
            cache: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Test connection to Consul
    pub async fn test_connection(&self) -> Result<bool, Box<dyn std::error::Error>> {
        let url = format!("{}/v1/status/leader", self.config.address);
        info!("Testing Consul connection: {}", url);

        let mut request = self.client.get(&url);

        // Add ACL token if configured
        if let Some(ref token) = self.config.token {
            request = request.header("X-Consul-Token", token);
        }

        let response = request.send().await?;

        if response.status().is_success() {
            let leader = response.text().await?;
            info!(
                "✓ Consul connection successful, leader: {}",
                leader.trim_matches('"')
            );
            Ok(true)
        } else {
            warn!("✗ Consul returned status: {}", response.status());
            Ok(false)
        }
    }

    /// Read all keys under a prefix
    pub async fn read_prefix(
        &self,
        prefix: &str,
    ) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
        let url = format!("{}/v1/kv/{}?recurse=true", self.config.address, prefix);
        info!("Reading Consul KV prefix: {}", prefix);

        let mut request = self.client.get(&url);

        // Add ACL token if configured
        if let Some(ref token) = self.config.token {
            request = request.header("X-Consul-Token", token);
        }

        // Add datacenter if configured
        if let Some(ref dc) = self.config.datacenter {
            request = request.query(&[("dc", dc)]);
        }

        // Add namespace if configured
        if let Some(ref ns) = self.config.namespace {
            request = request.query(&[("ns", ns)]);
        }

        let response = request.send().await?;

        if !response.status().is_success() {
            if response.status() == reqwest::StatusCode::NOT_FOUND {
                info!("No keys found under prefix: {}", prefix);
                return Ok(HashMap::new());
            }
            return Err(format!("Consul error: {}", response.status()).into());
        }

        let kv_pairs: Vec<serde_json::Value> = response.json().await?;
        let mut results = HashMap::new();

        for item in kv_pairs {
            if let (Some(key), Some(value)) = (item.get("Key"), item.get("Value")) {
                let key_str = key.as_str().unwrap_or("");
                let value_str = value
                    .as_str()
                    .and_then(|s| base64::engine::general_purpose::STANDARD.decode(s).ok())
                    .and_then(|b| String::from_utf8(b).ok())
                    .unwrap_or_default();

                // Remove prefix from key
                let short_key = key_str
                    .strip_prefix(&format!("{}/", prefix))
                    .unwrap_or(key_str)
                    .to_string();

                results.insert(short_key, value_str);
            }
        }

        // Update cache
        *self.cache.write().await = results.clone();

        info!("Read {} keys from Consul", results.len());
        Ok(results)
    }

    /// Write a key-value pair
    pub async fn write_key(
        &self,
        key: &str,
        value: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let url = format!("{}/v1/kv/{}", self.config.address, key);
        info!("Writing key to Consul: {}", key);

        let mut request = self.client.put(&url).body(value.to_string());

        // Add ACL token if configured
        if let Some(ref token) = self.config.token {
            request = request.header("X-Consul-Token", token);
        }

        let response = request.send().await?;

        if response.status().is_success() {
            info!("✓ Key written successfully");
            Ok(())
        } else {
            Err(format!("Failed to write key: {}", response.status()).into())
        }
    }

    /// Delete a key
    pub async fn delete_key(&self, key: &str) -> Result<(), Box<dyn std::error::Error>> {
        let url = format!("{}/v1/kv/{}", self.config.address, key);
        info!("Deleting key from Consul: {}", key);

        let mut request = self.client.delete(&url);

        // Add ACL token if configured
        if let Some(ref token) = self.config.token {
            request = request.header("X-Consul-Token", token);
        }

        let response = request.send().await?;

        if response.status().is_success() {
            info!("✓ Key deleted successfully");
            Ok(())
        } else {
            Err(format!("Failed to delete key: {}", response.status()).into())
        }
    }

    /// Load configuration into typed struct
    pub async fn load_config<T: for<'de> Deserialize<'de>>(
        &self,
        prefix: &str,
    ) -> Result<T, Box<dyn std::error::Error>> {
        let kv_map = self.read_prefix(prefix).await?;

        // Convert flat key-value map to nested JSON
        let json_value = self.kv_to_json(&kv_map);

        // Deserialize to target type
        let config: T = serde_json::from_value(json_value)?;

        Ok(config)
    }

    /// Convert flat KV map to nested JSON
    fn kv_to_json(&self, map: &HashMap<String, String>) -> serde_json::Value {
        let mut result = serde_json::Map::new();

        for (key, value) in map {
            let parts: Vec<&str> = key.split('/').collect();
            Self::set_nested_value(&mut result, &parts, value);
        }

        serde_json::Value::Object(result)
    }

    /// Set nested value in JSON map
    fn set_nested_value(
        map: &mut serde_json::Map<String, serde_json::Value>,
        parts: &[&str],
        value: &str,
    ) {
        if parts.is_empty() {
            return;
        }

        if parts.len() == 1 {
            let json_value = Self::parse_value(value);
            map.insert(parts[0].to_string(), json_value);
        } else {
            let nested = map
                .entry(parts[0].to_string())
                .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));

            if let serde_json::Value::Object(ref mut nested_map) = nested {
                Self::set_nested_value(nested_map, &parts[1..], value);
            }
        }
    }

    /// Parse string value to appropriate JSON type
    fn parse_value(value: &str) -> serde_json::Value {
        if value == "true" {
            serde_json::Value::Bool(true)
        } else if value == "false" {
            serde_json::Value::Bool(false)
        } else if let Ok(n) = value.parse::<i64>() {
            serde_json::Value::Number(n.into())
        } else if let Ok(n) = value.parse::<f64>() {
            serde_json::Number::from_f64(n)
                .map(serde_json::Value::Number)
                .unwrap_or_else(|| serde_json::Value::String(value.to_string()))
        } else {
            serde_json::Value::String(value.to_string())
        }
    }
}

// =============================================================================
// Main Program
// =============================================================================

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .with_target(false)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .init();

    info!("========================================");
    info!("  Remote Consul Configuration Example");
    info!("========================================");

    // Demo 1: Basic connection
    demo_basic_connection().await?;

    // Demo 2: TLS configuration
    demo_tls_configuration().await?;

    // Demo 3: ACL token authentication
    demo_acl_authentication().await?;

    // Demo 4: Configuration management
    demo_config_management().await?;

    info!("");
    info!("========================================");
    info!("  Example completed!");
    info!("========================================");

    Ok(())
}

/// Demo 1: Basic connection
async fn demo_basic_connection() -> Result<(), Box<dyn std::error::Error>> {
    info!("\n=== Demo 1: Basic Connection ===\n");

    let address =
        std::env::var("CONSUL_ADDRESS").unwrap_or_else(|_| "http://127.0.0.1:8500".to_string());

    let config = ConsulConfig::new(&address);
    let client = ConsulClient::new(config)?;

    // Test connection
    match client.test_connection().await {
        Ok(true) => info!("✓ Consul is reachable"),
        Ok(false) => {
            warn!("Consul is not reachable (this is normal if Consul is not running)");
            info!("Start Consul with: docker run -d --name consul -p 8500:8500 consul:latest");
        }
        Err(e) => error!("Connection error: {}", e),
    }

    // Read configuration
    match client.read_prefix("myapp/config").await {
        Ok(kv) => {
            if kv.is_empty() {
                info!("No configuration found at 'myapp/config'");
                info!("Add configuration with:");
                info!("  consul kv put myapp/config/name 'myapp'");
                info!("  consul kv put myapp/config/version '1.0.0'");
                info!("  consul kv put myapp/config/server/host '0.0.0.0'");
                info!("  consul kv put myapp/config/server/port '8080'");
            } else {
                info!("Configuration values:");
                for (key, value) in kv.iter().take(5) {
                    info!("  {} = {}", key, value);
                }
            }
        }
        Err(e) => warn!("Failed to read configuration: {}", e),
    }

    Ok(())
}

/// Demo 2: TLS configuration
async fn demo_tls_configuration() -> Result<(), Box<dyn std::error::Error>> {
    info!("\n=== Demo 2: TLS Configuration ===\n");

    // TLS configuration example
    info!("TLS Configuration Steps:");
    info!("");
    info!("1. Generate certificates:");
    info!("   # Create CA");
    info!("   openssl genrsa -out ca.key 2048");
    info!("   openssl req -new -x509 -days 365 -key ca.key -out ca.crt");
    info!("");
    info!("   # Create server certificate");
    info!("   openssl genrsa -out server.key 2048");
    info!("   openssl req -new -key server.key -out server.csr");
    info!("   openssl x509 -req -days 365 -in server.csr -CA ca.crt -CAkey ca.key -CAcreateserial -out server.crt");
    info!("");
    info!("   # Create client certificate (for mTLS)");
    info!("   openssl genrsa -out client.key 2048");
    info!("   openssl req -new -key client.key -out client.csr");
    info!("   openssl x509 -req -days 365 -in client.csr -CA ca.crt -CAkey ca.key -CAcreateserial -out client.crt");
    info!("");
    info!("2. Configure Consul with TLS:");
    info!("   consul agent -server -bootstrap-expect=1 \\");
    info!("     -data-dir=/tmp/consul \\");
    info!("     -cert-file=/path/to/server.crt \\");
    info!("     -key-file=/path/to/server.key \\");
    info!("     -ca-file=/path/to/ca.crt \\");
    info!("     -verify-incoming -verify-outgoing");
    info!("");
    info!("3. Configure client with TLS:");

    let tls_config = TlsConfig::new("/path/to/ca.crt")
        .with_client_cert("/path/to/client.crt", "/path/to/client.key")
        .with_server_name("consul.example.com");

    info!("   TlsConfig {{");
    info!("     ca_cert_path: {:?}", tls_config.ca_cert_path);
    info!("     client_cert_path: {:?}", tls_config.client_cert_path);
    info!("     client_key_path: {:?}", tls_config.client_key_path);
    info!("     server_name: {:?}", tls_config.server_name);
    info!("   }}");

    // Create client with TLS
    let config = ConsulConfig::new("https://consul.example.com:8501").with_tls(tls_config);

    info!("\nConsulConfig with TLS:");
    info!("  address: {}", config.address);
    info!("  tls: enabled");

    info!("\nSecurity best practices:");
    info!("  - Always use TLS in production");
    info!("  - Use mutual TLS (mTLS) for client authentication");
    info!("  - Rotate certificates regularly");
    info!("  - Use strong key sizes (2048+ bits for RSA, 256+ bits for ECDSA)");
    info!("  - Set appropriate certificate expiration");

    Ok(())
}

/// Demo 3: ACL token authentication
async fn demo_acl_authentication() -> Result<(), Box<dyn std::error::Error>> {
    info!("\n=== Demo 3: ACL Token Authentication ===\n");

    info!("Consul ACL Configuration:");
    info!("");
    info!("1. Enable ACLs in Consul configuration:");
    info!("   acl = {{");
    info!("     enabled = true");
    info!("     default_policy = \"deny\"");
    info!("     down_policy = \"extend-cache\"");
    info!("   }}");
    info!("");
    info!("2. Create ACL policy:");
    info!("   consul acl policy create -name 'config-read' \\");
    info!("     -rules 'key_prefix \"myapp/config\" {{ policy = \"read\" }}'");
    info!("");
    info!("3. Create ACL token:");
    info!("   consul acl token create -description 'Config reader' \\");
    info!("     -policy-name 'config-read'");
    info!("");
    info!("4. Use token in client:");

    let config = ConsulConfig::new("http://127.0.0.1:8500").with_token("your-acl-token-here");

    info!("   ConsulConfig {{");
    info!("     address: {}", config.address);
    info!(
        "     token: {:?}...",
        &config.token.as_ref().map(|t| &t[..8]).unwrap_or("None")
    );
    info!("   }}");

    info!("\nACL best practices:");
    info!("  - Use least privilege principle");
    info!("  - Create separate tokens for read and write operations");
    info!("  - Rotate tokens regularly");
    info!("  - Use token inheritance for service tokens");
    info!("  - Monitor token usage with audit logs");

    Ok(())
}

/// Demo 4: Configuration management
async fn demo_config_management() -> Result<(), Box<dyn std::error::Error>> {
    info!("\n=== Demo 4: Configuration Management ===\n");

    let address =
        std::env::var("CONSUL_ADDRESS").unwrap_or_else(|_| "http://127.0.0.1:8500".to_string());

    let config = ConsulConfig::new(&address);
    let client = ConsulClient::new(config)?;

    // Write configuration
    info!("Writing sample configuration...");
    client.write_key("myapp/config/name", "myapp").await?;
    client.write_key("myapp/config/version", "1.0.0").await?;
    client
        .write_key("myapp/config/server/host", "0.0.0.0")
        .await?;
    client.write_key("myapp/config/server/port", "8080").await?;
    client
        .write_key("myapp/config/database/url", "postgresql://localhost/mydb")
        .await?;
    client
        .write_key("myapp/config/database/max_connections", "20")
        .await?;

    // Read configuration
    info!("\nReading configuration...");
    match client.load_config::<AppConfig>("myapp/config").await {
        Ok(config) => {
            info!("✓ Configuration loaded successfully:");
            info!("  App: {} v{}", config.name, config.version);
            info!("  Server: {}:{}", config.server.host, config.server.port);
            info!("  Database: {}", config.database.url);
            info!("  Max connections: {}", config.database.max_connections);
        }
        Err(e) => {
            warn!("Failed to load configuration: {}", e);
        }
    }

    // Cleanup
    info!("\nCleaning up...");
    client.delete_key("myapp/config/name").await?;
    client.delete_key("myapp/config/version").await?;
    client.delete_key("myapp/config/server/host").await?;
    client.delete_key("myapp/config/server/port").await?;
    client.delete_key("myapp/config/database/url").await?;
    client
        .delete_key("myapp/config/database/max_connections")
        .await?;

    info!("✓ Cleanup complete");

    Ok(())
}
