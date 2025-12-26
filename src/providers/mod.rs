pub mod cli;
pub mod env;
pub mod file;
pub mod provider;

#[cfg(feature = "remote")]
pub mod remote;

// ConfigProvider trait adapters
pub mod cli_provider;
pub mod default_provider;
pub mod environment_provider;
pub mod file_provider;

#[cfg(feature = "remote")]
pub mod consul_provider;
#[cfg(feature = "remote")]
pub mod etcd_provider;
#[cfg(feature = "remote")]
pub mod http_provider;

// Re-export the provider trait and related types
pub use provider::{
    ConfigProvider, ProviderBuilder, ProviderManager, ProviderMetadata, ProviderStatus,
    ProviderType, SerializedProvider, WatchableProvider,
};
