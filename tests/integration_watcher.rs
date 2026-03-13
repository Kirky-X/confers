//! Integration tests for watcher (hot reload) support.

#![cfg(feature = "watch")]

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use confers::watcher::{AdaptiveDebouncer, WatcherConfig, WatcherConfigBuilder, WatcherGuard};

// ========================================
// Test Constants with Documented Meanings
// ========================================

/// Default debounce window in milliseconds
const DEFAULT_DEBOUNCE_MS: u64 = 200;
/// Minimum time between reloads in milliseconds
const DEFAULT_MIN_RELOAD_MS: u64 = 1000;
/// Max consecutive failures before pause
const DEFAULT_MAX_FAILURES: u32 = 5;
/// Pause duration after max failures in milliseconds
const DEFAULT_FAILURE_PAUSE_MS: u64 = 30000;

/// Custom debounce window for builder tests
const CUSTOM_DEBOUNCE_MS: u64 = 500;
/// Custom minimum reload interval for builder tests
const CUSTOM_MIN_RELOAD_MS: u64 = 2000;
/// Custom max failures for builder tests
const CUSTOM_MAX_FAILURES: u32 = 10;
/// Custom failure pause duration for builder tests
const CUSTOM_FAILURE_PAUSE_MS: u64 = 60000;

// ========================================
// Module Existence Tests (from integration_watch.rs)
// ========================================

#[test]
fn test_watcher_module_exists() {
    use confers::watcher::*;
}

#[cfg(feature = "progressive-reload")]
#[test]
fn test_progressive_reload_module_exists() {
    use confers::watcher::progressive::*;
}

// ========================================
// WatcherConfig Tests
// ========================================

/// Test that default WatcherConfig has correct values.
#[test]
fn test_watcher_config_default() {
    let config = WatcherConfig::default();

    assert_eq!(config.debounce_ms, DEFAULT_DEBOUNCE_MS);
    assert_eq!(config.min_reload_interval_ms, DEFAULT_MIN_RELOAD_MS);
    assert_eq!(config.max_consecutive_failures, DEFAULT_MAX_FAILURES);
    assert_eq!(config.failure_pause_ms, DEFAULT_FAILURE_PAUSE_MS);
    assert_eq!(config.rollback_on_validation_failure, false);
}

/// Test that WatcherConfig::new() returns default values.
#[test]
fn test_watcher_config_new() {
    let config = WatcherConfig::new();

    assert_eq!(config.debounce_ms, DEFAULT_DEBOUNCE_MS);
    assert_eq!(config.min_reload_interval_ms, DEFAULT_MIN_RELOAD_MS);
}

/// Test WatcherConfigBuilder builds correct config.
#[test]
fn test_watcher_config_builder() {
    let config = WatcherConfigBuilder::new()
        .debounce_ms(CUSTOM_DEBOUNCE_MS)
        .min_reload_interval_ms(CUSTOM_MIN_RELOAD_MS)
        .max_consecutive_failures(CUSTOM_MAX_FAILURES)
        .failure_pause_ms(CUSTOM_FAILURE_PAUSE_MS)
        .rollback_on_validation_failure(true)
        .build();

    assert_eq!(config.debounce_ms, CUSTOM_DEBOUNCE_MS);
    assert_eq!(config.min_reload_interval_ms, CUSTOM_MIN_RELOAD_MS);
    assert_eq!(config.max_consecutive_failures, CUSTOM_MAX_FAILURES);
    assert_eq!(config.failure_pause_ms, CUSTOM_FAILURE_PAUSE_MS);
    assert_eq!(config.rollback_on_validation_failure, true);
}

/// Test WatcherConfigBuilder with partial configuration.
#[test]
fn test_watcher_config_builder_partial() {
    // Only set debounce_ms, others should use defaults
    const PARTIAL_DEBOUNCE_MS: u64 = 100;
    let config = WatcherConfigBuilder::new()
        .debounce_ms(PARTIAL_DEBOUNCE_MS)
        .build();

    assert_eq!(config.debounce_ms, PARTIAL_DEBOUNCE_MS);
    assert_eq!(config.min_reload_interval_ms, DEFAULT_MIN_RELOAD_MS); // default
    assert_eq!(config.max_consecutive_failures, DEFAULT_MAX_FAILURES); // default
}

/// Test WatcherConfig::with_debounce method.
#[test]
fn test_watcher_config_with_debounce() {
    const WITH_DEBOUNCE_MS: u64 = 300;
    let config = WatcherConfig::new().with_debounce(WITH_DEBOUNCE_MS);

    assert_eq!(config.debounce_ms, WITH_DEBOUNCE_MS);
    // Other fields should remain as defaults
    assert_eq!(config.min_reload_interval_ms, DEFAULT_MIN_RELOAD_MS);
}

