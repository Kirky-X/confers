// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! Platform-level file debouncer using notify-debouncer-full.
//!
//! This module provides file system watching with platform-level debouncing,
//! wrapping the notify-debouncer-full crate for integration with confers.

#[cfg(feature = "watch")]
use crate::error::{ConfigError, ConfigResult};
#[cfg(feature = "watch")]
use crate::i18n::tr_args;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;
use tokio::sync::mpsc;

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
    /// Set by the watcher thread when it cannot establish the watch
    /// (debouncer creation or `watch` failure). Makes the failure
    /// observable through [`FsWatcher::is_running`] instead of the thread
    /// exiting silently.
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
    /// # Watch-thread failures
    ///
    /// The path is verified to exist before the watcher thread starts, but
    /// the OS may still refuse to establish the watch (e.g. permission
    /// changes or inotify watch limits). In that case the thread logs the
    /// error, records the failure and exits:
    /// [`is_running()`](Self::is_running) then reports `false` and
    /// [`recv()`](Self::recv) returns `None` instead of blocking forever.
    pub async fn with_recv_timeout(
        path: impl AsRef<Path>,
        debounce_ms: u64,
        recv_timeout_ms: u64,
    ) -> ConfigResult<Self> {
        let watch_path = Arc::new(path.as_ref().to_path_buf());

        // Verify the path exists
        if !watch_path.exists() {
            return Err(ConfigError::FileNotFound {
                filename: watch_path.as_ref().clone(),
                source: None,
            });
        }

        let (tx, rx) = mpsc::channel(100);
        let tx_store: SenderStore = Arc::new(std::sync::Mutex::new(Some(tx)));
        let running = Arc::new(std::sync::atomic::AtomicBool::new(true));
        let failed = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let dropped = Arc::new(std::sync::atomic::AtomicU64::new(0));
        let path_clone = Arc::clone(&watch_path);
        let running_clone = Arc::clone(&running);
        let failed_clone = Arc::clone(&failed);
        let dropped_clone = Arc::clone(&dropped);
        let tx_for_thread = SenderStore::clone(&tx_store);

        // Spawn the watcher in a dedicated thread (not tokio task)
        let watcher_thread = std::thread::spawn(move || {
            Self::run_watcher(
                &path_clone,
                debounce_ms,
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
    /// watcher is stopped or has failed to establish the watch (in which
    /// case [`is_running()`](Self::is_running) also reports `false`).
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

        // Close the receiver
        self.rx.take();
    }

    /// Check if the watcher is running.
    ///
    /// Returns `false` after [`stop()`](Self::stop) and also when the
    /// watcher thread exited early because the debouncer could not be
    /// created or the path could not be watched (see
    /// [`with_recv_timeout`](Self::with_recv_timeout)).
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

    /// Internal watcher function that runs in a dedicated thread.
    ///
    /// `tx_store` is the shared sender store: on any failure this thread
    /// empties it and drops its own sender clone on return, closing the
    /// channel so a concurrently awaiting `recv()` returns `None` instead of
    /// hanging (see `SenderStore`).
    fn run_watcher(
        path: &Path,
        debounce_ms: u64,
        recv_timeout_ms: u64,
        tx_store: SenderStore,
        running: Arc<std::sync::atomic::AtomicBool>,
        failed: Arc<std::sync::atomic::AtomicBool>,
        dropped: Arc<std::sync::atomic::AtomicU64>,
    ) {
        use notify_debouncer_full::{
            DebounceEventResult, new_debouncer, notify::EventKind, notify::RecursiveMode,
        };

        // Watch strategy: a file target is watched through its PARENT
        // directory with an exact-path filter, mirroring the proven
        // `MultiFsWatcher` rename protection. Watching the file's own inode
        // loses the watch on `rename(tmp, path)` atomic replacement (notify
        // does not re-arm on `MOVE_SELF`), so every change after the first
        // editor-style save would be silently missed. Watching the parent
        // directory survives both replacement and delete+recreate of the
        // watched path. Directory targets are watched directly and need no
        // filter (a directory inode is not swapped by renames of children).
        // R3-H1: both the watch root and the filter must be ABSOLUTE.
        // notify-debouncer-full canonicalizes its watch root and joins event
        // paths onto it, while `path_filter` would otherwise hold the
        // caller's raw (possibly relative) path — the two never compare
        // equal, every event gets filtered out, and the watcher goes
        // silently deaf even though `is_running()` stays true (the rustdoc
        // example's `FsWatcher::new("./config.toml", ..)` used to hit
        // exactly this).
        let absolutize = |p: &Path| -> PathBuf {
            std::fs::canonicalize(p).unwrap_or_else(|_| {
                // Target may not exist yet (delete+recreate flows): fall
                // back to a lexical absolutization against the current dir.
                if p.is_absolute() {
                    p.to_path_buf()
                } else {
                    std::env::current_dir().unwrap_or_default().join(p)
                }
            })
        };
        let absolute = absolutize(path);
        let (watch_target, path_filter): (PathBuf, Option<PathBuf>) = if absolute.is_dir() {
            (absolute, None)
        } else {
            let parent = absolute
                .parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| absolute.clone());
            (parent, Some(absolute))
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

        // Create the debouncer
        let mut debouncer =
            match new_debouncer(Duration::from_millis(debounce_ms), None, move |result| {
                let _ = bridge_tx.send(result);
            }) {
                Ok(d) => d,
                Err(e) => {
                    // Make the failure observable instead of exiting
                    // silently: `is_running()` reports false and closing the
                    // sender store lets a pending `recv()` return `None`.
                    log::error!(
                        "failed to create file watcher debouncer for {}: {e}",
                        path.display()
                    );
                    failed.store(true, std::sync::atomic::Ordering::SeqCst);
                    running.store(false, std::sync::atomic::Ordering::SeqCst);
                    close_sender(&tx_store);
                    return;
                }
            };

        // Start watching
        if let Err(e) = debouncer.watch(&watch_target, RecursiveMode::Recursive) {
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
            failed.store(true, std::sync::atomic::Ordering::SeqCst);
            running.store(false, std::sync::atomic::Ordering::SeqCst);
            close_sender(&tx_store);
            return;
        }

        let tx = {
            let guard = tx_store
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            guard.as_ref().map(|s| s.clone())
        };
        let Some(tx) = tx else {
            // `stop()` closed the store before the watch was established.
            drop(debouncer);
            return;
        };
        // The thread keeps its `tx_store` reference for the whole run: a
        // mid-run bridge failure must still be able to close the outgoing
        // channel (same as `MultiFsWatcher::run_watcher`).

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
                                                match tx.try_send(event_path.clone()) {
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
                    // Continue
                }
            }
        }

        // Explicitly stop the debouncer
        drop(debouncer);
    }
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
    /// Set by the watcher thread when it cannot establish any watch
    /// (debouncer creation failure, or every path failed to watch). Makes
    /// the failure observable through [`MultiFsWatcher::is_running`] instead
    /// of the thread exiting silently.
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
    /// # Watch-thread failures
    ///
    /// Individual paths that cannot be watched are logged and skipped; the
    /// watcher keeps running for the remaining paths. If the debouncer
    /// cannot be created or no path can be watched at all, the thread
    /// records the failure and exits: [`is_running()`](Self::is_running)
    /// then reports `false` and [`recv()`](Self::recv) returns `None`
    /// instead of blocking forever.
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
                message: "At least one path must be provided".to_string(),
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

        let (tx, rx) = mpsc::channel(100);
        let tx_store: SenderStore = Arc::new(std::sync::Mutex::new(Some(tx)));
        let running = Arc::new(std::sync::atomic::AtomicBool::new(true));
        let failed = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let dropped = Arc::new(std::sync::atomic::AtomicU64::new(0));
        let paths_arc = Arc::new(watch_paths);
        let running_clone = Arc::clone(&running);
        let failed_clone = Arc::clone(&failed);
        let dropped_clone = Arc::clone(&dropped);
        let paths_for_thread = Arc::clone(&paths_arc);
        let tx_for_thread = SenderStore::clone(&tx_store);

        // Spawn the watcher in a dedicated thread (not tokio task)
        let watcher_thread = std::thread::spawn(move || {
            Self::run_watcher(
                &paths_for_thread,
                debounce_ms,
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
    /// watcher is stopped or has failed to establish any watch (in which
    /// case [`is_running()`](Self::is_running) also reports `false`).
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

        // Close the receiver
        self.rx.take();
    }

    /// Check if the watcher is running.
    ///
    /// Returns `false` after [`stop()`](Self::stop) and also when the
    /// watcher thread exited early because the debouncer could not be
    /// created or no path could be watched (see
    /// [`with_recv_timeout`](Self::with_recv_timeout)).
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

    /// Internal watcher function that runs in a dedicated thread.
    ///
    /// `tx_store` is the shared sender store: on any failure this thread
    /// empties it and drops its own sender clone on return, closing the
    /// channel so a concurrently awaiting `recv()` returns `None` instead of
    /// hanging (see `SenderStore`).
    fn run_watcher(
        paths: &HashSet<PathBuf>,
        debounce_ms: u64,
        recv_timeout_ms: u64,
        tx_store: SenderStore,
        running: Arc<std::sync::atomic::AtomicBool>,
        failed: Arc<std::sync::atomic::AtomicBool>,
        dropped: Arc<std::sync::atomic::AtomicU64>,
    ) {
        use notify_debouncer_full::{
            DebounceEventResult, new_debouncer, notify::EventKind, notify::RecursiveMode,
        };

        // Create a bridge channel for the debouncer callback
        let (bridge_tx, bridge_rx) = std::sync::mpsc::channel::<DebounceEventResult>();

        // Create the debouncer
        let mut debouncer =
            match new_debouncer(Duration::from_millis(debounce_ms), None, move |result| {
                let _ = bridge_tx.send(result);
            }) {
                Ok(d) => d,
                Err(e) => {
                    // Make the failure observable instead of exiting
                    // silently: `is_running()` reports false and closing the
                    // sender store lets a pending `recv()` return `None`.
                    log::error!("failed to create file watcher debouncer: {e}");
                    failed.store(true, std::sync::atomic::Ordering::SeqCst);
                    running.store(false, std::sync::atomic::Ordering::SeqCst);
                    close_sender(&tx_store);
                    return;
                }
            };

        // Watch all paths. Individual failures are logged and skipped: the
        // watcher stays partially functional as long as at least one path
        // is being watched.
        let mut watched_any = false;
        for path in paths {
            let target = if path.is_dir() {
                Some(path.as_path())
            } else if path.is_file() {
                path.parent()
            } else {
                None
            };
            if let Some(target) = target {
                match debouncer.watch(target, RecursiveMode::Recursive) {
                    Ok(()) => watched_any = true,
                    Err(e) => {
                        log::warn!("failed to watch {}: {e}", target.display());
                    }
                }
            }
        }

        if !watched_any {
            // Nothing is being watched: the thread would spin without ever
            // producing an event. Record the failure so `is_running()`
            // reports false, and close the channel so a pending `recv()`
            // returns `None`.
            log::error!("failed to watch any of the requested paths");
            failed.store(true, std::sync::atomic::Ordering::SeqCst);
            running.store(false, std::sync::atomic::Ordering::SeqCst);
            close_sender(&tx_store);
            return;
        }

        let tx = {
            let guard = tx_store
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            guard.as_ref().map(|s| s.clone())
        };
        let Some(tx) = tx else {
            // `stop()` closed the store before the watch was established.
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
                                        if paths.contains(event_path) {
                                            match tx.try_send(event_path.clone()) {
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
                    // Continue
                }
            }
        }

        // Explicitly stop the debouncer
        drop(debouncer);
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
            match tokio::time::timeout(Duration::from_millis(250), watcher.recv()).await {
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

    fn assert_thread_reported_failure(
        running: &std::sync::atomic::AtomicBool,
        failed: &std::sync::atomic::AtomicBool,
        rx: &mut mpsc::Receiver<PathBuf>,
    ) {
        assert!(
            failed.load(std::sync::atomic::Ordering::SeqCst),
            "watcher thread must record the failure instead of exiting silently"
        );
        assert!(
            !running.load(std::sync::atomic::Ordering::SeqCst),
            "watcher thread must report itself as stopped after a failure"
        );
        // The thread dropped its sender on the failure path, so the channel
        // is disconnected and `recv()` can never hang on it.
        assert!(matches!(
            rx.try_recv(),
            Err(mpsc::error::TryRecvError::Disconnected)
        ));
    }

    /// Regression test for issue #456 (FsWatcher): a watcher thread that
    /// cannot establish the watch must surface the failure through the
    /// shared flags instead of exiting silently. The public constructors
    /// reject non-existent paths up front, so the thread body is exercised
    /// directly with a path `debouncer.watch` must reject.
    #[test]
    fn fs_watcher_thread_marks_failed_when_watch_cannot_be_established() {
        let running = Arc::new(std::sync::atomic::AtomicBool::new(true));
        let failed = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let (tx, mut rx) = mpsc::channel::<PathBuf>(1);
        let tx_store: SenderStore = Arc::new(std::sync::Mutex::new(Some(tx)));

        let handle = {
            let running = Arc::clone(&running);
            let failed = Arc::clone(&failed);
            std::thread::spawn(move || {
                FsWatcher::run_watcher(
                    Path::new("/nonexistent/confers-watch-target.toml"),
                    50,
                    50,
                    tx_store,
                    running,
                    failed,
                    Arc::new(std::sync::atomic::AtomicU64::new(0)),
                );
            })
        };
        handle.join().unwrap();

        assert_thread_reported_failure(&running, &failed, &mut rx);
    }

    /// Regression test for issue #456 (MultiFsWatcher): when no path can be
    /// watched at all, the failure must be observable through the shared
    /// flags rather than the thread spinning without ever producing events.
    #[test]
    fn multi_fs_watcher_thread_marks_failed_when_no_path_is_watchable() {
        let running = Arc::new(std::sync::atomic::AtomicBool::new(true));
        let failed = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let (tx, mut rx) = mpsc::channel::<PathBuf>(1);
        let tx_store: SenderStore = Arc::new(std::sync::Mutex::new(Some(tx)));
        let paths: HashSet<PathBuf> =
            [PathBuf::from("/nonexistent/confers-watch-target.toml")].into();

        let handle = {
            let running = Arc::clone(&running);
            let failed = Arc::clone(&failed);
            std::thread::spawn(move || {
                MultiFsWatcher::run_watcher(
                    &paths,
                    50,
                    50,
                    tx_store,
                    running,
                    failed,
                    Arc::new(std::sync::atomic::AtomicU64::new(0)),
                );
            })
        };
        handle.join().unwrap();

        assert_thread_reported_failure(&running, &failed, &mut rx);
    }

    /// `is_running()` and `recv()` must reflect an internally failed watcher
    /// instead of claiming it is alive and blocking forever.
    #[tokio::test]
    async fn failed_watcher_reports_not_running_and_recv_returns_none() {
        let mut watcher = FsWatcher::with_recv_timeout(std::env::temp_dir(), 50, 50)
            .await
            .expect("the system temp dir exists and is watchable");
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
        let outcome = tokio::time::timeout(std::time::Duration::from_secs(3), watcher.recv()).await;
        // 通道被关闭(Err/None)或事件流已终结 —— 绝不能是"仍在运行但
        // 永远没有事件"的静默失聪状态。
        assert!(
            outcome.is_err() || outcome.unwrap().is_none(),
            "parent-dir removal must terminate the watch observably"
        );
        assert!(
            !watcher.is_running(),
            "watcher must report not-running after parent removal"
        );
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

        std::thread::sleep(std::time::Duration::from_millis(600));
        std::fs::write(&file, "a = 2\n").unwrap();
        let event = tokio::time::timeout(std::time::Duration::from_secs(3), watcher.recv()).await;
        // Clean up before asserting: a failed expectation must not leak the
        // cwd-relative temp file into the repository root.
        let _ = std::fs::remove_file(&file);
        let event = event.expect("relative-path watch must deliver the change event (R3-H1)");
        // 事件以绝对化路径上报(过滤器亦为绝对路径),与 MultiFsWatcher 一致。
        assert_eq!(event, Some(std::path::absolute(&file).unwrap()));
    }
}
