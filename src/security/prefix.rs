// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Encryption prefix for identifying encrypted values.

use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// Encryption prefix for identifying encrypted values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum EncryptionPrefix {
    /// Standard encryption prefix "enc:"
    #[default]
    Enc,
}

impl EncryptionPrefix {
    /// Get the string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            EncryptionPrefix::Enc => "enc:",
        }
    }

    /// Check if a string starts with this prefix
    pub fn is_prefixed(&self, value: &str) -> bool {
        value.starts_with(self.as_str())
    }

    /// Strip the prefix from a value
    pub fn strip<'a>(&self, value: &'a str) -> Option<&'a str> {
        value.strip_prefix(self.as_str())
    }
}

impl std::fmt::Display for EncryptionPrefix {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for EncryptionPrefix {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "enc:" => Ok(Self::Enc),
            _ => Err(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encryption_prefix_as_str() {
        assert_eq!(EncryptionPrefix::Enc.as_str(), "enc:");
    }

    #[test]
    fn test_encryption_prefix_from_str() {
        assert_eq!(
            EncryptionPrefix::from_str("enc:"),
            Ok(EncryptionPrefix::Enc)
        );
        assert_eq!(EncryptionPrefix::from_str("invalid"), Err(()));
    }

    #[test]
    fn test_encryption_prefix_is_prefixed() {
        assert!(EncryptionPrefix::Enc.is_prefixed("enc:base64data"));
        assert!(!EncryptionPrefix::Enc.is_prefixed("plaintext"));
    }

    #[test]
    fn test_encryption_prefix_strip() {
        assert_eq!(
            EncryptionPrefix::Enc.strip("enc:base64data"),
            Some("base64data")
        );
        assert_eq!(EncryptionPrefix::Enc.strip("plaintext"), None);
    }
}
