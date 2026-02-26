//! Integration tests for progressive reload support.
//!
//! These tests verify the ProgressiveReloader implementation with various strategies.

#![cfg(feature = "progressive-reload")]

use std::sync::Arc;
use std::time::Duration;

use confers::error::ConfigError;
use confers::traits::ConfigProvider;
use confers::value::AnnotatedValue;
use confers::watcher::progressive::{
    HealthStatus, ProgressiveReloader, ReloadHealthCheck, ReloadOutcome, ReloadStrategy,
};
use async_trait::async_trait;

// Mock ConfigProvider for testing - using simpler trait
#[derive(Debug, Clone)]
struct MockConfigProvider;

impl ConfigProvider for MockConfigProvider {
    fn get_raw(&self, _key: &str) -> Option<&AnnotatedValue> {
        None
    }

    fn keys(&self) -> Vec<String> {
        vec![]
    }
}

// Mock health check that returns healthy status
#[derive(Debug)]
struct HealthyHealthCheck;

#[async_trait]
impl ReloadHealthCheck for HealthyHealthCheck {
    async fn check(&self, _provider: Arc<dyn ConfigProvider>) -> HealthStatus {
        HealthStatus::Healthy
    }
}

// Mock health check that returns critical status
#[derive(Debug)]
struct CriticalHealthCheck {
    reason: String,
}

impl CriticalHealthCheck {
    fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
        }
    }
}

#[async_trait]
impl ReloadHealthCheck for CriticalHealthCheck {
    async fn check(&self, _provider: Arc<dyn ConfigProvider>) -> HealthStatus {
        HealthStatus::Critical {
            reason: self.reason.clone(),
        }
    }
}

// Mock health check that returns degraded status
#[derive(Debug)]
struct DegradedHealthCheck {
    reason: String,
}

impl DegradedHealthCheck {
    fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
        }
    }
}

#[async_trait]
impl ReloadHealthCheck for DegradedHealthCheck {
    async fn check(&self, _provider: Arc<dyn ConfigProvider>) -> HealthStatus {
        HealthStatus::Degraded {
            reason: self.reason.clone(),
        }
    }
}

// Test: Immediate reload commits successfully
#[tokio::test]
async fn test_immediate_reload_commits() {
    let reloader = ProgressiveReloader::new(
        Arc::new(1i32),
        ReloadStrategy::Immediate,
    );

    let new_config = Arc::new(2i32);
    let provider = Arc::new(MockConfigProvider);

    let result = reloader.begin_reload(new_config.clone(), provider).await;

    assert!(matches!(result, Ok(ReloadOutcome::Committed)));
    assert_eq!(*reloader.current(), 2);
}

// Test: Immediate reload replaces current config atomically
#[tokio::test]
async fn test_immediate_reload_atomic() {
    let reloader = ProgressiveReloader::new(
        Arc::new(10i32),
        ReloadStrategy::Immediate,
    );

    // Reload with new value
    let result = reloader
        .begin_reload(Arc::new(20i32), Arc::new(MockConfigProvider))
        .await;

    assert!(matches!(result, Ok(ReloadOutcome::Committed)));
    assert_eq!(*reloader.current(), 20);
}

// Test: Canary strategy commits after healthy trial period
#[tokio::test]
async fn test_canary_reload_commits_when_healthy() {
    let health_check = Arc::new(HealthyHealthCheck);
    let reloader = ProgressiveReloader::builder()
        .initial(Arc::new(1i32))
        .strategy(ReloadStrategy::Canary {
            trial_duration: Duration::from_millis(50),
            poll_interval: Duration::from_millis(10),
        })
        .health_check(health_check)
        .build();

    let new_config = Arc::new(2i32);
    let provider = Arc::new(MockConfigProvider);

    let result = reloader.begin_reload(new_config.clone(), provider).await;

    assert!(matches!(result, Ok(ReloadOutcome::Committed)));
    assert_eq!(*reloader.current(), 2);
}

// Test: Canary strategy rolls back on critical health status
#[tokio::test]
async fn test_canary_reload_rollback_on_critical() {
    let health_check = Arc::new(CriticalHealthCheck::new("config validation failed".to_string()));
    let reloader = ProgressiveReloader::builder()
        .initial(Arc::new(1i32))
        .strategy(ReloadStrategy::Canary {
            trial_duration: Duration::from_secs(5),
            poll_interval: Duration::from_millis(10),
        })
        .health_check(health_check)
        .build();

    let new_config = Arc::new(2i32);
    let provider = Arc::new(MockConfigProvider);

    let result = reloader.begin_reload(new_config.clone(), provider).await;

    assert!(matches!(result, Err(ConfigError::ReloadRolledBack { .. })));
    assert_eq!(*reloader.current(), 1); // Rolled back to original
}

