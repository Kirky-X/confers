// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! Load-pipeline decryption for `#[config(encrypt = "xchacha20")]` fields.
//!
//! The derive macro registers [`decrypt_encrypted_tree`] as a `map_json`
//! transform for every encrypt-marked field. The transform runs after the
//! other tree transforms (rename/flatten/interpolate) so it observes the
//! serde-normalized merge tree.
//!
//! Semantics:
//! - A value matching the unified envelope ([`crate::security::
//!   EncryptedEnvelope`]) is decrypted with the field-derived key and
//!   replaced with the plaintext string.
//! - A value that is NOT an envelope passes through untouched (plain-text
//!   configuration stays valid).
//! - Decryption failure replaces the value with `null` so deserialization
//!   fails loudly naming the field (never silently hands out the raw
//!   ciphertext as if it were the secret), and a telemetry event records the
//!   failure.
//! - Without the `encryption` feature the transform only emits a warning
//!   telemetry event and leaves envelope values untouched (the build
//!   must not silently pretend encryption happened).

/// Environment variable carrying the master key: hex-encoded 32 bytes, or a
/// 32-byte ASCII secret (same decoding as the CLI doctor).
pub const MASTER_KEY_ENV: &str = "CONFERS_MASTER_KEY";

/// Decrypt every envelope value found anywhere in the tree, in place
/// (see module docs).
///
/// The walk is whole-tree: nested/flatten-hoisted fields participate too —
/// the derivation path is the value's full dotted path in the serde merge
/// space (e.g. `inner.api_key` for a hoisted flatten field), which must match
/// the path the encrypting side used.
pub fn decrypt_encrypted_tree(json: &mut serde_json::Value) {
    decrypt_tree_with_key(json, None);
}

/// Builder-integrated entry point (R2-M7): runs the tree walk with the
/// builder's injected master key when present, else the environment default.
#[allow(dead_code)]
pub(crate) fn apply_field_decryption(json: &mut serde_json::Value, master_key: Option<&[u8]>) {
    decrypt_tree_with_key(json, master_key);
}

fn decrypt_tree_with_key(json: &mut serde_json::Value, master_override: Option<&[u8]>) {
    match json {
        serde_json::Value::Object(map) => {
            for (k, v) in map.iter_mut() {
                let child_path = k.clone();
                decrypt_at_path(v, &child_path, master_override);
            }
        }
        serde_json::Value::Array(items) => {
            for (idx, v) in items.iter_mut().enumerate() {
                decrypt_at_path(v, &idx.to_string(), master_override);
            }
        }
        _ => {}
    }
}

fn decrypt_at_path(value: &mut serde_json::Value, path: &str, master_override: Option<&[u8]>) {
    match value {
        serde_json::Value::Object(map) => {
            for (k, v) in map.iter_mut() {
                let child_path = if path.is_empty() {
                    k.clone()
                } else {
                    format!("{path}.{k}")
                };
                decrypt_at_path(v, &child_path, master_override);
            }
        }
        serde_json::Value::Array(items) => {
            for (idx, v) in items.iter_mut().enumerate() {
                let child_path = format!("{path}.{idx}");
                decrypt_at_path(v, &child_path, master_override);
            }
        }
        serde_json::Value::String(text) => {
            let Some(envelope) = crate::envelope::EncryptedEnvelope::parse(text) else {
                // Plain value, or a malformed `enc:`-prefixed string.
                if crate::envelope::EncryptedEnvelope::is_encrypted(text) {
                    // Looks encrypted but does not parse (bad key version,
                    // non-standard base64, ...): never hand it through as
                    // the secret — fail loudly at the field (M4).
                    crate::telemetry::warn(
                        "confers.encryption.envelope_malformed",
                        &[("field", path)],
                    );
                    *value = decrypt_failure_marker(&crate::i18n::tr(
                        "error-decrypt-malformed-envelope",
                    ));
                }
                return;
            };

            #[cfg(feature = "encryption")]
            {
                // 密钥解析顺序:显式注入(builder master_key)>环境变量;
                // 注入键同样过 32 字节 + 弱密钥校验。
                let resolved: Option<Vec<u8>> = match master_override {
                    Some(k) if k.len() == 32 && !crate::secret::crypto::is_weak_key(k) => {
                        Some(k.to_vec())
                    }
                    Some(_) => None,
                    None => resolve_master_key(),
                };
                let Some(master_key) = resolved else {
                    crate::telemetry::warn(
                        "confers.encryption.master_key_missing",
                        &[("field", path)],
                    );
                    *value = decrypt_failure_marker(&crate::i18n::tr(
                        "error-decrypt-master-key-missing",
                    ));
                    return;
                };
                match decrypt_field_value(&envelope, &master_key, path) {
                    Ok(plaintext) => {
                        *value = serde_json::Value::String(plaintext);
                    }
                    Err(reason) => {
                        crate::telemetry::warn(
                            "confers.encryption.decrypt_failed",
                            &[("field", path), ("reason", reason.as_str())],
                        );
                        // Fail loudly at the field: the marker fails
                        // deserialization for String AND Option<String>
                        // (null would silently wash the failure into None),
                        // and the ciphertext is never handed through as the
                        // secret.
                        *value = decrypt_failure_marker(&reason);
                    }
                }
            }

            #[cfg(not(feature = "encryption"))]
            {
                let _ = (&envelope, master_override);
                crate::telemetry::warn("confers.encryption.feature_missing", &[("field", path)]);
                // Leave the envelope in place: without the feature the caller
                // opted out of encryption support entirely.
            }
        }
        _ => {
            let _ = value;
        }
    }
}

