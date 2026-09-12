// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Progressive Reload - Staged configuration deployment with health checks.

use std::sync::Arc;
use std::time::{Duration, Instant};

use arc_swap::ArcSwap;
use async_trait::async_trait;

use crate::error::{ConfigError, ConfigResult};
use crate::interface::ConfigProvider;

/// Reload strategy for hot reload.
///
/// Degenerate parameters commit immediately by construction: a `Canary`
/// with `trial_duration == Duration::ZERO` skips the trial loop and a
/// `Linear` with `steps == 0` skips the ramp — both behave like
/// [`ReloadStrategy::Immediate`]. Constructing them that way is not an
/// error; pass a positive trial/steps when health-checked staging matters.
#[derive(Debug, Clone, Default)]
pub enum ReloadStrategy {
    #[default]
    Immediate,
    Canary {
        trial_duration: Duration,
        poll_interval: Duration,
    },
    Linear {
        steps: u8,
        interval: Duration,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReloadOutcome {
    Committed,
    RolledBack { reason: String },
}

/// Health check result
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HealthStatus {
    Healthy,
    Degraded { reason: String },
    Critical { reason: String },
}

impl HealthStatus {
    /// Check if the status is healthy.
    pub fn is_healthy(&self) -> bool {
        matches!(self, HealthStatus::Healthy)
    }

    /// Check if the status requires rollback.
    pub fn requires_rollback(&self) -> bool {
        matches!(self, HealthStatus::Critical { .. })
    }
}

/// Reload health check trait
#[async_trait]
pub trait ReloadHealthCheck: Send + Sync {
    async fn check(&self, provider: Arc<dyn ConfigProvider>) -> HealthStatus;
}

struct ProgressiveReloaderInner<T: Clone + Send + Sync + 'static> {
    current: ArcSwap<T>,
    /// Configuration under trial during a canary/linear reload.
    ///
    /// A reload cancelled mid-flight (its future dropped) may leave the
    /// candidate set. That residue is unobservable through the public API
    /// and harmless: the next completed reload overwrites it, and a
    /// committed or rolled-back reload always clears it.
    candidate: ArcSwap<Option<Arc<T>>>,
    strategy: ReloadStrategy,
    /// Health check used by canary/linear reloads.
    ///
    /// Stored in a shared, atomically swappable slot so
    /// [`ProgressiveReloader::with_health_check`] can replace it in place
    /// for all clones of the reloader instead of forking the state.
    health_check: ArcSwap<Option<Arc<dyn ReloadHealthCheck>>>,
    /// Serializes concurrent `begin_reload` calls to prevent state corruption.
    ///
    /// The guard is intentionally held across the whole canary/linear
    /// rollout, including its sleeps and health-check polling: a staged
    /// deployment must never interleave with another reload.
    reload_lock: tokio::sync::Mutex<()>,
    /// Unified change stream the canary stage transitions are published to
    /// (upstream orchestration), when attached.
    #[cfg(feature = "change-stream")]
    change_stream: ArcSwap<Option<Arc<dyn crate::stream::ChangeStream>>>,
}

pub struct ProgressiveReloader<T: Clone + Send + Sync + 'static> {
    inner: Arc<ProgressiveReloaderInner<T>>,
}

impl<T: Clone + Send + Sync + 'static> Clone for ProgressiveReloader<T> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<T: Clone + Send + Sync + 'static> ProgressiveReloader<T> {
    pub fn new(initial: Arc<T>, strategy: ReloadStrategy) -> Self {
        Self {
            inner: Arc::new(ProgressiveReloaderInner {
                current: ArcSwap::new(initial),
                candidate: ArcSwap::new(Arc::new(None)),
                strategy,
                health_check: ArcSwap::new(Arc::new(None)),
                reload_lock: tokio::sync::Mutex::new(()),
                #[cfg(feature = "change-stream")]
                change_stream: ArcSwap::new(Arc::new(None)),
            }),
        }
    }

