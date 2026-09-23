// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! Unified encrypted-value envelope parsing/serialization (T038).
//!
//! Lives at the crate root (not behind the `security` feature) because the
//! load pipeline must detect envelope-shaped values in every configuration.

use base64::Engine as _;

/// Unified encrypted-value envelope (T038).
///
/// Canonical serialization is `enc:v1:<key_version>:<payload>` where
/// `payload` is `base64(nonce || ciphertext)`. The `key_version` field lets
/// key rotation identify which master key decrypts the value; legacy
/// spellings parse compatibly:
///
/// - `enc:v1:<payload>` (doctor format: no key version → defaults to `v1`)
/// - `enc:<payload>` (plain security prefix: no versions at all → `v1`)
///
/// Serialization is always canonical, so newly written values always carry a
/// key version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncryptedEnvelope {
    /// Envelope format version (currently always `1`).
    pub format_version: u8,
    /// Which key version decrypts this value (free-form identifier).
    pub key_version: String,
    /// `base64(nonce || ciphertext)`.
    pub payload: String,
}

impl EncryptedEnvelope {
    /// Envelope format version produced by [`Self::to_envelope_string`].
    pub const FORMAT_VERSION: u8 = 1;
    /// Key version assumed when a legacy spelling omits it.
    pub const DEFAULT_KEY_VERSION: &'static str = "v1";

    /// Wrap a payload under the given key version.
    pub fn new(key_version: impl Into<String>, payload: impl Into<String>) -> Self {
        Self {
            format_version: Self::FORMAT_VERSION,
            key_version: key_version.into(),
            payload: payload.into(),
        }
    }

    /// Canonical serialization: `enc:v1:<key_version>:<payload>`.
    pub fn to_envelope_string(&self) -> String {
        format!(
            "enc:v{}:{}:{}",
            self.format_version, self.key_version, self.payload
        )
    }

    /// Whether `value` looks like an encrypted envelope (canonical or any
    /// legacy spelling). Used for warnings and validation bypass decisions.
    pub fn is_encrypted(value: &str) -> bool {
        value.starts_with("enc:")
    }

    /// Parse the canonical form or any legacy spelling. The payload is
    /// validated as base64.
    pub fn parse(value: &str) -> Option<Self> {
        let rest = value.strip_prefix("enc:")?;
        let segments: Vec<&str> = rest.split(':').collect();
        let (format_version, key_version, payload) = match segments.len() {
            // canonical: v1 <keyver> <payload>
            3 => {
                let version = segments[0].strip_prefix('v')?;
                let version = version.parse::<u8>().ok()?;
                (version, segments[1].to_string(), segments[2])
            }
            // legacy doctor: v1 <payload>
            2 if segments[0].starts_with('v') => {
                let version = segments[0].strip_prefix('v')?;
                let version = version.parse::<u8>().ok()?;
                (version, Self::DEFAULT_KEY_VERSION.to_string(), segments[1])
            }
            // legacy security prefix: <payload>
            1 => (
                Self::FORMAT_VERSION,
                Self::DEFAULT_KEY_VERSION.to_string(),
                segments[0],
            ),
            _ => return None,
        };
        if payload.is_empty() {
            return None;
        }
        // Payload must be valid base64 (nonce || ciphertext).
        base64::engine::general_purpose::STANDARD
            .decode(payload)
            .ok()?;
        Some(Self {
            format_version,
            key_version,
            payload: payload.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t038_envelope_roundtrip_and_legacy_compat() {
        let payload = base64::engine::general_purpose::STANDARD.encode("nonce+ct");
        let canonical = EncryptedEnvelope::new("k2", &payload).to_envelope_string();
        assert_eq!(canonical, format!("enc:v1:k2:{payload}"));
        let parsed = EncryptedEnvelope::parse(&canonical).unwrap();
        assert_eq!(parsed.key_version, "k2");
        assert_eq!(parsed.payload, payload);
        assert_eq!(parsed.format_version, 1);

        // Legacy doctor form enc:v1:<payload>: key version defaults.
        let parsed = EncryptedEnvelope::parse(&format!("enc:v1:{payload}")).unwrap();
        assert_eq!(parsed.key_version, "v1");

        // Legacy security form enc:<payload>: everything defaults.
        let parsed = EncryptedEnvelope::parse(&format!("enc:{payload}")).unwrap();
        assert_eq!(parsed.key_version, "v1");
        assert!(EncryptedEnvelope::is_encrypted(&format!("enc:{payload}")));

        // Junk is rejected.
        assert!(EncryptedEnvelope::parse("enc:!!!not-base64!!!").is_none());
        assert!(EncryptedEnvelope::parse("enc:").is_none());
        assert!(EncryptedEnvelope::parse("plaintext").is_none());
    }
}
