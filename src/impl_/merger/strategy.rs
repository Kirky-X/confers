// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Merge strategies for configuration values.

use crate::types::ConfigValue;

/// Custom merge function type.
pub type CustomMergeFn = fn(&ConfigValue, &ConfigValue) -> ConfigValue;

/// Merge strategy for combining configuration values.
#[derive(Clone, Copy, Default)]
pub enum MergeStrategy {
    /// Replace the lower priority value entirely (default)
    #[default]
    Replace,
    /// Join string values with a separator
    Join { separator: &'static str },
    /// Append arrays: "low priority + high priority"
    Append,
    /// Prepend arrays: "high priority + low priority"
    Prepend,
    /// Join and append: join strings, append arrays
    JoinAppend { separator: &'static str },
    /// Deep merge maps recursively
    DeepMerge,
    /// Custom merge function
    Custom {
        /// Custom merge function
        func: CustomMergeFn,
        /// Name for debugging
        name: &'static str,
    },
}

impl std::fmt::Debug for MergeStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MergeStrategy::Replace => write!(f, "Replace"),
            MergeStrategy::Join { separator } => write!(f, "Join({:?})", separator),
            MergeStrategy::Append => write!(f, "Append"),
            MergeStrategy::Prepend => write!(f, "Prepend"),
            MergeStrategy::JoinAppend { separator } => write!(f, "JoinAppend({:?})", separator),
            MergeStrategy::DeepMerge => write!(f, "DeepMerge"),
            MergeStrategy::Custom { name, .. } => write!(f, "Custom({:?})", name),
        }
    }
}

impl PartialEq for MergeStrategy {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (MergeStrategy::Replace, MergeStrategy::Replace) => true,
            (MergeStrategy::Join { separator: a }, MergeStrategy::Join { separator: b }) => a == b,
            (MergeStrategy::Append, MergeStrategy::Append) => true,
            (MergeStrategy::Prepend, MergeStrategy::Prepend) => true,
            (
                MergeStrategy::JoinAppend { separator: a },
                MergeStrategy::JoinAppend { separator: b },
            ) => a == b,
            (MergeStrategy::DeepMerge, MergeStrategy::DeepMerge) => true,
            (MergeStrategy::Custom { name: a, .. }, MergeStrategy::Custom { name: b, .. }) => {
                a == b
            }
            _ => false,
        }
    }
}

impl Eq for MergeStrategy {}

impl MergeStrategy {
    /// Create a join strategy with separator
    pub fn join(separator: &'static str) -> Self {
        MergeStrategy::Join { separator }
    }

    /// Create a join-append strategy with separator
    pub fn join_append(separator: &'static str) -> Self {
        MergeStrategy::JoinAppend { separator }
    }

    /// Create a custom merge strategy
    pub fn custom(name: &'static str, func: CustomMergeFn) -> Self {
        MergeStrategy::Custom { func, name }
    }