    pub fn with_dependencies(
        initial: Arc<T>,
        strategy: ReloadStrategy,
        health_check: Option<Arc<dyn ReloadHealthCheck>>,
    ) -> Self {
        Self {
            inner: Arc::new(ProgressiveReloaderInner {
                current: ArcSwap::new(initial),
                candidate: ArcSwap::new(Arc::new(None)),
                strategy,
                health_check: ArcSwap::new(Arc::new(health_check)),
                reload_lock: tokio::sync::Mutex::new(()),
                #[cfg(feature = "change-stream")]
                change_stream: ArcSwap::new(Arc::new(None)),
            }),
        }
    }

    pub fn builder() -> ProgressiveReloaderBuilder<T> {
        ProgressiveReloaderBuilder::new()
    }

    #[inline]
    pub fn current(&self) -> Arc<T> {
        self.inner.current.load_full()
    }

    /// Attach (or replace) the health check used by canary/linear reloads.
    ///
    /// The health check lives in an atomically swappable slot shared by all
    /// clones of the reloader, so installing it through any handle (including
    /// clones taken before this call) affects every clone. The `current` and
    /// `candidate` snapshots and the reload lock are equally shared and stay
    /// intact — this method never forks the reloader state.
    pub fn with_health_check(self, health_check: Arc<dyn ReloadHealthCheck>) -> Self {
        // Swap the slot in place instead of rebuilding `inner`: rebuilding
        // would fork `current`/`candidate` snapshots and install a fresh
        // reload lock, breaking serialization between clones.
        self.inner.health_check.store(Arc::new(Some(health_check)));
        self
    }

    /// Attach the unified change stream as the canary event sink.
    ///
    /// Stage transitions (`trial_started` / `committed` / `rolled_back`) are
    /// published as `ChangeSource::Canary` events so an orchestrator can
    /// observe (and react to) a staged rollout across instances. Like
    /// [`Self::with_health_check`], the slot is shared by every clone of the
    /// reloader. Only available with the `change-stream` feature.
    #[cfg(feature = "change-stream")]
    pub fn with_change_stream(self, stream: Arc<dyn crate::stream::ChangeStream>) -> Self {
        self.inner.change_stream.store(Arc::new(Some(stream)));
        self
    }

    /// Publish a canary stage transition to the attached change stream
    /// (no-op without one). Publish failures never affect the reload.
    #[cfg(feature = "change-stream")]
    async fn publish_canary_stage(&self, stage: &'static str, detail: &str) {
        use crate::stream::{ChangeEvent, ChangeSource};
        let loaded = self.inner.change_stream.load_full();
        if let Some(stream) = loaded.as_ref() {
            let _ = stream
                .publish(ChangeEvent::new(
                    "canary",
                    Some(crate::types::ConfigValue::string(stage)),
                    Some(crate::types::ConfigValue::string(detail)),
                    ChangeSource::Canary,
                ))
                .await;
        }
    }

