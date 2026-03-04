//! Source chain for combining multiple configuration sources.
//!
//! The source chain manages multiple sources with priority ordering
//! and merges their values according to merge strategies.

use crate::error::{ConfigError, ConfigResult};
use crate::merger::{MergeEngine, MergeStrategy};
use crate::value::{AnnotatedValue, ConfigValue};
use indexmap::IndexMap;
use std::sync::Arc;

use super::source::{Source, SourceKind};

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
                crate::value::SourceId::new("empty"),
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
            let indexed_errors: Vec<(usize, ConfigError)> = errors
                .into_iter()
                .enumerate()
                .map(|(i, (_, e))| (i, e))
                .collect();
            let multi_err = crate::error::MultiSourceError::new(sources.len(), indexed_errors);
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
            crate::value::SourceId::new("merged"),
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
        self.source(Box::new(FileSource::new(path)))
    }

    /// Add an optional file source.
    pub fn file_optional(self, path: impl Into<std::path::PathBuf>) -> Self {
        use super::source::FileSource;
        self.source(Box::new(FileSource::new(path).optional()))
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
    use crate::config::source::{DefaultSource, MemorySource};

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
                crate::config::source::FileSource::new("/nonexistent.toml").optional(),
            ))
            .push(Box::new(
                MemorySource::new().set("key", ConfigValue::string("value")),
            ));

        let result = chain.collect().unwrap();
        assert!(result.is_map());
    }
}
