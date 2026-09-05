// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! E2E: 加密与 Secret(tests/e2e/encryption_e2e.rs)
//!
//! 场景固化(docs/TEST_SCENARIOS.md §2.5):
//! - ENC-20 派生宏 `encrypt = "xchacha20"` 字段的加载集成(按真实行为固化,
//!   见下方说明)与真实密钥工作流(env 密钥 → 派生 → 加密 → 配置文件携带密文 → 解密还原)
//!
//! 行为固化说明:`encrypt` 字段属性由宏解析并使字段按 sensitive 处理,
//! 但加载管线当前不自动执行"密文解密注入"(见报告);
//! 解密由使用方以 `EnvKeyProvider`/`derive_field_key`/`XChaCha20Crypto` 显式完成。
//! ENC-01…19/22 已有覆盖(tests/security/encryption.rs、src 内联)。
//! ENC-21(`aes256-gcm` 算法路径)无法测试:宏解析层接受该名称,
//! 但运行时未实现 AES-256-GCM(见报告)。ENC-23(非法算法编译期报错)
//! 固化于 macros crate 的 trybuild 用例。

use base64::Engine;
use confers::secret::{EnvKeyProvider, SecretKeyProvider, XChaCha20Crypto, derive_field_key};
use serial_test::serial;

/// 恰好 32 字节的测试密钥(满足 XChaCha20 256-bit;仅测试用,非真实凭据)。
const TEST_MASTER_KEY: &str = "test-key-with-exactly-32-bytes!!";

/// ENC-20:env 密钥 → 派生字段密钥 → 加密 → 文件携带密文 → 解密还原。
#[test]
#[serial]
fn enc20_encrypted_config_roundtrip_via_env_key() {
    assert_eq!(TEST_MASTER_KEY.len(), 32);

    unsafe { std::env::set_var("CONFERS_E2E_MASTER_KEY", TEST_MASTER_KEY) };

    // 1) 从环境变量取得主密钥(生产者侧)。
    let provider = EnvKeyProvider::new("CONFERS_E2E_MASTER_KEY");
    let master = provider.get_key().expect("env key must resolve");
    assert_eq!(master.len(), 32);

    // 2) 派生字段密钥并加密,密文写入配置文件。
    let field_key =
        derive_field_key(master.as_slice(), "secure_app.api_key", "v1").expect("derive field key");
    let crypto = XChaCha20Crypto::new();
    let (nonce, ciphertext) = crypto
        .encrypt(b"plaintext-secret-for-e2e", &field_key)
        .expect("encrypt");
    let enc_value = format!(
        "enc-{}:{}",
        base64::engine::general_purpose::STANDARD.encode(&nonce),
        base64::engine::general_purpose::STANDARD.encode(&ciphertext),
    );

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("secure.toml");
    std::fs::write(
        &path,
        format!("name = \"app\"\napi_key = \"{enc_value}\"\n"),
    )
    .unwrap();

    // 3) 消费者侧:加载配置拿到密文,再用同一派生密钥解密。
    #[derive(Debug, serde::Deserialize, Default)]
    struct SecureAppConfig {
        name: String,
        api_key: String,
    }
    let cfg: SecureAppConfig = confers::ConfigBuilder::new()
        .allow_absolute_paths()
        .file(&path)
        .build()
        .expect("load secure config");
    assert_eq!(cfg.name, "app");
    assert!(
        cfg.api_key.starts_with("enc-"),
        "ciphertext marker preserved"
    );

    let consumer_key = derive_field_key(master.as_slice(), "secure_app.api_key", "v1")
        .expect("derive same field key");
    let (nonce_b64, ct_b64) = cfg
        .api_key
        .trim_start_matches("enc-")
        .split_once(':')
        .expect("nonce:ciphertext layout");
    let plaintext = crypto
        .decrypt(&decode_b64(nonce_b64), &decode_b64(ct_b64), &consumer_key)
        .expect("decrypt with the same derived key");
    assert_eq!(plaintext, b"plaintext-secret-for-e2e");

    // 4) 错误密钥(不同 field path 派生)→ 解密失败,不泄漏明文。
    let wrong_key = derive_field_key(master.as_slice(), "other.field", "v1").unwrap();
    assert!(
        crypto
            .decrypt(&decode_b64(nonce_b64), &decode_b64(ct_b64), &wrong_key)
            .is_err()
    );

    unsafe { std::env::remove_var("CONFERS_E2E_MASTER_KEY") };
}

fn decode_b64(value: &str) -> Vec<u8> {
    base64::engine::general_purpose::STANDARD
        .decode(value)
        .expect("valid base64")
}
