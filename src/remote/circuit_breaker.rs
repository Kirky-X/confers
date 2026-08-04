// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See the project root for full license information.

//! Circuit breaker for remote configuration sources.
//!
//! Protects against repeated failures from remote endpoints by temporarily
//! suspending polls after a configurable number of consecutive failures.
//!
//! # State Machine
//!
//! ```text
//! Closed ──failures ≥ threshold──▶ Open
//!   ▲                                  │
//!   │                          backoff timeout
//!   │                                  │
//!   │                                  ▼
//!   └──────success────── HalfOpen ◀────┘
//!          │
//!          └──────failure──▶ Open
//! ```
//!
//! # Exponential Backoff
//!
//! The time spent in the `Open` state before transitioning to `HalfOpen`
//! follows an exponential backoff formula:
//!
//! `backoff = min(base_delay × 2^failure_count, max_delay)`
//!
//! Default `base_delay` is 1 second and `max_delay` is 60 seconds.

use std::time::{Duration, Instant};

/// Default failure threshold before opening the circuit.
const DEFAULT_THRESHOLD: u32 = 5;

/// Default base delay for exponential backoff.
const DEFAULT_BASE_DELAY: Duration = Duration::from_secs(1);

/// Default maximum backoff delay.
const DEFAULT_MAX_DELAY: Duration = Duration::from_secs(60);

/// Circuit breaker states.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// Normal operation — requests pass through.
    Closed,
    /// Failures exceeded threshold — requests are blocked.
    Open,
    /// Probing after backoff — one request is allowed.
    HalfOpen,
}

/// A circuit breaker that tracks consecutive failures and controls
/// whether remote poll operations should proceed.
#[derive(Debug)]
pub struct CircuitBreaker {
    state: CircuitState,
    /// Number of consecutive failures.
    failure_count: u32,
    /// Threshold at which Closed → Open transition occurs.
    threshold: u32,
    /// Base delay for exponential backoff calculation.
    base_delay: Duration,
    /// Maximum backoff delay cap.
    max_delay: Duration,
    /// Instant when the circuit last transitioned to Open.
    opened_at: Option<Instant>,
}

impl Default for CircuitBreaker {
    fn default() -> Self {
        Self::new()
    }
}

impl CircuitBreaker {
    /// Create a new circuit breaker with default settings.
    pub fn new() -> Self {
        Self {
            state: CircuitState::Closed,
            failure_count: 0,
            threshold: DEFAULT_THRESHOLD,
            base_delay: DEFAULT_BASE_DELAY,
            max_delay: DEFAULT_MAX_DELAY,
            opened_at: None,
        }
    }

    /// Set the failure threshold (number of consecutive failures to open).
    pub fn with_threshold(mut self, threshold: u32) -> Self {
        self.threshold = threshold;
        self
    }

    /// Set the base delay for exponential backoff.
    pub fn with_base_delay(mut self, delay: Duration) -> Self {
        self.base_delay = delay;
        self
    }

    /// Set the maximum backoff delay.
    pub fn with_max_delay(mut self, delay: Duration) -> Self {
        self.max_delay = delay;
        self
    }

    /// Returns `true` if a request is allowed in the current state.
    ///
    /// - `Closed`: always allowed.
    /// - `Open`: allowed only if the backoff timeout has elapsed (transitions to `HalfOpen`).
    /// - `HalfOpen`: allowed (single probe request).
    pub fn can_execute(&mut self) -> bool {
        match self.state {
            CircuitState::Closed => true,
            CircuitState::HalfOpen => true,
            CircuitState::Open => {
                if let Some(opened_at) = self.opened_at {
                    let backoff = self.backoff_duration();
                    if opened_at.elapsed() >= backoff {
                        self.state = CircuitState::HalfOpen;
                        true
                    } else {
                        false
                    }
                } else {
                    // Should not happen, but treat as HalfOpen to recover
                    self.state = CircuitState::HalfOpen;
                    true
                }
            }
        }
    }

    /// Record a successful operation. Transitions to `Closed` and resets
    /// the failure counter.
    pub fn record_success(&mut self) {
        self.failure_count = 0;
        self.state = CircuitState::Closed;
        self.opened_at = None;
    }

    /// Record a failed operation. Increments the failure counter and
    /// potentially transitions to `Open`.
    pub fn record_failure(&mut self) {
        self.failure_count = self.failure_count.saturating_add(1);

        match self.state {
            CircuitState::Closed => {
                if self.failure_count >= self.threshold {
                    self.state = CircuitState::Open;
                    self.opened_at = Some(Instant::now());
                }
            }
            CircuitState::HalfOpen => {
                // Any failure in HalfOpen immediately re-opens the circuit
                self.state = CircuitState::Open;
                self.opened_at = Some(Instant::now());
            }
            CircuitState::Open => {
                // Already open, just update the failure count
            }
        }
    }

    /// Returns the current circuit state.
    #[allow(dead_code)]
    pub fn state(&self) -> CircuitState {
        self.state
    }

    /// Returns the current consecutive failure count.
    #[allow(dead_code)]
    pub fn failure_count(&self) -> u32 {
        self.failure_count
    }

    /// Calculate the exponential backoff duration based on the current
    /// failure count.
    ///
    /// Formula: `min(base_delay × 2^failure_count, max_delay)`
    ///
    /// The exponent is capped at 32 to prevent overflow on `Duration::as_millis`.
    pub fn backoff_duration(&self) -> Duration {
        backoff_duration(self.base_delay, self.max_delay, self.failure_count)
    }
}