/// Marker value substituted for a failed decryption: a map fails
/// deserialization for string-shaped fields (with the field path from
/// serde-path-to_error), so the failure can never be silently swallowed —
/// including by `Option<String>` fields.
fn decrypt_failure_marker(reason: &str) -> serde_json::Value {
    serde_json::json!({ "__confers_decrypt_failed": reason })
}

/// Decode `CONFERS_MASTER_KEY`: hex-encoded 32 bytes or 32-byte ASCII
/// (mirrors the CLI doctor so one env var serves both paths).
#[cfg(feature = "encryption")]
fn resolve_master_key() -> Option<Vec<u8>> {
    let raw = std::env::var(MASTER_KEY_ENV).ok()?;
    let key = decode_master_key_bytes(raw.trim())?;
    // 与 EnvKeyProvider 同源的弱密钥防护 —— 常量字节主密钥不因走了
    // 加载管线而被绕过。
    if crate::secret::crypto::is_weak_key(&key) {
        return None;
    }
    Some(key)
}

#[cfg(feature = "encryption")]
fn decode_master_key_bytes(raw: &str) -> Option<Vec<u8>> {
    if raw.len().is_multiple_of(2)
        && let Ok(bytes) = hex_decode(raw)
        && bytes.len() == 32
    {
        return Some(bytes);
    }
    if raw.len() == 32 {
        return Some(raw.as_bytes().to_vec());
    }
    None
}

#[cfg(feature = "encryption")]
fn hex_decode(input: &str) -> Result<Vec<u8>, ()> {
    fn nibble(c: u8) -> Result<u8, ()> {
        match c {
            b'0'..=b'9' => Ok(c - b'0'),
            b'a'..=b'f' => Ok(c - b'a' + 10),
            b'A'..=b'F' => Ok(c - b'A' + 10),
            _ => Err(()),
        }
    }
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len() / 2);
    for pair in bytes.chunks_exact(2) {
        let hi = nibble(pair[0])?;
        let lo = nibble(pair[1])?;
        out.push((hi << 4) | lo);
    }
    Ok(out)
}

/// Decrypt one field envelope: derive the per-field key from the master key
/// and the envelope's key version, then XChaCha20-Poly1305 open the payload
/// (nonce || ciphertext, empty AAD — matching the CLI doctor convention).
#[cfg(feature = "encryption")]
fn decrypt_field_value(
    envelope: &crate::envelope::EncryptedEnvelope,
    master_key: &[u8],
    field_path: &str,
) -> Result<String, String> {
    use base64::Engine as _;

    let field_key =
        crate::secret::crypto::derive_field_key(master_key, field_path, &envelope.key_version)
            .map_err(|_| crate::i18n::tr("error-decrypt-key-derivation-failed"))?;

    let blob = base64::engine::general_purpose::STANDARD
        .decode(&envelope.payload)
        .map_err(|_| crate::i18n::tr("error-decrypt-payload-not-base64"))?;
    if blob.len() < 24 {
        return Err(crate::i18n::tr("error-decrypt-payload-too-short"));
    }
    let (nonce, ciphertext) = blob.split_at(24);
    let plaintext = crate::secret::crypto::XChaCha20Crypto::new()
        .decrypt(nonce, ciphertext, field_key.as_slice())
        .map_err(|_| crate::i18n::tr("error-decrypt-open-failed"))?;
    // zeroize 1.9 的 Zeroizing 无 into_inner;此处物化为配置值属有界出口
    // (Zeroizing 缓冲在 drop 时仍被清零)。
    String::from_utf8(plaintext.to_vec())
        .map_err(|_| crate::i18n::tr("error-decrypt-plaintext-not-utf8"))
}

