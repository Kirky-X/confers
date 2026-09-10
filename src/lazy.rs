// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Lazy segmented parsing (`lazy` feature).
//!
//! Huge configurations pay the full parse cost up front even when only a
//! couple of sections are actually read at startup. [`LazySegmentedConfig`]
//! splits a TOML document into segments at its top-level table headers — a
//! cheap line scan, no value parsing — and parses each segment **on first
//! access**, caching the result. Sections that are never touched are never
//! parsed, cutting startup memory and CPU for oversized documents.
//!
//! Segment boundaries are top-level `[table]` / `[[array-of-table]]`
//! headers; lines before the first header form the `""` (preamble) segment.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::error::{ConfigError, ConfigResult};
use crate::loader::{parse_content, Format};
use crate::types::{AnnotatedValue, SourceId};

const SOURCE_NAME: &str = "lazy";

/// A TOML document parsed lazily, one top-level section at a time.
#[derive(Debug, Default)]
pub struct LazySegmentedConfig {
    /// Raw text of each segment, keyed by its top-level table name
    /// (`""` = preamble before the first header).
    segments: HashMap<String, Arc<str>>,
    /// Parse cache: a segment is parsed at most once.
    parsed: Mutex<HashMap<String, Arc<AnnotatedValue>>>,
}

impl LazySegmentedConfig {
    /// Split a TOML document into lazily-parsed segments.
    ///
    /// The split is a line scan for top-level headers; values are **not**
    /// parsed here. Returns an error only when the document holds no
    /// recognizable content at all.
    pub fn from_toml_document(text: &str) -> Self {
        let mut segments: HashMap<String, Vec<&str>> = HashMap::new();

        fn header_name(line: &str) -> Option<String> {
            let trimmed = line.trim_start();
            if !trimmed.starts_with('[') {
                return None;
            }
            // `[[array]]` form: strip the double opening bracket first so
            // the first `]` terminates the name.
            let rest = trimmed
                .strip_prefix("[[")
                .unwrap_or_else(|| &trimmed[1..]);
            let end = rest.find(']')?;
            let name = rest[..end].trim();
            if name.is_empty() {
                return None;
            }
            // `[a.b]` belongs to top-level segment "a"; `[[x]]` likewise
            // maps to "x" (array-of-tables).
            let top = name.split('.').next().unwrap_or("").trim();
            Some(top.to_string())
        }

        let mut current_name = String::new();
        for line in text.lines() {
            match header_name(line) {
                Some(name) => {
                    current_name = name;
                    segments.entry(current_name.clone()).or_default().push(line);
                }
                None => {
                    segments
                        .entry(current_name.clone())
                        .or_default()
                        .push(line);
                }
            }
        }

        let mut segment_map = HashMap::new();
        for (name, lines) in segments {
            let joined: Arc<str> = Arc::from(lines.join("\n").as_str());
            segment_map.insert(name, joined);
        }
        Self {
            segments: segment_map,
            parsed: Mutex::new(HashMap::new()),
        }
    }

    /// Top-level segment names (no parsing performed).
    pub fn segment_keys(&self) -> Vec<String> {
        let mut keys: Vec<String> = self.segments.keys().cloned().collect();
        keys.sort();
        keys
    }

    /// Whether the segment was already parsed.
    pub fn is_parsed(&self, key: &str) -> bool {
        self.parsed
            .lock()
            .map(|cache| cache.contains_key(key))
            .unwrap_or(false)
    }

    /// Number of segments parsed so far (test/observability hook).
    pub fn parsed_count(&self) -> usize {
        self.parsed.lock().map(|c| c.len()).unwrap_or(0)
    }

