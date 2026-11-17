// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! Hot reload facade: file watching + staged reload + broadcast + graceful
//! shutdown assembled into one component.
//!
//! This module fixes one particular wiring of the watcher primitives into a
//! ready-to-use component. Callers who need a different shape (custom event
//! filtering, multi-file watching, manual orchestration) should keep using
//! [`FsWatcher`](super::FsWatcher) and
//! [`ProgressiveReloader`](super::ProgressiveReloader) directly — those
//! building blocks are intentionally untouched by this facade.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

use super::sanitize::flatten_reason;
use super::{FsWatcher, ProgressiveReloader, ReloadOutcome};
use crate::error::{ConfersResult, ConfigResult};
use crate::i18n::tr_args;
use crate::interface::ConfigProvider;

#[cfg(feature = "security")]
use crate::security::SecurityPolicy;
#[cfg(feature = "security")]
use crate::types::AnnotatedValue;
#[cfg(feature = "security")]
use arc_swap::ArcSwap;

/// Loader closure the facade calls when a file event arrives: rebuilds the
/// configuration from disk and hands back the new config together with its
/// provider (the provider feeds canary/linear health checks).
///
/// The closure runs on tokio's blocking thread pool, so a blocking disk read
/// never stalls the async reactor — but a long-running loader still delays
/// subsequent reloads, and a panic inside it is caught, counted as a reload
/// failure, and logged.
pub type HotReloadLoader<T> =
    Arc<dyn Fn() -> ConfigResult<(Arc<T>, Arc<dyn ConfigProvider>)> + Send + Sync>;

/// Facade binding [`FsWatcher`], [`ProgressiveReloader::begin_reload`], a
/// `tokio::sync::watch` broadcast channel, and `WatcherGuard`-style
/// graceful shutdown into a single hot-reload loop.
///
/// # Event flow
///
/// ```text
/// file change → FsWatcher event → loader() → begin_reload(candidate, provider)
///            → committed candidate broadcast via watch channel
/// ```
///
/// Loader failures and rejected reloads are never silent: they increment
/// [`reload_failures()`](Self::reload_failures) (queryable) and are logged.
/// The current configuration stays untouched in both cases.
///
/// # Event queueing semantics
///
/// Events are processed strictly one at a time. A
/// [`ReloadStrategy::Canary`](super::ReloadStrategy) / `Linear` reload holds
/// the loop for its whole trial window, so file changes arriving during a
/// trial queue up in the watcher's bounded channel and trigger back-to-back
/// full reloads afterwards; once that channel fills, `FsWatcher` drops the
/// excess events (visible via its `dropped_events()`). Under high-frequency
/// writers prefer `Immediate` or debounce upstream.
///
/// # Manual entry point
///
/// [`begin_reload`](Self::begin_reload) bypasses the file-event loop for
/// callers that reload programmatically (admin push, scheduler, tests); it
/// goes through the same staged reloader and broadcast channel.
pub struct HotReloader<T: Clone + Send + Sync + 'static> {
    reloader: Arc<ProgressiveReloader<T>>,
    reload_tx: tokio::sync::watch::Sender<Arc<T>>,
    stop_tx: tokio::sync::watch::Sender<bool>,
    failures: Arc<AtomicU64>,
    guard: super::WatcherGuard,
    /// Zeroize-on-replace controls shared with the event loop so the policy
    /// can be installed after `spawn` returns.
    #[cfg(feature = "security")]
    zeroize_ctl: Arc<ZeroizeCtl>,
}

/// Zeroize-on-replace controls: the declarative policy (zeroize-on-replace /
/// snapshot mode / audit-on-reload) plus the scrubbed-leaf counter. Defaults
/// to the behavior-compatibility policy — every opt-in is explicit.
#[cfg(feature = "security")]
struct ZeroizeCtl {
    policy: ArcSwap<SecurityPolicy>,
    zeroized_fields: AtomicU64,
}

