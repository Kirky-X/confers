// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

use figment::{Figment, Profile, Provider};
use figment::providers::{Env, Format, Toml, Json, Yaml, Serialized};
use serde::{Deserialize, Serialize};
use validator::Validate;
use std::path::Path;
use crate::error::ConfigError;
use crate::traits::{ConfigMap, Sanitize};
use crate::audit::{AuditLogger, AuditConfig};
use crate::encryption::ConfigEncryption;
use crate::providers::cli_provider::CliConfigProvider;

use crate::providers::etcd_provider::EtcdConfigProvider;
use crate::providers::http_provider::HttpProvider;
use crate::providers::ProviderManager;
use crate::providers::SerializedProvider;

use crate::providers::consul_provider::ConsulConfigProvider;

use crate::providers::file_provider::FileConfigProvider;
use crate::providers::environment_provider::EnvironmentProvider;

use std::collections::HashMap;
use std::time::Duration;
use figment::value::Value;

/// Configuration for remote config loading
#[derive(Debug, Clone)]
pub struct RemoteConfig {
    pub enabled: bool,
    pub url: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub token: Option<String>,
    pub ca_cert: Option<String>,
    pub client_cert: Option<String>,
    pub client_key: Option<String>,
    pub timeout: Duration,
}

impl Default for RemoteConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            url: None,
            username: None,
            password: None,
            token: None,
            ca_cert: None,
            client_cert: None,
            client_key: None,
            timeout: Duration::from_secs(30),
        }
    }
}

/// Configuration for audit logging
#[derive(Debug, Clone)]
pub struct AuditConfig {
    pub enabled: bool,
    pub file_path: Option<String>,
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            file_path: None,
        }
    }
}

/// Main configuration loader that handles loading configuration from various sources
#[derive(Debug)]
pub struct ConfigLoader<T> {
    /// Application name for config file discovery
    pub app_name: Option<String>,
    /// Default configuration values
    pub defaults: Option<T>,
    /// Explicit config files to load
    pub explicit_files: Vec<std::path::PathBuf>,
    /// Whether to use environment variables
    pub use_env: bool,
    /// Environment variable prefix
    pub env_prefix: Option<String>,
    /// CLI arguments provider
    pub cli_provider: Option<CliConfigProvider>,
    /// Remote configuration settings
    pub remote_config: RemoteConfig,
    /// Etcd provider for remote config
    pub etcd_provider: Option<crate::providers::etcd_provider::EtcdConfigProvider>,
    /// Whether to enable strict mode (fail on validation errors)
    pub strict: bool,
    /// Audit configuration
    pub audit: AuditConfig,
    /// Configuration for file watching
    pub watch: bool,
    /// Sanitizer function
    pub sanitizer: Option<Box<dyn Fn(T) -> Result<T, ConfigError> + Send + Sync>>,
    /// Format detection mode: "ByContent" or "ByExtension"
    pub format_detection: Option<String>,
    /// Arguments for CLI provider
    pub args: Option<Vec<String>>,
    /// Audit log path
    pub audit_log_path: Option<String>,
    /// Whether to enable audit logging
    pub audit_log: bool,
    /// Remote URL for config
    pub remote_url: Option<String>,
    /// Remote username
    pub remote_username: Option<String>,
    /// Remote password
    pub remote_password: Option<String>,
    /// Remote token
    pub remote_token: Option<String>,
    /// Remote CA cert
    pub remote_ca_cert: Option<String>,
    /// Remote client cert
    pub remote_client_cert: Option<String>,
    /// Remote client key
    pub remote_client_key: Option<String>,
}

impl<T> Default for ConfigLoader<T> {
    fn default() -> Self {
        Self {
            app_name: None,
            defaults: None,
            explicit_files: Vec::new(),
            use_env: true,
            env_prefix: Some("APP".to_string()),
            cli_provider: None,
            remote_config: RemoteConfig::default(),
            etcd_provider: None,
            strict: false,
            audit: AuditConfig::default(),
            watch: false,
            sanitizer: None,
            format_detection: Some("ByContent".to_string()),
            args: None,
            audit_log_path: None,
            audit_log: false,
            remote_url: None,
            remote_username: None,
            remote_password: None,
            remote_token: None,
            remote_ca_cert: None,
            remote_client_cert: None,
            remote_client_key: None,
        }
    }
}

