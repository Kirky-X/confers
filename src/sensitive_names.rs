// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! Canonical sensitive-name matching.
//!
//! Single source of truth for "is this configuration key name sensitive?"
//! decisions used by the CLI redaction, audit-log key redaction, and the
//! security module's pattern matching. Lives outside the feature-gated
//! `security` module so every feature configuration gets identical behavior.
//!
//! Matching rules:
//! - Names are compared lowercased with `-`, ` `, and `.` treated like `_`.
//! - Plural forms match (`passwords`, `tokens`, `keys`, `credentials`).
//! - Substring matching uses token boundaries so `monkey` does not match
//!   `key`, but `api_key`, `db.password`, and `authorization` all match.

/// Keyword fragments that mark a key name as sensitive.
const SENSITIVE_FRAGMENTS: &[&str] = &[
    "password",
    "passwd",
    "pwd",
    "secret",
    "token",
    "api_key",
    "access_key",
    "private_key",
    "secret_key",
    "public_key",
    "credential",
    "authorization",
    "auth",
    "bearer",
    "dsn",
    "connection_string",
    "database_url",
    "session_id",
    "cert",
    "signing_key",
    "encryption_key",
    "master_key",
];

/// Whether a configuration key name (last path segment, or the whole name)
/// should be treated as sensitive for masking/redaction decisions.
///
/// The name is normalized (`-`/`.`/space → `_`, lowercased) and matched
/// against [`SENSITIVE_FRAGMENTS`] with token boundaries: a fragment hit must
/// not be glued to other letters (`monkey` stays clean, `my_api_key` hits).
pub fn is_sensitive_name(name: &str) -> bool {
    let normalized: String = name
        .chars()
        .map(|c| match c {
            '-' | '.' | ' ' => '_',
            c => c.to_ascii_lowercase(),
        })
        .collect();

    SENSITIVE_FRAGMENTS
        .iter()
        .any(|frag| token_contains(&normalized, frag))
}

/// Boundary-aware substring test: `frag` matches when every side of the hit
/// is a token boundary (start/end of string or a non-alphanumeric char).
/// A trailing `s` on the haystack side is allowed so plurals
/// (`passwords`/`tokens`/`keys`) match their singular fragments.
fn token_contains(haystack: &str, frag: &str) -> bool {
    let hay = haystack.as_bytes();
    let needle = frag.as_bytes();
    if needle.is_empty() || hay.len() < needle.len() {
        return false;
    }
    for start in 0..=(hay.len() - needle.len()) {
        if &hay[start..start + needle.len()] != needle {
            continue;
        }
        let before_ok = start == 0 || !hay[start - 1].is_ascii_alphanumeric();
        // Accept one trailing 's' (plural) inside the haystack window.
        let after = start + needle.len();
        let after_ok = if after >= hay.len() {
            true
        } else if hay[after] == b's' {
            after + 1 == hay.len() || !hay[after + 1].is_ascii_alphanumeric()
        } else {
            !hay[after].is_ascii_alphanumeric()
        };
        if before_ok && after_ok {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sensitive_names_match() {
        for name in [
            "password",
            "PASSWORD",
            "db.password",
            "passwords",
            "passwd",
            "pwd",
            "api_key",
            "API-KEY",
            "access_token",
            "tokens",
            "private_key",
            "credentials",
            "authorization",
            "Authorization",
            "bearer",
            "dsn",
            "connection_string",
            "database_url",
            "session_id",
            "client_secret",
            "my.api.key",
        ] {
            assert!(is_sensitive_name(name), "{name} must be sensitive");
        }
    }

    #[test]
    fn benign_names_do_not_match() {
        for name in ["host", "port", "monkey", "keyboard", "timeout", "name"] {
            assert!(!is_sensitive_name(name), "{name} must NOT be sensitive");
        }
    }

    #[test]
    fn fragment_compound_names_fail_safe_to_sensitive() {
        // Boundary rules deliberately over-match compound words such as
        // `token_bucket_limit` or `password_policy_doc`. For diagnostic
        // redaction a false positive is acceptable (fail-safe); consistency
        // with security::patterns matters more.
        for name in [
            "token_bucket_limit",
            "password_policy_doc",
            "cert_file_note_placeholder",
        ] {
            assert!(is_sensitive_name(name), "{name} contains a fragment");
        }
    }
}
