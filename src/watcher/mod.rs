// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! File system watcher for hot reload support.

pub(crate) mod debounce;

#[cfg(feature = "progressive-reload")]
pub(crate) mod progressive;

#[cfg(feature = "watch")]
pub(crate) mod fs_watcher;

pub use debounce::AdaptiveDebouncer;

#[cfg(feature = "progressive-reload")]
pub use progressive::{
    HealthStatus, ProgressiveReloader, ProgressiveReloaderBuilder, ReloadHealthCheck,
    ReloadOutcome, ReloadStrategy,
};

#[cfg(feature = "watch")]
pub use fs_watcher::{FsWatcher, MultiFsWatcher};

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

use crate::error::{ConfersResult, ConfigConfigError};

/// Guard for managing watcher lifecycle.
///
/// When dropped, the watcher will be stopped automatically.
/// Use [`shutdown()`](Self::shutdown) for explicit cleanup with waiting.
pub struct WatcherGuard {
    running: Arc<AtomicBool>,
    /// Optional handle for async task cleanup.
    /// Guarded by a Mutex so `shutdown(&self)` can take the handle out
    /// without needing `&mut self`.
    task_handle: Mutex<Option<tokio::task::JoinHandle<()>>>,
}

impl WatcherGuard {
    /// Create a new WatcherGuard in stopped state.
    pub fn new() -> Self {
        Self {
            running: Arc::new(AtomicBool::new(false)),
            task_handle: Mutex::new(None),
        }
    }

    /// Create a new WatcherGuard from an existing running flag.
    pub fn from_running(running: Arc<AtomicBool>) -> Self {
        Self {
            running,
            task_handle: Mutex::new(None),
        }
    }

    /// Create a new WatcherGuard with an associated task handle.
    #[allow(dead_code)]
    pub(crate) fn with_task(
        running: Arc<AtomicBool>,
        task_handle: tokio::task::JoinHandle<()>,
    ) -> Self {
        Self {
            running,
            task_handle: Mutex::new(Some(task_handle)),
        }
    }

    /// Check if the watcher is currently running.
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Start the watcher.
    pub fn start(&self) {
        self.running.store(true, Ordering::SeqCst);
    }

    /// Stop the watcher.
    ///
    /// This signals the watcher task to stop. Use [`shutdown()`](Self::shutdown)
    /// for explicit cleanup with waiting.
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    /// Get a reference to the running flag for sharing.
    pub fn running_flag(&self) -> &Arc<AtomicBool> {
        &self.running
    }

    /// Shutdown the watcher gracefully with timeout.
    ///
    /// This method:
    /// 1. Signals the watcher to stop (sets `running` to false)
    /// 2. If a task handle was registered via `with_task` or `set_task_handle`,
    ///    awaits its completion with the given timeout
    /// 3. If no task handle was registered, returns immediately
    ///
    /// # Arguments
    ///
    /// * `timeout` - Maximum time to wait for the task to complete
    ///
    /// # Returns
    ///
    /// Returns `Ok(true)` if the task completed (or no task was registered),
    /// `Ok(false)` if the timeout elapsed before the task finished.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// async fn example() -> Result<(), Box<dyn std::error::Error>> {
    ///     use std::time::Duration;
    ///     use confers::watcher::WatcherGuard;
    ///
    ///     // Create a guard and shutdown after timeout
    ///     let guard = WatcherGuard::new();
    ///     let result = guard.shutdown(Duration::from_secs(5)).await?;
    ///     assert!(result);
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn shutdown(&self, timeout: Duration) -> crate::error::ConfersResult<bool> {
        self.stop();

        // Take the task handle out of the Mutex so we can await it.
        // The lock is released immediately after take(), so no deadlock risk.
        let handle = self.task_handle.lock().unwrap().take();

        match handle {
            None => Ok(true),
            Some(handle) => {
                match tokio::time::timeout(timeout, handle).await {
                    // Task completed within timeout
                    Ok(_) => Ok(true),
                    // Timeout elapsed — task did not finish in time
                    Err(_) => Ok(false),
                }
            }
        }
    }

    /// Set the task handle for this guard (internal use).
    #[allow(dead_code)]
    pub(crate) fn set_task_handle(&self, handle: tokio::task::JoinHandle<()>) {
        *self.task_handle.lock().unwrap() = Some(handle);
    }
}

#[cfg(feature = "watch")]
impl WatcherGuard {
    /// Start the watcher task (delegates to existing start method).
    #[allow(dead_code)]
    pub(crate) async fn lifecycle_start(&self) -> Result<(), ConfigConfigError> {
        self.start();
        Ok(())
    }

    /// Stop the watcher gracefully (delegates to shutdown).
    #[allow(dead_code)]
    pub(crate) async fn lifecycle_stop(&self) -> ConfersResult<()> {
        self.shutdown(Duration::from_secs(5)).await?;
        Ok(())
    }
}

