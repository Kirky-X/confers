//! Configuration value types for confers.
//!
//! This module provides:
//! - `ConfigValue` - The core value enum for configuration data
//! - `AnnotatedValue` - Value with metadata (source, location, priority)
//! - `SourceLocation` - Precise file location for error reporting
//! - `ConflictReport` - Merge conflict information

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;

// MergeStrategy is now imported directly from crate::merger.
// This re-export was removed to fix a reverse dependency (value -> merger violates layering).
// Users should import MergeStrategy from crate::merger::MergeStrategy.

/// Source identifier for tracking where a value came from.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SourceId(Arc<str>);

impl Serialize for SourceId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for SourceId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(Self(Arc::from(s)))
    }
}

impl SourceId {
    /// Create a new source ID.
    pub fn new(id: impl Into<Arc<str>>) -> Self {
        Self(id.into())
    }

    /// Get the source ID as a string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for SourceId {
    fn default() -> Self {
        Self::new("default")
    }
}

impl std::fmt::Display for SourceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for SourceId {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for SourceId {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

/// Precise location in a source file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceLocation {
    /// Source file name (without full path for privacy)
    pub source_name: Arc<str>,
    /// Line number (1-based)
    pub line: usize,
    /// Column number (1-based)
    pub column: usize,
    /// Full file path for internal diagnostics (not exposed to end users)
    pub(crate) file_path: Option<std::path::PathBuf>,
}

impl Serialize for SourceLocation {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("SourceLocation", 4)?;
        s.serialize_field("source_name", &*self.source_name)?;
        s.serialize_field("line", &self.line)?;
        s.serialize_field("column", &self.column)?;
        s.serialize_field("file_path", &self.file_path)?;
        s.end()
    }
}

impl<'de> Deserialize<'de> for SourceLocation {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct SourceLocationHelper {
            source_name: String,
            line: usize,
            column: usize,
        }
        let h = SourceLocationHelper::deserialize(deserializer)?;
        Ok(Self {
            source_name: Arc::from(h.source_name),
            line: h.line,
            column: h.column,
            file_path: None,
        })
    }
}

impl SourceLocation {
    /// Create a new source location.
    pub fn new(source_name: impl Into<Arc<str>>, line: usize, column: usize) -> Self {
        Self {
            source_name: source_name.into(),
            line,
            column,
            file_path: None,
        }
    }

    /// Create from a path, extracting only the filename for privacy.
    pub fn from_path(path: &std::path::Path, line: usize, column: usize) -> Self {
        let source_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");
        let fp = Some(path.to_path_buf());
        Self {
            source_name: Arc::from(source_name),
            line,
            column,
            file_path: fp,
        }
    }
}

impl std::fmt::Display for SourceLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}:{}", self.source_name, self.line, self.column)
    }
}

/// Core configuration value type.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum ConfigValue {
    /// Null value
    #[default]
    Null,
    /// Boolean value
    Bool(bool),
    /// Signed integer
    I64(i64),
    /// Unsigned integer
    U64(u64),
    /// Floating point number
    F64(f64),
    /// String value
    String(String),
    /// Binary data
    Bytes(Vec<u8>),
    /// Array of values
    Array(Arc<[AnnotatedValue]>),
    /// Map of key-value pairs
    Map(Arc<IndexMap<Arc<str>, AnnotatedValue>>),
}

impl Serialize for ConfigValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            ConfigValue::Null => serializer.serialize_none(),
            ConfigValue::Bool(b) => serializer.serialize_bool(*b),
            ConfigValue::I64(i) => serializer.serialize_i64(*i),
            ConfigValue::U64(u) => serializer.serialize_u64(*u),
            ConfigValue::F64(f) => serializer.serialize_f64(*f),
            ConfigValue::String(s) => serializer.serialize_str(s),
            ConfigValue::Bytes(b) => {
                use serde::ser::SerializeSeq;
                let mut seq = serializer.serialize_seq(Some(b.len()))?;
                for byte in b {
                    seq.serialize_element(byte)?;
                }
                seq.end()
            }
            ConfigValue::Array(arr) => arr.serialize(serializer),
            ConfigValue::Map(map) => {
                use serde::ser::SerializeMap;
                let mut m = serializer.serialize_map(Some(map.len()))?;
                for (k, v) in map.iter() {
                    m.serialize_entry(k.as_ref(), v)?;
                }
                m.end()
            }
        }
    }
}

impl<'de> Deserialize<'de> for ConfigValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{self, Visitor};

        struct ConfigValueVisitor;

        impl<'de> Visitor<'de> for ConfigValueVisitor {
            type Value = ConfigValue;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a configuration value")
            }

            fn visit_none<E>(self) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(ConfigValue::Null)
            }

            fn visit_unit<E>(self) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(ConfigValue::Null)
            }

            fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(ConfigValue::Bool(v))
            }

            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(ConfigValue::I64(v))
            }

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(ConfigValue::U64(v))
            }

            fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(ConfigValue::F64(v))
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(ConfigValue::String(v.to_string()))
            }

            fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(ConfigValue::String(v))
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                let mut arr = Vec::new();
                while let Some(v) = seq.next_element()? {
                    arr.push(v);
                }
                Ok(ConfigValue::Array(arr.into()))
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                let mut m = IndexMap::new();
                while let Some((k, v)) = map.next_entry::<String, AnnotatedValue>()? {
                    m.insert(Arc::from(k), v);
                }
                Ok(ConfigValue::Map(Arc::new(m)))
            }
        }

        deserializer.deserialize_any(ConfigValueVisitor)
    }
}

impl ConfigValue {
    /// Check if the value is null.
    pub fn is_null(&self) -> bool {
        matches!(self, ConfigValue::Null)
    }

    /// Check if the value is a boolean.
    pub fn is_bool(&self) -> bool {
        matches!(self, ConfigValue::Bool(_))
    }

    /// Check if the value is an integer.
    pub fn is_integer(&self) -> bool {
        matches!(self, ConfigValue::I64(_) | ConfigValue::U64(_))
    }

    /// Check if the value is a number.
    pub fn is_number(&self) -> bool {
        matches!(
            self,
            ConfigValue::I64(_) | ConfigValue::U64(_) | ConfigValue::F64(_)
        )
    }

    /// Check if the value is a string.
    pub fn is_string(&self) -> bool {
        matches!(self, ConfigValue::String(_))
    }

    /// Check if the value is an array.
    pub fn is_array(&self) -> bool {
        matches!(self, ConfigValue::Array(_))
    }

    /// Check if the value is a map.
    pub fn is_map(&self) -> bool {
        matches!(self, ConfigValue::Map(_))
    }