impl<T: Clone + Send + Sync + 'static> HotReloader<T> {
    /// Start the facade with an already-built reloader and watcher.
    ///
    /// * `reloader` — carries the initial config, the
    ///   [`ReloadStrategy`](super::ReloadStrategy), and any validator/health
    ///   check wiring; its current value seeds the broadcast channel.
    /// * `watcher` — watches the configuration file; consumed by the event
    ///   loop and stopped when the loop exits (shutdown or channel close).
    /// * `load` — rebuilds the configuration after each file event.
    pub fn spawn(
        reloader: ProgressiveReloader<T>,
        watcher: FsWatcher,
        load: HotReloadLoader<T>,
    ) -> Self {
        let initial = reloader.current();
        let (reload_tx, _rx) = tokio::sync::watch::channel(initial);
        let (stop_tx, stop_rx) = tokio::sync::watch::channel(false);
        let failures = Arc::new(AtomicU64::new(0));
        #[cfg(feature = "security")]
        let zeroize_ctl = Arc::new(ZeroizeCtl {
            policy: ArcSwap::new(Arc::new(SecurityPolicy::default())),
            zeroized_fields: AtomicU64::new(0),
        });

        let reloader = Arc::new(reloader);
        let task = {
            let reloader = Arc::clone(&reloader);
            let reload_tx = reload_tx.clone();
            let failures = Arc::clone(&failures);
            #[cfg(feature = "security")]
            let zeroize_ctl = Arc::clone(&zeroize_ctl);
            tokio::spawn(run_event_loop(
                reloader,
                watcher,
                load,
                reload_tx,
                failures,
                stop_rx,
                #[cfg(feature = "security")]
                zeroize_ctl,
            ))
        };
        let guard = super::WatcherGuard::with_task(Arc::new(AtomicBool::new(true)), task);

        Self {
            reloader,
            reload_tx,
            stop_tx,
            failures,
            guard,
            #[cfg(feature = "security")]
            zeroize_ctl,
        }
    }

    /// The currently committed configuration.
    pub fn current(&self) -> Arc<T> {
        self.reloader.current()
    }

    /// Install the declarative [`SecurityPolicy`] governing the swap
    /// semantics of this reloader (zeroize-on-replace, snapshot mode,
    /// audit-on-reload). The default is the behavior-compatibility policy —
    /// every opt-in is explicit.
    ///
    /// This is the assembly entry point for the watch pipeline: callers that
    /// build a [`SecurityPolicy`] alongside their
    /// [`ConfigBuilder`](crate::ConfigBuilder) hand it to the reloader here,
    /// right after [`spawn`](Self::spawn).
    #[cfg(feature = "security")]
    pub fn with_security_policy(self, policy: SecurityPolicy) -> Self {
        self.zeroize_ctl.policy.store(Arc::new(policy));
        self
    }

    /// Total number of sensitive string leaves scrubbed by
    /// zeroize-on-replace since this reloader started. A steady zero with
    /// `zeroize_on_replace` enabled usually means previous trees are still
    /// shared with readers (see [`SecurityPolicy`] boundary notes).
    #[cfg(feature = "security")]
    pub fn zeroized_fields(&self) -> u64 {
        self.zeroize_ctl.zeroized_fields.load(Ordering::SeqCst)
    }

    /// Subscribe to committed configuration updates. New subscribers see
    /// the latest value immediately.
    pub fn subscribe(&self) -> tokio::sync::watch::Receiver<Arc<T>> {
        self.reload_tx.subscribe()
    }

    /// Number of reload attempts that failed (loader error or the reloader
    /// rejected the candidate). Each failure is also logged; the current
    /// configuration is left untouched.
    pub fn reload_failures(&self) -> u64 {
        self.failures.load(Ordering::SeqCst)
    }

    /// Manual reload entry point: commit `new_config` through the staged
    /// reloader without waiting for a file event, then broadcast it.
    ///
    /// The swap-semantics wiring (zeroize-on-replace) applies here just as
    /// it does in the file-event loop: the previous pipeline-owned tree is
    /// scrubbed after the commit when the policy opts in.
    pub async fn begin_reload(
        &self,
        new_config: Arc<T>,
        provider: Arc<dyn ConfigProvider>,
    ) -> ConfigResult<ReloadOutcome> {
        #[cfg(feature = "security")]
        let previous = self.reloader.current();
        let outcome = self
            .reloader
            .begin_reload(new_config.clone(), provider)
            .await?;
        // A missing subscriber makes `send` fail; that is fine — watch
        // semantics hand the latest value to every future subscriber.
        // The send must precede the zeroize: the watch channel holds the
        // only other reference to the previous tree, and releasing it first
        // lets the scrub claim exclusive ownership.
        let _ = self.reload_tx.send(new_config);
        #[cfg(feature = "security")]
        self.zeroize_after_commit(&previous);
        Ok(outcome)
    }

    /// Scrub the pipeline-owned previous tree after a successful commit
    /// (zeroize-on-replace policy). Trees still shared with readers are
    /// skipped, never cloned-and-zeroized.
    #[cfg(feature = "security")]
    fn zeroize_after_commit(&self, previous: &Arc<T>) {
        apply_zeroize_policy(
            &self.zeroize_ctl.policy,
            &self.zeroize_ctl.zeroized_fields,
            previous,
        );
    }

    /// Stop the event loop and wait up to `timeout` for it to finish.
    ///
    /// Returns `Ok(true)` when the loop exited in time, `Ok(false)` when the
    /// timeout elapsed first (the loop keeps running in the background in
    /// that case — see [`WatcherGuard::shutdown`](super::WatcherGuard::shutdown)).
    pub async fn shutdown(&self, timeout: Duration) -> ConfersResult<bool> {
        let _ = self.stop_tx.send(true);
        self.guard.shutdown(timeout).await
    }
}

