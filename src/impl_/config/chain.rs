//! Source chain for combining multiple configuration sources.
//!
//! The source chain manages multiple sources with priority ordering
//! and merges their values according to merge strategies.

use crate::error::{ConfigError, ConfigResult};
use crate::impl_::merger::{MergeEngine, MergeStrategy};
use crate::interface::Source;
use crate::types::{AnnotatedValue, ConfigValue, SourceKind};
use indexmap::IndexMap;
use std::sync::Arc;

/// A chain of configuration sources with priority ordering.
///
/// Sources are collected and merged in order of priority.
/// Higher priority sources override values from lower priority sources.
pub struct SourceChain {
    /// Sources in the chain.
    sources: Vec<Box<dyn Source>>,
    /// Merge engine for combining values.
    merge_engine: MergeEngine,
    /// Whether to stop on first error.
    fail_fast: bool,
}

impl Default for SourceChain {
    fn default() -> Self {
        Self::new()
    }
}

impl SourceChain {
    /// Create a new empty source chain.
    pub fn new() -> Self {
        Self {
            sources: Vec::new(),
            merge_engine: MergeEngine::new(),
            fail_fast: true,
        }
    }

    /// Create a source chain with a default merge strategy.
    pub fn with_strategy(strategy: MergeStrategy) -> Self {
        Self {
            sources: Vec::new(),
            merge_engine: MergeEngine::new().with_default_strategy(strategy),
            fail_fast: true,
        }
    }

    /// Push a source to the chain.
    pub fn push(mut self, source: Box<dyn Source>) -> Self {
        self.sources.push(source);
        self
    }

    /// Add a source with explicit ordering.
    pub fn add_ordered(mut self, source: Box<dyn Source>) -> Self {
        // Insert in priority order (higher priority sources should be processed later)
        let priority = source.priority();
        let pos = self
            .sources
            .iter()
            .position(|s| s.priority() > priority)
            .unwrap_or(self.sources.len());
        self.sources.insert(pos, source);
        self
    }

    /// Set whether to stop on first error.
    pub fn fail_fast(mut self, fail_fast: bool) -> Self {
        self.fail_fast = fail_fast;
        self
    }

    /// Set a field-specific merge strategy.
    pub fn with_field_strategy(
        mut self,
        field: impl Into<Arc<str>>,
        strategy: MergeStrategy,
    ) -> Self {
        self.merge_engine = self.merge_engine.with_field_strategy(field, strategy);
        self
    }

    /// Get the number of sources.
    pub fn len(&self) -> usize {
        self.sources.len()
    }

    /// Check if the chain is empty.
    pub fn is_empty(&self) -> bool {
        self.sources.is_empty()
    }

    /// Get a reference to the sources in this chain.
    pub fn sources(&self) -> &[Box<dyn Source>] {
        &self.sources
    }

    /// Collect and merge all sources.
    pub fn collect(self) -> ConfigResult<AnnotatedValue> {
        let sources = self.sources;
        let merge_engine = self.merge_engine;
        let fail_fast = self.fail_fast;

        Self::collect_and_merge(sources, merge_engine, fail_fast)
    }

    fn collect_and_merge(
        sources: Vec<Box<dyn Source>>,
        merge_engine: MergeEngine,
        fail_fast: bool,
    ) -> ConfigResult<AnnotatedValue> {
        if sources.is_empty() {
            return Ok(AnnotatedValue::new(
                ConfigValue::Map(Arc::new(IndexMap::new())),
                crate::types::SourceId::new("empty"),
                "",
            ));
        }

        // Collect all source values
        let mut values: Vec<(String, ConfigResult<AnnotatedValue>)> = Vec::new();
        let mut errors: Vec<(String, ConfigError)> = Vec::new();

        for source in &sources {
            let name = source.name().to_string();
            let result = source.collect();

            match result {
                Ok(value) => values.push((name, Ok(value))),
                Err(e) => {
                    if fail_fast && !source.is_optional() {
                        return Err(e);
                    }
                    errors.push((name, e));
                }
            }
        }

        // Handle all errors case
        if values.is_empty() && !errors.is_empty() {
            let multi_err = crate::error::MultiSourceError::new(sources.len(), errors);
            return Err(ConfigError::MultiSource { source: multi_err });
        }

        // Sort by priority (lower priority first)
        let mut sorted_values: Vec<_> = values
            .into_iter()
            .filter_map(|(_, result)| result.ok())
            .collect();
        sorted_values.sort_by_key(|v| v.priority);

        // Merge all values
        let mut merged = AnnotatedValue::new(
            ConfigValue::Map(Arc::new(IndexMap::new())),
            crate::types::SourceId::new("merged"),
            "",
        );

        for value in sorted_values {
            merged = merge_engine.merge(&merged, &value)?;
        }

        Ok(merged)
    }

