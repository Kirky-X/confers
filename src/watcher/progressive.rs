//! Progressive Reload - Staged configuration deployment with health checks.

use std::sync::Arc;
use std::time::{Duration, Instant};

use arc_swap::ArcSwap;
use async_trait::async_trait;

use crate::error::{ConfigError, ConfigResult};
use crate::traits::ConfigProvider;

/// Reload strategy for hot reload.
#[derive(Debug, Clone, Default)]
pub enum ReloadStrategy {
    #[default]
    Immediate,
    Canary {
        trial_duration: Duration,
        poll_interval: Duration,
    },
    Linear {
        steps: u8,
        interval: Duration,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReloadOutcome {
    Committed,
    RolledBack { reason: String },
}

/// Health check result
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HealthStatus {
    Healthy,
    Degraded { reason: String },
    Critical { reason: String },
}

/// Reload health check trait
#[async_trait]
pub trait ReloadHealthCheck: Send + Sync {
    async fn check(&self, provider: Arc<dyn ConfigProvider>) -> HealthStatus;
}

struct ProgressiveReloaderInner<T: Clone + Send + Sync + 'static> {
    current: ArcSwap<T>,
    candidate: ArcSwap<Option<Arc<T>>>,
    strategy: ReloadStrategy,
    health_check: Option<Arc<dyn ReloadHealthCheck>>,
}

pub struct ProgressiveReloader<T: Clone + Send + Sync + 'static> {
    inner: Arc<ProgressiveReloaderInner<T>>,
}

impl<T: Clone + Send + Sync + 'static> Clone for ProgressiveReloader<T> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<T: Clone + Send + Sync + 'static> ProgressiveReloader<T> {
    pub fn new(initial: Arc<T>, strategy: ReloadStrategy) -> Self {
        Self {
            inner: Arc::new(ProgressiveReloaderInner {
                current: ArcSwap::new(initial),
                candidate: ArcSwap::new(Arc::new(None)),
                strategy,
                health_check: None,
            }),
        }
    }

    pub fn with_dependencies(
        initial: Arc<T>,
        strategy: ReloadStrategy,
        health_check: Option<Arc<dyn ReloadHealthCheck>>,
    ) -> Self {
        Self {
            inner: Arc::new(ProgressiveReloaderInner {
                current: ArcSwap::new(initial),
                candidate: ArcSwap::new(Arc::new(None)),
                strategy,
                health_check,
            }),
        }
    }

    pub fn builder() -> ProgressiveReloaderBuilder<T> {
        ProgressiveReloaderBuilder::new()
    }

    #[inline]
    pub fn current(&self) -> Arc<T> {
        self.inner.current.load_full()
    }

    pub fn with_health_check(mut self, health_check: Arc<dyn ReloadHealthCheck>) -> Self {
        Arc::get_mut(&mut self.inner)
            .expect("Cannot modify shared ProgressiveReloader")
            .health_check = Some(health_check);
        self
    }

    pub async fn begin_reload(
        &self,
        new_config: Arc<T>,
        provider: Arc<dyn ConfigProvider>,
    ) -> ConfigResult<ReloadOutcome> {
        match &self.inner.strategy {
            ReloadStrategy::Immediate => {
                self.inner.current.store(new_config);
                Ok(ReloadOutcome::Committed)
            }
            ReloadStrategy::Canary {
                trial_duration,
                poll_interval,
            } => {
                self.canary_reload(new_config, *trial_duration, *poll_interval, provider)
                    .await
            }
            ReloadStrategy::Linear { steps, interval } => {
                self.linear_reload(new_config, *steps, *interval, provider)
                    .await
            }
        }
    }

    async fn canary_reload(
        &self,
        new_config: Arc<T>,
        trial_duration: Duration,
        poll_interval: Duration,
        provider: Arc<dyn ConfigProvider>,
    ) -> ConfigResult<ReloadOutcome> {
        self.inner
            .candidate
            .store(Arc::new(Some(new_config.clone())));
        let deadline = Instant::now() + trial_duration;

        while Instant::now() < deadline {
            tokio::time::sleep(poll_interval).await;
            if let Some(hc) = &self.inner.health_check {
                match hc.check(provider.clone()).await {
                    HealthStatus::Critical { reason } => {
                        self.inner.candidate.store(Arc::new(None));
                        return Err(ConfigError::ReloadRolledBack { reason });
                    }
                    HealthStatus::Degraded { reason } => {
                        tracing::warn!("Canary degraded: {}", reason);
                    }
                    HealthStatus::Healthy => {}
                }
            }
        }

        self.inner.current.store(new_config);
        self.inner.candidate.store(Arc::new(None));
        Ok(ReloadOutcome::Committed)
    }

    async fn linear_reload(
        &self,
        new_config: Arc<T>,
        steps: u8,
        interval: Duration,
        provider: Arc<dyn ConfigProvider>,
    ) -> ConfigResult<ReloadOutcome> {
        self.inner
            .candidate
            .store(Arc::new(Some(new_config.clone())));

        for step in 0..steps {
            tokio::time::sleep(interval).await;
            if let Some(hc) = &self.inner.health_check {
                match hc.check(provider.clone()).await {
                    HealthStatus::Critical { reason } => {
                        self.inner.candidate.store(Arc::new(None));
                        return Err(ConfigError::ReloadRolledBack {
                            reason: format!("Linear step {} failed: {}", step + 1, reason),
                        });
                    }
                    HealthStatus::Degraded { reason } => {
                        tracing::warn!("Linear step {} degraded: {}", step + 1, reason);
                    }
                    HealthStatus::Healthy => {}
                }
            }
        }

        self.inner.current.store(new_config);
        self.inner.candidate.store(Arc::new(None));
        Ok(ReloadOutcome::Committed)
    }
}

