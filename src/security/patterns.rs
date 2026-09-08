// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! # Shared sensitive patterns
//!
//! Canonical source for sensitive data detection patterns and keywords,
//! used across the security module to avoid duplication.

use regex::Regex;
use std::collections::HashSet;
use std::sync::LazyLock;

/// Canonical sensitive detection patterns for matching field names.
///
/// Merged from `config_injector::DEFAULT_SENSITIVE_PATTERNS` and
/// `input_validation::DEFAULT_SENSITIVE_PATTERNS`, deduplicated.
pub(crate) static SENSITIVE_DETECTION_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    vec![
        Regex::new(r"(?i)password").unwrap(),
        Regex::new(r"(?i)secret").unwrap(),
        Regex::new(r"(?i)token").unwrap(),
        Regex::new(r"(?i)api_key").unwrap(),
        Regex::new(r"(?i)access_key").unwrap(),
        Regex::new(r"(?i)access_token").unwrap(),
        Regex::new(r"(?i)refresh_token").unwrap(),
        Regex::new(r"(?i)private_key").unwrap(),
        Regex::new(r"(?i)public_key").unwrap(),
        Regex::new(r"(?i)credential").unwrap(),
        Regex::new(r"(?i)auth").unwrap(),
        Regex::new(r"(?i)key").unwrap(),
        Regex::new(r"(?i)cert").unwrap(),
        Regex::new(r"(?i)password_hash").unwrap(),
        Regex::new(r"(?i)session_id").unwrap(),
        Regex::new(r"(?i)database_url").unwrap(),
        Regex::new(r"(?i)connection_string").unwrap(),
    ]
});

/// Canonical sensitive keywords for high-sensitivity field detection.
///
/// Merged from `input_validation::default_high_sensitivity_keywords` and
/// `error_sanitization::default_keywords`, deduplicated.
pub(crate) static SENSITIVE_KEYWORDS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    let mut set = HashSet::new();
    set.insert("password");
    set.insert("secret");
    set.insert("token");
    set.insert("key");
    set.insert("credential");
    set.insert("auth");
    set.insert("private");
    set.insert("encryption");
    set.insert("private_key");
    set.insert("master_key");
    set.insert("encryption_key");
    set.insert("api_secret");
    set.insert("access_token");
    set.insert("refresh_token");
    set.insert("client_secret");
    set.insert("db_password");
    set.insert("admin_password");
    set
});

/// Check whether `pattern` matches `text` with token boundaries.
///
/// A match is token-bounded when the character immediately before and the
/// character immediately after the match are each either a string boundary or
/// a non-alphanumeric character. The underscore `_` is deliberately treated as
/// a separator here (unlike regex `\b`, where `_` is a word character), so
/// `secret_key` and `my_secret` match the pattern `secret` while `secretary`
/// and `SecretaryOffice` do not.
///
/// Returns `true` if at least one occurrence is token-bounded (any bounded
/// occurrence counts, so real threats are never missed).
pub(crate) fn is_match_with_token_boundary(pattern: &Regex, text: &str) -> bool {
    pattern.find_iter(text).any(|m| {
        let before_ok = text[..m.start()]
            .chars()
            .next_back()
            .is_none_or(|c| !c.is_alphanumeric());
        let after_ok = text[m.end()..]
            .chars()
            .next()
            .is_none_or(|c| !c.is_alphanumeric());
        before_ok && after_ok
    })
}

/// Check whether `text` contains `needle` as a whole token.
///
/// Uses the same token-boundary rule as [`is_match_with_token_boundary`]:
/// the characters adjacent to the occurrence must be string boundaries or
/// non-alphanumeric characters, with `_` treated as a separator. Shared by
/// consumers that detect plain keywords (e.g. `contains_sensitive`) so that
/// `my_key` matches the keyword `key` while `monkey` does not.
pub(crate) fn contains_as_token(text: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return false;
    }
    text.match_indices(needle).any(|(start, matched)| {
        let end = start + matched.len();
        let before_ok = text[..start]
            .chars()
            .next_back()
            .is_none_or(|c| !c.is_alphanumeric());
        let after_ok = text[end..]
            .chars()
            .next()
            .is_none_or(|c| !c.is_alphanumeric());
        before_ok && after_ok
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_boundary_regex_matches() {
        // secret_key / my_secret must hit; secretary must not.
        let secret = Regex::new(r"(?i)secret").unwrap();
        assert!(is_match_with_token_boundary(&secret, "secret_key"));
        assert!(is_match_with_token_boundary(&secret, "my_secret"));
        assert!(is_match_with_token_boundary(&secret, "SECRET"));
        assert!(!is_match_with_token_boundary(&secret, "secretary"));
        assert!(!is_match_with_token_boundary(&secret, "SecretaryOffice"));

        // key: api_key hits, monkey/turkey do not.
        let key = Regex::new(r"(?i)key").unwrap();
        assert!(is_match_with_token_boundary(&key, "api_key"));
        assert!(is_match_with_token_boundary(&key, "KEY"));
        assert!(!is_match_with_token_boundary(&key, "monkey"));
        assert!(!is_match_with_token_boundary(&key, "turkey"));

        // auth: auth_token hits, author does not.
        let auth = Regex::new(r"(?i)auth").unwrap();
        assert!(is_match_with_token_boundary(&auth, "auth_token"));
        assert!(!is_match_with_token_boundary(&auth, "author"));
    }

    #[test]
    fn test_contains_as_token() {
        // Token-bounded occurrences match.
        assert!(contains_as_token("my_key", "key"));
        assert!(contains_as_token("key: value", "key"));
        assert!(contains_as_token("the password was set", "password"));
        // Substrings inside larger words do not match.
        assert!(!contains_as_token("monkey", "key"));
        assert!(!contains_as_token("whiskey", "key"));
        assert!(!contains_as_token("passwords", "password"));
        // Empty needle never matches.
        assert!(!contains_as_token("anything", ""));
    }
}
