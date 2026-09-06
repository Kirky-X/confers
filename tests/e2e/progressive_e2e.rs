// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! E2E: 渐进重载(tests/e2e/progressive_e2e.rs)
//!
//! 场景固化(docs/TEST_SCENARIOS.md §2.10):
//! - PGR-01/02 Immediate:真实 toml 文件构建新配置提交;current() 始终为完整
//!   旧值或完整新值(arc-swap 原子性),Arc 快照指针稳定
//! - PGR-03/04/05 Canary:真实健康检查读 provider 值——Healthy 提交、Critical
//!   以 Err(ConfigError::ReloadRolledBack) 回滚(真实行为,非 Ok(RolledBack))、
//!   Degraded 继续推进不回滚
//! - PGR-06 Canary 无 health_check 默认放行提交
//! - PGR-07 Linear 分步提交
//! - PGR-08 自定义 ReloadHealthCheck(真实校验函数)拒绝无效新配置
//! - PGR-09/10 Clone 语义与 builder 链式构建
//!
//! PGR-11/12(变体完备、ConfigProvider 适配)已有覆盖(tests/core/progressive.rs)。

use async_trait::async_trait;
use confers::error::ConfigError;
use confers::interface::ConfigProvider;
use confers::watcher::{
    HealthStatus, ProgressiveReloader, ReloadHealthCheck, ReloadOutcome, ReloadStrategy,
};
use confers::{ConfigBuilder, ConfigValue};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug, Clone, confers::Config, Deserialize)]
struct AppConfig {
    pub timeout_ms: u32,
    pub max_connections: u32,
}

/// 进程内真实配置数据 provider(模拟数据,非 test double)。
#[derive(Debug)]
struct MapProvider(HashMap<String, AnnotatedValue>);

use confers::types::{AnnotatedValue, SourceId};

impl MapProvider {
    fn with_u64(mut self, key: &str, value: u64) -> Self {
        self.0.insert(
            key.to_string(),
            AnnotatedValue::new(ConfigValue::from(value), SourceId::new("e2e"), key),
        );
        self
    }
}

impl ConfigProvider for MapProvider {
    fn get_raw(&self, key: &str) -> Option<&AnnotatedValue> {
        self.0.get(key)
    }

    fn keys(&self) -> Vec<String> {
        self.0.keys().cloned().collect()
    }
}

/// 真实健康检查:读 provider 中 timeout_ms/max_connections 判定状态。
#[derive(Debug)]
struct ValueHealthCheck {
    min_connections: u64,
}

#[async_trait]
impl ReloadHealthCheck for ValueHealthCheck {
    async fn check(&self, provider: Arc<dyn ConfigProvider>) -> HealthStatus {
        let connections = provider
            .get_raw("max_connections")
            .and_then(|v| v.inner.as_u64())
            .unwrap_or(0);
        let timeout = provider
            .get_raw("timeout_ms")
            .and_then(|v| v.inner.as_u64())
            .unwrap_or(0);

        if timeout == 0 {
            HealthStatus::Critical {
                reason: "timeout_ms must not be zero".to_string(),
            }
        } else if connections < self.min_connections {
            HealthStatus::Degraded {
                reason: format!("connections {connections} below {}", self.min_connections),
            }
        } else {
            HealthStatus::Healthy
        }
    }
}

fn write_config(path: &std::path::Path, timeout_ms: u32, max_connections: u32) {
    std::fs::write(
        path,
        format!("timeout_ms = {timeout_ms}\nmax_connections = {max_connections}\n"),
    )
    .expect("write config file");
}

fn load(path: &std::path::Path) -> AppConfig {
    ConfigBuilder::new()
        .allow_absolute_paths()
        .file(path)
        .build()
        .expect("config must load from real file")
}

