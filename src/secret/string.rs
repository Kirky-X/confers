// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

use std::fmt::Debug;
use zeroize::{Zeroize, Zeroizing};

/// A heap-allocated string whose contents are zeroized when the value is
/// dropped (or when [`Zeroize::zeroize`] is called explicitly).
///
/// # Security model
///
/// The internal buffer is owned and scrubbed on drop, and [`Debug`] and
/// [`std::fmt::Display`] are intentionally redacted (`[REDACTED]`) so the secret cannot
/// leak through formatting, logging, or `to_string()`. Unlike
/// [`SecretBytes`](crate::secret::SecretBytes), this type **does** implement
/// [`Clone`]: cloning is safe with respect to zeroization (each clone owns
/// and scrubs its own buffer), but it also means copies of the secret can
/// easily proliferate.
///
/// # WARNING: unmanaged plaintext escape hatches
///
/// [`std::ops::Deref`] (to [`str`]) and [`SecretString::expose`] hand out the plaintext
/// as ordinary, **unmanaged** Rust data, and [`SecretString::expose_clone`]
/// duplicates it while both copies are alive. Anything derived from those
/// values (`&str` copies, slices, substrings, formatted output) is *not*
/// zeroized and may linger in memory well past the lifetime of this value.
/// Treat these methods as deliberate, auditable disclosures:
///
/// - Prefer [`SecretString::expose`] for the narrowest possible borrow, and
///   drop the derived data as soon as possible.
/// - Use [`SecretString::expose_clone`] only when an owned copy is strictly
///   required (for example to hand the secret to an API that takes `String`
///   by value). The copy itself is zeroized on drop, but data derived from
///   it is not.
/// - Never let formatted output (`format!`, `.to_string()`) or `&str`/`String`
///   copies outlive the immediate need.
#[derive(Clone)]
pub struct SecretString(Zeroizing<String>);

impl Default for SecretString {
    fn default() -> Self {
        Self::new(String::new())
    }
}

impl SecretString {
    pub fn new(s: impl Into<String>) -> Self {
        Self(Zeroizing::new(s.into()))
    }

    pub fn expose(&self) -> &str {
        self.0.as_str()
    }

    /// Returns a managed clone of the secret, zeroized when dropped.
    ///
    /// The returned [`Zeroizing<String>`] scrubs its buffer when it goes out
    /// of scope, so the owned copy does not linger in memory the way a plain
    /// `String` would. Note that the plaintext is still duplicated while both
    /// values are alive, and anything derived from the copy (`&str`/`String`
    /// copies, slices, formatted output) is *not* managed — keep such
    /// derivatives short-lived.
    pub fn expose_clone(&self) -> Zeroizing<String> {
        self.0.clone()
    }
}

impl Debug for SecretString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[REDACTED]")
    }
}

/// Display is intentionally redacted to prevent accidental secret leakage
/// via `format!("{}", s)` or `.to_string()`.
impl std::fmt::Display for SecretString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[REDACTED]")
    }
}

impl std::ops::Deref for SecretString {
    type Target = str;

    fn deref(&self) -> &str {
        self.0.as_str()
    }
}

