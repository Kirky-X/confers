// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! Validation Example - Configuration Validation with Garde
//!
//! 本示例展示如何使用 confers 的验证功能：
//! - 基于 garde 框架的字段级验证
//! - 内置验证规则（长度、范围、格式）
//! - 自定义验证逻辑
//! - 验证错误展示和处理
//! - 与配置加载集成
//!
//! 设计依据：ADR-010（验证框架集成）
//!
//! 运行方式：
//!   cargo run --bin validation

use confers::i18n::{tr, tr_args};
use confers::validator::ValidationRule;
use serde::Deserialize;

// 配置结构定义

/// 应用配置结构（带验证）
///
/// 使用 `#[derive(Config, Validate)]` 和 `#[config(validate)]` 启用验证。
/// garde 验证属性直接应用于字段。
///
/// # 示例
///
/// ```rust,ignore
/// #[derive(Config, Deserialize, Validate)]
/// #[config(validate)]
/// struct ConfersConfig {
///     #[garde(length(min = 1, max = 253))]
///     pub host: String,
///
///     #[garde(range(min = 1, max = 65535))]
///     pub port: u16,
/// }
/// ```
#[derive(Debug, Clone, Deserialize)]
pub struct ConfersConfig {
    /// 服务器主机地址
    pub host: String,

    /// 服务器端口
    pub port: u16,

    /// 最大连接数
    pub max_connections: u32,

    /// 日志级别
    pub log_level: String,

    /// 管理员邮箱
    pub admin_email: String,

    /// 数据库连接 URL
    pub database_url: String,

    /// 超时时间（秒）
    pub timeout_seconds: f64,
}

impl Default for ConfersConfig {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 8080,
            max_connections: 10,
            log_level: "info".to_string(),
            admin_email: "admin@example.com".to_string(),
            database_url: "postgresql://localhost:5432/app".to_string(),
            timeout_seconds: 30.0,
        }
    }
}

/// 服务器配置
#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    /// 主机地址
    pub host: String,

    /// 端口
    pub port: u16,

    /// TLS 启用
    pub tls_enabled: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 8443,
            tls_enabled: true,
        }
    }
}

/// 用户配置
#[derive(Debug, Clone, Deserialize)]
pub struct UserConfig {
    /// 用户名
    pub username: String,

    /// 邮箱
    pub email: String,

    /// 年龄
    pub age: u8,
}

impl Default for UserConfig {
    fn default() -> Self {
        Self {
            username: "admin".to_string(),
            email: "admin@localhost".to_string(),
            age: 25,
        }
    }
}

// 验证结果处理

/// 验证错误详情
#[derive(Debug, Clone)]
pub struct ValidationErrorDetail {
    /// 字段路径
    pub path: String,

    /// 错误消息
    pub message: String,

    /// 错误类型
    pub error_type: String,
}

impl std::fmt::Display for ValidationErrorDetail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}: {}", self.error_type, self.path, self.message)
    }
}

/// 验证报告（简化的错误展示）
#[derive(Debug)]
pub struct ValidationReport {
    /// 错误列表
    pub errors: Vec<ValidationErrorDetail>,
}

impl ValidationReport {
    /// 从 garde::Report 创建验证报告
    ///
    /// 注意：garde::Report 的迭代器返回 &(Path, Error) 元组。
    /// Path 只包含结构化路径信息（字段名、索引），不包含规则名。
    /// 这里使用路径中的字段名作为 error_type。
    pub fn from_garde_report(report: garde::Report) -> Self {
        let errors = report
            .iter()
            .map(|(path, error): &(garde::Path, garde::Error)| {
                let path_str = path.to_string();
                let error_type = path_str
                    .split('.')
                    .next_back()
                    .unwrap_or(&path_str)
                    .to_string();

                ValidationErrorDetail {
                    path: path_str,
                    message: error.message().to_string(),
                    error_type,
                }
            })
            .collect();

        Self { errors }
    }

    /// 从错误列表创建报告
    pub fn from_errors(errors: Vec<ValidationErrorDetail>) -> Self {
        Self { errors }
    }

