// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! Behavioral baselining and anomaly detection for polled remote sources.
//!
//! [`BaselinedSource`] decorates any [`PolledSource`] with a per-source
//! behavioral baseline (`SourceBaseline`): it learns the source's normal
//! latency, payload size, and schema shape over a learning window, then
//! classifies each poll against three anomalies — [`AnomalyType::SchemaDrift`]
//! (structure fingerprint changed), [`AnomalyType::SizeOutOfRange`] (payload
//! beyond the tolerated interval), and [`AnomalyType::LatencySpike`] (latency
//! far above the exponential moving average).
//!
//! The [`BaselinePolicy`] controls the behavior: learning window size, spike
//! multiplier, size tolerance, and whether an anomaly fails the poll
//! (`fail_closed`) or is only recorded (and audited).

use std::time::Duration;

use sha2::{Digest, Sha256};

use crate::types::ConfigValue;

/// EMA smoothing factor for the latency average (and the deviation/jitter
/// average). 0.3 keeps the estimate responsive while smoothing single-poll
/// noise.
const EMA_ALPHA: f64 = 0.3;

/// Per-source behavioral baseline learned from successful polls.
///
/// All fields are updated by [`SourceBaseline::observe`]; the struct is the
/// learner's state, not a policy (see [`BaselinePolicy`]).
#[derive(Debug, Clone)]
pub struct SourceBaseline {
    /// Successful polls observed so far (drives the learning window).
    pub samples: u64,
    /// Exponential moving average of poll latency in milliseconds.
    pub ema_latency_ms: f64,
    /// EMA of the absolute latency deviation (the "jitter" band).
    pub latency_jitter_ms: f64,
    /// Smallest observed payload size in bytes (only ever shrinks).
    pub size_min: usize,
    /// Largest observed payload size in bytes (only ever grows).
    pub size_max: usize,
    /// Structure fingerprint of the last observed value (see
    /// [`schema_fingerprint`]).
    pub schema_hash: [u8; 32],
}

impl Default for SourceBaseline {
    fn default() -> Self {
        Self {
            samples: 0,
            ema_latency_ms: 0.0,
            latency_jitter_ms: 0.0,
            size_min: usize::MAX,
            size_max: 0,
            schema_hash: [0u8; 32],
        }
    }
}

impl SourceBaseline {
    /// Record one successful poll: updates the latency EMA, the deviation
    /// (jitter) EMA, the monotonic size interval, the schema fingerprint,
    /// and the sample counter. Order matters: the EMA is updated first so
    /// the deviation measures against the previous average.
    pub fn observe(&mut self, latency: Duration, payload_len: usize, fp: [u8; 32]) {
        let latency_ms = latency.as_secs_f64() * 1000.0;
        let prev_ema = self.ema_latency_ms;
        if self.samples == 0 {
            self.ema_latency_ms = latency_ms;
        } else {
            self.ema_latency_ms = EMA_ALPHA * latency_ms + (1.0 - EMA_ALPHA) * prev_ema;
            let deviation = (latency_ms - prev_ema).abs();
            self.latency_jitter_ms =
                EMA_ALPHA * deviation + (1.0 - EMA_ALPHA) * self.latency_jitter_ms;
        }
        self.size_min = self.size_min.min(payload_len);
        self.size_max = self.size_max.max(payload_len);
        self.schema_hash = fp;
        self.samples += 1;
    }

    /// Whether the learning window is satisfied: anomalies are only judged
    /// against a baseline with at least `min_samples` observations.
    pub fn ready(&self, min_samples: u64) -> bool {
        self.samples >= min_samples
    }
}

/// Structure fingerprint of a [`ConfigValue`] tree: a canonical DFS walk
/// over "path segment + leaf type" pairs (map keys sorted), SHA-256 digested.
/// Value contents never enter the hash — only the structure does, so two
/// trees with the same shape but different values share a fingerprint.
pub fn schema_fingerprint(value: &ConfigValue) -> [u8; 32] {
    let mut hasher = Sha256::new();
    fingerprint_walk(value, &mut hasher);
    hasher.finalize().into()
}

/// Canonical leaf-type tags: the leaf kind is part of the structure, its
/// value is not.
fn fingerprint_walk(value: &ConfigValue, hasher: &mut Sha256) {
    match value {
        ConfigValue::Map(map) => {
            // Sorted keys make the walk order-independent.
            let mut keys: Vec<&str> = map.keys().map(|k| k.as_ref()).collect();
            keys.sort_unstable();
            hasher.update(b"{");
            for key in keys {
                hasher.update(key.as_bytes());
                let child = map.get(key).expect("key came from the map itself");
                fingerprint_walk(&child.inner, hasher);
            }
            hasher.update(b"}");
        }
        ConfigValue::Array(items) => {
            // Array order IS structure (index positions are meaningful).
            hasher.update(b"[");
            for item in items.iter() {
                fingerprint_walk(&item.inner, hasher);
            }
            hasher.update(b"]");
        }
        leaf => hasher.update(leaf_tag(leaf).as_bytes()),
    }
}

