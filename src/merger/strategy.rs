//! Merge strategies for configuration values.

use crate::value::ConfigValue;

/// Custom merge function type.
pub type CustomMergeFn = fn(&ConfigValue, &ConfigValue) -> ConfigValue;

/// Merge strategy for combining configuration values.
#[derive(Clone, Default)]
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
            (MergeStrategy::JoinAppend { separator: a }, MergeStrategy::JoinAppend { separator: b }) => a == b,
            (MergeStrategy::DeepMerge, MergeStrategy::DeepMerge) => true,
            (MergeStrategy::Custom { name: a, .. }, MergeStrategy::Custom { name: b, .. }) => a == b,
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
}
