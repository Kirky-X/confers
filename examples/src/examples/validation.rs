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

use confers::validator::ValidationRule;
use serde::Deserialize;

// =============================================================================
// 配置结构定义
// =============================================================================

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

// =============================================================================
// 验证结果处理
// =============================================================================

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
                // 从路径中提取字段名作为错误类型
                // 使用 path 的 Display 表示，然后提取最后一个字段名
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
            println!("  Validation passed");
            return;
        }

        println!("  Validation failed ({} errors):", self.error_count());
        for (i, error) in self.errors.iter().enumerate() {
            println!("    {}. {}", i + 1, error);
        }
    }

    /// 以友好格式打印
    pub fn print_pretty(&self) {
        if self.is_valid() {
            println!("\n  [OK] Configuration validation passed");
            return;
        }

        println!("\n  [FAIL] Configuration validation failed");
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
                println!("    - field: {}", error.path);
                println!("      reason: {}", error.message);
            }
        }

        println!("\n  {}", "-".repeat(40));
        println!("  Total {} validation errors", self.error_count());
    }
}

// =============================================================================
// 自定义验证器
// =============================================================================

/// 验证规则引擎
pub struct ValidationEngine;

impl ValidationEngine {
    /// 验证主机地址格式
    ///
    /// 接受：localhost, 127.0.0.1, 0.0.0.0, myhost.local
    /// 拒绝：空字符串，超过 253 字符
    pub fn validate_host(host: &str) -> Result<(), String> {
        if host.is_empty() {
            return Err("host must not be empty".to_string());
        }

        if host.len() > 253 {
            return Err(format!(
                "host exceeds max length of 253 characters (current: {})",
                host.len()
            ));
        }

        // 简单的格式检查
        if host
            .chars()
            .any(|c| !c.is_ascii_alphanumeric() && c != '.' && c != '-' && c != ':')
        {
            return Err(format!("host contains invalid characters: {}", host));
        }

        Ok(())
    }

    /// 验证端口范围
    pub fn validate_port(port: u16) -> Result<(), String> {
        if port == 0 {
            return Err("port must not be 0 (reserved)".to_string());
        }

        if port < 1024 {
            return Err(format!(
                "port {} is a privileged port (< 1024), consider using an unprivileged port",
                port
            ));
        }

        // Note: port > 65535 check is unnecessary since u16 max is 65535

        Ok(())
    }

    /// 验证日志级别
    pub fn validate_log_level(level: &str) -> Result<(), String> {
        let valid_levels = ["trace", "debug", "info", "warn", "error", "off"];

        if !valid_levels.contains(&level.to_lowercase().as_str()) {
            return Err(format!(
                "invalid log level: '{}' (valid values: {:?})",
                level, valid_levels
            ));
        }

        Ok(())
    }

    /// 验证邮箱格式
    pub fn validate_email(email: &str) -> Result<(), String> {
        if email.is_empty() {
            return Err("email must not be empty".to_string());
        }

        // 简单的格式检查：必须包含 @
        if !email.contains('@') {
            return Err("invalid email format: missing @ symbol".to_string());
        }

        let parts: Vec<&str> = email.split('@').collect();
        if parts.len() != 2 {
            return Err("invalid email format: multiple @ symbols".to_string());
        }

        if parts[0].is_empty() {
            return Err("invalid email format: local part is empty".to_string());
        }

        if parts[1].is_empty() {
            return Err("invalid email format: domain part is empty".to_string());
        }

        // 检查域名部分
        if !parts[1].contains('.') {
            return Err("invalid email format: domain must contain a dot".to_string());
        }

        Ok(())
    }

    /// 验证超时时间
    pub fn validate_timeout(timeout: f64) -> Result<(), String> {
        if timeout <= 0.0 {
            return Err("timeout must be greater than 0".to_string());
        }

        if timeout > 86400.0 {
            return Err(format!(
                "timeout too long: {:.1}s (max: 86400s = 24h)",
                timeout
            ));
        }

        if timeout < 1.0 {
            return Err(format!("timeout too short: {:.1}s (min: 1s)", timeout));
        }

        Ok(())
    }

