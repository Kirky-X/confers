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
    rx: mpsc::Receiver<PathBuf>,
    /// Handle to the watcher task
    task_handle: tokio::task::JoinHandle<()>,
    /// Running flag
    running: Arc<std::sync::atomic::AtomicBool>,
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

        // Spawn watcher task
        let task_handle = tokio::spawn(async move {
            Self::watch_task(&path_clone, debounce_ms, tx, running_clone).await;
        });

        Ok(Self {
            watch_path,
            rx,
            task_handle,
            running,
        })
    }

    /// Receive the next file change event.
    ///
    /// Returns `Some(path)` when a file change is detected, `None` if the watcher is stopped.
    pub async fn recv(&mut self) -> Option<PathBuf> {
        self.rx.recv().await
    }

    /// Get the path being watched.
    pub fn watch_path(&self) -> &Path {
        &self.watch_path
    }

    /// Stop the watcher.
    pub fn stop(&self) {
        self.running
            .store(false, std::sync::atomic::Ordering::SeqCst);
    }

    /// Check if the watcher is running.
    pub fn is_running(&self) -> bool {
        self.running.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Internal watcher task that integrates with notify-debouncer-full.
    async fn watch_task(
        path: &Path,
        debounce_ms: u64,
        tx: mpsc::Sender<PathBuf>,
        running: Arc<std::sync::atomic::AtomicBool>,
    ) {
        use notify_debouncer_full::{
            new_debouncer, notify::EventKind, notify::RecursiveMode, DebounceEventResult,
        };

        let path_owned = path.to_path_buf();
        let running_sync = Arc::clone(&running);

        // Create a bridge channel
        let (bridge_tx, bridge_rx) = std::sync::mpsc::channel::<DebounceEventResult>();

        // Spawn the debouncer in a blocking thread
        std::thread::spawn(move || {
            match new_debouncer(Duration::from_millis(debounce_ms), None, move |result| {
                let _ = bridge_tx.send(result);
            }) {
                Ok(mut d) => {
                    if let Err(e) = d.watch(&path_owned, RecursiveMode::Recursive) {
                        tracing::error!("Failed to watch path {:?}: {:?}", path_owned, e);
                        return;
                    }
                    tracing::info!("FsWatcher watching: {:?}", path_owned);

                    // Keep the thread alive to process events
                    while running_sync.load(std::sync::atomic::Ordering::SeqCst) {
                        std::thread::sleep(Duration::from_millis(50));
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to create debouncer: {:?}", e);
                }
            }
        });

        // Process events in async context
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
                                            let _ = tx.send(event_path.clone()).await;
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                    tracing::warn!("Bridge channel disconnected");
                    break;
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    // Continue
                }
            }
        }
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
    rx: mpsc::Receiver<PathBuf>,
    /// Handle to the watcher task
    task_handle: tokio::task::JoinHandle<()>,
    /// Running flag
    running: Arc<std::sync::atomic::AtomicBool>,
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
        let paths_clone = Arc::clone(&paths_arc);

        // Spawn watcher task
        let task_handle = tokio::spawn(async move {
            Self::watch_task(&paths_clone, debounce_ms, tx, running_clone).await;
        });

        Ok(Self {
            watch_paths: paths_arc,
            rx,
            task_handle,
            running,
        })
    }

    /// Receive the next file change event.
    ///
    /// Returns `Some(path)` when a file change is detected, `None` if the watcher is stopped.
    pub async fn recv(&mut self) -> Option<PathBuf> {
        self.rx.recv().await
    }

    /// Get all paths being watched.
    pub fn watch_paths(&self) -> &HashSet<PathBuf> {
        &self.watch_paths
    }

    /// Stop the watcher.
    pub fn stop(&self) {
        self.running
            .store(false, std::sync::atomic::Ordering::SeqCst);
    }

    /// Check if the watcher is running.
    pub fn is_running(&self) -> bool {
        self.running.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Internal watcher task that integrates with notify-debouncer-full.
    async fn watch_task(
        paths: &HashSet<PathBuf>,
        debounce_ms: u64,
        tx: mpsc::Sender<PathBuf>,
        running: Arc<std::sync::atomic::AtomicBool>,
    ) {
        use notify_debouncer_full::{
            new_debouncer, notify::EventKind, notify::RecursiveMode, DebounceEventResult,
        };

        let paths_clone = paths.clone();
        let running_sync = Arc::clone(&running);

        // Create a bridge channel
        let (bridge_tx, bridge_rx) = std::sync::mpsc::channel::<DebounceEventResult>();

        // Spawn the debouncer in a blocking thread
        std::thread::spawn(move || {
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

            for path in &paths_clone {
                if path.is_dir() {
                    let _ = debouncer.watch(path.as_path(), RecursiveMode::Recursive);
                } else if path.is_file() {
                    if let Some(parent) = path.parent() {
                        let _ = debouncer.watch(parent, RecursiveMode::Recursive);
                    }
                }
            }

            tracing::info!("MultiFsWatcher watching {} paths", paths_clone.len());

            while running_sync.load(std::sync::atomic::Ordering::SeqCst) {
                std::thread::sleep(Duration::from_millis(50));
            }
        });

        // Process events in async context
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
                                            let _ = tx.send(event_path.clone()).await;
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                    tracing::warn!("Bridge channel disconnected");
                    break;
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    // Continue
                }
            }
        }
    }
}
