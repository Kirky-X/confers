// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! ConfigBus event rate limiter.
//!
//! Prevents event storms when multiple config change events arrive in rapid
//! succession (e.g., K8s ConfigMap rolling updates, rapid NATS publications).
//!
//! Uses two `AtomicU64` fields for lock-free rate limiting:
//! - `last_process_ms`: timestamp of the last processed event
//! - `in_progress`: flag indicating an event is currently being processed
//!
//! # Design (ADR-035 — ConfigBus)
//!
//! Based on `AtomicU64` epoch ms timestamps for lock-free operation.
//! No mutex, no channel, zero allocation on the fast path.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

/// ConfigBus event rate limiter: prevents event storms when multiple
/// `ConfigChangeEvent` values arrive in rapid succession.
///
/// # Lock-free design
///
/// Uses two atomic operations for the hot path (try_acquire):
/// - `last_process_ms` is loaded with `Acquire` ordering
/// - `in_progress` uses `compare_exchange` with `AcqRel` ordering
///
/// This keeps the limiter invisible on the hot path (~10ns per check).
pub struct BusEventLimiter {
    /// Minimum interval between processed events
    min_interval_ms: u64,
    /// Timestamp (epoch ms) of the last processed event
    last_process_ms: AtomicU64,
    /// Whether a processing cycle is currently active
    in_progress: AtomicBool,
}

impl BusEventLimiter {
    /// Create a new limiter with the specified minimum interval.
    pub fn new(min_interval: Duration) -> Self {
        Self {
            min_interval_ms: min_interval.as_millis() as u64,
            last_process_ms: AtomicU64::new(0),
            in_progress: AtomicBool::new(false),
        }
    }

    /// Create a new limiter with a default 500ms minimum interval.
    pub fn default_interval() -> Self {
        Self::new(Duration::from_millis(500))
    }

    /// Try to acquire a processing token.
    ///
    /// Returns `true` if the event should be processed, `false` if it should
    /// be skipped (rate-limited).
    pub fn try_acquire(&self) -> bool {
        let now = now_ms();
        let last = self.last_process_ms.load(Ordering::Acquire);

        if now.saturating_sub(last) < self.min_interval_ms {
            return false;
        }

        if self
            .in_progress
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Relaxed)
            .is_err()
        {
            return false;
        }

        self.last_process_ms.store(now, Ordering::Release);
        true
    }

    /// Release the processing token. Must be called after processing
    /// completes when `try_acquire()` returned `true`.
    pub fn release(&self) {
        self.in_progress.store(false, Ordering::Release);
    }

    /// Reset the limiter state.
    pub fn reset(&self) {
        self.last_process_ms.store(0, Ordering::Release);
        self.in_progress.store(false, Ordering::Release);
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_first_call_acquires() {
        let limiter = BusEventLimiter::new(Duration::from_millis(100));
        assert!(limiter.try_acquire());
        limiter.release();
    }

    #[test]
    fn test_rapid_second_call_rejected() {
        let limiter = BusEventLimiter::new(Duration::from_millis(500));
        assert!(limiter.try_acquire());
        assert!(!limiter.try_acquire()); // Too soon
        limiter.release();
    }

    #[test]
    fn test_release_allows_next() {
        let limiter = BusEventLimiter::new(Duration::from_millis(0));
        assert!(limiter.try_acquire());
        limiter.release();
        // After release, next should acquire (interval is 0)
        // but may fail if time hasn't advanced
        std::thread::sleep(Duration::from_millis(1));
        assert!(limiter.try_acquire());
        limiter.release();
    }

    #[test]
    fn test_reset_clears_state() {
        let limiter = BusEventLimiter::new(Duration::from_millis(500));
        assert!(limiter.try_acquire());
        limiter.reset();
        // After reset, should be able to acquire again
        assert!(limiter.try_acquire());
        limiter.release();
    }

    #[test]
    fn test_default_interval() {
        let limiter = BusEventLimiter::default_interval();
        assert!(limiter.try_acquire());
        limiter.release();
    }
}
