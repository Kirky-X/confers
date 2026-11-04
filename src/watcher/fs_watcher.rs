// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! Platform-level file debouncer using notify-debouncer-full.
//!
//! This module provides file system watching with platform-level debouncing,
//! wrapping the notify-debouncer-full crate for integration with confers.

#[cfg(feature = "watch")]
use crate::error::{ConfigError, ConfigResult};
#[cfg(feature = "watch")]
use crate::i18n::{t_simple, tr, tr_args};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;
use tokio::sync::mpsc;

use notify_debouncer_full::{DebounceEventResult, Debouncer, RecommendedCache};

/// Default recv timeout in milliseconds for polling the debouncer.
const DEFAULT_RECV_TIMEOUT_MS: u64 = 50;

/// Shared sender store for a watcher's event channel. Emptied by `stop()`
/// and by the watcher thread on failure: dropping the last sender closes the
/// channel, so a `recv()` that is already awaiting returns `None` instead of
/// hanging until `stop()` is called.
type SenderStore = Arc<std::sync::Mutex<Option<mpsc::Sender<PathBuf>>>>;

/// Close the channel by dropping the stored sender.
///
/// The watcher thread's own sender clone is dropped when the thread returns,
/// so after this call no sender remains and the channel is disconnected.
fn close_sender(store: &SenderStore) {
    let mut guard = store
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    *guard = None;
}

/// Absolute, canonical form of a watch target.
///
/// The watcher reports event paths in canonical form — notify canonicalizes
/// its watch root and joins event paths onto it — so every comparison against
/// a caller-supplied path has to use the same form. Comparing a raw path
/// against a canonicalized event path silently drops every event whenever
/// the target traverses a symlink (e.g. macOS' TMPDIR under `/var` →
/// `/private/var`), leaving the watcher permanently deaf.
fn canonical_watch_path(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| {
        // Target may not exist yet (delete+recreate flows): fall back to a
        // lexical absolutization against the current dir.
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir().unwrap_or_default().join(path)
        }
    })
}

/// Absolute form of a caller-supplied path that keeps symlinks untouched.
///
/// `std::path::absolute` resolves against the current directory without
/// resolving symlinks, so the watcher reports back exactly the path the
/// caller asked to watch — `event == watched_path` holds even when the
/// target sits behind a symlink.
fn lexical_absolute(path: &Path) -> PathBuf {
    std::path::absolute(path).unwrap_or_else(|_| canonical_watch_path(path))
}

/// Map a canonical event path back onto the caller's path form.
///
/// The watch root is canonical (notify canonicalizes it), so the event paths
/// are canonical as well. The consumer must receive the path form it handed
/// in, otherwise comparing the reported path with the watched path never
/// matches for targets behind a symlink.
fn reported_event_path(event_path: &Path, canonical_root: &Path, reported_root: &Path) -> PathBuf {
    match event_path.strip_prefix(canonical_root) {
        Ok(suffix) => reported_root.join(suffix),
        Err(_) => event_path.to_path_buf(),
    }
}

/// An OS-level watch that is already live.
///
/// Produced by [`FsWatcher::establish_watch`]: from the moment the value
/// exists, every change under the watch target is guaranteed observable by
/// the backend. This closes the R-watch-008 race where the watch was only
/// established later inside the watcher thread, silently dropping a change
/// written right after [`FsWatcher::new`].
struct EstablishedWatch {
    debouncer: Debouncer<super::notify_normalize::CreateAsModifyWatcher, RecommendedCache>,
    /// Receiver of debounced backend event batches.
    bridge_rx: std::sync::mpsc::Receiver<DebounceEventResult>,
    /// Directory actually watched: the file's parent for file targets.
    watch_target: PathBuf,
    /// Exact-path filter for file targets, `None` for directory targets.
    path_filter: Option<PathBuf>,
    /// Canonical watch root: event paths are canonical and are re-mapped
    /// onto the caller's path form through `reported`.
    absolute: PathBuf,
    /// Path form the consumer gets back (caller's path, made absolute).
    reported: PathBuf,
    /// Outlet throttle for file targets (R-watch-007).
    outlet_debouncer: Option<super::AdaptiveDebouncer>,
}

/// File system watcher with debouncing.
///
/// This watcher monitors file changes and emits debounced events
/// to avoid triggering multiple reloads for a single file modification.
pub struct FsWatcher {
    /// Path being watched
    watch_path: Arc<PathBuf>,
    /// Receiver for debounced file events
    rx: Option<mpsc::Receiver<PathBuf>>,
    /// Shared sender store for closing the channel (see `SenderStore`)
    tx: SenderStore,
    /// Handle to the watcher thread
    watcher_thread: Option<std::thread::JoinHandle<()>>,
    /// Running flag
    running: Arc<std::sync::atomic::AtomicBool>,
    /// Set by the watcher thread when it fails mid-run (parent directory
    /// removed, bridge channel died). Makes the failure observable through
    /// [`FsWatcher::is_running`] instead of the thread exiting silently.
    /// Establishment failures are not folded in here any more: the
    /// constructor returns `Err` for them (R-watch-008).
    failed: Arc<std::sync::atomic::AtomicBool>,
    /// Count of change events dropped because the event channel was full
    /// (slow consumer). Observable through [`FsWatcher::dropped_events`].
    dropped: Arc<std::sync::atomic::AtomicU64>,
}

impl Drop for FsWatcher {
    fn drop(&mut self) {
        self.stop();
    }
}

impl FsWatcher {
    /// Create a new file system watcher.
    ///
    /// # Arguments
    ///
    /// * `path` - The file or directory to watch
    /// * `debounce_ms` - Debounce duration in milliseconds (default: 200ms)
    ///
    /// # Example
    ///
    /// ```rust
    /// async fn example() -> Result<(), Box<dyn std::error::Error>> {
    ///     use confers::watcher::FsWatcher;
    ///
    ///     let mut watcher = FsWatcher::new("./config.toml", 200).await?;
    ///
    ///     // Wait for file changes
    ///     while let Some(path) = watcher.recv().await {
    ///         println!("File changed: {:?}", path);
    ///     }
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn new(path: impl AsRef<Path>, debounce_ms: u64) -> ConfigResult<Self> {
        Self::with_recv_timeout(path, debounce_ms, DEFAULT_RECV_TIMEOUT_MS).await
    }

