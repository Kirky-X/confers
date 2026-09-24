// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

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

/// Result of [`SourceChain::collect_report`]: the merged outcome plus the
/// per-source failures that were skipped instead of aborting the chain.
///
/// `failures` carries `(source name, rendered error message)` pairs in
/// source order; it is empty when every source succeeded or when
/// `fail_fast` aborted the chain on its first error.
pub struct ChainOutcome {
    /// The merged value, or the error that aborted/failed the collection.
    pub merged: ConfigResult<AnnotatedValue>,
    /// Skipped sources: `(source name, error message)`.
    pub failures: Vec<(String, String)>,
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

    /// Register sensitive configuration paths: conflict reports redact the
    /// values of these paths (and anything nested below them).
    pub fn with_sensitive_paths(mut self, paths: Vec<String>) -> Self {
        self.merge_engine = self.merge_engine.with_sensitive_paths(paths);
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
    ///
    /// # Partial failure semantics
    ///
    /// When `fail_fast` is disabled (or the failing source is optional),
    /// sources that fail are **silently skipped**: the merged result combines
    /// only the successful sources and the collected errors are *not*
    /// exposed to the caller. A [`ConfigError::MultiSource`] error is
    /// returned only when *every* source fails. Callers that need per-source
    /// error visibility must use [`Self::collect_report`] or collect the
    /// sources individually.
    pub fn collect(self) -> ConfigResult<AnnotatedValue> {
        self.collect_report().merged
    }

    /// Like [`Self::collect`], but keeps the per-source failures that were
    /// skipped in non-fail-fast mode visible so callers can surface them
    /// (e.g. as build warnings) instead of losing them silently.
    pub fn collect_report(self) -> ChainOutcome {
        let sources = self.sources;
        let merge_engine = self.merge_engine;
        let fail_fast = self.fail_fast;

        Self::collect_and_merge_report(sources, merge_engine, fail_fast)
    }

    fn collect_and_merge_report(
        sources: Vec<Box<dyn Source>>,
        merge_engine: MergeEngine,
        fail_fast: bool,
    ) -> ChainOutcome {
        if sources.is_empty() {
            return ChainOutcome {
                merged: Ok(AnnotatedValue::new(
                    ConfigValue::Map(Arc::new(IndexMap::new())),
                    crate::types::SourceId::new("empty"),
                    "",
                )),
                failures: Vec::new(),
            };
        }

        // Collect all source values, remembering whether each source supplies
        // default values so the merge order can put them first regardless of
        // declaration order or file names.
        let mut values: Vec<(bool, String, ConfigResult<AnnotatedValue>)> = Vec::new();
        let mut errors: Vec<(String, ConfigError)> = Vec::new();
        // Name + rendered message for every skipped source; ConfigError is
        // not `Clone`, so the report keeps the flattened form only.
        let mut failures: Vec<(String, String)> = Vec::new();

        for source in &sources {
            let name = source.name().to_string();
            let is_defaults = source.source_kind() == SourceKind::Default;
            let result = source.collect();

            match result {
                Ok(value) => values.push((is_defaults, name, Ok(value))),
                Err(e) => {
                    if fail_fast && !source.is_optional() {
                        return ChainOutcome {
                            merged: Err(e),
                            failures: Vec::new(),
                        };
                    }
                    failures.push((name.clone(), e.to_string()));
                    errors.push((name, e));
                }
            }
        }

        // Handle all errors case
        if values.is_empty() && !errors.is_empty() {
            let multi_err = crate::error::MultiSourceError::new(sources.len(), errors);
            return ChainOutcome {
                merged: Err(ConfigError::MultiSource { source: multi_err }),
                failures,
            };
        }

        // Sort by priority (lower priority first). The sort is stable, so
        // equal-priority sources keep declaration order ("later declaration
        // overrides earlier"). Default-value sources always merge before
        // every other source so they can never override file/env/memory
        // values, whatever the sources are named or declared.
        let mut sorted_values: Vec<(bool, AnnotatedValue)> = values
            .into_iter()
            .filter_map(|(is_defaults, _, result)| result.ok().map(|v| (is_defaults, v)))
            .collect();
        sorted_values.sort_by_key(|(is_defaults, value)| (value.priority, !*is_defaults));

        // Merge all values
        let mut merged = AnnotatedValue::new(
            ConfigValue::Map(Arc::new(IndexMap::new())),
            crate::types::SourceId::new("merged"),
            "",
        );

        for (_, value) in sorted_values {
            match merge_engine.merge(&merged, &value) {
                Ok(m) => merged = m,
                Err(e) => {
                    return ChainOutcome {
                        merged: Err(e),
                        failures,
                    };
                }
            }
        }

        ChainOutcome {
            merged: Ok(merged),
            failures,
        }
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
    /// Pending env nesting separator, applied when the next env source is
    /// created (`env_separator("__")` so `APP_DB__HOST` → `db.host`).
    env_separator: Option<String>,
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
            env_separator: None,
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

    /// Set the env nesting separator (e.g. `"__"` so `APP_DB__HOST` maps to
    /// `db.host`). Applies to the NEXT env source created by
    /// [`Self::env`](Self::env)/[`Self::env_with_prefix`](Self::env_with_prefix),
    /// so call it before those.
    pub fn env_separator(mut self, separator: impl Into<String>) -> Self {
        self.env_separator = Some(separator.into());
        self
    }

    /// Register sensitive configuration paths on the merge engine: conflict
    /// reports redact these paths' values (keep this wired to the
    /// builder so the first conflict-report caller cannot leak plaintext).
    pub fn with_sensitive_paths(mut self, paths: Vec<String>) -> Self {
        self.chain = self.chain.with_sensitive_paths(paths);
        self
    }

    /// Add an environment source.
    pub fn env(self) -> Self {
        use super::source::EnvSource;
        let mut source = EnvSource::new();
        if let Some(sep) = self.env_separator.clone() {
            source = source.separator(sep);
        }
        self.source(Box::new(source))
    }

    /// Add an environment source with prefix.
    pub fn env_with_prefix(self, prefix: impl Into<String>) -> Self {
        use super::source::EnvSource;
        let mut source = EnvSource::with_prefix(prefix);
        if let Some(sep) = self.env_separator.clone() {
            source = source.separator(sep);
        }
        self.source(Box::new(source))
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
    fn test_chain_partial_failure_merges_successful_sources() {
        // fail_fast=false + a failing REQUIRED source + a healthy source:
        // the chain merges the successful sources and silently drops the
        // error (documented partial-failure semantics). The chain only
        // errors when every source fails.
        let chain = SourceChain::new()
            .fail_fast(false)
            .push(Box::new(crate::impl_::config::FileSource::new(
                "/nonexistent-required.toml",
            )))
            .push(Box::new(
                MemorySource::new()
                    .set("key", ConfigValue::string("from_memory"))
                    .with_priority(50),
            ));

        let result = chain.collect().expect("successful sources must merge");
        assert!(result.is_map());
        if let ConfigValue::Map(map) = &result.inner {
            let val = map.get("key").expect("key should exist");
            if let ConfigValue::String(s) = &val.inner {
                assert_eq!(s, "from_memory");
            } else {
                panic!("expected String value");
            }
        } else {
            panic!("expected map");
        }
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

    fn write_temp_toml(
        name_prefix: &str,
        body: &str,
    ) -> (tempfile::NamedTempFile, std::path::PathBuf) {
        let file = tempfile::Builder::new()
            .prefix(name_prefix)
            .suffix(".toml")
            .tempfile_in(std::env::current_dir().unwrap())
            .unwrap();
        std::fs::write(file.path(), body).unwrap();
        let path = file.path().to_path_buf();
        let rel = path
            .strip_prefix(std::env::current_dir().unwrap())
            .unwrap_or(&path)
            .to_path_buf();
        (file, rel)
    }

    fn map_str(result: &AnnotatedValue, key: &str) -> String {
        match &result.inner {
            ConfigValue::Map(map) => match &map.get(key).expect("key should exist").inner {
                ConfigValue::String(s) => s.clone(),
                other => panic!("expected string for {key}, got {other:?}"),
            },
            _ => panic!("expected map"),
        }
    }

    #[test]
    #[cfg(feature = "toml")]
    fn test_default_never_overrides_file_regardless_of_name() {
        // Regression: same-priority tie-break used to compare source ids
        // alphabetically, so a file named before "default" lost to defaults.
        // Defaults must always merge first, whatever the file is called.
        let (_f1, path) = write_temp_toml("aaa_config", "host = \"file-host\"\n");
        let chain = SourceChainBuilder::new()
            .file(path)
            .defaults(std::collections::HashMap::from([(
                "host".to_string(),
                ConfigValue::string("default-host"),
            )]))
            .build();
        let merged = chain.collect().unwrap();
        assert_eq!(map_str(&merged, "host"), "file-host");

        // Same outcome when the default source is declared first.
        let (_f2, path) = write_temp_toml("aaa_config", "host = \"file-host\"\n");
        let chain = SourceChainBuilder::new()
            .defaults(std::collections::HashMap::from([(
                "host".to_string(),
                ConfigValue::string("default-host"),
            )]))
            .file(path)
            .build();
        let merged = chain.collect().unwrap();
        assert_eq!(map_str(&merged, "host"), "file-host");
    }

    #[test]
    #[cfg(feature = "toml")]
    fn test_declaration_order_two_files_later_wins() {
        // Regression: same-priority files used to be ordered by file name
        // (bbb < mmm), silently inverting the documented "later declaration
        // overrides earlier" semantics.
        let (_base, base_path) = write_temp_toml("mmm_base", "host = \"base-host\"\n");
        let (_over, over_path) = write_temp_toml("bbb_override", "host = \"override-host\"\n");
        let chain = SourceChainBuilder::new()
            .file(base_path)
            .file(over_path)
            .build();
        let merged = chain.collect().unwrap();
        assert_eq!(map_str(&merged, "host"), "override-host");
    }

    #[test]
    fn test_same_priority_declaration_order_beats_name_order() {
        // memory "zzz" declared first must lose to memory "aaa" declared
        // second, even though "aaa" sorts before "zzz" alphabetically.
        let chain = SourceChain::new()
            .push(Box::new(
                MemorySource::new()
                    .with_name("zzz")
                    .set("key", ConfigValue::string("first"))
                    .with_priority(50),
            ))
            .push(Box::new(
                MemorySource::new()
                    .with_name("aaa")
                    .set("key", ConfigValue::string("second"))
                    .with_priority(50),
            ));
        let merged = chain.collect().unwrap();
        assert_eq!(map_str(&merged, "key"), "second");
    }
}