/// Calculate exponential backoff duration.
///
/// Formula: `min(base_delay × 2^failure_count, max_delay)`
///
/// The exponent is capped to prevent overflow.
pub fn backoff_duration(
    base_delay: Duration,
    max_delay: Duration,
    failure_count: u32,
) -> Duration {
    // Cap the exponent so that base_delay * 2^exponent cannot overflow.
    // Duration::saturating_mul takes u32, so the multiplier must fit in u32.
    // 2^31 = 2,147,483,648 fits in u32; 2^32 does not.
    let exponent = failure_count.min(31);
    let multiplier = (1u64 << exponent) as u32;

    let delay = base_delay.saturating_mul(multiplier);
    if delay > max_delay {
        max_delay
    } else {
        delay
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_state_is_closed() {
        let cb = CircuitBreaker::new();
        assert_eq!(cb.state(), CircuitState::Closed);
        assert_eq!(cb.failure_count(), 0);
    }

    #[test]
    fn test_can_execute_when_closed() {
        let mut cb = CircuitBreaker::new();
        assert!(cb.can_execute());
    }

    #[test]
    fn test_closed_to_open_on_threshold() {
        let mut cb = CircuitBreaker::new().with_threshold(3);

        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Closed);
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Closed);
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);
        assert!(!cb.can_execute());
    }

    #[test]
    fn test_success_resets_to_closed() {
        let mut cb = CircuitBreaker::new().with_threshold(2);

        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);

        cb.record_success();
        assert_eq!(cb.state(), CircuitState::Closed);
        assert_eq!(cb.failure_count(), 0);
        assert!(cb.can_execute());
    }

    #[test]
    fn test_half_open_to_closed_on_success() {
        let mut cb = CircuitBreaker::new()
            .with_threshold(1)
            .with_base_delay(Duration::from_millis(10));

        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);

        // Wait for backoff to expire
        std::thread::sleep(Duration::from_millis(20));

        assert!(cb.can_execute()); // transitions to HalfOpen
        assert_eq!(cb.state(), CircuitState::HalfOpen);

        cb.record_success();
        assert_eq!(cb.state(), CircuitState::Closed);
    }

    #[test]
    fn test_half_open_to_open_on_failure() {
        let mut cb = CircuitBreaker::new()
            .with_threshold(1)
            .with_base_delay(Duration::from_millis(10));

        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);

        std::thread::sleep(Duration::from_millis(20));

        assert!(cb.can_execute()); // transitions to HalfOpen
        assert_eq!(cb.state(), CircuitState::HalfOpen);

        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);
        assert!(!cb.can_execute());
    }

    #[test]
    fn test_backoff_duration_calculation() {
        let base = Duration::from_secs(1);
        let max = Duration::from_secs(60);

        assert_eq!(backoff_duration(base, max, 0), Duration::from_secs(1));
        assert_eq!(backoff_duration(base, max, 1), Duration::from_secs(2));
        assert_eq!(backoff_duration(base, max, 2), Duration::from_secs(4));
        assert_eq!(backoff_duration(base, max, 3), Duration::from_secs(8));
        assert_eq!(backoff_duration(base, max, 4), Duration::from_secs(16));
        assert_eq!(backoff_duration(base, max, 5), Duration::from_secs(32));
        // Capped at max_delay
        assert_eq!(backoff_duration(base, max, 6), Duration::from_secs(60));
        assert_eq!(backoff_duration(base, max, 10), Duration::from_secs(60));
    }

    #[test]
    fn test_backoff_overflow_protection() {
        let base = Duration::from_secs(1);
        let max = Duration::from_secs(60);

        // Very large failure count should not panic
        assert_eq!(backoff_duration(base, max, 100), max);
        assert_eq!(backoff_duration(base, max, u32::MAX), max);
    }

    #[test]
    fn test_backoff_zero_base_delay() {
        let base = Duration::from_secs(0);
        let max = Duration::from_secs(60);

        assert_eq!(backoff_duration(base, max, 0), Duration::from_secs(0));
        assert_eq!(backoff_duration(base, max, 10), Duration::from_secs(0));
    }

    #[test]
    fn test_failure_count_saturates() {
        let mut cb = CircuitBreaker::new().with_threshold(u32::MAX);

        // Should not panic even with many failures
        for _ in 0..100 {
            cb.record_failure();
        }
        assert_eq!(cb.failure_count(), 100);
    }

    #[test]
    fn test_open_blocks_execution() {
        let mut cb = CircuitBreaker::new()
            .with_threshold(2)
            .with_base_delay(Duration::from_secs(60));

        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);

        // Should be blocked (backoff is 60s, we haven't waited)
        assert!(!cb.can_execute());
        assert_eq!(cb.state(), CircuitState::Open);
    }

    #[test]
    fn test_builder_configuration() {
        let cb = CircuitBreaker::new()
            .with_threshold(10)
            .with_base_delay(Duration::from_secs(5))
            .with_max_delay(Duration::from_secs(120));

        assert_eq!(cb.threshold, 10);
        assert_eq!(cb.base_delay, Duration::from_secs(5));
        assert_eq!(cb.max_delay, Duration::from_secs(120));
    }
}
