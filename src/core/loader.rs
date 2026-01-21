// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

#[cfg(feature = "audit")]
use crate::audit::AuditConfig as AuditConfigComplex;
use crate::audit::Sanitize;
#[cfg(feature = "encryption")]
use crate::encryption::ConfigEncryption;
use crate::error::ConfigError;
use crate::providers::cli_provider::CliConfigProvider;
use crate::providers::environment_provider::EnvironmentProvider;
use crate::providers::file_provider::FileConfigProvider;
use crate::providers::provider::ProviderManager;
use crate::providers::SerializedProvider;
#[cfg(all(feature = "remote", feature = "encryption"))]
use crate::security::secure_string::{SecureString, SensitivityLevel};
use figment::providers::{Format, Json, Serialized, Toml, Yaml};
#[cfg(feature = "encryption")]
use figment::value::Tag;
use figment::value::Value;
use figment::Figment;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
#[allow(unused_imports)]
use std::sync::Arc;
#[cfg(feature = "validation")]
use validator::Validate;

#[cfg(any(feature = "encryption", feature = "remote"))]
// use crate::security; // Uncomment when needed
/// A type alias for the sanitizer function
type SanitizerFn<T> = std::sync::Arc<dyn Fn(T) -> Result<T, ConfigError> + Send + Sync>;

/// Trait for optionally validating configuration
pub trait OptionalValidate {
    fn optional_validate(&self) -> Result<(), crate::error::ConfigError> {
        Ok(())
    }
}

#[cfg(feature = "validation")]
/// Implement OptionalValidate for types that implement Validate
impl<T: Validate> OptionalValidate for T {
    fn optional_validate(&self) -> Result<(), crate::error::ConfigError> {
        self.validate()
            .map_err(|e| crate::error::ConfigError::ValidationError(format!("{:?}", e)))
    }
}

#[cfg(feature = "remote")]
use crate::providers::consul_provider::ConsulConfigProvider;

#[cfg(feature = "remote")]
use crate::providers::etcd_provider::EtcdConfigProvider;

#[cfg(feature = "remote")]
use crate::providers::http_provider::HttpConfigProvider;

#[cfg(feature = "monitoring")]
use std::sync::OnceLock;

/// Get current memory usage in MB using sysinfo crate
/// Cross-platform support: Linux, macOS, Windows
/// Uses caching to avoid repeated system calls
#[allow(dead_code)]
#[cfg(feature = "monitoring")]
fn get_memory_usage_mb() -> Option<f64> {
    static LAST_MEMORY: OnceLock<(f64, std::time::Instant)> = OnceLock::new();
    let now = std::time::Instant::now();

    // Use cache duration from constants (1 second) to balance performance and accuracy
    if let Some((memory, time)) = LAST_MEMORY.get() {
        if now.duration_since(*time)
            < std::time::Duration::from_millis(crate::constants::time::MEMORY_CACHE_DURATION_MS)
        {
            return Some(*memory);
        }
    }

    use std::process;
    use sysinfo::{Pid, ProcessRefreshKind, RefreshKind, System};

    let sys = System::new_with_specifics(
        RefreshKind::nothing().with_processes(ProcessRefreshKind::everything()),
    );

    let current_pid = Pid::from_u32(process::id());
    let memory = sys
        .process(current_pid)
        .map(|process| process.memory() as f64 / 1024.0 / 1024.0);

    if let Some(mem_value) = memory {
        let _ = LAST_MEMORY.set((mem_value, now));
    }

    memory
}

/// Get current memory usage with cache (deprecated)
///
/// # Deprecated
///
/// This method is deprecated because OnceLock cannot clear the cache.
/// Please use `get_memory_usage_mb()` directly, which automatically refreshes
/// the cache after 1 second (see `crate::constants::time::MEMORY_CACHE_DURATION_MS`).
///
/// The cache cannot be force-refreshed due to OnceLock limitations.
/// For more accurate memory checks, the system relies on a reasonable cache
/// duration that balances performance and accuracy.
#[deprecated(since = "0.3.0", note = "Use get_memory_usage_mb() instead")]
#[allow(dead_code)]
#[cfg(feature = "monitoring")]
pub fn force_refresh_memory() -> Option<f64> {
    // Note: OnceLock cannot be cleared, so this function cannot force a refresh.
    // It returns the cached value (or fresh if cache expired).
    get_memory_usage_mb()
}

#[allow(dead_code)]
#[cfg(not(feature = "monitoring"))]
fn get_memory_usage_mb() -> Option<f64> {
    None
}

#[cfg(feature = "audit")]
use crate::audit::AuditLogger;

/// Configuration loader that supports multiple sources and formats
#[derive(Clone)]
pub struct ConfigLoader<T> {
    /// Default configuration values
    defaults: Option<T>,
    /// Explicit configuration files to load
    explicit_files: Vec<PathBuf>,
    /// Application name for standard config file locations
    app_name: Option<String>,
    /// Environment prefix for environment variables
    env_prefix: Option<String>,
    /// Whether to use environment variables
    use_env: bool,
    /// Whether to use strict mode (fail on any error)
    strict: bool,
    /// Whether to enable file watching
    watch: bool,
    /// Format detection mode (ByContent, ByExtension)
    format_detection: Option<String>,
    /// Custom sanitizer function
    sanitizer: Option<SanitizerFn<T>>,
    /// CLI configuration provider
    cli_provider: Option<CliConfigProvider>,
    /// Remote configuration settings
    #[cfg(feature = "remote")]
    remote_config: RemoteConfig,
    /// Etcd configuration provider
    #[cfg(feature = "remote")]
    etcd_provider: Option<EtcdConfigProvider>,
    /// Consul configuration provider
    #[cfg(feature = "remote")]
    consul_provider: Option<ConsulConfigProvider>,
    /// Audit configuration
    #[cfg(feature = "audit")]
    audit: AuditConfig,
    /// Maximum memory limit in MB (0 = no limit)
    memory_limit_mb: usize,
    /// Maximum configuration file size in MB (0 = no limit)
    max_config_size_mb: usize,
}

/// Remote configuration settings
#[cfg(all(feature = "remote", feature = "encryption"))]
#[derive(Clone, Debug)]
pub struct RemoteConfig {
    enabled: bool,
    url: Option<String>,
    token: Option<Arc<SecureString>>,
    username: Option<String>,
    password: Option<Arc<SecureString>>,
    ca_cert: Option<PathBuf>,
    client_cert: Option<PathBuf>,
    client_key: Option<PathBuf>,
    timeout: Option<String>,
    fallback: bool,
}

#[cfg(all(feature = "remote", feature = "encryption"))]
impl RemoteConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn with_url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }

    pub fn with_token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(Arc::new(SecureString::new(
            token.into(),
            SensitivityLevel::High,
        )));
        self
    }

    pub fn with_username(mut self, username: impl Into<String>) -> Self {
        self.username = Some(username.into());
        self
    }

    pub fn with_password(mut self, password: impl Into<String>) -> Self {
        self.password = Some(Arc::new(SecureString::new(
            password.into(),
            SensitivityLevel::Critical,
        )));
        self
    }

    pub fn with_timeout(mut self, timeout: impl Into<String>) -> Self {
        self.timeout = Some(timeout.into());
        self
    }

    pub fn with_fallback(mut self, fallback: bool) -> Self {
        self.fallback = fallback;
        self
    }

    pub fn url(&self) -> Option<&str> {
        self.url.as_deref()
    }

    pub fn username(&self) -> Option<&str> {
        self.username.as_deref()
    }
}

#[cfg(all(feature = "remote", feature = "encryption"))]
impl Default for RemoteConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            url: None,
            token: None,
            username: None,
            password: None,
            ca_cert: None,
            client_cert: None,
            client_key: None,
            timeout: None,
            fallback: true,
        }
    }
}