    /// 检查是否有错误
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }

    /// 获取错误数量
    pub fn error_count(&self) -> usize {
        self.errors.len()
    }

    /// 打印验证报告
    pub fn print(&self) {
        if self.is_valid() {
            println!("  {}", tr("example-validation-passed"));
            return;
        }

        println!(
            "  {}",
            tr_args(
                "example-validation-failed-count",
                &[("count", self.error_count().to_string())]
            )
        );
        for (i, error) in self.errors.iter().enumerate() {
            println!("    {}. {}", i + 1, error);
        }
    }

    /// 以友好格式打印
    pub fn print_pretty(&self) {
        if self.is_valid() {
            println!("\n  {}", tr("example-validation-pretty-ok"));
            return;
        }

        println!("\n  {}", tr("example-validation-pretty-fail"));
        println!("  {}", "-".repeat(40));

        // 按错误类型分组
        let mut by_type: std::collections::HashMap<&str, Vec<&ValidationErrorDetail>> =
            std::collections::HashMap::new();

        for error in &self.errors {
            by_type.entry(&error.error_type).or_default().push(error);
        }

        for (error_type, errors) in by_type {
            println!("\n  {error_type}:");
            for error in errors {
                println!(
                    "    {}",
                    tr_args(
                        "example-validation-error-field",
                        &[("field", error.path.clone())]
                    )
                );
                println!(
                    "      {}",
                    tr_args(
                        "example-validation-error-reason",
                        &[("reason", error.message.clone())]
                    )
                );
            }
        }

        println!("\n  {}", "-".repeat(40));
        println!(
            "  {}",
            tr_args(
                "example-validation-total-errors",
                &[("count", self.error_count().to_string())]
            )
        );
    }
}

// 自定义验证器

/// 验证规则引擎
pub struct ValidationEngine;

impl ValidationEngine {
    /// 验证主机地址格式
    ///
    /// 接受：localhost, 127.0.0.1, 0.0.0.0, myhost.local
    /// 拒绝：空字符串，超过 253 字符
    pub fn validate_host(host: &str) -> Result<(), String> {
        if host.is_empty() {
            return Err(tr("example-validation-host-empty"));
        }

        if host.len() > 253 {
            return Err(tr_args(
                "example-validation-host-too-long",
                &[("length", host.len().to_string())],
            ));
        }

        // 简单的格式检查
        if host
            .chars()
            .any(|c| !c.is_ascii_alphanumeric() && c != '.' && c != '-' && c != ':')
        {
            return Err(tr_args(
                "example-validation-host-invalid-chars",
                &[("host", host.to_string())],
            ));
        }

        Ok(())
    }

    /// 验证端口范围
    pub fn validate_port(port: u16) -> Result<(), String> {
        if port == 0 {
            return Err(tr("example-validation-port-reserved"));
        }

        if port < 1024 {
            return Err(tr_args(
                "example-validation-port-privileged",
                &[("port", port.to_string())],
            ));
        }

        // Note: port > 65535 check is unnecessary since u16 max is 65535

        Ok(())
    }

    /// 验证日志级别
    pub fn validate_log_level(level: &str) -> Result<(), String> {
        let valid_levels = ["trace", "debug", "info", "warn", "error", "off"];

        if !valid_levels.contains(&level.to_lowercase().as_str()) {
            return Err(tr_args(
                "example-validation-log-level-invalid",
                &[
                    ("level", level.to_string()),
                    ("valid", format!("{valid_levels:?}")),
                ],
            ));
        }

        Ok(())
    }

    /// 验证邮箱格式
    pub fn validate_email(email: &str) -> Result<(), String> {
        if email.is_empty() {
            return Err(tr("example-validation-email-empty"));
        }

        // 简单的格式检查：必须包含 @
        if !email.contains('@') {
            return Err(tr("example-validation-email-missing-at"));
        }

        let parts: Vec<&str> = email.split('@').collect();
        if parts.len() != 2 {
            return Err(tr("example-validation-email-multiple-at"));
        }

        if parts[0].is_empty() {
            return Err(tr("example-validation-email-local-empty"));
        }

        if parts[1].is_empty() {
            return Err(tr("example-validation-email-domain-empty"));
        }

        if !parts[1].contains('.') {
            return Err(tr("example-validation-email-domain-dot"));
        }

        Ok(())
    }