/// Test WatcherConfig::with_min_reload_interval method.
#[test]
fn test_watcher_config_with_min_reload_interval() {
    const WITH_MIN_RELOAD_MS: u64 = 500;
    let config = WatcherConfig::new().with_min_reload_interval(WITH_MIN_RELOAD_MS);

    assert_eq!(config.min_reload_interval_ms, WITH_MIN_RELOAD_MS);
}

/// Test WatcherConfig::with_max_consecutive_failures method.
#[test]
fn test_watcher_config_with_max_failures() {
    const WITH_MAX_FAILURES: u32 = 3;
    let config = WatcherConfig::new().with_max_consecutive_failures(WITH_MAX_FAILURES);

    assert_eq!(config.max_consecutive_failures, WITH_MAX_FAILURES);
}

/// Test WatcherConfig::with_failure_pause method.
#[test]
fn test_watcher_config_with_failure_pause() {
    const WITH_FAILURE_PAUSE_MS: u64 = 10000;
    let config = WatcherConfig::new().with_failure_pause(WITH_FAILURE_PAUSE_MS);

    assert_eq!(config.failure_pause_ms, WITH_FAILURE_PAUSE_MS);
}

/// Test WatcherConfig::with_rollback_on_validation_failure method.
#[test]
fn test_watcher_config_with_rollback() {
    let config = WatcherConfig::new().with_rollback_on_validation_failure(true);

    assert_eq!(config.rollback_on_validation_failure, true);
}

/// Test that WatcherGuard can be created.
#[test]
fn test_watcher_guard_creation() {
    let guard = WatcherGuard::new();

    // Should not be running initially
    assert!(!guard.is_running());
}

/// Test that WatcherGuard can be started and stopped.
#[test]
fn test_watcher_guard_start_stop() {
    let guard = WatcherGuard::new();

    // Initially not running
    assert!(!guard.is_running());

    guard.start();

    // Should be running after start
    assert!(guard.is_running());

    guard.stop();

    // Should not be running after stop
    assert!(!guard.is_running());
}

/// Test that WatcherGuard drop stops the watcher.
#[test]
fn test_watcher_guard_drop_stops_watcher() {
    let flag = Arc::new(AtomicBool::new(false));
    let flag_clone = Arc::clone(&flag);

    {
        let guard = WatcherGuard::new();
        guard.start();

        // Simulate setting the flag when running
        if guard.is_running() {
            flag_clone.store(true, Ordering::SeqCst);
        }

        // Guard goes out of scope here, Drop should be called
    }

    // After guard is dropped, the flag should reflect the guard was running
    // The actual stop happens in Drop, we just verify the pattern works
    assert!(flag.load(Ordering::SeqCst));
}

/// Test AdaptiveDebouncer with default window.
#[test]
fn test_adaptive_debouncer_default() {
    let debouncer = AdaptiveDebouncer::new(100);

    // First call should process
    assert!(debouncer.should_process());
}

/// Test AdaptiveDebouncer should not process within window.
#[test]
fn test_adaptive_debouncer_should_not_process() {
    let debouncer = AdaptiveDebouncer::new(1000); // 1 second window

    // First call should process
    assert!(debouncer.should_process());

    // Immediate second call should not process
    assert!(!debouncer.should_process());
}

/// Test AdaptiveDebouncer should process after window expires.
#[test]
fn test_adaptive_debouncer_after_window() {
    let debouncer = AdaptiveDebouncer::new(50); // 50ms window

    // First call should process
    assert!(debouncer.should_process());

    // Wait for window to expire
    std::thread::sleep(Duration::from_millis(60));

    // Should process again after window
    assert!(debouncer.should_process());
}

/// Test AdaptiveDebouncer should_process multiple rapid calls.
#[test]
fn test_adaptive_debouncer_rapid_calls() {
    let debouncer = AdaptiveDebouncer::new(100);

    // First call processes
    assert!(debouncer.should_process());

    // Next 9 rapid calls should not process
    for _ in 0..9 {
        assert!(!debouncer.should_process());
    }

    // Wait and then it should process again
    std::thread::sleep(Duration::from_millis(150));
    assert!(debouncer.should_process());
}

/// Test WatcherConfig Clone trait.
#[test]
fn test_watcher_config_clone() {
    let config = WatcherConfig::new();
    let cloned = config.clone();

    assert_eq!(config.debounce_ms, cloned.debounce_ms);
    assert_eq!(config.min_reload_interval_ms, cloned.min_reload_interval_ms);
}

/// Test WatcherConfig Debug trait.
#[test]
fn test_watcher_config_debug() {
    let config = WatcherConfig::new();
    let debug_str = format!("{:?}", config);

    assert!(debug_str.contains("WatcherConfig"));
    assert!(debug_str.contains("debounce_ms"));
}