#[tokio::test]
async fn pgr01020910_immediate_real_file_flow_atomic_and_clone() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("app.toml");
    write_config(&path, 100, 10);
    let v1 = load(&path);
    assert_eq!((v1.timeout_ms, v1.max_connections), (100, 10));

    let reloader = ProgressiveReloader::new(Arc::new(v1), ReloadStrategy::Immediate);
    let snapshot_before = Arc::clone(&reloader.current());

    // 真实文件变更 → 重新构建 → Immediate 提交(PGR-01)。
    write_config(&path, 200, 20);
    let v2 = load(&path);
    let provider = MapProvider(HashMap::new());
    let outcome = reloader
        .begin_reload(
            Arc::new(v2),
            Arc::new(
                provider
                    .with_u64("timeout_ms", 200)
                    .with_u64("max_connections", 20),
            ),
        )
        .await
        .expect("immediate reload must not error");
    assert!(matches!(outcome, ReloadOutcome::Committed));

    // current() 完整新值,无半新半旧(PGR-02)。
    let current = reloader.current();
    assert_eq!((current.timeout_ms, current.max_connections), (200, 20));

    // Arc 快照:旧快照保持旧值,不受 reload 影响(PGR-09)。
    assert_eq!(
        (snapshot_before.timeout_ms, snapshot_before.max_connections),
        (100, 10)
    );
    assert!(
        !Arc::ptr_eq(&snapshot_before, &reloader.current()),
        "current has moved to the new Arc"
    );

    // Clone 语义:克隆体与本体看到同一最新配置(PGR-09)。
    let cloned = reloader.clone();
    assert_eq!(
        (
            cloned.current().timeout_ms,
            cloned.current().max_connections
        ),
        (200, 20)
    );

    // builder 链式构建等价(PGR-10)。
    let built = ProgressiveReloader::<AppConfig>::builder()
        .initial(Arc::new(AppConfig {
            timeout_ms: 100,
            max_connections: 10,
        }))
        .strategy(ReloadStrategy::Immediate)
        .build();
    assert_eq!(
        (built.current().timeout_ms, built.current().max_connections),
        (100, 10)
    );
}

#[tokio::test]
async fn pgr03040506_canary_health_driven_outcomes() {
    // Healthy → 提交(PGR-03)。
    let healthy = ProgressiveReloader::builder()
        .initial(Arc::new(AppConfig {
            timeout_ms: 100,
            max_connections: 10,
        }))
        .strategy(ReloadStrategy::Canary {
            trial_duration: Duration::from_millis(60),
            poll_interval: Duration::from_millis(10),
        })
        .health_check(Arc::new(ValueHealthCheck { min_connections: 5 }))
        .build();
    let outcome = healthy
        .begin_reload(
            Arc::new(AppConfig {
                timeout_ms: 150,
                max_connections: 30,
            }),
            Arc::new(
                MapProvider(HashMap::new())
                    .with_u64("timeout_ms", 150)
                    .with_u64("max_connections", 30),
            ),
        )
        .await
        .expect("canary healthy");
    assert!(matches!(outcome, ReloadOutcome::Committed));
    assert_eq!(healthy.current().timeout_ms, 150);

    // Critical → 回滚,旧配置保持服务(PGR-04)。
    let critical = ProgressiveReloader::builder()
        .initial(Arc::new(AppConfig {
            timeout_ms: 100,
            max_connections: 10,
        }))
        .strategy(ReloadStrategy::Canary {
            trial_duration: Duration::from_millis(60),
            poll_interval: Duration::from_millis(10),
        })
        .health_check(Arc::new(ValueHealthCheck { min_connections: 1 }))
        .build();
    // 真实行为:回滚以 Err(ConfigError::ReloadRolledBack) 传播(非 Ok(RolledBack))。
    let err = critical
        .begin_reload(
            Arc::new(AppConfig {
                timeout_ms: 0,
                max_connections: 30,
            }),
            Arc::new(
                MapProvider(HashMap::new())
                    .with_u64("timeout_ms", 0)
                    .with_u64("max_connections", 30),
            ),
        )
        .await
        .expect_err("canary critical path must roll back");
    assert!(
        matches!(err, ConfigError::ReloadRolledBack { .. }),
        "rollback error expected, got {err:?}"
    );
    assert_eq!(
        critical.current().timeout_ms,
        100,
        "rolled back to old config"
    );

    // Degraded → 继续推进不回滚(PGR-05)。
    let degraded = ProgressiveReloader::builder()
        .initial(Arc::new(AppConfig {
            timeout_ms: 100,
            max_connections: 10,
        }))
        .strategy(ReloadStrategy::Canary {
            trial_duration: Duration::from_millis(60),
            poll_interval: Duration::from_millis(10),
        })
        .health_check(Arc::new(ValueHealthCheck {
            min_connections: 50,
        }))
        .build();
    let outcome = degraded
        .begin_reload(
            Arc::new(AppConfig {
                timeout_ms: 180,
                max_connections: 20,
            }),
            Arc::new(
                MapProvider(HashMap::new())
                    .with_u64("timeout_ms", 180)
                    .with_u64("max_connections", 20),
            ),
        )
        .await
        .expect("canary degraded path reports outcome");
    assert!(
        matches!(outcome, ReloadOutcome::Committed),
        "degraded must not roll back"
    );
    assert_eq!(degraded.current().timeout_ms, 180);

    // 无 health_check → 默认放行提交(PGR-06)。
    let no_check = ProgressiveReloader::builder()
        .initial(Arc::new(AppConfig {
            timeout_ms: 100,
            max_connections: 10,
        }))
        .strategy(ReloadStrategy::Canary {
            trial_duration: Duration::from_millis(40),
            poll_interval: Duration::from_millis(10),
        })
        .build();
    let outcome = no_check
        .begin_reload(
            Arc::new(AppConfig {
                timeout_ms: 210,
                max_connections: 5,
            }),
            Arc::new(MapProvider(HashMap::new())),
        )
        .await
        .expect("canary without health check");
    assert!(matches!(outcome, ReloadOutcome::Committed));
    assert_eq!(no_check.current().timeout_ms, 210);
}

