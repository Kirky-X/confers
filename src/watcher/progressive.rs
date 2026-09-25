// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! Progressive Reload - Staged configuration deployment with health checks.

use std::sync::Arc;
use std::time::{Duration, Instant};

use arc_swap::ArcSwap;
use async_trait::async_trait;

use crate::error::{ConfigError, ConfigResult};
use crate::i18n::tr_args;
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

/// Pre-commit validation for reload candidates (R-watch-009).
///
/// Runs once per reload, immediately before the candidate would be
/// committed. A failed validation rolls the reload back — keeping the
/// current configuration — when the reloader's
/// `rollback_on_validation_failure` flag is set; otherwise the failure is
/// logged and the commit proceeds (legacy behavior; the flag defaults to
/// false).
#[async_trait]
pub trait ReloadValidator<T: Clone + Send + Sync + 'static>: Send + Sync {
    /// Validate `candidate`. `Err(reason)` blocks (or is recorded, depending
    /// on the rollback flag) the commit.
    async fn validate(&self, candidate: &T) -> Result<(), String>;
}

/// Hard pre-commit gate for reload candidates.
///
/// Distinct from [`ReloadValidator`] (value-level validation whose failure
/// can be downgraded to a logged warning via
/// [`rollback_on_validation_failure`](ProgressiveReloader::with_rollback_on_validation_failure))
/// and from [`ReloadHealthCheck`] (which polls the candidate during the
/// canary/linear *trial window*, before any commit decision): a
/// `PreCommitCheck` runs immediately before the swap-to-current and its
/// `Err` is final — the commit never happens and `begin_reload` reports
/// `ConfigError::ReloadRejected`. Use it for release gates that must not be
/// negotiable (downstream dry-run, invariant checks).
#[async_trait]
pub trait PreCommitCheck<T: Clone + Send + Sync + 'static>: Send + Sync {
    /// Gate `candidate`. `Err(reason)` rejects the reload outright.
    async fn check(&self, candidate: &T) -> Result<(), String>;
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
    /// Pre-commit validator (R-watch-009).
    ///
    /// Shared swappable slot, same pattern as `health_check`.
    validator: ArcSwap<Option<Arc<dyn ReloadValidator<T>>>>,
    /// Whether a pre-commit validation failure rolls the reload back
    /// (true) or is recorded while the commit proceeds (false). Consumed
    /// from `WatcherConfig::rollback_on_validation_failure`.
    rollback_on_validation_failure: std::sync::atomic::AtomicBool,
    /// Hard pre-commit gate: an `Err` here rejects the reload outright
    /// (`ReloadRejected`), with no downgrade path. Shared swappable slot,
    /// same pattern as `health_check`/`validator`.
    pre_commit_check: ArcSwap<Option<Arc<dyn PreCommitCheck<T>>>>,
    /// Post-commit migration wiring: policy + injected registry +
    /// version transition, applied after every successful commit.
    #[cfg(feature = "migration")]
    migration: ArcSwap<Option<Arc<MigrationPlan>>>,
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

// reason 清洗实现在 super::sanitize（不随 progressive-reload 门控，shutdown 也用）。
use super::sanitize::flatten_reason;

/// Post-commit migration plan (R-watch-009).
///
/// After a successful reload commit the reloader invokes the injected
/// registry's `migrate(from → to)` according to the
/// [`MigrationOnReload`] policy. The registry stays injectable — e.g. the
/// `migration_registry()` generated by `#[derive(Config)]`, extended with
/// the needed migrations before being handed over here.
#[cfg(feature = "migration")]
struct MigrationPlan {
    registry: std::sync::Mutex<crate::migration::MigrationRegistry>,
    policy: crate::migration::MigrationOnReload,
    from_version: u32,
    to_version: u32,
    /// Last version the post-commit migration was applied for (used by the
    /// `OnVersionChange` policy). `0` = never applied.
    last_applied: std::sync::atomic::AtomicU32,
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
                validator: ArcSwap::new(Arc::new(None)),
                rollback_on_validation_failure: std::sync::atomic::AtomicBool::new(false),
                pre_commit_check: ArcSwap::new(Arc::new(None)),
                #[cfg(feature = "migration")]
                migration: ArcSwap::new(Arc::new(None)),
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
                validator: ArcSwap::new(Arc::new(None)),
                rollback_on_validation_failure: std::sync::atomic::AtomicBool::new(false),
                pre_commit_check: ArcSwap::new(Arc::new(None)),
                #[cfg(feature = "migration")]
                migration: ArcSwap::new(Arc::new(None)),
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

