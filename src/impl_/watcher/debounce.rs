//! Adaptive debouncer for file system events.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// Adaptive debouncer to prevent processing too many events in a short time.
pub struct AdaptiveDebouncer {
    last_event: AtomicU64,
    window_ms: u64,
}

impl AdaptiveDebouncer {
    /// Create a new debouncer with the specified window in milliseconds.
    pub fn new(window_ms: u64) -> Self {
        Self {
            last_event: AtomicU64::new(0),
            window_ms,
        }
    }

    /// Check if the event should be processed.
    /// Returns true if enough time has passed since the last processed event.
    pub fn should_process(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let last = self.last_event.load(Ordering::Relaxed);
        if now.saturating_sub(last) >= self.window_ms {
            self.last_event.store(now, Ordering::Relaxed);
            true
        } else {
            false
        }
    }

    /// Get the debounce window in milliseconds.
    pub fn window_ms(&self) -> u64 {
        self.window_ms
    }

    /// Reset the debouncer state.
    pub fn reset(&self) {
        self.last_event.store(0, Ordering::Relaxed);
    }
}

impl Default for AdaptiveDebouncer {
    fn default() -> Self {
        Self::new(200)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let debouncer = AdaptiveDebouncer::new(100);
        assert_eq!(debouncer.window_ms(), 100);
    }

    #[test]
    fn test_default() {
        let debouncer = AdaptiveDebouncer::default();
        assert_eq!(debouncer.window_ms(), 200);
    }

    #[test]
    fn test_should_process_first_call() {
        let debouncer = AdaptiveDebouncer::new(100);
        assert!(debouncer.should_process());
    }

    #[test]
    fn test_reset() {
        let debouncer = AdaptiveDebouncer::new(1000);
        let _ = debouncer.should_process();
        assert!(!debouncer.should_process());

        debouncer.reset();
        assert!(debouncer.should_process());
    }
}
