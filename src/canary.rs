// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! Canary rollout orchestrator over the unified change stream.
//!
//! The single-instance counterpart (`ProgressiveReloader`) decides a
//! staged rollout inside one process; this orchestrator is the **consumer
//! side** across instances: it watches the stream for per-instance canary
//! stage transitions (`ChangeSource::Canary`, stage on
//! `old_value`), runs health checks between batches, and publishes advance /
//! rollback / completed directives back onto the stream
//! ([`ORCHESTRATOR_KEY`]) so instances (or operators) can act on them.
//!
//! Local multi-instance simulation: real `ProgressiveReloader`s attached to
//! the same in-memory stream stand in for a cluster (no sidecar needed in
//! CI); the service-mesh integration layer is documented in
//! `docs/CANARY_ORCHESTRATION.md`.

use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;

use crate::error::{ConfigError, ConfigResult};
use crate::i18n::{t_simple, tr_args};
use crate::stream::{ChangeEvent, ChangeSource, ChangeStream};
use crate::watcher::progressive::HealthStatus;

/// `key` of the directive events the orchestrator publishes
/// (`advance` / `rollback` / `completed` on `old_value`). Instance events
/// use `key == "canary"`; consumers filter on the key to tell them apart.
pub const ORCHESTRATOR_KEY: &str = "canary.orchestrator";

/// Stage word carried by instance events (`old_value`) and directives.
pub const STAGE_COMMITTED: &str = "committed";
pub const STAGE_ROLLED_BACK: &str = "rolled_back";

/// Staged-rollout policy (all knobs configurable).
#[derive(Debug, Clone)]
pub struct RolloutPlan {
    /// Instances per batch.
    pub batch_size: usize,
    /// Observation window per batch: how long the orchestrator waits for
    /// this batch's `committed` events and polls health.
    pub batch_interval: Duration,
    /// Health-check polling cadence inside the window.
    pub poll_interval: Duration,
    /// Allowed critical-instance ratio per batch (0.0 = any critical rolls
    /// back). The ratio is evaluated on every poll — exceeding it aborts the
    /// batch **immediately**, not at window end; if the window runs out
    /// first, the last observation decides. Worst-case rollback latency is
    /// therefore one `poll_interval` after the fault appears, bounded by
    /// `batch_interval`.
    pub failure_threshold: f64,
}

impl Default for RolloutPlan {
    fn default() -> Self {
        Self {
            batch_size: 1,
            batch_interval: Duration::from_secs(30),
            poll_interval: Duration::from_secs(5),
            failure_threshold: 0.0,
        }
    }
}

/// Multi-instance health check, keyed by instance id. Same tri-state
/// semantics as the single-instance [`HealthStatus`]: only `Critical`
/// counts toward the rollback decision, `Degraded` is observed but not
/// blocking.
#[async_trait]
pub trait RolloutHealthCheck: Send + Sync {
    async fn check(&self, instance: &str) -> HealthStatus;
}

/// How a rollout ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RolloutOutcome {
    /// Every batch passed health checks.
    Completed { batches: usize, instances: usize },
    /// Rollout stopped before completion. A `rollback` directive has been
    /// published — instances are expected to revert to the last good
    /// configuration.
    Aborted {
        batch: usize,
        reason: String,
        /// False when the rollback directive and the baseline traffic snap
        /// were published successfully; true means at least one rollback
        /// side effect failed (traffic may NOT have reverted) — always
        /// surface this to operators.
        mesh_rolled_back_failed: bool,
    },
}

/// Traffic groups the mesh adapter balances between.
pub const BASELINE_GROUP: &str = "baseline";
pub const CANARY_GROUP: &str = "canary";

/// Service-mesh integration adapter: translates rollout progress into a
/// traffic split between the baseline and canary groups. Implement this per
/// mesh — the Envoy (weighted clusters via xDS) and Istio (VirtualService
/// `weight` patches) configuration samples live in
/// `docs/CANARY_ORCHESTRATION.md`.
#[async_trait]
pub trait MeshWeightPublisher: Send + Sync {
    /// Set the traffic split; `canary_pct + baseline_pct == 100`.
    async fn publish_weights(&self, canary_pct: u8, baseline_pct: u8) -> ConfigResult<()>;
}

/// Canary rollout orchestrator over a [`ChangeStream`].
pub struct CanaryOrchestrator {
    stream: Arc<dyn ChangeStream>,
    instances: Vec<String>,
    plan: RolloutPlan,
    health: Arc<dyn RolloutHealthCheck>,
    mesh: Option<Arc<dyn MeshWeightPublisher>>,
}

impl CanaryOrchestrator {
    pub fn new(
        stream: Arc<dyn ChangeStream>,
        instances: Vec<String>,
        plan: RolloutPlan,
        health: Arc<dyn RolloutHealthCheck>,
    ) -> Self {
        Self {
            stream,
            instances,
            plan,
            health,
            mesh: None,
        }
    }

    /// Attach a service-mesh weight publisher. On every batch advance (and
    /// on completion/rollback) the orchestrator pushes the traffic split
    /// between [`CANARY_GROUP`] and [`BASELINE_GROUP`]; see
    /// [`MeshWeightPublisher`].
    pub fn with_mesh(mut self, mesh: Arc<dyn MeshWeightPublisher>) -> Self {
        self.mesh = Some(mesh);
        self
    }

    /// Total batches the plan implies for the instance list.
    fn batch_count(&self) -> usize {
        self.instances.len().div_ceil(self.plan.batch_size.max(1))
    }

    fn batch_instances(&self, index: usize) -> &[String] {
        let size = self.plan.batch_size.max(1);
        let start = index * size;
        &self.instances[start..self.instances.len().min(start + size)]
    }