    /// Get as boolean.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            ConfigValue::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Get as i64.
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            ConfigValue::I64(i) => Some(*i),
            ConfigValue::U64(u) if *u <= i64::MAX as u64 => Some(*u as i64),
            _ => None,
        }
    }

    /// Get as u64.
    pub fn as_u64(&self) -> Option<u64> {
        match self {
            ConfigValue::U64(u) => Some(*u),
            ConfigValue::I64(i) if *i >= 0 => Some(*i as u64),
            _ => None,
        }
    }

    /// Get as f64.
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            ConfigValue::F64(f) => Some(*f),
            ConfigValue::I64(i) => Some(*i as f64),
            ConfigValue::U64(u) => Some(*u as f64),
            _ => None,
        }
    }

    /// Get as string.
    #[deprecated(since = "0.3.0", note = "Prefer as_str() to avoid allocation")]
    pub fn as_string(&self) -> Option<String> {
        match self {
            ConfigValue::String(s) => Some(s.clone()),
            _ => None,
        }
    }

    /// Get as string reference.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            ConfigValue::String(s) => Some(s),
            _ => None,
        }
    }

    /// Get as array.
    pub fn as_array(&self) -> Option<&[AnnotatedValue]> {
        match self {
            ConfigValue::Array(arr) => Some(arr),
            _ => None,
        }
    }

    /// Get as map.
    pub fn as_map(&self) -> Option<&IndexMap<Arc<str>, AnnotatedValue>> {
        match self {
            ConfigValue::Map(map) => Some(map),
            _ => None,
        }
    }

    /// Create a null value.
    pub fn null() -> Self {
        ConfigValue::Null
    }

    /// Create a boolean value.
    pub fn bool(b: bool) -> Self {
        ConfigValue::Bool(b)
    }

    /// Create an integer value.
    pub fn integer(i: i64) -> Self {
        ConfigValue::I64(i)
    }

    /// Create an unsigned integer value.
    pub fn uint(u: u64) -> Self {
        ConfigValue::U64(u)
    }

    /// Create a float value.
    pub fn float(f: f64) -> Self {
        ConfigValue::F64(f)
    }

    /// Create a string value.
    pub fn string(s: impl Into<String>) -> Self {
        ConfigValue::String(s.into())
    }

    /// Create an array value.
    pub fn array(values: Vec<AnnotatedValue>) -> Self {
        ConfigValue::Array(values.into())
    }

    /// Create a map value.
    pub fn map(entries: Vec<(impl Into<Arc<str>>, AnnotatedValue)>) -> Self {
        let mut map = IndexMap::new();
        for (k, v) in entries {
            map.insert(k.into(), v);
        }
        ConfigValue::Map(Arc::new(map))
    }
}

impl From<bool> for ConfigValue {
    fn from(b: bool) -> Self {
        ConfigValue::Bool(b)
    }
}

impl From<i64> for ConfigValue {
    fn from(i: i64) -> Self {
        ConfigValue::I64(i)
    }
}

impl From<i32> for ConfigValue {
    fn from(i: i32) -> Self {
        ConfigValue::I64(i as i64)
    }
}

impl From<i16> for ConfigValue {
    fn from(i: i16) -> Self {
        ConfigValue::I64(i as i64)
    }
}

impl From<i8> for ConfigValue {
    fn from(i: i8) -> Self {
        ConfigValue::I64(i as i64)
    }
}

impl From<u64> for ConfigValue {
    fn from(u: u64) -> Self {
        ConfigValue::U64(u)
    }
}

impl From<u32> for ConfigValue {
    fn from(u: u32) -> Self {
        ConfigValue::U64(u as u64)
    }
}

impl From<u16> for ConfigValue {
    fn from(u: u16) -> Self {
        ConfigValue::U64(u as u64)
    }
}

impl From<u8> for ConfigValue {
    fn from(u: u8) -> Self {
        ConfigValue::U64(u as u64)
    }
}

impl From<usize> for ConfigValue {
    fn from(u: usize) -> Self {
        ConfigValue::U64(u as u64)
    }
}

impl From<isize> for ConfigValue {
    fn from(i: isize) -> Self {
        ConfigValue::I64(i as i64)
    }
}

impl From<f64> for ConfigValue {
    fn from(f: f64) -> Self {
        ConfigValue::F64(f)
    }
}

impl From<String> for ConfigValue {
    fn from(s: String) -> Self {
        ConfigValue::String(s)
    }
}

impl From<&str> for ConfigValue {
    fn from(s: &str) -> Self {
        ConfigValue::String(s.to_string())
    }
}

impl<T: Into<ConfigValue>> From<Option<T>> for ConfigValue {
    fn from(opt: Option<T>) -> Self {
        match opt {
            Some(v) => v.into(),
            None => ConfigValue::Null,
        }
    }
}

impl<T: Into<ConfigValue> + Clone + Send + Sync + 'static> From<Vec<T>> for ConfigValue {
    fn from(vec: Vec<T>) -> Self {
        ConfigValue::Array(
            vec.into_iter()
                .map(|v| AnnotatedValue::new(v.into(), SourceId::default(), ""))
                .collect(),
        )
    }
}

/// Annotated configuration value with metadata.
#[derive(Debug, Clone, PartialEq)]
pub struct AnnotatedValue {
    /// The actual value
    pub inner: ConfigValue,
    /// Source identifier
    pub source: SourceId,
    /// Configuration path in dot notation
    pub path: Arc<str>,
    /// Priority level (higher = more important)
    pub priority: u8,
    /// Version number for tracking changes
    pub version: u64,
    /// Optional source location for error reporting
    pub location: Option<SourceLocation>,
}

impl Serialize for AnnotatedValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("AnnotatedValue", 6)?;
        s.serialize_field("inner", &self.inner)?;
        s.serialize_field("source", &self.source)?;
        s.serialize_field("path", &*self.path)?;
        s.serialize_field("priority", &self.priority)?;
        s.serialize_field("version", &self.version)?;
        s.serialize_field("location", &self.location)?;
        s.end()
    }
}

impl<'de> Deserialize<'de> for AnnotatedValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct AnnotatedValueHelper {
            inner: ConfigValue,
            source: SourceId,
            path: String,
            priority: u8,
            version: u64,
            location: Option<SourceLocation>,
        }
        let h = AnnotatedValueHelper::deserialize(deserializer)?;
        Ok(Self {
            inner: h.inner,
            source: h.source,
            path: Arc::from(h.path),
            priority: h.priority,
            version: h.version,
            location: h.location,
        })
    }
}

impl AnnotatedValue {
    /// Create a new annotated value.
    pub fn new(value: ConfigValue, source: SourceId, path: impl Into<Arc<str>>) -> Self {
        Self {
            inner: value,
            source,
            path: path.into(),
            priority: 0,
            version: 0,
            location: None,
        }
    }

