use crate::error::ConfigError;
use figment::Figment;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Duration;

/// Configuration provider trait that abstracts different config sources
pub trait ConfigProvider: Send + Sync {
    /// Load configuration from this provider
    fn load(&self) -> Result<Figment, ConfigError>;

    /// Get provider name for identification
    fn name(&self) -> &str;

    /// Check if this provider is available (e.g., files exist, network reachable)
    fn is_available(&self) -> bool;

    /// Get provider priority (lower number = higher priority)
    fn priority(&self) -> u8;

    /// Get provider metadata
    fn metadata(&self) -> ProviderMetadata;

    /// Convert to Any for downcasting
    fn as_any(&self) -> &dyn std::any::Any;
}

/// Metadata about a configuration provider
#[derive(Debug, Clone)]
pub struct ProviderMetadata {
    pub name: String,
    pub description: String,
    pub source_type: ProviderType,
    pub requires_network: bool,
    pub supports_watch: bool,
    pub priority: u8,
}

/// Type of configuration provider
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderType {
    File,
    Environment,
    Remote,
    Cli,
    Serialized,
    Default,
}

/// Configuration provider that can be watched for changes
pub trait WatchableProvider: ConfigProvider {
    /// Start watching for changes
    fn start_watching(&mut self) -> Result<(), ConfigError>;

    /// Stop watching for changes
    fn stop_watching(&mut self) -> Result<(), ConfigError>;

    /// Check if watching is enabled
    fn is_watching(&self) -> bool;

    /// Get poll interval for remote providers
    fn poll_interval(&self) -> Option<Duration>;
}

/// Wrapper for watchable providers that handles interior mutability
pub struct WatchableProviderWrapper {
    inner: Arc<RwLock<Box<dyn WatchableProvider>>>,
    name: String,
}

impl WatchableProviderWrapper {
    pub fn new(provider: Box<dyn WatchableProvider>) -> Self {
        let name = provider.name().to_string();
        Self {
            inner: Arc::new(RwLock::new(provider)),
            name,
        }
    }

    pub fn start_watching_safe(&self) -> Result<(), ConfigError> {
        match self.inner.try_write() {
            Ok(mut guard) => guard.start_watching(),
            Err(_) => Err(ConfigError::RuntimeError(
                "Failed to acquire write lock for start_watching".to_string(),
            )),
        }
    }

    pub fn stop_watching_safe(&self) -> Result<(), ConfigError> {
        match self.inner.try_write() {
            Ok(mut guard) => guard.stop_watching(),
            Err(_) => Err(ConfigError::RuntimeError(
                "Failed to acquire write lock for stop_watching".to_string(),
            )),
        }
    }
}

impl ConfigProvider for WatchableProviderWrapper {
    fn load(&self) -> Result<Figment, ConfigError> {
        match self.inner.try_read() {
            Ok(guard) => guard.load(),
            Err(_) => Err(ConfigError::RuntimeError(
                "Failed to acquire read lock for load".to_string(),
            )),
        }
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn is_available(&self) -> bool {
        match self.inner.try_read() {
            Ok(guard) => guard.is_available(),
            Err(_) => false,
        }
    }

    fn priority(&self) -> u8 {
        match self.inner.try_read() {
            Ok(guard) => guard.priority(),
            Err(_) => 0,
        }
    }

    fn metadata(&self) -> ProviderMetadata {
        match self.inner.try_read() {
            Ok(guard) => guard.metadata(),
            Err(_) => ProviderMetadata {
                name: self.name.clone(),
                description: "Unknown provider".to_string(),
                source_type: ProviderType::Default,
                requires_network: false,
                supports_watch: false,
                priority: 255,
            },
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl WatchableProvider for WatchableProviderWrapper {
    fn start_watching(&mut self) -> Result<(), ConfigError> {
        match self.inner.try_write() {
            Ok(mut guard) => guard.start_watching(),
            Err(_) => Err(ConfigError::RuntimeError(
                "Failed to acquire write lock for start_watching".to_string(),
            )),
        }
    }

    fn stop_watching(&mut self) -> Result<(), ConfigError> {
        match self.inner.try_write() {
            Ok(mut guard) => guard.stop_watching(),
            Err(_) => Err(ConfigError::RuntimeError(
                "Failed to acquire write lock for stop_watching".to_string(),
            )),
        }
    }

    fn is_watching(&self) -> bool {
        match self.inner.try_read() {
            Ok(guard) => guard.is_watching(),
            Err(_) => false,
        }
    }

    fn poll_interval(&self) -> Option<Duration> {
        match self.inner.try_read() {
            Ok(guard) => guard.poll_interval(),
            Err(_) => None,
        }
    }
}

/// Provider manager that coordinates multiple providers
pub struct ProviderManager {
    providers: Vec<Arc<dyn ConfigProvider>>,
    watch_providers: Vec<Arc<dyn WatchableProvider>>,
}

impl ProviderManager {
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
            watch_providers: Vec::new(),
        }
    }

    pub fn add_provider<P: ConfigProvider + 'static>(&mut self, provider: P) -> &mut Self {
        self.providers.push(Arc::new(provider));
        self
    }

    pub fn add_watchable_provider<P: WatchableProvider + 'static>(
        &mut self,
        provider: P,
    ) -> &mut Self {
        let wrapper = Arc::new(WatchableProviderWrapper::new(Box::new(provider)));
        // Add to both vectors since watchable providers should also be loadable
        self.watch_providers.push(wrapper.clone());
        self.providers.push(wrapper);
        self
    }

