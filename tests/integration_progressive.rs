//! Integration tests for progressive reload support.
//!
//! These tests verify the ProgressiveReloader implementation with various strategies.
//! Uses real configuration types instead of mocks for more realistic testing.

#![cfg(feature = "progressive-reload")]

mod common;

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use confers::error::ConfigError;
use confers::traits::ConfigProvider;
use confers::watcher::progressive::{
    HealthStatus, ProgressiveReloader, ReloadHealthCheck, ReloadOutcome, ReloadStrategy,
};

// Real health check that validates configuration values
#[derive(Debug)]
struct ConfigHealthCheck {
    max_timeout_ms: u32,
    min_connections: usize,
}

impl ConfigHealthCheck {
    fn new(max_timeout_ms: u32, min_connections: usize) -> Self {
        Self {
            max_timeout_ms,
            min_connections,
        }
    }
}

#[async_trait]
impl ReloadHealthCheck for ConfigHealthCheck {
    async fn check(&self, provider: Arc<dyn ConfigProvider>) -> HealthStatus {
        let timeout = provider
            .get_raw("timeout_ms")
            .and_then(|v| v.inner.as_u64())
            .unwrap_or(0) as u32;

        let connections = provider
            .get_raw("max_connections")
            .and_then(|v| v.inner.as_u64())
            .unwrap_or(0) as usize;

        if timeout > self.max_timeout_ms {
            return HealthStatus::Critical {
                reason: format!(
                    "timeout {} exceeds maximum allowed {}",
                    timeout, self.max_timeout_ms
                ),
            };
        }

        if connections < self.min_connections {
            return HealthStatus::Degraded {
                reason: format!(
                    "connections {} below minimum {}",
                    connections, self.min_connections
                ),
            };
        }

        HealthStatus::Healthy
    }
}

// Health check that always returns healthy (for testing successful scenarios)
#[derive(Debug)]
struct AlwaysHealthyCheck;

#[async_trait]
impl ReloadHealthCheck for AlwaysHealthyCheck {
    async fn check(&self, _provider: Arc<dyn ConfigProvider>) -> HealthStatus {
        HealthStatus::Healthy
    }
}

// Health check that always returns critical (for testing rollback scenarios)
#[derive(Debug)]
struct AlwaysCriticalCheck {
    reason: String,
}

impl AlwaysCriticalCheck {
    fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
        }
    }
}

#[async_trait]
impl ReloadHealthCheck for AlwaysCriticalCheck {
    async fn check(&self, _provider: Arc<dyn ConfigProvider>) -> HealthStatus {
        HealthStatus::Critical {
            reason: self.reason.clone(),
        }
    }
}

// Health check that always returns degraded (for testing degraded scenarios)
#[derive(Debug)]
struct AlwaysDegradedCheck {
    reason: String,
}

impl AlwaysDegradedCheck {
    fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
        }
    }
}

#[async_trait]
impl ReloadHealthCheck for AlwaysDegradedCheck {
    async fn check(&self, _provider: Arc<dyn ConfigProvider>) -> HealthStatus {
        HealthStatus::Degraded {
            reason: self.reason.clone(),
        }
    }
}

// Test: Immediate reload commits successfully
#[tokio::test]
async fn test_immediate_reload_commits() {
    let initial_config = Arc::new(common::TestConfig::new(100, 10));
    let reloader = ProgressiveReloader::new(initial_config, ReloadStrategy::Immediate);

    let new_config = Arc::new(common::TestConfig::new(200, 20));
    let provider = Arc::new(common::TestConfig::new(100, 10));

    let result = reloader.begin_reload(new_config.clone(), provider).await;

    assert!(matches!(result, Ok(ReloadOutcome::Committed)));
    assert_eq!(reloader.current().timeout_ms, 200);
    assert_eq!(reloader.current().max_connections, 20);
}

