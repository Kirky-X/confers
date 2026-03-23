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
}
