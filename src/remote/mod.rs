//! Remote configuration sources.

mod interval;

#[cfg(feature = "consul")]
pub mod consul;
#[cfg(feature = "etcd")]
pub mod etcd;
pub mod poll;

pub use interval::PollInterval;

#[cfg(feature = "consul")]
pub use consul::{ConsulSource, ConsulSourceBuilder};
#[cfg(feature = "etcd")]
pub use etcd::{EtcdSource, EtcdSourceBuilder, EtcdTlsConfig};
pub use poll::{HttpPolledSource, HttpPolledSourceBuilder, PolledSource};
