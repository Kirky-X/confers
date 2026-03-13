//! 渐进式重载示例程序
//!
//! 本示例展示如何使用 confers 库的渐进式重载功能，实现配置的热更新与自动回滚。
//!
//! 核心特性：
//! - 金丝雀发布 (Canary): 先小比例测试，逐步扩大
//! - 线性推出 (Linear): 平滑过渡到新配置
//! - 健康检查: 评估新配置的稳定性
//! - 自动回滚: 问题配置自动回退

use anyhow::Result;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{error, info, warn, Level};
use tracing_subscriber::FmtSubscriber;

// ============================================================
// 1. 配置结构定义
// ============================================================

/// 应用程序配置结构
/// 使用 serde 和 confers 进行配置管理
#[derive(Debug, Clone, serde::Deserialize)]
pub struct AppConfig {
    /// 服务器配置
    #[serde(rename = "server")]
    pub server: ServerConfig,

    /// 数据库配置
    #[serde(rename = "database")]
    pub database: DatabaseConfig,

    /// 功能开关配置
    #[serde(rename = "feature")]
    pub feature: FeatureConfig,

    /// 渐进式重载配置
    #[serde(rename = "reload")]
    pub reload: ReloadConfig,
}

/// 服务器配置
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub workers: u32,
}

/// 数据库配置
#[derive(Debug, Clone, serde::Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub connection_timeout: u32,
}

/// 功能开关配置
#[derive(Debug, Clone, serde::Deserialize)]
pub struct FeatureConfig {
    pub new_ui: bool,
    pub beta_api: bool,
    pub realtime_notification: bool,
}

/// 渐进式重载配置
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ReloadConfig {
    /// 重载策略: "canary" 或 "linear"
    pub strategy: String,

    /// 金丝雀策略配置
    #[serde(rename = "canary")]
    pub canary: Option<CanaryConfig>,

    /// 线性策略配置
    #[serde(rename = "linear")]
    pub linear: Option<LinearConfig>,

    /// 健康检查配置
    #[serde(rename = "health_check")]
    pub health_check: HealthCheckConfig,

    /// 自动回滚配置
    #[serde(rename = "rollback")]
    pub rollback: RollbackConfig,
}

/// 金丝雀发布策略配置
#[derive(Debug, Clone, serde::Deserialize)]
pub struct CanaryConfig {
    /// 初始流量比例 (0.0 ~ 1.0)
    pub initial_ratio: f64,
    /// 每次增加的流量比例
    pub step_ratio: f64,
    /// 每批次间隔秒数
    pub interval_seconds: u64,
}

/// 线性推出策略配置
#[derive(Debug, Clone, serde::Deserialize)]
pub struct LinearConfig {
    /// 每批次数量
    pub batch_size: u32,
    /// 每批次间隔秒数
    pub interval_seconds: u64,
}

/// 健康检查配置
#[derive(Debug, Clone, serde::Deserialize)]
pub struct HealthCheckConfig {
    /// 是否启用
    pub enabled: bool,
    /// 错误率阈值
    pub error_threshold: f64,
    /// 延迟阈值（毫秒）
    pub latency_ms: u64,
    /// 检查超时秒数
    pub check_timeout: u64,
    /// 失败容忍次数
    pub failure_tolerance: u32,
}

/// 自动回滚配置
#[derive(Debug, Clone, serde::Deserialize)]
pub struct RollbackConfig {
    /// 是否启用自动回滚
    pub enabled: bool,
    /// 最大重试次数
    pub max_retries: u32,
    /// 是否保留失败配置
    pub preserve_failed_config: bool,
    /// 回滚超时秒数
    pub rollback_timeout: u64,
}

// ============================================================
// 2. 渐进式重载核心类型定义
// ============================================================

/// 重载策略枚举
/// 定义两种主要的渐进式重载策略
#[derive(Debug, Clone, Default)]
pub enum ReloadStrategy {
    /// 立即重载（默认）
    #[default]
    Immediate,
    /// 金丝雀发布策略
    ///
    /// 特点：
    /// - 先将新配置应用到少量实例（初始比例）
    /// - 监控健康状态，如果稳定则逐步扩大比例
    /// - 适合风险较高的配置变更
    ///
    /// 比例进度示例: 10% → 30% → 50% → 100%
    Canary {
        /// 初始流量比例 (默认 10%)
        initial_ratio: f64,
        /// 每次增加的流量比例 (默认 20%)
        step_ratio: f64,
        /// 每批次间隔时间
        interval: Duration,
    },

