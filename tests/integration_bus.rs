//! Integration tests for ConfigBus.

#![cfg(feature = "config-bus")]

use confers::bus::{BusBuilder, ConfigBus, ConfigChangeEvent, InMemoryBus};
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
async fn test_in_memory_bus_new() {
    let bus = InMemoryBus::new();
    assert_eq!(bus.subscriber_count(), 0);
}

#[tokio::test]
async fn test_in_memory_bus_with_capacity() {
    let bus = InMemoryBus::with_capacity(2048);
    assert_eq!(bus.subscriber_count(), 0);
}

#[tokio::test]
async fn test_in_memory_bus_publish_subscribe() {
    let bus = InMemoryBus::new();
    let mut events = bus.subscribe().await.unwrap();
    assert_eq!(bus.subscriber_count(), 1);

    let event = ConfigChangeEvent::new(
        "instance-1",
        "test",
        vec!["key1".to_string()],
        "checksum",
    );
    bus.publish(event.clone()).await.unwrap();

    let received = timeout(Duration::from_millis(100), events.next())
        .await
        .unwrap()
        .unwrap();

    assert_eq!(received.instance_id, event.instance_id);
    assert_eq!(received.source, event.source);
}

#[tokio::test]
async fn test_in_memory_bus_multiple_subscribers() {
    let bus = InMemoryBus::new();
    let mut sub1 = bus.subscribe().await.unwrap();
    let mut sub2 = bus.subscribe().await.unwrap();
    assert_eq!(bus.subscriber_count(), 2);

    let event = ConfigChangeEvent::new("instance-1", "test", vec![], "abc");
    bus.publish(event).await.unwrap();

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
async fn test_bus_builder() {
    let bus = BusBuilder::new().capacity(2048).build();

    // Subscribe before publishing to receive the message
    let mut events = bus.subscribe().await.unwrap();

    let event = ConfigChangeEvent::new("test", "test", vec![], "xyz");
    bus.publish(event).await.unwrap();

    let received = timeout(Duration::from_millis(100), events.next())
        .await
        .unwrap()
        .unwrap();

    assert_eq!(received.checksum, "xyz");
}

#[test]
fn test_event_serialization() {
    let event = ConfigChangeEvent::new(
        "instance-1",
        "test",
        vec!["key1".to_string(), "key2".to_string()],
        "checksum",
    );

    let json = serde_json::to_string(&event).unwrap();
    let decoded: ConfigChangeEvent = serde_json::from_str(&json).unwrap();

    assert_eq!(decoded.instance_id, event.instance_id);
    assert_eq!(decoded.source, event.source);
    assert_eq!(decoded.changed_keys, event.changed_keys);
}