async fn run_event_loop<T: Clone + Send + Sync + 'static>(
    reloader: Arc<ProgressiveReloader<T>>,
    mut watcher: FsWatcher,
    load: HotReloadLoader<T>,
    reload_tx: tokio::sync::watch::Sender<Arc<T>>,
    failures: Arc<AtomicU64>,
    stop_rx: tokio::sync::watch::Receiver<bool>,
    #[cfg(feature = "security")] zeroize_ctl: Arc<ZeroizeCtl>,
) {
    // Stop is polled instead of racing the event await via `tokio::select!`:
    // the select macro needs tokio's "macros" feature, which the shared
    // tokio dependency does not enable; a bounded wait keeps the stop
    // latency at one tick without widening that dependency surface.
    const STOP_POLL_TICK: Duration = Duration::from_millis(100);
    loop {
        if matches!(stop_rx.has_changed(), Ok(true)) {
            break;
        }
        match tokio::time::timeout(STOP_POLL_TICK, watcher.recv()).await {
            Ok(None) => break,
            Ok(Some(_path)) => {
                // The loader does blocking disk I/O: run it on the blocking
                // thread pool so the reactor (and the stop-poll tick) keeps
                // breathing. A loader panic surfaces here as a JoinError and
                // is counted/logged like any other failure — never silent.
                let loader = Arc::clone(&load);
                match tokio::task::spawn_blocking(move || loader()).await {
                    Ok(Ok((candidate, provider))) => {
                        #[cfg(feature = "security")]
                        let previous = reloader.current();
                        match reloader.begin_reload(candidate.clone(), provider).await {
                            Ok(_) => {
                                // No subscriber → send fails; watch
                                // semantics give future subscribers the
                                // latest value, so this is harmless.
                                // The send must precede the zeroize: the
                                // watch channel holds the only other
                                // reference to the previous tree, and
                                // releasing it first lets the scrub claim
                                // exclusive ownership.
                                let _ = reload_tx.send(candidate);
                                #[cfg(feature = "security")]
                                apply_zeroize_policy(
                                    &zeroize_ctl.policy,
                                    &zeroize_ctl.zeroized_fields,
                                    &previous,
                                );
                            }
                            Err(e) => {
                                failures.fetch_add(1, Ordering::SeqCst);
                                log::error!(
                                    "{}",
                                    tr_args(
                                        "log-hot-reload-rejected",
                                        &[("reason", flatten_reason(&e.to_string()))]
                                    )
                                );
                            }
                        }
                    }
                    Ok(Err(e)) => {
                        failures.fetch_add(1, Ordering::SeqCst);
                        log::error!(
                            "{}",
                            tr_args(
                                "log-hot-reload-loader-failed",
                                &[("reason", flatten_reason(&e.to_string()))]
                            )
                        );
                    }
                    Err(join_err) => {
                        failures.fetch_add(1, Ordering::SeqCst);
                        log::error!(
                            "{}",
                            tr_args(
                                "log-hot-reload-loader-panicked",
                                &[("reason", flatten_reason(&join_err.to_string()))]
                            )
                        );
                    }
                }
            }
            Err(_elapsed) => continue,
        }
    }
}

