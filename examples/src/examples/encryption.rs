//! Encryption Example - Sensitive Data Protection
//!
//! This example demonstrates how to use confers' encryption features:
//! - SecretString for sensitive string data
//! - SecretBytes for sensitive binary data
//! - XChaCha20 encryption/decryption
//! - Field-level key derivation
//! - EnvKeyProvider for key management

use confers::secret::{
    derive_field_key, EnvKeyProvider, SecretBytes, SecretKeyProvider, SecretString, XChaCha20Crypto,
};
use confers::Config;
use serde::Deserialize;

#[derive(Config, Deserialize, Debug, Clone)]
pub struct AppConfig {
    #[config(default = "myapp".to_string())]
    pub name: String,

    #[config(default = "1.0.0".to_string())]
    pub version: String,

    #[config(default = "localhost".to_string())]
    pub db_host: String,

    #[config(default = 5432u16)]
    pub db_port: u16,

    #[config(default = "appuser".to_string())]
    pub db_user: String,
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    println!("========================================");
    println!("  Encryption Example - Sensitive Data Protection");
    println!("========================================");

    // Set environment variable as encryption key (production should use more secure methods)
    // Key must be 32 bytes
    std::env::set_var("APP_ENCRYPTION_KEY", "12345678901234567890123456789012");

    // Example 1: Basic SecretString usage
    demonstrate_secret_string();

    // Example 2: Basic SecretBytes usage
    demonstrate_secret_bytes();

    // Example 3: XChaCha20 encryption/decryption
    demonstrate_encryption_decryption();

    // Example 4: Field-level key derivation
    demonstrate_field_key_derivation();

    // Example 5: Use EnvKeyProvider
    demonstrate_env_key_provider();

    // Example 6: Basic config loading
    demonstrate_config_loading();

    println!("\n========================================");
    println!("  All examples completed!");
    println!("========================================");
}

fn demonstrate_secret_string() {
    println!("\n=== Example 1: SecretString Basic Usage ===");

    let password = SecretString::new("my-super-secret-password123");

    // Debug output is automatically redacted - won't leak actual content
    println!("Password (Debug): {:?}", password);

    // Correct way to expose sensitive value
    let exposed = password.expose();
    println!("Password (expose): {}", exposed);

    // Use Deref to get string reference
    let password_ref: &str = &password;
    println!("Password (Deref): {}", password_ref);

    // Clone sensitive value (note: both values can be accessed after clone)
    let cloned = password.clone();
    println!("Password (clone): {:?}", cloned);

    println!("SecretString memory address: {:p}", &password);
}

fn demonstrate_secret_bytes() {
    println!("\n=== Example 2: SecretBytes Basic Usage ===");

    let api_key = SecretBytes::new(vec![
        0x41, 0x50, 0x49, 0x5f, 0x4b, 0x45, 0x59, 0x5f, 0x53, 0x45, 0x43, 0x52, 0x45, 0x54, 0x5f,
        0x4b,
    ]);

    // Debug output is automatically redacted
    println!("API Key (Debug): {:?}", api_key);

    // Expose byte array
    let exposed = api_key.as_slice();
    println!("API Key (bytes): {:?}", exposed);

    // Convert to hex string
    let hex_string: String = api_key
        .as_slice()
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect();
    println!("API Key (hex): {}", hex_string);

    // Length info
    println!("API Key length: {}", api_key.len());

    // SecretBytes doesn't implement Clone (to prevent bypassing memory protection)
    // If you need the same content, create a new instance
    let api_key_clone = SecretBytes::new(api_key.as_slice().to_vec());
    println!("API Key (new instance): {:?}", api_key_clone);
}

fn demonstrate_encryption_decryption() {
    println!("\n=== Example 3: XChaCha20 Encryption/Decryption ===");

    // Create crypto instance
    let crypto = XChaCha20Crypto::new();

    // Prepare plaintext data (ASCII only for byte string literal)
    let plaintext = b"This is sensitive config data with database password.";

    // Generate 32-byte key
    let key = [
        0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f,
        0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e,
        0x1f, 0x20,
    ];

    // Encrypt: returns random nonce and ciphertext
    let (nonce, ciphertext) = crypto.encrypt(plaintext, &key).expect("Encryption failed");

    println!("Nonce length: {} bytes", nonce.len());
    println!("Plaintext length: {} bytes", plaintext.len());
    println!("Ciphertext length: {} bytes", ciphertext.len());

    // Decrypt
    let decrypted = crypto
        .decrypt(&nonce, &ciphertext, &key)
        .expect("Decryption failed");

    let decrypted_string = String::from_utf8(decrypted).expect("UTF-8 conversion failed");
    println!("Decrypted: {}", decrypted_string);

    // Verify same plaintext produces different ciphertext each time (because nonce is random)
    let (nonce2, _ciphertext2) = crypto.encrypt(plaintext, &key).expect("Encryption failed");
    println!("Same plaintext re-encrypted: nonce different: {:?}", nonce2);

    // Wrong key decryption fails
    let wrong_key = [0x00; 32];
    let result = crypto.decrypt(&nonce, &ciphertext, &wrong_key);
    match result {
        Ok(_) => println!("ERROR: Should have failed!"),
        Err(e) => println!("Correctly failed with wrong key: {:?}", e),
    }
}