// Test: Immediate reload replaces current config atomically
#[tokio::test]
async fn test_immediate_reload_atomic() {
    let initial_config = Arc::new(common::TestConfig::new(100, 10));
    let reloader = ProgressiveReloader::new(initial_config, ReloadStrategy::Immediate);

    let new_config = Arc::new(common::TestConfig::new(300, 30));
    let provider = Arc::new(common::TestConfig::new(100, 10));

    let result = reloader.begin_reload(new_config, provider).await;

    assert!(matches!(result, Ok(ReloadOutcome::Committed)));
    assert_eq!(reloader.current().timeout_ms, 300);
    assert_eq!(reloader.current().max_connections, 30);
}

// Test: Canary strategy commits after healthy trial period
#[tokio::test]
async fn test_canary_reload_commits_when_healthy() {
    let health_check = Arc::new(AlwaysHealthyCheck);
    let reloader = ProgressiveReloader::builder()
        .initial(Arc::new(common::TestConfig::new(100, 10)))
        .strategy(ReloadStrategy::Canary {
            trial_duration: Duration::from_millis(50),
            poll_interval: Duration::from_millis(10),
        })
        .health_check(health_check)
        .build();

    let new_config = Arc::new(common::TestConfig::new(200, 20));
    let provider = Arc::new(common::TestConfig::new(100, 10));

    let result = reloader.begin_reload(new_config.clone(), provider).await;

    assert!(matches!(result, Ok(ReloadOutcome::Committed)));
    assert_eq!(reloader.current().timeout_ms, 200);
}

// Test: Canary strategy rolls back on critical health status
#[tokio::test]
async fn test_canary_reload_rollback_on_critical() {
    let health_check = Arc::new(AlwaysCriticalCheck::new("config validation failed"));
    let reloader = ProgressiveReloader::builder()
        .initial(Arc::new(common::TestConfig::new(100, 10)))
        .strategy(ReloadStrategy::Canary {
            trial_duration: Duration::from_secs(5),
            poll_interval: Duration::from_millis(10),
        })
        .health_check(health_check)
        .build();

    let new_config = Arc::new(common::TestConfig::new(200, 20));
    let provider = Arc::new(common::TestConfig::new(100, 10));

    let result = reloader.begin_reload(new_config.clone(), provider).await;

    assert!(matches!(result, Err(ConfigError::ReloadRolledBack { .. })));
    assert_eq!(reloader.current().timeout_ms, 100);
}

// Test: Canary strategy warns but continues on degraded health
#[tokio::test]
async fn test_canary_reload_continues_on_degraded() {
    let health_check = Arc::new(AlwaysDegradedCheck::new("high latency"));
    let reloader = ProgressiveReloader::builder()
        .initial(Arc::new(common::TestConfig::new(100, 10)))
        .strategy(ReloadStrategy::Canary {
            trial_duration: Duration::from_millis(50),
            poll_interval: Duration::from_millis(10),
        })
        .health_check(health_check)
        .build();

    let new_config = Arc::new(common::TestConfig::new(200, 20));
    let provider = Arc::new(common::TestConfig::new(100, 10));

    let result = reloader.begin_reload(new_config.clone(), provider).await;

    assert!(matches!(result, Ok(ReloadOutcome::Committed)));
    assert_eq!(reloader.current().timeout_ms, 200);
}

// Test: Canary without health check works (pass-through)
#[tokio::test]
async fn test_canary_reload_without_health_check() {
    let reloader = ProgressiveReloader::builder()
        .initial(Arc::new(common::TestConfig::new(100, 10)))
        .strategy(ReloadStrategy::Canary {
            trial_duration: Duration::from_millis(50),
            poll_interval: Duration::from_millis(10),
        })
        .build();

    let new_config = Arc::new(common::TestConfig::new(200, 20));
    let provider = Arc::new(common::TestConfig::new(100, 10));

    let result = reloader.begin_reload(new_config.clone(), provider).await;

    assert!(matches!(result, Ok(ReloadOutcome::Committed)));
    assert_eq!(reloader.current().timeout_ms, 200);
}