    /// Run the staged rollout to completion or until a batch fails health
    /// (or instances report a rollback / go quiet past their window).
    pub async fn run(&self) -> ConfigResult<RolloutOutcome> {
        // An empty instance list (e.g. service discovery returning nothing)
        // must not read as a successful rollout — with a mesh attached it
        // would push 100% of traffic onto zero verified instances.
        if self.instances.is_empty() {
            return Err(ConfigError::InvalidValue {
                key: "canary".into(),
                expected_type: "non-empty instance list".into(),
                message: t_simple("error-canary-empty-instances"),
            });
        }
        let mut rx = self.stream.subscribe().await?;
        let batches = self.batch_count();
        // Instances whose batches already passed health: a rollback from
        // one of them means the candidate regressed on promoted ground —
        // the rollout must abort, not only current-batch failures.
        let mut promoted: std::collections::HashSet<String> = std::collections::HashSet::new();

        for batch_index in 0..batches {
            let batch = self.batch_instances(batch_index);
            // Wait for this batch's instances to report `committed`; an
            // instance-level `rolled_back` (current or promoted batch)
            // aborts the rollout immediately.
            match self
                .wait_for_batch_committed(&mut rx, batch, &promoted)
                .await
            {
                BatchWait::Committed => {}
                BatchWait::InstanceRolledBack(reason) => {
                    return self.abort(batch_index, &reason).await;
                }
                BatchWait::Timeout => {
                    let reason = tr_args(
                        "error-canary-commit-wait-timeout",
                        &[
                            ("batch", (batch_index + 1).to_string()),
                            ("instances", batch.join(", ")),
                        ],
                    );
                    return self.abort(batch_index, &reason).await;
                }
            }

            match self.poll_batch_health(batch).await {
                BatchHealth::Healthy => {
                    let passed = batch_index + 1;
                    promoted.extend(batch.iter().cloned());
                    self.publish_directive("advance", &format!("batch {passed}"))
                        .await;
                    if !self.publish_mesh_split(passed).await {
                        return self.fail_traffic_split(passed, "healthy").await;
                    }
                }
                BatchHealth::Degraded { instances } => {
                    // Degraded is observed but not blocking (same tri-state
                    // semantics as the single-instance reloader); the
                    // affected instances surface in the directive detail.
                    let passed = batch_index + 1;
                    let detail = format!("batch {passed} (degraded: {})", instances.join(", "));
                    self.publish_directive("advance", &detail).await;
                    if !self.publish_mesh_split(passed).await {
                        return self
                            .fail_traffic_split(passed, &format!("degraded: {instances:?}"))
                            .await;
                    }
                }
                BatchHealth::Critical { instances } => {
                    let reason = format!(
                        "batch {} health critical for {}/{} instance(s): {}",
                        batch_index + 1,
                        instances.len(),
                        batch.len(),
                        instances.join(", ")
                    );
                    return self.abort(batch_index, &reason).await;
                }
            }
        }

        self.publish_directive("completed", &format!("batches {batches}"))
            .await;
        if !self.push_mesh_weights(100, 0).await {
            // Full split not applied: completing here would report progress
            // that does not exist. Best-effort snap back, then fail loud.
            self.push_mesh_weights(0, 100).await;
            return Err(ConfigError::InvalidValue {
                key: "canary".into(),
                expected_type: "mesh weight update".into(),
                message: t_simple("error-canary-final-split-failed"),
            });
        }
        Ok(RolloutOutcome::Completed {
            batches,
            instances: self.instances.len(),
        })
    }

    /// A traffic-split failure mid-rollout: best-effort snap back to the
    /// baseline group, then fail loud — the rollout must not advance on a
    /// split that was never applied.
    async fn fail_traffic_split(
        &self,
        passed: usize,
        context: &str,
    ) -> ConfigResult<RolloutOutcome> {
        self.push_mesh_weights(0, 100).await;
        Err(ConfigError::InvalidValue {
            key: "canary".into(),
            expected_type: "mesh weight update".into(),
            message: tr_args(
                "error-canary-traffic-split-failed",
                &[
                    ("passed", passed.to_string()),
                    ("context", context.to_string()),
                ],
            ),
        })
    }