fn leaf_tag(leaf: &ConfigValue) -> &'static str {
    match leaf {
        ConfigValue::Null => "null",
        ConfigValue::Bool(_) => "bool",
        ConfigValue::I64(_) => "i64",
        ConfigValue::U64(_) => "u64",
        ConfigValue::F64(_) => "f64",
        ConfigValue::String(_) => "string",
        ConfigValue::Bytes(_) => "bytes",
        ConfigValue::Map(_) | ConfigValue::Array(_) => unreachable!("walked above"),
    }
}

/// Knobs for the baselining decorator. Serde defaults keep a fresh
/// [`BaselinePolicy`] conservative: enabled, a five-poll learning window,
/// a 10x latency spike multiplier, a ±300% size tolerance, and fail-closed.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct BaselinePolicy {
    /// Master switch: `false` turns all judgment off (only observe).
    pub enabled: bool,
    /// Polls a source needs before anomalies are judged at all.
    pub min_samples: u64,
    /// A poll is a `LatencySpike` when `latency > ema * multiplier + jitter`.
    pub latency_spike_multiplier: f64,
    /// Percentage the learned `[size_min, size_max]` interval is widened on
    /// both sides before a payload is judged `SizeOutOfRange`.
    pub size_tolerance_pct: u64,
    /// `true` — an anomaly fails the poll with
    /// [`ConfigError::InvalidValue { key: "remote.baseline" }`];
    /// `false` — the anomaly is only recorded (and audited), the poll
    /// result passes through.
    pub fail_closed: bool,
}

impl Default for BaselinePolicy {
    fn default() -> Self {
        Self {
            enabled: true,
            min_samples: 5,
            latency_spike_multiplier: 10.0,
            size_tolerance_pct: 300,
            fail_closed: true,
        }
    }
}

/// One classified anomaly from a single poll.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SourceAnomaly {
    /// The polled source's identifier.
    pub source_id: String,
    /// Which anomaly class fired (serialized snake_case).
    pub anomaly_type: AnomalyType,
    /// Human-readable deviation numbers for logs and audits.
    pub baseline_deviation: String,
}