/// Simple audit configuration for ConfigLoader
#[cfg(feature = "audit")]
#[derive(Clone, Debug, Default)]
pub struct AuditConfig {
    pub enabled: bool,
    pub file_path: Option<String>,
}

impl<T> Default for ConfigLoader<T> {
    fn default() -> Self {
        Self {
            defaults: None,
            explicit_files: Vec::new(),
            app_name: None,
            env_prefix: None,
            use_env: true,
            strict: false,
            watch: false,
            format_detection: None,
            sanitizer: None,
            cli_provider: None,
            #[cfg(feature = "remote")]
            remote_config: RemoteConfig::default(),
            #[cfg(feature = "remote")]
            etcd_provider: None,
            #[cfg(feature = "remote")]
            consul_provider: None,
            #[cfg(feature = "audit")]
            audit: AuditConfig::default(),
            memory_limit_mb: 512, // Increased to reasonable default for production
            max_config_size_mb: crate::constants::config::MAX_CONFIG_SIZE_MB,
        }
    }
}

impl<T: OptionalValidate> ConfigLoader<T> {
    /// Create a new configuration loader
    pub fn new() -> Self {
        Self::default()
    }

    /// Set default configuration values
    pub fn with_defaults(mut self, defaults: T) -> Self {
        self.defaults = Some(defaults);
        self
    }

    /// Add an explicit configuration file
    pub fn with_file(mut self, path: impl AsRef<Path>) -> Self {
        self.explicit_files.push(path.as_ref().to_path_buf());
        self
    }

    /// Add multiple explicit configuration files
    pub fn with_files(mut self, paths: Vec<impl AsRef<Path>>) -> Self {
        self.explicit_files
            .extend(paths.iter().map(|p| p.as_ref().to_path_buf()));
        self
    }

    /// Set the application name for standard config file locations
    pub fn with_app_name(mut self, name: impl Into<String>) -> Self {
        self.app_name = Some(name.into());
        self
    }

