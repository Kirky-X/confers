//! ConfigBus - Multi-instance configuration change broadcast.

use std::pin::Pin;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use futures_util::{Stream, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;

use crate::error::ConfigResult;

#[cfg(feature = "nats-bus")]
mod nats;
#[cfg(feature = "redis-bus")]
mod redis;

#[cfg(feature = "nats-bus")]
pub use nats::{NatsBusBuilder, NatsConfigBus};
#[cfg(feature = "redis-bus")]
pub use redis::{RedisBusBuilder, RedisConfigBus};

/// Configuration change event for multi-instance synchronization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigChangeEvent {
    pub instance_id: String,
    pub timestamp: DateTime<Utc>,
    pub source: String,
    pub changed_keys: Vec<String>,
    pub checksum: String,
}

impl ConfigChangeEvent {
    pub fn new(
        instance_id: impl Into<String>,
        source: impl Into<String>,
        changed_keys: Vec<String>,
        checksum: impl Into<String>,
    ) -> Self {
        Self {
            instance_id: instance_id.into(),
            timestamp: Utc::now(),
            source: source.into(),
            changed_keys,
            checksum: checksum.into(),
        }
    }
}

/// Trait for configuration change event bus implementations.
#[async_trait]
pub trait ConfigBus: Send + Sync {
    async fn publish(&self, event: ConfigChangeEvent) -> ConfigResult<()>;
    async fn subscribe(
        &self,
    ) -> ConfigResult<Pin<Box<dyn Stream<Item = ConfigChangeEvent> + Send>>>;
}

/// In-memory configuration change event bus using tokio broadcast channel.
#[derive(Clone)]
pub struct InMemoryBus {
    sender: broadcast::Sender<ConfigChangeEvent>,
}

impl InMemoryBus {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(1024);
        Self { sender }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    pub fn subscriber_count(&self) -> usize {
        self.sender.receiver_count()
    }
}

impl Default for InMemoryBus {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ConfigBus for InMemoryBus {
    async fn publish(&self, event: ConfigChangeEvent) -> ConfigResult<()> {
        match self.sender.send(event) {
            Ok(_) => Ok(()),
            Err(_) => {
                tracing::warn!("No active subscribers for config change event");
                Ok(())
            }
        }
    }

    async fn subscribe(
        &self,
    ) -> ConfigResult<Pin<Box<dyn Stream<Item = ConfigChangeEvent> + Send>>> {
        let receiver = self.sender.subscribe();
        let stream = BroadcastStream::new(receiver).filter_map(|r| async move { r.ok() });
        Ok(Box::pin(stream))
    }
}

/// Builder for creating ConfigBus instances.
pub struct BusBuilder {
    capacity: usize,
}

impl BusBuilder {
    pub fn new() -> Self {
        Self { capacity: 1024 }
    }

    pub fn capacity(mut self, capacity: usize) -> Self {
        self.capacity = capacity;
        self
    }

    pub fn build(&self) -> InMemoryBus {
        InMemoryBus::with_capacity(self.capacity)
    }
}

impl Default for BusBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::StreamExt;
    use tokio::time::{timeout, Duration};

    #[tokio::test]
    async fn test_config_change_event_creation() {
        let event = ConfigChangeEvent::new(
            "instance-1",
            "file watcher",
            vec!["database.host".to_string()],
            "checksum123",
        );

        assert_eq!(event.instance_id, "instance-1");
        assert_eq!(event.source, "file watcher");
        assert_eq!(event.changed_keys.len(), 1);
        assert_eq!(event.checksum, "checksum123");
    }

    #[tokio::test]
    async fn test_in_memory_bus_publish_subscribe() {
        let bus = InMemoryBus::new();
        let mut events = bus.subscribe().await.unwrap();

        let event =
            ConfigChangeEvent::new("instance-1", "test", vec!["key1".to_string()], "checksum");
        bus.publish(event.clone()).await.unwrap();

        let received = timeout(Duration::from_millis(100), events.next())
            .await
            .unwrap()
            .unwrap();

        assert_eq!(received.instance_id, event.instance_id);
        assert_eq!(received.source, event.source);
        assert_eq!(received.changed_keys, event.changed_keys);
    }

    #[tokio::test]
    async fn test_in_memory_bus_multiple_subscribers() {
        let bus = InMemoryBus::new();
        let mut sub1 = bus.subscribe().await.unwrap();
        let mut sub2 = bus.subscribe().await.unwrap();

        let event = ConfigChangeEvent::new("instance-1", "test", vec![], "abc");
        bus.publish(event.clone()).await.unwrap();

        let received1 = timeout(Duration::from_millis(100), sub1.next())
            .await
            .unwrap()
            .unwrap();
        let received2 = timeout(Duration::from_millis(100), sub2.next())
            .await
            .unwrap()
            .unwrap();

        assert_eq!(received1.checksum, "abc");
        assert_eq!(received2.checksum, "abc");
    }

    #[tokio::test]
    async fn test_in_memory_bus_subscriber_count() {
        let bus = InMemoryBus::new();
        assert_eq!(bus.subscriber_count(), 0);

        let _sub1 = bus.subscribe().await.unwrap();
        assert_eq!(bus.subscriber_count(), 1);

        let _sub2 = bus.subscribe().await.unwrap();
        assert_eq!(bus.subscriber_count(), 2);
    }

    #[tokio::test]
    async fn test_bus_builder() {
        let bus = BusBuilder::new().capacity(2048).build();

        // Subscribe first before publishing
        let mut events = bus.subscribe().await.unwrap();

        let event = ConfigChangeEvent::new("test", "test", vec![], "xyz");
        bus.publish(event).await.unwrap();

        let received = timeout(Duration::from_millis(200), events.next())
            .await
            .unwrap()
            .unwrap();

        assert_eq!(received.checksum, "xyz");
    }
}
