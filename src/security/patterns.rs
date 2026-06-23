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