/// The three anomaly classes, in judgment priority order: a structural
/// change invalidates the size/latency comparison, so it wins; size is
/// checked before latency because a size anomaly makes latency less
/// meaningful; at most one anomaly is reported per poll.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnomalyType {
    /// The structure fingerprint changed.
    SchemaDrift,
    /// The payload size left the tolerated interval.
    SizeOutOfRange,
    /// The latency exceeded `ema * multiplier + jitter`.
    LatencySpike,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::AnnotatedValue;

    fn annotated(v: ConfigValue) -> AnnotatedValue {
        AnnotatedValue::new(v, crate::types::SourceId::default(), "")
    }

    fn map(entries: Vec<(&str, AnnotatedValue)>) -> ConfigValue {
        let mut indexed = indexmap::IndexMap::new();
        for (k, v) in entries {
            indexed.insert(std::sync::Arc::from(k), v);
        }
        ConfigValue::Map(std::sync::Arc::new(indexed))
    }

    fn sample(latency_ms: f64, payload: usize) -> (Duration, usize) {
        (Duration::from_secs_f64(latency_ms / 1000.0), payload)
    }

    // ---- T002: SourceBaseline learning ----

    #[test]
    fn ema_follows_the_exact_recurrence() {
        let mut b = SourceBaseline::default();
        // First sample seeds the EMA.
        let (d1, _) = sample(40.0, 100);
        b.observe(d1, 100, [0u8; 32]);
        assert!(
            (b.ema_latency_ms - 40.0).abs() < 1e-9,
            "{}",
            b.ema_latency_ms
        );
        assert!((b.latency_jitter_ms - 0.0).abs() < 1e-9);

        // Second: ema = 0.3*80 + 0.7*40 = 52; deviation = |80-40| = 40 → jitter = 12.
        let (d2, _) = sample(80.0, 100);
        b.observe(d2, 100, [0u8; 32]);
        assert!(
            (b.ema_latency_ms - 52.0).abs() < 1e-9,
            "{}",
            b.ema_latency_ms
        );
        assert!(
            (b.latency_jitter_ms - 12.0).abs() < 1e-9,
            "{}",
            b.latency_jitter_ms
        );

        // Third: ema = 0.3*50 + 0.7*52 = 51.4; deviation = |50-52| = 2 → jitter = 9.0.
        let (d3, _) = sample(50.0, 100);
        b.observe(d3, 100, [0u8; 32]);
        assert!(
            (b.ema_latency_ms - 51.4).abs() < 1e-9,
            "{}",
            b.ema_latency_ms
        );
        assert!(
            (b.latency_jitter_ms - 9.0).abs() < 1e-9,
            "{}",
            b.latency_jitter_ms
        );
    }

    #[test]
    fn size_interval_only_expands() {
        let mut b = SourceBaseline::default();
        let (d, _) = sample(10.0, 500);
        b.observe(d, 500, [0u8; 32]);
        assert_eq!(b.size_min, 500);
        assert_eq!(b.size_max, 500);

        let (d, small) = sample(10.0, 200);
        b.observe(d, small, [0u8; 32]);
        assert_eq!(b.size_min, 200, "min only shrinks");
        assert_eq!(b.size_max, 500, "max keeps its ceiling");

        let (d, large) = sample(10.0, 900);
        b.observe(d, large, [0u8; 32]);
        assert_eq!(b.size_min, 200);
        assert_eq!(b.size_max, 900);
    }

    #[test]
    fn ready_window_boundary() {
        let mut b = SourceBaseline::default();
        assert!(!b.ready(1), "zero samples are never ready");
        for _ in 0..4 {
            let (d, n) = sample(1.0, 10);
            b.observe(d, n, [0u8; 32]);
        }
        assert!(!b.ready(5), "4 < 5");
        let (d, n) = sample(1.0, 10);
        b.observe(d, n, [0u8; 32]);
        assert!(b.ready(5), "5 >= 5");
        assert!(b.ready(4), "more than the floor is also ready");
    }

    // ---- T003: schema fingerprint ----

    #[test]
    fn same_structure_different_values_share_fingerprint() {
        let a = map(vec![
            ("host", annotated(ConfigValue::string("a.local"))),
            ("port", annotated(ConfigValue::uint(1))),
        ]);
        let b = map(vec![
            ("host", annotated(ConfigValue::string("zzz-other-value"))),
            ("port", annotated(ConfigValue::uint(999_999))),
        ]);
        assert_eq!(schema_fingerprint(&a), schema_fingerprint(&b));
    }

    #[test]
    fn key_order_does_not_change_fingerprint() {
        let a = map(vec![
            ("b", annotated(ConfigValue::uint(1))),
            ("a", annotated(ConfigValue::uint(2))),
        ]);
        let b = map(vec![
            ("a", annotated(ConfigValue::uint(2))),
            ("b", annotated(ConfigValue::uint(1))),
        ]);
        assert_eq!(schema_fingerprint(&a), schema_fingerprint(&b));
    }

    #[test]
    fn new_key_and_leaf_type_change_fingerprint() {
        let base = map(vec![("a", annotated(ConfigValue::string("x")))]);
        let with_new_key = map(vec![
            ("a", annotated(ConfigValue::string("x"))),
            ("b", annotated(ConfigValue::uint(1))),
        ]);
        assert_ne!(
            schema_fingerprint(&base),
            schema_fingerprint(&with_new_key),
            "adding a key is drift"
        );

        let changed_type = map(vec![("a", annotated(ConfigValue::uint(1)))]);
        assert_ne!(
            schema_fingerprint(&base),
            schema_fingerprint(&changed_type),
            "String→Int is drift"
        );
    }

    // ---- T004: policy + anomaly types ----

    #[test]
    fn missing_policy_fields_take_defaults() {
        let policy: BaselinePolicy =
            serde_json::from_str("{}").expect("missing fields must default");
        assert_eq!(policy, BaselinePolicy::default());
        assert!(policy.enabled);
        assert_eq!(policy.min_samples, 5);
        assert!((policy.latency_spike_multiplier - 10.0).abs() < 1e-9);
        assert_eq!(policy.size_tolerance_pct, 300);
        assert!(policy.fail_closed);
    }

    #[test]
    fn anomaly_type_serializes_snake_case() {
        assert_eq!(
            serde_json::to_string(&AnomalyType::LatencySpike).unwrap(),
            "\"latency_spike\""
        );
        assert_eq!(
            serde_json::to_string(&AnomalyType::SchemaDrift).unwrap(),
            "\"schema_drift\""
        );
        assert_eq!(
            serde_json::to_string(&AnomalyType::SizeOutOfRange).unwrap(),
            "\"size_out_of_range\""
        );
        let back: AnomalyType = serde_json::from_str("\"schema_drift\"").unwrap();
        assert_eq!(back, AnomalyType::SchemaDrift);
    }
}
