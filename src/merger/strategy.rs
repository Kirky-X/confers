//! Merge strategies for configuration values.

/// Merge strategy for combining configuration values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MergeStrategy {
    /// Replace the lower priority value entirely (default)
    #[default]
    Replace,
    /// Join string values with a separator
    Join { separator: &'static str },
    /// Append arrays: [low] + [high]
    Append,
    /// Prepend arrays: [high] + [low]
    Prepend,
    /// Join and append: join strings, append arrays
    JoinAppend { separator: &'static str },
    /// Deep merge maps recursively
    DeepMerge,
}

impl MergeStrategy {
    /// Create a join strategy with separator
    pub fn join(separator: &'static str) -> Self {
        MergeStrategy::Join { separator }
    }

    /// Create a join-append strategy with separator
    pub fn join_append(separator: &'static str) -> Self {
        MergeStrategy::JoinAppend { separator }
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
}