    /// 验证最大连接数
    pub fn validate_max_connections(connections: u32) -> Result<(), String> {
        if connections == 0 {
            return Err("max connections must not be 0".to_string());
        }

        if connections > 10000 {
            return Err(format!(
                "too many max connections: {} (recommended: <= 10000)",
                connections
            ));
        }

        Ok(())
    }

    /// 验证 URL 格式
    pub fn validate_url(url: &str) -> Result<(), String> {
        if url.is_empty() {
            return Err("URL must not be empty".to_string());
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
            return Err(format!(
                "URL must start with a valid scheme ({:?})",
                valid_prefixes
            ));
        }

        Ok(())
    }
}

// =============================================================================
// 验证上下文
// =============================================================================

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

        // 主机验证
        if !self.should_ignore("host")
            && let Err(msg) = ValidationEngine::validate_host(&config.host)
        {
            errors.push(ValidationErrorDetail {
                path: "host".to_string(),
                message: msg,
                error_type: "format".to_string(),
            });
        }

        // 端口验证
        if !self.should_ignore("port")
            && let Err(msg) = ValidationEngine::validate_port(config.port)
        {
            errors.push(ValidationErrorDetail {
                path: "port".to_string(),
                message: msg,
                error_type: "range".to_string(),
            });
        }

        // 日志级别验证
        if !self.should_ignore("log_level")
            && let Err(msg) = ValidationEngine::validate_log_level(&config.log_level)
        {
            errors.push(ValidationErrorDetail {
                path: "log_level".to_string(),
                message: msg,
                error_type: "enum".to_string(),
            });
        }

        // 邮箱验证
        if !self.should_ignore("admin_email")
            && let Err(msg) = ValidationEngine::validate_email(&config.admin_email)
        {
            errors.push(ValidationErrorDetail {
                path: "admin_email".to_string(),
                message: msg,
                error_type: "format".to_string(),
            });
        }

        // URL 验证
        if !self.should_ignore("database_url")
            && let Err(msg) = ValidationEngine::validate_url(&config.database_url)
        {
            errors.push(ValidationErrorDetail {
                path: "database_url".to_string(),
                message: msg,
                error_type: "format".to_string(),
            });
        }

        // 超时验证
        if !self.should_ignore("timeout_seconds")
            && let Err(msg) = ValidationEngine::validate_timeout(config.timeout_seconds)
        {
            errors.push(ValidationErrorDetail {
                path: "timeout_seconds".to_string(),
                message: msg,
                error_type: "range".to_string(),
            });
        }

        // 最大连接数验证
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

// =============================================================================
// 演示函数
// =============================================================================

/// 演示基础验证
fn demo_basic_validation() {
    println!("\n=== Demo 1: Basic field validation ===\n");

    // 测试主机验证
    println!("Host address validation:");
    for host in &["localhost", "127.0.0.1", "", "a".repeat(300).as_str()] {
        let result = ValidationEngine::validate_host(host);
        let status = if result.is_ok() { "OK" } else { "FAIL" };
        println!("  '{}': {}", host, status);
        if let Err(e) = result {
            println!("    error: {}", e);
        }
    }

    // 测试端口验证
    println!("\nPort range validation:");
    for port in &[0u16, 22, 8080, 65535] {
        let result = ValidationEngine::validate_port(*port);
        let status = if result.is_ok() { "OK" } else { "FAIL" };
        println!("  {}: {}", port, status);
        if let Err(e) = result {
            println!("    warning: {}", e);
        }
    }

    // 测试日志级别验证
    println!("\nLog level validation:");
    for level in &["debug", "INFO", "warning", "error", "invalid"] {
        let result = ValidationEngine::validate_log_level(level);
        let status = if result.is_ok() { "OK" } else { "FAIL" };
        println!("  '{}': {}", level, status);
        if let Err(e) = result {
            println!("    error: {}", e);
        }
    }
}

