// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

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
    ///
    /// The check-and-record step is atomic: when the debouncer is shared
    /// across threads, at most one caller per window gets `true` (a
    /// compare-and-swap loop; losers observe the winner's timestamp and are
    /// suppressed). A silent fallback applies when the system clock is set
    /// before the Unix epoch: the timestamp degrades to `0`, which suppresses
    /// every event (each call lands inside the window) until the clock passes
    /// the epoch again.
    pub fn should_process(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(std::time::Duration::ZERO)
            .as_millis() as u64;

        // Compare-and-swap loop: only the thread that successfully replaces
        // the previous timestamp with `now` may pass the window. A plain
        // load→check→store would let two threads pass simultaneously.
        let mut last = self.last_event.load(Ordering::Acquire);
        loop {
            if now.saturating_sub(last) < self.window_ms {
                return false;
            }
            match self.last_event.compare_exchange(
                last,
                now,
                // Success both reads the prior state and publishes ours, so
                // AcqRel pairs with the Release stores in `reset` and below.
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => return true,
                // Another thread updated the timestamp in the meantime;
                // re-check the window against the fresh value.
                Err(updated) => last = updated,
            }
        }
    }

    /// Get the debounce window in milliseconds.
    pub fn window_ms(&self) -> u64 {
        self.window_ms
    }

    /// Reset the debouncer state.
    ///
    /// Uses `Release` so a reset is properly synchronized with the
    /// `Acquire`/`AcqRel` loads in [`should_process`](Self::should_process)
    /// when the debouncer is shared across threads.
    pub fn reset(&self) {
        self.last_event.store(0, Ordering::Release);
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

    /// Regression test for issue #452: `should_process` used a non-atomic
    /// load→check→store, so two threads could pass the window check at the
    /// same time and a single window admitted two events. The CAS loop must
    /// admit exactly one winner per window even under thread contention.
    #[test]
    fn test_should_process_single_winner_across_threads() {
        use std::sync::Arc;
        use std::sync::Barrier;

        const THREADS: usize = 8;
        // A window far larger than the test runtime guarantees the only way
        // to pass is by winning the race, not by the window elapsing.
        let debouncer = Arc::new(AdaptiveDebouncer::new(60_000));
        let barrier = Arc::new(Barrier::new(THREADS));

        let handles: Vec<_> = (0..THREADS)
            .map(|_| {
                let debouncer = Arc::clone(&debouncer);
                let barrier = Arc::clone(&barrier);
                std::thread::spawn(move || {
                    barrier.wait();
                    debouncer.should_process()
                })
            })
            .collect();

        let winners = handles
            .into_iter()
            .map(|handle| u32::from(handle.join().unwrap()))
            .sum::<u32>();

        assert_eq!(
            winners, 1,
            "exactly one thread may pass the debounce window per period"
        );
    }

    /// Reset must let a new winner through even while other threads are
    /// contending, and the CAS loop must keep admitting only one per window.
    #[test]
    fn test_reset_between_contended_rounds() {
        use std::sync::Arc;
        use std::sync::Barrier;

        let debouncer = Arc::new(AdaptiveDebouncer::new(60_000));
        for _ in 0..5 {
            let barrier = Arc::new(Barrier::new(4));
            let handles: Vec<_> = (0..4)
                .map(|_| {
                    let debouncer = Arc::clone(&debouncer);
                    let barrier = Arc::clone(&barrier);
                    std::thread::spawn(move || {
                        barrier.wait();
                        debouncer.should_process()
                    })
                })
                .collect();
            let winners = handles
                .into_iter()
                .map(|handle| u32::from(handle.join().unwrap()))
                .sum::<u32>();
            assert_eq!(winners, 1);

            debouncer.reset();
        }
    }
}
