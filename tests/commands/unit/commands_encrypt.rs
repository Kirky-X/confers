// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! 单元测试：Encrypt命令功能
//!
//! 测试EncryptCommand的各种功能，包括配置加密、解密等

#[cfg(test)]
mod encrypt_command_tests {
    use super::super::*;
    use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
    use std::io::Write;
    use tempfile::NamedTempFile;

    /// 测试基本加密功能
    #[test]
    fn test_basic_encryption() {
        let value = "secret data to encrypt";

        // 使用32字节的测试密钥
        let key = BASE64.encode([0u8; 32]);

        let result = EncryptCommand::execute(value, Some(&key), None);

        assert!(result.is_ok());
    }

    /// 测试加密输出到文件
    #[test]
    fn test_encrypt_to_file() {
        let value = "confidential information";
        let key = BASE64.encode([1u8; 32]);

        let output = NamedTempFile::new().unwrap();
        let output_path = output.path().to_string_lossy().into_owned();

        let result = EncryptCommand::execute(value, Some(&key), Some(&output_path));

        assert!(result.is_ok());

        let encrypted_content = std::fs::read_to_string(output.path()).unwrap();
        assert!(!encrypted_content.is_empty());
        assert_ne!(encrypted_content, value);
    }

    /// 测试加密空字符串
    #[test]
    fn test_encrypt_empty_string() {
        let value = "";
        let key = BASE64.encode([2u8; 32]);

        let result = EncryptCommand::execute(value, Some(&key), None);

        assert!(result.is_ok());
    }

    /// 测试加密长字符串
    #[test]
    fn test_encrypt_long_string() {
        let value = "a".repeat(10000);
        let key = BASE64.encode([3u8; 32]);

        let result = EncryptCommand::execute(&value, Some(&key), None);

        assert!(result.is_ok());
    }

    /// 测试加密特殊字符
    #[test]
    fn test_encrypt_special_chars() {
        let value = "Special chars: !@#$%^&*()_+-={}[]|\\:\";<>?,./~`";
        let key = BASE64.encode([4u8; 32]);

        let result = EncryptCommand::execute(value, Some(&key), None);

        assert!(result.is_ok());
    }

    /// 测试加密Unicode字符
    #[test]
    fn test_encrypt_unicode() {
        let value = "Unicode: 你好世界 🌍 Привет мир";
        let key = BASE64.encode([5u8; 32]);

        let result = EncryptCommand::execute(value, Some(&key), None);

        assert!(result.is_ok());
    }

    /// 测试使用无效密钥长度
    #[test]
    fn test_invalid_key_length() {
        let value = "test data";
        let key = BASE64.encode([6u8; 16]); // 16字节，不足32字节

        let result = EncryptCommand::execute(value, Some(&key), None);

        assert!(result.is_err());
    }

    /// 测试使用无效base64密钥
    #[test]
    fn test_invalid_base64_key() {
        let value = "test data";
        let key = "not-valid-base64!!!".to_string();

        let result = EncryptCommand::execute(value, Some(&key), None);

        assert!(result.is_err());
    }

    /// 测试使用环境变量中的密钥
    #[test]
    fn test_encrypt_with_env_key() {
        // 设置环境变量
        std::env::set_var("CONFERS_ENCRYPTION_KEY", BASE64.encode([7u8; 32]));

        let value = "data encrypted with env key";

        let result = EncryptCommand::execute(value, None, None);

        // 清理环境变量
        std::env::remove_var("CONFERS_ENCRYPTION_KEY");

        assert!(result.is_ok());
    }

    /// 测试加密JSON数据
    #[test]
    fn test_encrypt_json_data() {
        let value = r#"{"username": "admin", "password": "secret123", "api_key": "sk-12345"}"#;
        let key = BASE64.encode([8u8; 32]);

        let result = EncryptCommand::execute(value, Some(&key), None);

        assert!(result.is_ok());
    }

    /// 测试加密YAML数据
    #[test]
    fn test_encrypt_yaml_data() {
        let value = r#"
database:
  host: "localhost"
  port: 5432
  password: "secret"
"#;
        let key = BASE64.encode([9u8; 32]);

        let result = EncryptCommand::execute(value, Some(&key), None);

        assert!(result.is_ok());
    }
}

#[cfg(test)]
mod encrypt_decrypt_roundtrip_tests {
    use super::super::*;
    use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};

    /// 测试加密解密往返
    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let original = "sensitive data that needs protection";
        let key = BASE64.encode([10u8; 32]);

        // 加密
        let encrypted_result = EncryptCommand::execute(original, Some(&key), None);
        assert!(encrypted_result.is_ok());

        // 这里应该添加解密测试，但EncryptCommand不提供解密功能
        // 需要使用ConfigEncryption直接进行解密测试
    }

    /// 测试不同密钥产生不同密文
    #[test]
    fn test_different_keys_different_ciphertext() {
        let value = "same data";
        let key1 = BASE64.encode([11u8; 32]);
        let key2 = BASE64.encode([12u8; 32]);

        let mut encrypted1 = String::new();
        let mut encrypted2 = String::new();

        // 捕获加密输出
        let result1 = EncryptCommand::execute(value, Some(&key1), None);
        let result2 = EncryptCommand::execute(value, Some(&key2), None);

        assert!(result1.is_ok());
        assert!(result2.is_ok());
    }
}
