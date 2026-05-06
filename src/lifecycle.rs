//! Component lifecycle management.
//!
//! Provides the `Lifecycle` trait for components that manage
//! background resources and the `LifecycleRegistry` for ordered
//! startup/shutdown sequencing.
//!
//! # Feature-gated async/sync
//!
//! When any async feature is enabled (remote, config-bus, encryption, watch),
//! the async version (using `async_trait`) is used. Otherwise, a sync
//! # Design (ADR-041)
//!
//! - `start()` is idempotent
//! - `stop()` must flush all pending persistent operations
//! - FIFO start / LIFO stop order

use crate::error::{ConfigConfigError, ConfigResult};

#[cfg(any(
    feature = "remote",
    feature = "config-bus",
    feature = "encryption",
    feature = "watch"
))]
mod async_impl {
    use super::*;
    use async_trait::async_trait;
    use std::sync::Arc;

    #[async_trait]
    pub trait Lifecycle: Send + Sync {
        async fn start(&self) -> Result<(), ConfigConfigError> {
            Ok(())
        }
        async fn stop(&self) -> ConfigResult<()> {
            Ok(())
        }
    }

    pub(crate) struct LifecycleRegistry {
        components: Vec<(String, Arc<dyn Lifecycle>)>,
    }

    impl LifecycleRegistry {
        pub(crate) fn new() -> Self {
            Self {
                components: Vec::new(),
            }
        }
        pub(crate) fn register(&mut self, name: impl Into<String>, component: Arc<dyn Lifecycle>) {
            self.components.push((name.into(), component));
        }
        #[allow(dead_code)]
        pub(crate) async fn start_all(&self) -> Result<(), ConfigConfigError> {
            for (name, c) in &self.components {
                c.start()
                    .await
                    .map_err(|e| ConfigConfigError::InvalidValue {
                        field: "lifecycle".into(),
                        expected_type: "operational".into(),
                        message: format!("{} failed: {}", name, e),
                    })?;
            }
            Ok(())
        }
        #[allow(dead_code)]
        pub(crate) async fn stop_all(&self) {
            for c in self.components.iter().rev() {
                let _ = c.1.stop().await;
            }
        }
    }
}

#[cfg(not(any(
    feature = "remote",
    feature = "config-bus",
    feature = "encryption",
    feature = "watch"
)))]
mod sync_impl {
    use super::*;

    pub trait Lifecycle: Send + Sync {
        fn start(&self) -> Result<(), ConfigConfigError> {
            Ok(())
        }
        fn stop(&self) -> ConfigResult<()> {
            Ok(())
        }
    }

    pub(crate) struct LifecycleRegistry {
        components: Vec<(String, Box<dyn Lifecycle>)>,
    }

    impl LifecycleRegistry {
        pub(crate) fn new() -> Self {
            Self {
                components: Vec::new(),
            }
        }
        pub(crate) fn register(&mut self, name: impl Into<String>, component: Box<dyn Lifecycle>) {
            self.components.push((name.into(), component));
        }
        #[allow(dead_code)]
        pub(crate) fn start_all(&self) -> Result<(), ConfigConfigError> {
            for (name, c) in &self.components {
                c.start().map_err(|e| ConfigConfigError::InvalidValue {
                    field: "lifecycle".into(),
                    expected_type: "operational".into(),
                    message: format!("{} failed: {}", name, e),
                })?;
            }
            Ok(())
        }
        #[allow(dead_code)]
        pub(crate) fn stop_all(&self) {
            for c in self.components.iter().rev() {
                let _ = c.1.stop();
            }
        }
    }
}

#[cfg(any(
    feature = "remote",
    feature = "config-bus",
    feature = "encryption",
    feature = "watch"
))]
pub use async_impl::Lifecycle;
#[cfg(any(
    feature = "remote",
    feature = "config-bus",
    feature = "encryption",
    feature = "watch"
))]
pub(crate) use async_impl::LifecycleRegistry;
#[cfg(not(any(
    feature = "remote",
    feature = "config-bus",
    feature = "encryption",
    feature = "watch"
)))]
pub use sync_impl::Lifecycle;
#[cfg(not(any(
    feature = "remote",
    feature = "config-bus",
    feature = "encryption",
    feature = "watch"
)))]
pub(crate) use sync_impl::LifecycleRegistry;