impl Zeroize for SecretString {
    fn zeroize(&mut self) {
        self.0.zeroize();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_from_string() {
        let s = SecretString::new(String::from("hello"));
        assert_eq!(s.expose(), "hello");
    }

    #[test]
    fn test_new_from_str_literal() {
        let s = SecretString::new("world");
        assert_eq!(s.expose(), "world");
    }

    #[test]
    fn test_new_from_owned_string() {
        let owned = String::from("owned-value");
        let s = SecretString::new(owned);
        assert_eq!(s.expose(), "owned-value");
    }

    #[test]
    fn test_default_is_empty_string() {
        let s = SecretString::default();
        assert!(s.expose().is_empty());
        assert_eq!(s.expose(), "");
    }

    #[test]
    fn test_expose_returns_str_reference() {
        let s = SecretString::new("reference");
        let r: &str = s.expose();
        assert_eq!(r, "reference");
    }

    #[test]
    fn test_expose_clone_returns_zeroizing_string() {
        let s = SecretString::new("clone-me");
        let cloned = s.expose_clone();
        assert_eq!(cloned.as_str(), "clone-me");
        // Original is still accessible
        assert_eq!(s.expose(), "clone-me");
    }

    #[test]
    fn test_expose_clone_is_independent_of_original() {
        let s = SecretString::new("independent");
        let mut cloned = s.expose_clone();
        cloned.push_str("-mutated");
        // Mutating the clone must not affect the original
        assert_eq!(s.expose(), "independent");
        assert_eq!(cloned.as_str(), "independent-mutated");
    }

    #[test]
    fn test_debug_does_not_leak_secret() {
        let s = SecretString::new("super-secret-token");
        let debug_str = format!("{:?}", s);
        assert_eq!(debug_str, "[REDACTED]");
        assert!(!debug_str.contains("super-secret-token"));
    }

    #[test]
    fn test_debug_alt_formatting_still_redacted() {
        let s = SecretString::new("another-secret");
        let debug_str = format!("{:#?}", s);
        assert_eq!(debug_str, "[REDACTED]");
    }

    #[test]
    fn test_deref_targets_str() {
        let s = SecretString::new("deref-target");
        // Deref allows calling str methods directly
        assert!(s.starts_with("deref"));
        assert_eq!(s.len(), "deref-target".len());
        assert_eq!(&*s, "deref-target");
    }

    #[test]
    fn test_deref_allows_str_methods() {
        let s = SecretString::new("Hello, World!");
        let upper: String = s.to_uppercase();
        assert_eq!(upper, "HELLO, WORLD!");
        assert!(s.contains("World"));
        assert_eq!(s.trim_end_matches('!'), "Hello, World");
    }

    #[test]
    fn test_clone_creates_equal_value() {
        let s1 = SecretString::new("cloneable");
        let s2 = s1.clone();
        assert_eq!(s1.expose(), s2.expose());
        assert_eq!(s2.expose(), "cloneable");
    }

    #[test]
    fn test_zeroize_clears_content() {
        let mut s = SecretString::new("will-be-cleared");
        assert_eq!(s.expose(), "will-be-cleared");
        s.zeroize();
        // After zeroize, the underlying string should be empty (Zeroizing<String> clears to empty)
        assert!(s.expose().is_empty());
    }

    #[test]
    fn test_zeroize_on_empty_string_is_noop() {
        let mut s = SecretString::new("");
        s.zeroize();
        assert!(s.expose().is_empty());
    }

    #[test]
    fn test_zeroize_idempotent() {
        let mut s = SecretString::new("twice");
        s.zeroize();
        s.zeroize();
        assert!(s.expose().is_empty());
    }

    #[test]
    fn test_long_secret_string_round_trip() {
        let long = "x".repeat(10_000);
        let s = SecretString::new(long.clone());
        assert_eq!(s.expose(), long);
        assert_eq!(s.expose_clone().as_str(), long);
    }

    #[test]
    fn test_unicode_secret_string() {
        let unicode = "你好，世界！🌍".to_string();
        let s = SecretString::new(unicode.clone());
        assert_eq!(s.expose(), unicode);
        assert_eq!(s.expose_clone().as_str(), unicode);
    }

    #[test]
    fn test_secret_string_with_special_chars() {
        let special = "line1\nline2\ttab\rcarriage";
        let s = SecretString::new(special);
        assert_eq!(s.expose(), special);
    }

    #[test]
    fn test_clone_then_zeroize_does_not_affect_other() {
        let s1 = SecretString::new("shared");
        let mut s2 = s1.clone();
        s2.zeroize();
        // Zeroizing the clone must not affect the original (separate allocations).
        assert_eq!(s1.expose(), "shared");
        assert!(s2.expose().is_empty());
    }

    #[test]
    fn test_debug_empty_secret_string() {
        let s = SecretString::default();
        let debug_str = format!("{:?}", s);
        assert_eq!(debug_str, "[REDACTED]");
    }
}