    /// Observe the configuration currently under trial (R-watch-009).
    ///
    /// Returns `Some` while a canary/linear reload holds a candidate in its
    /// trial window, and `None` before any reload and after a commit or
    /// rollback. This makes the trial phase observable to dashboards and
    /// orchestration without touching the committed `current()`.
    pub fn peek_candidate(&self) -> Option<Arc<T>> {
        let candidate = self.inner.candidate.load();
        candidate.as_ref().clone()
    }

    /// Attach (or replace) the pre-commit validator (R-watch-009).
    ///
    /// The validator lives in a shared slot (like the health check), so
    /// installing it through any clone affects every clone. It runs once per
    /// reload immediately before the candidate would be committed; see
    /// [`Self::with_rollback_on_validation_failure`] for what a failed
    /// validation does.
    pub fn with_validator(self, validator: Arc<dyn ReloadValidator<T>>) -> Self {
        self.inner.validator.store(Arc::new(Some(validator)));
        self
    }

    /// Attach (or replace) the hard pre-commit gate.
    ///
    /// Same shared-slot semantics as [`Self::with_health_check`]: installing
    /// through any clone affects every clone and never forks reloader state.
    /// Unlike the validator path there is no downgrade flag — a check `Err`
    /// rejects the reload with [`ConfigError::ReloadRejected`] and the
    /// current configuration stays untouched.
    pub fn with_pre_commit_check(self, check: Arc<dyn PreCommitCheck<T>>) -> Self {
        self.inner.pre_commit_check.store(Arc::new(Some(check)));
        self
    }

    /// Set whether a pre-commit validation failure rolls the reload back
    /// (R-watch-009).
    ///
    /// `true`: a validation failure keeps the current configuration and
    /// `begin_reload` returns `ConfigError::ReloadRolledBack`. `false` (the
    /// `WatcherConfig` default): the failure is logged and the commit
    /// proceeds. This mirrors `WatcherConfig::rollback_on_validation_failure`;
    /// prefer [`ProgressiveReloaderBuilder::watcher_config`] to consume the
    /// flag straight from the config.
    pub fn with_rollback_on_validation_failure(self, rollback: bool) -> Self {
        self.inner
            .rollback_on_validation_failure
            .store(rollback, std::sync::atomic::Ordering::SeqCst);
        self
    }

