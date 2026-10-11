// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! 本示例展示 confers 的安全模块功能：
//! - 使用 `EncryptionPrefix` 识别与处理加密值
//! - 展示 `Display` 与 `FromStr` trait 实现
//! - 使用 `EnvSecurityValidator` 验证环境变量名和值
//! - 展示 `EnvironmentValidationConfig` 配置与严格/宽松预设

use confers::i18n::{tr, tr_args};
use confers::security::{EncryptionPrefix, EnvSecurityValidator, EnvironmentValidationConfig};
use std::str::FromStr;

fn main() -> anyhow::Result<()> {
    println!("========================================");
    println!("  {}", tr("example-security-title"));
    println!("========================================");

    println!("\n{}", tr("example-security-encryption-prefix-basics"));
    let prefix = EncryptionPrefix::Enc;
    println!(
        "  {}",
        tr_args(
            "example-security-prefix-string",
            &[("prefix", format!("{:?}", prefix.as_str()))]
        )
    );

    let encrypted_value = "enc:SGVsbG8gV29ybGQ=";
    let plain_value = "not-encrypted";
    println!(
        "  {}",
        tr_args(
            "example-security-value-encrypted",
            &[
                ("value", format!("{encrypted_value:?}")),
                ("encrypted", prefix.is_prefixed(encrypted_value).to_string()),
            ]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-security-value-encrypted",
            &[
                ("value", format!("{plain_value:?}")),
                ("encrypted", prefix.is_prefixed(plain_value).to_string()),
            ]
        )
    );

    println!("\n{}", tr("example-security-strip-prefix"));
    match prefix.strip(encrypted_value) {
        Some(content) => println!("  {:?} -> {:?}", encrypted_value, content),
        None => println!("  {}", tr("example-security-no-prefix")),
    }
    match prefix.strip(plain_value) {
        Some(content) => println!("  {:?} -> {:?}", plain_value, content),
        None => println!(
            "  {}",
            tr_args(
                "example-security-no-prefix-plaintext",
                &[("value", format!("{plain_value:?}"))]
            )
        ),
    }

    println!("\n{}", tr("example-security-display-fromstr"));
    println!("  Display: {}", prefix);
    match EncryptionPrefix::from_str("enc:") {
        Ok(parsed) => println!("  FromStr(\"enc:\") -> {:?}", parsed),
        Err(()) => println!("  {}", tr("example-security-parse-failed")),
    }
    match EncryptionPrefix::from_str("invalid") {
        Ok(parsed) => println!("  FromStr(\"invalid\") -> {:?}", parsed),
        Err(()) => println!("  {}", tr("example-security-parse-failed-expected")),
    }

    println!("\n{}", tr("example-security-validator-defaults"));
    let validator = EnvSecurityValidator::default();
    println!(
        "  {}",
        tr_args(
            "example-security-app-port-allowed",
            &[(
                "allowed",
                validator.should_allow_env_var("APP_PORT").to_string()
            )]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-security-path-allowed",
            &[(
                "allowed",
                validator.should_allow_env_var("PATH").to_string()
            )]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-security-api-key-allowed",
            &[(
                "allowed",
                validator.should_allow_env_var("api_key").to_string()
            )]
        )
    );

    println!("\n{}", tr("example-security-validate-names"));
    for name in &[
        "DATABASE_HOST",
        "PATH",
        "SECRET_KEY",
        "app_port",
        "REDIS_PORT",
    ] {
        match validator.validate_env_name(name, None) {
            Ok(()) => println!(
                "  {}",
                tr_args("example-security-name-valid", &[("name", name.to_string())])
            ),
            Err(e) => println!(
                "  {}",
                tr_args(
                    "example-security-name-invalid",
                    &[("name", name.to_string()), ("error", e.to_string())]
                )
            ),
        }
    }

    println!("\n{}", tr("example-security-validate-values"));
    for value in &[
        "hello",
        "test123",
        "hello;world",
        "hello${world}",
        "hello\0world",
    ] {
        match validator.validate_env_value(value) {
            Ok(()) => println!(
                "  {}",
                tr_args(
                    "example-security-value-valid",
                    &[("value", format!("{value:?}"))]
                )
            ),
            Err(e) => println!(
                "  {}",
                tr_args(
                    "example-security-value-invalid",
                    &[("value", format!("{value:?}")), ("error", e.to_string())]
                )
            ),
        }
    }

    println!("\n{}", tr("example-security-encrypted-bypass"));
    let encrypted = "enc:SGVsbG8gV29ybGQ=";
    match validator.validate_env_value(encrypted) {
        Ok(()) => println!("  {}", tr("example-security-encrypted-value-passes")),
        Err(e) => println!(
            "  {}",
            tr_args(
                "example-security-encrypted-value-rejected",
                &[("error", e.to_string())]
            )
        ),
    }
    match validator.validate_env_name("API_KEY", Some(encrypted)) {
        Ok(()) => println!("  {}", tr("example-security-api-key-passes")),
        Err(e) => println!(
            "  {}",
            tr_args(
                "example-security-api-key-rejected",
                &[("error", e.to_string())]
            )
        ),
    }

    println!("\n{}", tr("example-security-custom-config"));
    let config = EnvironmentValidationConfig::new()
        .with_max_name_length(64)
        .with_max_value_length(1024);
    let custom_validator = EnvSecurityValidator::with_config(config);
    println!(
        "  {}",
        tr_args(
            "example-security-name-64-valid",
            &[(
                "valid",
                custom_validator
                    .validate_env_name(&"A".repeat(64), None)
                    .is_ok()
                    .to_string()
            )]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-security-name-65-valid",
            &[(
                "valid",
                custom_validator
                    .validate_env_name(&"A".repeat(65), None)
                    .is_ok()
                    .to_string()
            )]
        )
    );

    println!("\n{}", tr("example-security-strict-lenient"));
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

    println!("\n{}", tr("example-security-log-redaction"));
    let long_value = "x".repeat(150);
    let sanitized = validator.sanitize_for_logging(&long_value);
    println!(
        "  {}",
        tr_args(
            "example-security-original-length",
            &[("length", long_value.len().to_string())]
        )
    );
    println!(
        "  {}",
        tr_args(
            "example-security-redacted-length",
            &[("length", sanitized.len().to_string())]
        )
    );

    println!("\n========================================");
    println!("  {}", tr("example-security-completed"));
    println!("========================================");
    Ok(())
}