    /// No-op twin of the change-stream publisher for builds without the
    /// `change-stream` feature (reload paths call it unconditionally).
    #[cfg(not(feature = "change-stream"))]
    #[inline]
    async fn publish_canary_stage(&self, _stage: &'static str, _detail: &str) {}



    /// Begin a staged reload of the configuration.
    ///
    /// The reload lock is held for the entire duration of the call: for
    /// [`ReloadStrategy::Canary`] and [`ReloadStrategy::Linear`] that
    /// includes the whole trial/rollout window with its health-check
    /// polling. This is intentional — a staged rollout must not be
    /// interleaved with another reload — but callers should be aware that a
    /// long trial delays concurrent `begin_reload` calls (including those
    /// started through clones of this handle).
    ///
    /// If the future is cancelled (dropped) mid-reload, any canary/linear
    /// candidate left in the candidate slot stays there; it is not
    /// observable through the public API and the next completed reload
    /// clears it.
    pub async fn begin_reload(
        &self,
        new_config: Arc<T>,
        provider: Arc<dyn ConfigProvider>,
    ) -> ConfigResult<ReloadOutcome> {
        // Serialize concurrent reload operations to prevent state corruption.
        let _guard = self.inner.reload_lock.lock().await;
        match &self.inner.strategy {
            ReloadStrategy::Immediate => {
                self.inner.current.store(new_config);
                self.publish_canary_stage("committed", "immediate").await;
                Ok(ReloadOutcome::Committed)
            }
            ReloadStrategy::Canary {
                trial_duration,
                poll_interval,
            } => {
                self.canary_reload(new_config, *trial_duration, *poll_interval, provider)
                    .await
            }
            ReloadStrategy::Linear { steps, interval } => {
                self.linear_reload(new_config, *steps, *interval, provider)
                    .await
            }
        }
    }

    async fn canary_reload(
        &self,
        new_config: Arc<T>,
        trial_duration: Duration,
        poll_interval: Duration,
        provider: Arc<dyn ConfigProvider>,
    ) -> ConfigResult<ReloadOutcome> {
        self.inner
            .candidate
            .store(Arc::new(Some(new_config.clone())));
        self.publish_canary_stage("trial_started", "canary").await;
        let deadline = Instant::now() + trial_duration;

        while Instant::now() < deadline {
            tokio::time::sleep(poll_interval).await;
            let health_check = self.inner.health_check.load_full();
            if let Some(hc) = health_check.as_ref() {
                match hc.check(provider.clone()).await {
                    HealthStatus::Critical { reason } => {
                        self.inner.candidate.store(Arc::new(None));
                        self.publish_canary_stage("rolled_back", &reason).await;
                        return Err(ConfigError::ReloadRolledBack { reason });
                    }
                    HealthStatus::Degraded { reason } => {
                        // Canary degraded but not critical - continue monitoring
                        let _ = reason;
                    }
                    HealthStatus::Healthy => {}
                }
            }
        }

        self.inner.current.store(new_config);
        self.inner.candidate.store(Arc::new(None));
        self.publish_canary_stage("committed", "canary").await;
        Ok(ReloadOutcome::Committed)
    }

    async fn linear_reload(
        &self,
        new_config: Arc<T>,
        steps: u8,
        interval: Duration,
        provider: Arc<dyn ConfigProvider>,
    ) -> ConfigResult<ReloadOutcome> {
        self.inner
            .candidate
            .store(Arc::new(Some(new_config.clone())));

        for step in 0..steps {
            tokio::time::sleep(interval).await;
            let health_check = self.inner.health_check.load_full();
            if let Some(hc) = health_check.as_ref() {
                match hc.check(provider.clone()).await {
                    HealthStatus::Critical { reason } => {
                        self.inner.candidate.store(Arc::new(None));
                        let detail = format!("linear step {}: {}", step + 1, reason);
                        self.publish_canary_stage("rolled_back", &detail).await;
                        return Err(ConfigError::ReloadRolledBack {
                            reason: format!("Linear step {} failed: {}", step + 1, reason),
                        });
                    }
                    HealthStatus::Degraded { reason } => {
                        // Linear step degraded but not critical - continue to next step
                        let _ = reason;
                    }
                    HealthStatus::Healthy => {}
                }
            }
        }

        self.inner.current.store(new_config);
        self.inner.candidate.store(Arc::new(None));
        self.publish_canary_stage("committed", "linear").await;
        Ok(ReloadOutcome::Committed)
    }
}

pub struct ProgressiveReloaderBuilder<T: Clone + Send + Sync + 'static> {
    initial: Option<Arc<T>>,
    strategy: Option<ReloadStrategy>,
    health_check: Option<Arc<dyn ReloadHealthCheck>>,
}

impl<T: Clone + Send + Sync + 'static> ProgressiveReloaderBuilder<T> {
    pub fn new() -> Self {
        Self {
            initial: None,
            strategy: Some(ReloadStrategy::Immediate),
            health_check: None,
        }
    }

    pub fn initial(mut self, initial: Arc<T>) -> Self {
        self.initial = Some(initial);
        self
    }

    pub fn strategy(mut self, strategy: ReloadStrategy) -> Self {
        self.strategy = Some(strategy);
        self
    }

    pub fn health_check(mut self, health_check: Arc<dyn ReloadHealthCheck>) -> Self {
        self.health_check = Some(health_check);
        self
    }

    pub fn build(self) -> ProgressiveReloader<T> {
        let initial = self.initial.expect("initial configuration is required");
        let strategy = self.strategy.unwrap_or_default();
        ProgressiveReloader::with_dependencies(initial, strategy, self.health_check)
    }
}