    /// 验证超时时间
    pub fn validate_timeout(timeout: f64) -> Result<(), String> {
        if timeout <= 0.0 {
            return Err(tr("example-validation-timeout-positive"));
        }

        if timeout > 86400.0 {
            return Err(tr_args(
                "example-validation-timeout-too-long",
                &[("timeout", format!("{timeout:.1}"))],
            ));
        }

        if timeout < 1.0 {
            return Err(tr_args(
                "example-validation-timeout-too-short",
                &[("timeout", format!("{timeout:.1}"))],
            ));
        }

        Ok(())
    }

    /// 验证最大连接数
    pub fn validate_max_connections(connections: u32) -> Result<(), String> {
        if connections == 0 {
            return Err(tr("example-validation-max-connections-zero"));
        }

        if connections > 10000 {
            return Err(tr_args(
                "example-validation-max-connections-too-many",
                &[("connections", connections.to_string())],
            ));
        }

        Ok(())
    }

    /// 验证 URL 格式
    pub fn validate_url(url: &str) -> Result<(), String> {
        if url.is_empty() {
            return Err(tr("example-validation-url-empty"));
        }

        // 简单检查：必须有协议前缀
        let valid_prefixes = [
            "http://",
            "https://",
            "postgresql://",
            "mysql://",
            "redis://",
        ];
        if !valid_prefixes.iter().any(|p| url.starts_with(p)) {
            return Err(tr_args(
                "example-validation-url-scheme",
                &[("schemes", format!("{valid_prefixes:?}"))],
            ));
        }

        Ok(())
    }
}

// 验证上下文

/// 验证上下文配置
#[derive(Debug, Clone)]
pub struct ValidationContext {
    /// 是否严格模式（不允许任何警告）
    pub strict: bool,

    /// 是否显示详细信息
    pub verbose: bool,

    /// 忽略的字段列表
    pub ignored_fields: Vec<String>,
}

impl Default for ValidationContext {
    fn default() -> Self {
        Self {
            strict: false,
            verbose: true,
            ignored_fields: Vec::new(),
        }
    }
}

impl ValidationContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_strict(mut self, strict: bool) -> Self {
        self.strict = strict;
        self
    }

    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    pub fn ignore_field(mut self, field: impl Into<String>) -> Self {
        self.ignored_fields.push(field.into());
        self
    }

    /// 运行完整验证
    pub fn validate(&self, config: &ConfersConfig) -> ValidationReport {
        let mut errors = Vec::new();

        if !self.should_ignore("host")
            && let Err(msg) = ValidationEngine::validate_host(&config.host)
        {
            errors.push(ValidationErrorDetail {
                path: "host".to_string(),
                message: msg,
                error_type: "format".to_string(),
            });
        }

        if !self.should_ignore("port")
            && let Err(msg) = ValidationEngine::validate_port(config.port)
        {
            errors.push(ValidationErrorDetail {
                path: "port".to_string(),
                message: msg,
                error_type: "range".to_string(),
            });
        }

        if !self.should_ignore("log_level")
            && let Err(msg) = ValidationEngine::validate_log_level(&config.log_level)
        {
            errors.push(ValidationErrorDetail {
                path: "log_level".to_string(),
                message: msg,
                error_type: "enum".to_string(),
            });
        }

        if !self.should_ignore("admin_email")
            && let Err(msg) = ValidationEngine::validate_email(&config.admin_email)
        {
            errors.push(ValidationErrorDetail {
                path: "admin_email".to_string(),
                message: msg,
                error_type: "format".to_string(),
            });
        }

        if !self.should_ignore("database_url")
            && let Err(msg) = ValidationEngine::validate_url(&config.database_url)
        {
            errors.push(ValidationErrorDetail {
                path: "database_url".to_string(),
                message: msg,
                error_type: "format".to_string(),
            });
        }

        if !self.should_ignore("timeout_seconds")
            && let Err(msg) = ValidationEngine::validate_timeout(config.timeout_seconds)
        {
            errors.push(ValidationErrorDetail {
                path: "timeout_seconds".to_string(),
                message: msg,
                error_type: "range".to_string(),
            });
        }

        if !self.should_ignore("max_connections")
            && let Err(msg) = ValidationEngine::validate_max_connections(config.max_connections)
        {
            errors.push(ValidationErrorDetail {
                path: "max_connections".to_string(),
                message: msg,
                error_type: "range".to_string(),
            });
        }

        ValidationReport::from_errors(errors)
    }

    fn should_ignore(&self, field: &str) -> bool {
        self.ignored_fields.iter().any(|f| f == field)
    }
}