    /// 线性推出策略
    ///
    /// 特点：
    /// - 均匀地将流量切换到新配置
    /// - 每批次间隔固定时间
    /// - 适合需要平滑过渡的场景
    ///
    /// 比例进度示例: 10% → 20% → 30% → ... → 100%
    Linear {
        /// 每批次的实例/流量数量
        batch_size: u32,
        /// 每批次间隔时间
        interval: Duration,
    },
}

impl ReloadStrategy {
    /// 获取当前策略名称
    pub fn name(&self) -> &'static str {
        match self {
            ReloadStrategy::Immediate => "Immediate (立即重载)",
            ReloadStrategy::Canary { .. } => "Canary (金丝雀发布)",
            ReloadStrategy::Linear { .. } => "Linear (线性推出)",
        }
    }
}

/// 健康检查状态枚举
///
/// 用于评估新配置是否稳定
#[derive(Debug, Clone, PartialEq)]
pub enum HealthStatus {
    /// 配置完全正常，可以继续扩大比例
    Healthy,
    /// 配置存在问题，需要回滚
    Unhealthy,
    /// 暂时无法判断，需要更多时间观察
    Unknown,
}

/// 健康检查上下文
/// 包含健康检查所需的各种指标和信息
#[derive(Debug, Clone)]
pub struct HealthCheckContext {
    /// 当前重载比例 (0.0 ~ 1.0)
    pub current_ratio: f64,
    /// 请求总数
    pub total_requests: u64,
    /// 错误数量
    pub error_count: u64,
    /// 平均响应时间（毫秒）
    pub avg_latency_ms: f64,
    /// P99 响应时间（毫秒）
    pub p99_latency_ms: f64,
    /// 内存使用量（字节）
    pub memory_usage_bytes: u64,
    /// CPU 使用率 (0.0 ~ 1.0)
    pub cpu_usage: f64,
    /// 额外自定义指标
    pub custom_metrics: std::collections::HashMap<String, String>,
}

impl HealthCheckContext {
    /// 创建一个新的健康检查上下文
    pub fn new(ratio: f64) -> Self {
        Self {
            current_ratio: ratio,
            total_requests: 0,
            error_count: 0,
            avg_latency_ms: 0.0,
            p99_latency_ms: 0.0,
            memory_usage_bytes: 0,
            cpu_usage: 0.0,
            custom_metrics: std::collections::HashMap::new(),
        }
    }

    /// 计算错误率
    pub fn error_rate(&self) -> f64 {
        if self.total_requests == 0 {
            return 0.0;
        }
        self.error_count as f64 / self.total_requests as f64
    }
}

/// 健康检查器特征
///
/// 用户可以实现此特征来定义自定义的健康检查逻辑
pub trait HealthCheck: Send + Sync {
    /// 执行健康检查
    ///
    /// # 参数
    /// - `context`: 健康检查上下文，包含各种指标
    ///
    /// # 返回
    /// - `HealthStatus::Healthy`: 配置健康，继续扩大比例
    /// - `HealthStatus::Unhealthy`: 配置不健康，需要回滚
    /// - `HealthStatus::Unknown`: 无法判断，需要更多观察时间
    fn check(&self, context: &HealthCheckContext) -> HealthStatus;

    /// 获取健康检查器名称（用于日志）
    fn name(&self) -> &'static str {
        "DefaultHealthCheck"
    }

    /// 重置健康检查器状态（可选实现）
    fn reset(&mut self) {}
}

// ============================================================
// 3. 健康检查实现示例
// ============================================================

/// 示例：基于指标的健康检查器
///
/// 检查以下指标：
/// - 错误率不超过阈值
/// - 响应延迟不超过阈值
/// - 内存使用在合理范围内
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct MetricsBasedHealthCheck {
    /// 错误率阈值 (默认 5%)
    error_threshold: f64,
    /// 响应延迟阈值 (默认 100ms)
    latency_threshold_ms: u64,
    /// 失败容忍次数
    failure_tolerance: u32,
    /// 当前连续失败次数
    consecutive_failures: u32,
}

