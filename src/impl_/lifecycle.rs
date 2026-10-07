// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! Component lifecycle management.
//!
//! Provides the `Lifecycle` trait for components that manage
//! background resources and the `LifecycleRegistry` that collects
//! components registered through
//! [`ConfigBuilder::register_lifecycle`](crate::config::ConfigBuilder::register_lifecycle).
//!
//! # Feature-gated async/sync
//!
//! When any async feature is enabled (remote, config-bus, encryption, watch),
//! the async version (using `async_trait`) is used. Otherwise, a sync
//! version compiled with zero async dependencies takes its place.
//!
//! # Design (ADR-041)
//!
//! - `start()` is idempotent
//! - `stop()` must flush all pending persistent operations

use crate::error::{ConfigConfigError, ConfigResult};

#[cfg(feature = "async-core")]
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
    }
}

#[cfg(not(feature = "async-core"))]
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
}

#[cfg(feature = "async-core")]
pub use async_impl::Lifecycle;
#[cfg(feature = "async-core")]
pub(crate) use async_impl::LifecycleRegistry;
#[cfg(not(feature = "async-core"))]
pub use sync_impl::Lifecycle;

#[cfg(test)]
mod tests {
    use crate::error::ConfigConfigError;

    struct TestComponent {
        started: std::sync::atomic::AtomicBool,
        stopped: std::sync::atomic::AtomicBool,
    }

    impl TestComponent {
        fn new() -> Self {
            Self {
                started: std::sync::atomic::AtomicBool::new(false),
                stopped: std::sync::atomic::AtomicBool::new(false),
            }
        }
    }

    // Lifecycle is imported from the active impl (sync or async depending on features)
    #[cfg(feature = "async-core")]
    mod async_tests {
        use super::*;
        use crate::Lifecycle;

        struct AsyncComponent(TestComponent);

        #[async_trait::async_trait]
        impl crate::Lifecycle for AsyncComponent {
            async fn start(&self) -> Result<(), ConfigConfigError> {
                self.0
                    .started
                    .store(true, std::sync::atomic::Ordering::Release);
                Ok(())
            }
            async fn stop(&self) -> crate::ConfigResult<()> {
                self.0
                    .stopped
                    .store(true, std::sync::atomic::Ordering::Release);
                Ok(())
            }
        }

        #[tokio::test]
        async fn test_component_lifecycle_start_stop() {
            let comp = AsyncComponent(TestComponent::new());
            assert!(!comp.0.started.load(std::sync::atomic::Ordering::Acquire));
            comp.start().await.unwrap();
            assert!(comp.0.started.load(std::sync::atomic::Ordering::Acquire));
            comp.stop().await.unwrap();
            assert!(comp.0.stopped.load(std::sync::atomic::Ordering::Acquire));
        }

        #[tokio::test]
        async fn test_lifecycle_default_impl() {
            struct NoopComponent;
            #[async_trait::async_trait]
            impl crate::Lifecycle for NoopComponent {}

            let comp = NoopComponent;
            assert!(comp.start().await.is_ok());
            assert!(comp.stop().await.is_ok());
        }
    }

    #[cfg(not(feature = "async-core"))]
    mod sync_tests {
        use super::*;
        // Bring the `Lifecycle` trait into scope so its `start()`/`stop()`
        // methods are callable on `SyncComponent` (which `impl`s the trait via
        // the fully-qualified `crate::Lifecycle` path). Without this import the
        // compiler reports E0599 "method not found" under default features.
        use crate::Lifecycle;

        struct SyncComponent(TestComponent);

        impl crate::Lifecycle for SyncComponent {
            fn start(&self) -> Result<(), ConfigConfigError> {
                self.0
                    .started
                    .store(true, std::sync::atomic::Ordering::Release);
                Ok(())
            }
            fn stop(&self) -> crate::ConfigResult<()> {
                self.0
                    .stopped
                    .store(true, std::sync::atomic::Ordering::Release);
                Ok(())
            }
        }

        #[test]
        fn test_component_lifecycle_start_stop() {
            let comp = SyncComponent(TestComponent::new());
            assert!(!comp.0.started.load(std::sync::atomic::Ordering::Acquire));
            comp.start().unwrap();
            assert!(comp.0.started.load(std::sync::atomic::Ordering::Acquire));
            comp.stop().unwrap();
            assert!(comp.0.stopped.load(std::sync::atomic::Ordering::Acquire));
        }
    }
}
