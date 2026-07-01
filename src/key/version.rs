// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Key format version for tracking key format compatibility.

use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// Key format version for tracking key format compatibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum KeyFormatVersion {
    /// Version 1 (current)
    V1,
}

impl KeyFormatVersion {
    /// Current stable version
    pub const CURRENT: Self = Self::V1;

    /// Get the string representation (e.g., "v1")
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::V1 => "v1",
        }
    }

    /// Get the numeric version
    pub fn as_u32(&self) -> u32 {
        match self {
            Self::V1 => 1,
        }
    }

    /// Get the version prefix
    pub fn prefix() -> &'static str {
        "v"
    }

    /// Parse from numeric version
    pub fn from_u32(v: u32) -> Option<Self> {
        match v {
            1 => Some(Self::V1),
            _ => None,
        }
    }

    /// Get the next version (for migration)
    pub fn next(&self) -> Option<Self> {
        match self {
            Self::V1 => None, // No next version yet
        }
    }

    /// Check if this version is deprecated
    pub fn is_deprecated(&self) -> bool {
        match self {
            Self::V1 => false, // Current version
        }
    }
}

impl std::fmt::Display for KeyFormatVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Default for KeyFormatVersion {
    fn default() -> Self {
        Self::CURRENT
    }
}

impl FromStr for KeyFormatVersion {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "v1" => Ok(Self::V1),
            _ => Err(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_format_version_as_str() {
        assert_eq!(KeyFormatVersion::V1.as_str(), "v1");
    }

    #[test]
    fn test_key_format_version_from_str() {
        assert_eq!(KeyFormatVersion::from_str("v1"), Ok(KeyFormatVersion::V1));
        assert_eq!(KeyFormatVersion::from_str("invalid"), Err(()));
    }

    #[test]
    fn test_key_format_version_as_u32() {
        assert_eq!(KeyFormatVersion::V1.as_u32(), 1);
    }

    #[test]
    fn test_key_format_version_from_u32() {
        assert_eq!(KeyFormatVersion::from_u32(1), Some(KeyFormatVersion::V1));
        assert_eq!(KeyFormatVersion::from_u32(2), None);
    }

    #[test]
    fn test_key_format_version_prefix() {
        assert_eq!(KeyFormatVersion::prefix(), "v");
    }

    #[test]
    fn test_key_format_version_next() {
        assert_eq!(KeyFormatVersion::V1.next(), None);
    }

    #[test]
    fn test_key_format_version_is_deprecated() {
        assert!(!KeyFormatVersion::V1.is_deprecated());
    }

    #[test]
    fn test_key_format_version_display() {
        let v = KeyFormatVersion::V1;
        assert_eq!(format!("{}", v), "v1");
        // Display output must match as_str
        assert_eq!(format!("{}", v), v.as_str());
    }

    #[test]
    fn test_key_format_version_default_is_current() {
        let v = KeyFormatVersion::default();
        assert_eq!(v, KeyFormatVersion::CURRENT);
        assert_eq!(v, KeyFormatVersion::V1);
    }

    #[test]
    fn test_key_format_version_current_constant() {
        assert_eq!(KeyFormatVersion::CURRENT, KeyFormatVersion::V1);
        assert_eq!(KeyFormatVersion::CURRENT.as_str(), "v1");
        assert_eq!(KeyFormatVersion::CURRENT.as_u32(), 1);
    }

    #[test]
    fn test_key_format_version_clone_copy() {
        let v1 = KeyFormatVersion::V1;
        let copied = v1; // Copy
        assert_eq!(v1, copied);
        // Clone is available via derive but Copy makes explicit clone redundant;
        // verify the trait is present by using copy semantics.
        let moved = v1;
        assert_eq!(moved, v1);
    }

    #[test]
    fn test_key_format_version_eq_ne() {
        assert_eq!(KeyFormatVersion::V1, KeyFormatVersion::V1);
        // Only one variant exists; assert self-equality holds.
        let v = KeyFormatVersion::V1;
        assert_eq!(v, v);
    }

    #[test]
    fn test_key_format_version_ord() {
        // Single variant: ordering is reflexive.
        let v1 = KeyFormatVersion::V1;
        assert!(v1 >= v1);
        assert!(v1 <= v1);
    }

    #[test]
    fn test_key_format_version_partial_ord_consistency() {
        let a = KeyFormatVersion::V1;
        let b = KeyFormatVersion::V1;
        // PartialOrd must agree with PartialEq
        assert_eq!(a.partial_cmp(&b), Some(std::cmp::Ordering::Equal));
        assert_eq!(a.cmp(&b), std::cmp::Ordering::Equal);
    }

    #[test]
    fn test_key_format_version_hash_consistency() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut h1 = DefaultHasher::new();
        let mut h2 = DefaultHasher::new();
        KeyFormatVersion::V1.hash(&mut h1);
        KeyFormatVersion::V1.hash(&mut h2);
        assert_eq!(h1.finish(), h2.finish(), "equal values must hash equally");
    }