impl MetricsBasedHealthCheck {
    /// 创建新的健康检查器
    pub fn new(error_threshold: f64, latency_threshold_ms: u64, failure_tolerance: u32) -> Self {
        Self {
            error_threshold,
            latency_threshold_ms,
            failure_tolerance,
            consecutive_failures: 0,
        }
    }

    /// 重置失败计数
    pub fn reset(&mut self) {
        self.consecutive_failures = 0;
    }
}

impl HealthCheck for MetricsBasedHealthCheck {
    fn check(&self, context: &HealthCheckContext) -> HealthStatus {
        // 检查错误率
        let error_rate = context.error_rate();
        if error_rate > self.error_threshold {
            info!(
                "健康检查失败: 错误率 {:.2}% 超过阈值 {:.2}%",
                error_rate * 100.0,
                self.error_threshold * 100.0
            );
            return HealthStatus::Unhealthy;
        }

        // 检查 P99 延迟
        if context.p99_latency_ms > self.latency_threshold_ms as f64 {
            info!(
                "健康检查失败: P99 延迟 {:.2}ms 超过阈值 {}ms",
                context.p99_latency_ms, self.latency_threshold_ms
            );
            return HealthStatus::Unhealthy;
        }

        // 所有检查通过
        HealthStatus::Healthy
    }

    fn name(&self) -> &'static str {
        "MetricsBasedHealthCheck"
    }
}

/// 示例：自定义业务健康检查器
///
/// 用户可以基于业务逻辑实现更复杂的健康检查
#[derive(Default)]
pub struct BusinessHealthCheck {
    /// 检查项列表
    checks: Vec<Box<dyn Fn() -> bool + Send + Sync>>,
}

impl BusinessHealthCheck {
    /// 添加一个检查项
    pub fn add_check<F>(&mut self, _name: &'static str, check: F)
    where
        F: Fn() -> bool + Send + Sync + 'static,
    {
        self.checks.push(Box::new(check));
    }
}

impl HealthCheck for BusinessHealthCheck {
    fn check(&self, _context: &HealthCheckContext) -> HealthStatus {
        // 执行业务检查
        for (i, check) in self.checks.iter().enumerate() {
            if !check() {
                info!("业务检查项 {} 失败", i);
                return HealthStatus::Unhealthy;
            }
        }
        HealthStatus::Healthy
    }

    fn name(&self) -> &'static str {
        "BusinessHealthCheck"
    }
}

// ============================================================
// 4. 渐进式重载器实现
// ============================================================

/// 渐进式重载器
///
/// 负责管理配置的重载流程，包括：
/// - 策略执行（金丝雀/线性）
/// - 健康检查
/// - 自动回滚
pub struct ProgressiveReloader {
    /// 当前配置
    config: Arc<RwLock<AppConfig>>,
    /// 上一稳定配置（用于回滚）
    previous_config: Arc<RwLock<Option<AppConfig>>>,
    /// 重载策略
    strategy: ReloadStrategy,
    /// 健康检查器
    health_check: Arc<RwLock<dyn HealthCheck>>,
    /// 是否启用自动回滚
    enable_rollback: bool,
    /// 当前重载比例
    current_ratio: Arc<RwLock<f64>>,
    /// 是否正在进行重载
    is_reloading: Arc<RwLock<bool>>,
    /// 重试次数
    retry_count: Arc<RwLock<u32>>,
}

impl ProgressiveReloader {
    /// 构建渐进式重载器
    pub fn builder() -> ReloaderBuilder {
        ReloaderBuilder::default()
    }