impl<T: Clone + Send + Sync + 'static> Default for ProgressiveReloaderBuilder<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interface::ConfigProvider;
    use crate::types::AnnotatedValue;

    #[derive(Debug, Clone)]
    struct MockProvider;

    impl ConfigProvider for MockProvider {
        fn get_raw(&self, _key: &str) -> Option<&AnnotatedValue> {
            None
        }

        fn keys(&self) -> Vec<String> {
            vec![]
        }
    }

    #[test]
    fn test_current_returns_initial() {
        let reloader = ProgressiveReloader::new(Arc::new(42i32), ReloadStrategy::Immediate);
        assert_eq!(*reloader.current(), 42);
    }

    #[test]
    fn test_builder_default_strategy() {
        let reloader = ProgressiveReloader::builder()
            .initial(Arc::new(1i32))
            .build();
        assert_eq!(*reloader.current(), 1);
    }

    #[test]
    fn test_clone_preserves_shared_state() {
        let reloader = ProgressiveReloader::new(Arc::new(42i32), ReloadStrategy::Immediate);
        let cloned = reloader.clone();

        // Both should share the same state
        assert_eq!(*cloned.current(), 42);
    }

    #[tokio::test]
    async fn test_immediate_reload() {
        let reloader = ProgressiveReloader::new(Arc::new(1i32), ReloadStrategy::Immediate);
        let result = reloader
            .begin_reload(Arc::new(2i32), Arc::new(MockProvider))
            .await
            .unwrap();
        assert!(matches!(result, ReloadOutcome::Committed));
        assert_eq!(*reloader.current(), 2);
    }

    #[tokio::test]
    async fn test_canary_reload_healthy() {
        struct HealthyCheck;
        #[async_trait]
        impl ReloadHealthCheck for HealthyCheck {
            async fn check(&self, _provider: Arc<dyn ConfigProvider>) -> HealthStatus {
                HealthStatus::Healthy
            }
        }

        let reloader = ProgressiveReloader::new(
            Arc::new(1i32),
            ReloadStrategy::Canary {
                trial_duration: Duration::from_millis(50),
                poll_interval: Duration::from_millis(10),
            },
        )
        .with_health_check(Arc::new(HealthyCheck));

        let result = reloader
            .begin_reload(Arc::new(2i32), Arc::new(MockProvider))
            .await
            .unwrap();
        assert!(matches!(result, ReloadOutcome::Committed));
        assert_eq!(*reloader.current(), 2);
    }

    #[tokio::test]
    async fn test_canary_reload_critical_rollback() {
        struct CriticalCheck;
        #[async_trait]
        impl ReloadHealthCheck for CriticalCheck {
            async fn check(&self, _provider: Arc<dyn ConfigProvider>) -> HealthStatus {
                HealthStatus::Critical {
                    reason: "service unhealthy".to_string(),
                }
            }
        }

        let reloader = ProgressiveReloader::new(
            Arc::new(1i32),
            ReloadStrategy::Canary {
                trial_duration: Duration::from_millis(100),
                poll_interval: Duration::from_millis(10),
            },
        )
        .with_health_check(Arc::new(CriticalCheck));

        let result = reloader
            .begin_reload(Arc::new(2i32), Arc::new(MockProvider))
            .await;
        assert!(matches!(result, Err(ConfigError::ReloadRolledBack { .. })));
        assert_eq!(*reloader.current(), 1);
    }

    #[tokio::test]
    async fn test_linear_reload() {
        let reloader = ProgressiveReloader::new(
            Arc::new(1i32),
            ReloadStrategy::Linear {
                steps: 3,
                interval: Duration::from_millis(10),
            },
        );

        let result = reloader
            .begin_reload(Arc::new(2i32), Arc::new(MockProvider))
            .await
            .unwrap();
        assert!(matches!(result, ReloadOutcome::Committed));
        assert_eq!(*reloader.current(), 2);
    }

    /// Regression test for issue #478: `with_health_check` used to rebuild
    /// the inner state, so a clone taken before the call forked from the
    /// original (separate `current`/`candidate` snapshots and reload lock).
    /// The health check must land in a shared slot: installing it through
    /// the clone has to roll back a reload started through the original
    /// handle.
    #[tokio::test]
    async fn test_with_health_check_after_clone_shares_state() {
        struct CriticalCheck;
        #[async_trait]
        impl ReloadHealthCheck for CriticalCheck {
            async fn check(&self, _provider: Arc<dyn ConfigProvider>) -> HealthStatus {
                HealthStatus::Critical {
                    reason: "clone must share the health check".to_string(),
                }
            }
        }

        let reloader = ProgressiveReloader::new(
            Arc::new(1i32),
            ReloadStrategy::Canary {
                trial_duration: Duration::from_millis(100),
                poll_interval: Duration::from_millis(10),
            },
        );
        let cloned = reloader.clone().with_health_check(Arc::new(CriticalCheck));

        // The check installed through `cloned` must be visible to `reloader`
        // and roll its reload back — proving the state is shared, not forked.
        let result = reloader
            .begin_reload(Arc::new(2i32), Arc::new(MockProvider))
            .await;
        assert!(matches!(result, Err(ConfigError::ReloadRolledBack { .. })));
        assert_eq!(*reloader.current(), 1);
        assert_eq!(*cloned.current(), 1);
    }

    /// Regression test for issue #478 (reload serialization): reloads
    /// started through different clones must be serialized by the single
    /// shared reload lock, so health checks from distinct reloads never
    /// overlap.
    #[tokio::test]
    async fn test_clones_share_reload_lock() {
        use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};

        // Records the maximum number of health checks observed in flight.
        struct OverlapCheck {
            in_flight: Arc<AtomicUsize>,
            max_in_flight: Arc<AtomicUsize>,
        }
        #[async_trait]
        impl ReloadHealthCheck for OverlapCheck {
            async fn check(&self, _provider: Arc<dyn ConfigProvider>) -> HealthStatus {
                let now = self.in_flight.fetch_add(1, AtomicOrdering::SeqCst) + 1;
                self.max_in_flight.fetch_max(now, AtomicOrdering::SeqCst);
                tokio::time::sleep(Duration::from_millis(10)).await;
                self.in_flight.fetch_sub(1, AtomicOrdering::SeqCst);
                HealthStatus::Healthy
            }
        }

        let in_flight = Arc::new(AtomicUsize::new(0));
        let max_in_flight = Arc::new(AtomicUsize::new(0));
        let reloader = ProgressiveReloader::new(
            Arc::new(1i32),
            ReloadStrategy::Canary {
                trial_duration: Duration::from_millis(60),
                poll_interval: Duration::from_millis(5),
            },
        )
        .with_health_check(Arc::new(OverlapCheck {
            in_flight: Arc::clone(&in_flight),
            max_in_flight: Arc::clone(&max_in_flight),
        }));

        async fn reload_through(
            reloader: ProgressiveReloader<i32>,
            value: i32,
        ) -> ConfigResult<ReloadOutcome> {
            reloader
                .begin_reload(Arc::new(value), Arc::new(MockProvider))
                .await
        }

        let a = reloader.clone();
        let b = reloader.clone();
        let h1 = tokio::spawn(reload_through(a, 2));
        let h2 = tokio::spawn(reload_through(b, 3));

        let (r1, r2) = tokio::join!(h1, h2);
        assert!(matches!(r1.unwrap().unwrap(), ReloadOutcome::Committed));
        assert!(matches!(r2.unwrap().unwrap(), ReloadOutcome::Committed));

        // One reload holds the shared lock across its whole canary trial,
        // so the second reload's health checks cannot overlap the first's.
        assert_eq!(
            max_in_flight.load(AtomicOrdering::SeqCst),
            1,
            "the shared reload lock must serialize reloads across clones"
        );
    }


    #[cfg(all(feature = "change-stream", feature = "progressive-reload"))]
    mod canary_events {
        use super::*;
        use crate::stream::{ChangeEvent, ChangeSource, InMemoryChangeStream};
        use std::pin::Pin;
        use std::sync::Mutex;
        use std::time::Duration;

        /// Capturing subscriber: records published events via the port.
        struct CollectingStream {
            inner: InMemoryChangeStream,
            events: Mutex<Vec<ChangeEvent>>,
        }

        #[async_trait]
        impl crate::stream::ChangeStream for CollectingStream {
            async fn publish(&self, event: ChangeEvent) -> ConfigResult<()> {
                self.events.lock().unwrap().push(event.clone());
                self.inner.publish(event).await
            }

            async fn subscribe(
                &self,
            ) -> ConfigResult<
                Pin<Box<dyn futures_util::Stream<Item = ChangeEvent> + Send>>,
            > {
                self.inner.subscribe().await
            }

            async fn ack(&self, version: u64) -> ConfigResult<()> {
                self.inner.ack(version).await
            }
        }

        struct HealthyCheck;

        #[async_trait]
        impl ReloadHealthCheck for HealthyCheck {
            async fn check(&self, _: Arc<dyn ConfigProvider>) -> HealthStatus {
                HealthStatus::Healthy
            }
        }

        struct CriticalCheck;

        #[async_trait]
        impl ReloadHealthCheck for CriticalCheck {
            async fn check(&self, _: Arc<dyn ConfigProvider>) -> HealthStatus {
                HealthStatus::Critical {
                    reason: "probe failed".to_string(),
                }
            }
        }

        fn provider() -> Arc<dyn ConfigProvider> {
            Arc::new(MockProvider)
        }

        #[tokio::test]
        async fn canary_stages_are_published_to_the_change_stream() {
            let sink = Arc::new(CollectingStream {
                inner: InMemoryChangeStream::new(),
                events: Mutex::new(Vec::new()),
            });
            let reloader = ProgressiveReloader::new(
                Arc::new(1u32),
                ReloadStrategy::Canary {
                    trial_duration: Duration::from_millis(20),
                    poll_interval: Duration::from_millis(5),
                },
            )
            .with_health_check(Arc::new(HealthyCheck))
            .with_change_stream(sink.clone());

            let outcome = reloader
                .begin_reload(Arc::new(2u32), provider())
                .await
                .expect("reload");
            assert!(matches!(outcome, ReloadOutcome::Committed));

            let stages: Vec<(String, String)> = sink
                .events
                .lock()
                .unwrap()
                .iter()
                .map(|e| {
                    (
                        e.old_value.as_ref().and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        e.new_value.as_ref().and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    )
                })
                .collect();

            assert_eq!(
                stages,
                vec![
                    ("trial_started".to_string(), "canary".to_string()),
                    ("committed".to_string(), "canary".to_string()),
                ],
                "canary stage transitions reach the change stream"
            );
            // All events carry the canary source marker.
            assert!(
                sink.events
                    .lock()
                    .unwrap()
                    .iter()
                    .all(|e| e.source == ChangeSource::Canary)
            );
        }

        #[tokio::test]
        async fn rollback_is_published_to_the_change_stream() {
            let sink = Arc::new(CollectingStream {
                inner: InMemoryChangeStream::new(),
                events: Mutex::new(Vec::new()),
            });
            let reloader = ProgressiveReloader::new(
                Arc::new(1u32),
                ReloadStrategy::Canary {
                    trial_duration: Duration::from_millis(50),
                    poll_interval: Duration::from_millis(5),
                },
            )
            .with_health_check(Arc::new(CriticalCheck))
            .with_change_stream(sink.clone());

            let outcome = reloader
                .begin_reload(Arc::new(2u32), provider())
                .await
                .expect_err("critical health must roll back");

            assert!(matches!(outcome, ConfigError::ReloadRolledBack { .. }));
            let stages: Vec<String> = sink
                .events
                .lock()
                .unwrap()
                .iter()
                .filter_map(|e| e.old_value.as_ref().and_then(|v| v.as_str()))
                .map(|s| s.to_string())
                .collect();
            assert_eq!(
                stages.last().map(String::as_str),
                Some("rolled_back"),
                "rollback transition is published"
            );
            // The current config is untouched after the rollback.
            assert_eq!(*reloader.current(), 1);
        }

        #[tokio::test]
        async fn no_stream_attached_is_a_noop() {
            let reloader = ProgressiveReloader::new(
                Arc::new(1u32),
                ReloadStrategy::Canary {
                    trial_duration: Duration::from_millis(10),
                    poll_interval: Duration::from_millis(5),
                },
            )
            .with_health_check(Arc::new(HealthyCheck));

            let outcome = reloader
                .begin_reload(Arc::new(2u32), provider())
                .await
                .expect("reload without stream still works");
            assert!(matches!(outcome, ReloadOutcome::Committed));
        }
    }
}