    /// Get a list of source names.
    pub fn source_names(&self) -> Vec<&str> {
        self.sources.iter().map(|s| s.name()).collect()
    }

    /// Get source kinds.
    pub fn source_kinds(&self) -> Vec<SourceKind> {
        self.sources.iter().map(|s| s.source_kind()).collect()
    }
}

/// Builder for creating source chains with a fluent API.
pub struct SourceChainBuilder {
    chain: SourceChain,
    /// Whether to allow absolute paths for file sources.
    allow_absolute_paths: bool,
}

impl Default for SourceChainBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl SourceChainBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self {
            chain: SourceChain::new(),
            allow_absolute_paths: false,
        }
    }

    /// Add a source.
    pub fn source(mut self, source: Box<dyn Source>) -> Self {
        self.chain = self.chain.push(source);
        self
    }

    /// Add a file source.
    pub fn file(self, path: impl Into<std::path::PathBuf>) -> Self {
        use super::source::FileSource;
        let mut source = FileSource::new(path);
        if self.allow_absolute_paths {
            source = source.allow_absolute_paths();
        }
        self.source(Box::new(source))
    }

    /// Add an optional file source.
    pub fn file_optional(self, path: impl Into<std::path::PathBuf>) -> Self {
        use super::source::FileSource;
        let mut source = FileSource::new(path).optional();
        if self.allow_absolute_paths {
            source = source.allow_absolute_paths();
        }
        self.source(Box::new(source))
    }

    /// Allow absolute paths for file sources (use with caution, mainly for testing).
    pub fn allow_absolute_paths(mut self) -> Self {
        self.allow_absolute_paths = true;
        self
    }

    /// Add an environment source.
    pub fn env(self) -> Self {
        use super::source::EnvSource;
        self.source(Box::new(EnvSource::new()))
    }

    /// Add an environment source with prefix.
    pub fn env_with_prefix(self, prefix: impl Into<String>) -> Self {
        use super::source::EnvSource;
        self.source(Box::new(EnvSource::with_prefix(prefix)))
    }

    /// Add a default source.
    pub fn defaults(self, defaults: std::collections::HashMap<String, ConfigValue>) -> Self {
        use super::source::DefaultSource;
        self.source(Box::new(DefaultSource::with_defaults(defaults)))
    }

    /// Add a memory source.
    pub fn memory(self, values: std::collections::HashMap<String, ConfigValue>) -> Self {
        use super::source::MemorySource;
        self.source(Box::new(MemorySource::with_values(values)))
    }

    /// Add a memory source with custom priority.
    pub fn memory_with_priority(
        self,
        values: std::collections::HashMap<String, ConfigValue>,
        priority: u8,
    ) -> Self {
        use super::source::MemorySource;
        self.source(Box::new(
            MemorySource::with_values(values).with_priority(priority),
        ))
    }

    /// Set merge strategy.
    pub fn strategy(mut self, strategy: MergeStrategy) -> Self {
        self.chain.merge_engine = self.chain.merge_engine.with_default_strategy(strategy);
        self
    }

    /// Set field-specific merge strategy.
    pub fn field_strategy(mut self, field: impl Into<Arc<str>>, strategy: MergeStrategy) -> Self {
        self.chain = self.chain.with_field_strategy(field, strategy);
        self
    }

    /// Set fail fast mode.
    pub fn fail_fast(mut self, fail_fast: bool) -> Self {
        self.chain = self.chain.fail_fast(fail_fast);
        self
    }

    /// Build the source chain.
    pub fn build(self) -> SourceChain {
        self.chain
    }

    /// Get file paths from file sources for watching.
    pub fn get_watch_paths(&self) -> Vec<std::path::PathBuf> {
        self.chain
            .sources
            .iter()
            .filter(|s| s.source_kind() == SourceKind::File)
            .filter_map(|s| s.file_path().map(|p| p.to_path_buf()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::impl_::config::{DefaultSource, MemorySource};

    #[test]
    fn test_empty_chain() {
        let chain = SourceChain::new();
        let result = chain.collect().unwrap();
        assert!(result.is_map());
    }

    #[test]
    fn test_single_source() {
        let chain = SourceChain::new().push(Box::new(
            MemorySource::new().set("key", ConfigValue::string("value")),
        ));

        let result = chain.collect().unwrap();
        assert!(result.is_map());
    }

    #[test]
    fn test_multiple_sources() {
        let chain = SourceChain::new()
            .push(Box::new(
                DefaultSource::new().set("key", ConfigValue::string("default")),
            ))
            .push(Box::new(
                MemorySource::new()
                    .set("key", ConfigValue::string("override"))
                    .with_priority(50),
            ));

        let result = chain.collect().unwrap();
        assert!(result.is_map());
    }

    #[test]
    fn test_builder() {
        let chain = SourceChainBuilder::new()
            .defaults(std::collections::HashMap::from([(
                "key".to_string(),
                ConfigValue::string("default"),
            )]))
            .memory(std::collections::HashMap::from([(
                "key".to_string(),
                ConfigValue::string("memory"),
            )]))
            .build();

        assert!(!chain.is_empty());
        assert_eq!(chain.len(), 2);
    }

    #[test]
    fn test_source_names() {
        let chain = SourceChain::new()
            .push(Box::new(MemorySource::new().with_name("first")))
            .push(Box::new(MemorySource::new().with_name("second")));

        let names = chain.source_names();
        assert_eq!(names, vec!["first", "second"]);
    }

    #[test]
    fn test_fail_fast_optional() {
        let chain = SourceChain::new()
            .fail_fast(false)
            .push(Box::new(
                crate::impl_::config::FileSource::new("/nonexistent.toml").optional(),
            ))
            .push(Box::new(
                MemorySource::new().set("key", ConfigValue::string("value")),
            ));

        let result = chain.collect().unwrap();
        assert!(result.is_map());
    }

    #[test]
    fn test_chain_default_trait() {
        let chain = SourceChain::default();
        assert!(chain.is_empty());
    }

    #[test]
    fn test_chain_is_empty() {
        let chain = SourceChain::new();
        assert!(chain.is_empty());
        let chain2 = SourceChain::new().push(Box::new(MemorySource::new()));
        assert!(!chain2.is_empty());
    }

    #[test]
    fn test_chain_len() {
        let chain = SourceChain::new();
        assert_eq!(chain.len(), 0);
        let chain2 = SourceChain::new()
            .push(Box::new(MemorySource::new()))
            .push(Box::new(MemorySource::new()));
        assert_eq!(chain2.len(), 2);
    }

    #[test]
    fn test_chain_with_strategy() {
        let chain = SourceChain::with_strategy(MergeStrategy::Append);
        let result = chain.collect().unwrap();
        assert!(result.is_map());
    }

    #[test]
    fn test_chain_add_ordered() {
        let chain = SourceChain::new()
            .add_ordered(Box::new(MemorySource::new().with_priority(10)))
            .add_ordered(Box::new(MemorySource::new().with_priority(50)))
            .add_ordered(Box::new(MemorySource::new().with_priority(30)));
        assert_eq!(chain.len(), 3);
        let result = chain.collect().unwrap();
        assert!(result.is_map());
    }

    #[test]
    fn test_chain_with_field_strategy() {
        let chain = SourceChain::new()
            .with_field_strategy("name", MergeStrategy::Replace)
            .push(Box::new(
                MemorySource::new().set("name", ConfigValue::string("x")),
            ));
        let result = chain.collect().unwrap();
        assert!(result.is_map());
    }

    #[test]
    fn test_chain_sources_accessor() {
        let chain = SourceChain::new()
            .push(Box::new(MemorySource::new()))
            .push(Box::new(DefaultSource::new()));
        assert_eq!(chain.sources().len(), 2);
    }

    #[test]
    fn test_chain_source_kinds() {
        let chain = SourceChain::new()
            .push(Box::new(MemorySource::new()))
            .push(Box::new(DefaultSource::new()));
        let kinds = chain.source_kinds();
        assert_eq!(kinds, vec![SourceKind::Memory, SourceKind::Default]);
    }

    #[test]
    fn test_chain_override_behavior() {
        // Higher priority source overrides lower priority
        let chain = SourceChain::new()
            .push(Box::new(
                DefaultSource::new().set("key", ConfigValue::string("default_val")),
            ))
            .push(Box::new(
                MemorySource::new()
                    .set("key", ConfigValue::string("override_val"))
                    .with_priority(50),
            ));
        let result = chain.collect().unwrap();
        assert!(result.is_map());
        if let ConfigValue::Map(map) = &result.inner {
            let val = map.get("key").expect("key should exist");
            if let ConfigValue::String(s) = &val.inner {
                assert_eq!(s, "override_val");
            } else {
                panic!("expected String value");
            }
        } else {
            panic!("expected map");
        }
    }

    #[test]
    fn test_chain_fail_fast_required_error() {
        // fail_fast=true + required source fails → immediate error
        let chain = SourceChain::new().fail_fast(true).push(Box::new(
            crate::impl_::config::FileSource::new("/nonexistent.toml"),
        ));
        let result = chain.collect();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ConfigError::FileNotFound { .. }
        ));
    }

    #[test]
    fn test_chain_multi_source_error() {
        // fail_fast=false + all required sources fail → MultiSource error
        let chain = SourceChain::new()
            .fail_fast(false)
            .push(Box::new(crate::impl_::config::FileSource::new(
                "/nonexistent1.toml",
            )))
            .push(Box::new(crate::impl_::config::FileSource::new(
                "/nonexistent2.toml",
            )));
        let result = chain.collect();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ConfigError::MultiSource { .. }
        ));
    }

    #[test]
    fn test_builder_default_trait() {
        let builder = SourceChainBuilder::default();
        let chain = builder.build();
        assert!(chain.is_empty());
    }

    #[test]
    fn test_builder_source_method() {
        let chain = SourceChainBuilder::new()
            .source(Box::new(
                MemorySource::new().set("k", ConfigValue::string("v")),
            ))
            .build();
        assert_eq!(chain.len(), 1);
    }

    #[test]
    fn test_builder_file_method() {
        let chain = SourceChainBuilder::new().file("config.toml").build();
        assert_eq!(chain.len(), 1);
        assert_eq!(chain.source_kinds(), vec![SourceKind::File]);
    }

    #[test]
    fn test_builder_file_optional_method() {
        let chain = SourceChainBuilder::new()
            .file_optional("missing.toml")
            .build();
        assert_eq!(chain.len(), 1);
    }

    #[test]
    fn test_builder_env_method() {
        let chain = SourceChainBuilder::new().env().build();
        assert_eq!(chain.len(), 1);
        assert_eq!(chain.source_kinds(), vec![SourceKind::Environment]);
    }

    #[test]
    fn test_builder_env_with_prefix_method() {
        let chain = SourceChainBuilder::new().env_with_prefix("X_").build();
        assert_eq!(chain.len(), 1);
        assert_eq!(chain.source_kinds(), vec![SourceKind::Environment]);
    }

    #[test]
    fn test_builder_memory_with_priority() {
        let chain = SourceChainBuilder::new()
            .memory_with_priority(
                std::collections::HashMap::from([("k".to_string(), ConfigValue::string("v"))]),
                99,
            )
            .build();
        assert_eq!(chain.len(), 1);
    }

    #[test]
    fn test_builder_strategy_method() {
        let chain = SourceChainBuilder::new()
            .strategy(MergeStrategy::Append)
            .build();
        let result = chain.collect().unwrap();
        assert!(result.is_map());
    }

    #[test]
    fn test_builder_field_strategy_method() {
        let chain = SourceChainBuilder::new()
            .field_strategy("key", MergeStrategy::Replace)
            .build();
        let result = chain.collect().unwrap();
        assert!(result.is_map());
    }

    #[test]
    fn test_builder_fail_fast_method() {
        let chain = SourceChainBuilder::new().fail_fast(false).build();
        let result = chain.collect().unwrap();
        assert!(result.is_map());
    }

    #[test]
    fn test_builder_allow_absolute_paths_method() {
        let chain = SourceChainBuilder::new()
            .allow_absolute_paths()
            .file("/absolute/path.toml")
            .build();
        assert_eq!(chain.len(), 1);
    }

    #[test]
    fn test_builder_get_watch_paths() {
        let builder = SourceChainBuilder::new()
            .file("config1.toml")
            .file("config2.json")
            .env();
        let paths = builder.get_watch_paths();
        // Only file sources contribute paths (env source has none)
        assert_eq!(paths.len(), 2);
    }

    #[test]
    fn test_chain_source_names_multi() {
        let chain = SourceChain::new()
            .push(Box::new(MemorySource::new().with_name("alpha")))
            .push(Box::new(MemorySource::new().with_name("beta")))
            .push(Box::new(DefaultSource::new()));
        let names = chain.source_names();
        assert_eq!(names, vec!["alpha", "beta", "default"]);
    }
}