#[cfg(all(test, feature = "encryption"))]
mod tests {
    use super::*;
    use base64::Engine as _;
    use serial_test::serial;

    /// Build a canonical envelope string the way an operator would after
    /// encrypting a value offline (or via the doctor): field-derived key,
    /// random nonce, XChaCha20-Poly1305.
    fn make_envelope(master: &[u8], field: &str, key_version: &str, plaintext: &str) -> String {
        let field_key =
            crate::secret::crypto::derive_field_key(master, field, key_version).unwrap();
        let (nonce, ct) = crate::secret::crypto::XChaCha20Crypto::new()
            .encrypt(plaintext.as_bytes(), field_key.as_slice())
            .unwrap();
        let mut blob = nonce;
        blob.extend_from_slice(&ct);
        crate::envelope::EncryptedEnvelope::new(
            key_version,
            base64::engine::general_purpose::STANDARD.encode(&blob),
        )
        .to_envelope_string()
    }

    #[test]
    #[serial]
    fn t035_decrypts_envelope_and_keeps_plaintext() {
        let master = b"0123456789abcdef0123456789abcdef".to_vec(); // pragma: allowlist secret
        let envelope = make_envelope(&master, "api_key", "v1", "tok-abc123"); // pragma: allowlist secret

        let mut json = serde_json::json!({
            "api_key": envelope,
            "host": "db.internal",
        });
        // FIXME: Audit that the environment access only happens in single-threaded code.
        unsafe { std::env::set_var(MASTER_KEY_ENV, hex_encode(&master)) };
        decrypt_encrypted_tree(&mut json);
        // FIXME: Audit that the environment access only happens in single-threaded code.
        unsafe { std::env::remove_var(MASTER_KEY_ENV) };

        assert_eq!(json["api_key"], "tok-abc123", "envelope must decrypt");
        assert_eq!(json["host"], "db.internal", "plain values stay intact");
    }

    #[test]
    #[serial]
    fn t035_decrypt_failure_fails_loudly_with_marker() {
        let master = b"0123456789abcdef0123456789abcdef".to_vec(); // pragma: allowlist secret
        let envelope = make_envelope(&master, "api_key", "v1", "tok-abc123"); // pragma: allowlist secret

        let mut json = serde_json::json!({ "api_key": envelope });
        // Wrong master key: derivation yields a different field key → AEAD
        // open fails → the value must become the failure marker (never the
        // ciphertext, never null — null would wash the failure into None for
        // Option<String> fields).
        // FIXME: Audit that the environment access only happens in single-threaded code.
        unsafe { std::env::set_var(MASTER_KEY_ENV, hex_encode(&[9u8; 32])) };
        decrypt_encrypted_tree(&mut json);
        // FIXME: Audit that the environment access only happens in single-threaded code.
        unsafe { std::env::remove_var(MASTER_KEY_ENV) };

        assert!(
            json["api_key"].get("__confers_decrypt_failed").is_some(),
            "failed decryption must leave the loud marker, got {}",
            json["api_key"]
        );
    }

    #[test]
    #[serial]
    fn t035_missing_master_key_fails_loudly() {
        let envelope = make_envelope(b"0123456789abcdef0123456789abcdef", "pw", "v1", "x");
        let mut json = serde_json::json!({ "pw": envelope });
        // FIXME: Audit that the environment access only happens in single-threaded code.
        unsafe { std::env::remove_var(MASTER_KEY_ENV) };
        decrypt_encrypted_tree(&mut json);
        assert!(
            json["pw"].get("__confers_decrypt_failed").is_some(),
            "missing master key must leave the loud marker"
        );
    }

    fn hex_encode(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }

    #[test]
    #[serial]
    fn arrays_and_nested_maps_are_traversed_and_decrypted() {
        let master = b"0123456789abcdef0123456789abcdef".to_vec(); // pragma: allowlist secret
        unsafe { std::env::set_var(MASTER_KEY_ENV, hex_encode(&master)) };

        let nested_pw = make_envelope(&master, "db.creds.pw", "v1", "nested-secret");
        let list_pw = make_envelope(&master, "servers.0.pw", "v1", "list-secret");

        let mut json = serde_json::json!({
            "db": { "creds": { "pw": nested_pw } },
            "servers": [ { "pw": list_pw }, { "pw": "plain" } ],
        });
        decrypt_encrypted_tree(&mut json);
        unsafe { std::env::remove_var(MASTER_KEY_ENV) };

        assert_eq!(
            json["db"]["creds"]["pw"], "nested-secret",
            "nested map paths decrypt with the derived field key"
        );
        assert_eq!(
            json["servers"][0]["pw"], "list-secret",
            "array elements are addressed by index path"
        );
        assert_eq!(json["servers"][1]["pw"], "plain");
    }

