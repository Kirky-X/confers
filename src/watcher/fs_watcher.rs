// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Platform-level file debouncer using notify-debouncer-full.
//!
//! This module provides file system watching with platform-level debouncing,
//! wrapping the notify-debouncer-full crate for integration with confers.

#[cfg(feature = "watch")]
use crate::error::{ConfigError, ConfigResult};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
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
    /// Shared sender store for closing the channel (see [`SenderStore`])
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
        let path_clone = Arc::clone(&watch_path);
        let running_clone = Arc::clone(&running);
        let failed_clone = Arc::clone(&failed);
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
            );
        });

        Ok(Self {
            watch_path,
            rx: Some(rx),
            tx: tx_store,
            watcher_thread: Some(watcher_thread),
            running,
            failed,
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
    /// the channel (see [`SenderStore`]) and the pending `recv()` returns
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

    /// Internal watcher function that runs in a dedicated thread.
    ///
    /// `tx_store` is the shared sender store: on any failure this thread
    /// empties it and drops its own sender clone on return, closing the
    /// channel so a concurrently awaiting `recv()` returns `None` instead of
    /// hanging (see [`SenderStore`]).
    fn run_watcher(
        path: &Path,
        debounce_ms: u64,
        recv_timeout_ms: u64,
        tx_store: SenderStore,
        running: Arc<std::sync::atomic::AtomicBool>,
        failed: Arc<std::sync::atomic::AtomicBool>,
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
        if let Err(e) = debouncer.watch(path, RecursiveMode::Recursive) {
            log::error!("failed to watch {}: {e}", path.display());
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
        // Drop this extra handle: the store and the closure above hold the
        // channel open; `close_sender` on failure only needs the store.
        drop(tx_store);

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
                                    // Forward all file-system events for paths
                                    // within the watched directory. The is_file()
                                    // check was removed because it drops deletion
                                    // events (the path no longer exists) and can
                                    // race with creation events on some platforms.
                                    for event_path in &event.paths {
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
                                                // Channel full — event silently dropped.
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
                                _ => {}
                            }
                        }
                    }
                }
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                    // Bridge channel disconnected - exit gracefully
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
    /// Shared sender store for closing the channel (see [`SenderStore`])
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
        let paths_arc = Arc::new(watch_paths);
        let running_clone = Arc::clone(&running);
        let failed_clone = Arc::clone(&failed);
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
            );
        });

        Ok(Self {
            watch_paths: paths_arc,
            rx: Some(rx),
            tx: tx_store,
            watcher_thread: Some(watcher_thread),
            running,
            failed,
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
    /// the channel (see [`SenderStore`]) and the pending `recv()` returns
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

    /// Internal watcher function that runs in a dedicated thread.
    ///
    /// `tx_store` is the shared sender store: on any failure this thread
    /// empties it and drops its own sender clone on return, closing the
    /// channel so a concurrently awaiting `recv()` returns `None` instead of
    /// hanging (see [`SenderStore`]).
    fn run_watcher(
        paths: &HashSet<PathBuf>,
        debounce_ms: u64,
        recv_timeout_ms: u64,
        tx_store: SenderStore,
        running: Arc<std::sync::atomic::AtomicBool>,
        failed: Arc<std::sync::atomic::AtomicBool>,
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
                                                    // Channel full — event silently dropped.
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
                    // Bridge channel disconnected - exit gracefully
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
                MultiFsWatcher::run_watcher(&paths, 50, 50, tx_store, running, failed);
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
}