    /// 启动金丝雀发布
    ///
    /// # 参数
    /// - `new_config`: 新的配置
    ///
    /// # 流程
    /// 1. 保存当前配置作为回滚点
    /// 2. 以初始比例（10%）应用新配置
    /// 3. 执行健康检查
    /// 4. 如果健康，扩大比例；如果不健康，回滚
    pub async fn start_canary_reload(&self, new_config: AppConfig) -> Result<()> {
        // 检查是否正在进行重载
        {
            let mut is_reloading = self.is_reloading.write().await;
            if *is_reloading {
                warn!("重载正在进行中，跳过此次请求");
                return Ok(());
            }
            *is_reloading = true;
        }

        info!("开始金丝雀发布，策略: {:?}", self.strategy.name());

        // 保存当前配置作为回滚点
        let current = self.config.read().await.clone();
        *self.previous_config.write().await = Some(current.clone());

        // 重置状态
        *self.current_ratio.write().await = 0.0;
        self.health_check.write().await.reset();
        *self.retry_count.write().await = 0;

        // 获取金丝雀配置
        let canary_config = match &self.strategy {
            ReloadStrategy::Canary {
                initial_ratio,
                step_ratio,
                interval,
            } => (initial_ratio, step_ratio, interval),
            _ => {
                error!("当前策略不是金丝雀发布");
                *self.is_reloading.write().await = false;
                return Ok(());
            }
        };

        // 开始金丝雀发布流程
        let mut current_ratio = *canary_config.0;

        while current_ratio <= 1.0 {
            // 应用当前比例的配置
            info!("应用金丝雀配置，比例: {:.1}%", current_ratio * 100.0);

            // 模拟应用配置到对应比例的实例
            self.apply_config_ratio(&new_config, current_ratio).await?;

            // 等待一段时间让新配置稳定
            tokio::time::sleep(*canary_config.2).await;

            // 执行健康检查
            let context = self.collect_health_metrics(current_ratio).await;
            let status = self.health_check.read().await.check(&context);

            match status {
                HealthStatus::Healthy => {
                    info!("健康检查通过 ✓，当前比例: {:.1}%", current_ratio * 100.0);

                    // 检查是否完成
                    if current_ratio >= 1.0 {
                        info!("金丝雀发布完成 ✓，新配置已完全生效");
                        break;
                    }

                    // 扩大比例
                    current_ratio = (current_ratio + canary_config.1).min(1.0);
                }
                HealthStatus::Unhealthy => {
                    error!("健康检查失败 ✗，触发自动回滚");

                    // 检查是否超过最大重试次数
                    let mut retry = self.retry_count.write().await;
                    *retry += 1;

                    let max_retries = self.config.read().await.reload.rollback.max_retries;
                    if *retry > max_retries {
                        error!("重试次数超过最大限制 {}，停止重载", max_retries);
                        // 执行回滚
                        if self.enable_rollback {
                            self.rollback().await?;
                        }
                        break;
                    }

                    // 自动回滚
                    if self.enable_rollback {
                        self.rollback().await?;
                    }

                    // 重置比例重新开始
                    current_ratio = *canary_config.0;
                }
                HealthStatus::Unknown => {
                    info!("健康检查结果未知，继续观察");
                    // 继续等待，不改变比例
                }
            }
        }

        *self.is_reloading.write().await = false;
        Ok(())
    }

    /// 启动线性推出
    pub async fn start_linear_reload(&self, new_config: AppConfig) -> Result<()> {
        info!("开始线性推出，策略: {:?}", self.strategy.name());

        // 保存当前配置
        let current = self.config.read().await.clone();
        *self.previous_config.write().await = Some(current);

        *self.current_ratio.write().await = 0.0;
        *self.is_reloading.write().await = true;

        // 获取线性配置
        let linear_config = match &self.strategy {
            ReloadStrategy::Linear {
                batch_size,
                interval,
            } => (batch_size, interval),
            _ => {
                error!("当前策略不是线性推出");
                *self.is_reloading.write().await = false;
                return Ok(());
            }
        };

        let mut current_ratio = 0.0;
        let step = *linear_config.0 as f64 / 100.0;

        while current_ratio < 1.0 {
            // 增加比例
            current_ratio = (current_ratio + step).min(1.0);

            info!("线性推出进度: {:.1}%", current_ratio * 100.0);

            // 应用配置
            self.apply_config_ratio(&new_config, current_ratio).await?;

            // 等待
            tokio::time::sleep(*linear_config.1).await;

            // 健康检查
            let context = self.collect_health_metrics(current_ratio).await;
            let status = self.health_check.read().await.check(&context);

            if status == HealthStatus::Unhealthy && self.enable_rollback {
                error!("健康检查失败，触发回滚");
                self.rollback().await?;
                break;
            }
        }

        *self.is_reloading.write().await = false;
        Ok(())
    }