/// Zeroize-on-replace wiring shared by the event loop and the manual
/// [`HotReloader::begin_reload`] path: scrub the pipeline-owned previous
/// tree when the policy opts in, and count the scrubbed leaves.
///
/// Call ordering matters: the caller must broadcast the new value through
/// the watch channel BEFORE this runs, so the channel releases its
/// reference to the previous tree and the scrub can claim exclusive
/// ownership (`Arc::get_mut`). Trees still shared with readers are skipped,
/// never cloned-and-zeroized (see [`SecurityPolicy`] boundary notes).
#[cfg(feature = "security")]
fn apply_zeroize_policy<T: Clone + Send + Sync + 'static>(
    policy: &ArcSwap<SecurityPolicy>,
    counter: &AtomicU64,
    previous: &Arc<T>,
) {
    let policy = policy.load_full();
    if !policy.zeroize_on_replace {
        return;
    }
    let mut previous = Arc::clone(previous);
    let scrubbed = zeroize_pipeline_tree(&mut previous);
    if scrubbed > 0 {
        counter.fetch_add(scrubbed as u64, Ordering::SeqCst);
        let count = scrubbed.to_string();
        log::debug!(
            "{}",
            tr_args("log-hot-reload-zeroized", &[("count", count)])
        );
    }
}

/// Specialize the scrub for the `T == AnnotatedValue` (dynamic/untyped
/// pipeline) case: downcast the exclusively-held previous value and zeroize
/// every sensitive string leaf. User-derived types own their trees; the
/// pipeline holds nothing to scrub there.
#[cfg(feature = "security")]
fn zeroize_pipeline_tree<T: Clone + Send + Sync + 'static>(previous: &mut Arc<T>) -> usize {
    let Some(value) = Arc::get_mut(previous) else {
        return 0;
    };
    let Some(annotated) = (value as &mut dyn std::any::Any).downcast_mut::<AnnotatedValue>() else {
        return 0;
    };
    crate::security::hot_swap::zeroize_sensitive_values(&mut annotated.inner)
}

#[cfg(all(test, feature = "security"))]
mod zeroize_tests {
    use super::*;
    use crate::types::{ConfigValue, SourceId};
    use std::sync::atomic::AtomicU64;

    struct EmptyProvider;

    impl ConfigProvider for EmptyProvider {
        fn get_raw(&self, _key: &str) -> Option<&AnnotatedValue> {
            None
        }

        fn keys(&self) -> Vec<String> {
            Vec::new()
        }
    }

    fn annotated_tree(password: &str) -> AnnotatedValue {
        let mut indexed = indexmap::IndexMap::new();
        indexed.insert(
            Arc::from("password"),
            AnnotatedValue::new(ConfigValue::string(password), SourceId::default(), ""),
        );
        indexed.insert(
            Arc::from("host"),
            AnnotatedValue::new(ConfigValue::string("db.local"), SourceId::default(), ""),
        );
        AnnotatedValue::new(ConfigValue::Map(Arc::new(indexed)), SourceId::default(), "")
    }

    #[test]
    fn pipeline_scrub_zeroizes_exclusively_held_tree_bytes() {
        // Byte-level semantics of the pipeline scrub helper: an exclusively
        // held AnnotatedValue tree has its sensitive strings zeroized, the
        // rest left intact, and the count reported.
        let mut tree = Arc::new(annotated_tree("s3cret"));
        let scrubbed = zeroize_pipeline_tree(&mut tree);
        assert_eq!(scrubbed, 1);
        let annotated = Arc::try_unwrap(tree).unwrap();
        let Some(ConfigValue::Map(map)) = Some(annotated.inner) else {
            panic!("map");
        };
        assert_eq!(
            map.get("password").unwrap().inner,
            ConfigValue::String(String::new()),
            "password bytes must be zeroized/cleared"
        );
        assert_eq!(
            map.get("host").unwrap().inner,
            ConfigValue::String("db.local".to_string()),
            "non-sensitive leaf keeps its value"
        );
    }

