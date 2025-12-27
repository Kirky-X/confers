// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

use confers::encryption::ConfigEncryption;
use confers::utils::path::PathUtils;
use serde::{Deserialize, Serialize};
#[cfg(feature = "audit")]
use serde_json::Value;
use std::env;

// Only compiled if audit feature is on, but we assume it is default or we run with --all-features
#[cfg(feature = "audit")]
use confers::audit::Sanitize;

#[test]
fn test_path_utils() {
    // 测试规范化
    env::set_var("TEST_HOME", "/tmp/test");
    // shellexpand 可能会将 ~ 解析为主目录
    // $TEST_HOME 应该可以正常工作
    let path_str = "$TEST_HOME/config.toml";
    // 我们需要确保在能够正确展开环境变量的系统上运行（Linux 环境）

    let normalized = PathUtils::normalize(path_str).expect("Failed to normalize");
    // 如果路径是相对路径，absolutize 可能会添加当前工作目录，但在此处展开后它是绝对路径
    assert_eq!(normalized.to_str().unwrap(), "/tmp/test/config.toml");

    // 测试安全性
    let unsafe_path = "/etc/passwd";
    assert!(PathUtils::validate_security(std::path::Path::new(unsafe_path)).is_err());

    let safe_path = "/tmp/safe";
    assert!(PathUtils::validate_security(std::path::Path::new(safe_path)).is_ok());
}

#[test]
fn test_encryption_gcm() {
    let key = [1u8; 32];
    let enc = ConfigEncryption::new(key);
    let plaintext = "secret_password";

    let encrypted = enc.encrypt(plaintext).expect("Encrypt failed");
    assert!(encrypted.starts_with("enc:AES256GCM:"));

    let decrypted = enc.decrypt(&encrypted).expect("Decrypt failed");
    assert_eq!(decrypted, plaintext);
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
struct SensitiveConfig {
    api_key: String,
    public_field: String,
}

confers::sanitize_impl_with_sensitive!(SensitiveConfig, {
    api_key => true,
    public_field => false
});

#[test]
#[cfg(feature = "audit")]
fn test_audit_masking() {
    let config = SensitiveConfig {
        api_key: "super_secret".to_string(),
        public_field: "public".to_string(),
    };

    let sanitized = config.sanitize();

    if let Value::Object(map) = sanitized {
        assert_eq!(map["api_key"], Value::String("supe...********".to_string()));
        assert_eq!(map["public_field"], Value::String("public".to_string()));
    } else {
        panic!("Sanitized value should be an object");
    }
}