    /// Create a new file system watcher with custom recv timeout.
    ///
    /// # Arguments
    ///
    /// * `path` - The file or directory to watch
    /// * `debounce_ms` - Debounce duration in milliseconds
    /// * `recv_timeout_ms` - Recv timeout for polling debouncer events (default: 50ms)
    ///
    /// Lower values mean faster response but higher CPU usage.
    /// Higher values mean slower response but lower CPU usage.
    ///
    /// # Establishment contract (R-watch-008)
    ///
    /// The OS-level watch is established synchronously before this
    /// constructor returns: after an `Ok(_)` return the watch is live, so a
    /// change written by the caller at that instant is guaranteed
    /// observable. Establishment failure (path gone, debouncer creation or
    /// backend watch refused) is returned as `Err` — no deaf watcher is
    /// handed out. Failures *after* establishment keep the observable
    /// failure protocol: [`is_running()`](Self::is_running) reports `false`
    /// and [`recv()`](Self::recv) returns `None` instead of blocking
    /// forever.
    ///
    /// Residual limitation: the canonicalize → backend-watch handshake is
    /// not atomic against symlink re-pointing (requires a local attacker
    /// with a writable parent directory). This fix narrowed the
    /// pre-existing window; eliminating it would need an inode re-check
    /// after establishment.
    pub async fn with_recv_timeout(
        path: impl AsRef<Path>,
        debounce_ms: u64,
        recv_timeout_ms: u64,
    ) -> ConfigResult<Self> {
        let watch_path = Arc::new(path.as_ref().to_path_buf());

        if !watch_path.exists() {
            return Err(ConfigError::FileNotFound {
                filename: watch_path.as_ref().clone(),
                source: None,
            });
        }

        // Establish the OS-level watch before this constructor returns.
        // Establishing inside the spawned thread left a scheduling-dependent
        // window in which a change written by the caller right after `new()`
        // happened before the backend watch existed and was silently lost
        // forever (R-watch-008). After this call the watch is live: the
        // first change the caller issues is observable. The blocking
        // backend handshake runs on the blocking pool so the async worker
        // is not stalled (up to a few ms per watcher, more on network
        // mounts).
        let path_for_establish = Arc::clone(&watch_path);
        let watch = tokio::task::spawn_blocking(move || {
            Self::establish_watch(path_for_establish.as_path(), debounce_ms)
        })
        .await
        .map_err(|e| ConfigError::WatcherError {
            // No panic payload in the message: the payload can carry
            // arbitrary variable content (CWE-209); the boolean split keeps
            // the two failure modes distinguishable without it.
            message: if e.is_panic() {
                t_simple("error-watcher-establish-task-panicked")
            } else {
                t_simple("error-watcher-establish-task-cancelled")
            },
            path: Some(watch_path.as_ref().clone()),
            recoverable: false,
        })??;

        let (tx, rx) = mpsc::channel(100);
        let tx_store: SenderStore = Arc::new(std::sync::Mutex::new(Some(tx)));
        let running = Arc::new(std::sync::atomic::AtomicBool::new(true));
        let failed = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let dropped = Arc::new(std::sync::atomic::AtomicU64::new(0));
        let running_clone = Arc::clone(&running);
        let failed_clone = Arc::clone(&failed);
        let dropped_clone = Arc::clone(&dropped);
        let tx_for_thread = SenderStore::clone(&tx_store);

        // Spawn the watcher in a dedicated thread (not tokio task); it only
        // forwards already-established watch events.
        let watcher_thread = std::thread::spawn(move || {
            Self::forward_loop(
                watch,
                recv_timeout_ms,
                tx_for_thread,
                running_clone,
                failed_clone,
                dropped_clone,
            );
        });

        Ok(Self {
            watch_path,
            rx: Some(rx),
            tx: tx_store,
            watcher_thread: Some(watcher_thread),
            running,
            failed,
            dropped,
        })
    }

    /// Receive the next file change event.
    ///
    /// Returns `Some(path)` when a file change is detected, `None` if the
    /// watcher is stopped or has failed mid-run (in which case
    /// [`is_running()`](Self::is_running) also reports `false`).
    ///
    /// The `is_running()` check is a fast path only: if the watcher thread
    /// fails while a `recv()` call is already awaiting, the thread closes
    /// the channel (see `SenderStore`) and the pending `recv()` returns
    /// `None` — it can never hang.
    pub async fn recv(&mut self) -> Option<PathBuf> {
        // A stopped or failed watcher can never produce events again;
        // return `None` instead of waiting on a channel that stays open.
        if !self.is_running() {
            return None;
        }
        if let Some(ref mut rx) = self.rx {
            let path = rx.recv().await;
            if path.is_some() {
                // Critical-path span: a debounced file event is handed to the
                // reload pipeline.
                #[cfg(feature = "tracing")]
                let _reload = tracing::info_span!("confers.reload").entered();
                crate::telemetry::event("confers.reload.triggered", &[("file", "changed")]);
            }
            path
        } else {
            None
        }
    }

    /// Get the path being watched.
    pub fn watch_path(&self) -> &Path {
        &self.watch_path
    }

    /// Stop the watcher.
    pub fn stop(&mut self) {
        if !self.running.load(std::sync::atomic::Ordering::SeqCst) {
            return;
        }

        self.running
            .store(false, std::sync::atomic::Ordering::SeqCst);

        // Drop the stored sender to close the channel, which will cause
        // recv() to return None
        close_sender(&self.tx);

        // Wait for the watcher thread to finish
        if let Some(handle) = self.watcher_thread.take() {
            let _ = handle.join();
        }

        self.rx.take();
    }

