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

use super::poll::PolledSource;
use crate::error::{ConfigError, ConfigResult};
use crate::types::{AnnotatedValue, ConfigValue};

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

    /// Judge one poll against this baseline (T005): at most one anomaly,
    /// priority `SchemaDrift` > `SizeOutOfRange` > `LatencySpike` — a
    /// structural change invalidates the size/latency comparison, and a
    /// size anomaly makes the latency reading less meaningful. Returns the
    /// anomaly class plus a human-readable deviation for logs and audits.
    pub(crate) fn judge(
        &self,
        latency: Duration,
        payload_len: usize,
        fp: [u8; 32],
        policy: &BaselinePolicy,
    ) -> Option<(AnomalyType, String)> {
        if fp != self.schema_hash {
            return Some((
                AnomalyType::SchemaDrift,
                format!(
                    "structure fingerprint changed (expected {}, got {})",
                    hex_short(&self.schema_hash),
                    hex_short(&fp)
                ),
            ));
        }

        let (min, max) = {
            let factor = (policy.size_tolerance_pct as f64) / 100.0;
            let lo = (self.size_min as f64 * (1.0 - factor)) as usize;
            let hi = ((self.size_max as f64 * (1.0 + factor)) as usize).saturating_add(1);
            (lo, hi)
        };
        if payload_len < min || payload_len > max {
            return Some((
                AnomalyType::SizeOutOfRange,
                format!("payload {payload_len}B vs tolerated [{min}B, {max}B]"),
            ));
        }

        let latency_ms = latency.as_secs_f64() * 1000.0;
        let ceiling_ms =
            self.ema_latency_ms * policy.latency_spike_multiplier + self.latency_jitter_ms;
        if latency_ms > ceiling_ms {
            let ratio = if ceiling_ms > 0.0 {
                format!(
                    " (x{:.0})",
                    latency_ms / self.ema_latency_ms.max(f64::MIN_POSITIVE)
                )
            } else {
                String::new()
            };
            return Some((
                AnomalyType::LatencySpike,
                format!(
                    "latency {latency_ms:.0}ms vs ema {:.0}ms{ratio}",
                    self.ema_latency_ms
                ),
            ));
        }

        None
    }
}