/// 演示邮箱和 URL 验证
fn demo_format_validation() {
    println!("\n=== Demo 2: Format validation ===\n");

    // 邮箱验证
    println!("Email format validation:");
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
            println!("    error: {}", e);
        }
    }

    // URL 验证
    println!("\nDatabase URL validation:");
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
            println!("    error: {}", e);
        }
    }
}

/// 演示验证上下文
fn demo_validation_context() {
    println!("\n=== Demo 3: Validation context ===\n");

    let config = ConfersConfig {
        host: "".to_string(),
        port: 80,
        max_connections: 0,
        log_level: "invalid".to_string(),
        admin_email: "invalid".to_string(),
        database_url: "bad".to_string(),
        timeout_seconds: -1.0,
    };

    // 默认上下文
    let ctx = ValidationContext::new();
    let report = ctx.validate(&config);
    println!("Default context validation result:");
    report.print_pretty();

    // 忽略特定字段
    println!("\nAfter ignoring 'port' and 'timeout_seconds':");
    let ctx = ValidationContext::new()
        .ignore_field("port")
        .ignore_field("timeout_seconds");
    let report = ctx.validate(&config);
    report.print_pretty();

    // 严格模式
    println!("\nStrict mode (show details):");
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
    println!("\n=== Demo 4: Valid configuration validation ===\n");

    let config = ConfersConfig::default();

    println!("Validate default configuration:");
    println!("  host: {}", config.host);
    println!("  port: {}", config.port);
    println!("  max_connections: {}", config.max_connections);
    println!("  log_level: {}", config.log_level);
    println!("  admin_email: {}", config.admin_email);
    println!("  timeout_seconds: {}", config.timeout_seconds);

    let ctx = ValidationContext::new();
    let report = ctx.validate(&config);

    println!("\nValidation result:");
    report.print_pretty();
}

/// 演示规则解析
fn demo_rule_parsing() {
    println!("\n=== Demo 5: Validation rule parsing ===\n");

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
            None => println!("  '{}' -> (unknown rule)", rule_str),
        }
    }
}

/// 演示错误处理策略
fn demo_error_handling() {
    println!("\n=== Demo 6: Error handling strategies ===\n");

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
    println!("Strategy 1: exit the program if validation fails");
    if !report.is_valid() {
        println!(
            "  Detected {} errors; the application cannot start",
            report.error_count()
        );
        for error in &report.errors {
            println!("    - {}", error);
        }
        println!("  Fix the configuration and restart");
        return;
    }

    // 策略 2: 使用哨兵值继续运行
    println!("Strategy 2: use sentinel values to run with warnings");
}

/// 演示与 Config derive 集成
fn demo_config_integration() {
    println!("\n=== Demo 7: Integration with Config Derive ===\n");

    println!("ConfigDerive validation integration:");
    println!();
    println!("  Method 1: automatic validation with #[config(validate)]");
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
    println!("  Method 2: call validate() manually");
    println!();
    println!("  ```rust");
    println!("  let config = ConfersConfig::load_sync()?;");
    println!("  if let Err(report) = config.validate() {{");
    println!("      eprintln!(\"Validation failed: {{:?}}\", report);");
    println!("      return Err(\"Invalid config\".into());");
    println!("  }}");
    println!("  ```");
    println!();
    println!("  Method 3: custom validation with ValidationContext");
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

// =============================================================================
// 主程序
// =============================================================================

fn main() {
    println!("========================================");
    println!("  Validation Example - config validation");
    println!("========================================");

    demo_basic_validation();
    demo_format_validation();
    demo_validation_context();
    demo_valid_config();
    demo_rule_parsing();
    demo_error_handling();
    demo_config_integration();

    println!("\n========================================");
    println!("  Example completed");
    println!("========================================");
}