impl<T> ConfigLoader<T> {
    /// Create a new ConfigLoader with default settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the application name for config file discovery
    pub fn with_app_name(mut self, name: impl Into<String>) -> Self {
        self.app_name = Some(name.into());
        self
    }

    /// Set default configuration values
    pub fn with_defaults(mut self, defaults: T) -> Self {
        self.defaults = Some(defaults);
        self
    }

    /// Add an explicit config file to load
    pub fn with_file(mut self, file: impl AsRef<Path>) -> Self {
        self.explicit_files.push(file.as_ref().to_path_buf());
        self
    }

    /// Set whether to use environment variables
    pub fn with_env(mut self, use_env: bool) -> Self {
        self.use_env = use_env;
        self
    }

    /// Set environment variable prefix
    pub fn with_env_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.env_prefix = Some(prefix.into());
        self
    }

    /// Set CLI arguments provider
    pub fn with_cli_provider(mut self, provider: CliConfigProvider) -> Self {
        self.cli_provider = Some(provider);
        self
    }

    /// Enable remote configuration with URL
    pub fn with_remote(mut self, url: impl Into<String>) -> Self {
        self.remote_config.enabled = true;
        self.remote_config.url = Some(url.into());
        self
    }

    /// Alias for with_remote - enable remote configuration with URL
    pub fn remote(mut self, url: impl Into<String>) -> Self {
        self.with_remote(url)
    }

    /// Set remote configuration credentials
    pub fn with_remote_auth(mut self, username: impl Into<String>, password: impl Into<String>) -> Self {
        self.remote_config.username = Some(username.into());
        self.remote_config.password = Some(password.into());
        self
    }

    /// Set remote configuration token
    pub fn with_remote_token(mut self, token: impl Into<String>) -> Self {
        self.remote_config.token = Some(token.into());
        self
    }

    /// Set remote configuration TLS certificates
    pub fn with_remote_tls(mut self, ca_cert: impl Into<String>, client_cert: Option<String>, client_key: Option<String>) -> Self {
        self.remote_config.ca_cert = Some(ca_cert.into());
        self.remote_config.client_cert = client_cert;
        self.remote_config.client_key = client_key;
        self
    }

    /// Set etcd provider
    pub fn with_etcd_provider(mut self, provider: crate::providers::etcd_provider::EtcdConfigProvider) -> Self {
        self.etcd_provider = Some(provider);
        self
    }

    /// Enable strict mode (fail on validation errors)
    pub fn with_strict(mut self, strict: bool) -> Self {
        self.strict = strict;
        self
    }

    /// Enable audit logging
    pub fn with_audit(mut self, enabled: bool) -> Self {
        self.audit.enabled = enabled;
        self
    }

    /// Set audit log file path
    pub fn with_audit_file(mut self, path: impl Into<String>) -> Self {
        self.audit.file_path = Some(path.into());
        self
    }

    /// Enable file watching
    pub fn with_watch(mut self, watch: bool) -> Self {
        self.watch = watch;
        self
    }

    /// Set sanitizer function
    pub fn with_sanitizer(mut self, sanitizer: impl Fn(T) -> Result<T, ConfigError> + Send + Sync + 'static) -> Self {
        self.sanitizer = Some(Box::new(sanitizer));
        self
    }

    /// Set format detection mode
    pub fn with_format_detection(mut self, mode: impl Into<String>) -> Self {
        self.format_detection = Some(mode.into());
        self
    }

    /// Set arguments for CLI provider
    pub fn with_args(mut self, args: Vec<String>) -> Self {
        self.args = Some(args);
        self
    }

    /// Enable audit logging with path
    pub fn with_audit_log(mut self, enabled: bool, path: Option<String>) -> Self {
        self.audit_log = enabled;
        self.audit_log_path = path;
        self
    }

    /// Set remote URL
    pub fn with_remote_url(mut self, url: Option<String>) -> Self {
        self.remote_url = url;
        self
    }

    /// Set remote credentials
    pub fn with_remote_credentials(mut self, username: Option<String>, password: Option<String>) -> Self {
        self.remote_username = username;
        self.remote_password = password;
        self
    }

    /// Set remote token
    pub fn with_remote_token_opt(mut self, token: Option<String>) -> Self {
        self.remote_token = token;
        self
    }

    /// Set remote TLS certificates
    pub fn with_remote_certs(mut self, ca_cert: Option<String>, client_cert: Option<String>, client_key: Option<String>) -> Self {
        self.remote_ca_cert = ca_cert;
        self.remote_client_cert = client_cert;
        self.remote_client_key = client_key;
        self
    }

    /// Detect file format based on content
    pub fn detect_format(path: &Path) -> Option<String> {
        if let Ok(content) = std::fs::read_to_string(path) {
            let trimmed = content.trim();
            if trimmed.starts_with('{') {
                Some("json".to_string())
            } else if trimmed.contains("=") && !trimmed.starts_with("[") {
                Some("toml".to_string())
            } else if trimmed.starts_with("---") || (trimmed.contains(":") && !trimmed.contains("=")) {
                Some("yaml".to_string())
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Detect file format based on extension
    pub fn detect_format_by_extension(path: &Path) -> Option<String> {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_lowercase())
            .and_then(|ext| match ext.as_str() {
                "toml" => Some("toml".to_string()),
                "json" => Some("json".to_string()),
                "yaml" | "yml" => Some("yaml".to_string()),
                _ => None,
            })
    }

    /// Load remote configuration
    #[cfg(feature = "remote")]
    async fn load_remote_config(&self) -> Result<HashMap<String, Value>, ConfigError> {
        use crate::providers::http_provider::HttpProvider;
        
        if let Some(url) = &self.remote_config.url {
            let mut provider = HttpProvider::new(url.clone());
            
            if let Some(token) = &self.remote_config.token {
                provider = provider.with_bearer_token(token);
            }
            
            if let Some(username) = &self.remote_config.username {
                provider = provider.with_auth(
                    username,
                    self.remote_config.password.as_deref().unwrap_or(""),
                );
            }
            
            if let Some(ca_cert) = &self.remote_config.ca_cert {
                provider = provider.with_tls(
                    Some(ca_cert.clone()),
                    self.remote_config.client_cert.clone(),
                    self.remote_config.client_key.clone(),
                );
            }
            
            provider.load().await
        } else {
            Ok(HashMap::new())
        }
    }

    /// Expand template variables in a value recursively
    fn expand_templates_recursive(&self, value: &mut Value) -> bool {
        match value {
            Value::String(s) => {
                if s.contains("${") {
                    *value = Value::String(self.expand_templates(s).unwrap_or_else(|| s.clone()));
                    true
                } else {
                    false
                }
            }
            Value::Map(map) => {
                let mut changed = false;
                for v in map.values_mut() {
                    if self.expand_templates_recursive(v) {
                        changed = true;
                    }
                }
                changed
            }
            Value::Seq(seq) => {
                let mut changed = false;
                for v in seq.iter_mut() {
                    if self.expand_templates_recursive(v) {
                        changed = true;
                    }
                }
                changed
            }
            _ => false,
        }
    }

    /// Expand template variables in a string
    fn expand_templates(&self, s: &str) -> Option<String> {
        if !s.contains("${") {
            return Some(s.to_string());
        }

        let mut result = s.to_string();
        let mut start = 0;
        
        while let Some(var_start) = result[start..].find("${") {
            let var_start = start + var_start;
            if let Some(var_end) = result[var_start..].find('}') {
                let var_end = var_start + var_end;
                let var_name = &result[var_start + 2..var_end];
                
                if let Ok(env_value) = std::env::var(var_name) {
                    result.replace_range(var_start..=var_end, &env_value);
                    start = var_start + env_value.len();
                } else {
                    start = var_end + 1;
                }
            } else {
                break;
            }
        }
        
        Some(result)
    }

    /// Decrypt encrypted values recursively
    fn decrypt_value_recursive(&self, value: &mut Value, encryptor: &ConfigEncryption) -> bool {
        match value {
            Value::String(s) => {
                if s.starts_with("ENC(") && s.ends_with(")") {
                    let encrypted = &s[4..s.len() - 1];
                    if let Ok(decrypted) = encryptor.decrypt(encrypted) {
                        *value = Value::String(decrypted);
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            Value::Map(map) => {
                let mut changed = false;
                for v in map.values_mut() {
                    if self.decrypt_value_recursive(v, encryptor) {
                        changed = true;
                    }
                }
                changed
            }
            Value::Seq(seq) => {
                let mut changed = false;
                for v in seq.iter_mut() {
                    if self.decrypt_value_recursive(v, encryptor) {
                        changed = true;
                    }
                }
                changed
            }
            _ => false,
        }
    }

    /// Build a provider manager with all configured providers
    fn build_provider_manager(&self) -> Result<ProviderManager, ConfigError> {
        // Add default configuration provider (lowest priority)
        let default_figment = Figment::new().merge(Serialized::from(
            self.defaults.clone().unwrap_or_default(),
            "default",
        ));
        let mut manager = ProviderManager::new();
        manager.add_provider(SerializedProvider::new(default_figment, "default"));

        // Add explicit file providers
        for file_path in &self.explicit_files {
            if file_path.exists() && !is_editor_temp_file(file_path) {
                let format = Self::detect_format(file_path);
                if let Some(fmt) = format {
                    let file_figment = match fmt.as_str() {
                        "toml" => Figment::new().merge(Toml::file(file_path)),
                        "json" => Figment::new().merge(Json::file(file_path)),
                        "yaml" => Figment::new().merge(Yaml::file(file_path)),
                        _ => continue,
                    };
                    manager.add_provider(SerializedProvider::new(file_figment, file_path.to_string_lossy().as_ref()));
                }
            }
        }

        // Add environment provider
        if self.use_env {
            let env_prefix = self.env_prefix.as_deref().unwrap_or("");
            let env_figment = Figment::new().merge(Env::prefixed(env_prefix).split("__"));
            manager.add_provider(SerializedProvider::new(env_figment, "environment"));
        }

        // Add CLI provider
        if let Some(cli_provider) = &self.cli_provider {
            let cli_figment = cli_provider.load()?;
            manager.add_provider(SerializedProvider::new(cli_figment, "cli"));
        }

        // Add remote providers
        #[cfg(feature = "remote")]
        {
            if self.remote_config.enabled {
                if let Some(url) = &self.remote_config.url {
                    let mut provider = HttpProvider::new(url.clone());
                    
                    if let Some(token) = &self.remote_config.token {
                        provider = provider.with_bearer_token(token);
                    }
                    
                    if let Some(username) = &self.remote_config.username {
                        provider = provider.with_auth(
                            username,
                            self.remote_config.password.as_deref().unwrap_or(""),
                        );
                    }
                    
                    if let Some(ca_cert) = &self.remote_config.ca_cert {
                        provider = provider.with_tls(
                            Some(ca_cert.clone()),
                            self.remote_config.client_cert.clone(),
                            self.remote_config.client_key.clone(),
                        );
                    }
                    
                    let remote_figment = provider.load().await?;
                    manager.add_provider(SerializedProvider::new(remote_figment, "remote"));
                }
            }

            if let Some(etcd_provider) = &self.etcd_provider {
                let etcd_figment = etcd_provider.load().await?;
                manager.add_provider(SerializedProvider::new(etcd_figment, "etcd"));
            }
        }

        Ok(manager)
    }
}

impl<T> ConfigLoader<T>
where
    T: Sanitize + for<'de> Deserialize<'de> + Serialize + Default + Clone + Validate + crate::ConfigMap,
{
    /// Helper method to load configuration with a given figment
    fn load_with_figment(
        &self,
        mut figment: Figment,
        run_env: Option<String>,
        app_name: &str,
        audit_info: Option<(Vec<(String, String, Option<String>, Option<std::time::Duration>)>, std::time::Instant)>,
    ) -> Result<T, ConfigError> {
        let load_start = std::time::Instant::now();
        let mut config_sources_status = Vec::new();
        if let Some((ref mut status, _)) = audit_info {
            config_sources_status = status.clone();
        }

        // 4. Load explicit files
        let mut explicit_files_loaded = 0;
        for file in &self.explicit_files {
            let file_start = std::time::Instant::now();
            let path_str = file.to_string_lossy();
            let mut _file_loaded = false;

            if !is_editor_temp_file(file) {
                // 根据format_detection设置选择格式检测方式
                let format_mode = self.format_detection.as_deref().unwrap_or("ByContent");
                let detected_format = match format_mode {
                    "ByContent" => ConfigLoader::<T>::detect_format(file),
                    "ByExtension" => ConfigLoader::<T>::detect_format_by_extension(file),
                    _ => ConfigLoader::<T>::detect_format(file), // 默认使用ByContent
                };

                if let Some(fmt) = detected_format {
                    match fmt.as_str() {
                        "toml" => figment = figment.merge(Toml::file(path_str.as_ref())),
                        "yaml" => figment = figment.merge(Yaml::file(path_str.as_ref())),
                        "json" => figment = figment.merge(Json::file(path_str.as_ref())),
                        _ => {
                            // 如果检测到未知格式，尝试作为JSON加载
                            figment = figment.merge(Json::file(path_str.as_ref()));
                        }
                    }
                    _file_loaded = true;
                } else {
                    // 如果无法检测格式，默认尝试JSON格式
                    figment = figment.merge(Json::file(path_str.as_ref()));
                    _file_loaded = true;
                }

                if _file_loaded {
                    config_sources_status.push((
                        format!("explicit_file:{}", file.display()),
                        "Success".to_string(),
                        None,
                        Some(file_start.elapsed()),
                    ));
                }
                explicit_files_loaded += 1;
            }
        }

        // 5. Load environment variables
        let _env_prefix = self.env_prefix.as_deref().unwrap_or("");
        figment = figment.merge(Env::prefixed(_env_prefix).split("__"));

        // 6. Load CLI arguments
        if let Some(cli_provider) = &self.cli_provider {
            figment = figment.merge(cli_provider.load()?);
        }

        // 7. Load remote config if enabled
        #[cfg(feature = "remote")]
        if self.remote_config.enabled {
            let remote_start = std::time::Instant::now();
            match self.load_remote_config().await {
                Ok(remote_config) => {
                    figment = figment.merge(Serialized::from(&remote_config, "remote"));
                    if let Some((ref mut status, _)) = audit_info {
                        status.push(("remote".to_string(), "success".to_string(), None, Some(remote_start.elapsed())));
                    }
                }
                Err(e) => {
                    if let Some((ref mut status, _)) = audit_info {
                        status.push(("remote".to_string(), format!("error: {}", e), None, Some(remote_start.elapsed())));
                    }
                    if self.strict {
                        return Err(e);
                    }
                }
            }
        }

        // 8. Load etcd config if available
        #[cfg(feature = "remote")]
        if let Some(etcd_provider) = &self.etcd_provider {
            let etcd_start = std::time::Instant::now();
            match etcd_provider.load().await {
                Ok(etcd_config) => {
                    figment = figment.merge(Serialized::from(&etcd_config, "etcd"));
                    if let Some((ref mut status, _)) = audit_info {
                        status.push(("etcd".to_string(), "success".to_string(), None, Some(etcd_start.elapsed())));
                    }
                }
                Err(e) => {
                    if let Some((ref mut status, _)) = audit_info {
                        status.push(("etcd".to_string(), format!("error: {}", e), None, Some(etcd_start.elapsed())));
                    }
                    if self.strict {
                        return Err(e);
                    }
                }
            }
        }

        // 9. Extract and validate configuration
        let mut config: T = figment.extract()?;

        // Apply sanitization if available
        if let Some(sanitizer) = &self.sanitizer {
            config = sanitizer(config)?;
        }

        // Validate configuration
        config.validate().map_err(|e| ConfigError::ValidationError(format!("{:?}", e)))?;

        // 10. Audit logging
        #[cfg(feature = "audit")]
        if self.audit.enabled {
            let default_path = self.audit.file_path.as_deref().unwrap_or("config_audit.log");
            let validation_error = config.validate().err().map(|e| format!("{:?}", e));
            let config_source = Some(format!("Config loaded from {} explicit files", explicit_files_loaded));

            let audit_config = AuditConfig {
                validation_error,
                config_source,
                load_duration: Some(load_start.elapsed()),
                config_sources_status: Some(config_sources_status),
                files_attempted: None,
                files_loaded: None,
                format_distribution: None,
                env_vars_count: None,
                memory_usage_mb: None,
            };

            if let Err(e) = AuditLogger::log_to_file_with_source(
                &config,
                std::path::Path::new(&default_path),
                audit_config,
            ) {
                eprintln!("Warning: Failed to write audit log: {}", e);
            }
        }

        if self.strict {
            validation_result?;
        }

        Ok(config)
    }

    /// Load configuration asynchronously with audit support
    #[cfg(feature = "audit")]
    pub async fn load(&self) -> Result<T, ConfigError>
    where
        T: Sanitize + for<'de> Deserialize<'de> + Serialize + Default + Clone + Validate + crate::ConfigMap,
    {
        let mut figment = Figment::new();

        // 1. Load defaults if provided
        if let Some(ref defaults) = self.defaults {
            figment = figment.merge(Serialized::from(defaults, "default"));
        }

        // 2. Load standard config files
        let mut standard_files_loaded = 0;
        let mut search_paths = vec![std::path::PathBuf::from(".")];

        if let Some(config_dir) = dirs::config_dir() {
            if let Some(app_name) = &self.app_name {
                search_paths.push(config_dir.join(app_name));
            }
            search_paths.push(config_dir);
        }

        if let Some(home) = dirs::home_dir() {
            search_paths.push(home);
        }

        #[cfg(unix)]
        if let Some(app_name) = &self.app_name {
            search_paths.push(std::path::PathBuf::from(format!("/etc/{}", app_name)));
        }

        let run_env = std::env::var("RUN_ENV").ok();
        let app_name = self.app_name.as_deref().unwrap_or("app");

        let mut config_sources_status = Vec::new();
        for path in &search_paths {
            let base_path = if let Some(app_name) = &self.app_name {
                path.join(app_name)
            } else {
                path.clone()
            };
            let formats = ["toml", "json", "yaml", "yml"];

            // Find all existing config files in priority order
            let mut existing_files = Vec::new();
            for format in &formats {
                let file_path = base_path.join(format!("config.{}", format));
                if file_path.exists() {
                    existing_files.push(file_path);
                }
            }

            // Load files in reverse order (highest priority first)
            for file_path in existing_files.iter().rev() {
                let path_str = file_path.to_string_lossy();
                let format = ConfigLoader::<T>::detect_format(file_path);

                if let Some(fmt) = format {
                    let file_start = std::time::Instant::now();
                    match fmt.as_str() {
                        "toml" => figment = figment.merge(Toml::file(path_str.as_ref())),
                        "yaml" => figment = figment.merge(Yaml::file(path_str.as_ref())),
                        "json" => figment = figment.merge(Json::file(path_str.as_ref())),
                        _ => {}
                    }
                    config_sources_status.push((
                        format!("standard_file:{}", file_path.display()),
                        "Success".to_string(),
                        None,
                        Some(file_start.elapsed()),
                    ));
                    standard_files_loaded += 1;
                }
            }

            // Load environment-specific config files
            if let Some(ref env) = run_env {
                let mut existing_env_files = Vec::new();
                for format in &formats {
                    let env_file_path = path.join(format!("{}.{}.{}", app_name, env, format));
                    if env_file_path.exists() {
                        existing_env_files.push(env_file_path);
                    }
                }

                for env_file_path in existing_env_files.iter().rev() {
                    let path_str = env_file_path.to_string_lossy();
                    let format = ConfigLoader::<T>::detect_format(env_file_path);

                    if let Some(fmt) = format {
                        let file_start = std::time::Instant::now();
                        match fmt.as_str() {
                            "toml" => figment = figment.merge(Toml::file(path_str.as_ref())),
                            "yaml" => figment = figment.merge(Yaml::file(path_str.as_ref())),
                            "json" => figment = figment.merge(Json::file(path_str.as_ref())),
                            _ => {}
                        }
                        config_sources_status.push((
                            format!("env_file:{}", env_file_path.display()),
                            "Success".to_string(),
                            None,
                            Some(file_start.elapsed()),
                        ));
                        standard_files_loaded += 1;
                    }
                }
            }
        }

        if standard_files_loaded == 0 {
            config_sources_status.push((
                "standard_files".to_string(),
                "Skipped".to_string(),
                None,
                None,
            ));
        }

        let audit_info = Some((config_sources_status, std::time::Instant::now()));
        self.load_with_figment(figment, run_env, app_name, audit_info).await
    }

    /// Load configuration asynchronously without audit support
    #[cfg(not(feature = "audit"))]
    pub async fn load(&self) -> Result<T, ConfigError>
    where
        T: for<'de> Deserialize<'de> + Serialize + Default + Clone + Validate + crate::ConfigMap,
    {
        let mut figment = Figment::new();

        // 1. Load defaults if provided
        if let Some(ref defaults) = self.defaults {
            figment = figment.merge(Serialized::from(defaults, "default"));
        }

        // 2. Load standard config files
        let mut standard_files_loaded = 0;
        let mut search_paths = vec![std::path::PathBuf::from(".")];

        if let Some(config_dir) = dirs::config_dir() {
            if let Some(app_name) = &self.app_name {
                search_paths.push(config_dir.join(app_name));
            }
            search_paths.push(config_dir);
        }

        if let Some(home) = dirs::home_dir() {
            search_paths.push(home);
        }

        #[cfg(unix)]
        if let Some(app_name) = &self.app_name {
            search_paths.push(std::path::PathBuf::from(format!("/etc/{}", app_name)));
        }

        let run_env = std::env::var("RUN_ENV").ok();
        let app_name = self.app_name.as_deref().unwrap_or("app");

        for path in &search_paths {
            let base_path = if let Some(app_name) = &self.app_name {
                path.join(app_name)
            } else {
                path.clone()
            };
            let formats = ["toml", "json", "yaml", "yml"];

            // Find all existing config files in priority order
            let mut existing_files = Vec::new();
            for format in &formats {
                let file_path = base_path.join(format!("config.{}", format));
                if file_path.exists() {
                    existing_files.push(file_path);
                }
            }

            // Load files in reverse order (highest priority first)
            for file_path in existing_files.iter().rev() {
                let path_str = file_path.to_string_lossy();
                let format = ConfigLoader::<T>::detect_format(file_path);

                if let Some(fmt) = format {
                    match fmt.as_str() {
                        "toml" => figment = figment.merge(Toml::file(path_str.as_ref())),
                        "yaml" => figment = figment.merge(Yaml::file(path_str.as_ref())),
                        "json" => figment = figment.merge(Json::file(path_str.as_ref())),
                        _ => {}
                    }
                    standard_files_loaded += 1;
                }
            }

            // Load environment-specific config files
            if let Some(ref env) = run_env {
                let mut existing_env_files = Vec::new();
                for format in &formats {
                    let env_file_path = path.join(format!("{}.{}.{}", app_name, env, format));
                    if env_file_path.exists() {
                        existing_env_files.push(env_file_path);
                    }
                }

                for env_file_path in existing_env_files.iter().rev() {
                    let path_str = env_file_path.to_string_lossy();
                    let format = ConfigLoader::<T>::detect_format(env_file_path);

                    if let Some(fmt) = format {
                        match fmt.as_str() {
                            "toml" => figment = figment.merge(Toml::file(path_str.as_ref())),
                            "yaml" => figment = figment.merge(Yaml::file(path_str.as_ref())),
                            "json" => figment = figment.merge(Json::file(path_str.as_ref())),
                            _ => {}
                        }
                        standard_files_loaded += 1;
                    }
                }
            }
        }

        self.load_with_figment(figment, run_env, app_name, None).await
    }

    /// Load configuration with file watching
    #[cfg(feature = "audit")]
    pub fn load_with_watcher(
        self,
    ) -> Result<(T, Option<crate::watcher::ConfigWatcher>), ConfigError>
    where
        T: Sanitize,
    {
        let explicit_files = self.explicit_files.clone();
        let watch = self.watch;
        let config = self.load()?;
        
        let watcher = if watch {
            Some(crate::watcher::ConfigWatcher::new(explicit_files)?)
        } else {
            None
        };
        
        Ok((config, watcher))
    }
}

/// Check if a file is an editor temporary file
fn is_editor_temp_file(path: &Path) -> bool {
    let file_name = path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");
    
    file_name.ends_with('~') ||
    file_name.starts_with('.') && file_name.ends_with('.') ||
    file_name.starts_with('#') && file_name.ends_with('#') ||
    file_name.ends_with(".swp") ||
    file_name.ends_with(".swo") ||
    file_name.ends_with(".tmp")
}