/// Short hex form of a fingerprint (first 8 bytes) for deviation strings.
fn hex_short(hash: &[u8; 32]) -> String {
    hash.iter()
        .take(8)
        .map(|b| format!("{b:02x}"))
        .collect::<Vec<_>>()
        .join("")
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

/// A [`PolledSource`] decorator that learns each source's behavioral
/// baseline ([`SourceBaseline`], one bucket per [`SourceId`] in a
/// [`dashmap::DashMap`]) and classifies every poll against the
/// [`BaselinePolicy`].
///
/// Judgment rules:
/// - the inner source's own `Err` passes through untouched (a failing
///   source must not shift the baseline);
/// - `policy.enabled == false` or a still-learning baseline (window not
///   full) only observes and passes the poll through;
/// - a classified anomaly with `fail_closed` fails the poll with
///   [`ConfigError::InvalidValue { key: "remote.baseline" }`] **without**
///   observing it; with `fail_closed == false` the poll is observed and
///   passes through (record/audit only).
///
/// `payload_len` is the JSON serialization length of the polled tree — a
/// deterministic proxy for the remote payload size, computed from the same
/// value every consumer sees.
pub struct BaselinedSource<S: PolledSource> {
    inner: S,
    policy: BaselinePolicy,
    /// One learning bucket per polled source.
    baselines: std::sync::Arc<dashmap::DashMap<crate::types::SourceId, SourceBaseline>>,
    /// Receives a [`crate::audit::AuditEvent::SourceAnomaly`] on every
    /// classified anomaly (fail-closed and fail-open alike).
    #[cfg(feature = "audit")]
    audit_writer: Option<std::sync::Arc<crate::audit::AuditWriter>>,
}

impl<S: PolledSource> BaselinedSource<S> {
    /// Decorate `inner` with the given policy.
    pub fn new(inner: S, policy: BaselinePolicy) -> Self {
        Self {
            inner,
            policy,
            baselines: std::sync::Arc::new(dashmap::DashMap::new()),
            #[cfg(feature = "audit")]
            audit_writer: None,
        }
    }

    /// Inject the audit writer that receives a `SourceAnomaly` event for every
    /// anomaly this decorator classifies. A write failure is debug-logged and
    /// never changes the poll outcome.
    #[cfg(feature = "audit")]
    pub fn with_audit_writer(mut self, writer: std::sync::Arc<crate::audit::AuditWriter>) -> Self {
        self.audit_writer = Some(writer);
        self
    }

    /// The learned baseline for a source, if any polls were observed.
    pub fn baseline(&self, source_id: &crate::types::SourceId) -> Option<SourceBaseline> {
        self.baselines.get(source_id).map(|b| b.value().clone())
    }

    /// The configured policy (introspection for tests and diagnostics).
    pub fn policy(&self) -> &BaselinePolicy {
        &self.policy
    }

    /// Measure the payload size for baseline purposes.
    fn payload_len(value: &AnnotatedValue) -> usize {
        serde_json::to_vec(&value.inner)
            .map(|bytes| bytes.len())
            .unwrap_or(0)
    }
}

#[async_trait::async_trait]
impl<S: PolledSource> PolledSource for BaselinedSource<S> {
    async fn poll(&self) -> ConfigResult<AnnotatedValue> {
        let start = std::time::Instant::now();
        // Inner failures pass through untouched: a broken source is a real
        // outage signal, not an anomaly against a baseline built on success.
        let value = self.inner.poll().await?;
        let latency = start.elapsed();
        let fp = schema_fingerprint(&value.inner);
        let payload_len = Self::payload_len(&value);

        let sid = self.inner.source_id();
        let mut entry = self.baselines.entry(sid.clone()).or_default();
        let baseline = entry.value_mut();

        let anomaly = if self.policy.enabled && baseline.ready(self.policy.min_samples) {
            baseline.judge(latency, payload_len, fp, &self.policy)
        } else {
            None
        };
        if let Some((kind, deviation)) = &anomaly {
            // Audit every classified anomaly — fail-closed and fail-open
            // alike. The audit path never changes the poll outcome.
            #[cfg(feature = "audit")]
            if let Some(writer) = &self.audit_writer {
                emit_anomaly_audit(writer, &sid, *kind, deviation);
            }
            if self.policy.fail_closed {
                // Fail closed WITHOUT observing: an anomalous poll must not
                // shift the baseline it just deviated from.
                return Err(ConfigError::InvalidValue {
                    key: "remote.baseline".into(),
                    expected_type: format!("{} within baseline", kind_json(*kind)),
                    message: format!("remote source anomaly: {deviation}"),
                });
            }
            // Fail-open: record and let the value through.
            log_anomaly_warning(*kind, deviation);
        }

        baseline.observe(latency, payload_len, fp);
        Ok(value)
    }

    fn poll_interval(&self) -> Option<Duration> {
        self.inner.poll_interval()
    }

    fn source_id(&self) -> crate::types::SourceId {
        self.inner.source_id()
    }
}

fn kind_json(kind: AnomalyType) -> &'static str {
    match kind {
        AnomalyType::SchemaDrift => "schema_drift",
        AnomalyType::SizeOutOfRange => "size_out_of_range",
        AnomalyType::LatencySpike => "latency_spike",
    }
}

#[cfg(feature = "audit")]
fn emit_anomaly_audit(
    writer: &crate::audit::AuditWriter,
    source_id: &crate::types::SourceId,
    kind: AnomalyType,
    deviation: &str,
) {
    let event = crate::audit::AuditEvent::SourceAnomaly {
        source_id: source_id.as_str().to_string(),
        anomaly_type: kind_json(kind).to_string(),
        baseline_deviation: deviation.to_string(),
        timestamp: chrono::Utc::now(),
    };
    if let Err(e) = writer.write(event) {
        log::debug!(
            "{}",
            crate::i18n::tr_args(
                "log-remote-baseline-audit-write-failed",
                &[("reason", e.to_string())]
            )
        );
    }
}