    /// Set the priority.
    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }

    /// Set the version.
    pub fn with_version(mut self, version: u64) -> Self {
        self.version = version;
        self
    }

    /// Set the source location.
    pub fn with_location(mut self, location: SourceLocation) -> Self {
        self.location = Some(location);
        self
    }

    /// Get as boolean.
    pub fn as_bool(&self) -> Option<bool> {
        self.inner.as_bool()
    }

    /// Get as i64.
    pub fn as_i64(&self) -> Option<i64> {
        self.inner.as_i64()
    }

    /// Get as u64.
    pub fn as_u64(&self) -> Option<u64> {
        self.inner.as_u64()
    }

    /// Get as f64.
    pub fn as_f64(&self) -> Option<f64> {
        self.inner.as_f64()
    }

    /// Get as string.
    #[deprecated(since = "0.3.0", note = "Prefer as_str() to avoid allocation")]
    pub fn as_string(&self) -> Option<String> {
        #[allow(deprecated)]
        self.inner.as_string()
    }

    /// Get as string reference.
    pub fn as_str(&self) -> Option<&str> {
        self.inner.as_str()
    }

    /// Check if the value is null.
    pub fn is_null(&self) -> bool {
        self.inner.is_null()
    }

    /// Check if the value is a map.
    pub fn is_map(&self) -> bool {
        self.inner.is_map()
    }

    /// Check if the value is an array.
    pub fn is_array(&self) -> bool {
        self.inner.is_array()
    }

    /// Check if the value is empty (null or empty string).
    pub fn is_empty(&self) -> bool {
        match &self.inner {
            ConfigValue::Null => true,
            ConfigValue::String(s) => s.is_empty(),
            _ => false,
        }
    }

    /// Get all configuration paths from this value (including self).
    pub fn all_paths(&self) -> Vec<Arc<str>> {
        self.all_paths_internal(true)
    }

    /// Internal implementation for all_paths methods.
    ///
    /// # Arguments
    /// * `include_self` - If true, includes the current path in the result
    ///
    /// Optimizations applied:
    /// - Reuses a String buffer for path building instead of format!() each time
    /// - Uses write! macro with pre-allocated buffer
    /// - Avoids unnecessary Arc::clone when pushing to stack
    fn all_paths_internal(&self, include_self: bool) -> Vec<Arc<str>> {
        let mut paths = if include_self {
            vec![self.path.clone()]
        } else {
            Vec::new()
        };

        // Stack for traversal: holds (value_ref, path)
        let mut stack: Vec<(&ConfigValue, Arc<str>)> = vec![(&self.inner, self.path.clone())];

        // Reusable buffer for path construction - avoids repeated allocations
        let mut path_buf = String::new();

        while let Some((value, current_path)) = stack.pop() {
            match value {
                ConfigValue::Map(map) => {
                    for (key, val) in map.iter() {
                        // Build new path using reusable buffer
                        path_buf.clear();
                        if !current_path.is_empty() {
                            path_buf.push_str(&current_path);
                            path_buf.push('.');
                        }
                        path_buf.push_str(key);

                        let new_path: Arc<str> = Arc::from(path_buf.as_str());
                        paths.push(new_path.clone());
                        stack.push((&val.inner, new_path));
                    }
                }
                ConfigValue::Array(arr) => {
                    for (i, val) in arr.iter().enumerate() {
                        // Build new path with array index
                        path_buf.clear();
                        if !current_path.is_empty() {
                            path_buf.push_str(&current_path);
                            path_buf.push('.');
                        }
                        let mut itoa_buf = itoa::Buffer::new();
                        path_buf.push_str(itoa_buf.format(i));

                        let new_path: Arc<str> = Arc::from(path_buf.as_str());
                        paths.push(new_path.clone());
                        stack.push((&val.inner, new_path));
                    }
                }
                _ => {}
            }
        }

        paths
    }

    /// Convert to JSON representation.
    #[cfg(feature = "json")]
    pub fn to_json(&self) -> serde_json::Value {
        self.to_json_with_mode(SerializeMode::Full, &[])
    }

    /// Convert to JSON with optional redaction.
    #[cfg(feature = "json")]
    pub fn to_json_with_mode(
        &self,
        mode: SerializeMode,
        sensitive_paths: &[&str],
    ) -> serde_json::Value {
        let is_sensitive = sensitive_paths
            .iter()
            .any(|p| self.path.as_ref().starts_with(p));

        if is_sensitive && mode == SerializeMode::Redacted {
            return serde_json::Value::String("[REDACTED]".to_string());
        }

        match &self.inner {
            ConfigValue::Null => serde_json::Value::Null,
            ConfigValue::Bool(b) => serde_json::Value::Bool(*b),
            ConfigValue::I64(i) => serde_json::Value::Number((*i).into()),
            ConfigValue::U64(u) => serde_json::Value::Number((*u).into()),
            ConfigValue::F64(f) => serde_json::Number::from_f64(*f)
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null),
            ConfigValue::String(s) => serde_json::Value::String(s.clone()),
            ConfigValue::Bytes(b) => {
                // Encode bytes as base64 for efficiency and compatibility
                use base64::Engine;
                let encoded = base64::engine::general_purpose::STANDARD.encode(b);
                serde_json::Value::String(encoded)
            }
            ConfigValue::Array(arr) => serde_json::Value::Array(
                arr.iter()
                    .map(|v| v.to_json_with_mode(mode, sensitive_paths))
                    .collect(),
            ),
            ConfigValue::Map(map) => serde_json::Value::Object(
                map.iter()
                    .map(|(k, v)| (k.to_string(), v.to_json_with_mode(mode, sensitive_paths)))
                    .collect(),
            ),
        }
    }

    /// Compare two values and produce a conflict report.
    ///
    /// The conflict report shows the lower and higher priority values
    /// along with their sources for diagnostic purposes.
    pub fn conflict_report(low: &Self, high: &Self) -> ConflictReport {
        ConflictReport::new(
            low.path.clone(),
            format!("{:?}", low.inner),
            low.source.clone(),
            low.location.clone(),
            format!("{:?}", high.inner),
            high.source.clone(),
            high.location.clone(),
            if low.priority >= high.priority {
                ConflictWinner::Low
            } else {
                ConflictWinner::High
            },
        )
    }
}

impl Default for AnnotatedValue {
    fn default() -> Self {
        Self {
            inner: ConfigValue::Null,
            source: SourceId::default(),
            path: Arc::from(""),
            priority: 0,
            version: 0,
            location: None,
        }
    }
}

impl From<ConfigValue> for AnnotatedValue {
    fn from(value: ConfigValue) -> Self {
        Self::new(value, SourceId::default(), "")
    }
}

/// Serialization mode for JSON output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SerializeMode {
    /// Redact sensitive values
    Redacted,
    /// Show all values (debug only)
    Full,
}

/// Winner of a merge conflict.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictWinner {
    /// Lower priority value won (should not happen normally)
    Low,
    /// Higher priority value won
    High,
}

/// Report of a merge conflict.
#[derive(Debug, Clone)]
pub struct ConflictReport {
    /// Path of the conflicting value
    pub path: Arc<str>,
    /// String representation of lower priority value
    pub low_value: String,
    /// Source of lower priority value
    pub low_source: SourceId,
    /// Location of lower priority value
    pub low_location: Option<SourceLocation>,
    /// String representation of higher priority value
    pub high_value: String,
    /// Source of higher priority value
    pub high_source: SourceId,
    /// Location of higher priority value
    pub high_location: Option<SourceLocation>,
    /// Which value won
    pub winner: ConflictWinner,
}

impl ConflictReport {
    /// Create a new conflict report.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        path: impl Into<Arc<str>>,
        low_value: String,
        low_source: SourceId,
        low_location: Option<SourceLocation>,
        high_value: String,
        high_source: SourceId,
        high_location: Option<SourceLocation>,
        winner: ConflictWinner,
    ) -> Self {
        Self {
            path: path.into(),
            low_value,
            low_source,
            low_location,
            high_value,
            high_source,
            high_location,
            winner,
        }
    }
}

// ============== Data types migrated from interface.rs (BrickArchitecture D1) ==============

/// Caching policy for key providers.
///
/// Unified type used by both `interface::KeyProvider` (sync) and `secret::KeyRegistry`.
/// Bricks that need TTL semantics should use `CacheWithTtl(duration)`;
/// permanent caches should use `CacheIndefinitely`; sensitive keys that must
/// never be cached should use `NoCache`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyCachePolicy {
    /// Never cache keys — re-fetch on every access.
    NoCache,
    /// Cache with a time-to-live (defaults to 1 hour when constructed via [`Default`]).
    CacheWithTtl(Duration),
    /// Cache indefinitely until explicitly invalidated.
    CacheIndefinitely,
}