    pub fn load_all(&self) -> Result<Figment, ConfigError> {
        let mut figment = Figment::new();

        // Create a sorted vector of references to providers
        let mut provider_refs: Vec<&Arc<dyn ConfigProvider>> = self.providers.iter().collect();

        // Sort by priority (lower number = higher priority)
        provider_refs.sort_by_key(|p| p.priority());

        for provider_ref in provider_refs {
            let is_available = provider_ref.is_available();

            if is_available {
                match provider_ref.load() {
                    Ok(provider_figment) => {
                        figment = figment.merge(provider_figment);
                    }
                    Err(e) => {
                        return Err(e);
                    }
                }
            }
        }

        Ok(figment)
    }

    pub fn start_watching(&mut self) -> Result<(), ConfigError> {
        for provider in &self.watch_providers {
            // Downcast to WatchableProviderWrapper to use safe methods
            if let Some(wrapper) = provider.as_any().downcast_ref::<WatchableProviderWrapper>() {
                wrapper.start_watching_safe()?;
            }
        }
        Ok(())
    }

    pub fn stop_watching(&mut self) -> Result<(), ConfigError> {
        for provider in &self.watch_providers {
            // Downcast to WatchableProviderWrapper to use safe methods
            if let Some(wrapper) = provider.as_any().downcast_ref::<WatchableProviderWrapper>() {
                wrapper.stop_watching_safe()?;
            }
        }
        Ok(())
    }

    pub fn get_provider_status(&self) -> HashMap<String, ProviderStatus> {
        let mut status = HashMap::new();

        for provider in &self.providers {
            let metadata = provider.metadata();
            status.insert(
                metadata.name.clone(),
                ProviderStatus {
                    available: provider.is_available(),
                    priority: provider.priority(),
                    metadata,
                },
            );
        }

        status
    }
}

impl Default for ProviderManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Status information for a provider
#[derive(Debug, Clone)]
pub struct ProviderStatus {
    pub available: bool,
    pub priority: u8,
    pub metadata: ProviderMetadata,
}

/// Builder for creating configuration providers
pub struct ProviderBuilder {
    providers: Vec<Arc<dyn ConfigProvider>>,
}

impl ProviderBuilder {
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
        }
    }

    pub fn build(self) -> ProviderManager {
        ProviderManager {
            providers: self.providers,
            watch_providers: Vec::new(),
        }
    }
}

impl Default for ProviderBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Serialized data configuration provider
pub struct SerializedProvider {
    figment: Figment,
    name: String,
}

impl SerializedProvider {
    pub fn new(figment: Figment, name: impl Into<String>) -> Self {
        Self {
            figment,
            name: name.into(),
        }
    }
}

impl ConfigProvider for SerializedProvider {
    fn load(&self) -> Result<Figment, ConfigError> {
        Ok(self.figment.clone())
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn is_available(&self) -> bool {
        true
    }

    fn priority(&self) -> u8 {
        5 // Very low priority for defaults
    }

    fn metadata(&self) -> ProviderMetadata {
        ProviderMetadata {
            name: self.name.clone(),
            description: format!("Serialized data provider '{}'", self.name),
            source_type: ProviderType::Serialized,
            requires_network: false,
            supports_watch: false,
            priority: self.priority(),
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