impl Default for WatcherGuard {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for WatcherGuard {
    fn drop(&mut self) {
        self.stop();
        // Note: We can't await in Drop, so task cancellation is best-effort
        if let Some(handle) = self.task_handle.get_mut().unwrap().take() {
            handle.abort();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;

    /// Regression test for A-H-10: shutdown with no task handle returns Ok(true)
    /// and does not block on the timeout.
    #[tokio::test]
    async fn test_shutdown_no_task_handle_returns_true() {
        let guard = WatcherGuard::new();
        guard.start();
        assert!(guard.is_running());

        // Should return immediately with Ok(true) since there's no task to await
        let result = guard.shutdown(Duration::from_secs(5)).await;
        assert!(result.is_ok(), "shutdown should return Ok");
        assert!(result.unwrap(), "shutdown with no task should return true");
        assert!(
            !guard.is_running(),
            "guard should be stopped after shutdown"
        );
    }

    /// A-H-10: shutdown with a task that completes quickly returns Ok(true).
    #[tokio::test]
    async fn test_shutdown_task_completes_within_timeout_returns_true() {
        let running = Arc::new(AtomicBool::new(true));
        // Spawn a task that finishes immediately
        let handle = tokio::spawn(async {});

        let guard = WatcherGuard::with_task(running, handle);
        let result = guard.shutdown(Duration::from_secs(2)).await;
        assert!(
            result.unwrap(),
            "shutdown should return true when task completes within timeout"
        );
    }

    /// A-H-10: shutdown with a task that sleeps longer than the timeout
    /// returns Ok(false), indicating the task did not finish in time.
    #[tokio::test]
    async fn test_shutdown_task_exceeds_timeout_returns_false() {
        let running = Arc::new(AtomicBool::new(true));
        // Spawn a task that sleeps for 2 seconds — much longer than our timeout
        let handle = tokio::spawn(async {
            tokio::time::sleep(Duration::from_secs(2)).await;
        });

        let guard = WatcherGuard::with_task(running, handle);
        // Use a short timeout so the test runs fast
        let result = guard.shutdown(Duration::from_millis(50)).await;
        assert!(
            !result.unwrap(),
            "shutdown should return false when task exceeds timeout"
        );
    }

    /// Verify set_task_handle registers a task that shutdown can await.
    /// Also ensures the pub(crate) setter is exercised (no dead code).
    #[tokio::test]
    async fn test_set_task_handle_then_shutdown() {
        let guard = WatcherGuard::new();
        guard.start();

        // Register a task via the setter (not the with_task constructor)
        guard.set_task_handle(tokio::spawn(async {}));

        let result = guard.shutdown(Duration::from_secs(2)).await;
        assert!(
            result.unwrap(),
            "shutdown should return true for a task that completes via set_task_handle"
        );
        assert!(!guard.is_running());
    }
}

#[derive(Debug, Clone)]
pub struct WatcherConfig {
    pub debounce_ms: u64,
    pub min_reload_interval_ms: u64,
    pub max_consecutive_failures: u32,
    pub failure_pause_ms: u64,
    pub rollback_on_validation_failure: bool,
}

impl Default for WatcherConfig {
    fn default() -> Self {
        Self {
            debounce_ms: 200,
            min_reload_interval_ms: 1000,
            max_consecutive_failures: 5,
            failure_pause_ms: 30000,
            rollback_on_validation_failure: false,
        }
    }
}

impl WatcherConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn builder() -> WatcherConfigBuilder {
        WatcherConfigBuilder::new()
    }

    pub fn with_debounce(mut self, ms: u64) -> Self {
        self.debounce_ms = ms;
        self
    }

    pub fn with_min_reload_interval(mut self, ms: u64) -> Self {
        self.min_reload_interval_ms = ms;
        self
    }

    pub fn with_max_consecutive_failures(mut self, count: u32) -> Self {
        self.max_consecutive_failures = count;
        self
    }

    pub fn with_failure_pause(mut self, ms: u64) -> Self {
        self.failure_pause_ms = ms;
        self
    }

    pub fn with_rollback_on_validation_failure(mut self, rollback: bool) -> Self {
        self.rollback_on_validation_failure = rollback;
        self
    }
}

pub struct WatcherConfigBuilder {
    debounce_ms: Option<u64>,
    min_reload_interval_ms: Option<u64>,
    max_consecutive_failures: Option<u32>,
    failure_pause_ms: Option<u64>,
    rollback_on_validation_failure: Option<bool>,
}

impl WatcherConfigBuilder {
    pub fn new() -> Self {
        Self {
            debounce_ms: None,
            min_reload_interval_ms: None,
            max_consecutive_failures: None,
            failure_pause_ms: None,
            rollback_on_validation_failure: None,
        }
    }

    pub fn debounce_ms(mut self, ms: u64) -> Self {
        self.debounce_ms = Some(ms);
        self
    }

    pub fn min_reload_interval_ms(mut self, ms: u64) -> Self {
        self.min_reload_interval_ms = Some(ms);
        self
    }

    pub fn max_consecutive_failures(mut self, count: u32) -> Self {
        self.max_consecutive_failures = Some(count);
        self
    }

    pub fn failure_pause_ms(mut self, ms: u64) -> Self {
        self.failure_pause_ms = Some(ms);
        self
    }

    pub fn rollback_on_validation_failure(mut self, rollback: bool) -> Self {
        self.rollback_on_validation_failure = Some(rollback);
        self
    }

    pub fn build(self) -> WatcherConfig {
        WatcherConfig {
            debounce_ms: self.debounce_ms.unwrap_or(200),
            min_reload_interval_ms: self.min_reload_interval_ms.unwrap_or(1000),
            max_consecutive_failures: self.max_consecutive_failures.unwrap_or(5),
            failure_pause_ms: self.failure_pause_ms.unwrap_or(30000),
            rollback_on_validation_failure: self.rollback_on_validation_failure.unwrap_or(false),
        }
    }
}

impl Default for WatcherConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}