pub struct ProgressiveReloaderBuilder<T: Clone + Send + Sync + 'static> {
    initial: Option<Arc<T>>,
    strategy: Option<ReloadStrategy>,
    health_check: Option<Arc<dyn ReloadHealthCheck>>,
}

impl<T: Clone + Send + Sync + 'static> ProgressiveReloaderBuilder<T> {
    pub fn new() -> Self {
        Self {
            initial: None,
            strategy: Some(ReloadStrategy::Immediate),
            health_check: None,
        }
    }

    pub fn initial(mut self, initial: Arc<T>) -> Self {
        self.initial = Some(initial);
        self
    }

    pub fn strategy(mut self, strategy: ReloadStrategy) -> Self {
        self.strategy = Some(strategy);
        self
    }

    pub fn health_check(mut self, health_check: Arc<dyn ReloadHealthCheck>) -> Self {
        self.health_check = Some(health_check);
        self
    }

    pub fn build(self) -> ProgressiveReloader<T> {
        let initial = self.initial.expect("initial configuration is required");
        let strategy = self.strategy.unwrap_or_default();
        ProgressiveReloader::with_dependencies(initial, strategy, self.health_check)
    }
}

impl<T: Clone + Send + Sync + 'static> Default for ProgressiveReloaderBuilder<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::ConfigProvider;
    use crate::value::AnnotatedValue;

    #[derive(Debug, Clone)]
    struct MockProvider;

    impl ConfigProvider for MockProvider {
        fn get_raw(&self, _key: &str) -> Option<&AnnotatedValue> {
            None
        }

        fn keys(&self) -> Vec<String> {
            vec![]
        }
    }

    #[test]
    fn test_current_returns_initial() {
        let reloader = ProgressiveReloader::new(Arc::new(42i32), ReloadStrategy::Immediate);
        assert_eq!(*reloader.current(), 42);
    }

    #[test]
    fn test_builder_default_strategy() {
        let reloader = ProgressiveReloader::builder()
            .initial(Arc::new(1i32))
            .build();
        assert_eq!(*reloader.current(), 1);
    }

    #[test]
    fn test_clone_preserves_shared_state() {
        let reloader = ProgressiveReloader::new(Arc::new(42i32), ReloadStrategy::Immediate);
        let cloned = reloader.clone();

        // Both should share the same state
        assert_eq!(*cloned.current(), 42);
    }

    #[tokio::test]
    async fn test_immediate_reload() {
        let reloader = ProgressiveReloader::new(Arc::new(1i32), ReloadStrategy::Immediate);
        let result = reloader
            .begin_reload(Arc::new(2i32), Arc::new(MockProvider))
            .await
            .unwrap();
        assert!(matches!(result, ReloadOutcome::Committed));
        assert_eq!(*reloader.current(), 2);
    }

    #[tokio::test]
    async fn test_canary_reload_healthy() {
        struct HealthyCheck;
        #[async_trait]
        impl ReloadHealthCheck for HealthyCheck {
            async fn check(&self, _provider: Arc<dyn ConfigProvider>) -> HealthStatus {
                HealthStatus::Healthy
            }
        }

        let reloader = ProgressiveReloader::new(
            Arc::new(1i32),
            ReloadStrategy::Canary {
                trial_duration: Duration::from_millis(50),
                poll_interval: Duration::from_millis(10),
            },
        )
        .with_health_check(Arc::new(HealthyCheck));

        let result = reloader
            .begin_reload(Arc::new(2i32), Arc::new(MockProvider))
            .await
            .unwrap();
        assert!(matches!(result, ReloadOutcome::Committed));
        assert_eq!(*reloader.current(), 2);
    }

    #[tokio::test]
    async fn test_canary_reload_critical_rollback() {
        struct CriticalCheck;
        #[async_trait]
        impl ReloadHealthCheck for CriticalCheck {
            async fn check(&self, _provider: Arc<dyn ConfigProvider>) -> HealthStatus {
                HealthStatus::Critical {
                    reason: "service unhealthy".to_string(),
                }
            }
        }

        let reloader = ProgressiveReloader::new(
            Arc::new(1i32),
            ReloadStrategy::Canary {
                trial_duration: Duration::from_millis(100),
                poll_interval: Duration::from_millis(10),
            },
        )
        .with_health_check(Arc::new(CriticalCheck));

        let result = reloader
            .begin_reload(Arc::new(2i32), Arc::new(MockProvider))
            .await;
        assert!(matches!(result, Err(ConfigError::ReloadRolledBack { .. })));
        assert_eq!(*reloader.current(), 1);
    }

    #[tokio::test]
    async fn test_linear_reload() {
        let reloader = ProgressiveReloader::new(
            Arc::new(1i32),
            ReloadStrategy::Linear {
                steps: 3,
                interval: Duration::from_millis(10),
            },
        );

        let result = reloader
            .begin_reload(Arc::new(2i32), Arc::new(MockProvider))
            .await
            .unwrap();
        assert!(matches!(result, ReloadOutcome::Committed));
        assert_eq!(*reloader.current(), 2);
    }
}