    /// Check if this is a custom strategy
    pub fn is_custom(&self) -> bool {
        matches!(self, MergeStrategy::Custom { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_strategy() {
        assert_eq!(MergeStrategy::default(), MergeStrategy::Replace);
    }

    #[test]
    fn test_join_strategy() {
        let s = MergeStrategy::join(":");
        assert_eq!(s, MergeStrategy::Join { separator: ":" });
    }

    #[test]
    fn test_custom_strategy() {
        fn my_merge(low: &ConfigValue, high: &ConfigValue) -> ConfigValue {
            match (low, high) {
                (ConfigValue::String(l), ConfigValue::String(h)) => {
                    ConfigValue::String(format!("{}+{}", l, h))
                }
                _ => high.clone(),
            }
        }

        let s = MergeStrategy::custom("my_merge", my_merge);
        assert!(s.is_custom());
        assert_eq!(
            s,
            MergeStrategy::Custom {
                name: "my_merge",
                func: my_merge,
            }
        );
    }

    #[test]
    fn test_append_strategy() {
        let s = MergeStrategy::Append;
        assert_eq!(s, MergeStrategy::Append);
    }

    #[test]
    fn test_prepend_strategy() {
        let s = MergeStrategy::Prepend;
        assert_eq!(s, MergeStrategy::Prepend);
    }

    #[test]
    fn test_deep_merge_strategy() {
        let s = MergeStrategy::DeepMerge;
        assert_eq!(s, MergeStrategy::DeepMerge);
    }

    #[test]
    fn test_join_append_constructor() {
        let s = MergeStrategy::join_append(",");
        assert_eq!(s, MergeStrategy::JoinAppend { separator: "," });
    }

    #[test]
    fn test_is_custom_false_for_non_custom() {
        assert!(!MergeStrategy::Replace.is_custom());
        assert!(!MergeStrategy::Append.is_custom());
        assert!(!MergeStrategy::Prepend.is_custom());
        assert!(!MergeStrategy::DeepMerge.is_custom());
        assert!(!MergeStrategy::join(":").is_custom());
        assert!(!MergeStrategy::join_append(":").is_custom());
    }

    #[test]
    fn test_debug_format_replace() {
        assert_eq!(format!("{:?}", MergeStrategy::Replace), "Replace");
    }

    #[test]
    fn test_debug_format_join() {
        assert_eq!(format!("{:?}", MergeStrategy::join(":")), "Join(\":\")");
    }

    #[test]
    fn test_debug_format_append() {
        assert_eq!(format!("{:?}", MergeStrategy::Append), "Append");
    }

    #[test]
    fn test_debug_format_prepend() {
        assert_eq!(format!("{:?}", MergeStrategy::Prepend), "Prepend");
    }

    #[test]
    fn test_debug_format_join_append() {
        assert_eq!(
            format!("{:?}", MergeStrategy::join_append(":")),
            "JoinAppend(\":\")"
        );
    }

    #[test]
    fn test_debug_format_deep_merge() {
        assert_eq!(format!("{:?}", MergeStrategy::DeepMerge), "DeepMerge");
    }

    #[test]
    fn test_debug_format_custom() {
        fn noop(_: &ConfigValue, high: &ConfigValue) -> ConfigValue {
            high.clone()
        }
        let s = MergeStrategy::custom("noop", noop);
        assert_eq!(format!("{:?}", s), "Custom(\"noop\")");
    }

    #[test]
    fn test_partial_eq_same_variants() {
        assert_eq!(MergeStrategy::Replace, MergeStrategy::Replace);
        assert_eq!(MergeStrategy::join(":"), MergeStrategy::join(":"));
        assert_eq!(MergeStrategy::Append, MergeStrategy::Append);
        assert_eq!(MergeStrategy::Prepend, MergeStrategy::Prepend);
        assert_eq!(
            MergeStrategy::join_append(","),
            MergeStrategy::join_append(",")
        );
        assert_eq!(MergeStrategy::DeepMerge, MergeStrategy::DeepMerge);
    }

    #[test]
    fn test_partial_eq_different_separators() {
        assert_ne!(MergeStrategy::join(":"), MergeStrategy::join(","));
        assert_ne!(
            MergeStrategy::join_append(":"),
            MergeStrategy::join_append(",")
        );
    }

    #[test]
    fn test_partial_eq_different_variants() {
        assert_ne!(MergeStrategy::Replace, MergeStrategy::Append);
        assert_ne!(MergeStrategy::Append, MergeStrategy::Prepend);
        assert_ne!(MergeStrategy::join(":"), MergeStrategy::join_append(":"));
        assert_ne!(MergeStrategy::DeepMerge, MergeStrategy::Replace);
        assert_ne!(
            MergeStrategy::join(":"),
            MergeStrategy::Custom {
                name: "join",
                func: |_, h| h.clone(),
            }
        );
    }

    #[test]
    fn test_custom_eq_by_name_only() {
        fn f1(_: &ConfigValue, h: &ConfigValue) -> ConfigValue {
            h.clone()
        }
        fn f2(_: &ConfigValue, h: &ConfigValue) -> ConfigValue {
            h.clone()
        }
        // Same name → equal, even with different funcs
        let s1 = MergeStrategy::custom("name", f1);
        let s2 = MergeStrategy::custom("name", f2);
        assert_eq!(s1, s2);
        // Different names → not equal
        let s3 = MergeStrategy::custom("other", f1);
        assert_ne!(s1, s3);
    }

    #[test]
    fn test_clone_copy_behavior() {
        let s = MergeStrategy::join(":");
        let cloned = s;
        assert_eq!(s, cloned);
        // Copy semantic: after assignment, both are still valid
        let copied = s;
        assert_eq!(s, copied);
    }
}
