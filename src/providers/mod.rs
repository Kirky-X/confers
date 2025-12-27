// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

pub mod environment_provider;
pub mod provider;

#[cfg(feature = "remote")]
pub mod remote;

pub mod cli_provider;
pub mod default_provider;
pub mod file_provider;

#[cfg(feature = "remote")]
pub mod consul_provider;
#[cfg(feature = "remote")]
pub mod etcd_provider;
#[cfg(feature = "remote")]
pub mod http_provider;

pub use provider::{
    ConfigProvider, ProviderBuilder, ProviderManager, ProviderMetadata, ProviderStatus,
    ProviderType, SerializedProvider, WatchableProvider,
};
