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
        #[allow(dead_code)]
        pub(crate) fn is_empty(&self) -> bool {
            self.components.is_empty()
        }
        #[allow(dead_code)]
        pub(crate) fn len(&self) -> usize {
            self.components.len()
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
        #[allow(dead_code)]
        pub(crate) fn is_empty(&self) -> bool {
            self.components.is_empty()
        }
        pub(crate) fn len(&self) -> usize {
            self.components.len()
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
    #[cfg(any(
        feature = "remote",
        feature = "config-bus",
        feature = "encryption",
        feature = "watch"
    ))]
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

        #[tokio::test]
        async fn test_registry_start_all_stop_all() {
            use super::super::async_impl::LifecycleRegistry;
            use std::sync::Arc;

            let mut reg = LifecycleRegistry::new();
            let c1 = Arc::new(AsyncComponent(TestComponent::new()));
            let c2 = Arc::new(AsyncComponent(TestComponent::new()));

            reg.register("comp1", c1.clone());
            reg.register("comp2", c2.clone());

            reg.start_all().await.unwrap();
            assert!(c1.0.started.load(std::sync::atomic::Ordering::Acquire));
            assert!(c2.0.started.load(std::sync::atomic::Ordering::Acquire));

            reg.stop_all().await;
            assert!(c1.0.stopped.load(std::sync::atomic::Ordering::Acquire));
            assert!(c2.0.stopped.load(std::sync::atomic::Ordering::Acquire));
        }

        #[tokio::test]
        async fn test_registry_stop_reverse_order() {
            use super::super::async_impl::LifecycleRegistry;
            use std::sync::Arc;

            let order: Vec<&str> = Vec::new();
            let order = std::sync::Arc::new(std::sync::Mutex::new(order));

            struct OrderedComp {
                name: &'static str,
                order: Arc<std::sync::Mutex<Vec<&'static str>>>,
            }
            #[async_trait::async_trait]
            impl crate::Lifecycle for OrderedComp {
                async fn stop(&self) -> crate::ConfigResult<()> {
                    self.order.lock().unwrap().push(self.name);
                    Ok(())
                }
            }

            let mut reg = LifecycleRegistry::new();
            let c1 = Arc::new(OrderedComp {
                name: "a",
                order: order.clone(),
            });
            let c2 = Arc::new(OrderedComp {
                name: "b",
                order: order.clone(),
            });
            let c3 = Arc::new(OrderedComp {
                name: "c",
                order: order.clone(),
            });

            reg.register("c", c3);
            reg.register("b", c2);
            reg.register("a", c1);

            reg.start_all().await.unwrap();
            reg.stop_all().await;
            let stopped_order = order.lock().unwrap().clone();
            assert_eq!(
                stopped_order,
                vec!["a", "b", "c"],
                "should stop in reverse registration order"
            );
        }

        #[tokio::test]
        async fn test_registry_empty() {
            use super::super::async_impl::LifecycleRegistry;
            let reg = LifecycleRegistry::new();
            assert!(reg.is_empty());
            reg.start_all().await.unwrap();
            reg.stop_all().await;
        }
    }

    #[cfg(not(any(
        feature = "remote",
        feature = "config-bus",
        feature = "encryption",
        feature = "watch"
    )))]
    mod sync_tests {
        use super::*;

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

        #[test]
        fn test_registry_start_all_stop_all() {
            use crate::impl_::lifecycle::sync_impl::LifecycleRegistry;
            let mut reg = LifecycleRegistry::new();
            let c1 = Box::new(SyncComponent(TestComponent::new()));
            let c2 = Box::new(SyncComponent(TestComponent::new()));

            reg.register("comp1", c1);
            reg.register("comp2", c2);

            assert!(!reg.is_empty());
            reg.start_all().unwrap();
            reg.stop_all();
        }
    }
}