    #[test]
    #[serial]
    fn malformed_envelopes_fail_loudly_instead_of_passing_through() {
        unsafe { std::env::remove_var(MASTER_KEY_ENV) };

        // `enc:`-prefixed but unparsable (bad key version / non-base64).
        let mut json = serde_json::json!({ "api_key": "enc:v9:not-base64!!" }); // pragma: allowlist secret
        decrypt_encrypted_tree(&mut json);
        assert!(
            json["api_key"].get("__confers_decrypt_failed").is_some(),
            "malformed envelope must become the marker, got {}",
            json["api_key"]
        );
    }

    #[test]
    #[serial]
    fn undecodable_payload_variants_fail_with_distinct_reasons() {
        let master = b"0123456789abcdef0123456789abcdef".to_vec(); // pragma: allowlist secret
        unsafe { std::env::set_var(MASTER_KEY_ENV, hex_encode(&master)) };

        // Payload is not valid base64.
        let mut bad_b64 = serde_json::json!({ "pw": "enc:v1:v1:!!!not-base64!!!" });
        decrypt_encrypted_tree(&mut bad_b64);
        assert!(bad_b64["pw"].get("__confers_decrypt_failed").is_some());

        // Payload decodes but is shorter than the 24-byte nonce.
        let short = crate::envelope::EncryptedEnvelope::new(
            "v1",
            base64::engine::general_purpose::STANDARD.encode(b"short"),
        )
        .to_envelope_string();
        let mut too_short = serde_json::json!({ "pw": short });
        decrypt_encrypted_tree(&mut too_short);
        assert!(too_short["pw"].get("__confers_decrypt_failed").is_some());

        unsafe { std::env::remove_var(MASTER_KEY_ENV) };
    }

    #[test]
    #[serial]
    fn injected_master_key_beats_env_and_weak_keys_are_rejected() {
        let env_master = b"0123456789abcdef0123456789abcdef".to_vec(); // pragma: allowlist secret
        let injected = b"fedcba9876543210fedcba9876543210".to_vec(); // pragma: allowlist secret
        unsafe { std::env::set_var(MASTER_KEY_ENV, hex_encode(&env_master)) };

        // 用注入密钥加密:即便 env 里是另一把钥匙,注入键必须获胜。
        let envelope = make_envelope(&injected, "pw", "v1", "injected-wins");
        let mut json = serde_json::json!({ "pw": envelope });
        apply_field_decryption(&mut json, Some(&injected));
        unsafe { std::env::remove_var(MASTER_KEY_ENV) };
        assert_eq!(json["pw"], "injected-wins");

        // 注入弱密钥(全零):回落到 env 解析,env 键解不开 → 失败标记。
        let envelope2 = make_envelope(&injected, "pw2", "v1", "x");
        let mut weak = serde_json::json!({ "pw2": envelope2 });
        apply_field_decryption(&mut weak, Some(&[0u8; 32]));
        assert!(weak["pw2"].get("__confers_decrypt_failed").is_some());

        // 注入长度错误的密钥:同样回落,env 键解不开 → 失败标记。
        let envelope3 = make_envelope(&injected, "pw3", "v1", "y");
        let mut bad_len = serde_json::json!({ "pw3": envelope3 });
        apply_field_decryption(&mut bad_len, Some(&[1u8; 16]));
        assert!(bad_len["pw3"].get("__confers_decrypt_failed").is_some());
    }
}

#[cfg(all(test, not(feature = "encryption")))]
mod no_feature_tests {
    use super::*;

    #[test]
    fn t036_envelope_left_in_place_without_feature() {
        let mut json = serde_json::json!({ "api_key": "enc:v1:k:YWJj" }); // pragma: allowlist secret
        // Valid base64 payload "abc".
        decrypt_encrypted_tree(&mut json);
        assert_eq!(
            json["api_key"], "enc:v1:k:YWJj",
            "without the encryption feature the envelope must be left untouched (warn only)"
        );
    }
}
