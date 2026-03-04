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

// Re-export MergeStrategy from merger module to avoid breaking existing code
pub use crate::merger::MergeStrategy;

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
}

impl Serialize for SourceLocation {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("SourceLocation", 3)?;
        s.serialize_field("source_name", &*self.source_name)?;
        s.serialize_field("line", &self.line)?;
        s.serialize_field("column", &self.column)?;
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
        }
    }

    /// Create from a path, extracting only the filename for privacy.
    pub fn from_path(path: &std::path::Path, line: usize, column: usize) -> Self {
        let source_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");
        Self::new(source_name, line, column)
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
    pub fn as_string(&self) -> Option<String> {
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

    /// Get all configuration paths from this value (including self).
    pub fn all_paths(&self) -> Vec<Arc<str>> {
        self.all_paths_internal(true)
    }

    /// Get all paths including this value (legacy method for compatibility).
    #[deprecated(
        since = "0.3.0",
        note = "Use all_paths() instead - this method is identical"
    )]
    pub fn all_paths_including_self(&self) -> Vec<Arc<str>> {
        self.all_paths_internal(true)
    }

    /// Internal implementation for all_paths methods.
    ///
    /// # Arguments
    /// * `include_self` - If true, includes the current path in the result
    fn all_paths_internal(&self, include_self: bool) -> Vec<Arc<str>> {
        let mut paths = if include_self {
            vec![self.path.clone()]
        } else {
            Vec::new()
        };
        let mut stack: Vec<&ConfigValue> = vec![&self.inner];
        // Start traversal from inner value with current path
        let start_path = self.path.clone();
        let mut path_stack: Vec<Arc<str>> = vec![start_path];

        // Iterative traversal using explicit stack to avoid stack overflow
        while let Some(value) = stack.pop() {
            let current_path = path_stack.pop().unwrap_or_else(|| self.path.clone());

            match value {
                ConfigValue::Map(map) => {
                    for (key, val) in map.iter() {
                        let new_path = if current_path.is_empty() {
                            key.clone()
                        } else {
                            let s = format!("{}.{}", current_path, key);
                            Arc::from(s.as_str())
                        };
                        paths.push(new_path.clone());
                        stack.push(&val.inner);
                        path_stack.push(new_path);
                    }
                }
                ConfigValue::Array(arr) => {
                    for (i, val) in arr.iter().enumerate() {
                        let new_path: Arc<str> = Arc::from(format!("{}.{}", current_path, i));
                        paths.push(new_path.clone());
                        stack.push(&val.inner);
                        path_stack.push(new_path);
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
    ///
    /// Note: Consider using the builder pattern via `Self::builder()` for better readability.
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

/// Builder for creating ConflictReport instances.
#[derive(Debug, Clone, Default)]
pub struct ConflictReportBuilder {
    path: Option<Arc<str>>,
    low_value: Option<String>,
    low_source: Option<SourceId>,
    low_location: Option<SourceLocation>,
    high_value: Option<String>,
    high_source: Option<SourceId>,
    high_location: Option<SourceLocation>,
    winner: Option<ConflictWinner>,
}

impl ConflictReportBuilder {
    /// Set the path of the conflicting value.
    pub fn path(mut self, path: impl Into<Arc<str>>) -> Self {
        self.path = Some(path.into());
        self
    }

    /// Set the low priority value and its source.
    pub fn low_value(mut self, value: String, source: SourceId) -> Self {
        self.low_value = Some(value);
        self.low_source = Some(source);
        self
    }

    /// Set the low priority value location.
    pub fn low_location(mut self, loc: SourceLocation) -> Self {
        self.low_location = Some(loc);
        self
    }

    /// Set the high priority value and its source.
    pub fn high_value(mut self, value: String, source: SourceId) -> Self {
        self.high_value = Some(value);
        self.high_source = Some(source);
        self
    }

    /// Set the high priority value location.
    pub fn high_location(mut self, loc: SourceLocation) -> Self {
        self.high_location = Some(loc);
        self
    }

    /// Set which value won.
    pub fn winner(mut self, winner: ConflictWinner) -> Self {
        self.winner = Some(winner);
        self
    }

    /// Build the ConflictReport.
    ///
    /// # Panics
    ///
    /// Panics if required fields (path, values, sources, winner) are not set.
    pub fn build(self) -> ConflictReport {
        ConflictReport::new(
            self.path.expect("path required"),
            self.low_value.expect("low_value required"),
            self.low_source.expect("low_source required"),
            self.low_location,
            self.high_value.expect("high_value required"),
            self.high_source.expect("high_source required"),
            self.high_location,
            self.winner.expect("winner required"),
        )
    }
}

impl ConflictReport {
    /// Create a builder for ConflictReport.
    pub fn builder() -> ConflictReportBuilder {
        ConflictReportBuilder::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

        let v = ConfigValue::float(3.14);
        assert!(v.is_number());
        assert_eq!(v.as_f64(), Some(3.14));

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
        let report = ConflictReport::builder()
            .path("database.host")
            .low_value("localhost".to_string(), SourceId::new("file1"))
            .high_value("127.0.0.1".to_string(), SourceId::new("file2"))
            .winner(ConflictWinner::High)
            .build();

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
}
