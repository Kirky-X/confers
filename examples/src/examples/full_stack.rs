//! Full Stack - Complete Feature Set Example
//!
//! This example demonstrates the comprehensive capabilities of confers.


use confers::dynamic::DynamicField;
use confers::secret::{derive_field_key, SecretString, XChaCha20Crypto};
use confers::watcher::WatcherConfig;
use confers::Config;
use serde::Deserialize;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

#[derive(Config, Deserialize, Debug, Clone)]
pub struct AppConfig {
    #[config(default = "myapp".to_string())]
    pub name: String,

    #[config(default = "1.0.0".to_string())]
    pub version: String,

    #[config(default = "development".to_string())]
    pub environment: String,

    #[config(default = "127.0.0.1".to_string())]
    pub host: String,

    #[config(default = 8080u16)]
    pub port: u16,

    #[config(default = 4usize)]
    pub workers: usize,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .with_thread_ids(false)
        .with_file(true)
        .with_line_number(true)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("Failed to set log subscriber");

    info!("============================================================");
    info!("Full Stack - Complete Feature Set Example");
    info!("============================================================");

    std::env::set_var("APP_ENCRYPTION_KEY", "12345678901234567890123456789012");

    demo_config_loading()?;
    demo_encryption()?;
    demo_migration()?;
    demo_dynamic_fields()?;
    demo_watcher()?;

    info!("============================================================");
    info!("All demonstrations completed!");
    info!("============================================================");

    Ok(())
}

fn demo_config_loading() -> Result<(), Box<dyn std::error::Error>> {
    info!("");
    info!("============================================================");
    info!("Demo 1: Configuration Loading");
    info!("============================================================");

    let config = AppConfig::load_sync()?;

    info!("App Name: {}", config.name);
    info!("App Version: {}", config.version);
    info!("Environment: {}", config.environment);
    info!("Server: {}:{}", config.host, config.port);
    info!("Workers: {}", config.workers);

    Ok(())
}

fn demo_encryption() -> Result<(), Box<dyn std::error::Error>> {
    info!("");
    info!("============================================================");
    info!("Demo 2: Encryption");
    info!("============================================================");

    // SecretString usage
    let password = SecretString::new("super-secret-password");
    info!("Password (Debug): {:?}", password);
    info!("Password (expose): {}", password.expose());

    // XChaCha20 encryption
    let crypto = XChaCha20Crypto::new();
    let plaintext = b"This is sensitive data";
    let key = [
        0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f,
        0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e,
        0x1f, 0x20,
    ];

    let (nonce, ciphertext) = crypto.encrypt(plaintext, &key)?;
    info!(
        "Encrypted: nonce={:?}, ciphertext len={}",
        nonce,
        ciphertext.len()
    );

    let decrypted = crypto.decrypt(&nonce, &ciphertext, &key)?;
    info!("Decrypted: {}", String::from_utf8(decrypted)?);

    // Field key derivation
    let master_key = key;
    let field_key = derive_field_key(&master_key, "database.password", "v1")?;
    info!("Field key derived: {:02x?}", &field_key[..8]);

    Ok(())
}

fn demo_migration() -> Result<(), Box<dyn std::error::Error>> {
    info!("");
    info!("============================================================");
    info!("Demo 3: Configuration Migration");
    info!("============================================================");

    // Note: Migration is handled through Config derive macro
    // See the migration example for detailed usage
    info!("Migration support via Config derive macro");
    info!("Use #[config(version = N)] and ConfigMigration trait");

    Ok(())
}

fn demo_dynamic_fields() -> Result<(), Box<dyn std::error::Error>> {
    info!("");
    info!("============================================================");
    info!("Demo 4: Dynamic Fields");
    info!("============================================================");

    // Create a dynamic field
    let field = DynamicField::new(42i32);
    info!("Initial value: {}", field.get());

    field.update(100);
    info!("After update(100): {}", field.get());

    Ok(())
}

fn demo_watcher() -> Result<(), Box<dyn std::error::Error>> {
    info!("");
    info!("============================================================");
    info!("Demo 5: File Watcher");
    info!("============================================================");

    // Note: This is a demo - in production you'd watch a real config file
    let watcher_config = WatcherConfig::default();

    info!(
        "Watcher config: debounce={}ms, min_interval={}ms, max_failures={}",
        watcher_config.debounce_ms,
        watcher_config.min_reload_interval_ms,
        watcher_config.max_consecutive_failures
    );

    Ok(())
}
