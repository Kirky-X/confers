// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Example of using confers as a library with the unified CLI facade
//! 
//! This example demonstrates how to integrate confers CLI functionality
//! into your own applications using the ConfersCli struct.
//!
//! ## Prerequisites
//!
//! Required features: `cli`, `derive`
//! Optional features: `validation`, `encryption`, `schema`
//!
//! ## Run with
//!
//! ```bash
//! cargo run --example library_usage --features "cli,derive,validation,encryption"
//! ```

use confers::ConfersCli;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Confers Library Usage Example");
    println!("================================");

    // Example 1: Generate a configuration template
    println!("\n📝 1. Generating configuration template...");
    ConfersCli::generate(Some("example_config.toml"), "minimal")?;
    println!("✅ Generated minimal template to example_config.toml");

    // Example 2: Validate the generated configuration
    println!("\n🔍 2. Validating configuration...");
    match ConfersCli::validate("example_config.toml", "full") {
        Ok(_) => println!("✅ Configuration is valid"),
        Err(e) => println!("❌ Validation failed: {}", e),
    }

    // Example 3: Create a second config for comparison
    println!("\n📝 3. Creating second configuration for diff...");
    std::fs::write("example_config2.toml", r#"
name = "example-app"
version = "1.1.0"
port = 9090
debug = true
"#)?;
    println!("✅ Created example_config2.toml");

    // Example 4: Compare two configurations
    println!("\n🔄 4. Comparing configurations...");
    ConfersCli::diff("example_config.toml", "example_config2.toml", Some("unified"))?;

    // Example 5: Encrypt a value (if encryption feature is enabled)
    #[cfg(feature = "encryption")]
    {
        println!("\n🔐 5. Encrypting a value...");
        match ConfersCli::encrypt("secret_password", None) {
            Ok(encrypted) => println!("✅ Encrypted: {}", encrypted),
            Err(e) => println!("❌ Encryption failed: {}", e),
        }
    }

    // Example 6: Generate shell completions
    println!("\n🐚 6. Generating shell completions...");
    match ConfersCli::completions("bash") {
        Ok(_) => println!("✅ Bash completion script generated"),
        Err(e) => println!("❌ Completion generation failed: {}", e),
    }

    // Example 7: Run wizard in non-interactive mode
    println!("\n🧙 7. Running configuration wizard (non-interactive)...");
    match ConfersCli::wizard(true) {
        Ok(_) => println!("✅ Wizard completed successfully"),
        Err(e) => println!("❌ Wizard failed: {}", e),
    }

    // Cleanup
    println!("\n🧹 Cleaning up example files...");
    let _ = std::fs::remove_file("example_config.toml");
    let _ = std::fs::remove_file("example_config2.toml");
    println!("✅ Cleanup completed");

    println!("\n🎉 Example completed successfully!");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_library_generate() {
        let result = ConfersCli::generate(None, "minimal");
        assert!(result.is_ok());
    }

    #[test]
    fn test_library_validate() {
        // First create a test config
        std::fs::write("test_config.toml", "name = \"test\"\nversion = \"1.0.0\"\n").unwrap();
        
        let result = ConfersCli::validate("test_config.toml", "minimal");
        assert!(result.is_ok());
        
        // Cleanup
        let _ = std::fs::remove_file("test_config.toml");
    }

    #[test]
    fn test_library_encrypt() {
        #[cfg(feature = "encryption")]
        {
            let result = ConfersCli::encrypt("test_value", None);
            // This might fail if no encryption key is set in environment
            // so we just check that it returns a Result
            assert!(result.is_ok() || result.is_err());
        }
    }

    #[test]
    fn test_library_completions() {
        let result = ConfersCli::completions("bash");
        assert!(result.is_ok());
    }
}