// 演示函数

/// 演示基础验证
fn demo_basic_validation() {
    println!("\n{}\n", tr("example-validation-demo1"));

    println!("{}", tr("example-validation-host-header"));
    for host in &["localhost", "127.0.0.1", "", "a".repeat(300).as_str()] {
        let result = ValidationEngine::validate_host(host);
        let status = if result.is_ok() { "OK" } else { "FAIL" };
        println!("  '{}': {}", host, status);
        if let Err(e) = result {
            println!(
                "    {}",
                tr_args("example-validation-item-error", &[("error", e)])
            );
        }
    }

    println!("\n{}", tr("example-validation-port-header"));
    for port in &[0u16, 22, 8080, 65535] {
        let result = ValidationEngine::validate_port(*port);
        let status = if result.is_ok() { "OK" } else { "FAIL" };
        println!("  {}: {}", port, status);
        if let Err(e) = result {
            println!(
                "    {}",
                tr_args("example-validation-item-warning", &[("error", e)])
            );
        }
    }

    println!("\n{}", tr("example-validation-log-level-header"));
    for level in &["debug", "INFO", "warning", "error", "invalid"] {
        let result = ValidationEngine::validate_log_level(level);
        let status = if result.is_ok() { "OK" } else { "FAIL" };
        println!("  '{}': {}", level, status);
        if let Err(e) = result {
            println!(
                "    {}",
                tr_args("example-validation-item-error", &[("error", e)])
            );
        }
    }
}

/// 演示邮箱和 URL 验证
fn demo_format_validation() {
    println!("\n{}\n", tr("example-validation-demo2"));

    println!("{}", tr("example-validation-email-header"));
    for email in &[
        "admin@example.com",
        "user.name+tag@domain.co.uk",
        "invalid-email",
        "@nodomain.com",
        "no-at-sign.com",
        "",
    ] {
        let result = ValidationEngine::validate_email(email);
        let status = if result.is_ok() { "OK" } else { "FAIL" };
        println!("  '{}': {}", email, status);
        if let Err(e) = result {
            println!(
                "    {}",
                tr_args("example-validation-item-error", &[("error", e)])
            );
        }
    }

    println!("\n{}", tr("example-validation-db-url-header"));
    for url in &[
        "postgresql://localhost:5432/app",
        "mysql://user:pass@host:3306/db",
        "redis://localhost:6379",
        "invalid-url",
        "",
    ] {
        let result = ValidationEngine::validate_url(url);
        let status = if result.is_ok() { "OK" } else { "FAIL" };
        println!("  '{}': {}", url, status);
        if let Err(e) = result {
            println!(
                "    {}",
                tr_args("example-validation-item-error", &[("error", e)])
            );
        }
    }
}

/// 演示验证上下文
fn demo_validation_context() {
    println!("\n{}\n", tr("example-validation-demo3"));

    let config = ConfersConfig {
        host: "".to_string(),
        port: 80,
        max_connections: 0,
        log_level: "invalid".to_string(),
        admin_email: "invalid".to_string(),
        database_url: "bad".to_string(),
        timeout_seconds: -1.0,
    };

    let ctx = ValidationContext::new();
    let report = ctx.validate(&config);
    println!("{}", tr("example-validation-default-context"));
    report.print_pretty();

    println!("\n{}", tr("example-validation-after-ignore"));
    let ctx = ValidationContext::new()
        .ignore_field("port")
        .ignore_field("timeout_seconds");
    let report = ctx.validate(&config);
    report.print_pretty();

    println!("\n{}", tr("example-validation-strict-mode"));
    let ctx = ValidationContext::new()
        .with_verbose(true)
        .with_strict(true);
    let report = ctx.validate(&config);
    if ctx.verbose {
        report.print_pretty();
    }
}

