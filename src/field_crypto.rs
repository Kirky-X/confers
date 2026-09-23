// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! Load-pipeline decryption for `#[config(encrypt = "xchacha20")]` fields.
//!
//! The derive macro registers [`decrypt_encrypted_fields`] as a `map_json`
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
//!   telemetry event and leaves envelope values untouched (T036: the build
//!   must not silently pretend encryption happened).

/// Environment variable carrying the master key: hex-encoded 32 bytes, or a
/// 32-byte ASCII secret (same decoding as the CLI doctor).
pub const MASTER_KEY_ENV: &str = "CONFERS_MASTER_KEY";

/// Decrypt every envelope value found at `keys` in place (see module docs).
pub fn decrypt_encrypted_fields(json: &mut serde_json::Value, keys: &[&str]) {
    for key in keys {
        let Some(value) = crate::tree_transform::get_path(json, key)
            .and_then(serde_json::Value::as_str)
            .map(str::to_string)
        else {
            continue;
        };
        let Some(envelope) = crate::envelope::EncryptedEnvelope::parse(&value) else {
            // Plain value: nothing to do.
            continue;
        };

        #[cfg(feature = "encryption")]
        {
            let Some(master_key) = resolve_master_key() else {
                crate::telemetry::event("confers.encryption.master_key_missing", &[("field", key)]);
                replace_with_null(json, key);
                continue;
            };
            match decrypt_field_value(&envelope, &master_key, key) {
                Ok(plaintext) => {
                    crate::tree_transform::set_path(
                        json,
                        key,
                        serde_json::Value::String(plaintext),
                    );
                }
                Err(reason) => {
                    crate::telemetry::event(
                        "confers.encryption.decrypt_failed",
                        &[("field", key), ("reason", reason)],
                    );
                    // Fail loudly at the field: null fails deserialization
                    // with the field path (serde-path-to_error), never
                    // handing the ciphertext through as the secret.
                    replace_with_null(json, key);
                }
            }
        }

        #[cfg(not(feature = "encryption"))]
        {
            let _ = envelope;
            crate::telemetry::event("confers.encryption.feature_missing", &[("field", key)]);
            // Leave the envelope in place: without the feature the caller
            // opted out of encryption support entirely (T036).
        }
    }
}

#[cfg(feature = "encryption")]
fn replace_with_null(json: &mut serde_json::Value, key: &str) {
    crate::tree_transform::set_path(json, key, serde_json::Value::Null);
}

/// Decode `CONFERS_MASTER_KEY`: hex-encoded 32 bytes or 32-byte ASCII
/// (mirrors the CLI doctor so one env var serves both paths).
#[cfg(feature = "encryption")]
fn resolve_master_key() -> Option<Vec<u8>> {
    let raw = std::env::var(MASTER_KEY_ENV).ok()?;
    decode_master_key_bytes(raw.trim())
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
) -> Result<String, &'static str> {
    use base64::Engine as _;

    let field_key =
        crate::secret::crypto::derive_field_key(master_key, field_path, &envelope.key_version)
            .map_err(|_| "key derivation failed")?;

    let blob = base64::engine::general_purpose::STANDARD
        .decode(&envelope.payload)
        .map_err(|_| "payload is not valid base64")?;
    if blob.len() < 24 {
        return Err("payload too short for a nonce");
    }
    let (nonce, ciphertext) = blob.split_at(24);
    let plaintext = crate::secret::crypto::XChaCha20Crypto::new()
        .decrypt(nonce, ciphertext, field_key.as_slice())
        .map_err(|_| "decryption failed")?;
    // zeroize 1.9 的 Zeroizing 无 into_inner;此处物化为配置值属有界出口
    // (Zeroizing 缓冲在 drop 时仍被清零)。
    String::from_utf8(plaintext.to_vec()).map_err(|_| "plaintext is not valid UTF-8")
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
        decrypt_encrypted_fields(&mut json, &["api_key"]);
        // FIXME: Audit that the environment access only happens in single-threaded code.
        unsafe { std::env::remove_var(MASTER_KEY_ENV) };

        assert_eq!(json["api_key"], "tok-abc123", "envelope must decrypt");
        assert_eq!(json["host"], "db.internal", "plain values stay intact");
    }

    #[test]
    #[serial]
    fn t035_decrypt_failure_fails_loudly_with_null() {
        let master = b"0123456789abcdef0123456789abcdef".to_vec(); // pragma: allowlist secret
        let envelope = make_envelope(&master, "api_key", "v1", "tok-abc123"); // pragma: allowlist secret

        let mut json = serde_json::json!({ "api_key": envelope });
        // Wrong master key: derivation yields a different field key → AEAD
        // open fails → the value must become null (never the ciphertext).
        // FIXME: Audit that the environment access only happens in single-threaded code.
        unsafe { std::env::set_var(MASTER_KEY_ENV, hex_encode(&[9u8; 32])) };
        decrypt_encrypted_fields(&mut json, &["api_key"]);
        // FIXME: Audit that the environment access only happens in single-threaded code.
        unsafe { std::env::remove_var(MASTER_KEY_ENV) };

        assert_eq!(json["api_key"], serde_json::Value::Null);
    }

    #[test]
    #[serial]
    fn t035_missing_master_key_fails_loudly() {
        let envelope = make_envelope(b"0123456789abcdef0123456789abcdef", "pw", "v1", "x");
        let mut json = serde_json::json!({ "pw": envelope });
        // FIXME: Audit that the environment access only happens in single-threaded code.
        unsafe { std::env::remove_var(MASTER_KEY_ENV) };
        decrypt_encrypted_fields(&mut json, &["pw"]);
        assert_eq!(json["pw"], serde_json::Value::Null);
    }

    fn hex_encode(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }
}

#[cfg(all(test, not(feature = "encryption")))]
mod no_feature_tests {
    use super::*;

    #[test]
    fn t036_envelope_left_in_place_without_feature() {
        let mut json = serde_json::json!({ "api_key": "enc:v1:k:YWJj" }); // pragma: allowlist secret
        // Valid base64 payload "abc".
        decrypt_encrypted_fields(&mut json, &["api_key"]);
        assert_eq!(
            json["api_key"], "enc:v1:k:YWJj",
            "without the encryption feature the envelope must be left untouched (warn only)"
        );
    }
}