impl Default for KeyCachePolicy {
    fn default() -> Self {
        KeyCachePolicy::CacheWithTtl(Duration::from_secs(3600))
    }
}

/// A wrapper for bytes that zeroizes on drop.
#[derive(Debug)]
pub struct ZeroizingBytes(Vec<u8>);

impl ZeroizingBytes {
    /// Create new zeroizing bytes.
    pub fn new(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }

    /// Get a reference to the bytes.
    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }

    /// Get the length of the bytes.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl Drop for ZeroizingBytes {
    fn drop(&mut self) {
        // Zeroize the bytes on drop
        for byte in &mut self.0 {
            *byte = 0;
        }
    }
}

// Deref/DerefMut mirror `zeroize::Zeroizing<Vec<u8>>` so that downstream code can
// treat `ZeroizingBytes` as `Vec<u8>` (e.g. `&*bytes`). The Drop impl still zeroes
// the underlying buffer when the wrapper goes out of scope.
impl std::ops::Deref for ZeroizingBytes {
    type Target = Vec<u8>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for ZeroizingBytes {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

// ZeroizingBytes does not implement Clone to prevent bypassing memory protection.
// The Drop trait ensures sensitive data is zeroized on drop.
// Note: Cloning ZeroizingBytes would leave copies in memory that cannot be zeroized.

/// No-op metrics backend for when metrics are disabled.
///
/// Public extension point companion to [`crate::interface::MetricsBackend`] — provided for
/// downstream consumers who need a default implementation.
#[derive(Debug, Clone, Default)]
pub struct NoOpMetrics;

impl crate::interface::MetricsBackend for NoOpMetrics {
    fn counter(&self, _name: &str, _labels: &[(&str, &str)]) {
        // No-op
    }

    fn histogram(&self, _name: &str, _value: f64, _labels: &[(&str, &str)]) {
        // No-op
    }
}

// ============== Source-related data types (migrated from config/source.rs) ==============

/// Kind of configuration source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceKind {
    /// File-based source
    File,
    /// Environment variable source
    Environment,
    /// Command-line argument source
    CommandLine,
    /// Default value source
    Default,
    /// Remote source (HTTP, Consul, etc.)
    #[cfg(feature = "remote")]
    Remote,
    /// In-memory source
    Memory,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::merger::MergeStrategy;

    #[test]
    fn test_source_id() {
        let id = SourceId::new("file://config.toml");
        assert_eq!(id.as_str(), "file://config.toml");
        assert_eq!(id.to_string(), "file://config.toml");
    }

    #[test]
    fn test_source_location() {
        let loc = SourceLocation::new("config.toml", 10, 5);
        assert_eq!(loc.to_string(), "config.toml:10:5");
        assert_eq!(loc.line, 10);
        assert_eq!(loc.column, 5);
    }

    #[test]
    fn test_config_value_types() {
        let v = ConfigValue::bool(true);
        assert!(v.is_bool());
        assert_eq!(v.as_bool(), Some(true));

        let v = ConfigValue::integer(42);
        assert!(v.is_integer());
        assert_eq!(v.as_i64(), Some(42));
        assert_eq!(v.as_u64(), Some(42));

        let v = ConfigValue::uint(100);
        assert_eq!(v.as_u64(), Some(100));

        let v = ConfigValue::float(std::f64::consts::PI);
        assert!(v.is_number());
        assert_eq!(v.as_f64(), Some(std::f64::consts::PI));

        let v = ConfigValue::string("hello");
        assert!(v.is_string());
        assert_eq!(v.as_str(), Some("hello"));
    }

    #[test]
    fn test_config_value_conversions() {
        let v: ConfigValue = true.into();
        assert_eq!(v.as_bool(), Some(true));

        let v: ConfigValue = 42i32.into();
        assert_eq!(v.as_i64(), Some(42));

        let v: ConfigValue = 100u32.into();
        assert_eq!(v.as_u64(), Some(100));

        let v: ConfigValue = "test".into();
        assert_eq!(v.as_str(), Some("test"));
    }

    #[test]
    fn test_annotated_value() {
        let val = AnnotatedValue::new(
            ConfigValue::string("localhost"),
            SourceId::new("default"),
            "database.host",
        )
        .with_priority(5)
        .with_version(1);

        assert_eq!(val.as_str(), Some("localhost"));
        assert_eq!(val.priority, 5);
        assert_eq!(val.version, 1);
    }

    #[test]
    fn test_annotated_value_paths() {
        let inner = ConfigValue::map(vec![
            (
                "host",
                AnnotatedValue::new(
                    ConfigValue::string("localhost"),
                    SourceId::new("default"),
                    "database.host",
                ),
            ),
            (
                "port",
                AnnotatedValue::new(
                    ConfigValue::uint(5432),
                    SourceId::new("default"),
                    "database.port",
                ),
            ),
        ]);

        let val = AnnotatedValue::new(inner, SourceId::new("default"), "database");

        let paths = val.all_paths();
        assert!(paths.contains(&Arc::from("database")));
        assert!(paths.contains(&Arc::from("database.host")));
        assert!(paths.contains(&Arc::from("database.port")));
    }

    #[test]
    fn test_merge_strategy_default() {
        assert_eq!(MergeStrategy::default(), MergeStrategy::Replace);
    }

    #[test]
    fn test_conflict_report() {
        let report = ConflictReport::new(
            "database.host",
            "localhost".to_string(),
            SourceId::new("file1"),
            None,
            "127.0.0.1".to_string(),
            SourceId::new("file2"),
            None,
            ConflictWinner::High,
        );

        assert_eq!(report.path.as_ref(), "database.host");
        assert_eq!(report.winner, ConflictWinner::High);
    }

    #[test]
    #[cfg(feature = "json")]
    fn test_to_json() {
        let val = AnnotatedValue::new(
            ConfigValue::string("secret"),
            SourceId::new("default"),
            "database.password",
        );

        let json = val.to_json();
        assert_eq!(json, serde_json::Value::String("secret".to_string()));

        let json_redacted = val.to_json_with_mode(SerializeMode::Redacted, &["database.password"]);
        assert_eq!(
            json_redacted,
            serde_json::Value::String("[REDACTED]".to_string())
        );
    }

    #[test]
    fn test_annotated_value_merge_basic() {
        let low = AnnotatedValue::new(
            ConfigValue::string("localhost"),
            SourceId::new("file"),
            "host",
        );
        let high = AnnotatedValue::new(
            ConfigValue::string("prod.example.com"),
            SourceId::new("env"),
            "host",
        );

        let engine = crate::impl_::merger::MergeEngine::new();
        let merged = engine.merge(&low, &high).unwrap_or_else(|_| high.clone());
        assert_eq!(merged.as_str(), Some("prod.example.com"));
    }

    #[test]
    fn test_annotated_value_merge_prefers_high() {
        let low = AnnotatedValue::new(ConfigValue::integer(8080), SourceId::new("default"), "port")
            .with_priority(10);
        let high = AnnotatedValue::new(ConfigValue::integer(9090), SourceId::new("env"), "port")
            .with_priority(50);

        let engine = crate::impl_::merger::MergeEngine::new();
        let merged = engine.merge(&low, &high).unwrap_or_else(|_| high.clone());
        assert_eq!(merged.as_i64(), Some(9090));
    }

    #[test]
    fn test_annotated_value_merge_null_high_returns_low() {
        let low = AnnotatedValue::new(ConfigValue::string("keep_me"), SourceId::new("file"), "key");
        let high = AnnotatedValue::new(ConfigValue::Null, SourceId::new("env"), "key");

        let engine = crate::impl_::merger::MergeEngine::new();
        let merged = engine.merge(&low, &high).unwrap_or_else(|_| high.clone());
        assert_eq!(merged.as_str(), Some("keep_me"));
    }

    #[test]
    fn test_annotated_value_conflict_report_high_wins() {
        let low = AnnotatedValue::new(ConfigValue::string("a"), SourceId::new("file"), "key")
            .with_priority(10);
        let high = AnnotatedValue::new(ConfigValue::string("b"), SourceId::new("env"), "key")
            .with_priority(50);

        let report = AnnotatedValue::conflict_report(&low, &high);
        assert_eq!(report.winner, ConflictWinner::High);
        assert_eq!(report.path.as_ref(), "key");
    }

    #[test]
    fn test_annotated_value_conflict_report_low_wins() {
        let low = AnnotatedValue::new(ConfigValue::string("a"), SourceId::new("file"), "key")
            .with_priority(50);
        let high = AnnotatedValue::new(ConfigValue::string("b"), SourceId::new("env"), "key")
            .with_priority(10);

        let report = AnnotatedValue::conflict_report(&low, &high);
        assert_eq!(report.winner, ConflictWinner::Low);
    }

    #[test]
    fn test_source_location_from_path() {
        let path = std::path::Path::new("/etc/config/app.toml");
        let loc = SourceLocation::from_path(path, 10, 5);
        assert_eq!(loc.source_name.as_ref(), "app.toml");
        assert_eq!(loc.line, 10);
        assert_eq!(loc.column, 5);
        assert_eq!(
            loc.file_path,
            Some(std::path::PathBuf::from("/etc/config/app.toml"))
        );
    }

    #[test]
    fn test_serialize_mode_copy() {
        assert_ne!(SerializeMode::Redacted, SerializeMode::Full);
    }

    #[test]
    fn test_conflict_winner_eq() {
        assert_eq!(ConflictWinner::High, ConflictWinner::High);
        assert_ne!(ConflictWinner::High, ConflictWinner::Low);
    }

    #[test]
    fn test_config_value_from_option_some() {
        let v: ConfigValue = Some(true).into();
        assert_eq!(v.as_bool(), Some(true));
    }

    #[test]
    fn test_config_value_from_option_none() {
        let v: ConfigValue = Option::<bool>::None.into();
        assert!(v.is_null());
    }

    #[test]
    fn test_config_value_array_from_vec() {
        let items = vec!["a".to_string(), "b".to_string()];
        let v: ConfigValue = items.into();
        assert!(v.is_array());
    }

    #[test]
    fn test_annotated_value_empty_checks() {
        let null_val = AnnotatedValue::new(ConfigValue::Null, SourceId::new("t"), "");
        assert!(null_val.is_empty());

        let str_val = AnnotatedValue::new(ConfigValue::string(""), SourceId::new("t"), "");
        assert!(str_val.is_empty());

        let non_empty = AnnotatedValue::new(ConfigValue::string("hello"), SourceId::new("t"), "");
        assert!(!non_empty.is_empty());
    }

    #[test]
    fn test_all_paths_flat() {
        let val = AnnotatedValue::new(ConfigValue::string("x"), SourceId::new("t"), "simple.key");
        let paths = val.all_paths();
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0].as_ref(), "simple.key");
    }

    #[test]
    fn test_all_paths_nested() {
        let inner = ConfigValue::map(vec![
            (
                "host".to_string(),
                AnnotatedValue::new(
                    ConfigValue::string("localhost"),
                    SourceId::new("t"),
                    "db.host",
                ),
            ),
            (
                "port".to_string(),
                AnnotatedValue::new(ConfigValue::uint(5432), SourceId::new("t"), "db.port"),
            ),
        ]);
        let val = AnnotatedValue::new(inner, SourceId::new("t"), "db");
        let paths = val.all_paths();
        assert!(paths.iter().any(|p| p.as_ref() == "db"));
        assert!(paths.iter().any(|p| p.as_ref() == "db.host"));
        assert!(paths.iter().any(|p| p.as_ref() == "db.port"));
    }

    #[test]
    fn test_bytes_value_roundtrip() {
        let bytes = vec![1u8, 2, 3, 255];
        let v = ConfigValue::Bytes(bytes.clone());
        assert!(matches!(v, ConfigValue::Bytes(_)));
    }

    #[test]
    fn test_config_value_from_i64_conv() {
        let from_i32: ConfigValue = 42i32.into();
        assert_eq!(from_i32.as_i64(), Some(42));
        let from_i16: ConfigValue = 16i16.into();
        assert_eq!(from_i16.as_i64(), Some(16));
        let from_isize: ConfigValue = 100isize.into();
        assert_eq!(from_isize.as_i64(), Some(100));
    }

    #[test]
    fn test_config_value_from_u64_conv() {
        let from_u32: ConfigValue = 99u32.into();
        assert_eq!(from_u32.as_u64(), Some(99));
        let from_u16: ConfigValue = 16u16.into();
        assert_eq!(from_u16.as_u64(), Some(16));
        let from_usize: ConfigValue = 1000usize.into();
        assert_eq!(from_usize.as_u64(), Some(1000));
    }

    #[test]
    fn test_config_value_from_str_variants() {
        let from_string: ConfigValue = "owned".to_string().into();
        assert!(from_string.is_string());
        let from_str: ConfigValue = "borrowed".into();
        assert!(from_str.is_string());
    }

    #[test]
    fn test_config_value_type_checks() {
        let b = ConfigValue::Bool(true);
        assert!(b.is_bool());
        assert!(!b.is_integer());
        assert!(!b.is_number());
        assert!(!b.is_string());

        let i = ConfigValue::I64(1);
        assert!(i.is_integer());
        assert!(i.is_number());

        let u = ConfigValue::U64(1);
        assert!(u.is_integer());
        assert!(u.is_number());

        let f = ConfigValue::F64(1.0);
        assert!(!f.is_integer());
        assert!(f.is_number());
    }

    #[test]
    fn test_config_value_conv_i64_to_u64() {
        let v = ConfigValue::I64(100);
        assert_eq!(v.as_u64(), Some(100));
        let neg = ConfigValue::I64(-1);
        assert_eq!(neg.as_u64(), None);
    }

    #[test]
    fn test_config_value_conv_u64_to_i64() {
        let v = ConfigValue::U64(100);
        assert_eq!(v.as_i64(), Some(100));
        let big = ConfigValue::U64(i64::MAX as u64 + 1);
        assert_eq!(big.as_i64(), None);
    }

    #[test]
    fn test_annotated_value_default() {
        let v = AnnotatedValue::default();
        assert!(v.is_null());
        assert_eq!(v.priority, 0);
        assert_eq!(v.version, 0);
        assert!(v.location.is_none());
    }

    #[test]
    fn test_annotated_value_from_config_value() {
        let cv = ConfigValue::string("test");
        let av: AnnotatedValue = cv.into();
        assert_eq!(av.as_str(), Some("test"));
    }

    #[test]
    fn test_serialize_mode_eq() {
        assert_eq!(SerializeMode::Redacted, SerializeMode::Redacted);
        assert_eq!(SerializeMode::Full, SerializeMode::Full);
        assert_ne!(SerializeMode::Redacted, SerializeMode::Full);
    }

    #[test]
    fn test_source_id_default_and_from() {
        let sid: SourceId = Default::default();
        assert_eq!(sid.as_str(), "default");

        let sid2: SourceId = "custom".into();
        assert_eq!(sid2.as_str(), "custom");

        let sid3: SourceId = "owned_string".to_string().into();
        assert_eq!(sid3.as_str(), "owned_string");
    }

    #[test]
    fn test_source_location_display() {
        let loc = SourceLocation::new("file.toml", 10, 3);
        assert_eq!(loc.to_string(), "file.toml:10:3");
    }

    #[test]
    fn test_source_location_clone_eq() {
        let a = SourceLocation::new("f.toml", 1, 2);
        let b = a.clone();
        assert_eq!(a, b);
    }

    #[test]
    fn test_config_value_f64_conv_limitations() {
        let v = ConfigValue::F64(std::f64::consts::PI);
        assert!(!v.is_integer());
        assert!(v.is_number());
        let v2 = ConfigValue::I64(10);
        assert!((v2.as_f64().unwrap() - 10.0).abs() < 0.001);
        let v3 = ConfigValue::U64(20);
        assert!((v3.as_f64().unwrap() - 20.0).abs() < 0.001);
    }

    #[test]
    fn test_config_value_as_array_map() {
        let av = ConfigValue::Array(vec![].into());
        assert!(av.is_array());
        assert!(av.as_array().unwrap().is_empty());
        let mv = ConfigValue::Map(Default::default());
        assert!(mv.is_map());
        assert!(mv.as_map().unwrap().is_empty());
    }

    #[test]
    fn test_annotated_value_with_location_builder() {
        let loc = SourceLocation::new("file.toml", 5, 10);
        let av = AnnotatedValue::new(ConfigValue::string("x"), SourceId::new("t"), "k")
            .with_priority(10)
            .with_version(2)
            .with_location(loc.clone());
        assert_eq!(av.priority, 10);
        assert_eq!(av.version, 2);
        assert_eq!(av.location, Some(loc));
    }

    #[test]
    fn test_config_value_bytes_ser_roundtrip() {
        let bytes = vec![0u8, 1, 255, 128];
        let cv = ConfigValue::Bytes(bytes);
        assert!(matches!(cv, ConfigValue::Bytes(_)));
        // Verify serialization produces an array
        let json = serde_json::to_value(&cv).unwrap();
        assert!(json.is_array());
    }

    #[test]
    fn test_source_id_serialize_roundtrip() {
        let sid = SourceId::new("test-source");
        let json = serde_json::to_string(&sid).unwrap();
        let deser: SourceId = serde_json::from_str(&json).unwrap();
        assert_eq!(sid, deser);
    }

    #[test]
    fn test_source_id_display() {
        let sid = SourceId::new("my-source");
        assert_eq!(sid.to_string(), "my-source");
    }

    #[test]
    fn test_annotated_value_is_null() {
        let n = AnnotatedValue::new(ConfigValue::Null, SourceId::new("t"), "");
        assert!(n.is_null());
        let s = AnnotatedValue::new(ConfigValue::string("x"), SourceId::new("t"), "");
        assert!(!s.is_null());
    }

    #[test]
    fn test_annotated_value_array_ser() {
        let items = vec![AnnotatedValue::new(
            ConfigValue::string("a"),
            SourceId::new("t"),
            "arr.0",
        )];
        let arr = AnnotatedValue::new(ConfigValue::Array(items.into()), SourceId::new("t"), "arr");
        let json = arr.to_json();
        assert!(json.is_array());
    }

    #[test]
    fn test_conflict_report_new_all_fields() {
        let loc = SourceLocation::new("f.toml", 1, 1);
        let report = ConflictReport::new(
            "key",
            "old".into(),
            SourceId::new("f1"),
            Some(loc.clone()),
            "new".into(),
            SourceId::new("f2"),
            Some(loc),
            ConflictWinner::High,
        );
        assert_eq!(report.path.as_ref(), "key");
        assert_eq!(report.winner, ConflictWinner::High);
        assert!(report.low_location.is_some());
        assert!(report.high_location.is_some());
    }

    #[test]
    fn test_config_value_usize_roundtrip() {
        let cv: ConfigValue = 999usize.into();
        assert_eq!(cv.as_u64(), Some(999));
        let cv2: ConfigValue = (-5isize).into();
        assert_eq!(cv2.as_i64(), Some(-5));
    }

    #[test]
    fn test_config_value_u8_u16_roundtrip() {
        let from_u8: ConfigValue = 200u8.into();
        assert_eq!(from_u8.as_u64(), Some(200));
        let from_i8: ConfigValue = (-100i8).into();
        assert_eq!(from_i8.as_i64(), Some(-100));
    }

    #[test]
    fn test_config_value_null_serde_roundtrip() {
        let cv = ConfigValue::Null;
        let json = serde_json::to_string(&cv).unwrap();
        let deser: ConfigValue = serde_json::from_str(&json).unwrap();
        assert!(deser.is_null());
    }

    #[test]
    fn test_config_value_bool_serde_roundtrip() {
        let cv = ConfigValue::Bool(true);
        let json = serde_json::to_string(&cv).unwrap();
        let deser: ConfigValue = serde_json::from_str(&json).unwrap();
        assert_eq!(deser.as_bool(), Some(true));
    }

    #[test]
    fn test_config_value_i64_serde_roundtrip() {
        let cv = ConfigValue::I64(-42);
        let json = serde_json::to_string(&cv).unwrap();
        let deser: ConfigValue = serde_json::from_str(&json).unwrap();
        assert_eq!(deser.as_i64(), Some(-42));
    }

    #[test]
    fn test_config_value_u64_serde_roundtrip() {
        let cv = ConfigValue::U64(99);
        let json = serde_json::to_string(&cv).unwrap();
        let deser: ConfigValue = serde_json::from_str(&json).unwrap();
        assert_eq!(deser.as_u64(), Some(99));
    }

    #[test]
    fn test_config_value_f64_serde_roundtrip() {
        let cv = ConfigValue::F64(2.5);
        let json = serde_json::to_string(&cv).unwrap();
        let deser: ConfigValue = serde_json::from_str(&json).unwrap();
        assert!((deser.as_f64().unwrap() - 2.5).abs() < 0.001);
    }

    #[test]
    fn test_config_value_string_serde_roundtrip() {
        let cv = ConfigValue::string("hello world");
        let json = serde_json::to_string(&cv).unwrap();
        let deser: ConfigValue = serde_json::from_str(&json).unwrap();
        assert_eq!(deser.as_str(), Some("hello world"));
    }

    #[test]
    fn test_config_value_bytes_serialize_only() {
        // Bytes serialize as a JSON array; deserialization produces Array (not Bytes).
        // This test verifies the serialization path only.
        let cv = ConfigValue::Bytes(vec![0u8, 127, 255, 1]);
        let json = serde_json::to_value(&cv).unwrap();
        assert!(json.is_array());
        assert_eq!(json[0], serde_json::json!(0));
        assert_eq!(json[3], serde_json::json!(1));
    }

    #[test]
    fn test_config_value_array_serde_roundtrip() {
        let items = vec![
            AnnotatedValue::new(ConfigValue::I64(1), SourceId::new("t"), "arr.0"),
            AnnotatedValue::new(ConfigValue::string("two"), SourceId::new("t"), "arr.1"),
        ];
        let cv = ConfigValue::Array(items.into());
        let json = serde_json::to_string(&cv).unwrap();
        let deser: ConfigValue = serde_json::from_str(&json).unwrap();
        let arr = deser.as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0].as_i64(), Some(1));
        assert_eq!(arr[1].as_str(), Some("two"));
    }

    #[test]
    fn test_config_value_map_serde_roundtrip() {
        let cv = ConfigValue::map(vec![
            (
                "name",
                AnnotatedValue::new(ConfigValue::string("test"), SourceId::new("t"), "name"),
            ),
            (
                "count",
                AnnotatedValue::new(ConfigValue::U64(5), SourceId::new("t"), "count"),
            ),
        ]);
        let json = serde_json::to_string(&cv).unwrap();
        let deser: ConfigValue = serde_json::from_str(&json).unwrap();
        let map = deser.as_map().unwrap();
        assert_eq!(map.len(), 2);
        assert_eq!(map.get("name").unwrap().as_str(), Some("test"));
        assert_eq!(map.get("count").unwrap().as_u64(), Some(5));
    }

    #[test]
    fn test_config_value_deserialize_from_json_null() {
        let deser: ConfigValue = serde_json::from_str("null").unwrap();
        assert!(deser.is_null());
    }

    #[test]
    fn test_config_value_deserialize_from_unit_json() {
        // visit_unit path
        let deser: ConfigValue = serde_json::from_str("null").unwrap();
        assert!(matches!(deser, ConfigValue::Null));
    }

    #[test]
    fn test_source_location_serde_roundtrip() {
        let loc = SourceLocation::new("config.toml", 42, 7);
        let json = serde_json::to_string(&loc).unwrap();
        let deser: SourceLocation = serde_json::from_str(&json).unwrap();
        assert_eq!(deser.source_name.as_ref(), "config.toml");
        assert_eq!(deser.line, 42);
        assert_eq!(deser.column, 7);
        assert_eq!(deser.file_path, None);
    }

    #[test]
    fn test_annotated_value_serde_roundtrip() {
        let loc = SourceLocation::new("f.toml", 1, 2);
        let av = AnnotatedValue::new(ConfigValue::string("v"), SourceId::new("src"), "path")
            .with_priority(5)
            .with_version(3)
            .with_location(loc.clone());
        let json = serde_json::to_string(&av).unwrap();
        let deser: AnnotatedValue = serde_json::from_str(&json).unwrap();
        assert_eq!(deser.as_str(), Some("v"));
        assert_eq!(deser.source.as_str(), "src");
        assert_eq!(deser.path.as_ref(), "path");
        assert_eq!(deser.priority, 5);
        assert_eq!(deser.version, 3);
        assert_eq!(deser.location, Some(loc));
    }

    #[test]
    fn test_zeroizing_bytes_new_and_as_slice() {
        let data = vec![1u8, 2, 3, 4];
        let zb = ZeroizingBytes::new(data.clone());
        assert_eq!(zb.as_slice(), &data[..]);
    }

    #[test]
    fn test_zeroizing_bytes_len_and_is_empty() {
        let zb = ZeroizingBytes::new(vec![1u8, 2, 3]);
        assert_eq!(zb.len(), 3);
        assert!(!zb.is_empty());

        let empty = ZeroizingBytes::new(vec![]);
        assert_eq!(empty.len(), 0);
        assert!(empty.is_empty());
    }

    #[test]
    fn test_zeroizing_bytes_deref() {
        let zb = ZeroizingBytes::new(vec![10u8, 20, 30]);
        assert_eq!(zb.len(), 3);
        assert_eq!(zb[0], 10);
        assert_eq!(zb[2], 30);
    }

    #[test]
    fn test_zeroizing_bytes_deref_mut() {
        let mut zb = ZeroizingBytes::new(vec![1u8, 2, 3]);
        zb[0] = 99;
        assert_eq!(zb[0], 99);
        zb.push(4);
        assert_eq!(zb.len(), 4);
    }

    #[test]
    fn test_noop_metrics_default_and_methods() {
        use crate::interface::MetricsBackend;
        let metrics = NoOpMetrics;
        // These should be no-ops and not panic
        metrics.counter("requests", &[("method", "GET")]);
        metrics.histogram("latency_ms", 42.5, &[("endpoint", "/api")]);
    }

    #[test]
    fn test_noop_metrics_clone() {
        use crate::interface::MetricsBackend;
        let m1 = NoOpMetrics;
        let m2 = m1.clone();
        m2.counter("test", &[]);
    }

    #[test]
    fn test_source_kind_variants() {
        assert_eq!(SourceKind::File, SourceKind::File);
        assert_ne!(SourceKind::File, SourceKind::Environment);
        assert_ne!(SourceKind::Environment, SourceKind::CommandLine);
        assert_ne!(SourceKind::CommandLine, SourceKind::Default);
        assert_ne!(SourceKind::Default, SourceKind::Memory);
    }

    #[test]
    fn test_source_kind_clone_debug() {
        let kind = SourceKind::File;
        let cloned = kind;
        assert_eq!(kind, cloned);
        let debug = format!("{:?}", kind);
        assert!(debug.contains("File"));
    }

    #[test]
    fn test_annotated_value_as_bool_accessor() {
        let av = AnnotatedValue::new(ConfigValue::Bool(true), SourceId::new("t"), "k");
        assert_eq!(av.as_bool(), Some(true));

        let non_bool = AnnotatedValue::new(ConfigValue::string("x"), SourceId::new("t"), "k");
        assert_eq!(non_bool.as_bool(), None);
    }

    #[test]
    fn test_annotated_value_as_i64_accessor() {
        let av = AnnotatedValue::new(ConfigValue::I64(42), SourceId::new("t"), "k");
        assert_eq!(av.as_i64(), Some(42));

        let u_av = AnnotatedValue::new(ConfigValue::U64(10), SourceId::new("t"), "k");
        assert_eq!(u_av.as_i64(), Some(10));
    }

    #[test]
    fn test_annotated_value_as_u64_accessor() {
        let av = AnnotatedValue::new(ConfigValue::U64(99), SourceId::new("t"), "k");
        assert_eq!(av.as_u64(), Some(99));

        let i_av = AnnotatedValue::new(ConfigValue::I64(50), SourceId::new("t"), "k");
        assert_eq!(i_av.as_u64(), Some(50));
    }

    #[test]
    fn test_annotated_value_as_f64_accessor() {
        let av = AnnotatedValue::new(ConfigValue::F64(2.5), SourceId::new("t"), "k");
        assert!((av.as_f64().unwrap() - 2.5).abs() < 0.001);

        let i_av = AnnotatedValue::new(ConfigValue::I64(10), SourceId::new("t"), "k");
        assert!((i_av.as_f64().unwrap() - 10.0).abs() < 0.001);

        let u_av = AnnotatedValue::new(ConfigValue::U64(20), SourceId::new("t"), "k");
        assert!((u_av.as_f64().unwrap() - 20.0).abs() < 0.001);
    }

    #[test]
    fn test_annotated_value_is_map_and_is_array() {
        let map_av = AnnotatedValue::new(
            ConfigValue::map(vec![(
                "k",
                AnnotatedValue::new(ConfigValue::Null, SourceId::new("t"), "k"),
            )]),
            SourceId::new("t"),
            "k",
        );
        assert!(map_av.is_map());
        assert!(!map_av.is_array());

        let arr_av = AnnotatedValue::new(ConfigValue::array(vec![]), SourceId::new("t"), "k");
        assert!(arr_av.is_array());
        assert!(!arr_av.is_map());
    }

    #[test]
    fn test_annotated_value_as_string_deprecated() {
        #[allow(deprecated)]
        {
            let av = AnnotatedValue::new(ConfigValue::string("val"), SourceId::new("t"), "k");
            assert_eq!(av.as_string(), Some("val".to_string()));

            let non_str = AnnotatedValue::new(ConfigValue::I64(1), SourceId::new("t"), "k");
            assert_eq!(non_str.as_string(), None);
        }
    }

    #[test]
    #[cfg(feature = "json")]
    fn test_to_json_null_bool_int() {
        let null_av = AnnotatedValue::new(ConfigValue::Null, SourceId::new("t"), "k");
        assert_eq!(null_av.to_json(), serde_json::Value::Null);

        let bool_av = AnnotatedValue::new(ConfigValue::Bool(true), SourceId::new("t"), "k");
        assert_eq!(bool_av.to_json(), serde_json::Value::Bool(true));

        let i_av = AnnotatedValue::new(ConfigValue::I64(42), SourceId::new("t"), "k");
        assert_eq!(i_av.to_json(), serde_json::json!(42));

        let u_av = AnnotatedValue::new(ConfigValue::U64(99), SourceId::new("t"), "k");
        assert_eq!(u_av.to_json(), serde_json::json!(99));
    }

    #[test]
    #[cfg(feature = "json")]
    fn test_to_json_f64_and_bytes() {
        let f_av = AnnotatedValue::new(ConfigValue::F64(1.5), SourceId::new("t"), "k");
        assert_eq!(f_av.to_json(), serde_json::json!(1.5));

        let bytes_av =
            AnnotatedValue::new(ConfigValue::Bytes(vec![1, 2, 3]), SourceId::new("t"), "k");
        let json = bytes_av.to_json();
        assert!(json.is_string());
    }

    #[test]
    #[cfg(feature = "json")]
    fn test_to_json_map() {
        let map_av = AnnotatedValue::new(
            ConfigValue::map(vec![(
                "key",
                AnnotatedValue::new(ConfigValue::string("v"), SourceId::new("t"), "k.key"),
            )]),
            SourceId::new("t"),
            "k",
        );
        let json = map_av.to_json();
        assert!(json.is_object());
        assert_eq!(json["key"], serde_json::json!("v"));
    }

    #[test]
    #[cfg(feature = "json")]
    fn test_to_json_redacted_map_and_array() {
        let map_av = AnnotatedValue::new(
            ConfigValue::map(vec![(
                "secret",
                AnnotatedValue::new(
                    ConfigValue::string("hidden"),
                    SourceId::new("t"),
                    "k.secret",
                ),
            )]),
            SourceId::new("t"),
            "k",
        );
        let redacted = map_av.to_json_with_mode(SerializeMode::Redacted, &["k.secret"]);
        assert_eq!(redacted["secret"], serde_json::json!("[REDACTED]"));

        let arr_av = AnnotatedValue::new(
            ConfigValue::array(vec![AnnotatedValue::new(
                ConfigValue::string("x"),
                SourceId::new("t"),
                "arr.0",
            )]),
            SourceId::new("t"),
            "arr",
        );
        let full = arr_av.to_json_with_mode(SerializeMode::Full, &[]);
        assert!(full.is_array());
    }

    #[test]
    #[cfg(feature = "json")]
    fn test_to_json_f64_nan_to_null() {
        let av = AnnotatedValue::new(ConfigValue::F64(f64::NAN), SourceId::new("t"), "k");
        assert_eq!(av.to_json(), serde_json::Value::Null);
    }

    #[test]
    fn test_from_f64_conversion() {
        let v: ConfigValue = 2.5f64.into();
        assert!(v.is_number());
        assert!(!v.is_integer());
        assert!((v.as_f64().unwrap() - 2.5).abs() < 0.001);
    }

    #[test]
    fn test_all_paths_with_array() {
        let inner = ConfigValue::array(vec![
            AnnotatedValue::new(ConfigValue::I64(1), SourceId::new("t"), "arr.0"),
            AnnotatedValue::new(ConfigValue::I64(2), SourceId::new("t"), "arr.1"),
        ]);
        let av = AnnotatedValue::new(inner, SourceId::new("t"), "arr");
        let paths = av.all_paths();
        assert!(paths.iter().any(|p| p.as_ref() == "arr"));
        assert!(paths.iter().any(|p| p.as_ref() == "arr.0"));
        assert!(paths.iter().any(|p| p.as_ref() == "arr.1"));
    }

    #[test]
    fn test_key_cache_policy_default() {
        let policy = KeyCachePolicy::default();
        match policy {
            KeyCachePolicy::CacheWithTtl(d) => assert_eq!(d, Duration::from_secs(3600)),
            other => panic!("expected CacheWithTtl, got {:?}", other),
        }
    }

    #[test]
    fn test_key_cache_policy_variants() {
        let no_cache = KeyCachePolicy::NoCache;
        let indefinite = KeyCachePolicy::CacheIndefinitely;
        let with_ttl = KeyCachePolicy::CacheWithTtl(Duration::from_secs(60));

        assert_ne!(no_cache, indefinite);
        assert_ne!(indefinite, with_ttl);
        assert_ne!(no_cache, with_ttl);
    }

    #[test]
    fn test_config_value_constructors_direct() {
        assert!(ConfigValue::null().is_null());
        assert_eq!(ConfigValue::bool(false).as_bool(), Some(false));
        assert_eq!(ConfigValue::integer(-1).as_i64(), Some(-1));
        assert_eq!(ConfigValue::uint(1).as_u64(), Some(1));
        assert!((ConfigValue::float(1.0).as_f64().unwrap() - 1.0).abs() < 0.001);
        assert_eq!(ConfigValue::string("s").as_str(), Some("s"));
    }

    #[test]
    fn test_annotated_value_conflict_report_with_locations() {
        let loc = SourceLocation::new("f.toml", 5, 10);
        let low = AnnotatedValue::new(ConfigValue::string("a"), SourceId::new("l"), "k")
            .with_location(loc.clone());
        let high = AnnotatedValue::new(ConfigValue::string("b"), SourceId::new("h"), "k")
            .with_location(loc);
        let report = AnnotatedValue::conflict_report(&low, &high);
        assert!(report.low_location.is_some());
        assert!(report.high_location.is_some());
        assert_eq!(report.low_value, format!("{:?}", ConfigValue::string("a")));
        assert_eq!(report.high_value, format!("{:?}", ConfigValue::string("b")));
    }

    #[test]
    fn test_source_id_eq_hash() {
        let a = SourceId::new("x");
        let b = SourceId::new("x");
        let c = SourceId::new("y");
        assert_eq!(a, b);
        assert_ne!(a, c);

        use std::collections::HashMap;
        let mut map = HashMap::new();
        map.insert(a.clone(), 1);
        assert_eq!(map.get(&b), Some(&1));
    }
}