fn demonstrate_field_key_derivation() {
    println!("\n=== Example 4: Field-Level Key Derivation ===");

    // Master key (from secure storage)
    let master_key = [
        0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88,
        0x99, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77,
        0x88, 0x99,
    ];

    // Derive independent keys for different fields
    let db_password_key = derive_field_key(&master_key, "database.password", "v1")
        .expect("Failed to derive database password key");
    let api_key_key =
        derive_field_key(&master_key, "api.key", "v1").expect("Failed to derive API key");
    let jwt_secret_key = derive_field_key(&master_key, "api.jwt_secret", "v1")
        .expect("Failed to derive JWT secret key");

    println!("Database password key: {:02x?}", &db_password_key[..8]);
    println!("API key: {:02x?}", &api_key_key[..8]);
    println!("JWT secret key: {:02x?}", &jwt_secret_key[..8]);

    // Verify same input produces same output (deterministic)
    let db_password_key2 =
        derive_field_key(&master_key, "database.password", "v1").expect("Failed to derive key");
    assert_eq!(db_password_key, db_password_key2);
    println!("Same input produces same key: PASS");

    // Verify different fields produce different keys
    assert_ne!(db_password_key, api_key_key);
    println!("Different fields produce different keys: PASS");

    // Verify key rotation (new version produces new key)
    let db_password_key_v2 =
        derive_field_key(&master_key, "database.password", "v2").expect("Failed to derive key");
    assert_ne!(db_password_key, db_password_key_v2);
    println!("Key version change produces new key: PASS");
}

fn demonstrate_env_key_provider() {
    println!("\n=== Example 5: Using EnvKeyProvider ===");

    // Create EnvKeyProvider, reads key from environment variable
    let provider = EnvKeyProvider::new("APP_ENCRYPTION_KEY");

    // Get key
    let key_result = provider.get_key();

    match key_result {
        Ok(key) => {
            println!("Successfully got key from environment variable");
            println!("Key length: {} bytes", key.as_slice().len());
            println!("Key type: {}", provider.provider_type());

            // Use key for encryption
            let crypto = XChaCha20Crypto::new();
            let plaintext = b"Using EnvKeyProvider to protect sensitive data";

            let (nonce, ciphertext) = crypto
                .encrypt(plaintext, key.as_slice())
                .expect("Encryption failed");

            println!(
                "Encryption successful, ciphertext length: {} bytes",
                ciphertext.len()
            );

            // Decrypt to verify
            let decrypted = crypto
                .decrypt(&nonce, &ciphertext, key.as_slice())
                .expect("Decryption failed");

            let decrypted_string = String::from_utf8(decrypted).expect("UTF-8 conversion failed");
            println!("Decryption successful: {}", decrypted_string);
        }
        Err(e) => {
            println!("ERROR: Failed to get key: {:?}", e);
        }
    }

    // Test with builder pattern
    println!("\nUsing builder pattern:");
    let builder_provider = EnvKeyProvider::builder()
        .env_var("APP_ENCRYPTION_KEY")
        .build()
        .expect("Failed to create provider");

    if let Ok(key) = builder_provider.get_key() {
        println!("Builder pattern got key: {} bytes", key.as_slice().len());
    }

    // Clean up environment variable
    std::env::remove_var("APP_ENCRYPTION_KEY");

    // Test when key doesn't exist
    println!("\nTesting non-existent key:");
    let missing_provider = EnvKeyProvider::new("NON_EXISTENT_KEY_12345");
    match missing_provider.get_key() {
        Ok(_) => println!("ERROR: Should have returned error!"),
        Err(e) => println!("Correctly returned error for missing key: {:?}", e),
    }

    // Test with short key
    println!("\nTesting short key:");
    std::env::set_var("SHORT_KEY", "short");
    let short_key_provider = EnvKeyProvider::new("SHORT_KEY");
    match short_key_provider.get_key() {
        Ok(_) => println!("ERROR: Should have returned error!"),
        Err(e) => println!("Correctly returned error for short key: {:?}", e),
    }
    std::env::remove_var("SHORT_KEY");
}

fn demonstrate_config_loading() {
    println!("\n=== Example 6: Basic Config Loading ===");

    // Set the key again for this demo
    std::env::set_var("APP_ENCRYPTION_KEY", "12345678901234567890123456789012");

    // Load config with defaults
    let config = AppConfig::load_sync().expect("Failed to load config");

    println!("App name: {}", config.name);
    println!("App version: {}", config.version);
    println!("Database host: {}", config.db_host);
    println!("Database port: {}", config.db_port);
    println!("Database user: {}", config.db_user);

    println!("Config loading: PASS");
}
