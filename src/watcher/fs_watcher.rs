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

/// File system watcher with debouncing.
///
/// This watcher monitors file changes and emits debounced events
/// to avoid triggering multiple reloads for a single file modification.
#[allow(dead_code)]
pub struct FsWatcher {
    /// Path being watched
    watch_path: Arc<PathBuf>,
    /// Receiver for debounced file events
    rx: Option<mpsc::Receiver<PathBuf>>,
    /// Sender for closing the channel
    tx: Option<mpsc::Sender<PathBuf>>,
    /// Handle to the watcher thread
    watcher_thread: Option<std::thread::JoinHandle<()>>,
    /// Running flag
    running: Arc<std::sync::atomic::AtomicBool>,
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
    ///     use confers::watcher::fs_watcher::FsWatcher;
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
        let watch_path = Arc::new(path.as_ref().to_path_buf());

        // Verify the path exists
        if !watch_path.exists() {
            return Err(ConfigError::FileNotFound {
                filename: watch_path.as_ref().clone(),
                source: None,
            });
        }

        let (tx, rx) = mpsc::channel(100);
        let running = Arc::new(std::sync::atomic::AtomicBool::new(true));
        let path_clone = Arc::clone(&watch_path);
        let running_clone = Arc::clone(&running);
        let tx_for_thread = tx.clone();

        // Spawn the watcher in a dedicated thread (not tokio task)
        let watcher_thread = std::thread::spawn(move || {
            Self::run_watcher(&path_clone, debounce_ms, tx_for_thread, running_clone);
        });