fn log_anomaly_warning(kind: AnomalyType, deviation: &str) {
    log::warn!(
        "{}",
        crate::i18n::tr_args(
            "log-remote-baseline-anomaly",
            &[
                ("kind", kind_json(kind).to_string()),
                ("deviation", deviation.to_string()),
            ]
        )
    );
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

    // ---- T005: judgment logic ----

    fn learned_baseline() -> SourceBaseline {
        // 5 fast polls of ~100-byte payloads with one shape.
        let mut b = SourceBaseline::default();
        let fp = schema_fingerprint(&map(vec![("a", annotated(ConfigValue::string("x")))]));
        for _ in 0..5 {
            let (d, n) = sample(10.0, 100);
            b.observe(d, n, fp);
        }
        b
    }

    #[test]
    fn schema_drift_is_the_minimal_trigger() {
        let b = learned_baseline();
        let drifted = schema_fingerprint(&map(vec![
            ("a", annotated(ConfigValue::string("x"))),
            ("new", annotated(ConfigValue::uint(1))),
        ]));
        let (d, n) = sample(10.0, 100);
        let policy = BaselinePolicy::default();
        let (kind, deviation) = b.judge(d, n, drifted, &policy).expect("drift fires");
        assert_eq!(kind, AnomalyType::SchemaDrift);
        assert!(deviation.contains("fingerprint"), "{deviation}");
    }

    #[test]
    fn size_out_of_range_minimal_trigger() {
        let b = learned_baseline();
        let fp = schema_fingerprint(&map(vec![("a", annotated(ConfigValue::string("x")))]));
        let (d, _) = sample(10.0, 5000); // tolerated ceiling: 100 * (1+3.0) = 400 (+1)
        let policy = BaselinePolicy::default();
        let (kind, deviation) = b.judge(d, 5000, fp, &policy).expect("size fires");
        assert_eq!(kind, AnomalyType::SizeOutOfRange);
        assert!(deviation.contains("5000B"), "{deviation}");
    }

    #[test]
    fn latency_spike_minimal_trigger() {
        let b = learned_baseline();
        let fp = schema_fingerprint(&map(vec![("a", annotated(ConfigValue::string("x")))]));
        // ema 10ms, ceiling 10*10.0 + ~0 jitter ≈ 100ms → 1000ms fires.
        let (d, n) = sample(1000.0, 100);
        let policy = BaselinePolicy::default();
        let (kind, deviation) = b.judge(d, n, fp, &policy).expect("spike fires");
        assert_eq!(kind, AnomalyType::LatencySpike);
        assert!(deviation.contains("x"), "ratio in deviation: {deviation}");
    }

    #[test]
    fn drift_masks_size_and_spike_priority() {
        let b = learned_baseline();
        let drifted = schema_fingerprint(&map(vec![("other", annotated(ConfigValue::null()))]));
        // Latency AND size both out of range too — drift must win alone.
        let (d, _) = sample(5000.0, 999_999);
        let policy = BaselinePolicy::default();
        let (kind, _) = b
            .judge(d, 999_999, drifted, &policy)
            .expect("anomaly fires");
        assert_eq!(kind, AnomalyType::SchemaDrift, "drift masks the rest");
    }

    #[test]
    fn in_band_poll_yields_no_anomaly() {
        let b = learned_baseline();
        let fp = schema_fingerprint(&map(vec![("a", annotated(ConfigValue::string("x")))]));
        let (d, n) = sample(12.0, 150); // within ceiling and interval
        let policy = BaselinePolicy::default();
        assert!(b.judge(d, n, fp, &policy).is_none());
    }

    // ---- T006: BaselinedSource decorator ----

    /// Programmable mock: each `poll` sleeps `latency_ms` and returns the
    /// current payload (structure can be swapped mid-test).
    struct MockSource {
        latency_ms: u64,
        payload: std::sync::Mutex<ConfigValue>,
        id: crate::types::SourceId,
    }

    impl MockSource {
        fn new(id: &str, payload: ConfigValue) -> Self {
            Self {
                latency_ms: 1,
                payload: std::sync::Mutex::new(payload),
                id: crate::types::SourceId::new(id),
            }
        }
    }

    #[async_trait::async_trait]
    impl PolledSource for MockSource {
        async fn poll(&self) -> ConfigResult<AnnotatedValue> {
            tokio::time::sleep(Duration::from_millis(self.latency_ms)).await;
            let payload = self.payload.lock().unwrap().clone();
            Ok(AnnotatedValue::new(
                payload,
                crate::types::SourceId::default(),
                "",
            ))
        }

        fn poll_interval(&self) -> Option<Duration> {
            Some(Duration::from_secs(30))
        }

        fn source_id(&self) -> crate::types::SourceId {
            self.id.clone()
        }
    }

    const MOCK_SHAPE: fn() -> ConfigValue =
        || map(vec![("a", annotated(ConfigValue::string("x")))]);

    fn drifted_shape() -> ConfigValue {
        map(vec![
            ("a", annotated(ConfigValue::string("x"))),
            ("b", annotated(ConfigValue::uint(1))),
        ])
    }

    #[tokio::test]
    async fn fail_closed_rejects_without_observing() {
        let source = BaselinedSource::new(
            MockSource::new("mock-fc", MOCK_SHAPE()),
            BaselinePolicy::default(),
        );
        let sid = source.source_id();

        // Learning window: 5 accepted polls.
        for _ in 0..5 {
            source.poll().await.expect("in-window polls pass");
        }
        assert_eq!(source.baseline(&sid).unwrap().samples, 5);

        // Structural mutation → fail_closed Err, no observation.
        source
            .inner
            .payload
            .lock()
            .unwrap()
            .clone_from(&drifted_shape());
        let err = source.poll().await.expect_err("drift must fail closed");
        assert!(err.to_string().contains("remote.baseline"), "{err}");
        assert_eq!(
            source.baseline(&sid).unwrap().samples,
            5,
            "the rejected poll must not join the baseline"
        );
    }

    #[tokio::test]
    async fn learning_window_passes_drift_through() {
        let source = BaselinedSource::new(
            MockSource::new("mock-window", MOCK_SHAPE()),
            BaselinePolicy {
                min_samples: 5,
                ..BaselinePolicy::default()
            },
        );
        source
            .inner
            .payload
            .lock()
            .unwrap()
            .clone_from(&drifted_shape());
        // Only 2 samples: window not full → drift passes through.
        for _ in 0..2 {
            source
                .poll()
                .await
                .expect("in-window polls pass regardless of shape");
        }
        let sid = source.source_id();
        assert_eq!(source.baseline(&sid).unwrap().samples, 2);
    }

    #[tokio::test]
    async fn disabled_policy_only_observes() {
        let source = BaselinedSource::new(
            MockSource::new("mock-disabled", MOCK_SHAPE()),
            BaselinePolicy {
                enabled: false,
                ..BaselinePolicy::default()
            },
        );
        // Learn 5 first, then drift: disabled → pass through.
        for _ in 0..5 {
            source.poll().await.expect("learning polls pass");
        }
        source
            .inner
            .payload
            .lock()
            .unwrap()
            .clone_from(&drifted_shape());
        source.poll().await.expect("disabled policy never rejects");
        let sid = source.source_id();
        assert_eq!(source.baseline(&sid).unwrap().samples, 6, "still observed");
    }

    #[tokio::test]
    async fn fail_open_records_and_passes() {
        let source = BaselinedSource::new(
            MockSource::new("mock-open", MOCK_SHAPE()),
            BaselinePolicy {
                fail_closed: false,
                ..BaselinePolicy::default()
            },
        );
        for _ in 0..5 {
            source.poll().await.expect("learning polls pass");
        }
        source
            .inner
            .payload
            .lock()
            .unwrap()
            .clone_from(&drifted_shape());
        source
            .poll()
            .await
            .expect("fail_open lets the drifted poll through");
        let sid = source.source_id();
        assert_eq!(
            source.baseline(&sid).unwrap().samples,
            6,
            "fail_open anomaly is observed too"
        );
    }

    #[tokio::test]
    async fn inner_err_passes_through_without_touching_baseline() {
        struct ErrSource;

        #[async_trait::async_trait]
        impl PolledSource for ErrSource {
            async fn poll(&self) -> ConfigResult<AnnotatedValue> {
                Err(ConfigError::InvalidValue {
                    key: "mock".into(),
                    expected_type: "response".into(),
                    message: "boom".into(),
                })
            }

            fn poll_interval(&self) -> Option<Duration> {
                None
            }

            fn source_id(&self) -> crate::types::SourceId {
                crate::types::SourceId::new("mock-err")
            }
        }

        let source = BaselinedSource::new(ErrSource, BaselinePolicy::default());
        let err = source.poll().await.expect_err("inner Err passes through");
        assert!(err.to_string().contains("boom"), "{err}");
        let sid = source.source_id();
        assert!(
            source.baseline(&sid).is_none(),
            "failed polls must not create or touch a baseline"
        );
    }

    #[tokio::test]
    async fn poll_interval_and_source_id_pass_through() {
        let inner = MockSource::new("mock-ids", MOCK_SHAPE());
        let expected_interval = inner.poll_interval();
        let expected_id = inner.source_id();
        let source = BaselinedSource::new(inner, BaselinePolicy::default());
        assert_eq!(source.poll_interval(), expected_interval);
        assert_eq!(source.source_id(), expected_id);
    }

    #[tokio::test]
    async fn latency_spike_fails_closed() {
        let mut source = BaselinedSource::new(
            MockSource::new("mock-spike", MOCK_SHAPE()),
            BaselinePolicy {
                latency_spike_multiplier: 5.0,
                ..BaselinePolicy::default()
            },
        );
        for _ in 0..5 {
            source.poll().await.expect("fast learning polls pass");
        }
        // Now 100x slower than the ~1ms EMA → spike.
        source.inner.latency_ms = 100;
        let err = source
            .poll()
            .await
            .expect_err("a latency spike must fail closed");
        assert!(err.to_string().contains("remote.baseline"), "{err}");
    }

    // ---- T009: audit emission ----

    /// Collect every `SourceAnomaly` event line written into `dir`.
    #[cfg(feature = "audit")]
    fn anomaly_records(dir: &std::path::Path) -> Vec<serde_json::Value> {
        let mut out = Vec::new();
        for entry in std::fs::read_dir(dir).expect("read audit dir").flatten() {
            let content = std::fs::read_to_string(entry.path()).unwrap_or_default();
            for line in content.lines() {
                let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
                    continue;
                };
                if value.get("SourceAnomaly").is_some() {
                    out.push(value);
                }
            }
        }
        out
    }

    #[cfg(feature = "audit")]
    fn auditing_source(dir: &std::path::Path, fail_closed: bool) -> BaselinedSource<MockSource> {
        let writer = std::sync::Arc::new(crate::audit::AuditWriter::with_config(
            crate::audit::AuditConfig {
                enabled: true,
                log_dir: Some(dir.to_path_buf()),
                hmac_key: None,
            },
        ));
        BaselinedSource::new(
            MockSource::new("mock-audit", MOCK_SHAPE()),
            BaselinePolicy {
                fail_closed,
                ..BaselinePolicy::default()
            },
        )
        .with_audit_writer(writer)
    }

    #[cfg(feature = "audit")]
    #[tokio::test]
    async fn schema_drift_writes_source_anomaly_and_fails_closed() {
        let dir = tempfile::tempdir().expect("tempdir");
        let source = auditing_source(dir.path(), true);

        for _ in 0..5 {
            source.poll().await.expect("learning polls pass");
        }
        assert!(
            anomaly_records(dir.path()).is_empty(),
            "no anomaly while learning"
        );

        source
            .inner
            .payload
            .lock()
            .unwrap()
            .clone_from(&drifted_shape());
        let err = source.poll().await.expect_err("drift fails closed");
        assert!(err.to_string().contains("remote.baseline"), "{err}");

        let records = anomaly_records(dir.path());
        assert_eq!(records.len(), 1, "exactly one anomaly event is audited");
        let event = records[0]
            .get("SourceAnomaly")
            .and_then(|v| v.as_object())
            .expect("SourceAnomaly payload");
        assert_eq!(
            event.get("source_id").and_then(|v| v.as_str()),
            Some("mock-audit")
        );
        assert_eq!(
            event.get("anomaly_type").and_then(|v| v.as_str()),
            Some("schema_drift")
        );
        assert!(
            event
                .get("baseline_deviation")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .contains("fingerprint"),
            "human-readable deviation is audited"
        );
    }

    #[cfg(feature = "audit")]
    #[tokio::test]
    async fn fail_open_still_writes_source_anomaly() {
        let dir = tempfile::tempdir().expect("tempdir");
        let source = auditing_source(dir.path(), false);

        for _ in 0..5 {
            source.poll().await.expect("learning polls pass");
        }
        source
            .inner
            .payload
            .lock()
            .unwrap()
            .clone_from(&drifted_shape());
        source
            .poll()
            .await
            .expect("fail-open lets the drifted poll through");

        let records = anomaly_records(dir.path());
        assert_eq!(records.len(), 1, "fail-open anomalies are audited too");
        let kind = records[0]
            .get("SourceAnomaly")
            .and_then(|v| v.get("anomaly_type"))
            .and_then(|v| v.as_str());
        assert_eq!(kind, Some("schema_drift"));
    }
}