    /// Set the environment prefix for environment variables
    pub fn with_env_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.env_prefix = Some(prefix.into());
        self
    }

    /// Enable or disable environment variables
    pub fn with_env(mut self, enabled: bool) -> Self {
        self.use_env = enabled;
        self
    }

    /// Enable or disable strict mode
    pub fn with_strict(mut self, strict: bool) -> Self {
        self.strict = strict;
        self
    }

    /// Enable or disable file watching
    pub fn with_watch(mut self, watch: bool) -> Self {
        self.watch = watch;
        self
    }

    /// Set format detection mode
    pub fn with_format_detection(mut self, mode: impl Into<String>) -> Self {
        self.format_detection = Some(mode.into());
        self
    }

    /// Set custom sanitizer function
    pub fn with_sanitizer(
        mut self,
        sanitizer: impl Fn(T) -> Result<T, ConfigError> + Send + Sync + 'static,
    ) -> Self {
        self.sanitizer = Some(std::sync::Arc::new(sanitizer));
        self
    }

    /// Set CLI configuration provider
    pub fn with_cli_provider(mut self, provider: CliConfigProvider) -> Self {
        self.cli_provider = Some(provider);
        self
    }

    /// Configure remote configuration
    #[cfg(feature = "remote")]
    pub fn with_remote_config(mut self, url: impl Into<String>) -> Self {
        self.remote_config.enabled = true;
        self.remote_config.url = Some(url.into());
        self
    }

    /// Alias for with_remote_config - enable remote configuration with URL
    #[cfg(feature = "remote")]
    pub fn remote(self, url: impl Into<String>) -> Self {
        self.with_remote_config(url)
    }

    /// Alias for with_remote_config - enable remote configuration with URL
    #[cfg(feature = "remote")]
    pub fn with_remote(self, url: impl Into<String>) -> Self {
        self.with_remote_config(url)
    }

    /// Configure remote configuration with authentication
    #[cfg(feature = "remote")]
    pub fn with_remote_auth(
        mut self,
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Self {
        self.remote_config.enabled = true;
        self.remote_config.username = Some(username.into());
        self.remote_config.password = Some(Arc::new(SecureString::new(
            password.into(),
            SensitivityLevel::Critical,
        )));
        self
    }

    /// Configure remote configuration with bearer token
    #[cfg(feature = "remote")]
    pub fn with_remote_token(mut self, token: impl Into<String>) -> Self {
        self.remote_config.enabled = true;
        self.remote_config.token = Some(Arc::new(SecureString::new(
            token.into(),
            SensitivityLevel::High,
        )));
        self
    }

    /// Configure remote configuration with TLS
    #[cfg(feature = "remote")]
    pub fn with_remote_tls(
        mut self,
        ca_cert: impl AsRef<Path>,
        client_cert: Option<impl AsRef<Path>>,
        client_key: Option<impl AsRef<Path>>,
    ) -> Self {
        self.remote_config.enabled = true;
        self.remote_config.ca_cert = Some(ca_cert.as_ref().to_path_buf());
        self.remote_config.client_cert = client_cert.map(|p| p.as_ref().to_path_buf());
        self.remote_config.client_key = client_key.map(|p| p.as_ref().to_path_buf());
        self
    }

    /// Set etcd configuration provider
    #[cfg(feature = "remote")]
    pub fn with_etcd(mut self, provider: EtcdConfigProvider) -> Self {
        self.etcd_provider = Some(provider);
        self
    }

    /// Set consul configuration provider
    #[cfg(feature = "remote")]
    pub fn with_consul(mut self, provider: ConsulConfigProvider) -> Self {
        self.consul_provider = Some(provider);
        self
    }

    /// Configure audit logging
    #[cfg(feature = "audit")]
    pub fn with_audit(mut self, enabled: bool) -> Self {
        self.audit.enabled = enabled;
        self
    }

    /// Configure audit file path
    #[cfg(feature = "audit")]
    pub fn with_audit_file(mut self, path: impl Into<String>) -> Self {
        self.audit.enabled = true;
        self.audit.file_path = Some(path.into());
        self
    }

    /// Set remote configuration timeout
    #[cfg(feature = "remote")]
    pub fn with_remote_timeout(mut self, timeout: impl Into<String>) -> Self {
        self.remote_config.timeout = Some(timeout.into());
        self
    }

    /// Set memory limit in MB
    pub fn with_memory_limit(mut self, limit_mb: usize) -> Self {
        if limit_mb > 0 && limit_mb < 100 {
            #[cfg(feature = "tracing")]
            tracing::warn!(
                "Memory limit of {}MB may be too low for production. Recommended minimum: 100MB",
                limit_mb
            );
        }
        self.memory_limit_mb = limit_mb;
        self
    }

    /// Set maximum configuration file size in MB
    ///
    /// This prevents loading extremely large configuration files that could
    /// cause memory issues or DoS attacks. Set to 0 to disable the limit.
    ///
    /// # Arguments
    ///
    /// * `size_mb` - Maximum file size in megabytes (default: 10MB)
    ///
    /// # Example
    ///
    /// ```rust
    /// # use confers::ConfigLoader;
    /// # use serde::{Deserialize, Serialize};
    /// # #[derive(Debug, Clone, Serialize, Deserialize)]
    /// # struct Config {}
    /// # impl confers::OptionalValidate for Config {
    /// #     fn optional_validate(&self) -> Result<(), confers::ConfigError> {
    /// #         Ok(())
    /// #     }
    /// # }
    /// let loader = ConfigLoader::<Config>::new()
    ///     .with_max_config_size(5); // Limit to 5MB
    /// ```
    pub fn with_max_config_size(mut self, size_mb: usize) -> Self {
        self.max_config_size_mb = size_mb;
        self
    }

    /// Set remote configuration fallback
    #[cfg(feature = "remote")]
    pub fn with_remote_fallback(mut self, fallback: bool) -> Self {
        self.remote_config.fallback = fallback;
        self
    }

    /// Set remote username
    #[cfg(feature = "remote")]
    pub fn with_remote_username(mut self, username: impl Into<String>) -> Self {
        self.remote_config.username = Some(username.into());
        self
    }

    /// Alias for with_remote_username
    #[cfg(feature = "remote")]
    pub fn remote_username(self, username: impl Into<String>) -> Self {
        self.with_remote_username(username)
    }

    /// Set remote password
    #[cfg(feature = "remote")]
    pub fn with_remote_password(mut self, password: impl Into<String>) -> Self {
        self.remote_config.password = Some(Arc::new(SecureString::new(
            password.into(),
            SensitivityLevel::Critical,
        )));
        self
    }

    /// Alias for with_remote_password
    #[cfg(feature = "remote")]
    pub fn remote_password(self, password: impl Into<String>) -> Self {
        self.with_remote_password(password)
    }

    /// Configure audit logging
    #[cfg(feature = "audit")]
    pub fn with_audit_log(mut self, enabled: bool) -> Self {
        self.audit.enabled = enabled;
        self
    }

    /// Configure audit file path
    #[cfg(feature = "audit")]
    pub fn with_audit_log_path(mut self, path: impl Into<String>) -> Self {
        self.audit.enabled = true;
        self.audit.file_path = Some(path.into());
        self
    }

    /// Configure remote CA cert
    #[cfg(feature = "remote")]
    pub fn with_remote_ca_cert(mut self, path: impl Into<std::path::PathBuf>) -> Self {
        self.remote_config.ca_cert = Some(path.into());
        self
    }

    /// Configure remote client cert
    #[cfg(feature = "remote")]
    pub fn with_remote_client_cert(mut self, path: impl Into<std::path::PathBuf>) -> Self {
        self.remote_config.client_cert = Some(path.into());
        self
    }

    /// Configure remote client key
    #[cfg(feature = "remote")]
    pub fn with_remote_client_key(mut self, path: impl Into<std::path::PathBuf>) -> Self {
        self.remote_config.client_key = Some(path.into());
        self
    }

    /// Detect file format by content with improved heuristics
    pub fn detect_format(path: &Path) -> Option<String> {
        use crate::utils::file_format::{detect_format_by_content, detect_format_by_extension};

        // Try content detection first
        if let Some(format) = detect_format_by_content(path) {
            return Some(format.to_string());
        }

        // Fall back to extension detection
        detect_format_by_extension(path).map(|f| f.to_string())
    }

    /// Detect file format by extension
    pub fn detect_format_by_extension(path: &Path) -> Option<String> {
        use crate::utils::file_format::detect_format_by_extension;
        detect_format_by_extension(path).map(|f| f.to_string())
    }

    /// Setup base provider with default configuration
    ///
    /// This helper method initializes the ProviderManager and adds the base figment
    /// as a SerializedProvider, which includes default configuration values.
    #[allow(dead_code)]
    fn setup_base_provider(&self, figment: &Figment) -> ProviderManager {
        let mut manager = ProviderManager::new();
        manager.add_provider(SerializedProvider::new(figment.clone(), "base_config"));
        manager
    }

    /// Setup file provider for loading explicit configuration files
    ///
    /// This helper method adds a FileConfigProvider to the manager if explicit files
    /// are configured. Files are loaded with priority 40 (lower than environment variables).
    #[allow(dead_code)]
    fn setup_file_provider(&self, manager: &mut ProviderManager) -> Result<(), ConfigError> {
        if !self.explicit_files.is_empty() {
            // Check file sizes before loading
            if self.max_config_size_mb > 0 {
                for path in &self.explicit_files {
                    if path.exists() {
                        let metadata = std::fs::metadata(path)
                            .map_err(|e| ConfigError::IoError(e.to_string()))?;

                        let file_size = metadata.len();
                        let max_size_bytes = self.max_config_size_mb * 1024 * 1024;

                        if file_size > max_size_bytes as u64 {
                            return Err(ConfigError::ConfigTooLarge {
                                path: path.clone(),
                                size_mb: (file_size / (1024 * 1024)) as usize,
                                limit_mb: self.max_config_size_mb,
                            });
                        }
                    }
                }
            }

            let mut file_provider = FileConfigProvider::new(self.explicit_files.clone())
                .with_name("explicit_files")
                .with_priority(40); // Lower priority than environment (loaded first, overridden)

            if let Some(format_mode) = &self.format_detection {
                file_provider = file_provider.with_format_detection(format_mode.clone());
            }

            manager.add_provider(file_provider);
        }
        Ok(())
    }

    /// Setup environment variable provider
    ///
    /// This helper method adds an EnvironmentProvider to the manager if environment
    /// loading is enabled. Environment variables have priority 50 (higher than files).
    #[allow(dead_code)]
    fn setup_env_provider<C: crate::ConfigMap>(&self, manager: &mut ProviderManager) {
        if self.use_env {
            let env_prefix = self.env_prefix.as_deref().unwrap_or("");
            let mut env_provider = EnvironmentProvider::new(env_prefix).with_priority(50);

            // Add custom environment variable mappings from ConfigMap trait
            let custom_mappings = C::env_mapping();
            if !custom_mappings.is_empty() {
                env_provider = env_provider.with_custom_mappings(custom_mappings);
            }

            manager.add_provider(env_provider);
        }
    }

    /// Setup remote configuration providers (HTTP, etcd, Consul)
    ///
    /// This helper method adds remote configuration providers to the manager if they
    /// are configured. All remote providers have priority 50.
    #[cfg(feature = "remote")]
    #[allow(dead_code)]
    fn setup_remote_providers(&self, manager: &mut ProviderManager) {
        // Load HTTP remote config if enabled
        if self.remote_config.enabled {
            if let Some(url) = &self.remote_config.url {
                let mut http_provider = HttpConfigProvider::new(url.clone()).with_priority(50);

                if let Some(token) = &self.remote_config.token {
                    http_provider = http_provider.with_bearer_token_secure(Arc::clone(token));
                }

                if let (Some(username), Some(password)) =
                    (&self.remote_config.username, &self.remote_config.password)
                {
                    http_provider =
                        http_provider.with_auth_secure(username.clone(), Arc::clone(password));
                }

                if let Some(ca_cert) = &self.remote_config.ca_cert {
                    http_provider = http_provider.with_tls(
                        ca_cert.clone(),
                        self.remote_config.client_cert.clone(),
                        self.remote_config.client_key.clone(),
                    );
                }

                manager.add_provider(http_provider);
            }
        }

        // Load etcd config if provided
        if let Some(etcd_provider) = &self.etcd_provider {
            let mut provider = etcd_provider.clone();
            if let (Some(ca_cert), Some(client_cert), Some(client_key)) = (
                self.remote_config.ca_cert.as_ref(),
                self.remote_config.client_cert.as_ref(),
                self.remote_config.client_key.as_ref(),
            ) {
                provider = provider.with_tls(
                    Some(ca_cert.to_string_lossy().into_owned()),
                    Some(client_cert.to_string_lossy().into_owned()),
                    Some(client_key.to_string_lossy().into_owned()),
                );
            } else if let Some(ca_cert) = self.remote_config.ca_cert.as_ref() {
                provider =
                    provider.with_tls(Some(ca_cert.to_string_lossy().into_owned()), None, None);
            }

            // Also apply auth if provided in remote_config
            if let (Some(username), Some(password)) =
                (&self.remote_config.username, &self.remote_config.password)
            {
                provider = provider.with_auth_secure(username.clone(), Arc::clone(password));
            }

            manager.add_provider(provider);
        }

        // Load consul config if provided
        if let Some(consul_provider) = &self.consul_provider {
            let mut provider = consul_provider.clone();
            if let (Some(ca_cert), Some(client_cert), Some(client_key)) = (
                self.remote_config.ca_cert.as_ref(),
                self.remote_config.client_cert.as_ref(),
                self.remote_config.client_key.as_ref(),
            ) {
                provider = provider.with_tls(
                    Some(ca_cert.to_string_lossy().into_owned()),
                    Some(client_cert.to_string_lossy().into_owned()),
                    Some(client_key.to_string_lossy().into_owned()),
                );
            } else if let Some(ca_cert) = self.remote_config.ca_cert.as_ref() {
                provider =
                    provider.with_tls(Some(ca_cert.to_string_lossy().into_owned()), None, None);
            }

            // Also apply token if provided in remote_config
            if let Some(token) = &self.remote_config.token {
                provider = provider.with_token_secure(Arc::clone(token));
            }

            manager.add_provider(provider);
        }
    }

    /// Apply decryption to configuration if encryption is enabled
    /// Apply memory limit check before configuration extraction
    ///
    /// This helper method checks if the current memory usage exceeds the configured
    /// limit and returns an error if it does.
    #[allow(dead_code)]
    #[cfg(feature = "monitoring")]
    fn apply_memory_check(&self) -> Result<(), ConfigError> {
        if self.memory_limit_mb > 0 {
            let current_mb = get_memory_usage_mb().ok_or_else(|| {
                ConfigError::RuntimeError("Failed to get memory usage".to_string())
            })?;

            if current_mb as usize > self.memory_limit_mb {
                return Err(ConfigError::MemoryLimitExceeded {
                    limit: self.memory_limit_mb,
                    current: current_mb as usize,
                });
            }
        }
        Ok(())
    }

    /// Finalize configuration by applying template expansion, sanitization, and validation
    ///
    /// This helper method applies post-processing steps to the extracted configuration:
    /// 1. Template expansion
    /// 2. Sanitization (if configured)
    /// 3. Validation
    #[allow(dead_code)]
    fn finalize_config(&self, mut config: T) -> Result<T, ConfigError>
    where
        T: Serialize + for<'de> Deserialize<'de> + Clone,
    {
        // Apply template expansion
        self.apply_template_expansion(&mut config)?;

        // Apply sanitization if available
        if let Some(sanitizer) = &self.sanitizer {
            config = sanitizer(config)?;
        }

        // Validate configuration - return error if validation fails
        if let Err(ref validation_errors) = config.optional_validate() {
            return Err(ConfigError::ValidationError(validation_errors.to_string()));
        }

        Ok(config)
    }

    /// Helper method to load configuration with a given figment (non-audit version)
    #[allow(clippy::type_complexity)]
    #[allow(dead_code)]
    #[cfg(feature = "audit")]
    async fn load_with_figment(
        &self,
        mut figment: Figment,
        _run_env: Option<String>,
        _app_name: Option<&str>,
        mut audit_info: Option<(
            Vec<(String, String, Option<String>, Option<std::time::Duration>)>,
            std::time::Instant,
        )>,
    ) -> Result<T, ConfigError>
    where
        T: for<'de> Deserialize<'de>
            + Serialize
            + Default
            + Clone
            + OptionalValidate
            + crate::ConfigMap,
    {
        let _load_start = std::time::Instant::now();
        let mut config_sources_status = Vec::new();
        if let Some((ref mut status, _)) = audit_info {
            config_sources_status = status.clone();
        }

        // Initialize ProviderManager
        let mut manager = ProviderManager::new();

        // 1. Add base figment as SerializedProvider (includes defaults)
        manager.add_provider(SerializedProvider::new(figment.clone(), "base_config"));

        // 2. Load explicit files using FileConfigProvider
        let mut _explicit_files_loaded = 0;
        let file_start = std::time::Instant::now();

        if !self.explicit_files.is_empty() {
            // Check file sizes before loading
            if self.max_config_size_mb > 0 {
                for path in &self.explicit_files {
                    if path.exists() {
                        let metadata = std::fs::metadata(path)
                            .map_err(|e| ConfigError::IoError(e.to_string()))?;

                        let file_size = metadata.len();
                        let max_size_bytes = self.max_config_size_mb * 1024 * 1024;

                        if file_size > max_size_bytes as u64 {
                            return Err(ConfigError::ConfigTooLarge {
                                path: path.clone(),
                                size_mb: (file_size / (1024 * 1024)) as usize,
                                limit_mb: self.max_config_size_mb,
                            });
                        }
                    }
                }
            }

            let mut file_provider = FileConfigProvider::new(self.explicit_files.clone())
                .with_name("explicit_files")
                .with_priority(40); // Lower priority than environment (loaded first, overridden)

            if let Some(format_mode) = &self.format_detection {
                file_provider = file_provider.with_format_detection(format_mode.clone());
            }

            manager.add_provider(file_provider);

            // We count loaded files for audit/status purposes
            // This is an approximation since FileConfigProvider handles loading internally
            for file in &self.explicit_files {
                if file.exists() && !is_editor_temp_file(file) {
                    _explicit_files_loaded += 1;
                    config_sources_status.push((
                        format!("explicit_file:{}", file.display()),
                        "Success".to_string(),
                        None,
                        Some(file_start.elapsed()),
                    ));
                }
            }
        }

        // 3. Load environment variables
        if self.use_env {
            let env_prefix = self.env_prefix.as_deref().unwrap_or("");
            let mut env_provider = EnvironmentProvider::new(env_prefix).with_priority(50); // Loaded after files (priority 40), so it can override file values

            // Add custom environment variable mappings from ConfigMap trait
            let custom_mappings = T::env_mapping();
            if !custom_mappings.is_empty() {
                env_provider = env_provider.with_custom_mappings(custom_mappings);
            }

            manager.add_provider(env_provider);
        }

        // 4. Load CLI arguments
        if let Some(cli_provider) = self.cli_provider.clone() {
            manager.add_provider(cli_provider);
        }

        // 5. Load remote config if enabled
        #[cfg(feature = "remote")]
        if self.remote_config.enabled {
            if let Some(url) = &self.remote_config.url {
                let mut http_provider = HttpConfigProvider::new(url.clone()).with_priority(50);

                if let Some(token) = &self.remote_config.token {
                    http_provider = http_provider.with_bearer_token_secure(Arc::clone(token));
                }

                if let (Some(username), Some(password)) =
                    (&self.remote_config.username, &self.remote_config.password)
                {
                    http_provider =
                        http_provider.with_auth_secure(username.clone(), Arc::clone(password));
                }

                if let Some(ca_cert) = &self.remote_config.ca_cert {
                    http_provider = http_provider.with_tls(
                        ca_cert.clone(),
                        self.remote_config.client_cert.clone(),
                        self.remote_config.client_key.clone(),
                    );
                }

                manager.add_provider(http_provider);
            }
        }

        // 6. Load etcd config if provided
        #[cfg(feature = "remote")]
        if let Some(etcd_provider) = &self.etcd_provider {
            let mut provider = etcd_provider.clone();
            if let (Some(ca_cert), Some(client_cert), Some(client_key)) = (
                self.remote_config.ca_cert.as_ref(),
                self.remote_config.client_cert.as_ref(),
                self.remote_config.client_key.as_ref(),
            ) {
                provider = provider.with_tls(
                    Some(ca_cert.to_string_lossy().into_owned()),
                    Some(client_cert.to_string_lossy().into_owned()),
                    Some(client_key.to_string_lossy().into_owned()),
                );
            } else if let Some(ca_cert) = self.remote_config.ca_cert.as_ref() {
                provider =
                    provider.with_tls(Some(ca_cert.to_string_lossy().into_owned()), None, None);
            }

            // Also apply auth if provided in remote_config
            if let (Some(username), Some(password)) =
                (&self.remote_config.username, &self.remote_config.password)
            {
                provider = provider.with_auth_secure(username.clone(), Arc::clone(password));
            }

            manager.add_provider(provider);
        }

        // 7. Load consul config if provided
        #[cfg(feature = "remote")]
        if let Some(consul_provider) = &self.consul_provider {
            let mut provider = consul_provider.clone();
            if let (Some(ca_cert), Some(client_cert), Some(client_key)) = (
                self.remote_config.ca_cert.as_ref(),
                self.remote_config.client_cert.as_ref(),
                self.remote_config.client_key.as_ref(),
            ) {
                provider = provider.with_tls(
                    Some(ca_cert.to_string_lossy().into_owned()),
                    Some(client_cert.to_string_lossy().into_owned()),
                    Some(client_key.to_string_lossy().into_owned()),
                );
            } else if let Some(ca_cert) = self.remote_config.ca_cert.as_ref() {
                provider =
                    provider.with_tls(Some(ca_cert.to_string_lossy().into_owned()), None, None);
            }

            // Also apply token if provided in remote_config
            if let Some(token) = &self.remote_config.token {
                provider = provider.with_token_secure(Arc::clone(token));
            }

            manager.add_provider(provider);
        }

        // 8. Extract and validate configuration using ProviderManager
        figment = manager.load_all()?;

        // Merge with initial figment to preserve profiles/metadata if any
        // Note: load_all returns a new Figment merged from all providers

        #[cfg(feature = "encryption")]
        {
            figment = self.decrypt_figment(figment)?;
        }

        // Check memory limit before extraction
        #[cfg(feature = "monitoring")]
        if self.memory_limit_mb > 0 {
            let current_mb = get_memory_usage_mb().ok_or_else(|| {
                ConfigError::RuntimeError("Failed to get memory usage".to_string())
            })?;

            if current_mb as usize > self.memory_limit_mb {
                return Err(ConfigError::MemoryLimitExceeded {
                    limit: self.memory_limit_mb,
                    current: current_mb as usize,
                });
            }
        }

        let mut config: T = figment
            .extract()
            .map_err(|e| ConfigError::ParseError(e.to_string()))?;

        // Apply template expansion
        self.apply_template_expansion(&mut config)?;

        // Apply sanitization if available
        if let Some(sanitizer) = &self.sanitizer {
            config = sanitizer(config)?;
        }

        // Validate configuration - return error if validation fails (strict mode)
        if let Err(ref validation_errors) = config.optional_validate() {
            return Err(ConfigError::ValidationError(validation_errors.to_string()));
        }

        Ok(config)
    }

    /// Helper method to load configuration with a given figment (non-audit version)
    #[allow(clippy::type_complexity)]
    #[cfg(not(feature = "audit"))]
    async fn load_with_figment(
        &self,
        mut figment: Figment,
        _run_env: Option<String>,
        _app_name: Option<&str>,
        _audit_info: Option<(
            Vec<(String, String, Option<String>, Option<std::time::Duration>)>,
            std::time::Instant,
        )>,
    ) -> Result<T, ConfigError>
    where
        T: for<'de> Deserialize<'de> + Serialize + Default + Clone + crate::ConfigMap,
    {
        // Initialize ProviderManager
        let mut manager = ProviderManager::new();

        // 1. Add base figment as SerializedProvider (includes defaults)
        manager.add_provider(SerializedProvider::new(figment.clone(), "base_config"));

        // 2. Load explicit files using FileConfigProvider
        if !self.explicit_files.is_empty() {
            let mut file_provider = FileConfigProvider::new(self.explicit_files.clone())
                .with_name("explicit_files")
                .with_priority(40); // Loaded before environment (priority 50), so environment can override

            if let Some(format_mode) = &self.format_detection {
                file_provider = file_provider.with_format_detection(format_mode.clone());
            }

            manager.add_provider(file_provider);
        }

        // 3. Load environment variables
        if self.use_env {
            let env_prefix = self.env_prefix.as_deref().unwrap_or("");
            let mut env_provider = EnvironmentProvider::new(env_prefix).with_priority(50); // Higher priority than files (loaded later, overrides file values)

            // Add custom environment variable mappings from ConfigMap trait
            let custom_mappings = T::env_mapping();
            if !custom_mappings.is_empty() {
                env_provider = env_provider.with_custom_mappings(custom_mappings);
            }

            manager.add_provider(env_provider);
        }

        // 4. Load CLI arguments
        if let Some(cli_provider) = self.cli_provider.clone() {
            manager.add_provider(cli_provider);
        }

        // 5. Load remote config if enabled
        #[cfg(feature = "remote")]
        if self.remote_config.enabled {
            if let Some(url) = &self.remote_config.url {
                let mut http_provider = HttpConfigProvider::new(url.clone()).with_priority(50);

                if let Some(token) = &self.remote_config.token {
                    http_provider = http_provider.with_bearer_token_secure(Arc::clone(token));
                }

                if let (Some(username), Some(password)) =
                    (&self.remote_config.username, &self.remote_config.password)
                {
                    http_provider =
                        http_provider.with_auth_secure(username.clone(), Arc::clone(password));
                }

                if let Some(ca_cert) = &self.remote_config.ca_cert {
                    http_provider = http_provider.with_tls(
                        ca_cert.clone(),
                        self.remote_config.client_cert.clone(),
                        self.remote_config.client_key.clone(),
                    );
                }

                manager.add_provider(http_provider);
            }
        }

        // 6. Load etcd config if provided
        #[cfg(feature = "remote")]
        if let Some(etcd_provider) = &self.etcd_provider {
            let mut provider = etcd_provider.clone();
            if let (Some(ca_cert), Some(client_cert), Some(client_key)) = (
                self.remote_config.ca_cert.as_ref(),
                self.remote_config.client_cert.as_ref(),
                self.remote_config.client_key.as_ref(),
            ) {
                provider = provider.with_tls(
                    Some(ca_cert.to_string_lossy().into_owned()),
                    Some(client_cert.to_string_lossy().into_owned()),
                    Some(client_key.to_string_lossy().into_owned()),
                );
            } else if let Some(ca_cert) = self.remote_config.ca_cert.as_ref() {
                provider =
                    provider.with_tls(Some(ca_cert.to_string_lossy().into_owned()), None, None);
            }

            // Also apply auth if provided in remote_config
            if let (Some(username), Some(password)) =
                (&self.remote_config.username, &self.remote_config.password)
            {
                provider = provider.with_auth_secure(username.clone(), Arc::clone(password));
            }

            manager.add_provider(provider);
        }

        // 7. Load consul config if provided
        #[cfg(feature = "remote")]
        if let Some(consul_provider) = &self.consul_provider {
            let mut provider = consul_provider.clone();
            if let (Some(ca_cert), Some(client_cert), Some(client_key)) = (
                self.remote_config.ca_cert.as_ref(),
                self.remote_config.client_cert.as_ref(),
                self.remote_config.client_key.as_ref(),
            ) {
                provider = provider.with_tls(
                    Some(ca_cert.to_string_lossy().into_owned()),
                    Some(client_cert.to_string_lossy().into_owned()),
                    Some(client_key.to_string_lossy().into_owned()),
                );
            } else if let Some(ca_cert) = self.remote_config.ca_cert.as_ref() {
                provider =
                    provider.with_tls(Some(ca_cert.to_string_lossy().into_owned()), None, None);
            }

            // Also apply token if provided in remote_config
            if let Some(token) = &self.remote_config.token {
                provider = provider.with_token_secure(Arc::clone(token));
            }

            manager.add_provider(provider);
        }

        // 8. Extract and validate configuration using ProviderManager
        figment = manager.load_all()?;

        // Merge with initial figment to preserve profiles/metadata if any
        // Note: load_all returns a new Figment merged from all providers

        #[cfg(feature = "encryption")]
        {
            figment = self.decrypt_figment(figment)?;
        }

        // Check memory limit before extraction
        #[cfg(feature = "monitoring")]
        if self.memory_limit_mb > 0 {
            let current_mb = get_memory_usage_mb().ok_or_else(|| {
                ConfigError::RuntimeError("Failed to get memory usage".to_string())
            })?;

            if current_mb as usize > self.memory_limit_mb {
                return Err(ConfigError::MemoryLimitExceeded {
                    limit: self.memory_limit_mb,
                    current: current_mb as usize,
                });
            }
        }

        let mut config: T = figment
            .extract()
            .map_err(|e| ConfigError::ParseError(e.to_string()))?;

        // Apply template expansion
        self.apply_template_expansion(&mut config)?;

        // Apply sanitization if available
        if let Some(sanitizer) = &self.sanitizer {
            config = sanitizer(config)?;
        }

        // Validate configuration - return error if validation fails
        if let Err(ref validation_errors) = config.optional_validate() {
            return Err(ConfigError::ValidationError(validation_errors.to_string()));
        }

        Ok(config)
    }

    /// Load configuration asynchronously with audit support
    #[cfg(all(feature = "audit", feature = "validation"))]
    pub async fn load(&self) -> Result<T, ConfigError>
    where
        T: Sanitize
            + for<'de> Deserialize<'de>
            + Serialize
            + Default
            + Clone
            + OptionalValidate
            + crate::ConfigMap,
    {
        let mut figment = Figment::new();

        // 1. Load defaults if provided
        if let Some(ref defaults) = self.defaults {
            figment = figment.merge(Serialized::from(defaults, "default"));
        }

        // 2. Load standard config files
        let mut _standard_files_loaded = 0;
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
        let mut format_distribution = std::collections::HashMap::new();

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
                    _standard_files_loaded += 1;

                    // Track format distribution
                    *format_distribution.entry(fmt.clone()).or_insert(0) += 1;
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
                        _standard_files_loaded += 1;

                        // Track format distribution for env files
                        *format_distribution.entry(fmt.clone()).or_insert(0) += 1;
                    }
                }
            }
        }

        if _standard_files_loaded == 0 {
            config_sources_status.push((
                "standard_files".to_string(),
                "Skipped".to_string(),
                None,
                None,
            ));
        }

        let audit_info = Some((
            config_sources_status,
            std::time::Instant::now(),
            format_distribution,
        ));
        self.load_with_figment_audit(figment, run_env, app_name, audit_info)
            .await
    }

    /// Load configuration asynchronously with audit support (no validation)
    #[cfg(all(feature = "audit", not(feature = "validation")))]
    pub async fn load(&self) -> Result<T, ConfigError>
    where
        T: Sanitize + for<'de> Deserialize<'de> + Serialize + Default + Clone + crate::ConfigMap,
    {
        let mut figment = Figment::new();

        // 1. Load defaults if provided
        if let Some(ref defaults) = self.defaults {
            figment = figment.merge(Serialized::from(defaults, "default"));
        }

        // 2. Load standard config files
        let mut _standard_files_loaded = 0;
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
        let mut format_distribution = std::collections::HashMap::new();

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
                    _standard_files_loaded += 1;

                    // Track format distribution
                    *format_distribution.entry(fmt.clone()).or_insert(0) += 1;
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
                        _standard_files_loaded += 1;

                        // Track format distribution for env files
                        *format_distribution.entry(fmt.clone()).or_insert(0) += 1;
                    }
                }
            }
        }

        if _standard_files_loaded == 0 {
            config_sources_status.push((
                "standard_files".to_string(),
                "Skipped".to_string(),
                None,
                None,
            ));
        }

        let audit_info = Some((
            config_sources_status,
            std::time::Instant::now(),
            format_distribution,
        ));
        self.load_with_figment_audit(figment, run_env, app_name, audit_info)
            .await
    }

    /// Load configuration synchronously
    #[cfg(feature = "validation")]
    pub fn load_sync(&self) -> Result<T, ConfigError>
    where
        T: Sanitize
            + for<'de> Deserialize<'de>
            + Serialize
            + Default
            + Clone
            + Validate
            + crate::ConfigMap,
    {
        Self::syncify(async { self.load().await })
    }

    /// Load configuration synchronously (without validation)
    #[cfg(not(feature = "validation"))]
    pub fn load_sync(&self) -> Result<T, ConfigError>
    where
        T: Sanitize + for<'de> Deserialize<'de> + Serialize + Default + Clone + crate::ConfigMap,
    {
        Self::syncify(async { self.load().await })
    }

    #[doc(hidden)]
    pub fn syncify<F, R>(f: F) -> Result<R, ConfigError>
    where
        F: std::future::Future<Output = Result<R, ConfigError>>,
    {
        if let Ok(_handle) = tokio::runtime::Handle::try_current() {
            tokio::task::block_in_place(|| {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|e| {
                        ConfigError::RuntimeError(format!("Failed to create runtime: {}", e))
                    })?;
                rt.block_on(f)
            })
        } else {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| {
                    ConfigError::RuntimeError(format!("Failed to create runtime: {}", e))
                })?;
            runtime.block_on(f)
        }
    }

    /// Load configuration synchronously with audit support
    #[cfg(all(feature = "audit", feature = "validation"))]
    pub fn load_sync_with_audit(&self) -> Result<T, ConfigError>
    where
        T: Sanitize
            + for<'de> Deserialize<'de>
            + Serialize
            + Default
            + Clone
            + Validate
            + crate::ConfigMap,
    {
        Self::syncify(async { self.load().await })
    }

    /// Load configuration synchronously with audit support (without validation)
    #[cfg(all(feature = "audit", not(feature = "validation")))]
    pub fn load_sync_with_audit(&self) -> Result<T, ConfigError>
    where
        T: Sanitize + for<'de> Deserialize<'de> + Serialize + Default + Clone + crate::ConfigMap,
    {
        Self::syncify(async { self.load().await })
    }

    /// Load configuration synchronously with watcher support
    #[cfg(all(feature = "watch", feature = "validation"))]
    pub fn load_sync_with_watcher(
        &self,
    ) -> Result<(T, Option<crate::watcher::ConfigWatcher>), ConfigError>
    where
        T: Sanitize
            + for<'de> Deserialize<'de>
            + Serialize
            + Default
            + Clone
            + Validate
            + crate::ConfigMap,
    {
        Self::syncify(async { self.load_with_watcher().await })
    }

    /// Load configuration synchronously with watcher support (without validation)
    #[cfg(all(feature = "watch", not(feature = "validation")))]
    pub fn load_sync_with_watcher(
        &self,
    ) -> Result<(T, Option<crate::watcher::ConfigWatcher>), ConfigError>
    where
        T: Sanitize + for<'de> Deserialize<'de> + Serialize + Default + Clone + crate::ConfigMap,
    {
        Self::syncify(async { self.load_with_watcher().await })
    }

    /// Helper method to load configuration with a given figment (audit version)
    #[cfg(feature = "audit")]
    #[allow(clippy::type_complexity)]
    async fn load_with_figment_audit(
        &self,
        mut figment: Figment,
        _run_env: Option<String>,
        _app_name: &str,
        mut audit_info: Option<(
            Vec<(String, String, Option<String>, Option<std::time::Duration>)>,
            std::time::Instant,
            std::collections::HashMap<String, u32>,
        )>,
    ) -> Result<T, ConfigError>
    where
        T: Sanitize
            + for<'de> Deserialize<'de>
            + Serialize
            + Default
            + Clone
            + OptionalValidate
            + crate::ConfigMap,
    {
        let load_start = std::time::Instant::now();
        let mut config_sources_status = Vec::new();
        let mut format_distribution = std::collections::HashMap::new();
        if let Some((ref mut status, _, ref mut fmt_dist)) = audit_info {
            config_sources_status = status.clone();
            format_distribution = fmt_dist.clone();
        }

        // Initialize ProviderManager
        let mut manager = ProviderManager::new();

        // 1. Add base figment as SerializedProvider (includes defaults)
        manager.add_provider(SerializedProvider::new(figment.clone(), "base_config"));

        // 4. Load explicit files
        let mut _explicit_files_loaded = 0;
        let file_start = std::time::Instant::now();

        if !self.explicit_files.is_empty() {
            let mut file_provider = FileConfigProvider::new(self.explicit_files.clone())
                .with_name("explicit_files")
                .with_priority(40); // Higher priority than environment

            if let Some(format_mode) = &self.format_detection {
                file_provider = file_provider.with_format_detection(format_mode.clone());
            }

            // We count loaded files for audit/status purposes
            // This is an approximation since FileConfigProvider handles loading internally
            for file in &self.explicit_files {
                if file.exists() && !is_editor_temp_file(file) {
                    _explicit_files_loaded += 1;

                    // Detect format for explicit files to track distribution
                    let format = file_provider.detect_format(file);
                    if let Some(fmt) = format {
                        *format_distribution.entry(fmt.clone()).or_insert(0) += 1;
                    }

                    config_sources_status.push((
                        format!("explicit_file:{}", file.display()),
                        "Success".to_string(),
                        None,
                        Some(file_start.elapsed()),
                    ));
                }
            }

            manager.add_provider(file_provider);
        }

        // 5. Load standard config files if app_name is provided
        // This part is a bit tricky because we're already inside load() which handles standard files
        // But load_with_figment_audit is designed to replace the manual loading in load()
        // However, the current implementation of load() already loads standard files into figment
        // BEFORE calling this function. So we don't need to load them again here.
        // We just need to track them for audit purposes, which is passed in audit_info.

        // 6. Load environment variables if enabled
        if self.use_env {
            let env_prefix = self.env_prefix.as_deref().unwrap_or("");
            let mut env_provider = EnvironmentProvider::new(env_prefix)
                .with_custom_mappings(T::env_mapping())
                .with_priority(50);

            // Add custom environment variable mappings from ConfigMap trait
            let custom_mappings = T::env_mapping();
            if !custom_mappings.is_empty() {
                env_provider = env_provider.with_custom_mappings(custom_mappings);
            }

            manager.add_provider(env_provider);
        }

        // 7. Load CLI overrides if available
        if let Some(cli_provider) = self.cli_provider.clone() {
            manager.add_provider(cli_provider);
        }

        // 8. Load remote configuration if enabled
        #[cfg(feature = "remote")]
        if self.remote_config.enabled {
            if let Some(url) = &self.remote_config.url {
                let mut http_provider = HttpConfigProvider::new(url.clone()).with_priority(50);

                if let Some(token) = &self.remote_config.token {
                    http_provider = http_provider.with_bearer_token_secure(Arc::clone(token));
                }

                if let (Some(username), Some(password)) =
                    (&self.remote_config.username, &self.remote_config.password)
                {
                    http_provider =
                        http_provider.with_auth_secure(username.clone(), Arc::clone(password));
                }

                if let Some(ca_cert) = &self.remote_config.ca_cert {
                    http_provider = http_provider.with_tls(
                        ca_cert.clone(),
                        self.remote_config.client_cert.clone(),
                        self.remote_config.client_key.clone(),
                    );
                }

                manager.add_provider(http_provider);
            }
        }

        // 9. Extract and validate configuration using ProviderManager
        figment = manager.load_all()?;

        #[cfg(feature = "encryption")]
        {
            figment = self.decrypt_figment(figment)?;
        }

        // Check memory limit before extraction
        #[cfg(feature = "monitoring")]
        if self.memory_limit_mb > 0 {
            let current_mb = get_memory_usage_mb().ok_or_else(|| {
                ConfigError::RuntimeError("Failed to get memory usage".to_string())
            })?;

            if current_mb as usize > self.memory_limit_mb {
                return Err(ConfigError::MemoryLimitExceeded {
                    limit: self.memory_limit_mb,
                    current: current_mb as usize,
                });
            }
        }

        // Extract configuration
        let mut config: T = figment
            .extract()
            .map_err(|e| ConfigError::ParseError(e.to_string()))?;

        // Apply template expansion
        self.apply_template_expansion(&mut config)?;

        // Apply decryption
        #[cfg(feature = "encryption")]
        self.apply_decryption(&mut config)?;

        // Apply sanitization if available
        if let Some(sanitizer) = &self.sanitizer {
            config = sanitizer(config)?;
        }

        // Apply audit sanitization
        let _sanitized = config.sanitize();

        // Validate configuration
        config.optional_validate()?;

        // 10. Audit logging
        let default_path = self
            .audit
            .file_path
            .as_deref()
            .unwrap_or("config_audit.log");
        let validation_error = None;
        let config_source = Some(format!(
            "Config loaded from {} explicit files",
            _explicit_files_loaded
        ));

        // Calculate load statistics - only explicit files in this function
        let total_files_loaded = _explicit_files_loaded;
        // Use the tracked format distribution instead of creating a new one
        let env_vars_count = std::env::vars().count() as u32;

        // Estimate memory usage (simplified)
        let memory_usage_mb = get_memory_usage_mb();

        let audit_config = AuditConfigComplex {
            validation_error,
            config_source,
            load_duration: Some(load_start.elapsed()),
            config_sources_status: Some(config_sources_status),
            files_attempted: Some(total_files_loaded),
            files_loaded: Some(total_files_loaded),
            format_distribution: Some(format_distribution),
            env_vars_count: Some(env_vars_count),
            memory_usage_mb,
        };

        if let Err(e) = AuditLogger::log_to_file_with_source(
            &config,
            std::path::Path::new(&default_path),
            audit_config,
        ) {
            eprintln!("Warning: Failed to write audit log: {}", e);
        }

        Ok(config)
    }

    /// Load configuration asynchronously without audit support
    #[cfg(all(not(feature = "audit"), feature = "validation"))]
    pub async fn load(&self) -> Result<T, ConfigError>
    where
        T: for<'de> Deserialize<'de> + Serialize + Default + Clone + crate::ConfigMap,
    {
        let mut figment = Figment::new();

        // 1. Load defaults if provided
        if let Some(ref defaults) = self.defaults {
            figment = figment.merge(Serialized::from(defaults, "default"));
        }

        // 2. Load standard config files
        let mut _standard_files_loaded = 0;
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
                    _standard_files_loaded += 1;
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
                        _standard_files_loaded += 1;
                    }
                }
            }
        }

        self.load_with_figment(figment, run_env, Some(app_name), None)
            .await
    }

    /// Load configuration asynchronously without audit support (no validation)
    #[cfg(all(not(feature = "audit"), not(feature = "validation")))]
    pub async fn load(&self) -> Result<T, ConfigError>
    where
        T: for<'de> Deserialize<'de> + Serialize + Default + Clone + crate::ConfigMap,
    {
        let mut figment = Figment::new();

        // 1. Load defaults if provided
        if let Some(ref defaults) = self.defaults {
            figment = figment.merge(Serialized::from(defaults, "default"));
        }

        // 2. Load standard config files
        let mut _standard_files_loaded = 0;
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
                    _standard_files_loaded += 1;
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
                        _standard_files_loaded += 1;
                    }
                }
            }
        }

        self.load_with_figment(figment, run_env, Some(app_name), None)
            .await
    }

    /// Load configuration with file watching
    #[cfg(all(feature = "watch", feature = "validation"))]
    pub async fn load_with_watcher(
        &self,
    ) -> Result<(T, Option<crate::watcher::ConfigWatcher>), ConfigError>
    where
        T: Sanitize
            + for<'de> Deserialize<'de>
            + Serialize
            + Default
            + Clone
            + crate::ConfigMap
            + Validate,
    {
        let explicit_files = self.explicit_files.clone();
        let watch = self.watch;
        let config = self.load().await?;

        let watcher = if watch {
            Some(crate::watcher::ConfigWatcher::new(explicit_files))
        } else {
            None
        };

        Ok((config, watcher))
    }

    /// Load configuration with file watching (without validation)
    #[cfg(all(feature = "watch", not(feature = "validation")))]
    pub async fn load_with_watcher(
        &self,
    ) -> Result<(T, Option<crate::watcher::ConfigWatcher>), ConfigError>
    where
        T: Sanitize + for<'de> Deserialize<'de> + Serialize + Default + Clone + crate::ConfigMap,
    {
        let explicit_files = self.explicit_files.clone();
        let watch = self.watch;
        let config = self.load().await?;

        let watcher = if watch {
            Some(crate::watcher::ConfigWatcher::new(explicit_files))
        } else {
            None
        };

        Ok((config, watcher))
    }

    /// Expand template variables in a value recursively
    fn expand_templates_recursive(&self, value: &mut Value) -> bool {
        match value {
            Value::String(tag, s) => {
                if s.contains("${") {
                    let expanded = self.expand_templates(s).unwrap_or_else(|| s.clone());
                    *value = Value::String(*tag, expanded);
                    true
                } else {
                    false
                }
            }
            Value::Dict(_tag, dict) => {
                let mut changed = false;
                for v in dict.values_mut() {
                    if self.expand_templates_recursive(v) {
                        changed = true;
                    }
                }
                changed
            }
            Value::Array(_tag, array) => {
                let mut changed = false;
                for v in array.iter_mut() {
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

                // Try with env prefix first, then without prefix
                let env_value = if let Some(prefix) = &self.env_prefix {
                    let prefixed_name = format!("{}_{}", prefix, var_name);
                    std::env::var(&prefixed_name).or_else(|_| std::env::var(var_name))
                } else {
                    std::env::var(var_name)
                };

                if let Ok(env_value) = env_value {
                    // Security: Validate environment value before substitution
                    // Block potentially dangerous characters that could enable injection attacks
                    if Self::is_safe_env_value(&env_value) {
                        result.replace_range(var_start..=var_end, &env_value);
                        start = var_start + env_value.len();
                    } else {
                        #[cfg(feature = "tracing")]
                        tracing::warn!(
                            "Environment variable '{}' contains unsafe characters, skipping substitution",
                            var_name
                        );
                        start = var_end + 1;
                    }
                } else {
                    start = var_end + 1;
                }
            } else {
                break;
            }
        }

        Some(result)
    }

    /// Check if an environment variable value is safe to substitute
    /// Blocks characters that could enable injection attacks
    fn is_safe_env_value(value: &str) -> bool {
        // Check for dangerous shell characters and injection patterns
        let dangerous_patterns = [';', '|', '&', '$', '`', '\'', '"', '\\', '\n', '\r', '\0'];

        // Check for dangerous patterns
        if value.contains("&&") || value.contains("||") {
            return false;
        }
        if value.contains("${") || value.contains("$(") {
            return false;
        }

        // Check for dangerous characters
        !value.chars().any(|c| dangerous_patterns.contains(&c))
    }

    /// Decrypt encrypted values recursively
    #[cfg(feature = "encryption")]
    #[allow(clippy::only_used_in_recursion)]
    fn decrypt_value_recursive(&self, value: &mut Value, encryptor: &ConfigEncryption) -> bool {
        match value {
            Value::String(_tag, s) => {
                if s.starts_with("enc:AES256GCM:") {
                    if let Ok(decrypted) = encryptor.decrypt(s) {
                        *value = Value::String(*_tag, decrypted);
                        true
                    } else {
                        false
                    }
                } else if s.starts_with("ENC(") && s.ends_with(")") {
                    let encrypted = &s[4..s.len() - 1];
                    if let Ok(decrypted) = encryptor.decrypt(encrypted) {
                        *value = Value::String(*_tag, decrypted);
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            Value::Dict(_tag, dict) => {
                let mut changed = false;
                for v in dict.values_mut() {
                    if self.decrypt_value_recursive(v, encryptor) {
                        changed = true;
                    }
                }
                changed
            }
            Value::Array(_tag, array) => {
                let mut changed = false;
                for v in array.iter_mut() {
                    if self.decrypt_value_recursive(v, encryptor) {
                        changed = true;
                    }
                }
                changed
            }
            _ => false,
        }
    }

    /// Apply template expansion and decryption to a configuration object
    fn apply_template_expansion<U>(&self, config: &mut U) -> Result<(), ConfigError>
    where
        U: Serialize + for<'de> Deserialize<'de> + Clone,
    {
        // Serialize the config to a Value
        let mut value = Value::serialize(config.clone())
            .map_err(|e| ConfigError::ParseError(format!("Failed to serialize config: {}", e)))?;

        // Try to decrypt values if encryption key is available
        #[cfg(feature = "encryption")]
        {
            if let Ok(encryptor) = ConfigEncryption::from_env() {
                self.decrypt_value_recursive(&mut value, &encryptor);
            }
        }

        // Expand templates recursively
        self.expand_templates_recursive(&mut value);

        // Deserialize back to the config type
        *config = value
            .deserialize()
            .map_err(|e| ConfigError::ParseError(format!("Failed to deserialize config: {}", e)))?;

        Ok(())
    }

    /// Apply decryption to configuration values
    #[cfg(feature = "encryption")]
    #[allow(dead_code)]
    fn apply_decryption<U>(&self, config: &mut U) -> Result<(), ConfigError>
    where
        U: Serialize + for<'de> Deserialize<'de> + Clone,
    {
        // Check if encryption key is available
        if let Ok(encryptor) = ConfigEncryption::from_env() {
            // Serialize the config to a Value
            let mut value = Value::serialize(config.clone()).map_err(|e| {
                ConfigError::ParseError(format!("Failed to serialize config: {}", e))
            })?;

            // Decrypt values recursively
            self.decrypt_value_recursive(&mut value, &encryptor);

            // Deserialize back to the config type
            match value.deserialize::<U>() {
                Ok(deserialized) => {
                    *config = deserialized;
                }
                Err(e) => {
                    return Err(ConfigError::ParseError(format!(
                        "Failed to deserialize config: {}",
                        e
                    )));
                }
            }
        }

        Ok(())
    }

    /// Decrypt encrypted values in a figment before extraction
    #[cfg(feature = "encryption")]
    fn decrypt_figment(&self, figment: Figment) -> Result<Figment, ConfigError> {
        // Try to get encryption key from environment
        if let Ok(encryptor) = ConfigEncryption::from_env() {
            // Extract the figment as a Value first
            // We use extract_inner to get the merged value without validation
            // If extraction fails, we fallback to an empty dict
            let mut value = match figment.extract_inner::<Value>("") {
                Ok(v) => v,
                Err(_) => Value::Dict(Tag::Default, std::collections::BTreeMap::new()),
            };

            // Apply decryption recursively
            self.decrypt_value_recursive(&mut value, &encryptor);

            // Create a new figment with the decrypted value
            // We merge the decrypted value ON TOP of the original figment
            // This ensures decrypted values take precedence
            let decrypted_figment = Figment::new()
                .merge(figment)
                .merge(Serialized::from(value, "decrypted"));

            return Ok(decrypted_figment);
        }

        Ok(figment)
    }
}

/// Check if a file is an editor temporary file
pub fn is_editor_temp_file(path: &Path) -> bool {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");

    // 避免单字符匹配（如单独的"#"不应被视为临时文件）
    if file_name.len() <= 1 {
        return file_name.ends_with('~');
    }

    file_name.ends_with('~')
        || file_name.starts_with('.') && file_name.ends_with('.')
        || file_name.starts_with('#') && file_name.ends_with('#')
        || file_name.ends_with(".swp")
        || file_name.ends_with(".swo")
        || file_name.ends_with(".tmp")
}
