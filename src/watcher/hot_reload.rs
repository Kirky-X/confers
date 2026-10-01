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
/// `tokio::sync::watch` broadcast channel, and [`WatcherGuard`]-style
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

        let reloader = Arc::new(reloader);
        let task = {
            let reloader = Arc::clone(&reloader);
            let reload_tx = reload_tx.clone();
            let failures = Arc::clone(&failures);
            tokio::spawn(run_event_loop(
                reloader, watcher, load, reload_tx, failures, stop_rx,
            ))
        };
        let guard = super::WatcherGuard::with_task(Arc::new(AtomicBool::new(true)), task);

        Self {
            reloader,
            reload_tx,
            stop_tx,
            failures,
            guard,
        }
    }

    /// The currently committed configuration.
    pub fn current(&self) -> Arc<T> {
        self.reloader.current()
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
    pub async fn begin_reload(
        &self,
        new_config: Arc<T>,
        provider: Arc<dyn ConfigProvider>,
    ) -> ConfigResult<ReloadOutcome> {
        let outcome = self
            .reloader
            .begin_reload(new_config.clone(), provider)
            .await?;
        // A missing subscriber makes `send` fail; that is fine — watch
        // semantics hand the latest value to every future subscriber.
        let _ = self.reload_tx.send(new_config);
        Ok(outcome)
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
                        match reloader.begin_reload(candidate.clone(), provider).await {
                            Ok(_) => {
                                // No subscriber → send fails; watch
                                // semantics give future subscribers the
                                // latest value, so this is harmless.
                                let _ = reload_tx.send(candidate);
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