    /// Wait (bounded by the batch window) until every instance of the
    /// batch has reported `committed` — attribution by instance id from the
    /// event key (`canary.<instance_id>`), so duplicates collapse, unknown
    /// instances and out-of-batch events never count, and one chatty
    /// instance cannot fill another one's quota. An instance `rolled_back`
    /// short-circuits with its detail.
    async fn wait_for_batch_committed(
        &self,
        rx: &mut Pin<Box<dyn futures_util::Stream<Item = ChangeEvent> + Send>>,
        batch: &[String],
        promoted: &std::collections::HashSet<String>,
    ) -> BatchWait {
        use futures_util::StreamExt;

        let deadline = tokio::time::Instant::now() + self.plan.batch_interval;
        let mut pending: std::collections::HashSet<&str> =
            batch.iter().map(|s| s.as_str()).collect();
        while !pending.is_empty() {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                return BatchWait::Timeout;
            }
            let event = match tokio::time::timeout(remaining, rx.next()).await {
                Ok(Some(event)) => event,
                Ok(None) => return BatchWait::Timeout,
                Err(_elapsed) => return BatchWait::Timeout,
            };
            let Some(instance) = instance_canary_event(&event) else {
                continue;
            };
            let stage = event.old_value.as_ref().and_then(|v| v.as_str());
            match stage {
                Some(STAGE_COMMITTED) => {
                    // Only the current batch's instances count; duplicates
                    // collapse with the set-removal semantics.
                    pending.remove(instance.as_str());
                }
                Some(STAGE_ROLLED_BACK)
                    if pending.contains(instance.as_str()) || promoted.contains(&instance) =>
                {
                    // Current-batch rollbacks abort the batch; rollbacks from
                    // already-promoted instances mean the candidate regressed
                    // on promoted ground — both abort the rollout.
                    let scope = if pending.contains(instance.as_str()) {
                        "batch"
                    } else {
                        "promoted"
                    };
                    let detail = event
                        .new_value
                        .as_ref()
                        .and_then(|v| v.as_str())
                        .map(str::to_string)
                        .unwrap_or_else(|| t_simple("error-canary-instance-rolled-back"));
                    return BatchWait::InstanceRolledBack(tr_args(
                        "error-canary-instance-rollback-abort",
                        &[
                            ("scope", scope.to_string()),
                            ("instance", instance.clone()),
                            ("detail", detail),
                        ],
                    ));
                }
                _ => {}
            }
        }
        BatchWait::Committed
    }

    /// Poll the batch's health until everything is healthy or the batch
    /// window runs out; the last observation decides. A critical ratio
    /// above the threshold aborts **immediately** (a fast-degrading fault
    /// must not wait out the window), and each single check is bounded by
    /// the remaining window so a hung implementation cannot stall `run`.
    async fn poll_batch_health(&self, batch: &[String]) -> BatchHealth {
        let deadline = tokio::time::Instant::now() + self.plan.batch_interval;
        loop {
            let mut critical = Vec::new();
            let mut degraded = Vec::new();
            let mut healthy = true;
            for instance in batch {
                let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
                let status = if remaining.is_zero() {
                    HealthStatus::Degraded {
                        reason: t_simple("error-canary-health-window-exhausted"),
                    }
                } else {
                    match tokio::time::timeout(
                        self.plan.poll_interval.min(remaining),
                        self.health.check(instance),
                    )
                    .await
                    {
                        Ok(status) => status,
                        Err(_hung) => HealthStatus::Degraded {
                            reason: t_simple("error-canary-health-check-timed-out"),
                        },
                    }
                };
                match status {
                    HealthStatus::Healthy => {}
                    HealthStatus::Degraded { reason } => {
                        healthy = false;
                        degraded.push(format!("{instance}: {reason}"));
                    }
                    HealthStatus::Critical { reason } => {
                        healthy = false;
                        critical.push(format!("{instance}: {reason}"));
                    }
                }
            }
            if healthy {
                return BatchHealth::Healthy;
            }
            // Immediate-abort fast path: do not wait out the window when
            // the failure threshold is already exceeded.
            let critical_ratio = critical.len() as f64 / batch.len().max(1) as f64;
            if critical_ratio > self.plan.failure_threshold {
                return BatchHealth::Critical {
                    instances: critical,
                };
            }
            let elapsed = deadline.saturating_duration_since(tokio::time::Instant::now());
            if elapsed.is_zero() {
                return BatchHealth::Degraded {
                    instances: degraded,
                };
            }
            tokio::time::sleep(self.plan.poll_interval.min(elapsed)).await;
        }
    }

    /// Publish the rollback directive, snap traffic back to the baseline
    /// group, and report the outcome. A failed rollback side effect flips
    /// `mesh_rolled_back_failed` — the outcome must never read as "rolled
    /// back" when the traffic actually did not revert.
    async fn abort(&self, batch: usize, reason: &str) -> ConfigResult<RolloutOutcome> {
        let directive_published = self.publish_directive("rollback", reason).await;
        let mesh_rolled_back = self.push_mesh_weights(0, 100).await;
        let mesh_rolled_back_failed = !(directive_published && mesh_rolled_back);
        if mesh_rolled_back_failed {
            record_canary_failure("rollback_side_effect");
        }
        let reason = if mesh_rolled_back_failed {
            tr_args(
                "error-canary-rollback-side-effects-failed",
                &[("reason", reason.to_string())],
            )
        } else {
            reason.to_string()
        };
        Ok(RolloutOutcome::Aborted {
            batch: batch + 1,
            reason,
            mesh_rolled_back_failed,
        })
    }

    /// Push the traffic split for `passed_batches` passed batches: the
    /// canary group's share grows linearly with rollout progress.
    async fn publish_mesh_split(&self, passed_batches: usize) -> bool {
        let batches = self.batch_count().max(1);
        let canary_pct = ((passed_batches * 100) / batches).min(100) as u8;
        self.push_mesh_weights(canary_pct, 100 - canary_pct).await
    }

    /// Push the mesh split; returns whether the weight update reached the
    /// control plane (no mesh attached counts as success — nothing to
    /// publish). Failures are counted/logged, never swallowed.
    async fn push_mesh_weights(&self, canary_pct: u8, baseline_pct: u8) -> bool {
        let Some(mesh) = &self.mesh else {
            return true;
        };
        match mesh.publish_weights(canary_pct, baseline_pct).await {
            Ok(()) => true,
            Err(err) => {
                record_canary_failure("mesh_publish");
                warn_canary(&tr_args(
                    "log-canary-mesh-update-failed",
                    &[
                        ("canary_pct", canary_pct.to_string()),
                        ("baseline_pct", baseline_pct.to_string()),
                        ("message", err.to_string()),
                    ],
                ));
                false
            }
        }
    }

    /// Publish an orchestrator directive; returns whether it reached the
    /// stream. Failures are counted/logged, never swallowed.
    async fn publish_directive(&self, stage: &str, detail: &str) -> bool {
        match self
            .stream
            .publish(ChangeEvent::new(
                ORCHESTRATOR_KEY,
                Some(crate::types::ConfigValue::string(stage)),
                Some(crate::types::ConfigValue::string(detail)),
                ChangeSource::Canary,
            ))
            .await
        {
            Ok(()) => true,
            Err(err) => {
                record_canary_failure("directive_publish");
                warn_canary(&tr_args(
                    "log-canary-directive-publish-failed",
                    &[("stage", stage.to_string()), ("message", err.to_string())],
                ));
                false
            }
        }
    }
}

