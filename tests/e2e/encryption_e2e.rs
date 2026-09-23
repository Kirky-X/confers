// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! E2E: `encrypt` 字段属性真实解密(tests/e2e/encryption_e2e.rs)
//!
//! T035 回归:`encrypt = "xchacha20"` 此前只把字段标为敏感、不产生任何
//! 加解密 —— 密文字符串被原样反序列化后「可用」。现在加载管线必须:
//! 统一 envelope 密文经字段派生密钥解密注入(主密钥来自
//! `CONFERS_MASTER_KEY`),明文值原样直通;解密失败该字段反序列化失败。
//!
//! 说明:`encrypt = "aes256-gcm"` 在宏展开期被拒绝(未实现),由
//! macros/tests/compile_fail.rs 的 ENC-21 用例固化。

use base64::Engine as _;
use confers::{
    Config, EncryptedEnvelope,
    secret::{XChaCha20Crypto, derive_field_key},
};
use serial_test::serial;

const MASTER: &[u8] = b"0123456789abcdef0123456789abcdef"; // pragma: allowlist secret

fn make_envelope(field: &str, key_version: &str, plaintext: &str) -> String {
    let field_key = derive_field_key(MASTER, field, key_version).unwrap();
    let (nonce, ct) = XChaCha20Crypto::new()
        .encrypt(plaintext.as_bytes(), field_key.as_slice())
        .unwrap();
    let mut blob = nonce;
    blob.extend_from_slice(&ct);
    EncryptedEnvelope::new(
        key_version,
        base64::engine::general_purpose::STANDARD.encode(&blob),
    )
    .to_envelope_string()
}

#[derive(Debug, Config, serde::Deserialize)]
struct EncryptedProbe {
    pub host: String,

    #[config(encrypt = "xchacha20")]
    pub api_key: String,
}

#[test]
#[serial]
fn enc01_envelope_decrypts_to_plaintext_in_derive_pipeline() {
    let envelope = make_envelope("api_key", "v1", "tok-e2e-123");
    let cwd = std::env::current_dir().unwrap();
    let file = tempfile::Builder::new()
        .prefix("enc01")
        .suffix(".toml")
        .tempfile_in(&cwd)
        .unwrap();
    std::fs::write(
        file.path(),
        format!("host = \"db.internal\"\napi_key = \"{envelope}\"\n"),
    )
    .unwrap();
    let rel = file.path().strip_prefix(&cwd).unwrap_or(file.path());

    // FIXME: Audit that the environment access only happens in single-threaded code.
    unsafe {
        std::env::set_var(
            "CONFERS_MASTER_KEY",
            "3031323334353637383961626364656630313233343536373839616263646566",
        )
    };
    let cfg = EncryptedProbe::load_file(rel).expect("encrypted config must load");
    // FIXME: Audit that the environment access only happens in single-threaded code.
    unsafe { std::env::remove_var("CONFERS_MASTER_KEY") };

    assert_eq!(cfg.host, "db.internal");
    assert_eq!(
        cfg.api_key, "tok-e2e-123",
        "envelope must decrypt to the original plaintext, not the ciphertext string"
    );
}

#[test]
#[serial]
fn enc02_plain_values_pass_through_untouched() {
    let cwd = std::env::current_dir().unwrap();
    let file = tempfile::Builder::new()
        .prefix("enc02")
        .suffix(".toml")
        .tempfile_in(&cwd)
        .unwrap();
    std::fs::write(
        file.path(),
        "host = \"db.internal\"\napi_key = \"plain-token\"\n",
    )
    .unwrap();
    let rel = file.path().strip_prefix(&cwd).unwrap_or(file.path());

    let cfg = EncryptedProbe::load_file(rel).expect("plain config must load");
    assert_eq!(cfg.api_key, "plain-token", "plaintext must pass through");
}

/// R2-M7(主密钥显式注入):ConfigBuilder::master_key 直接注入主密钥,
/// 绕过 CONFERS_MASTER_KEY 环境查找;弱密钥/错密钥照常失败。
#[derive(Debug, confers::Config, serde::Deserialize)]
struct InjectProbe {
    #[serde(rename = "db_addr")]
    #[config(encrypt = "xchacha20")]
    pub db_addr: String,
}

#[test]
#[serial]
fn r2m7_builder_master_key_injection_overrides_env() {
    let envelope = make_envelope("db_addr", "v1", "tok-injected");
    let cwd = std::env::current_dir().unwrap();
    let file = tempfile::Builder::new()
        .prefix("r2m7")
        .suffix(".toml")
        .tempfile_in(&cwd)
        .unwrap();
    std::fs::write(file.path(), format!("db_addr = \"{envelope}\"\n")).unwrap();
    let rel = file
        .path()
        .strip_prefix(&cwd)
        .unwrap_or(file.path())
        .to_path_buf();

    use confers::secret::{XChaCha20Crypto, derive_field_key};
    let field_key = derive_field_key(MASTER, "db_addr", "v1").unwrap();
    let (nonce, ct) = XChaCha20Crypto::new()
        .encrypt(b"ignored", field_key.as_slice())
        .unwrap();
    let _ = (nonce, ct); // 证明确实持有正确字段密钥

    // 环境变量故意设成错误密钥:注入必须优先于环境。
    unsafe { std::env::set_var("CONFERS_MASTER_KEY", hex_encode(&[1u8; 32])) };
    let cfg: InjectProbe = confers::ConfigBuilder::new()
        .file(&rel)
        .encrypted_fields()
        .master_key(MASTER.to_vec())
        .build()
        .expect("injected master key must decrypt");
    unsafe { std::env::remove_var("CONFERS_MASTER_KEY") };
    assert_eq!(cfg.db_addr, "tok-injected");
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
