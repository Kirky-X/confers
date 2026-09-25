// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! Reload failure-pause policy for hot-reload loops.
//!
//! Hot-reload loops reload the configuration on every file event. When the
//! configuration stays broken (parse error, failed validation), retrying on
//! each event burns CPU and floods logs. [`WatcherConfig::max_consecutive_failures`]
//! and [`WatcherConfig::failure_pause_ms`] exist for exactly this, and
//! [`ReloadFailurePolicy`] turns them into the deterministic policy the loop
//! executes: count consecutive failures, open a pause window when the cap is
//! reached, and suppress reload attempts while the window is open.

use std::time::{Duration, Instant};

use super::WatcherConfig;

/// Tracks consecutive reload failures and the pause window that follows.
///
/// The owning reload loop calls [`record_failure`](Self::record_failure) when
/// a reload attempt fails and [`record_success`](Self::record_success) when
/// one succeeds. While [`pause_remaining`](Self::pause_remaining) returns
/// `Some`, the loop should skip reload attempts.
#[derive(Debug)]
pub struct ReloadFailurePolicy {
    max_consecutive_failures: u32,
    failure_pause: Duration,
    consecutive_failures: u32,
    pause_opened_at: Option<Instant>,
}

impl ReloadFailurePolicy {
    /// Create a policy from the watcher configuration.
    pub fn new(config: WatcherConfig) -> Self {
        Self {
            max_consecutive_failures: config.max_consecutive_failures,
            failure_pause: Duration::from_millis(config.failure_pause_ms),
            consecutive_failures: 0,
            pause_opened_at: None,
        }
    }

    /// Record a failed reload attempt.
    ///
    /// Returns `true` when this failure reached
    /// [`WatcherConfig::max_consecutive_failures`] and the pause window just
    /// opened. The failure counter resets as the window opens: after the
    /// pause expires the loop starts counting afresh.
    ///
    /// Failures recorded while the window is open leave it unchanged — the
    /// loop is expected to suppress reload attempts while paused.
    pub fn record_failure(&mut self) -> bool {
        if self.pause_opened_at.is_some() {
            return false;
        }
        self.consecutive_failures += 1;
        if self.consecutive_failures >= self.max_consecutive_failures {
            self.pause_opened_at = Some(Instant::now());
            self.consecutive_failures = 0;
            true
        } else {
            false
        }
    }

    /// Remaining pause time while the window is open, `None` otherwise.
    ///
    /// The window closes automatically once
    /// [`WatcherConfig::failure_pause_ms`] has elapsed since it opened.
    pub fn pause_remaining(&self) -> Option<Duration> {
        let opened_at = self.pause_opened_at?;
        let elapsed = opened_at.elapsed();
        if elapsed >= self.failure_pause {
            None
        } else {
            Some(self.failure_pause - elapsed)
        }
    }

    /// Record a successful reload: resets the consecutive failure counter.
    pub fn record_success(&mut self) {
        self.consecutive_failures = 0;
    }

    /// Consecutive failures since the last success or pause window opening.
    pub fn consecutive_failures(&self) -> u32 {
        self.consecutive_failures
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy(max_failures: u32, pause_ms: u64) -> ReloadFailurePolicy {
        ReloadFailurePolicy::new(
            WatcherConfig::builder()
                .max_consecutive_failures(max_failures)
                .failure_pause_ms(pause_ms)
                .build(),
        )
    }

    #[test]
    fn failures_below_cap_do_not_open_pause() {
        let mut p = policy(3, 1000);
        assert!(!p.record_failure());
        assert!(!p.record_failure());
        assert_eq!(p.consecutive_failures(), 2);
        assert!(p.pause_remaining().is_none());
    }

    #[test]
    fn reaching_cap_opens_pause_and_resets_counter() {
        let mut p = policy(2, 60_000);
        assert!(!p.record_failure());
        assert!(p.record_failure(), "second failure must open the pause");
        assert_eq!(p.consecutive_failures(), 0, "counter resets as pause opens");
        let remaining = p.pause_remaining().expect("pause must be open");
        assert!(remaining <= Duration::from_millis(60_000));
    }

    #[test]
    fn success_resets_failure_counter() {
        let mut p = policy(2, 1000);
        p.record_failure();
        p.record_success();
        assert_eq!(p.consecutive_failures(), 0);
        assert!(
            !p.record_failure(),
            "count restarts from zero after success"
        );
        assert!(p.pause_remaining().is_none());
    }

    #[test]
    fn failures_during_pause_do_not_extend_it() {
        let mut p = policy(1, 60_000);
        assert!(p.record_failure());
        let remaining = p.pause_remaining().unwrap();
        assert!(!p.record_failure(), "no new window opens while paused");
        let still = p.pause_remaining().unwrap();
        assert!(
            still <= remaining,
            "window must not be extended: {still:?} vs {remaining:?}"
        );
    }
}
