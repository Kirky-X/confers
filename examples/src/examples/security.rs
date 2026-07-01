//! 本示例展示 confers 的安全模块功能：
//! - 使用 `EncryptionPrefix` 识别与处理加密值
//! - 展示 `Display` 与 `FromStr` trait 实现
//! - 使用 `EnvSecurityValidator` 验证环境变量名和值
//! - 展示 `EnvironmentValidationConfig` 配置与严格/宽松预设

use confers::security::{EncryptionPrefix, EnvSecurityValidator, EnvironmentValidationConfig};
use std::str::FromStr;

fn main() -> anyhow::Result<()> {
    println!("========================================");
    println!("  Security - 安全模块示例");
    println!("========================================");

    // 1. EncryptionPrefix 基本用法
    println!("\n[EncryptionPrefix 基本用法]");
    let prefix = EncryptionPrefix::Enc;
    println!("  前缀字符串: {:?}", prefix.as_str());

    let encrypted_value = "enc:SGVsbG8gV29ybGQ=";
    let plain_value = "not-encrypted";
    println!(
        "  值 {:?} 是否加密: {}",
        encrypted_value,
        prefix.is_prefixed(encrypted_value)
    );
    println!(
        "  值 {:?} 是否加密: {}",
        plain_value,
        prefix.is_prefixed(plain_value)
    );

    // 2. strip 去除前缀
    println!("\n[去除加密前缀]");
    match prefix.strip(encrypted_value) {
        Some(content) => println!("  {:?} -> {:?}", encrypted_value, content),
        None => println!("  无前缀"),
    }
    match prefix.strip(plain_value) {
        Some(content) => println!("  {:?} -> {:?}", plain_value, content),
        None => println!("  {:?} 无前缀（明文）", plain_value),
    }

    // 3. Display 和 FromStr
    println!("\n[Display 和 FromStr]");
    println!("  Display: {}", prefix);
    match EncryptionPrefix::from_str("enc:") {
        Ok(parsed) => println!("  FromStr(\"enc:\") -> {:?}", parsed),
        Err(()) => println!("  解析失败"),
    }
    match EncryptionPrefix::from_str("invalid") {
        Ok(parsed) => println!("  FromStr(\"invalid\") -> {:?}", parsed),
        Err(()) => println!("  FromStr(\"invalid\") -> 解析失败（预期）"),
    }

    // 4. EnvSecurityValidator 默认验证器
    println!("\n[EnvSecurityValidator 默认]");
    let validator = EnvSecurityValidator::default();
    println!(
        "  APP_PORT 允许: {}",
        validator.should_allow_env_var("APP_PORT")
    );
    println!("  PATH 允许: {}", validator.should_allow_env_var("PATH"));
    println!(
        "  api_key (小写) 允许: {}",
        validator.should_allow_env_var("api_key")
    );

    // 5. 验证环境变量名
    println!("\n[验证环境变量名]");
    for name in &[
        "DATABASE_HOST",
        "PATH",
        "SECRET_KEY",
        "app_port",
        "REDIS_PORT",
    ] {
        match validator.validate_env_name(name, None) {
            Ok(()) => println!("  {} -> 有效", name),
            Err(e) => println!("  {} -> 无效: {}", name, e),
        }
    }

    // 6. 验证环境变量值
    println!("\n[验证环境变量值]");
    for value in &[
        "hello",
        "test123",
        "hello;world",
        "hello${world}",
        "hello\0world",
    ] {
        match validator.validate_env_value(value) {
            Ok(()) => println!("  {:?} -> 有效", value),
            Err(e) => println!("  {:?} -> 无效: {}", value, e),
        }
    }

    // 7. 加密值绕过验证
    println!("\n[加密值绕过验证]");
    let encrypted = "enc:SGVsbG8gV29ybGQ=";
    match validator.validate_env_value(encrypted) {
        Ok(()) => println!("  加密值通过值验证"),
        Err(e) => println!("  加密值被拒绝: {}", e),
    }
    match validator.validate_env_name("API_KEY", Some(encrypted)) {
        Ok(()) => println!("  API_KEY（加密值）通过名验证"),
        Err(e) => println!("  API_KEY 被拒绝: {}", e),
    }

    // 8. 自定义配置
    println!("\n[自定义验证配置]");
    let config = EnvironmentValidationConfig::new()
        .with_max_name_length(64)
        .with_max_value_length(1024);
    let custom_validator = EnvSecurityValidator::with_config(config);
    println!(
        "  64 字符名有效: {}",
        custom_validator
            .validate_env_name(&"A".repeat(64), None)
            .is_ok()
    );
    println!(
        "  65 字符名有效: {}",
        custom_validator
            .validate_env_name(&"A".repeat(65), None)
            .is_ok()
    );

    // 9. strict 和 lenient 预设
    println!("\n[严格与宽松验证器]");
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
    println!("\n[日志脱敏]");
    let long_value = "x".repeat(150);
    let sanitized = validator.sanitize_for_logging(&long_value);
    println!("  原始长度: {}", long_value.len());
    println!("  脱敏长度: {} (以 ... 结尾)", sanitized.len());

    println!("\n========================================");
    println!("  示例运行完成!");
    println!("========================================");
    Ok(())
}
