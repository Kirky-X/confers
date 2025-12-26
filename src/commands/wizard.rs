use crate::error::ConfigError;
use std::io::{self, Write};

type ValidationFn = Box<dyn Fn(&str) -> bool + Send>;

/// Wizard step definition
struct WizardStep {
    prompt: String,
    help: Option<String>,
    validate: Option<ValidationFn>,
    default: Option<String>,
}

/// Interactive wizard for configuration generation
pub struct ConfigWizard {
    steps: Vec<WizardStep>,
}

impl ConfigWizard {
    /// Create a new wizard with default configuration steps
    pub fn new() -> Self {
        let mut steps = Vec::new();

        // Step 1: Application name
        steps.push(WizardStep {
            prompt: "请输入应用名称:".to_string(),
            help: Some("这是配置的标识符，用于区分不同应用的配置".to_string()),
            validate: Some(Box::new(|s| !s.trim().is_empty())),
            default: Some("myapp".to_string()),
        });

        // Step 2: Version
        steps.push(WizardStep {
            prompt: "请输入应用版本 (默认: 1.0.0):".to_string(),
            help: Some("遵循语义化版本规范 (semver.org)".to_string()),
            validate: Some(Box::new(|s| {
                let trimmed = s.trim();
                trimmed.is_empty() || trimmed.matches('.').count() <= 2
            })),
            default: Some("1.0.0".to_string()),
        });

        // Step 3: Server host
        steps.push(WizardStep {
            prompt: "请输入服务器地址 (默认: localhost):".to_string(),
            help: Some("服务绑定的网络地址，0.0.0.0 表示所有网络接口".to_string()),
            validate: None,
            default: Some("localhost".to_string()),
        });

        // Step 4: Server port
        steps.push(WizardStep {
            prompt: "请输入服务器端口 (默认: 8080):".to_string(),
            help: Some("服务监听的端口号，范围 1-65535".to_string()),
            validate: Some(Box::new(|s| {
                let trimmed = s.trim();
                trimmed.is_empty()
                    || (trimmed.parse::<u16>().is_ok()
                        && trimmed.parse::<u16>().map(|p| p > 0).unwrap_or(false))
            })),
            default: Some("8080".to_string()),
        });

        // Step 5: Database URL
        steps.push(WizardStep {
            prompt: "请输入数据库连接URL (默认: postgres://localhost/mydb):".to_string(),
            help: Some("标准格式: postgres://[用户名]:[密码]@[主机]:[端口]/[数据库名]".to_string()),
            validate: None,
            default: Some("postgres://localhost/mydb".to_string()),
        });

        // Step 6: Log level
        steps.push(WizardStep {
            prompt: "请选择日志级别 (debug/info/warn/error, 默认: info):".to_string(),
            help: Some("debug: 详细调试信息, info: 一般信息, warn: 警告, error: 错误".to_string()),
            validate: Some(Box::new(|s| {
                let trimmed = s.trim().to_lowercase();
                matches!(trimmed.as_str(), "" | "debug" | "info" | "warn" | "error")
            })),
            default: Some("info".to_string()),
        });

        // Step 7: Output file
        steps.push(WizardStep {
            prompt: "请输入输出配置文件路径 (可选，直接回车则输出到控制台):".to_string(),
            help: Some("指定保存配置的文件路径，如 ./config.toml".to_string()),
            validate: None,
            default: None,
        });

        Self { steps }
    }

    /// Run the wizard interactively
    pub fn run(&self) -> Result<GeneratedConfig, ConfigError> {
        println!("\n🧙 配置向导 - 交互式生成配置文件\n");
        println!("========================================\n");

        let mut config = GeneratedConfig::default();

        for (index, step) in self.steps.iter().enumerate() {
            self.run_step(index, step, &mut config)?;
        }

        Ok(config)
    }

    /// Run a single wizard step
    fn run_step(
        &self,
        index: usize,
        step: &WizardStep,
        config: &mut GeneratedConfig,
    ) -> io::Result<()> {
        // Print help if available
        if let Some(help) = &step.help {
            println!("💡 {}", help);
        }

        // Print prompt with default value
        let prompt = if let Some(default) = &step.default {
            format!("{} [{}]: ", step.prompt, default)
        } else {
            step.prompt.clone()
        };

        print!("{}", prompt);
        io::stdout().flush()?;

        // Read user input
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim().to_string();

        // Use default if input is empty
        let value = if input.is_empty() {
            step.default.clone().unwrap_or_default()
        } else {
            input
        };

        // Validate input
        if let Some(validate) = &step.validate {
            if !validate(&value) {
                eprintln!("❌ 输入验证失败，请重新输入。");
                return self.run_step(index, step, config);
            }
        }

        // Store value in config
        self.store_value(index, value, config);

        println!();
        Ok(())
    }

