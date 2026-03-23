//! Metrics Example - Configuration Metrics Collection
//!
//! 本示例展示如何使用 confers 的指标收集功能：
//! - 配置操作指标收集
//! - Prometheus exporter 集成
//! - 自定义指标注册和记录
//! - 带标签的指标（labels/tags）
//! - 指标查询和展示
//!
//! 设计依据：ADR-014（指标与可观测性）
//!
//! 运行方式：
//!   cargo run --bin metrics
//!   # 在另一个终端启动 Prometheus 并抓取指标:
//   # prometheus --config.file=prometheus.yml

use serde::Deserialize;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Type alias for complex metric data storage to reduce type complexity
type MetricDataKey = (String, Vec<(String, String)>);
type MetricDataStore = HashMap<MetricDataKey, (f64, u64)>;

// =============================================================================
// 配置结构定义
// =============================================================================

/// 应用配置
#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub name: String,
    pub port: u16,
    pub workers: usize,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            name: "myapp".to_string(),
            port: 8080,
            workers: 4,
        }
    }
}

/// 服务器指标
#[derive(Debug, Default)]
pub struct ServerMetrics {
    /// 当前连接数
    pub connections: AtomicU64,

    /// 总请求数
    pub total_requests: AtomicU64,

    /// 错误请求数
    pub error_requests: AtomicU64,

    /// 总响应时间（毫秒）
    pub total_response_time_ms: AtomicU64,
}

impl ServerMetrics {
    pub fn new() -> Self {
        Self {
            connections: AtomicU64::new(0),
            total_requests: AtomicU64::new(0),
            error_requests: AtomicU64::new(0),
            total_response_time_ms: AtomicU64::new(0),
        }
    }
}

/// 配置加载指标
#[derive(Debug, Default)]
pub struct ConfigMetrics {
    /// 加载次数
    pub load_count: AtomicU64,

    /// 加载失败次数
    pub load_failure_count: AtomicU64,

    /// 最后加载时间戳
    pub last_load_timestamp: std::sync::Mutex<Option<u64>>,

    /// 配置版本
    pub config_version: std::sync::Mutex<Option<u32>>,
}

impl ConfigMetrics {
    pub fn new() -> Self {
        Self {
            load_count: AtomicU64::new(0),
            load_failure_count: AtomicU64::new(0),
            last_load_timestamp: std::sync::Mutex::new(None),
            config_version: std::sync::Mutex::new(None),
        }
    }
}

/// 应用指标收集器
#[derive(Debug)]
pub struct MetricsCollector {
    /// 服务器指标
    pub server: Arc<ServerMetrics>,

    /// 配置指标
    pub config: Arc<ConfigMetrics>,

    /// 自定义指标
    pub custom: Arc<CustomMetrics>,

