// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! 本示例展示 confers 的安全模块功能：
//! - 使用 `EncryptionPrefix` 识别与处理加密值
//! - 展示 `Display` 与 `FromStr` trait 实现
//! - 使用 `EnvSecurityValidator` 验证环境变量名和值
//! - 展示 `EnvironmentValidationConfig` 配置与严格/宽松预设

use confers::security::{EncryptionPrefix, EnvSecurityValidator, EnvironmentValidationConfig};
use std::str::FromStr;

fn main() -> anyhow::Result<()> {
    println!("========================================");
    println!("  Security - security module demo");
    println!("========================================");

    // 1. EncryptionPrefix 基本用法
    println!("\n[EncryptionPrefix basics]");
    let prefix = EncryptionPrefix::Enc;
    println!("  prefix string: {:?}", prefix.as_str());

    let encrypted_value = "enc:SGVsbG8gV29ybGQ=";
    let plain_value = "not-encrypted";
    println!(
        "  value {:?} is encrypted: {}",
        encrypted_value,
        prefix.is_prefixed(encrypted_value)
    );
    println!(
        "  value {:?} is encrypted: {}",
        plain_value,
        prefix.is_prefixed(plain_value)
    );

    // 2. strip 去除前缀
    println!("\n[Strip encryption prefix]");
    match prefix.strip(encrypted_value) {
        Some(content) => println!("  {:?} -> {:?}", encrypted_value, content),
        None => println!("  no prefix"),
    }
    match prefix.strip(plain_value) {
        Some(content) => println!("  {:?} -> {:?}", plain_value, content),
        None => println!("  {:?} no prefix (plaintext)", plain_value),
    }

    // 3. Display 和 FromStr
    println!("\n[Display and FromStr]");
    println!("  Display: {}", prefix);
    match EncryptionPrefix::from_str("enc:") {
        Ok(parsed) => println!("  FromStr(\"enc:\") -> {:?}", parsed),
        Err(()) => println!("  parse failed"),
    }
    match EncryptionPrefix::from_str("invalid") {
        Ok(parsed) => println!("  FromStr(\"invalid\") -> {:?}", parsed),
        Err(()) => println!("  FromStr(\"invalid\") -> parse failed (expected)"),
    }

    // 4. EnvSecurityValidator 默认验证器
    println!("\n[EnvSecurityValidator defaults]");
    let validator = EnvSecurityValidator::default();
    println!(
        "  APP_PORT allowed: {}",
        validator.should_allow_env_var("APP_PORT")
    );
    println!("  PATH allowed: {}", validator.should_allow_env_var("PATH"));
    println!(
        "  api_key (lowercase) allowed: {}",
        validator.should_allow_env_var("api_key")
    );

    // 5. 验证环境变量名
    println!("\n[Validate environment variable names]");
    for name in &[
        "DATABASE_HOST",
        "PATH",
        "SECRET_KEY",
        "app_port",
        "REDIS_PORT",
    ] {
        match validator.validate_env_name(name, None) {
            Ok(()) => println!("  {} -> valid", name),
            Err(e) => println!("  {} -> invalid: {}", name, e),
        }
    }

    // 6. 验证环境变量值
    println!("\n[Validate environment variable values]");
    for value in &[
        "hello",
        "test123",
        "hello;world",
        "hello${world}",
        "hello\0world",
    ] {
        match validator.validate_env_value(value) {
            Ok(()) => println!("  {:?} -> valid", value),
            Err(e) => println!("  {:?} -> invalid: {}", value, e),
        }
    }

    // 7. 加密值绕过验证
    println!("\n[Encrypted value bypasses validation]");
    let encrypted = "enc:SGVsbG8gV29ybGQ=";
    match validator.validate_env_value(encrypted) {
        Ok(()) => println!("  encrypted value passes value validation"),
        Err(e) => println!("  encrypted value rejected: {}", e),
    }
    match validator.validate_env_name("API_KEY", Some(encrypted)) {
        Ok(()) => println!("  API_KEY(encrypted value) passes name validation"),
        Err(e) => println!("  API_KEY rejected: {}", e),
    }

    // 8. 自定义配置
    println!("\n[Custom validation configuration]");
    let config = EnvironmentValidationConfig::new()
        .with_max_name_length(64)
        .with_max_value_length(1024);
    let custom_validator = EnvSecurityValidator::with_config(config);
    println!(
        "  64-character name valid: {}",
        custom_validator
            .validate_env_name(&"A".repeat(64), None)
            .is_ok()
    );
    println!(
        "  65-character name valid: {}",
        custom_validator
            .validate_env_name(&"A".repeat(65), None)
            .is_ok()
    );

    // 9. strict 和 lenient 预设
    println!("\n[Strict and lenient validators]");
    let strict = EnvSecurityValidator::strict();
    let lenient = EnvSecurityValidator::lenient();
    println!(
        "  strict PATH: {}",
        strict.validate_env_name("PATH", None).is_ok()
    );
    println!(
        "  lenient PATH: {}",
        lenient.validate_env_name("PATH", None).is_ok()
    );
    println!(
        "  strict 'hello;world': {}",
        strict.validate_env_value("hello;world").is_ok()
    );
    println!(
        "  lenient 'hello;world': {}",
        lenient.validate_env_value("hello;world").is_ok()
    );

    // 10. 日志脱敏
    println!("\n[Log redaction]");
    let long_value = "x".repeat(150);
    let sanitized = validator.sanitize_for_logging(&long_value);
    println!("  Original length: {}", long_value.len());
    println!("  Redacted length: {} (ends with ...)", sanitized.len());

    println!("\n========================================");
    println!("  Example completed!");
    println!("========================================");
    Ok(())
}