    #[tokio::test]
    async fn zeroize_on_replace_scrubs_previous_tree_across_rounds() {
        let dir = tempfile::tempdir().unwrap();
        let cfg_path = dir.path().join("app.conf");
        std::fs::write(&cfg_path, "value=1").unwrap();

        let watcher = FsWatcher::new(&cfg_path, 50).await.unwrap();
        let reloader = ProgressiveReloader::new(
            Arc::new(annotated_tree("pw-round-0")),
            super::super::ReloadStrategy::Immediate,
        );
        let round: Arc<AtomicU64> = Arc::new(AtomicU64::new(0));
        let round_loader = Arc::clone(&round);
        let loader: HotReloadLoader<AnnotatedValue> = Arc::new(move || {
            let r = round_loader.fetch_add(1, Ordering::SeqCst);
            Ok((
                Arc::new(annotated_tree(&format!("pw-round-{}", r + 1))),
                Arc::new(EmptyProvider),
            ))
        });

        let kit =
            HotReloader::spawn(reloader, watcher, loader).with_security_policy(SecurityPolicy {
                zeroize_on_replace: true,
                ..SecurityPolicy::default()
            });
        assert_eq!(kit.zeroized_fields(), 0);

        // Round 1 commits: the previous (initial) tree is scrubbed.
        kit.begin_reload(
            Arc::new(annotated_tree("pw-round-1")),
            Arc::new(EmptyProvider),
        )
        .await
        .unwrap();
        assert_eq!(kit.zeroized_fields(), 1, "initial tree scrubbed");
        assert_eq!(
            kit.current()
                .inner
                .as_map()
                .and_then(|m| m.get("password"))
                .and_then(|v| v.inner.as_str()),
            Some("pw-round-1"),
            "current tree carries the freshly committed value"
        );

        // Round 2 commits: the round-1 tree is scrubbed too.
        kit.begin_reload(
            Arc::new(annotated_tree("pw-round-2")),
            Arc::new(EmptyProvider),
        )
        .await
        .unwrap();
        assert_eq!(kit.zeroized_fields(), 2, "both previous trees scrubbed");

        kit.shutdown(Duration::from_millis(500)).await.unwrap();
    }

    #[tokio::test]
    async fn default_policy_keeps_previous_tree_intact() {
        let dir = tempfile::tempdir().unwrap();
        let cfg_path = dir.path().join("app.conf");
        std::fs::write(&cfg_path, "value=1").unwrap();

        let watcher = FsWatcher::new(&cfg_path, 50).await.unwrap();
        let initial = Arc::new(annotated_tree("pw-initial"));
        let initial_reader = Arc::clone(&initial);
        let reloader = ProgressiveReloader::new(initial, super::super::ReloadStrategy::Immediate);
        let loader: HotReloadLoader<AnnotatedValue> =
            Arc::new(|| Ok((Arc::new(annotated_tree("pw-next")), Arc::new(EmptyProvider))));

        // No policy installed → behavior-compatibility default.
        let kit = HotReloader::spawn(reloader, watcher, loader);
        kit.begin_reload(Arc::new(annotated_tree("pw-next")), Arc::new(EmptyProvider))
            .await
            .unwrap();
        assert_eq!(kit.zeroized_fields(), 0, "default policy never scrubs");
        assert_eq!(
            initial_reader
                .inner
                .as_map()
                .and_then(|m| m.get("password"))
                .and_then(|v| v.inner.as_str()),
            Some("pw-initial"),
            "the tree we hold keeps its bytes (and zeroize would have skipped a shared tree anyway)"
        );

        kit.shutdown(Duration::from_millis(500)).await.unwrap();
    }

    #[tokio::test]
    async fn zeroize_skips_tree_shared_with_readers() {
        let dir = tempfile::tempdir().unwrap();
        let cfg_path = dir.path().join("app.conf");
        std::fs::write(&cfg_path, "value=1").unwrap();

        let watcher = FsWatcher::new(&cfg_path, 50).await.unwrap();
        let reloader = ProgressiveReloader::new(
            Arc::new(annotated_tree("pw-shared")),
            crate::watcher::ReloadStrategy::Immediate,
        );
        let loader: HotReloadLoader<AnnotatedValue> =
            Arc::new(|| Ok((Arc::new(annotated_tree("pw-next")), Arc::new(EmptyProvider))));

        let kit =
            HotReloader::spawn(reloader, watcher, loader).with_security_policy(SecurityPolicy {
                zeroize_on_replace: true,
                ..SecurityPolicy::default()
            });
        // A reader that cloned the current value before the swap keeps the
        // previous tree alive -> the scrub must skip it, not clone-and-zero.
        let rx = kit.subscribe();
        let held = rx.borrow().clone();
        kit.begin_reload(Arc::new(annotated_tree("pw-next")), Arc::new(EmptyProvider))
            .await
            .unwrap();
        assert_eq!(
            kit.zeroized_fields(),
            0,
            "shared previous tree must be skipped (never cloned-and-zeroized)"
        );
        assert_eq!(
            held.inner
                .as_map()
                .and_then(|m| m.get("password"))
                .and_then(|v| v.inner.as_str()),
            Some("pw-shared"),
            "the reader's copy is untouched"
        );

        kit.shutdown(Duration::from_millis(500)).await.unwrap();
    }
}