        Ok(Self {
            watch_path,
            rx: Some(rx),
            tx: Some(tx),
            watcher_thread: Some(watcher_thread),
            running,
        })
    }

    /// Receive the next file change event.
    ///
    /// Returns `Some(path)` when a file change is detected, `None` if the watcher is stopped.
    pub async fn recv(&mut self) -> Option<PathBuf> {
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

        // Drop the sender to close the channel, which will cause recv() to return None
        self.tx.take();

        // Wait for the watcher thread to finish
        if let Some(handle) = self.watcher_thread.take() {
            let _ = handle.join();
        }

        // Close the receiver
        self.rx.take();
    }

    /// Check if the watcher is running.
    pub fn is_running(&self) -> bool {
        self.running.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Internal watcher function that runs in a dedicated thread.
    fn run_watcher(
        path: &Path,
        debounce_ms: u64,
        tx: mpsc::Sender<PathBuf>,
        running: Arc<std::sync::atomic::AtomicBool>,
    ) {
        use notify_debouncer_full::{
            new_debouncer, notify::EventKind, notify::RecursiveMode, DebounceEventResult,
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
                    tracing::error!("Failed to create debouncer: {:?}", e);
                    return;
                }
            };

        // Start watching
        if let Err(e) = debouncer.watch(path, RecursiveMode::Recursive) {
            tracing::error!("Failed to watch path {:?}: {:?}", path, e);
            return;
        }

        tracing::info!("FsWatcher watching: {:?}", path);

        // Process events
        while running.load(std::sync::atomic::Ordering::SeqCst) {
            match bridge_rx.recv_timeout(Duration::from_millis(50)) {
                Ok(result) => {
                    if let Ok(events) = result {
                        for event in events {
                            match event.kind {
                                EventKind::Create(_)
                                | EventKind::Modify(_)
                                | EventKind::Remove(_) => {
                                    for event_path in &event.paths {
                                        if event_path.is_file() {
                                            // Try to send, but don't block if channel is closed
                                            match tx.try_send(event_path.clone()) {
                                                Ok(_) => {}
                                                Err(mpsc::error::TrySendError::Full(_)) => {
                                                    // Channel full, skip this event
                                                }
                                                Err(mpsc::error::TrySendError::Closed(_)) => {
                                                    // Channel closed, exit
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
                    tracing::debug!("Bridge channel disconnected");
                    break;
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    // Continue
                }
            }
        }

        // Explicitly stop the debouncer
        drop(debouncer);
        tracing::debug!("FsWatcher stopped");
    }
}

/// Multi-file watcher that watches multiple paths.
///
/// This is useful when watching multiple configuration files.
#[allow(dead_code)]
pub struct MultiFsWatcher {
    /// Paths being watched
    watch_paths: Arc<HashSet<PathBuf>>,
    /// Receiver for debounced file events
    rx: Option<mpsc::Receiver<PathBuf>>,
    /// Sender for closing the channel
    tx: Option<mpsc::Sender<PathBuf>>,
    /// Handle to the watcher thread
    watcher_thread: Option<std::thread::JoinHandle<()>>,
    /// Running flag
    running: Arc<std::sync::atomic::AtomicBool>,
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
    ///     use confers::watcher::fs_watcher::MultiFsWatcher;
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
        let running = Arc::new(std::sync::atomic::AtomicBool::new(true));
        let paths_arc = Arc::new(watch_paths);
        let running_clone = Arc::clone(&running);
        let paths_for_thread = Arc::clone(&paths_arc);
        let tx_for_thread = tx.clone();

        // Spawn the watcher in a dedicated thread (not tokio task)
        let watcher_thread = std::thread::spawn(move || {
            Self::run_watcher(&paths_for_thread, debounce_ms, tx_for_thread, running_clone);
        });

        Ok(Self {
            watch_paths: paths_arc,
            rx: Some(rx),
            tx: Some(tx),
            watcher_thread: Some(watcher_thread),
            running,
        })
    }

    /// Receive the next file change event.
    ///
    /// Returns `Some(path)` when a file change is detected, `None` if the watcher is stopped.
    pub async fn recv(&mut self) -> Option<PathBuf> {
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

        // Drop the sender to close the channel, which will cause recv() to return None
        self.tx.take();

        // Wait for the watcher thread to finish
        if let Some(handle) = self.watcher_thread.take() {
            let _ = handle.join();
        }

        // Close the receiver
        self.rx.take();
    }

    /// Check if the watcher is running.
    pub fn is_running(&self) -> bool {
        self.running.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Internal watcher function that runs in a dedicated thread.
    fn run_watcher(
        paths: &HashSet<PathBuf>,
        debounce_ms: u64,
        tx: mpsc::Sender<PathBuf>,
        running: Arc<std::sync::atomic::AtomicBool>,
    ) {
        use notify_debouncer_full::{
            new_debouncer, notify::EventKind, notify::RecursiveMode, DebounceEventResult,
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
                    tracing::error!("Failed to create debouncer: {:?}", e);
                    return;
                }
            };

        // Watch all paths
        for path in paths {
            if path.is_dir() {
                let _ = debouncer.watch(path.as_path(), RecursiveMode::Recursive);
            } else if path.is_file() {
                if let Some(parent) = path.parent() {
                    let _ = debouncer.watch(parent, RecursiveMode::Recursive);
                }
            }
        }

        tracing::info!("MultiFsWatcher watching {} paths", paths.len());

        // Process events
        while running.load(std::sync::atomic::Ordering::SeqCst) {
            match bridge_rx.recv_timeout(Duration::from_millis(50)) {
                Ok(result) => {
                    if let Ok(events) = result {
                        for event in events {
                            match event.kind {
                                EventKind::Create(_)
                                | EventKind::Modify(_)
                                | EventKind::Remove(_) => {
                                    for event_path in &event.paths {
                                        if event_path.is_file() && paths.contains(event_path) {
                                            // Try to send, but don't block if channel is closed
                                            match tx.try_send(event_path.clone()) {
                                                Ok(_) => {}
                                                Err(mpsc::error::TrySendError::Full(_)) => {
                                                    // Channel full, skip this event
                                                }
                                                Err(mpsc::error::TrySendError::Closed(_)) => {
                                                    // Channel closed, exit
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
                    tracing::debug!("Bridge channel disconnected");
                    break;
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    // Continue
                }
            }
        }

        // Explicitly stop the debouncer
        drop(debouncer);
        tracing::debug!("MultiFsWatcher stopped");
    }
}