// Test: Canary strategy warns but continues on degraded health
#[tokio::test]
async fn test_canary_reload_continues_on_degraded() {
    let health_check = Arc::new(DegradedHealthCheck::new("high latency".to_string()));
    let reloader = ProgressiveReloader::builder()
        .initial(Arc::new(1i32))
        .strategy(ReloadStrategy::Canary {
            trial_duration: Duration::from_millis(50),
            poll_interval: Duration::from_millis(10),
        })
        .health_check(health_check)
        .build();

    let new_config = Arc::new(2i32);
    let provider = Arc::new(MockConfigProvider);

    let result = reloader.begin_reload(new_config.clone(), provider).await;

    // Should commit after trial period despite degradation warnings
    assert!(matches!(result, Ok(ReloadOutcome::Committed)));
    assert_eq!(*reloader.current(), 2);
}

// Test: Canary without health check works (pass-through)
#[tokio::test]
async fn test_canary_reload_without_health_check() {
    let reloader = ProgressiveReloader::builder()
        .initial(Arc::new(1i32))
        .strategy(ReloadStrategy::Canary {
            trial_duration: Duration::from_millis(50),
            poll_interval: Duration::from_millis(10),
        })
        .build();

    let new_config = Arc::new(2i32);
    let provider = Arc::new(MockConfigProvider);

    let result = reloader.begin_reload(new_config.clone(), provider).await;

    assert!(matches!(result, Ok(ReloadOutcome::Committed)));
    assert_eq!(*reloader.current(), 2);
}

// Test: ProgressiveReloader can be cloned
#[test]
fn test_progressive_reloader_is_clone() {
    let reloader = ProgressiveReloader::new(
        Arc::new(1i32),
        ReloadStrategy::Immediate,
    );

    let _cloned = reloader.clone();
    // If this compiles, the test passes
}

// Test: ProgressiveReloader current() returns Arc<T>
#[test]
fn test_current_returns_arc() {
    let reloader = ProgressiveReloader::new(
        Arc::new(42i32),
        ReloadStrategy::Immediate,
    );

    let current = reloader.current();
    assert_eq!(*current, 42);
}

// Test: ReloadOutcome enum variants
#[test]
fn test_reload_outcome_variants() {
    let committed = ReloadOutcome::Committed;
    let rolled_back = ReloadOutcome::RolledBack {
        reason: "test".to_string(),
    };

    assert!(matches!(committed, ReloadOutcome::Committed));
    assert!(matches!(
        rolled_back,
        ReloadOutcome::RolledBack { reason } if reason == "test"
    ));
}

// Test: HealthStatus enum variants
#[test]
fn test_health_status_variants() {
    let healthy = HealthStatus::Healthy;
    let degraded = HealthStatus::Degraded {
        reason: "high latency".to_string(),
    };
    let critical = HealthStatus::Critical {
        reason: "service down".to_string(),
    };

    assert!(matches!(healthy, HealthStatus::Healthy));
    assert!(matches!(
        degraded,
        HealthStatus::Degraded { reason } if reason == "high latency"
    ));
    assert!(matches!(
        critical,
        HealthStatus::Critical { reason } if reason == "service down"
    ));
}

// Test: Default builder creates working reloader
#[test]
fn test_builder_default() {
    let reloader = ProgressiveReloader::builder()
        .initial(Arc::new(1i32))
        .build();

    assert_eq!(*reloader.current(), 1);
}

// Test: with_dependencies constructor
#[test]
fn test_with_dependencies() {
    let health_check = Arc::new(HealthyHealthCheck);
    let reloader = ProgressiveReloader::with_dependencies(
        Arc::new(1i32),
        ReloadStrategy::Immediate,
        Some(health_check),
    );

    assert_eq!(*reloader.current(), 1);
}

// Test: Linear strategy commits after all steps
#[tokio::test]
async fn test_linear_reload_commits_after_steps() {
    let reloader = ProgressiveReloader::builder()
        .initial(Arc::new(1i32))
        .strategy(ReloadStrategy::Linear {
            steps: 3,
            interval: Duration::from_millis(20),
        })
        .build();

    let new_config = Arc::new(2i32);
    let provider = Arc::new(MockConfigProvider);

    let result = reloader.begin_reload(new_config.clone(), provider).await;

    assert!(matches!(result, Ok(ReloadOutcome::Committed)));
    assert_eq!(*reloader.current(), 2);
}