#[tokio::test]
async fn pgr0708_linear_steps_and_real_validation_rejects_bad_config() {
    // Linear:分步就绪后逐步提交(PGR-07)。
    let linear = ProgressiveReloader::builder()
        .initial(Arc::new(AppConfig {
            timeout_ms: 100,
            max_connections: 10,
        }))
        .strategy(ReloadStrategy::Linear {
            steps: 2,
            interval: Duration::from_millis(10),
        })
        .health_check(Arc::new(ValueHealthCheck { min_connections: 1 }))
        .build();
    let outcome = linear
        .begin_reload(
            Arc::new(AppConfig {
                timeout_ms: 300,
                max_connections: 30,
            }),
            Arc::new(
                MapProvider(HashMap::new())
                    .with_u64("timeout_ms", 300)
                    .with_u64("max_connections", 30),
            ),
        )
        .await
        .expect("linear reload");
    assert!(matches!(outcome, ReloadOutcome::Committed));
    assert_eq!(linear.current().timeout_ms, 300);

    // 真实校验函数拒绝无效新配置:timeout=0 → Critical → 回滚(PGR-08)。
    let guard = ProgressiveReloader::builder()
        .initial(Arc::new(AppConfig {
            timeout_ms: 100,
            max_connections: 10,
        }))
        .strategy(ReloadStrategy::Canary {
            trial_duration: Duration::from_millis(50),
            poll_interval: Duration::from_millis(5),
        })
        .health_check(Arc::new(ValueHealthCheck { min_connections: 1 }))
        .build();
    let err = guard
        .begin_reload(
            Arc::new(AppConfig {
                timeout_ms: 0,
                max_connections: 10,
            }),
            Arc::new(MapProvider(HashMap::new()).with_u64("timeout_ms", 0)),
        )
        .await
        .expect_err("invalid config must be rejected");
    assert!(
        matches!(err, ConfigError::ReloadRolledBack { .. }),
        "rollback error expected, got {err:?}"
    );
    assert_eq!(
        guard.current().timeout_ms,
        100,
        "invalid config never committed"
    );
}