// Test: Real health check validates configuration values
#[tokio::test]
async fn test_real_health_check_validates_config() {
    let health_check = Arc::new(ConfigHealthCheck::new(500, 5));
    let reloader = ProgressiveReloader::builder()
        .initial(Arc::new(common::TestConfig::new(100, 10)))
        .strategy(ReloadStrategy::Canary {
            trial_duration: Duration::from_millis(50),
            poll_interval: Duration::from_millis(10),
        })
        .health_check(health_check)
        .build();

    let new_config = Arc::new(common::TestConfig::new(200, 15));
    let provider = Arc::new(common::TestConfig::new(200, 15));

    let result = reloader.begin_reload(new_config.clone(), provider).await;

    assert!(matches!(result, Ok(ReloadOutcome::Committed)));
    assert_eq!(reloader.current().timeout_ms, 200);
}

// Test: Real health check triggers critical on invalid config
#[tokio::test]
async fn test_real_health_check_critical_on_invalid() {
    let health_check = Arc::new(ConfigHealthCheck::new(500, 5));
    let reloader = ProgressiveReloader::builder()
        .initial(Arc::new(common::TestConfig::new(100, 10)))
        .strategy(ReloadStrategy::Canary {
            trial_duration: Duration::from_secs(5),
            poll_interval: Duration::from_millis(10),
        })
        .health_check(health_check)
        .build();

    let new_config = Arc::new(common::TestConfig::new(1000, 15));
    let provider = Arc::new(common::TestConfig::new(1000, 15));

    let result = reloader.begin_reload(new_config.clone(), provider).await;

    assert!(matches!(result, Err(ConfigError::ReloadRolledBack { .. })));
    assert_eq!(reloader.current().timeout_ms, 100);
}

// Test: ProgressiveReloader can be cloned
#[test]
fn test_progressive_reloader_is_clone() {
    let reloader = ProgressiveReloader::new(
        Arc::new(common::TestConfig::new(100, 10)),
        ReloadStrategy::Immediate,
    );

    let _cloned = reloader.clone();
}

// Test: ProgressiveReloader current() returns Arc<T>
#[test]
fn test_current_returns_arc() {
    let reloader = ProgressiveReloader::new(
        Arc::new(common::TestConfig::new(42, 100)),
        ReloadStrategy::Immediate,
    );

    let current = reloader.current();
    assert_eq!(current.timeout_ms, 42);
    assert_eq!(current.max_connections, 100);
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
        .initial(Arc::new(common::TestConfig::new(100, 10)))
        .build();

    assert_eq!(reloader.current().timeout_ms, 100);
}

// Test: with_dependencies constructor
#[test]
fn test_with_dependencies() {
    let health_check = Arc::new(AlwaysHealthyCheck);
    let reloader = ProgressiveReloader::with_dependencies(
        Arc::new(common::TestConfig::new(100, 10)),
        ReloadStrategy::Immediate,
        Some(health_check),
    );

    assert_eq!(reloader.current().timeout_ms, 100);
}

// Test: Linear strategy commits after all steps
#[tokio::test]
async fn test_linear_reload_commits_after_steps() {
    let reloader = ProgressiveReloader::builder()
        .initial(Arc::new(common::TestConfig::new(100, 10)))
        .strategy(ReloadStrategy::Linear {
            steps: 3,
            interval: Duration::from_millis(20),
        })
        .build();

    let new_config = Arc::new(common::TestConfig::new(200, 20));
    let provider = Arc::new(common::TestConfig::new(100, 10));

    let result = reloader.begin_reload(new_config.clone(), provider).await;

    assert!(matches!(result, Ok(ReloadOutcome::Committed)));
    assert_eq!(reloader.current().timeout_ms, 200);
}

// Test: ConfigProvider implementation works correctly
#[test]
fn test_config_provider_implementation() {
    let config = common::TestConfig::new(500, 100);

    assert!(config.get_raw("timeout_ms").is_some());
    assert!(config.get_raw("max_connections").is_some());
    assert!(config.get_raw("nonexistent").is_none());

    let keys = config.keys();
    assert_eq!(keys.len(), 4);
}

// Test: ConfigHealthCheck returns degraded for low connections
#[tokio::test]
async fn test_config_health_check_degraded() {
    let health_check = ConfigHealthCheck::new(1000, 10);

    let config = Arc::new(common::TestConfig::new(500, 5));
    let status = health_check.check(config).await;

    assert!(matches!(status, HealthStatus::Degraded { .. }));
}