    /// 应用配置到指定比例的实例
    async fn apply_config_ratio(&self, config: &AppConfig, ratio: f64) -> Result<()> {
        // 这里应该是实际的配置应用逻辑
        // 例如：通过配置中心推送配置到对应比例的实例
        info!("模拟: 应用 {:.1}% 的配置", ratio * 100.0);
        *self.config.write().await = config.clone();
        Ok(())
    }

    /// 收集健康指标
    async fn collect_health_metrics(&self, ratio: f64) -> HealthCheckContext {
        // 这里应该是从监控系统获取实际指标
        // 模拟一些指标数据
        HealthCheckContext {
            current_ratio: ratio,
            total_requests: 1000,
            error_count: 10,
            avg_latency_ms: 50.0,
            p99_latency_ms: 80.0,
            memory_usage_bytes: 100_000_000,
            cpu_usage: 0.3,
            custom_metrics: std::collections::HashMap::new(),
        }
    }

    /// 执行回滚
    async fn rollback(&self) -> Result<()> {
        info!("执行自动回滚...");

        let previous = self.previous_config.read().await;
        if let Some(config) = previous.as_ref() {
            *self.config.write().await = config.clone();
            info!("已回滚到上一稳定配置 ✓");
        }

        *self.current_ratio.write().await = 0.0;
        Ok(())
    }

    /// 获取当前配置
    pub async fn get_config(&self) -> AppConfig {
        self.config.read().await.clone()
    }
}

/// 渐进式重载器构建器
#[derive(Default)]
pub struct ReloaderBuilder {
    strategy: Option<ReloadStrategy>,
    health_check: Option<Arc<RwLock<dyn HealthCheck>>>,
    enable_rollback: Option<bool>,
    config: Option<AppConfig>,
}

impl ReloaderBuilder {
    /// 设置重载策略
    pub fn strategy(mut self, strategy: ReloadStrategy) -> Self {
        self.strategy = Some(strategy);
        self
    }

    /// 设置健康检查器
    pub fn health_check<H: HealthCheck + 'static>(mut self, health_check: H) -> Self {
        self.health_check = Some(Arc::new(RwLock::new(health_check)));
        self
    }

    /// 设置是否启用自动回滚
    pub fn rollback_on_failure(mut self, enable: bool) -> Self {
        self.enable_rollback = Some(enable);
        self
    }

    /// 设置初始配置
    pub fn initial_config(mut self, config: AppConfig) -> Self {
        self.config = Some(config);
        self
    }

    /// 构建渐进式重载器
    pub fn build(self) -> ProgressiveReloader {
        let default_config = AppConfig {
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 8080,
                workers: 4,
            },
            database: DatabaseConfig {
                url: "postgres://localhost:5432/appdb".to_string(),
                max_connections: 10,
                connection_timeout: 30,
            },
            feature: FeatureConfig {
                new_ui: false,
                beta_api: false,
                realtime_notification: false,
            },
            reload: ReloadConfig {
                strategy: "canary".to_string(),
                canary: Some(CanaryConfig {
                    initial_ratio: 0.1,
                    step_ratio: 0.2,
                    interval_seconds: 10,
                }),
                linear: Some(LinearConfig {
                    batch_size: 10,
                    interval_seconds: 5,
                }),
                health_check: HealthCheckConfig {
                    enabled: true,
                    error_threshold: 0.05,
                    latency_ms: 100,
                    check_timeout: 5,
                    failure_tolerance: 2,
                },
                rollback: RollbackConfig {
                    enabled: true,
                    max_retries: 3,
                    preserve_failed_config: true,
                    rollback_timeout: 30,
                },
            },
        };

        let health_check = self
            .health_check
            .unwrap_or_else(|| Arc::new(RwLock::new(MetricsBasedHealthCheck::new(0.05, 100, 2))));

        let strategy = self.strategy.unwrap_or_else(|| ReloadStrategy::Canary {
            initial_ratio: 0.1,
            step_ratio: 0.2,
            interval: Duration::from_secs(10),
        });

        ProgressiveReloader {
            config: Arc::new(RwLock::new(self.config.unwrap_or(default_config.clone()))),
            previous_config: Arc::new(RwLock::new(None)),
            strategy,
            health_check,
            enable_rollback: self.enable_rollback.unwrap_or(true),
            current_ratio: Arc::new(RwLock::new(0.0)),
            is_reloading: Arc::new(RwLock::new(false)),
            retry_count: Arc::new(RwLock::new(0)),
        }
    }
}