    #[test]
    fn test_key_format_version_in_hashset() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(KeyFormatVersion::V1);
        set.insert(KeyFormatVersion::V1);
        assert_eq!(set.len(), 1, "duplicate inserts must dedupe");
        assert!(set.contains(&KeyFormatVersion::V1));
    }

    #[test]
    fn test_key_format_version_serialize_deserialize() {
        let v = KeyFormatVersion::V1;
        let json = serde_json::to_string(&v).expect("serialize");
        let de: KeyFormatVersion = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(de, v);
    }

    #[test]
    fn test_key_format_version_serialize_value() {
        let v = KeyFormatVersion::V1;
        let json = serde_json::to_string(&v).expect("serialize");
        // Serde serializes the enum variant name as a string.
        assert_eq!(json, "\"V1\"");
    }

    #[test]
    fn test_key_format_version_deserialize_invalid_returns_error() {
        let bad_json = "\"V999\"";
        let result: Result<KeyFormatVersion, _> = serde_json::from_str(bad_json);
        assert!(result.is_err());
    }

    #[test]
    fn test_key_format_version_debug_format() {
        let v = KeyFormatVersion::V1;
        let debug_str = format!("{:?}", v);
        assert_eq!(debug_str, "V1");
    }

    #[test]
    fn test_key_format_version_from_str_roundtrip() {
        let v = KeyFormatVersion::V1;
        let s = v.to_string();
        let parsed: KeyFormatVersion = s.parse().expect("parse");
        assert_eq!(parsed, v);
    }

    #[test]
    fn test_key_format_version_from_u32_roundtrip() {
        let v = KeyFormatVersion::V1;
        let n = v.as_u32();
        let parsed = KeyFormatVersion::from_u32(n).expect("from_u32");
        assert_eq!(parsed, v);
    }

    #[test]
    fn test_key_format_version_from_u32_zero_returns_none() {
        assert_eq!(KeyFormatVersion::from_u32(0), None);
    }

    #[test]
    fn test_key_format_version_from_u32_large_returns_none() {
        assert_eq!(KeyFormatVersion::from_u32(u32::MAX), None);
    }

    #[test]
    fn test_key_format_version_from_str_case_sensitive() {
        // "V1" (uppercase) is NOT a valid version — only "v1" lowercase is.
        assert!(KeyFormatVersion::from_str("V1").is_err());
        assert!(KeyFormatVersion::from_str("v1").is_ok());
    }

    #[test]
    fn test_key_format_version_from_str_empty_returns_err() {
        assert!(KeyFormatVersion::from_str("").is_err());
    }

    #[test]
    fn test_key_format_version_next_is_none_for_v1() {
        // No V2 exists yet — next() returns None.
        assert_eq!(KeyFormatVersion::V1.next(), None);
    }

    #[test]
    fn test_key_format_version_is_deprecated_false_for_all_variants() {
        // Only V1 exists; it is the current version and not deprecated.
        assert!(!KeyFormatVersion::V1.is_deprecated());
        assert!(!KeyFormatVersion::CURRENT.is_deprecated());
    }

    #[test]
    fn test_key_format_version_prefix_is_lowercase_v() {
        assert_eq!(KeyFormatVersion::prefix(), "v");
        assert_ne!(KeyFormatVersion::prefix(), "V");
    }
}