    /// Whether pre-commit validation failures roll the reload back.
    pub fn rollback_on_validation_failure(&self) -> bool {
        self.inner
            .rollback_on_validation_failure
            .load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Consume the watcher policy (R-watch-009): takes
    /// `WatcherConfig::rollback_on_validation_failure` into the pre-commit
    /// validation failure path, so the flag defined in
    /// [`super::WatcherConfig`] genuinely controls rollback behavior.
    pub fn with_watcher_config(self, config: super::WatcherConfig) -> Self {
        self.with_rollback_on_validation_failure(config.rollback_on_validation_failure)
    }

    /// Wire the post-commit migration (R-watch-009).
    ///
    /// After every successful commit the reloader invokes the injected
    /// registry's `migrate(from → to)` according to `policy`:
    /// - `MigrationOnReload::Always` runs on every commit;
    /// - `MigrationOnReload::OnVersionChange` runs only when `to` differs
    ///   from the last applied version;
    /// - `MigrationOnReload::Disabled` never runs.
    ///
    /// The registry is shared by all clones of the reloader and stays
    /// injectable — e.g. the `migration_registry()` generated by
    /// `#[derive(Config)]`, registered with the needed migrations first. A
    /// migration failure is logged and never un-commits the configuration.
    #[cfg(feature = "migration")]
    pub fn with_migration(
        self,
        registry: crate::migration::MigrationRegistry,
        policy: crate::migration::MigrationOnReload,
        from_version: u32,
        to_version: u32,
    ) -> Self {
        self.inner
            .migration
            .store(Arc::new(Some(Arc::new(MigrationPlan {
                registry: std::sync::Mutex::new(registry),
                policy,
                from_version,
                to_version,
                last_applied: std::sync::atomic::AtomicU32::new(0),
            }))));
        self
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

    /// Pre-commit validation (R-watch-009).
    ///
    /// Runs the attached validator (if any) right before the commit. A
    /// failure yields `Err(reason)`: with `rollback_on_validation_failure`
    /// set the caller rolls back; without it the failure is only logged and
    /// the commit proceeds.
    async fn validate_before_commit(&self, candidate: &Arc<T>) -> Result<(), String> {
        let validator = self.inner.validator.load_full();
        let Some(validator) = validator.as_ref() else {
            return Ok(());
        };
        if let Err(reason) = validator.validate(candidate).await {
            if self
                .inner
                .rollback_on_validation_failure
                .load(std::sync::atomic::Ordering::SeqCst)
            {
                return Err(reason);
            }
            log::warn!(
                "{}",
                tr_args(
                    "log-reload-validation-commit-anyway",
                    &[("reason", flatten_reason(&reason))]
                )
            );
        }
        Ok(())
    }

    /// Hard pre-commit gate: runs the attached check (if any) immediately
    /// before the swap-to-current. An `Err` is final — the candidate is
    /// dropped and the reload reports `ConfigError::ReloadRejected`.
    async fn enforce_pre_commit_check(&self, candidate: &Arc<T>) -> ConfigResult<()> {
        let check = self.inner.pre_commit_check.load_full();
        let Some(check) = check.as_ref() else {
            return Ok(());
        };
        if let Err(reason) = check.check(candidate).await {
            self.inner.candidate.store(Arc::new(None));
            // The reason reaches logs and the canary change stream verbatim,
            // so control characters are flattened and the length is bounded
            // (log-injection / unbounded-message guard).
            let detail = format!("pre-commit check: {}", flatten_reason(&reason));
            self.publish_canary_stage("rejected", &detail).await;
            return Err(ConfigError::ReloadRejected { reason: detail });
        }
        Ok(())
    }

    /// Commit `new_config` after pre-commit validation.
    ///
    /// A validation failure with the rollback flag set clears the candidate,
    /// publishes the `rolled_back` stage and returns
    /// `ConfigError::ReloadRolledBack`, keeping the current configuration.
    async fn commit(&self, new_config: Arc<T>, stage: &'static str) -> ConfigResult<()> {
        if let Err(reason) = self.validate_before_commit(&new_config).await {
            self.inner.candidate.store(Arc::new(None));
            let detail = tr_args(
                "error-reload-precommit-validation-failed",
                &[("reason", reason)],
            );
            self.publish_canary_stage("rolled_back", &detail).await;
            return Err(ConfigError::ReloadRolledBack { reason: detail });
        }
        self.enforce_pre_commit_check(&new_config).await?;
        self.inner.current.store(new_config);
        self.inner.candidate.store(Arc::new(None));
        self.publish_canary_stage("committed", stage).await;
        // Apply the post-commit migration per its policy.
        #[cfg(feature = "migration")]
        self.run_post_commit_migration();
        Ok(())
    }

    /// Run the post-commit migration per its [`MigrationOnReload`]-policy
    /// (R-watch-009). Failures are logged and never un-commit the
    /// configuration.
    #[cfg(feature = "migration")]
    fn run_post_commit_migration(&self) {
        use std::sync::atomic::Ordering;

        let plan = self.inner.migration.load_full();
        let Some(plan) = plan.as_ref() else {
            return;
        };
        let to = plan.to_version;
        let should_run = match plan.policy {
            crate::migration::MigrationOnReload::Disabled => false,
            crate::migration::MigrationOnReload::Always => true,
            crate::migration::MigrationOnReload::OnVersionChange => {
                plan.last_applied.load(Ordering::SeqCst) != to
            }
        };
        if !should_run {
            return;
        }

        let mut registry = plan.registry.lock().unwrap_or_else(|p| p.into_inner());
        // The reloader carries typed configuration, not an AnnotatedValue
        // tree; this post-commit call is the version-transition signal the
        // registry itself documents (see `MigrationRegistry::migrate`'s
        // example): registered migrations receive a null-seeded value and
        // apply their side effects for the from → to transition.
        let value = crate::types::AnnotatedValue::new(
            crate::types::ConfigValue::null(),
            crate::types::SourceId::new("progressive-reload"),
            "post-commit migration",
        );
        match registry.migrate(value, plan.from_version, plan.to_version) {
            Ok(_) => {
                plan.last_applied.store(to, Ordering::SeqCst);
            }
            Err(error) => {
                log::warn!(
                    "{}",
                    tr_args(
                        "log-post-commit-migration-failed",
                        &[
                            ("from", plan.from_version.to_string()),
                            ("to", plan.to_version.to_string()),
                            ("message", error.to_string()),
                        ]
                    )
                );
            }
        }
    }

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
                self.commit(new_config, "immediate").await?;
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
                        let reason = flatten_reason(&reason);
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

        self.commit(new_config, "canary").await?;
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
                        let step_no = (step + 1).to_string();
                        let detail = tr_args(
                            "error-reload-linear-step-rolled-back",
                            &[("step", step_no.clone()), ("reason", reason.clone())],
                        );
                        self.publish_canary_stage("rolled_back", &detail).await;
                        return Err(ConfigError::ReloadRolledBack {
                            reason: tr_args(
                                "error-reload-linear-step-failed",
                                &[("step", step_no), ("reason", reason)],
                            ),
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

        self.commit(new_config, "linear").await?;
        Ok(ReloadOutcome::Committed)
    }
}

pub struct ProgressiveReloaderBuilder<T: Clone + Send + Sync + 'static> {
    initial: Option<Arc<T>>,
    strategy: Option<ReloadStrategy>,
    health_check: Option<Arc<dyn ReloadHealthCheck>>,
    watcher_config: Option<super::WatcherConfig>,
    validator: Option<Arc<dyn ReloadValidator<T>>>,
    pre_commit_check: Option<Arc<dyn PreCommitCheck<T>>>,
}

impl<T: Clone + Send + Sync + 'static> ProgressiveReloaderBuilder<T> {
    pub fn new() -> Self {
        Self {
            initial: None,
            strategy: Some(ReloadStrategy::Immediate),
            health_check: None,
            watcher_config: None,
            validator: None,
            pre_commit_check: None,
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

    /// Consume the watcher policy: the config's
    /// `rollback_on_validation_failure` flag controls the pre-commit
    /// validation failure behavior of the built reloader.
    pub fn watcher_config(mut self, config: super::WatcherConfig) -> Self {
        self.watcher_config = Some(config);
        self
    }

    /// Attach the pre-commit validator (R-watch-009).
    pub fn validator(mut self, validator: Arc<dyn ReloadValidator<T>>) -> Self {
        self.validator = Some(validator);
        self
    }

    pub fn pre_commit_check(mut self, check: Arc<dyn PreCommitCheck<T>>) -> Self {
        self.pre_commit_check = Some(check);
        self
    }

    pub fn build(self) -> ProgressiveReloader<T> {
        let initial = self.initial.expect("initial configuration is required");
        let strategy = self.strategy.unwrap_or_default();
        let reloader = ProgressiveReloader::with_dependencies(initial, strategy, self.health_check);
        if let Some(config) = self.watcher_config {
            reloader.inner.rollback_on_validation_failure.store(
                config.rollback_on_validation_failure,
                std::sync::atomic::Ordering::SeqCst,
            );
        }
        if let Some(validator) = self.validator {
            reloader.inner.validator.store(Arc::new(Some(validator)));
        }
        if let Some(check) = self.pre_commit_check {
            reloader.inner.pre_commit_check.store(Arc::new(Some(check)));
        }
        reloader
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
            ) -> ConfigResult<Pin<Box<dyn futures_util::Stream<Item = ChangeEvent> + Send>>>
            {
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
                        e.old_value
                            .as_ref()
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        e.new_value
                            .as_ref()
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
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

    /// R-watch-009: `peek_candidate` exposes the trial configuration
    /// during a staged reload and is cleared after the commit.
    #[tokio::test]
    async fn peek_candidate_observes_trial_and_clears_after_commit() {
        let reloader = Arc::new(ProgressiveReloader::new(
            Arc::new(1i32),
            ReloadStrategy::Canary {
                trial_duration: Duration::from_millis(120),
                poll_interval: Duration::from_millis(10),
            },
        ));

        // Before any reload: no candidate.
        assert!(reloader.peek_candidate().is_none());

        let handle = {
            let reloader = Arc::clone(&reloader);
            tokio::spawn(async move {
                reloader
                    .begin_reload(Arc::new(2i32), Arc::new(MockProvider))
                    .await
            })
        };

        // During the trial window the candidate is observable.
        let mut saw_candidate = false;
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        while std::time::Instant::now() < deadline {
            if let Some(candidate) = reloader.peek_candidate() {
                assert_eq!(*candidate, 2, "the trial candidate is observable");
                saw_candidate = true;
                break;
            }
            if handle.is_finished() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(2)).await;
        }
        assert!(
            saw_candidate,
            "peek_candidate must return Some during the canary trial"
        );

        let outcome = handle.await.unwrap().unwrap();
        assert!(matches!(outcome, ReloadOutcome::Committed));
        assert!(
            reloader.peek_candidate().is_none(),
            "candidate is cleared after the commit"
        );
        assert_eq!(*reloader.current(), 2);
    }

    /// Always-rejecting pre-commit validator.
    struct RejectingValidator;

    #[async_trait]
    impl ReloadValidator<i32> for RejectingValidator {
        async fn validate(&self, _candidate: &i32) -> Result<(), String> {
            Err("candidate rejected".to_string())
        }
    }

    /// R-watch-009: a pre-commit validation failure with
    /// `rollback_on_validation_failure = true` keeps the current
    /// configuration and reports `ReloadRolledBack`; with the flag unset the
    /// commit proceeds. The flag is genuinely consumed from
    /// `WatcherConfig` (directly and through the builder).
    #[tokio::test]
    async fn validation_failure_rollback_follows_the_config_flag() {
        // Flag set: rollback, keep the old value, candidate cleared.
        let reloader = ProgressiveReloader::new(Arc::new(1i32), ReloadStrategy::Immediate)
            .with_validator(Arc::new(RejectingValidator))
            .with_rollback_on_validation_failure(true);
        assert!(reloader.rollback_on_validation_failure());

        let outcome = reloader
            .begin_reload(Arc::new(2i32), Arc::new(MockProvider))
            .await;
        assert!(matches!(outcome, Err(ConfigError::ReloadRolledBack { .. })));
        assert_eq!(
            *reloader.current(),
            1,
            "current configuration kept on validation failure"
        );
        assert!(reloader.peek_candidate().is_none());

        // Flag unset (WatcherConfig default): the failure is recorded and
        // the commit proceeds.
        let reloader = ProgressiveReloader::new(Arc::new(1i32), ReloadStrategy::Immediate)
            .with_validator(Arc::new(RejectingValidator))
            .with_watcher_config(super::super::WatcherConfig::default());
        assert!(!reloader.rollback_on_validation_failure());
        let outcome = reloader
            .begin_reload(Arc::new(2i32), Arc::new(MockProvider))
            .await;
        assert!(matches!(outcome, Ok(ReloadOutcome::Committed)));
        assert_eq!(*reloader.current(), 2);

        // The flag flows from WatcherConfig through the builder.
        let config =
            super::super::WatcherConfig::default().with_rollback_on_validation_failure(true);
        let reloader = ProgressiveReloader::builder()
            .initial(Arc::new(5i32))
            .validator(Arc::new(RejectingValidator))
            .watcher_config(config)
            .build();
        let outcome = reloader
            .begin_reload(Arc::new(6i32), Arc::new(MockProvider))
            .await;
        assert!(matches!(outcome, Err(ConfigError::ReloadRolledBack { .. })));
        assert_eq!(*reloader.current(), 5);
    }

    /// Always-accepting pre-commit check.
    struct AcceptingCheck;

    #[async_trait]
    impl PreCommitCheck<i32> for AcceptingCheck {
        async fn check(&self, _candidate: &i32) -> Result<(), String> {
            Ok(())
        }
    }

    /// Always-rejecting pre-commit check.
    struct RejectingCheck;

    #[async_trait]
    impl PreCommitCheck<i32> for RejectingCheck {
        async fn check(&self, _candidate: &i32) -> Result<(), String> {
            Err("invariant violated".to_string())
        }
    }

    /// A pre-commit check rejection is a hard stop: no commit, candidate
    /// cleared, `ReloadRejected` reported — unlike the validator path, no
    /// flag can turn this failure into a degraded commit.
    #[tokio::test]
    async fn pre_commit_check_rejection_blocks_the_commit_hard() {
        let reloader = ProgressiveReloader::new(Arc::new(1i32), ReloadStrategy::Immediate)
            .with_pre_commit_check(Arc::new(RejectingCheck));

        let outcome = reloader
            .begin_reload(Arc::new(2i32), Arc::new(MockProvider))
            .await;
        match outcome {
            Err(ConfigError::ReloadRejected { reason }) => assert!(
                reason.contains("invariant violated"),
                "reason must carry the check message, got: {reason}"
            ),
            other => panic!("expected ReloadRejected, got {other:?}"),
        }
        assert_eq!(
            *reloader.current(),
            1,
            "current configuration kept on rejection"
        );
        assert!(reloader.peek_candidate().is_none());
    }

    /// A passing pre-commit check lets the reload commit as usual.
    #[tokio::test]
    async fn pre_commit_check_pass_allows_the_commit() {
        let reloader = ProgressiveReloader::new(Arc::new(1i32), ReloadStrategy::Immediate)
            .with_pre_commit_check(Arc::new(AcceptingCheck));

        let outcome = reloader
            .begin_reload(Arc::new(2i32), Arc::new(MockProvider))
            .await;
        assert!(matches!(outcome, Ok(ReloadOutcome::Committed)));
        assert_eq!(*reloader.current(), 2);
    }

    /// Without an attached pre-commit check the commit path is unchanged.
    #[tokio::test]
    async fn no_pre_commit_check_keeps_the_commit_path_unchanged() {
        let reloader = ProgressiveReloader::new(Arc::new(1i32), ReloadStrategy::Immediate);

        let outcome = reloader
            .begin_reload(Arc::new(2i32), Arc::new(MockProvider))
            .await;
        assert!(matches!(outcome, Ok(ReloadOutcome::Committed)));
        assert_eq!(*reloader.current(), 2);
    }

    /// `ReloadRejected` metadata: code mapping (the frozen `ErrorCode` has no
    /// free code for it, so it shares the reload-failure family), localized
    /// key, and the en-template-mirrors-Display guard discipline.
    #[test]
    fn reload_rejected_error_metadata() {
        use crate::i18n::{I18nExt, LocalizedMsg};

        let err = ConfigError::ReloadRejected {
            reason: "bad".into(),
        };
        assert_eq!(err.code(), crate::error::ErrorCode::ReloadRolledBack);
        assert!(
            err.user_message().contains("bad"),
            "reason must reach the user message"
        );
        assert_eq!(LocalizedMsg::message_key(&err), "error-reload-rejected");
        assert_eq!(err.to_string(), err.message_en());
    }

    /// R-watch-009: after a successful commit the injected migration
    /// registry is invoked according to the `MigrationOnReload` policy.
    #[cfg(feature = "migration")]
    mod migration_wiring {
        use super::*;
        use crate::migration::{MigrationOnReload, MigrationRegistry};
        use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};

        fn recording_registry(calls: Arc<AtomicUsize>) -> MigrationRegistry {
            let mut registry = MigrationRegistry::new();
            registry.register(1, 2, move |value| {
                calls.fetch_add(1, AtomicOrdering::SeqCst);
                Ok(value)
            });
            registry.precompute_paths();
            registry
        }

        #[tokio::test]
        async fn post_commit_migration_follows_the_policy() {
            // Always: every commit invokes the registry.
            let calls = Arc::new(AtomicUsize::new(0));
            let reloader = ProgressiveReloader::new(Arc::new(1i32), ReloadStrategy::Immediate)
                .with_migration(
                    recording_registry(Arc::clone(&calls)),
                    MigrationOnReload::Always,
                    1,
                    2,
                );
            reloader
                .begin_reload(Arc::new(2i32), Arc::new(MockProvider))
                .await
                .unwrap();
            reloader
                .begin_reload(Arc::new(3i32), Arc::new(MockProvider))
                .await
                .unwrap();
            assert_eq!(
                calls.load(AtomicOrdering::SeqCst),
                2,
                "Always migrates on every commit"
            );

            // OnVersionChange: the second commit to the same version is
            // skipped.
            let calls = Arc::new(AtomicUsize::new(0));
            let reloader = ProgressiveReloader::new(Arc::new(1i32), ReloadStrategy::Immediate)
                .with_migration(
                    recording_registry(Arc::clone(&calls)),
                    MigrationOnReload::OnVersionChange,
                    1,
                    2,
                );
            reloader
                .begin_reload(Arc::new(2i32), Arc::new(MockProvider))
                .await
                .unwrap();
            reloader
                .begin_reload(Arc::new(3i32), Arc::new(MockProvider))
                .await
                .unwrap();
            assert_eq!(
                calls.load(AtomicOrdering::SeqCst),
                1,
                "OnVersionChange migrates only once per target version"
            );

            // Disabled: never invoked.
            let calls = Arc::new(AtomicUsize::new(0));
            let reloader = ProgressiveReloader::new(Arc::new(1i32), ReloadStrategy::Immediate)
                .with_migration(
                    recording_registry(Arc::clone(&calls)),
                    MigrationOnReload::Disabled,
                    1,
                    2,
                );
            reloader
                .begin_reload(Arc::new(2i32), Arc::new(MockProvider))
                .await
                .unwrap();
            assert_eq!(
                calls.load(AtomicOrdering::SeqCst),
                0,
                "Disabled never invokes the registry"
            );
        }

        /// A failed migration must not un-commit the configuration.
        #[tokio::test]
        async fn failed_migration_keeps_the_commit() {
            let mut registry = MigrationRegistry::new();
            registry.register(1, 2, |_| {
                Err(crate::error::ConfigError::migration_failed(1, 2, "boom"))
            });
            registry.precompute_paths();

            let reloader = ProgressiveReloader::new(Arc::new(1i32), ReloadStrategy::Immediate)
                .with_migration(registry, MigrationOnReload::Always, 1, 2);
            let outcome = reloader
                .begin_reload(Arc::new(2i32), Arc::new(MockProvider))
                .await
                .unwrap();
            assert!(matches!(outcome, ReloadOutcome::Committed));
            assert_eq!(*reloader.current(), 2, "commit survives migration failure");
        }
    }
}