    /// Check if the watcher is running.
    ///
    /// Returns `false` after [`stop()`](Self::stop) and also when the
    /// watcher thread failed mid-run (parent directory removed, bridge
    /// channel died). Establishment failures are not part of this state:
    /// the constructor returns `Err` for them (R-watch-008).
    pub fn is_running(&self) -> bool {
        self.running.load(std::sync::atomic::Ordering::SeqCst)
            && !self.failed.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Number of change events dropped because the event channel was full
    /// (slow consumer).
    ///
    /// Every dropped event also emits a warning log, but the counter makes
    /// the loss queryable after the fact — a non-zero value means the
    /// consumer fell behind and missed file-change notifications (a reload
    /// trigger was lost and should be reconciled manually).
    pub fn dropped_events(&self) -> u64 {
        self.dropped.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Establish the OS-level watch synchronously on the calling thread.
    ///
    /// Blocks briefly (on macOS this spans `FSEventStreamStart`, typically
    /// well under a millisecond) so that after this call the watch is live:
    /// every change issued afterwards is guaranteed observable
    /// (R-watch-008). Every establishment failure is returned as an error
    /// instead of handing out a watcher that would silently never deliver
    /// events.
    fn establish_watch(path: &Path, debounce_ms: u64) -> ConfigResult<EstablishedWatch> {
        use notify_debouncer_full::new_debouncer_opt;
        use notify_debouncer_full::notify::{Config, RecursiveMode};

        // Watch strategy: a file target is watched through its PARENT
        // directory with an exact-path filter, mirroring the proven
        // `MultiFsWatcher` rename protection. Watching the file's own inode
        // loses the watch on `rename(tmp, path)` atomic replacement (notify
        // does not re-arm on `MOVE_SELF`), so every change after the first
        // editor-style save would be silently missed. Watching the parent
        // directory survives both replacement and delete+recreate of the
        // watched path. Directory targets are watched directly and need no
        // filter (a directory inode is not swapped by renames of children).
        // both the watch root and the filter must be ABSOLUTE.
        // notify-debouncer-full canonicalizes its watch root and joins event
        // paths onto it, while `path_filter` would otherwise hold the
        // caller's raw (possibly relative) path — the two never compare
        // equal, every event gets filtered out, and the watcher goes
        // silently deaf even though `is_running()` stays true (the rustdoc
        // example's `FsWatcher::new("./config.toml", ..)` used to hit
        // exactly this).
        let absolute = canonical_watch_path(path);
        // The path form the consumer gets back: the caller's own path made
        // absolute, symlinks untouched.
        let reported = lexical_absolute(path);
        let (watch_target, path_filter): (PathBuf, Option<PathBuf>) = if absolute.is_dir() {
            (absolute.clone(), None)
        } else {
            let parent = absolute
                .parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| absolute.clone());
            (parent, Some(absolute.clone()))
        };

        // R-watch-007: for file targets the event outlet passes
        // through the library's own `AdaptiveDebouncer` (window =
        // `debounce_ms`), so rapid consecutive writes collapse into a
        // bounded number of forwarded events. The notify-level debounce
        // coalesces within its window; this second layer bounds
        // cross-window duplicates too. The final value is never lost: every
        // forwarded event makes the consumer reload the current file
        // content. Directory targets skip this layer — a single shared
        // time-throttle across different files would silently drop one
        // file's event because another file was forwarded just before.
        let outlet_debouncer = path_filter
            .as_ref()
            .map(|_| super::AdaptiveDebouncer::new(debounce_ms));

        // Create a bridge channel for the debouncer callback
        let (bridge_tx, bridge_rx) = std::sync::mpsc::channel::<DebounceEventResult>();

        let mut debouncer = new_debouncer_opt::<
            _,
            super::notify_normalize::CreateAsModifyWatcher,
            RecommendedCache,
        >(
            Duration::from_millis(debounce_ms),
            None,
            move |result: DebounceEventResult| {
                let _ = bridge_tx.send(result);
            },
            RecommendedCache::new(),
            Config::default(),
        )
        .map_err(|e| {
            log::error!(
                "failed to create file watcher debouncer for {}: {e}",
                path.display()
            );
            ConfigError::WatcherError {
                message: format!(
                    "failed to create file watcher debouncer for {}: {e}",
                    path.display()
                ),
                path: Some(path.to_path_buf()),
                recoverable: false,
            }
        })?;

        debouncer
            .watch(&watch_target, RecursiveMode::Recursive)
            .map_err(|e| {
                log::error!(
                    "{}",
                    tr_args(
                        "log-fs-watch-failed",
                        &[
                            ("path", watch_target.display().to_string()),
                            ("message", e.to_string()),
                        ]
                    )
                );
                // The error surfaces to the caller in the caller's own path
                // form: the canonical target would expose the symlink
                // resolution (e.g. `/etc/app` → `/data/app-v2`) beyond the
                // internal log, where it stays for debugging.
                ConfigError::WatcherError {
                    message: tr_args(
                        "log-fs-watch-failed",
                        &[
                            ("path", path.display().to_string()),
                            ("message", e.to_string()),
                        ],
                    ),
                    path: Some(path.to_path_buf()),
                    recoverable: false,
                }
            })?;

        Ok(EstablishedWatch {
            debouncer,
            bridge_rx,
            watch_target,
            path_filter,
            absolute,
            reported,
            outlet_debouncer,
        })
    }

    /// Forward debounced events from an established watch to the consumer
    /// channel until the watcher is stopped or fails observably.
    ///
    /// `tx_store` is the shared sender store: on any failure this thread
    /// empties it and drops its own sender clone on return, closing the
    /// channel so a concurrently awaiting `recv()` returns `None` instead of
    /// hanging (see `SenderStore`).
    fn forward_loop(
        watch: EstablishedWatch,
        recv_timeout_ms: u64,
        tx_store: SenderStore,
        running: Arc<std::sync::atomic::AtomicBool>,
        failed: Arc<std::sync::atomic::AtomicBool>,
        dropped: Arc<std::sync::atomic::AtomicU64>,
    ) {
        use notify_debouncer_full::notify::EventKind;

        let EstablishedWatch {
            debouncer,
            bridge_rx,
            watch_target,
            path_filter,
            absolute,
            reported,
            outlet_debouncer,
        } = watch;

        let tx = {
            let guard = tx_store
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            guard.as_ref().map(|s| s.clone())
        };
        let Some(tx) = tx else {
            // `stop()` closed the store before the forwarding loop started.
            drop(debouncer);
            return;
        };
        // The thread keeps its `tx_store` reference for the whole run: a
        // mid-run bridge failure must still be able to close the outgoing
        // channel (same as `MultiFsWatcher`'s forwarding loop).

        let recv_timeout = Duration::from_millis(recv_timeout_ms);

        // Process events
        while running.load(std::sync::atomic::Ordering::SeqCst) {
            match bridge_rx.recv_timeout(recv_timeout) {
                Ok(result) => {
                    if let Ok(events) = result {
                        for event in events {
                            // the watched PARENT directory itself being
                            // removed/renamed kills the kernel watch (its inode
                            // is gone) — without detection the watcher stays
                            // silently deaf forever (is_running stays true, no
                            // events ever arrive). Mirror the bridge-failure
                            // protocol: mark failed, close the channel, stop.
                            if path_filter.is_some()
                                && matches!(event.kind, EventKind::Remove(_))
                                && event.paths.iter().any(|p| {
                                    Some(p.as_path())
                                        == path_filter.as_ref().and_then(|f| f.parent())
                                })
                            {
                                log::error!(
                                    "{}",
                                    tr_args(
                                        "log-fs-watcher-parent-removed",
                                        &[(
                                            "path",
                                            format!(
                                                "{:?}",
                                                path_filter.as_ref().and_then(|f| f.parent())
                                            )
                                        )]
                                    )
                                );
                                failed.store(true, Ordering::SeqCst);
                                running.store(false, Ordering::SeqCst);
                                if let Ok(mut store) = tx_store.lock() {
                                    store.take(); // close the channel: recv() returns None
                                }
                                return;
                            }
                            match event.kind {
                                EventKind::Create(_)
                                | EventKind::Modify(_)
                                | EventKind::Remove(_) => {
                                    // Forward all file-system events for paths
                                    // within the watched directory. The is_file()
                                    // check was removed because it drops deletion
                                    // events (the path no longer exists) and can
                                    // race with creation events on some platforms.
                                    //
                                    // For file targets every forwarded path must
                                    // equal the watched path exactly (same filter
                                    // discipline as `MultiFsWatcher`): the parent
                                    // watch also observes sibling files and the
                                    // rename source, none of which are changes to
                                    // the watched file.
                                    for event_path in &event.paths {
                                        if path_filter
                                            .as_ref()
                                            .is_none_or(|filter| filter == event_path)
                                        {
                                            // Outlet throttle (file targets only):
                                            // admit at most one event per window;
                                            // within-window duplicates are merged.
                                            let forward = outlet_debouncer
                                                .as_ref()
                                                .is_none_or(|d| d.should_process());
                                            if forward {
                                                match tx.try_send(reported_event_path(
                                                    event_path, &absolute, &reported,
                                                )) {
                                                    Ok(_) => {
                                                        // Critical-path metric: watcher
                                                        // trigger forwarded to reload
                                                        // consumers.
                                                        crate::metrics::record_counter(
                                                            crate::metrics::names::WATCHER_EVENTS_TOTAL,
                                                            &[],
                                                        );
                                                    }
                                                    Err(mpsc::error::TrySendError::Full(_)) => {
                                                        // Channel full — the consumer fell
                                                        // behind. Make the loss observable:
                                                        // warn now, count for later audit
                                                        // (R-watch-002).
                                                        let total = dropped
                                                            .fetch_add(1, Ordering::SeqCst)
                                                            + 1;
                                                        log::warn!(
                                                            "{}",
                                                            tr_args(
                                                                "log-fs-watcher-channel-full",
                                                                &[
                                                                    (
                                                                        "path",
                                                                        event_path
                                                                            .display()
                                                                            .to_string()
                                                                    ),
                                                                    ("total", total.to_string()),
                                                                ]
                                                            )
                                                        );
                                                    }
                                                    Err(mpsc::error::TrySendError::Closed(_)) => {
                                                        running.store(
                                                            false,
                                                            std::sync::atomic::Ordering::SeqCst,
                                                        );
                                                        return;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                    // Bridge died mid-run (debouncer gone): no further events
                    // can arrive. Mirror the establishment-failure protocol —
                    // leaving `running`/`failed` untouched here would keep
                    // `is_running()` true and block a pending `recv()` forever.
                    running.store(false, std::sync::atomic::Ordering::SeqCst);
                    failed.store(true, std::sync::atomic::Ordering::SeqCst);
                    close_sender(&tx_store);
                    break;
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    // The watch root can disappear without the backend
                    // reporting it — macOS FSEvents delivers no event for the
                    // removal of the watched root, so the watcher would stay
                    // silently deaf forever. Polling the root keeps the
                    // failure observable, which is the whole point of the
                    // parent-removal protocol above.
                    if !watch_target.exists() {
                        log::error!(
                            "{}",
                            tr_args(
                                "log-fs-watcher-parent-removed",
                                &[("path", watch_target.display().to_string())]
                            )
                        );
                        failed.store(true, Ordering::SeqCst);
                        running.store(false, Ordering::SeqCst);
                        close_sender(&tx_store);
                        return;
                    }
                }
            }
        }

        // Explicitly stop the debouncer
        drop(debouncer);
    }

    /// Thread body used by the thread-level regression tests: establish
    /// with failures folded into the shared flags (the pre-R-watch-008
    /// protocol, where a failed establishment was only observable through
    /// the flags) and then run the forwarding loop.
    #[cfg(test)]
    fn run_watcher(
        path: &Path,
        debounce_ms: u64,
        recv_timeout_ms: u64,
        tx_store: SenderStore,
        running: Arc<std::sync::atomic::AtomicBool>,
        failed: Arc<std::sync::atomic::AtomicBool>,
        dropped: Arc<std::sync::atomic::AtomicU64>,
    ) {
        match Self::establish_watch(path, debounce_ms) {
            Ok(watch) => {
                Self::forward_loop(watch, recv_timeout_ms, tx_store, running, failed, dropped);
            }
            Err(e) => {
                log::error!(
                    "{}",
                    tr_args(
                        "log-fs-watch-establish-failed",
                        &[
                            ("path", path.display().to_string()),
                            ("message", e.to_string()),
                        ]
                    )
                );
                failed.store(true, std::sync::atomic::Ordering::SeqCst);
                running.store(false, std::sync::atomic::Ordering::SeqCst);
                close_sender(&tx_store);
            }
        }
    }
}

/// An OS-level watch over multiple roots that is already live (see
/// [`EstablishedWatch`] for the R-watch-008 rationale).
struct MultiEstablishedWatch {
    debouncer: Debouncer<super::notify_normalize::CreateAsModifyWatcher, RecommendedCache>,
    /// Receiver of debounced backend event batches.
    bridge_rx: std::sync::mpsc::Receiver<DebounceEventResult>,
    /// Watch roots: deduplicated parents of file targets and dir targets.
    targets: HashSet<PathBuf>,
    /// Canonical event path → consumer-reported path.
    report_map: HashMap<PathBuf, PathBuf>,
}

/// Multi-file watcher that watches multiple paths.
///
/// This is useful when watching multiple configuration files.
pub struct MultiFsWatcher {
    /// Paths being watched
    watch_paths: Arc<HashSet<PathBuf>>,
    /// Receiver for debounced file events
    rx: Option<mpsc::Receiver<PathBuf>>,
    /// Shared sender store for closing the channel (see `SenderStore`)
    tx: SenderStore,
    /// Handle to the watcher thread
    watcher_thread: Option<std::thread::JoinHandle<()>>,
    /// Running flag
    running: Arc<std::sync::atomic::AtomicBool>,
    /// Set by the watcher thread when it fails mid-run (a watch root
    /// disappeared, bridge channel died). Makes the failure observable
    /// through [`MultiFsWatcher::is_running`] instead of the thread exiting
    /// silently. Establishment failures are not folded in here any more:
    /// the constructor returns `Err` for them (R-watch-008).
    failed: Arc<std::sync::atomic::AtomicBool>,
    /// Count of change events dropped because the event channel was full
    /// (slow consumer). Observable through [`MultiFsWatcher::dropped_events`].
    dropped: Arc<std::sync::atomic::AtomicU64>,
}

impl Drop for MultiFsWatcher {
    fn drop(&mut self) {
        self.stop();
    }
}

impl MultiFsWatcher {
    /// Create a new multi-file system watcher.
    ///
    /// # Arguments
    ///
    /// * `paths` - Iterator of files or directories to watch
    /// * `debounce_ms` - Debounce duration in milliseconds (default: 200ms)
    ///
    /// # Example
    ///
    /// ```rust
    /// async fn example() -> Result<(), Box<dyn std::error::Error>> {
    ///     use confers::watcher::MultiFsWatcher;
    ///
    ///     let paths = vec!["./config.toml", "./config.prod.toml"];
    ///     let mut watcher = MultiFsWatcher::new(paths, 200).await?;
    ///
    ///     while let Some(path) = watcher.recv().await {
    ///         println!("File changed: {:?}", path);
    ///     }
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn new(
        paths: impl IntoIterator<Item = impl AsRef<Path>>,
        debounce_ms: u64,
    ) -> ConfigResult<Self> {
        Self::with_recv_timeout(paths, debounce_ms, DEFAULT_RECV_TIMEOUT_MS).await
    }

    /// Create a new multi-file system watcher with custom recv timeout.
    ///
    /// # Arguments
    ///
    /// * `paths` - Iterator of files or directories to watch
    /// * `debounce_ms` - Debounce duration in milliseconds
    /// * `recv_timeout_ms` - Recv timeout for polling debouncer events (default: 50ms)
    ///
    /// Lower values mean faster response but higher CPU usage.
    /// Higher values mean slower response but lower CPU usage.
    ///
    /// # Establishment contract (R-watch-008)
    ///
    /// All roots are established synchronously before this constructor
    /// returns: after an `Ok(_)` return every accepted watch is live, so
    /// changes written at that instant are guaranteed observable.
    /// Individual paths that cannot be watched are logged and skipped; if
    /// the debouncer cannot be created or no path can be watched at all,
    /// the constructor returns `Err` instead of a deaf watcher. Skipped
    /// roots are only observable through the warning logs — there is no
    /// per-root query API. Failures *after* establishment keep the
    /// observable failure protocol: [`is_running()`](Self::is_running)
    /// reports `false` and [`recv()`](Self::recv) returns `None` instead of
    /// blocking forever.
    ///
    /// Serial establishment costs roughly a millisecond per root (backend
    /// handshake); the blocking handshake runs on tokio's blocking pool, so
    /// async workers are not stalled. There is no built-in cap on the
    /// number of roots; serial establishment cost grows linearly.
    pub async fn with_recv_timeout(
        paths: impl IntoIterator<Item = impl AsRef<Path>>,
        debounce_ms: u64,
        recv_timeout_ms: u64,
    ) -> ConfigResult<Self> {
        let watch_paths: HashSet<PathBuf> = paths
            .into_iter()
            .map(|p| p.as_ref().to_path_buf())
            .collect();

        if watch_paths.is_empty() {
            return Err(ConfigError::InvalidValue {
                key: "paths".to_string(),
                expected_type: "non-empty path list".to_string(),
                message: t_simple("error-watcher-paths-empty"),
            });
        }

        // Verify all paths exist
        for path in &watch_paths {
            if !path.exists() {
                return Err(ConfigError::FileNotFound {
                    filename: path.clone(),
                    source: None,
                });
            }
        }

        // Establish every root before this constructor returns: after this
        // call the accepted watches are live, so a change written by the
        // caller right away is observable (R-watch-008). Establishing inside
        // the spawned thread used to leave a scheduling-dependent window in
        // which an early change was silently lost. The serial backend
        // handshake (~1 ms per root) runs on the blocking pool so the async
        // worker is not stalled.
        let paths_for_establish = watch_paths.clone();
        let watch = tokio::task::spawn_blocking(move || {
            Self::establish_watch_multi(&paths_for_establish, debounce_ms)
        })
        .await
        .map_err(|e| ConfigError::WatcherError {
            // No panic payload in the message: the payload can carry
            // arbitrary variable content (CWE-209); the boolean split keeps
            // the two failure modes distinguishable without it.
            message: if e.is_panic() {
                t_simple("error-watcher-establish-task-panicked")
            } else {
                t_simple("error-watcher-establish-task-cancelled")
            },
            path: None,
            recoverable: false,
        })??;

        let (tx, rx) = mpsc::channel(100);
        let tx_store: SenderStore = Arc::new(std::sync::Mutex::new(Some(tx)));
        let running = Arc::new(std::sync::atomic::AtomicBool::new(true));
        let failed = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let dropped = Arc::new(std::sync::atomic::AtomicU64::new(0));
        let paths_arc = Arc::new(watch_paths);
        let running_clone = Arc::clone(&running);
        let failed_clone = Arc::clone(&failed);
        let dropped_clone = Arc::clone(&dropped);
        let tx_for_thread = SenderStore::clone(&tx_store);

        // Spawn the watcher in a dedicated thread (not tokio task); it only
        // forwards already-established watch events.
        let watcher_thread = std::thread::spawn(move || {
            Self::forward_loop_multi(
                watch,
                recv_timeout_ms,
                tx_for_thread,
                running_clone,
                failed_clone,
                dropped_clone,
            );
        });

        Ok(Self {
            watch_paths: paths_arc,
            rx: Some(rx),
            tx: tx_store,
            watcher_thread: Some(watcher_thread),
            running,
            failed,
            dropped,
        })
    }

    /// Receive the next file change event.
    ///
    /// Returns `Some(path)` when a file change is detected, `None` if the
    /// watcher is stopped or has failed mid-run (in which case
    /// [`is_running()`](Self::is_running) also reports `false`).
    ///
    /// The `is_running()` check is a fast path only: if the watcher thread
    /// fails while a `recv()` call is already awaiting, the thread closes
    /// the channel (see `SenderStore`) and the pending `recv()` returns
    /// `None` — it can never hang.
    pub async fn recv(&mut self) -> Option<PathBuf> {
        // A stopped or failed watcher can never produce events again;
        // return `None` instead of waiting on a channel that stays open.
        if !self.is_running() {
            return None;
        }
        if let Some(ref mut rx) = self.rx {
            rx.recv().await
        } else {
            None
        }
    }

    /// Get all paths being watched.
    pub fn watch_paths(&self) -> &HashSet<PathBuf> {
        &self.watch_paths
    }

    /// Stop the watcher.
    pub fn stop(&mut self) {
        if !self.running.load(std::sync::atomic::Ordering::SeqCst) {
            return;
        }

        self.running
            .store(false, std::sync::atomic::Ordering::SeqCst);

        // Drop the stored sender to close the channel, which will cause
        // recv() to return None
        close_sender(&self.tx);

        // Wait for the watcher thread to finish
        if let Some(handle) = self.watcher_thread.take() {
            let _ = handle.join();
        }

        self.rx.take();
    }

    /// Check if the watcher is running.
    ///
    /// Returns `false` after [`stop()`](Self::stop) and also when the
    /// watcher thread failed mid-run (a watch root disappeared, bridge
    /// channel died). Establishment failures are not part of this state:
    /// the constructor returns `Err` for them (R-watch-008).
    pub fn is_running(&self) -> bool {
        self.running.load(std::sync::atomic::Ordering::SeqCst)
            && !self.failed.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Number of change events dropped because the event channel was full
    /// (slow consumer).
    ///
    /// Every dropped event also emits a warning log, but the counter makes
    /// the loss queryable after the fact — a non-zero value means the
    /// consumer fell behind and missed file-change notifications (a reload
    /// trigger was lost and should be reconciled manually).
    pub fn dropped_events(&self) -> u64 {
        self.dropped.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Establish the OS-level watches for all roots synchronously on the
    /// calling thread (R-watch-008): after this call every accepted watch is
    /// live. Individual unwatchable roots are logged and skipped; if nothing
    /// can be watched, an error is returned instead of a watcher that would
    /// silently never deliver events.
    fn establish_watch_multi(
        paths: &HashSet<PathBuf>,
        debounce_ms: u64,
    ) -> ConfigResult<MultiEstablishedWatch> {
        use notify_debouncer_full::new_debouncer_opt;
        use notify_debouncer_full::notify::{Config, RecursiveMode};

        // Create a bridge channel for the debouncer callback
        let (bridge_tx, bridge_rx) = std::sync::mpsc::channel::<DebounceEventResult>();

        let mut debouncer = new_debouncer_opt::<
            _,
            super::notify_normalize::CreateAsModifyWatcher,
            RecommendedCache,
        >(
            Duration::from_millis(debounce_ms),
            None,
            move |result: DebounceEventResult| {
                let _ = bridge_tx.send(result);
            },
            RecommendedCache::new(),
            Config::default(),
        )
        .map_err(|e| {
            log::error!(
                "{}",
                tr_args(
                    "log-fs-watcher-debouncer-create-failed",
                    &[("message", e.to_string())]
                )
            );
            ConfigError::WatcherError {
                message: tr_args(
                    "log-fs-watcher-debouncer-create-failed",
                    &[("message", e.to_string())],
                ),
                path: None,
                recoverable: false,
            }
        })?;

        // The event filter must use the same path form the watcher reports.
        // notify canonicalizes its watch root, so event paths are canonical;
        // matching them against the raw caller paths drops every event when
        // the target traverses a symlink (e.g. macOS' TMPDIR under `/var` →
        // `/private/var`), which leaves the watcher permanently deaf. The
        // consumer, however, receives the caller's own path form back.
        let report_map: HashMap<PathBuf, PathBuf> = paths
            .iter()
            .map(|p| (canonical_watch_path(p), lexical_absolute(p)))
            .collect();

        // Watch roots are deduplicated. Watched files usually share one
        // parent directory, and re-watching the same root restarts the
        // backend stream every time — with many files in one directory that
        // restart storm is slow enough for the watch to be established only
        // after the first writes already happened, so every early event is
        // lost.
        let mut targets: HashSet<PathBuf> = HashSet::new();
        for path in paths {
            if path.is_dir() {
                targets.insert(path.clone());
            } else if path.is_file()
                && let Some(parent) = path.parent()
            {
                targets.insert(parent.to_path_buf());
            }
        }

        // Watch all roots. Individual failures are logged and skipped: the
        // watcher stays partially functional as long as at least one root
        // is being watched.
        let mut watched_any = false;
        for target in &targets {
            match debouncer.watch(target, RecursiveMode::Recursive) {
                Ok(()) => watched_any = true,
                Err(e) => {
                    log::warn!(
                        "{}",
                        tr_args(
                            "log-fs-watch-failed",
                            &[
                                ("path", target.display().to_string()),
                                ("message", e.to_string()),
                            ]
                        )
                    );
                }
            }
        }

        if !watched_any {
            // Nothing is being watched: the loop would spin without ever
            // producing an event. Fail the construction instead of handing
            // out a watcher that can never deliver (R-watch-008).
            log::error!("{}", tr("log-fs-watcher-no-path-watched"));
            return Err(ConfigError::WatcherError {
                message: t_simple("log-fs-watcher-no-path-watched"),
                path: None,
                recoverable: false,
            });
        }

        Ok(MultiEstablishedWatch {
            debouncer,
            bridge_rx,
            targets,
            report_map,
        })
    }

    /// Forward debounced events from the established watches to the consumer
    /// channel until the watcher is stopped or fails observably.
    ///
    /// `tx_store` is the shared sender store: on any failure this thread
    /// empties it and drops its own sender clone on return, closing the
    /// channel so a concurrently awaiting `recv()` returns `None` instead of
    /// hanging (see `SenderStore`).
    fn forward_loop_multi(
        watch: MultiEstablishedWatch,
        recv_timeout_ms: u64,
        tx_store: SenderStore,
        running: Arc<std::sync::atomic::AtomicBool>,
        failed: Arc<std::sync::atomic::AtomicBool>,
        dropped: Arc<std::sync::atomic::AtomicU64>,
    ) {
        use notify_debouncer_full::notify::EventKind;

        let MultiEstablishedWatch {
            debouncer,
            bridge_rx,
            targets,
            report_map,
        } = watch;

        let tx = {
            let guard = tx_store
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            guard.as_ref().map(|s| s.clone())
        };
        let Some(tx) = tx else {
            // `stop()` closed the store before the forwarding loop started.
            drop(debouncer);
            return;
        };

        let recv_timeout = Duration::from_millis(recv_timeout_ms);

        // Process events
        while running.load(std::sync::atomic::Ordering::SeqCst) {
            match bridge_rx.recv_timeout(recv_timeout) {
                Ok(result) => {
                    if let Ok(events) = result {
                        for event in events {
                            match event.kind {
                                EventKind::Create(_)
                                | EventKind::Modify(_)
                                | EventKind::Remove(_) => {
                                    // Forward events for paths in the watched set.
                                    // Do NOT call is_file() here — after a deletion
                                    // the path no longer exists and is_file() returns
                                    // false, silently dropping the Remove event.
                                    for event_path in &event.paths {
                                        if let Some(reported) = report_map.get(event_path) {
                                            match tx.try_send(reported.clone()) {
                                                Ok(_) => {
                                                    // Critical-path metric: watcher
                                                    // trigger forwarded to reload
                                                    // consumers.
                                                    crate::metrics::record_counter(
                                                        crate::metrics::names::WATCHER_EVENTS_TOTAL,
                                                        &[],
                                                    );
                                                }
                                                Err(mpsc::error::TrySendError::Full(_)) => {
                                                    // Channel full — the consumer fell
                                                    // behind. Make the loss observable:
                                                    // warn now, count for later audit
                                                    // (R-watch-002).
                                                    let total =
                                                        dropped.fetch_add(1, Ordering::SeqCst) + 1;
                                                    log::warn!(
                                                        "{}",
                                                        tr_args(
                                                            "log-fs-watcher-channel-full",
                                                            &[
                                                                (
                                                                    "path",
                                                                    event_path
                                                                        .display()
                                                                        .to_string()
                                                                ),
                                                                ("total", total.to_string()),
                                                            ]
                                                        )
                                                    );
                                                }
                                                Err(mpsc::error::TrySendError::Closed(_)) => {
                                                    running.store(
                                                        false,
                                                        std::sync::atomic::Ordering::SeqCst,
                                                    );
                                                    return;
                                                }
                                            }
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                    // Bridge died mid-run (debouncer gone): no further events
                    // can arrive. Mirror the establishment-failure protocol —
                    // leaving `running`/`failed` untouched here would keep
                    // `is_running()` true and block a pending `recv()` forever.
                    running.store(false, std::sync::atomic::Ordering::SeqCst);
                    failed.store(true, std::sync::atomic::Ordering::SeqCst);
                    close_sender(&tx_store);
                    break;
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    // The watch roots can disappear without the backend
                    // reporting it — macOS FSEvents delivers no event for the
                    // removal of a watched root, so the watcher would stay
                    // silently deaf for every path under it.
                    if let Some(vanished) = targets.iter().find(|t| !t.exists()) {
                        log::error!(
                            "{}",
                            tr_args(
                                "log-fs-watcher-parent-removed",
                                &[("path", vanished.display().to_string())]
                            )
                        );
                        failed.store(true, Ordering::SeqCst);
                        running.store(false, Ordering::SeqCst);
                        close_sender(&tx_store);
                        return;
                    }
                }
            }
        }

        // Explicitly stop the debouncer
        drop(debouncer);
    }

    /// Thread body used by the thread-level regression tests: establish
    /// with failures folded into the shared flags (the pre-R-watch-008
    /// protocol, where a failed establishment was only observable through
    /// the flags) and then run the forwarding loop.
    #[cfg(test)]
    fn run_watcher(
        paths: &HashSet<PathBuf>,
        debounce_ms: u64,
        recv_timeout_ms: u64,
        tx_store: SenderStore,
        running: Arc<std::sync::atomic::AtomicBool>,
        failed: Arc<std::sync::atomic::AtomicBool>,
        dropped: Arc<std::sync::atomic::AtomicU64>,
    ) {
        match Self::establish_watch_multi(paths, debounce_ms) {
            Ok(watch) => {
                Self::forward_loop_multi(
                    watch,
                    recv_timeout_ms,
                    tx_store,
                    running,
                    failed,
                    dropped,
                );
            }
            Err(e) => {
                log::error!(
                    "{}",
                    tr_args(
                        "log-fs-watch-multi-establish-failed",
                        &[("message", e.to_string())]
                    )
                );
                failed.store(true, std::sync::atomic::Ordering::SeqCst);
                running.store(false, std::sync::atomic::Ordering::SeqCst);
                close_sender(&tx_store);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Critical-path metric: every watcher event forwarded to consumers must
    /// increment confers_watcher_events_total on the installed backend.
    #[tokio::test]
    #[serial_test::serial]
    async fn watcher_metrics_count_forwarded_events() {
        crate::metrics::clear_metrics_backend();
        let recorder = crate::metrics::test_support::RecordingBackend::installed();

        let dir = tempfile::tempdir().expect("tempdir");
        let mut watcher = FsWatcher::with_recv_timeout(dir.path(), 30, 50)
            .await
            .expect("watchable temp dir");

        // Give the debouncer thread a beat to establish the inotify watch
        // before writing (same pattern as tests/e2e/watch_e2e.rs).
        tokio::time::sleep(std::time::Duration::from_millis(400)).await;

        let file = dir.path().join("metrics-trigger.toml");
        std::fs::write(&file, b"tick").expect("write triggers an event");

        // Every recv is bounded: a missing event fails the test instead of
        // hanging the suite (recv blocks indefinitely while running).
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        let mut received = false;
        while std::time::Instant::now() < deadline {
            match tokio::time::timeout(std::time::Duration::from_millis(500), watcher.recv()).await
            {
                Ok(Some(_)) => {
                    received = true;
                    break;
                }
                Ok(None) => break, // watcher stopped/failed
                Err(_elapsed) => continue,
            }
        }
        assert!(received, "watcher must deliver the change event");

        // The forwarding thread emits the metric right after try_send
        // succeeds; give it a beat to be observed.
        std::thread::sleep(std::time::Duration::from_millis(100));
        assert!(
            recorder.counter_count(crate::metrics::names::WATCHER_EVENTS_TOTAL) >= 1,
            "forwarded watcher events must be counted"
        );

        watcher.stop();
        crate::metrics::clear_metrics_backend();
    }

    /// Bounded event collection: drains up to `expected` events before the
    /// deadline. A missing event fails the test via `Err` instead of hanging.
    async fn recv_events(
        watcher: &mut FsWatcher,
        expected: usize,
        patience: Duration,
    ) -> Result<Vec<PathBuf>, ()> {
        let deadline = std::time::Instant::now() + patience;
        let mut events = Vec::new();
        while events.len() < expected {
            let remaining = deadline.saturating_duration_since(std::time::Instant::now());
            if remaining.is_zero() {
                return Err(());
            }
            match tokio::time::timeout(remaining, watcher.recv()).await {
                Ok(Some(path)) => events.push(path),
                Ok(None) => return Err(()), // watcher stopped/failed
                Err(_elapsed) => return Err(()),
            }
        }
        Ok(events)
    }

    /// Regression test (R-watch-001): FsWatcher on a file target must
    /// keep delivering events across consecutive `rename(tmp, path)` atomic
    /// replacements. The old implementation watched the file's own inode;
    /// notify does not re-arm on `MOVE_SELF`, so the first rename silently
    /// killed the watch and every later change was lost.
    #[tokio::test]
    async fn consecutive_atomic_replacements_keep_delivering_events() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("reload.toml");
        std::fs::write(&path, b"gen = 0").expect("initial content");

        let mut watcher = FsWatcher::with_recv_timeout(&path, 30, 50)
            .await
            .expect("watchable file");

        // Let the inotify watch on the parent directory settle before the
        // first replacement (same pattern as the metrics test above).
        tokio::time::sleep(Duration::from_millis(400)).await;

        // Two consecutive editor-style atomic replacements. Both must each
        // deliver at least one event; the second one is the regression: it
        // was permanently lost with inode watching.
        for round in 1..=2u32 {
            let tmp = dir.path().join("reload.toml.tmp");
            std::fs::write(&tmp, format!("version = {round}")).expect("write tmp");
            std::fs::rename(&tmp, &path).expect("atomic replace");

            let events = recv_events(&mut watcher, 1, Duration::from_secs(5))
                .await
                .expect("atomic replacement must deliver an event");
            assert!(
                events.iter().all(|p| p == &path),
                "forwarded paths must be the watched file, got {events:?}"
            );
        }

        watcher.stop();
    }

    /// Regression test (R-watch-001): deleting and recreating the
    /// watched file must not end the watch — changes to the new file stay
    /// observable (parent-directory watching, not inode watching).
    #[tokio::test]
    async fn delete_and_recreate_keeps_delivering_events() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("reload.toml");
        std::fs::write(&path, b"v1").expect("initial content");

        let mut watcher = FsWatcher::with_recv_timeout(&path, 30, 50)
            .await
            .expect("watchable file");
        tokio::time::sleep(Duration::from_millis(400)).await;

        // Delete + recreate, then modify the recreated file.
        std::fs::remove_file(&path).expect("delete");
        std::fs::write(&path, b"v2").expect("recreate");

        let events = recv_events(&mut watcher, 1, Duration::from_secs(5))
            .await
            .expect("delete/recreate must be observable");
        assert!(
            events.iter().all(|p| p == &path),
            "forwarded paths must be the watched file, got {events:?}"
        );

        // A later change to the recreated file must still arrive.
        std::fs::write(&path, b"v3").expect("modify recreated file");
        let events = recv_events(&mut watcher, 1, Duration::from_secs(5))
            .await
            .expect("changes to the recreated file must stay observable");
        assert!(events.iter().all(|p| p == &path));

        watcher.stop();
    }

    /// Regression test (R-watch-002): events overflowing the event
    /// channel (slow consumer) must be observable through `dropped_events()`
    /// instead of vanishing silently. A warning log is emitted on the same
    /// guarded branch; the global `log` slot is process-wide and owned by the
    /// bus suite's capture logger, so the test pins the counter as the
    /// queryable signal.
    #[tokio::test]
    #[serial_test::serial]
    async fn dropped_events_are_counted_when_consumer_is_slow() {
        let dir = tempfile::tempdir().expect("tempdir");
        // Directory target: no path filter, every file event is forwarded, so
        // the debounced batches of many file creations overflow the
        // 100-slot channel while the consumer never calls recv().
        let mut watcher = FsWatcher::with_recv_timeout(dir.path(), 30, 50)
            .await
            .expect("watchable temp dir");
        tokio::time::sleep(Duration::from_millis(400)).await;

        // Create far more files than the channel capacity: each contributes
        // at least one forwarded event entry.
        for i in 0..150u32 {
            let file = dir.path().join(format!("flood-{i}.toml"));
            std::fs::write(&file, b"x").expect("flood write");
        }

        // Give the watcher thread time to push the debounced batches into
        // the undrained channel; once full, every further event drops.
        tokio::time::sleep(Duration::from_millis(1200)).await;

        assert!(
            watcher.dropped_events() > 0,
            "slow-consumer overflow must be observable through dropped_events()"
        );
        // A full channel is backpressure, not a failure: the watcher lives on.
        assert!(watcher.is_running());
        watcher.stop();
    }

    /// R-watch-007: rapid consecutive writes to the watched file must
    /// be merged through the AdaptiveDebouncer outlet into a bounded number
    /// of forwarded events, and at least one event must be forwarded so the
    /// final written value is observable (a reload reads current content).
    #[tokio::test]
    async fn rapid_writes_are_merged_by_the_outlet_debouncer() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("burst.toml");
        std::fs::write(&path, b"v0").expect("initial content");

        // 防抖窗 500ms：慢 CI 上调度拉伸可把 10ms 写间隔拉到数十 ms，
        // 100ms 窗会让突发被切碎；500ms 窗在 3 倍拉伸下仍单窗合并。
        let mut watcher = FsWatcher::with_recv_timeout(&path, 500, 50)
            .await
            .expect("watchable file");
        tokio::time::sleep(Duration::from_millis(400)).await;

        // 20 rapid writes, 10ms apart — the whole burst spans one or two
        // debounce windows.
        for i in 1..=20u32 {
            std::fs::write(&path, format!("v{i}")).expect("burst write");
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        // Drain events for a bounded window after the burst.
        let deadline = std::time::Instant::now() + Duration::from_millis(3000);
        let mut count = 0usize;
        while std::time::Instant::now() < deadline {
            // 单次等待须大于防抖窗：合并事件在静默 debounce_ms 后才发出，
            // 迭代超时短于防抖窗会在首个事件到达前误判静默而提前退出。
            match tokio::time::timeout(Duration::from_millis(1000), watcher.recv()).await {
                Ok(Some(_)) => count += 1,
                _ => break,
            }
        }

        assert!(
            (1..=5).contains(&count),
            "20 rapid writes must merge into a handful of outlet events, got {count}"
        );
        // The final state is intact and observable via a (consumer-side) reload.
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "v20");
        watcher.stop();
    }

    /// Regression test for issue #456 (FsWatcher): a path that cannot be
    /// watched must fail establishment explicitly instead of producing a
    /// deaf watcher. Since R-watch-008 the constructor surfaces this as
    /// `Err`; the public path-not-found branch is covered by
    /// `tests/watcher/establishment.rs`, so here the establishment helper
    /// is exercised directly with an existing-but-unwatchable style target
    /// (`debouncer.watch` must reject).
    #[test]
    fn establish_watch_fails_for_unwatchable_path() {
        let error =
            FsWatcher::establish_watch(Path::new("/nonexistent/confers-watch-target.toml"), 50)
                .err()
                .expect("unwatchable path must fail establishment");
        assert!(matches!(error, ConfigError::WatcherError { .. }));
    }

    /// Regression test for issue #456 (MultiFsWatcher): when no path can be
    /// watched at all, the constructor must fail explicitly instead of
    /// handing out a watcher that can never deliver events.
    #[test]
    fn establish_watch_multi_fails_when_no_path_is_watchable() {
        let paths: HashSet<PathBuf> =
            [PathBuf::from("/nonexistent/confers-watch-target.toml")].into();
        let error = MultiFsWatcher::establish_watch_multi(&paths, 50)
            .err()
            .expect("unwatchable paths must fail establishment");
        assert!(matches!(error, ConfigError::WatcherError { .. }));
    }

    /// `is_running()` and `recv()` must reflect an internally failed watcher
    /// instead of claiming it is alive and blocking forever.
    #[tokio::test]
    async fn failed_watcher_reports_not_running_and_recv_returns_none() {
        // A dedicated temp dir, not the system temp dir: recursive watch
        // establishment fails on root-owned subdirectories (e.g. Linux CI's
        // `/tmp/systemd-private-*`), which is a property of the host, not of
        // the failure-observability contract under test.
        let dir = tempfile::TempDir::new().unwrap();
        let mut watcher = FsWatcher::with_recv_timeout(dir.path(), 50, 50)
            .await
            .expect("a fresh temp dir is watchable");
        assert!(watcher.is_running());

        // Simulate the watcher thread recording an internal failure.
        watcher
            .failed
            .store(true, std::sync::atomic::Ordering::SeqCst);

        assert!(!watcher.is_running());
        // Returns promptly with `None` instead of hanging on the channel.
        assert!(watcher.recv().await.is_none());

        // stop() stays a safe cleanup on an already failed watcher.
        watcher.stop();
        assert!(!watcher.is_running());
    }

    #[tokio::test]
    async fn t015r_parent_dir_removal_becomes_observable_failure() {
        // 回归:父目录被删(内核 watch 随 inode 消亡)不得静默失聪
        // ——watcher 必须转为 failed/关闭通道,让调用方可感知重建。
        let dir = tempfile::TempDir::new().unwrap();
        let file = dir.path().join("cfg.toml");
        std::fs::write(&file, "a = 1\n").unwrap();

        let mut watcher = FsWatcher::new(file.clone(), 1).await.expect("watch");
        std::thread::sleep(std::time::Duration::from_millis(600));

        drop(dir); // 删除父目录(tempdir 删除即整个目录树消失)
        // 可观测 = 收到事件或通道关闭，二者任一即可：FSEvents 对被删根
        // 目录可能投递其子文件(被监视文件)的 Remove 批次——转发为 Some
        // 是正确行为；也可能不投递——此时经 Timeout 分支轮询到根消失，
        // 关闭通道返回 None。静默失聪只有一个形态：3s 内既无事件、通道
        // 也永远开着。
        let outcome = tokio::time::timeout(std::time::Duration::from_secs(3), watcher.recv()).await;
        assert!(
            outcome.is_ok(),
            "parent-dir removal must terminate the watch observably (recv hung for 3s)"
        );
        // 无论先到的是事件还是 None，停机(failed)必须有界完成：Timeout
        // 分支在 recv_timeout 节奏内轮询到根消失即标记失败并关闭通道。
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        while watcher.is_running() {
            assert!(
                std::time::Instant::now() < deadline,
                "watcher must report not-running after parent removal"
            );
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
    }

    #[tokio::test]
    async fn t015r_relative_path_target_still_delivers_events() {
        // 回归:相对路径(含 "./x.toml" 与裸 "x.toml")在过滤器绝对
        // 化之前永远与事件路径不相等,watcher 静默失聪。
        // 在 cwd 内创建临时文件,构造真正的相对路径("t015r-cfg.toml")。
        let cwd = std::env::current_dir().unwrap();
        let file = cwd.join("t015r-cfg.toml");
        std::fs::write(&file, "a = 1\n").unwrap();
        let relative = std::path::PathBuf::from("t015r-cfg.toml");
        assert!(relative.is_relative(), "test precondition: relative path");

        let mut watcher = FsWatcher::new(relative.clone(), 1)
            .await
            .expect("watch relative path");

        // 文件目标的 watch 根是父目录（此处为 crate 根）且递归建立，慢盘
        // 高负载下（全量测试并行时）递归 watch 的建立可能晚于首次写入，
        // 事件因此丢失；重写幂等，事件到达即证明相对路径过滤正确。
        std::thread::sleep(std::time::Duration::from_millis(600));
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        let mut event = None;
        while event.is_none() {
            assert!(
                std::time::Instant::now() < deadline,
                "relative-path watch must deliver the change event (R3-H1)"
            );
            std::fs::write(&file, "a = 2\n").unwrap();
            match tokio::time::timeout(std::time::Duration::from_millis(500), watcher.recv()).await
            {
                Ok(Some(path)) => event = Some(path),
                Ok(None) => panic!("watcher must stay alive while delivering events"),
                Err(_elapsed) => continue,
            }
        }
        // Clean up before asserting: a failed expectation must not leak the
        // cwd-relative temp file into the repository root.
        let _ = std::fs::remove_file(&file);
        // 事件以绝对化路径上报(过滤器亦为绝对路径),与 MultiFsWatcher 一致。
        assert_eq!(event, Some(std::path::absolute(&file).unwrap()));
    }

    /// 相对路径的目标在 canonicalize 失败时走词法绝对化回退；父目录不存在
    /// 则 watch 建立失败——失败必须以 `Err` 显性化（不得静默失聪）。
    #[test]
    fn establish_watch_fails_for_relative_nonexistent_path() {
        let error = FsWatcher::establish_watch(
            Path::new("confers-nonexistent-sub/confers-target.toml"),
            50,
        )
        .err()
        .expect("relative nonexistent path must fail establishment");
        assert!(matches!(error, ConfigError::WatcherError { .. }));
    }

    /// 消费者退场（唯一接收端被丢弃）后事件投递必然 `Closed`：watcher
    /// 循环必须就此停机，而不是带着断开的通道继续空转。
    fn assert_thread_stops_when_receiver_dropped(
        spawn_watcher: impl FnOnce(
            SenderStore,
            Arc<std::sync::atomic::AtomicBool>,
        ) -> std::thread::JoinHandle<()>,
        trigger: impl Fn(),
    ) {
        let (tx, rx) = mpsc::channel::<PathBuf>(100);
        let tx_store: SenderStore = Arc::new(std::sync::Mutex::new(Some(tx)));
        let running = Arc::new(std::sync::atomic::AtomicBool::new(true));
        drop(rx);

        let handle = spawn_watcher(Arc::clone(&tx_store), Arc::clone(&running));

        std::thread::sleep(Duration::from_millis(400));
        trigger();

        // 事件送达 try_send 即得 Closed，循环随即退出；用有界轮询避免
        // 极端调度下 join 无限阻塞。
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while running.load(std::sync::atomic::Ordering::SeqCst) {
            assert!(
                std::time::Instant::now() < deadline,
                "watcher thread must observe the closed channel and stop"
            );
            std::thread::sleep(Duration::from_millis(50));
            trigger();
        }
        handle.join().expect("watcher thread exits on Closed");
    }

    /// FsWatcher：接收端被丢弃后线程经 `Closed` 分支停机。
    #[test]
    fn fs_watcher_thread_stops_when_receiver_dropped() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("closed.toml");
        std::fs::write(&path, b"v0").expect("initial content");

        let watch_path = path.clone();
        assert_thread_stops_when_receiver_dropped(
            move |tx_store, running| {
                let failed = Arc::new(std::sync::atomic::AtomicBool::new(false));
                std::thread::spawn(move || {
                    FsWatcher::run_watcher(
                        &watch_path,
                        30,
                        50,
                        tx_store,
                        running,
                        failed,
                        Arc::new(std::sync::atomic::AtomicU64::new(0)),
                    );
                })
            },
            || std::fs::write(&path, b"v1").expect("trigger event"),
        );
    }

    /// MultiFsWatcher：同上，接收端被丢弃后线程经 `Closed` 分支停机。
    #[test]
    fn multi_fs_watcher_thread_stops_when_receiver_dropped() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("closed.toml");
        std::fs::write(&path, b"v0").expect("initial content");

        let watch_path = path.clone();
        assert_thread_stops_when_receiver_dropped(
            move |tx_store, running| {
                let failed = Arc::new(std::sync::atomic::AtomicBool::new(false));
                std::thread::spawn(move || {
                    let paths: HashSet<PathBuf> = [watch_path].into();
                    MultiFsWatcher::run_watcher(
                        &paths,
                        30,
                        50,
                        tx_store,
                        running,
                        failed,
                        Arc::new(std::sync::atomic::AtomicU64::new(0)),
                    );
                })
            },
            || std::fs::write(&path, b"v1").expect("trigger event"),
        );
    }

    /// MultiFsWatcher：stop() 之后 recv() 立即返回 None，不阻塞；丢事件
    /// 计数器可查询。
    #[tokio::test]
    async fn multi_fs_watcher_recv_after_stop_returns_none() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut watcher = MultiFsWatcher::new([dir.path()], 50)
            .await
            .expect("watchable temp dir");
        assert!(watcher.is_running());
        assert_eq!(watcher.dropped_events(), 0);

        watcher.stop();
        assert!(
            watcher.recv().await.is_none(),
            "recv after stop must return None instead of waiting"
        );
        assert!(watcher.recv().await.is_none());
    }

    /// MultiFsWatcher（R-watch-002）：慢消费者把事件通道挤满后，被丢弃的
    /// 事件必须可通过 dropped_events() 观测，且 watcher 依然存活。
    #[tokio::test]
    async fn multi_fs_watcher_counts_dropped_events_on_slow_consumer() {
        let dir = tempfile::tempdir().expect("tempdir");
        // MultiFsWatcher 只转发精确命中监视集合的路径：把 200 个文件全部
        // 纳入集合，洪泛写入即可超过 100 槽的通道容量。
        let paths: Vec<PathBuf> = (0..200u32)
            .map(|i| {
                let file = dir.path().join(format!("flood-{i}.toml"));
                std::fs::write(&file, b"x").expect("seed file");
                file
            })
            .collect();

        let mut watcher = MultiFsWatcher::new(paths, 30)
            .await
            .expect("watchable temp dir");
        tokio::time::sleep(Duration::from_millis(400)).await;

        for i in 0..200u32 {
            let file = dir.path().join(format!("flood-{i}.toml"));
            std::fs::write(&file, b"y").expect("flood write");
        }

        // 给 watcher 线程时间把去抖后的事件推进未消费的通道；通道填满后
        // 每个后续事件都被丢弃并计数。
        tokio::time::sleep(Duration::from_millis(1200)).await;

        assert!(
            watcher.dropped_events() > 0,
            "slow-consumer overflow must be observable through dropped_events()"
        );
        // 通道满只是背压，不是故障：watcher 仍在运行。
        assert!(watcher.is_running());
        watcher.stop();
    }
}