// ============================================================
// 5. 主程序
// ============================================================

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("设置日志失败");

    info!("========================================");
    info!("  渐进式重载示例程序启动");
    info!("========================================");

    // 读取配置文件 (使用异步 I/O)
    let config_content = tokio::fs::read_to_string("config/config.toml").await?;
    let config: AppConfig = toml::from_str(&config_content)?;

    info!("配置文件加载成功:");
    info!("  - 服务器: {}:{}", config.server.host, config.server.port);
    info!("  - 工作线程: {}", config.server.workers);
    info!("  - 数据库: {}", config.database.url);
    info!(
        "  - 功能开关: new_ui={}, beta_api={}",
        config.feature.new_ui, config.feature.beta_api
    );
    info!("  - 重载策略: {}", config.reload.strategy);

    // 根据配置创建重载策略
    let strategy = match config.reload.strategy.as_str() {
        "canary" => {
            let canary = config.reload.canary.as_ref().expect("金丝雀配置缺失");
            ReloadStrategy::Canary {
                initial_ratio: canary.initial_ratio,
                step_ratio: canary.step_ratio,
                interval: Duration::from_secs(canary.interval_seconds),
            }
        }
        "linear" => {
            let linear = config.reload.linear.as_ref().expect("线性配置缺失");
            ReloadStrategy::Linear {
                batch_size: linear.batch_size,
                interval: Duration::from_secs(linear.interval_seconds),
            }
        }
        _ => {
            error!("未知的重载策略: {}, 使用默认策略", config.reload.strategy);
            ReloadStrategy::default()
        }
    };

    // 创建健康检查器
    let health_check = MetricsBasedHealthCheck::new(
        config.reload.health_check.error_threshold,
        config.reload.health_check.latency_ms,
        config.reload.health_check.failure_tolerance,
    );

    // 创建渐进式重载器
    let reloader = ProgressiveReloader::builder()
        .strategy(strategy)
        .health_check(health_check)
        .rollback_on_failure(config.reload.rollback.enabled)
        .initial_config(config.clone())
        .build();

    info!("渐进式重载器创建成功");
    info!("  - 策略: {}", reloader.strategy.name());
    info!(
        "  - 自动回滚: {}",
        if config.reload.rollback.enabled {
            "启用"
        } else {
            "禁用"
        }
    );

    // 模拟配置变更
    info!("");
    info!("========================================");
    info!("  模拟配置变更场景");
    info!("========================================");

    // 创建新配置（模拟配置变更）
    let mut new_config = config.clone();
    new_config.server.port = 8081; // 更改端口
    new_config.feature.new_ui = true; // 启用新功能
    new_config.database.max_connections = 20; // 增加连接数

    info!("新配置:");
    info!(
        "  - 端口: {} -> {}",
        config.server.port, new_config.server.port
    );
    info!(
        "  - new_ui: {} -> {}",
        config.feature.new_ui, new_config.feature.new_ui
    );
    info!(
        "  - max_connections: {} -> {}",
        config.database.max_connections, new_config.database.max_connections
    );

    // 根据策略启动重载
    match config.reload.strategy.as_str() {
        "canary" => {
            reloader.start_canary_reload(new_config).await?;
        }
        "linear" => {
            reloader.start_linear_reload(new_config).await?;
        }
        _ => {}
    }

    // 显示最终配置
    let final_config = reloader.get_config().await;
    info!("");
    info!("========================================");
    info!("  最终配置状态");
    info!("========================================");
    info!("  端口: {}", final_config.server.port);
    info!("  new_ui: {}", final_config.feature.new_ui);
    info!(
        "  max_connections: {}",
        final_config.database.max_connections
    );

    info!("");
    info!("示例程序执行完成！");

    Ok(())
}