/// 演示有效配置验证
fn demo_valid_config() {
    println!("\n{}\n", tr("example-validation-demo4"));

    let config = ConfersConfig::default();

    println!("{}", tr("example-validation-validate-default"));
    println!("  host: {}", config.host);
    println!("  port: {}", config.port);
    println!("  max_connections: {}", config.max_connections);
    println!("  log_level: {}", config.log_level);
    println!("  admin_email: {}", config.admin_email);
    println!("  timeout_seconds: {}", config.timeout_seconds);

    let ctx = ValidationContext::new();
    let report = ctx.validate(&config);

    println!("\n{}", tr("example-validation-result-header"));
    report.print_pretty();
}

/// 演示规则解析
fn demo_rule_parsing() {
    println!("\n{}\n", tr("example-validation-demo5"));

    let rules = [
        "length(min=1, max=100)",
        "range(min=1, max=65535)",
        "email",
        "url",
        "ip",
        "invalid",
    ];

    for rule_str in &rules {
        match ValidationRule::from_str(rule_str) {
            Some(rule) => println!("  '{}' -> {:?}", rule_str, rule),
            None => println!(
                "  {}",
                tr_args(
                    "example-validation-unknown-rule",
                    &[("rule", rule_str.to_string())]
                )
            ),
        }
    }
}

/// 演示错误处理策略
fn demo_error_handling() {
    println!("\n{}\n", tr("example-validation-demo6"));

    let config = ConfersConfig {
        host: "".to_string(),
        port: 0,
        max_connections: 0,
        log_level: "invalid".to_string(),
        admin_email: "bad".to_string(),
        database_url: "".to_string(),
        timeout_seconds: 0.0,
    };

    let ctx = ValidationContext::new();
    let report = ctx.validate(&config);

    // 策略 1: 打印并退出
    println!("{}", tr("example-validation-strategy-1"));
    if !report.is_valid() {
        println!(
            "  {}",
            tr_args(
                "example-validation-detected-errors",
                &[("count", report.error_count().to_string())]
            )
        );
        for error in &report.errors {
            println!("    - {}", error);
        }
        println!("  {}", tr("example-validation-fix-and-restart"));
        return;
    }

    // 策略 2: 使用哨兵值继续运行
    println!("{}", tr("example-validation-strategy-2"));
}

/// 演示与 Config derive 集成
fn demo_config_integration() {
    println!("\n{}\n", tr("example-validation-demo7"));

    println!("{}", tr("example-validation-derive-intro"));
    println!();
    println!("  {}", tr("example-validation-method-1"));
    println!();
    println!("  ```rust");
    println!("  use confers::Config;");
    println!("  use garde::Validate;");
    println!();
    println!("  #[derive(Config, Deserialize, Validate)]");
    println!("  #[config(validate)]");
    println!("  struct ConfersConfig {{");
    println!("      #[garde(length(min = 1, max = 253))]");
    println!("      pub host: String,");
    println!();
    println!("      #[garde(range(min = 1, max = 65535))]");
    println!("      pub port: u16,");
    println!("  }}");
    println!("  ```");
    println!();
    println!("  {}", tr("example-validation-method-2"));
    println!();
    println!("  ```rust");
    println!("  let config = ConfersConfig::load_sync()?;");
    println!("  if let Err(report) = config.validate() {{");
    println!("      {}", tr("example-validation-validation-failed-print"));
    println!("      return Err(\"Invalid config\".into());");
    println!("  }}");
    println!("  ```");
    println!();
    println!("  {}", tr("example-validation-method-3"));
    println!();
    println!("  ```rust");
    println!("  let ctx = ValidationContext::new()");
    println!("      .ignore_field(\"optional_field\")");
    println!("      .with_strict(true);");
    println!();
    println!("  let report = ctx.validate(&config);");
    println!("  if !report.is_valid() {{");
    println!("      report.print_pretty();");
    println!("  }}");
    println!("  ```");
}

// 主程序

fn main() {
    println!("========================================");
    println!("  {}", tr("example-validation-title"));
    println!("========================================");

    demo_basic_validation();
    demo_format_validation();
    demo_validation_context();
    demo_valid_config();
    demo_rule_parsing();
    demo_error_handling();
    demo_config_integration();

    println!("\n========================================");
    println!("  {}", tr("example-validation-completed"));
    println!("========================================");
}