    /// Store the input value in the appropriate config field
    fn store_value(&self, index: usize, value: String, config: &mut GeneratedConfig) {
        match index {
            0 => config.name = value,
            1 => config.version = value,
            2 => config.server.host = value,
            3 => config.server.port = value.parse().unwrap_or(8080),
            4 => config.database.url = value,
            5 => config.logging.level = value,
            6 => {
                if !value.is_empty() {
                    config.output_path = Some(value);
                }
            }
            _ => {}
        }
    }

    /// Run the wizard with predefined values (for non-interactive mode)
    pub fn run_with_values(&self, values: &[&str]) -> Result<GeneratedConfig, ConfigError> {
        let mut config = GeneratedConfig::default();

        for (index, value) in values.iter().enumerate().take(self.steps.len()) {
            if index < self.steps.len() {
                let step = &self.steps[index];
                let trimmed_value = value.trim();
                let value = if trimmed_value.is_empty() {
                    step.default.clone().unwrap_or_default()
                } else {
                    trimmed_value.to_string()
                };

                if let Some(validate) = &step.validate {
                    if !validate(&value) {
                        return Err(ConfigError::ParseError(format!(
                            "Validation failed for step {}: {}",
                            index + 1,
                            value
                        )));
                    }
                }

                self.store_value(index, value, &mut config);
            }
        }

        Ok(config)
    }
}

impl Default for ConfigWizard {
    fn default() -> Self {
        Self::new()
    }
}

/// Generated configuration structure
#[derive(Debug, Clone, Default)]
pub struct GeneratedConfig {
    pub name: String,
    pub version: String,
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub logging: LoggingConfig,
    pub output_path: Option<String>,
}

impl GeneratedConfig {
    /// Convert to TOML string
    pub fn to_toml(&self) -> String {
        format!(
            r#"# Generated by confers - Configuration Management Tool
# ============================================================

[app]
name = "{}"
version = "{}"

[server]
host = "{}"
port = {}

[database]
url = "{}"

[logging]
level = "{}"

# ============================================================
"#,
            self.name,
            self.version,
            self.server.host,
            self.server.port,
            self.database.url,
            self.logging.level
        )
    }

    /// Save configuration to file
    pub fn save(&self) -> Result<(), ConfigError> {
        if let Some(path) = &self.output_path {
            let toml_content = self.to_toml();
            std::fs::write(path, toml_content)
                .map_err(|e| ConfigError::FormatDetectionFailed(e.to_string()))?;
            println!("✅ 配置已保存到: {}", path);
        } else {
            println!("{}", self.to_toml());
        }
        Ok(())
    }
}

/// Server configuration
#[derive(Debug, Clone, Default)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

/// Database configuration
#[derive(Debug, Clone, Default)]
pub struct DatabaseConfig {
    pub url: String,
}

/// Logging configuration
#[derive(Debug, Clone, Default)]
pub struct LoggingConfig {
    pub level: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wizard_creation() {
        let wizard = ConfigWizard::new();
        assert_eq!(wizard.steps.len(), 7);
    }

    #[test]
    fn test_wizard_run_with_values() {
        let wizard = ConfigWizard::new();
        let values = [
            "testapp",
            "2.0.0",
            "0.0.0.0",
            "3000",
            "postgres://user:pass@localhost:5432/testdb",
            "debug",
            "",
        ];

        let config = wizard.run_with_values(&values).unwrap();
        assert_eq!(config.name, "testapp");
        assert_eq!(config.version, "2.0.0");
        assert_eq!(config.server.host, "0.0.0.0");
        assert_eq!(config.server.port, 3000);
        assert_eq!(
            config.database.url,
            "postgres://user:pass@localhost:5432/testdb"
        );
        assert_eq!(config.logging.level, "debug");
        assert!(config.output_path.is_none());
    }

    #[test]
    fn test_config_to_toml() {
        let config = GeneratedConfig {
            name: "myapp".to_string(),
            version: "1.0.0".to_string(),
            server: ServerConfig {
                host: "localhost".to_string(),
                port: 8080,
            },
            database: DatabaseConfig {
                url: "postgres://localhost/mydb".to_string(),
            },
            logging: LoggingConfig {
                level: "info".to_string(),
            },
            output_path: None,
        };

        let toml = config.to_toml();
        assert!(toml.contains("name = \"myapp\""));
        assert!(toml.contains("version = \"1.0.0\""));
        assert!(toml.contains("host = \"localhost\""));
        assert!(toml.contains("port = 8080"));
        assert!(toml.contains("url = \"postgres://localhost/mydb\""));
        assert!(toml.contains("level = \"info\""));
    }

    #[test]
    fn test_server_config_default() {
        let server = ServerConfig::default();
        assert_eq!(server.host, "");
        assert_eq!(server.port, 0);
    }

    #[test]
    fn test_database_config_default() {
        let database = DatabaseConfig::default();
        assert!(database.url.is_empty());
    }

    #[test]
    fn test_logging_config_default() {
        let logging = LoggingConfig::default();
        assert!(logging.level.is_empty());
    }
}