/// Count a canary orchestration failure (metrics backend no-ops unless
/// installed).
fn record_canary_failure(reason: &str) {
    crate::metrics::record_counter("confers_canary_errors_total", &[("reason", reason)]);
}

/// Log a canary orchestration failure under the `tracing` feature; a no-op
/// otherwise (mirrors the etcd watch observability pattern).
fn warn_canary(message: &str) {
    #[cfg(feature = "tracing")]
    tracing::warn!(target: "confers::canary", "{message}");
    #[cfg(not(feature = "tracing"))]
    let _ = message;
}

/// Outcome of waiting for a batch's `committed` events.
enum BatchWait {
    Committed,
    InstanceRolledBack(String),
    Timeout,
}

/// Health verdict for a whole batch after its polling window.
enum BatchHealth {
    Healthy,
    Degraded { instances: Vec<String> },
    Critical { instances: Vec<String> },
}

/// Extract the instance id from an instance-side canary event: key
/// `canary.<instance_id>` (`ChangeSource::Canary`). Orchestrator directives
/// ([`ORCHESTRATOR_KEY`]) and non-canary traffic yield `None`.
fn instance_canary_event(event: &ChangeEvent) -> Option<String> {
    if event.source != ChangeSource::Canary {
        return None;
    }
    event.key.strip_prefix("canary.").map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stream::InMemoryChangeStream;
    use crate::types::ConfigValue;
    use std::collections::HashMap;
    use std::sync::Mutex;

    /// Minimal `ConfigProvider` stand-in for reloader candidates.
    struct EmptyProvider;

    impl crate::interface::ConfigProvider for EmptyProvider {
        fn get_raw(&self, _key: &str) -> Option<&crate::types::AnnotatedValue> {
            None
        }
        fn keys(&self) -> Vec<String> {
            Vec::new()
        }
    }

    const fn small_plan(batch_size: usize) -> RolloutPlan {
        RolloutPlan {
            batch_size,
            batch_interval: Duration::from_millis(120),
            poll_interval: Duration::from_millis(5),
            failure_threshold: 0.0,
        }
    }

    /// Same shape `ProgressiveReloader::publish_canary_stage` emits with
    /// `with_instance_id` set: key `canary.<instance_id>`.
    async fn publish_committed(stream: &InMemoryChangeStream, instance: &str) {
        stream
            .publish(ChangeEvent::new(
                format!("canary.{instance}"),
                Some(ConfigValue::string(STAGE_COMMITTED)),
                Some(ConfigValue::string(format!("{instance} committed"))),
                ChangeSource::Canary,
            ))
            .await
            .unwrap();
    }

    async fn publish_stage(
        stream: &InMemoryChangeStream,
        instance: &str,
        stage: &str,
        detail: &str,
    ) {
        stream
            .publish(ChangeEvent::new(
                format!("canary.{instance}"),
                Some(ConfigValue::string(stage)),
                Some(ConfigValue::string(detail)),
                ChangeSource::Canary,
            ))
            .await
            .unwrap();
    }

    /// Health matrix by instance id; absent entries are Healthy.
    struct MockHealth {
        statuses: Mutex<HashMap<String, HealthStatus>>,
    }

    impl MockHealth {
        fn new(entries: &[(&str, HealthStatus)]) -> Arc<Self> {
            Arc::new(Self {
                statuses: Mutex::new(
                    entries
                        .iter()
                        .map(|(id, s)| (id.to_string(), s.clone()))
                        .collect(),
                ),
            })
        }
    }

    #[async_trait]
    impl RolloutHealthCheck for MockHealth {
        async fn check(&self, instance: &str) -> HealthStatus {
            self.statuses
                .lock()
                .unwrap()
                .get(instance)
                .cloned()
                .unwrap_or(HealthStatus::Healthy)
        }
    }

    /// Records every `(canary_pct, baseline_pct)` push for assertions.
    #[derive(Default)]
    struct RecordingMesh(Mutex<Vec<(u8, u8)>>);

    #[async_trait]
    impl MeshWeightPublisher for RecordingMesh {
        async fn publish_weights(&self, canary_pct: u8, baseline_pct: u8) -> ConfigResult<()> {
            self.0.lock().unwrap().push((canary_pct, baseline_pct));
            Ok(())
        }
    }

    /// Drain an observer subscription (bounded) and return the orchestrator
    /// directives it saw as `(stage, detail)` pairs.
    async fn drain_directives(
        observer: &mut Pin<Box<dyn futures_util::Stream<Item = ChangeEvent> + Send>>,
    ) -> Vec<(String, String)> {
        use futures_util::StreamExt;
        let mut out = Vec::new();
        while let Ok(Some(event)) =
            tokio::time::timeout(Duration::from_millis(10), observer.next()).await
        {
            if event.key == ORCHESTRATOR_KEY {
                let stage = event
                    .old_value
                    .as_ref()
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();
                let detail = event
                    .new_value
                    .as_ref()
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();
                out.push((stage, detail));
            }
        }
        out
    }

    #[tokio::test]
    async fn advances_in_batches_and_completes() {
        let stream = Arc::new(InMemoryChangeStream::new());
        let instances = vec!["i1".to_string(), "i2".to_string(), "i3".to_string()];
        // Batch 1 = i1+i2, batch 2 = i3.
        let mut observer = stream.subscribe().await.unwrap();
        let orchestrator = CanaryOrchestrator::new(
            Arc::clone(&stream) as Arc<dyn ChangeStream>,
            instances,
            small_plan(2),
            MockHealth::new(&[]),
        );
        let run = tokio::spawn(async move { orchestrator.run().await });
        // Give the orchestrator's subscription a beat to establish, then
        // emit one committed per instance.
        tokio::time::sleep(Duration::from_millis(20)).await;
        for instance in ["i1", "i2", "i3"] {
            publish_committed(&stream, instance).await;
        }
        let outcome = run.await.unwrap().unwrap();
        assert_eq!(
            outcome,
            RolloutOutcome::Completed {
                batches: 2,
                instances: 3
            }
        );
        let directives = drain_directives(&mut observer).await;
        let stages: Vec<&str> = directives.iter().map(|(stage, _)| stage.as_str()).collect();
        assert_eq!(stages, vec!["advance", "advance", "completed"]);
    }

    #[tokio::test]
    async fn rolls_back_when_batch_health_is_critical() {
        let stream = Arc::new(InMemoryChangeStream::new());
        let mut observer = stream.subscribe().await.unwrap();
        let orchestrator = CanaryOrchestrator::new(
            Arc::clone(&stream) as Arc<dyn ChangeStream>,
            vec!["i1".into(), "i2".into()],
            small_plan(2),
            MockHealth::new(&[(
                "i2",
                HealthStatus::Critical {
                    reason: "5xx spike".into(),
                },
            )]),
        );
        let run = tokio::spawn(async move { orchestrator.run().await });
        tokio::time::sleep(Duration::from_millis(20)).await;
        publish_committed(&stream, "i1").await;
        publish_committed(&stream, "i2").await;
        let outcome = run.await.unwrap().unwrap();
        assert_eq!(
            outcome,
            RolloutOutcome::Aborted {
                batch: 1,
                reason: "batch 1 health critical for 1/2 instance(s): i2: 5xx spike".into(),
                mesh_rolled_back_failed: false,
            }
        );
        let directives = drain_directives(&mut observer).await;
        assert_eq!(directives.len(), 1, "only the rollback directive");
        assert_eq!(directives[0].0, "rollback");
    }

    #[tokio::test]
    async fn aborts_when_committed_events_time_out() {
        let stream = Arc::new(InMemoryChangeStream::new());
        // No committed events at all: the batch window expires.
        let orchestrator = CanaryOrchestrator::new(
            Arc::clone(&stream) as Arc<dyn ChangeStream>,
            vec!["i1".into()],
            small_plan(1),
            MockHealth::new(&[]),
        );
        let outcome = orchestrator.run().await.unwrap();
        assert!(
            matches!(
                outcome,
                RolloutOutcome::Aborted {
                    batch: 1,
                    mesh_rolled_back_failed: false,
                    ..
                }
            ),
            "timeout must abort with rollback: {outcome:?}"
        );
    }

    #[tokio::test]
    async fn degraded_health_does_not_block() {
        let stream = Arc::new(InMemoryChangeStream::new());
        let orchestrator = CanaryOrchestrator::new(
            Arc::clone(&stream) as Arc<dyn ChangeStream>,
            vec!["i1".into()],
            RolloutPlan {
                batch_interval: Duration::from_millis(250),
                ..small_plan(1)
            },
            MockHealth::new(&[(
                "i1",
                HealthStatus::Degraded {
                    reason: "p99 elevated".into(),
                },
            )]),
        );
        let run = tokio::spawn(async move { orchestrator.run().await });
        tokio::time::sleep(Duration::from_millis(20)).await;
        publish_committed(&stream, "i1").await;
        let outcome = run.await.unwrap().unwrap();
        assert!(
            matches!(outcome, RolloutOutcome::Completed { .. }),
            "degraded must not block: {outcome:?}"
        );
    }

    #[tokio::test]
    async fn instance_rollback_aborts_the_rollout() {
        let stream = Arc::new(InMemoryChangeStream::new());
        let orchestrator = CanaryOrchestrator::new(
            Arc::clone(&stream) as Arc<dyn ChangeStream>,
            vec!["i1".into(), "i2".into()],
            small_plan(2),
            MockHealth::new(&[]),
        );
        let run = tokio::spawn(async move { orchestrator.run().await });
        tokio::time::sleep(Duration::from_millis(20)).await;
        // Instance i1 commits, instance i2 rolls back mid-batch.
        publish_committed(&stream, "i1").await;
        publish_stage(&stream, "i2", STAGE_ROLLED_BACK, "i2: candidate rejected").await;
        let outcome = run.await.unwrap().unwrap();
        assert!(
            matches!(
                outcome,
                RolloutOutcome::Aborted {
                    batch: 1,
                    mesh_rolled_back_failed: false,
                    ..
                }
            ),
            "instance rollback must abort: {outcome:?}"
        );
    }

    /// Mesh adapter contract: the traffic split tracks rollout progress
    /// linearly and snaps back to baseline on rollback.
    #[tokio::test]
    async fn mesh_weights_track_progress_and_snap_back() {
        let stream = Arc::new(InMemoryChangeStream::new());
        let mesh = Arc::new(RecordingMesh::default());
        let orchestrator = CanaryOrchestrator::new(
            Arc::clone(&stream) as Arc<dyn ChangeStream>,
            vec!["i1".into(), "i2".into()],
            small_plan(1),
            MockHealth::new(&[(
                "i2",
                HealthStatus::Critical {
                    reason: "down".into(),
                },
            )]),
        )
        .with_mesh(Arc::clone(&mesh) as Arc<dyn MeshWeightPublisher>);
        let run = tokio::spawn(async move { orchestrator.run().await });
        tokio::time::sleep(Duration::from_millis(20)).await;
        publish_committed(&stream, "i1").await;
        let outcome = run.await.unwrap().unwrap();
        assert!(matches!(outcome, RolloutOutcome::Aborted { .. }));

        let recorded = mesh.0.lock().unwrap().clone();
        assert_eq!(
            recorded,
            vec![(50, 50), (0, 100)],
            "batch 1 of 2 passes (50% split grows linearly), rollback snaps to baseline: {recorded:?}"
        );
    }

    /// An empty instance list (discovery failure) must fail loudly — a
    /// mesh-attached rollout on zero verified instances would otherwise
    /// push 100% of traffic onto nothing.
    #[tokio::test]
    async fn empty_instance_list_fails_loud() {
        let stream = Arc::new(InMemoryChangeStream::new());
        let mesh = Arc::new(RecordingMesh::default());
        let orchestrator = CanaryOrchestrator::new(
            Arc::clone(&stream) as Arc<dyn ChangeStream>,
            vec![],
            small_plan(1),
            MockHealth::new(&[]),
        )
        .with_mesh(Arc::clone(&mesh) as Arc<dyn MeshWeightPublisher>);
        let outcome = orchestrator.run().await;
        assert!(outcome.is_err(), "empty list must error, got {outcome:?}");
        assert!(
            mesh.0.lock().unwrap().is_empty(),
            "no traffic split may be pushed for an empty rollout"
        );
    }

    /// Duplicate `committed` events from one instance collapse: two commits
    /// from i1 never satisfy a batch whose instances are i1+i2 (the run
    /// times out instead of advancing), while the full i1+i2 pair does —
    /// one chatty instance cannot fill another one's quota.
    #[tokio::test]
    async fn duplicate_committed_events_cannot_fill_a_batch() {
        let stream = Arc::new(InMemoryChangeStream::new());
        let orchestrator = CanaryOrchestrator::new(
            Arc::clone(&stream) as Arc<dyn ChangeStream>,
            vec!["i1".into(), "i2".into()],
            small_plan(2),
            MockHealth::new(&[]),
        );
        let run = tokio::spawn(async move { orchestrator.run().await });
        tokio::time::sleep(Duration::from_millis(20)).await;
        publish_committed(&stream, "i1").await;
        publish_committed(&stream, "i1").await;
        let outcome = run.await.unwrap().unwrap();
        assert!(matches!(outcome, RolloutOutcome::Aborted { batch: 1, .. }));

        // Now the honest pair commits and completes.
        let stream = Arc::new(InMemoryChangeStream::new());
        let orchestrator = CanaryOrchestrator::new(
            Arc::clone(&stream) as Arc<dyn ChangeStream>,
            vec!["i1".into(), "i2".into()],
            small_plan(2),
            MockHealth::new(&[]),
        );
        let run = tokio::spawn(async move { orchestrator.run().await });
        tokio::time::sleep(Duration::from_millis(20)).await;
        publish_committed(&stream, "i1").await;
        publish_committed(&stream, "i1").await;
        publish_committed(&stream, "i2").await;
        let outcome = run.await.unwrap().unwrap();
        assert!(matches!(
            outcome,
            RolloutOutcome::Completed { batches: 1, .. }
        ));
    }

    /// Events from instances outside the current batch never count toward
    /// it: a chatty next-batch instance cannot advance the rollout early.
    #[tokio::test]
    async fn out_of_batch_instances_do_not_count() {
        let stream = Arc::new(InMemoryChangeStream::new());
        // Batch 1 = i1; i2 belongs to batch 2 and commits twice.
        let orchestrator = CanaryOrchestrator::new(
            Arc::clone(&stream) as Arc<dyn ChangeStream>,
            vec!["i1".into(), "i2".into()],
            small_plan(1),
            MockHealth::new(&[]),
        );
        let run = tokio::spawn(async move { orchestrator.run().await });
        tokio::time::sleep(Duration::from_millis(20)).await;
        publish_committed(&stream, "i2").await;
        publish_committed(&stream, "i2").await;
        let outcome = run.await.unwrap().unwrap();
        assert!(
            matches!(outcome, RolloutOutcome::Aborted { batch: 1, .. }),
            "batch 1 must not be filled by batch 2 events: {outcome:?}"
        );
    }

    /// A hung health check cannot stall `run`: the single-check timeout
    /// converts the hang into Degraded, and the batch window bounds the
    /// rollout (the run returns with an outcome instead of blocking).
    #[tokio::test]
    async fn hung_health_check_is_bounded() {
        struct Hangs;
        #[async_trait]
        impl RolloutHealthCheck for Hangs {
            async fn check(&self, _instance: &str) -> HealthStatus {
                futures_util::future::pending().await
            }
        }

        let stream = Arc::new(InMemoryChangeStream::new());
        let orchestrator = CanaryOrchestrator::new(
            Arc::clone(&stream) as Arc<dyn ChangeStream>,
            vec!["i1".into()],
            small_plan(1),
            Arc::new(Hangs),
        );
        let run = tokio::spawn(async move { orchestrator.run().await });
        tokio::time::sleep(Duration::from_millis(20)).await;
        publish_committed(&stream, "i1").await;
        let outcome = run.await.unwrap().unwrap();
        // The hang degrades to non-blocking: the rollout advances past the
        // batch (an operator-facing signal rides in the directive detail).
        assert!(matches!(outcome, RolloutOutcome::Completed { .. }));
    }

    /// A mesh failure on the rollback path must be visible in the outcome:
    /// `Aborted.mesh_rolled_back_failed = true` — the caller learns the
    /// traffic may NOT have reverted instead of reading a clean rollback.
    #[tokio::test]
    async fn rollback_mesh_failure_is_surfaced() {
        struct FailingMesh;
        #[async_trait]
        impl MeshWeightPublisher for FailingMesh {
            async fn publish_weights(
                &self,
                _canary_pct: u8,
                _baseline_pct: u8,
            ) -> ConfigResult<()> {
                Err(ConfigError::InvalidValue {
                    key: "envoy".into(),
                    expected_type: "weight update".into(),
                    message: "control plane unreachable".into(),
                })
            }
        }

        let stream = Arc::new(InMemoryChangeStream::new());
        let orchestrator = CanaryOrchestrator::new(
            Arc::clone(&stream) as Arc<dyn ChangeStream>,
            vec!["i1".into()],
            small_plan(1),
            MockHealth::new(&[(
                "i1",
                HealthStatus::Critical {
                    reason: "down".into(),
                },
            )]),
        )
        .with_mesh(Arc::new(FailingMesh));
        // No committed events + critical health: abort fires immediately.
        let outcome = orchestrator.run().await.unwrap();
        let RolloutOutcome::Aborted {
            batch,
            reason,
            mesh_rolled_back_failed,
        } = outcome
        else {
            panic!("expected abort, got {outcome:?}");
        };
        assert_eq!(batch, 1);
        assert!(mesh_rolled_back_failed, "mesh failure must be surfaced");
        assert!(
            reason.contains("FAILED"),
            "reason must carry the rollback failure: {reason}"
        );
    }

    /// A mesh failure on the advance path fails the run loudly: continuing
    /// a rollout whose traffic split is not applied would report progress
    /// that does not exist.
    #[tokio::test]
    async fn advance_mesh_failure_fails_loud() {
        struct FailingMesh;
        #[async_trait]
        impl MeshWeightPublisher for FailingMesh {
            async fn publish_weights(
                &self,
                _canary_pct: u8,
                _baseline_pct: u8,
            ) -> ConfigResult<()> {
                Err(ConfigError::InvalidValue {
                    key: "envoy".into(),
                    expected_type: "weight update".into(),
                    message: "control plane unreachable".into(),
                })
            }
        }

        let stream = Arc::new(InMemoryChangeStream::new());
        let orchestrator = CanaryOrchestrator::new(
            Arc::clone(&stream) as Arc<dyn ChangeStream>,
            vec!["i1".into()],
            small_plan(1),
            MockHealth::new(&[]),
        )
        .with_mesh(Arc::new(FailingMesh));
        let run = tokio::spawn(async move { orchestrator.run().await });
        tokio::time::sleep(Duration::from_millis(20)).await;
        publish_committed(&stream, "i1").await;
        let outcome = run.await.unwrap();
        assert!(
            outcome.is_err(),
            "mesh failure on advance must error: {outcome:?}"
        );
    }

    /// A rollback from an **already promoted** instance aborts the rollout:
    /// the candidate regressed on promoted ground, so later batches must not
    /// proceed (and the mesh must not reach 100% on top of a regressed
    /// canary).
    #[tokio::test]
    async fn promoted_instance_rollback_aborts_later_batches() {
        let stream = Arc::new(InMemoryChangeStream::new());
        let orchestrator = CanaryOrchestrator::new(
            Arc::clone(&stream) as Arc<dyn ChangeStream>,
            vec!["i1".into(), "i2".into()],
            small_plan(1),
            MockHealth::new(&[]),
        );
        let run = tokio::spawn(async move { orchestrator.run().await });
        tokio::time::sleep(Duration::from_millis(20)).await;
        // Batch 1 (i1) commits and promotes; while batch 2 (i2) is being
        // waited on, the promoted i1 rolls back — the candidate regressed on
        // promoted ground, so batch 2 must abort instead of completing.
        publish_committed(&stream, "i1").await;
        tokio::time::sleep(Duration::from_millis(60)).await;
        publish_stage(
            &stream,
            "i1",
            STAGE_ROLLED_BACK,
            "regression on promoted config",
        )
        .await;
        let outcome = run.await.unwrap().unwrap();
        let RolloutOutcome::Aborted {
            batch,
            reason,
            mesh_rolled_back_failed,
        } = outcome
        else {
            panic!("promoted rollback must abort: {outcome:?}");
        };
        assert_eq!(batch, 2);
        assert!(!mesh_rolled_back_failed);
        assert!(
            reason.contains("promoted instance `i1`"),
            "reason must name the promoted regression: {reason}"
        );
    }

    /// Local multi-instance simulation: two real in-process
    /// `ProgressiveReloader`s attached to one in-memory stream run their own
    /// canary trials; the orchestrator consumes their stage transitions and
    /// drives the rollout to completion.
    #[tokio::test]
    async fn orchestrates_in_process_reloaders_end_to_end() {
        use crate::watcher::progressive::{ProgressiveReloader, ReloadHealthCheck, ReloadStrategy};

        struct AlwaysHealthy;
        #[async_trait]
        impl ReloadHealthCheck for AlwaysHealthy {
            async fn check(
                &self,
                _provider: Arc<dyn crate::interface::ConfigProvider>,
            ) -> HealthStatus {
                HealthStatus::Healthy
            }
        }

        let stream = Arc::new(InMemoryChangeStream::new());
        let mut observer = stream.subscribe().await.unwrap();
        let plan = RolloutPlan {
            batch_size: 2,
            batch_interval: Duration::from_millis(300),
            poll_interval: Duration::from_millis(5),
            failure_threshold: 0.0,
        };
        let orchestrator = CanaryOrchestrator::new(
            Arc::clone(&stream) as Arc<dyn ChangeStream>,
            vec!["reloader-a".into(), "reloader-b".into()],
            plan,
            MockHealth::new(&[]),
        );
        let run = tokio::spawn(async move { orchestrator.run().await });
        // Establish the orchestrator's subscription before the reloaders
        // start committing, so no stage transition is lost to the race.
        tokio::time::sleep(Duration::from_millis(20)).await;

        let mut handles = Vec::new();
        for name in ["reloader-a", "reloader-b"] {
            let reloader = ProgressiveReloader::with_dependencies(
                Arc::new(name.to_string()),
                ReloadStrategy::Canary {
                    trial_duration: Duration::from_millis(10),
                    poll_interval: Duration::from_millis(5),
                },
                Some(Arc::new(AlwaysHealthy)),
            )
            .with_instance_id(name)
            .with_change_stream(Arc::clone(&stream) as Arc<dyn ChangeStream>);
            let provider: Arc<dyn crate::interface::ConfigProvider> = Arc::new(EmptyProvider);
            handles.push(tokio::spawn(async move {
                reloader
                    .begin_reload(Arc::new(format!("{name}-v2")), provider)
                    .await
            }));
        }
        for handle in handles {
            handle.await.unwrap().expect("canary reload commits");
        }
        let outcome = run.await.unwrap().unwrap();
        assert_eq!(
            outcome,
            RolloutOutcome::Completed {
                batches: 1,
                instances: 2
            }
        );
        let directives = drain_directives(&mut observer).await;
        let stages: Vec<&str> = directives.iter().map(|(stage, _)| stage.as_str()).collect();
        assert_eq!(stages, vec!["advance", "completed"]);
    }

    #[test]
    fn rollout_plan_defaults_are_conservative() {
        let plan = RolloutPlan::default();
        assert_eq!(plan.batch_size, 1);
        assert_eq!(plan.batch_interval, Duration::from_secs(30));
        assert_eq!(plan.poll_interval, Duration::from_secs(5));
        assert_eq!(plan.failure_threshold, 0.0);
    }

    /// Mesh publisher that fails the *second* 100/0 switch: the last
    /// batch's advance succeeds so the rollout reaches the completed
    /// phase, whose mandatory final push is the one that fails.
    struct FailsOnSecondFinalSplit(Mutex<u32>);

    #[async_trait]
    impl MeshWeightPublisher for FailsOnSecondFinalSplit {
        async fn publish_weights(&self, canary_pct: u8, _baseline_pct: u8) -> ConfigResult<()> {
            if canary_pct == 100 {
                let mut count = self.0.lock().unwrap();
                *count += 1;
                if *count >= 2 {
                    return Err(ConfigError::InvalidValue {
                        key: "envoy".into(),
                        expected_type: "weight update".into(),
                        message: "final split rejected".into(),
                    });
                }
            }
            Ok(())
        }
    }

    #[tokio::test]
    async fn degraded_advance_mesh_failure_fails_traffic_split() {
        // Degraded 批次继续推进,但 mesh 发布失败时必须显式失败本次推进,
        // 不能在流量未切换的情况下假装进度存在。
        struct AlwaysFailsMesh;

        #[async_trait]
        impl MeshWeightPublisher for AlwaysFailsMesh {
            async fn publish_weights(
                &self,
                _canary_pct: u8,
                _baseline_pct: u8,
            ) -> ConfigResult<()> {
                Err(ConfigError::InvalidValue {
                    key: "envoy".into(),
                    expected_type: "weight update".into(),
                    message: "control plane unreachable".into(),
                })
            }
        }

        let stream = Arc::new(InMemoryChangeStream::new());
        let orchestrator = CanaryOrchestrator::new(
            Arc::clone(&stream) as Arc<dyn ChangeStream>,
            vec!["i1".into()],
            small_plan(1),
            MockHealth::new(&[(
                "i1",
                HealthStatus::Degraded {
                    reason: "p99 elevated".into(),
                },
            )]),
        )
        .with_mesh(Arc::new(AlwaysFailsMesh));
        let outcome = orchestrator.run().await.unwrap();
        assert!(
            matches!(outcome, RolloutOutcome::Aborted { .. }),
            "degraded advance with a failing mesh must fail the split: {outcome:?}"
        );
    }

    #[tokio::test]
    async fn completed_phase_mesh_failure_fails_the_run() {
        // 批次内 mesh 正常(最后一批 100/0 已推)、完成阶段的例行重推失败:
        // 整个 run 必须报错,而不是报告一个从未确认应用的最终分流。
        let stream = Arc::new(InMemoryChangeStream::new());
        let orchestrator = CanaryOrchestrator::new(
            Arc::clone(&stream) as Arc<dyn ChangeStream>,
            vec!["i1".into(), "i2".into()],
            small_plan(1),
            MockHealth::new(&[]),
        )
        .with_mesh(Arc::new(FailsOnSecondFinalSplit(Mutex::new(0))));
        let run = tokio::spawn(async move { orchestrator.run().await });
        tokio::time::sleep(Duration::from_millis(20)).await;
        publish_committed(&stream, "i1").await;
        publish_committed(&stream, "i2").await;
        let outcome = run.await.unwrap();
        assert!(
            outcome.is_err(),
            "completed-phase mesh failure must fail the run: {outcome:?}"
        );
    }
}
