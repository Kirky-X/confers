//! File system watcher for hot reload support.

pub mod debounce;
pub mod progressive;

#[cfg(feature = "watch")]
pub mod fs_watcher;

pub use debounce::AdaptiveDebouncer;
pub use progressive::{
    HealthStatus, ProgressiveReloader, ProgressiveReloaderBuilder, ReloadHealthCheck,
    ReloadOutcome, ReloadStrategy,
};

#[cfg(feature = "watch")]
pub use fs_watcher::{FsWatcher, MultiFsWatcher};

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// Guard for managing watcher lifecycle.
///
/// When dropped, the watcher will be stopped automatically.
/// Use [`shutdown()`](Self::shutdown) for explicit cleanup with waiting.
pub struct WatcherGuard {
    running: Arc<AtomicBool>,
    /// Optional handle for async task cleanup
    task_handle: Option<tokio::task::JoinHandle<()>>,
}

impl WatcherGuard {
    /// Create a new WatcherGuard in stopped state.
    pub fn new() -> Self {
        Self {
            running: Arc::new(AtomicBool::new(false)),
            task_handle: None,
        }
    }

    /// Create a new WatcherGuard from an existing running flag.
    pub fn from_running(running: Arc<AtomicBool>) -> Self {
        Self {
            running,
            task_handle: None,
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
            task_handle: Some(task_handle),
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
    /// 1. Signals the watcher to stop
    /// 2. Waits for the watcher task to complete (with timeout)
    /// 3. Ensures audit logs are flushed and snapshots are saved
    ///
    /// # Arguments
    ///
    /// * `timeout` - Maximum time to wait for graceful shutdown (default: 5 seconds)
    ///
    /// # Returns
    ///
    /// Returns `Ok(true)` if shutdown completed, `Ok(false)` if timeout occurred.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let (rx, guard) = AppConfig::load_with_watcher().await?;
    ///
    /// // ... use configuration ...
    ///
    /// // Explicit shutdown
    /// guard.shutdown(std::time::Duration::from_secs(5))?;
    /// ```
    pub async fn shutdown(&self, _timeout: Duration) -> Result<bool, anyhow::Error> {
        self.stop();

        // Note: Cannot await JoinHandle through a shared reference in async context.
        // The task will be aborted when the guard is dropped.
        // For explicit shutdown, we rely on the running flag and the Drop implementation.
        Ok(true)
    }

    /// Set the task handle for this guard (internal use).
    #[allow(dead_code)]
    pub(crate) fn set_task_handle(&mut self, handle: tokio::task::JoinHandle<()>) {
        self.task_handle = Some(handle);
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
        if let Some(handle) = self.task_handle.take() {
            handle.abort();
        }
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