    /// 指标注册表
    registry: Arc<MetricsRegistry>,
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl MetricsCollector {
    /// 创建新的指标收集器
    pub fn new() -> Self {
        Self {
            server: Arc::new(ServerMetrics::new()),
            config: Arc::new(ConfigMetrics::new()),
            custom: Arc::new(CustomMetrics::default()),
            registry: Arc::new(MetricsRegistry::new()),
        }
    }

    /// 记录请求
    pub fn record_request(&self, response_time_ms: u64, is_error: bool) {
        self.server.total_requests.fetch_add(1, Ordering::Relaxed);
        self.server
            .total_response_time_ms
            .fetch_add(response_time_ms, Ordering::Relaxed);

        if is_error {
            self.server.error_requests.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// 记录连接
    pub fn record_connection(&self, delta: i64) {
        if delta > 0 {
            self.server
                .connections
                .fetch_add(delta as u64, Ordering::Relaxed);
        } else {
            let current = self.server.connections.load(Ordering::Relaxed);
            let new_val = current.saturating_sub((-delta) as u64);
            self.server.connections.store(new_val, Ordering::Relaxed);
        }
    }

    /// 记录配置加载
    pub fn record_config_load(&self, success: bool, version: Option<u32>) {
        if success {
            self.config.load_count.fetch_add(1, Ordering::Relaxed);
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            *self.config.last_load_timestamp.lock().unwrap() = Some(timestamp);
            if let Some(v) = version {
                *self.config.config_version.lock().unwrap() = Some(v);
            }
        } else {
            self.config
                .load_failure_count
                .fetch_add(1, Ordering::Relaxed);
        }
    }

    /// 记录自定义指标
    pub fn record_custom(&self, name: &str, value: f64, labels: &[(&str, &str)]) {
        self.custom.record(name, value, labels);
    }

    /// 获取指标注册表
    pub fn registry(&self) -> &MetricsRegistry {
        &self.registry
    }

    /// 获取所有指标的 Prometheus 格式输出
    pub fn to_prometheus_format(&self) -> String {
        let mut output = String::new();

        // 服务器指标
        output.push_str("# HELP server_connections Current number of connections\n");
        output.push_str("# TYPE server_connections gauge\n");
        output.push_str(&format!(
            "server_connections {}\n",
            self.server.connections.load(Ordering::Relaxed)
        ));

        output.push_str("# HELP server_total_requests Total number of requests\n");
        output.push_str("# TYPE server_total_requests counter\n");
        output.push_str(&format!(
            "server_total_requests {}\n",
            self.server.total_requests.load(Ordering::Relaxed)
        ));

        output.push_str("# HELP server_error_requests Total number of error requests\n");
        output.push_str("# TYPE server_error_requests counter\n");
        output.push_str(&format!(
            "server_error_requests {}\n",
            self.server.error_requests.load(Ordering::Relaxed)
        ));

        output.push_str("# HELP server_response_time_ms Total response time in ms\n");
        output.push_str("# TYPE server_response_time_ms counter\n");
        output.push_str(&format!(
            "server_response_time_ms {}\n",
            self.server.total_response_time_ms.load(Ordering::Relaxed)
        ));

        // 计算平均响应时间
        let total = self.server.total_requests.load(Ordering::Relaxed);
        let total_time = self.server.total_response_time_ms.load(Ordering::Relaxed);
        let avg_time = if total > 0 {
            total_time as f64 / total as f64
        } else {
            0.0
        };
        output.push_str("# HELP server_avg_response_time_ms Average response time in ms\n");
        output.push_str("# TYPE server_avg_response_time_ms gauge\n");
        output.push_str(&format!("server_avg_response_time_ms {}\n", avg_time));

        // 配置指标
        output.push_str("# HELP config_load_count Configuration load count\n");
        output.push_str("# TYPE config_load_count counter\n");
        output.push_str(&format!(
            "config_load_count {}\n",
            self.config.load_count.load(Ordering::Relaxed)
        ));

        output.push_str("# HELP config_load_failure_count Configuration load failure count\n");
        output.push_str("# TYPE config_load_failure_count counter\n");
        output.push_str(&format!(
            "config_load_failure_count {}\n",
            self.config.load_failure_count.load(Ordering::Relaxed)
        ));

        // 自定义指标
        output.push_str(&self.custom.to_prometheus_format());

        output
    }

    /// 获取指标摘要
    pub fn summary(&self) -> MetricsSummary {
        let total = self.server.total_requests.load(Ordering::Relaxed);
        let errors = self.server.error_requests.load(Ordering::Relaxed);
        let total_time = self.server.total_response_time_ms.load(Ordering::Relaxed);

        MetricsSummary {
            connections: self.server.connections.load(Ordering::Relaxed),
            total_requests: total,
            error_requests: errors,
            error_rate: if total > 0 {
                errors as f64 / total as f64 * 100.0
            } else {
                0.0
            },
            avg_response_time_ms: if total > 0 {
                total_time as f64 / total as f64
            } else {
                0.0
            },
            config_loads: self.config.load_count.load(Ordering::Relaxed),
            config_failures: self.config.load_failure_count.load(Ordering::Relaxed),
        }
    }
}

/// 指标摘要
#[derive(Debug)]
pub struct MetricsSummary {
    pub connections: u64,
    pub total_requests: u64,
    pub error_requests: u64,
    pub error_rate: f64,
    pub avg_response_time_ms: f64,
    pub config_loads: u64,
    pub config_failures: u64,
}

/// 自定义指标存储
#[derive(Debug, Clone, Default)]
pub struct CustomMetrics {
    /// 指标数据: (name, labels) -> (value, timestamp)
    data: Arc<std::sync::Mutex<MetricDataStore>>,

    /// 指标元数据
    metadata: Arc<std::sync::Mutex<HashMap<String, MetricMetadata>>>,
}

impl CustomMetrics {
    /// 记录自定义指标
    pub fn record(&self, name: &str, value: f64, labels: &[(&str, &str)]) {
        let key = (
            name.to_string(),
            labels
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        );

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        self.data.lock().unwrap().insert(key, (value, timestamp));

        // 更新元数据
        let mut meta = self.metadata.lock().unwrap();
        meta.entry(name.to_string())
            .or_insert_with(|| MetricMetadata {
                name: name.to_string(),
                metric_type: MetricType::Gauge,
                description: String::new(),
                unit: String::new(),
            });
    }

    /// 递增计数器
    pub fn increment(&self, name: &str, labels: &[(&str, &str)], delta: f64) {
        let key = (
            name.to_string(),
            labels
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        );

        let mut data = self.data.lock().unwrap();
        let (current, _) = data.get(&key).copied().unwrap_or((0.0, 0));
        data.insert(key, (current + delta, 0));
    }

    /// 转换为 Prometheus 格式
    pub fn to_prometheus_format(&self) -> String {
        let mut output = String::new();
        let data = self.data.lock().unwrap();

        for ((name, labels), (value, _)) in data.iter() {
            let labels_str = if labels.is_empty() {
                String::new()
            } else {
                format!(
                    "{{{}}}",
                    labels
                        .iter()
                        .map(|(k, v)| format!("{}=\"{}\"", k, v))
                        .collect::<Vec<_>>()
                        .join(",")
                )
            };

            let safe_name: String = name.replace(['.', '-'], "_");
            output.push_str(&format!("# HELP {} Custom metric\n", safe_name));
            output.push_str(&format!("# TYPE {} gauge\n", safe_name));
            output.push_str(&format!("{}{} {}\n", safe_name, labels_str, value));
        }

        output
    }
}

/// 指标元数据
#[derive(Debug, Clone)]
pub struct MetricMetadata {
    pub name: String,
    pub metric_type: MetricType,
    pub description: String,
    pub unit: String,
}

/// 指标类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetricType {
    Counter,
    Gauge,
    Histogram,
    Summary,
}

/// 指标注册表
#[derive(Debug, Default)]
pub struct MetricsRegistry {
    /// 注册的指标
    metrics: std::sync::Mutex<Vec<RegisteredMetric>>,
}

#[derive(Debug, Clone)]
pub struct RegisteredMetric {
    pub name: String,
    pub metric_type: MetricType,
    pub description: String,
    pub labels: Vec<String>,
}

impl MetricsRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// 注册新指标
    pub fn register(&self, metric: RegisteredMetric) {
        self.metrics.lock().unwrap().push(metric);
    }

    /// 获取所有注册的指标
    pub fn list(&self) -> Vec<RegisteredMetric> {
        self.metrics.lock().unwrap().clone()
    }
}

// =============================================================================
// Prometheus Exporter
// =============================================================================

/// Prometheus 指标端点处理器
pub struct PrometheusExporter {
    /// 指标收集器
    collector: MetricsCollector,

