// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Remote configuration sources.

pub(crate) mod common;
mod interval;

#[cfg(feature = "consul")]
pub(crate) mod consul;
#[cfg(feature = "etcd")]
pub(crate) mod etcd;
pub(crate) mod poll;

pub use interval::PollInterval;

#[cfg(feature = "consul")]
pub use consul::{ConsulSource, ConsulSourceBuilder, ConsulTlsConfig};
#[cfg(feature = "etcd")]
pub use etcd::{EtcdSource, EtcdSourceBuilder, EtcdTlsConfig};
pub use poll::{HttpPolledSource, HttpPolledSourceBuilder, PolledSource};