    /// Parse and cache the segment on first access; subsequent calls return
    /// the cached value without re-parsing.
    pub fn get_segment(&self, key: &str) -> ConfigResult<Option<Arc<AnnotatedValue>>> {
        let raw = match self.segments.get(key) {
            Some(raw) => raw.clone(),
            None => return Ok(None),
        };

        if let Ok(cache) = self.parsed.lock() {
            if let Some(cached) = cache.get(key) {
                return Ok(Some(Arc::clone(cached)));
            }
        }

        let value = parse_content(
            raw.as_ref(),
            Format::Toml,
            SourceId::new(format!("{SOURCE_NAME}:{key}")),
            None,
        )
        .map_err(|e| ConfigError::ParseError {
            format: "toml".to_string(),
            message: format!("lazy segment '{key}' failed to parse: {}", e),
            location: None,
            source: None,
        })?;
        let shared = Arc::new(value);

        if let Ok(mut cache) = self.parsed.lock() {
            cache.insert(key.to_string(), Arc::clone(&shared));
        }
        Ok(Some(shared))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BIG_DOC: &str = r#"
title = "lazy demo"

[database]
host = "db.internal"
port = 5432

[cache]
backend = "memory"

[[servers]]
name = "s1"

[[servers]]
name = "s2"
"#;

    #[test]
    fn segment_keys_listed_without_parsing() {
        let lazy = LazySegmentedConfig::from_toml_document(BIG_DOC);
        assert_eq!(
            lazy.segment_keys(),
            vec!["", "cache", "database", "servers"]
        );
        // The split is a scan only: nothing parsed yet.
        assert_eq!(lazy.parsed_count(), 0);
        assert!(!lazy.is_parsed("database"));
    }

    #[test]
    fn segments_parse_on_first_access_and_cache() {
        let lazy = LazySegmentedConfig::from_toml_document(BIG_DOC);

        let database = lazy.get_segment("database").expect("parse").expect("present");
        assert_eq!(lazy.parsed_count(), 1, "only the accessed segment parsed");
        assert!(lazy.is_parsed("database"));
        assert!(!lazy.is_parsed("cache"), "untouched segments stay unparsed");

        // The fragment keeps its table header: values live under the
        // segment's own name.
        let host = database
            .inner
            .as_map()
            .and_then(|m| m.get("database"))
            .and_then(|t| t.inner.as_map())
            .and_then(|m| m.get("host"))
            .and_then(|v| v.as_str());
        assert_eq!(host, Some("db.internal"));
        let port = database
            .inner
            .as_map()
            .and_then(|m| m.get("database"))
            .and_then(|t| t.inner.as_map())
            .and_then(|m| m.get("port"))
            .and_then(|v| v.as_i64());
        assert_eq!(port, Some(5432));

        // Second access hits the cache (no new parse).
        let again = lazy.get_segment("database").expect("cached").expect("present");
        assert!(Arc::ptr_eq(&database, &again), "cached handle is shared");
        assert_eq!(lazy.parsed_count(), 1);
    }

    #[test]
    fn array_of_tables_form_their_own_segment() {
        let lazy = LazySegmentedConfig::from_toml_document(BIG_DOC);
        eprintln!("RAW SEGMENT: {:?}", lazy.segments.get("servers"));
        let servers = lazy.get_segment("servers").expect("parse").expect("present");
        // The [[servers]] entries parse into an array under the segment name.
        let count = servers
            .inner
            .as_map()
            .and_then(|m| m.get("servers"))
            .and_then(|v| v.inner.as_array())
            .map(|a| a.len());
        assert_eq!(count, Some(2), "both [[servers]] entries parse");
        // "cache" remains unparsed — untouched segments cost nothing.
        assert!(!lazy.is_parsed("cache"));
        assert_eq!(lazy.parsed_count(), 1);
    }

    #[test]
    fn unknown_segment_is_none_without_parsing() {
        let lazy = LazySegmentedConfig::from_toml_document(BIG_DOC);
        assert!(lazy.get_segment("missing").expect("ok").is_none());
        assert_eq!(lazy.parsed_count(), 0);
    }

    #[test]
    fn preamble_segment_holds_pre_header_keys() {
        let lazy = LazySegmentedConfig::from_toml_document(BIG_DOC);
        let preamble = lazy.get_segment("").expect("parse").expect("present");
        let title = preamble
            .inner
            .as_map()
            .and_then(|m| m.get("title"))
            .and_then(|v| v.as_str());
        assert_eq!(title, Some("lazy demo"));
    }
}