    /// 端点路径
    path: String,
}

impl PrometheusExporter {
    /// 创建新的 Exporter
    pub fn new(collector: MetricsCollector) -> Self {
        Self {
            collector,
            path: "/metrics".to_string(),
        }
    }

    /// 获取端点路径
    pub fn path(&self) -> &str {
        &self.path
    }

    /// 处理请求并返回指标
    pub fn handle_request(&self) -> String {
        self.collector.to_prometheus_format()
    }

    /// 获取 Content-Type
    pub fn content_type(&self) -> &str {
        "text/plain; version=0.0.4; charset=utf-8"
    }
}

// =============================================================================
// 演示函数
// =============================================================================

/// 演示基础指标收集
fn demo_basic_metrics() {
    println!("\n=== 演示 1: 基础指标收集 ===\n");

    let collector = MetricsCollector::new();

    // 模拟请求
    let requests = [
        (50, false),
        (120, false),
        (80, false),
        (200, true),
        (45, false),
        (300, true),
        (60, false),
        (90, false),
    ];

    println!("模拟请求处理:");
    for (i, (time_ms, is_error)) in requests.iter().enumerate() {
        collector.record_request(*time_ms, *is_error);
        println!(
            "  请求 {}: {}ms, error={}",
            i + 1,
            time_ms,
            if *is_error { "是" } else { "否" }
        );
    }

    // 模拟连接变化
    collector.record_connection(10);
    println!("\n建立 10 个连接");
    collector.record_connection(-3);
    println!("关闭 3 个连接");

    let summary = collector.summary();
    println!("\n指标摘要:");
    println!("  当前连接数: {}", summary.connections);
    println!("  总请求数: {}", summary.total_requests);
    println!("  错误请求数: {}", summary.error_requests);
    println!("  错误率: {:.2}%", summary.error_rate);
    println!("  平均响应时间: {:.2}ms", summary.avg_response_time_ms);

    println!("\nPrometheus 格式输出 (部分):");
    let output = collector.to_prometheus_format();
    for line in output.lines().take(10) {
        println!("  {}", line);
    }
}

/// 演示带标签的指标
fn demo_labeled_metrics() {
    println!("\n=== 演示 2: 带标签的指标 ===\n");

    let collector = MetricsCollector::new();

    // 按端点记录请求
    let endpoints = vec![
        ("/api/users", 100, false),
        ("/api/users", 150, false),
        ("/api/users", 500, true),
        ("/api/products", 80, false),
        ("/api/products", 90, false),
        ("/api/orders", 200, false),
        ("/api/orders", 300, true),
        ("/api/orders", 250, false),
    ];

    println!("按端点记录请求:");
    for (endpoint, time_ms, is_error) in &endpoints {
        collector.record_custom(
            "http_request_duration_ms",
            *time_ms as f64,
            &[
                ("endpoint", endpoint),
                ("status", if *is_error { "error" } else { "ok" }),
            ],
        );

        if *is_error {
            collector.record_custom(
                "http_request_total",
                1.0,
                &[("endpoint", endpoint), ("status", "error")],
            );
        }
    }

    println!("\n带标签的指标 (Prometheus 格式):");
    let output = collector.to_prometheus_format();
    for line in output.lines().skip_while(|l| !l.contains("http_")) {
        println!("  {}", line);
    }
}

/// 演示配置加载指标
fn demo_config_metrics() {
    println!("\n=== 演示 3: 配置加载指标 ===\n");

    let collector = MetricsCollector::new();

    // 模拟多次配置加载
    let load_results = [
        (true, Some(1)),
        (true, Some(2)),
        (false, None),
        (true, Some(3)),
        (true, Some(3)),
    ];

    println!("模拟配置加载:");
    for (i, (success, version)) in load_results.iter().enumerate() {
        collector.record_config_load(*success, *version);
        let status = if *success {
            format!("成功 (v{:?})", version)
        } else {
            "失败".to_string()
        };
        println!("  加载 {}: {}", i + 1, status);
    }

    let summary = collector.summary();
    println!("\n配置指标摘要:");
    println!("  成功加载次数: {}", summary.config_loads);
    println!("  加载失败次数: {}", summary.config_failures);
    let success_rate = if summary.config_loads + summary.config_failures > 0 {
        summary.config_loads as f64 / (summary.config_loads + summary.config_failures) as f64
            * 100.0
    } else {
        0.0
    };
    println!("  成功率: {:.1}%", success_rate);
}

/// 演示 Prometheus Exporter
fn demo_prometheus_exporter() {
    println!("\n=== 演示 4: Prometheus Exporter ===\n");

    let collector = MetricsCollector::new();

    // 模拟一些请求
    for i in 0..20 {
        let time_ms = 50 + (i * 5) % 200;
        let is_error = i % 7 == 0;
        collector.record_request(time_ms, is_error);
    }
    collector.record_connection(5);

    let exporter = PrometheusExporter::new(collector);

    println!("Prometheus Exporter 配置:");
    println!("  端点路径: {}", exporter.path());
    println!("  Content-Type: {}", exporter.content_type());
    println!();

    println!("完整 Prometheus 格式输出:");
    println!("{}", exporter.handle_request());
}

/// 演示自定义指标注册
fn demo_custom_metrics_registration() {
    println!("\n=== 演示 5: 自定义指标注册 ===\n");

    let collector = MetricsCollector::new();

    // 注册自定义指标
    let registry = collector.registry();
    registry.register(RegisteredMetric {
        name: "app_active_users".to_string(),
        metric_type: MetricType::Gauge,
        description: "当前活跃用户数".to_string(),
        labels: vec!["region".to_string()],
    });

    registry.register(RegisteredMetric {
        name: "queue_depth".to_string(),
        metric_type: MetricType::Gauge,
        description: "消息队列深度".to_string(),
        labels: vec!["queue_name".to_string()],
    });

    // 记录自定义指标
    collector.record_custom("app_active_users", 150.0, &[("region", "us-east")]);
    collector.record_custom("app_active_users", 80.0, &[("region", "eu-west")]);
    collector.record_custom("queue_depth", 42.0, &[("queue_name", "tasks")]);
    collector.record_custom("queue_depth", 15.0, &[("queue_name", "events")]);

    println!("注册的指标:");
    for metric in registry.list() {
        println!(
            "  - {} ({:?}): {}",
            metric.name, metric.metric_type, metric.description
        );
        if !metric.labels.is_empty() {
            println!("    标签: {:?}", metric.labels);
        }
    }

    println!("\n记录的指标值:");
    println!("{}", collector.to_prometheus_format());
}

/// 演示指标类型
fn demo_metric_types() {
    println!("\n=== 演示 6: 指标类型 ===\n");

    let collector = MetricsCollector::new();

    // Counter: 只增不减
    println!("Counter (计数器):");
    println!("  - 用于累积值，如总请求数");
    println!("  - 只增不减");
    collector.record_custom("counter.example", 1.0, &[]);
    collector.record_custom("counter.example", 1.0, &[]);
    collector.record_custom("counter.example", 1.0, &[]);

    // Gauge: 可增可减
    println!("\nGauge (仪表):");
    println!("  - 用于当前值，如连接数");
    println!("  - 可增可减");
    collector.record_custom("gauge.example", 100.0, &[]);
    collector.record_custom("gauge.example", 80.0, &[]);
    collector.record_custom("gauge.example", 120.0, &[]);

    // Histogram: 直方图
    println!("\nHistogram (直方图):");
    println!("  - 用于分布统计，如响应时间分布");
    println!("  - 自动计算百分位数");

    let buckets = vec![10.0, 50.0, 100.0, 200.0, 500.0, 1000.0];
    for bucket in &buckets {
        collector.record_custom("histogram.example", *bucket, &[("le", &bucket.to_string())]);
    }

    // Summary: 摘要
    println!("\nSummary (摘要):");
    println!("  - 用于百分位数，如 p50, p95, p99");
    println!("  - 服务端计算");

    let percentiles = vec![0.5, 0.9, 0.95, 0.99];
    for p in &percentiles {
        collector.record_custom(
            "summary.example",
            100.0 * p,
            &[("quantile", &format!("{}", p))],
        );
    }

    println!("\n所有自定义指标:");
    println!("{}", collector.to_prometheus_format());
}

/// 演示与 Prometheus 集成
fn demo_prometheus_integration() {
    println!("\n=== 演示 7: Prometheus 集成 ===\n");

    let collector = MetricsCollector::new();

    // 模拟应用运行
    println!("模拟应用指标收集:");
    for i in 0..100 {
        let time_ms = 50 + (i % 50) as u64;
        let is_error = i % 20 == 0;
        collector.record_request(time_ms, is_error);
    }
    collector.record_connection(50);
    collector.record_config_load(true, Some(1));

    // 生成 Prometheus 配置
    println!("\nPrometheus scrape 配置:");
    println!("```yaml");
    println!("scrape_configs:");
    println!("  - job_name: 'myapp'");
    println!("    static_configs:");
    println!("      - targets: ['localhost:8080']");
    println!("    metrics_path: '/metrics'");
    println!("    scrape_interval: 15s");
    println!("```");

    // 生成 Grafana 仪表板 JSON
    println!("\nGrafana 面板配置:");
    println!("{{");
    println!("  \"title\": \"MyApp Overview\",");
    println!("  \"panels\": [");
    println!("    {{");
    println!("      \"title\": \"Request Rate\",");
    println!("      \"metric\": \"server_total_requests\"");
    println!("    }},");
    println!("    {{");
    println!("      \"title\": \"Error Rate\",");
    println!("      \"expr\": \"rate(server_error_requests[5m])\"");
    println!("    }},");
    println!("    {{");
    println!("      \"title\": \"Response Time\",");
    println!("      \"expr\": \"server_avg_response_time_ms\"");
    println!("    }}");
    println!("  ]");
    println!("}}");
}

/// 演示指标聚合和查询
fn demo_metrics_aggregation() {
    println!("\n=== 演示 8: 指标聚合和查询 ===\n");

    let collector = MetricsCollector::new();

    // 模拟多维度指标
    let scenarios = vec![
        ("/api/v1/users", "GET", 100, false),
        ("/api/v1/users", "POST", 200, false),
        ("/api/v1/users", "DELETE", 50, true),
        ("/api/v2/products", "GET", 80, false),
        ("/api/v2/products", "PUT", 150, false),
        ("/health", "GET", 5, false),
    ];

    for (endpoint, method, time_ms, is_error) in scenarios {
        collector.record_custom(
            "http_request_duration_ms",
            time_ms as f64,
            &[("endpoint", endpoint), ("method", method)],
        );
        if is_error {
            collector.record_custom(
                "http_request_errors_total",
                1.0,
                &[("endpoint", endpoint), ("method", method)],
            );
        }
    }

    println!("多维度指标查询示例 (Prometheus):");
    println!();
    println!("  # 按端点查询总请求时间");
    println!("  sum by (endpoint) (http_request_duration_ms)");
    println!();
    println!("  # 按方法查询平均响应时间");
    println!("  avg by (method) (http_request_duration_ms)");
    println!();
    println!("  # 错误率");
    println!("  sum(http_request_errors_total) / sum(http_request_duration_ms) * 100");
    println!();
    println!("  # P95 响应时间");
    println!("  histogram_quantile(0.95, rate(http_request_duration_ms_bucket[5m]))");
}

// =============================================================================
// 主程序
// =============================================================================

fn main() {
    println!("========================================");
    println!("  Metrics Example");
    println!("  Configuration Metrics Collection");
    println!("========================================");

    demo_basic_metrics();
    demo_labeled_metrics();
    demo_config_metrics();
    demo_prometheus_exporter();
    demo_custom_metrics_registration();
    demo_metric_types();
    demo_prometheus_integration();
    demo_metrics_aggregation();

    println!("\n========================================");
    println!("  示例运行完成");
    println!("========================================");
}
