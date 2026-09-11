// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Unified configuration change stream port.
//!
//! File watching ([`crate::watcher`]), remote sources ([`crate::remote`]) and
//! the config bus ([`crate::bus`]) each had their own change notification
//! mechanism. [`ChangeStream`] is the single port they all publish into:
//! every change is a [`ChangeEvent`] envelope carrying the key, the old and
//! new values and the originating source, with a monotonic `version`
//! assigned by the stream itself.
//!
//! The in-memory implementation reuses [`InMemoryBus`] (the `ConfigBus`
//! broadcast transport) so consumers of both ports observe identical
//! delivery semantics (capacity, lag behaviour).

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use futures_util::StreamExt;

use crate::bus::{ConfigBus, ConfigChangeEvent, InMemoryBus};
use crate::error::{ConfigConfigError, ConfigResult};
use crate::lifecycle::Lifecycle;
use crate::types::ConfigValue;

/// Where a configuration change originated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChangeSource {
    /// File-system watcher reload.
    File,
    /// Remote configuration source (etcd/Consul/HTTP/Nacos/...).
    Remote(String),
    /// Bus broadcast from another instance.
    Bus,
    /// Progressive-reload canary state transition.
    Canary,
    /// Any other producer (custom integrations).
    Other(String),
}

impl ChangeSource {
    /// Stable machine-readable name of the source (used in events/logs).
    pub fn as_str(&self) -> &str {
        match self {
            Self::File => "file",
            Self::Remote(_) => "remote",
            Self::Bus => "bus",
            Self::Canary => "canary",
            Self::Other(name) => name.as_str(),
        }
    }
}

impl std::fmt::Display for ChangeSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Remote(name) => write!(f, "remote:{name}"),
            Self::Other(name) => write!(f, "{name}"),
            other => write!(f, "{}", other.as_str()),
        }
    }
}

/// A unified configuration change envelope.
///
/// `version` is assigned monotonically per publishing stream; subscribers
/// use it for ordering and [`ChangeStream::ack`].
#[derive(Debug, Clone, PartialEq)]
pub struct ChangeEvent {
    /// Monotonic sequence assigned by the publishing stream.
    pub version: u64,
    /// Configuration key (dot-notation) that changed.
    pub key: String,
    /// Previous value, when known.
    pub old_value: Option<ConfigValue>,
    /// New value, when known (`None` for deletions).
    pub new_value: Option<ConfigValue>,
    /// Origin of the change.
    pub source: ChangeSource,
}

impl ChangeEvent {
    /// Create an envelope without a version (assigned on publish).
    pub fn new(
        key: impl Into<String>,
        old_value: Option<ConfigValue>,
        new_value: Option<ConfigValue>,
        source: ChangeSource,
    ) -> Self {
        Self {
            version: 0,
            key: key.into(),
            old_value,
            new_value,
            source,
        }
    }
}

/// Unified configuration change stream port.
///
/// Producers (file watcher bridges, remote sources, progressive reloaders)
/// call [`ChangeStream::publish`]; consumers [`subscribe`](ChangeStream::subscribe)
/// and [`ack`](ChangeStream::ack) each delivered version.
#[async_trait]
pub trait ChangeStream: Send + Sync {
    /// Publish a change envelope. The implementor assigns the monotonic
    /// `version` (any version on the input event is overwritten).
    async fn publish(&self, event: ChangeEvent) -> ConfigResult<()>;

    /// Subscribe to the change stream.
    async fn subscribe(
        &self,
    ) -> ConfigResult<Pin<Box<dyn futures_util::Stream<Item = ChangeEvent> + Send>>>;

    /// Acknowledge that `version` has been applied. Unacked events are
    /// eligible for redelivery by reliable implementations; the in-memory
    /// implementation uses ack only to release retained payloads.
    async fn ack(&self, version: u64) -> ConfigResult<()>;

    /// Number of events published but not yet acked (best-effort).
    fn pending_count(&self) -> usize {
        0
    }
}

use std::pin::Pin;

/// In-memory [`ChangeStream`] built on top of the `ConfigBus` transport.
///
/// Payloads (old/new values) are retained keyed by version while the
/// corresponding `ConfigChangeEvent` summary flows through the reused
/// [`InMemoryBus`]; [`ChangeStream::ack`] releases the payload.
#[derive(Clone)]
pub struct InMemoryChangeStream {
    bus: InMemoryBus,
    payloads: Arc<Mutex<dyn PendingStore>>,
    next_version: Arc<AtomicU64>,
}

/// Bounded version -> payload store with FIFO eviction.
trait PendingStore: Send {
    fn insert(&mut self, version: u64, event: ChangeEvent);
    fn get(&self, version: u64) -> Option<&ChangeEvent>;
    fn remove(&mut self, version: u64) -> Option<ChangeEvent>;
    fn len(&self) -> usize;
}

#[derive(Default)]
struct BoundedPendingStore {
    map: std::collections::HashMap<u64, ChangeEvent>,
    order: std::collections::VecDeque<u64>,
    capacity: usize,
}

impl PendingStore for BoundedPendingStore {
    fn insert(&mut self, version: u64, event: ChangeEvent) {
        if self.map.len() >= self.capacity
            && let Some(evict) = self.order.pop_front()
        {
            self.map.remove(&evict);
        }
        self.order.push_back(version);
        self.map.insert(version, event);
    }

    fn remove(&mut self, version: u64) -> Option<ChangeEvent> {
        let removed = self.map.remove(&version);
        if removed.is_some()
            && let Some(pos) = self.order.iter().position(|v| *v == version)
        {
            self.order.remove(pos);
        }
        removed
    }

    fn get(&self, version: u64) -> Option<&ChangeEvent> {
        self.map.get(&version)
    }

    fn len(&self) -> usize {
        self.map.len()
    }
}

impl InMemoryChangeStream {
    /// Create a stream with the default retention capacity (1024).
    pub fn new() -> Self {
        Self::with_capacity(1024)
    }

    /// Create a stream with an explicit pending-payload capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            bus: InMemoryBus::with_capacity(capacity.max(16)),
            payloads: Arc::new(Mutex::new(BoundedPendingStore {
                map: std::collections::HashMap::new(),
                order: std::collections::VecDeque::new(),
                capacity: capacity.max(1),
            })),
            next_version: Arc::new(AtomicU64::new(1)),
        }
    }

    /// Convenience producer for file-watcher originated changes.
    pub async fn publish_watch(
        &self,
        key: impl Into<String>,
        old_value: Option<ConfigValue>,
        new_value: Option<ConfigValue>,
    ) -> ConfigResult<()> {
        self.publish(ChangeEvent::new(
            key,
            old_value,
            new_value,
            ChangeSource::File,
        ))
        .await
    }

    /// Convenience producer for remote-source originated changes.
    pub async fn publish_remote(
        &self,
        source: impl Into<String>,
        key: impl Into<String>,
        old_value: Option<ConfigValue>,
        new_value: Option<ConfigValue>,
    ) -> ConfigResult<()> {
        self.publish(ChangeEvent::new(
            key,
            old_value,
            new_value,
            ChangeSource::Remote(source.into()),
        ))
        .await
    }
}

impl Default for InMemoryChangeStream {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ChangeStream for InMemoryChangeStream {
    async fn publish(&self, event: ChangeEvent) -> ConfigResult<()> {
        let version = self.next_version.fetch_add(1, Ordering::Relaxed);
        let mut stored = event;
        stored.version = version;

        // Retain the full envelope, then broadcast a summary over the
        // reused ConfigBus transport.
        if let Ok(mut store) = self.payloads.try_lock() {
            store.insert(version, stored.clone());
        }
        let summary = ConfigChangeEvent::new(
            "in-memory",
            stored.source.as_str(),
            vec![stored.key.clone()],
            version.to_string(),
        );
        self.bus.publish(summary).await
    }

    async fn subscribe(
        &self,
    ) -> ConfigResult<Pin<Box<dyn futures_util::Stream<Item = ChangeEvent> + Send>>> {
        let payloads = Arc::clone(&self.payloads);
        let bus_stream = self.bus.subscribe().await?;
        let mapped = bus_stream.filter_map(move |summary: ConfigChangeEvent| {
            let payloads = Arc::clone(&payloads);
            async move {
                let version: u64 = summary.checksum.parse().unwrap_or(0);
                // Payloads stay retained until acked (see `ack`), so a clone
                // is delivered here rather than a destructive remove.
                payloads
                    .try_lock()
                    .ok()
                    .and_then(|store| store.get(version).cloned())
            }
        });
        Ok(Box::pin(mapped))
    }

    async fn ack(&self, version: u64) -> ConfigResult<()> {
        if let Ok(mut store) = self.payloads.try_lock() {
            store.remove(version);
        }
        Ok(())
    }

    fn pending_count(&self) -> usize {
        self.payloads.try_lock().map(|store| store.len()).unwrap_or(0)
    }
}

#[async_trait]
impl Lifecycle for InMemoryChangeStream {
    async fn start(&self) -> Result<(), ConfigConfigError> {
        Ok(())
    }

    async fn stop(&self) -> ConfigResult<()> {
        Ok(())
    }
}

/// Bridge a [`crate::watcher::FsWatcher`] path-event feed into a
/// [`ChangeStream`], unifying file changes with remote changes on one port.
///
/// `to_event` maps a debounced file path to a change envelope (returning
/// `None` skips the event). The bridge ends when the watcher closes its
/// channel.
pub async fn bridge_fs_watcher<F>(
    stream: &InMemoryChangeStream,
    watcher: &mut crate::watcher::FsWatcher,
    mut to_event: F,
) where
    F: FnMut(std::path::PathBuf) -> Option<ChangeEvent>,
{
    while let Some(path) = watcher.recv().await {
        if let Some(event) = to_event(path) {
            let _ = stream.publish(event).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::StreamExt;
    use std::time::Duration;
    use tokio::time::timeout;

    #[tokio::test]
    async fn subscriber_receives_watch_event() {
        let stream = InMemoryChangeStream::new();
        let mut rx = stream.subscribe().await.unwrap();

        stream
            .publish_watch(
                "server.host",
                Some(ConfigValue::string("old")),
                Some(ConfigValue::string("new")),
            )
            .await
            .unwrap();

        let event = timeout(Duration::from_millis(200), rx.next())
            .await
            .expect("timed out waiting for watch event")
            .expect("stream ended");

        assert_eq!(event.key, "server.host");
        assert_eq!(event.source, ChangeSource::File);
        assert_eq!(event.old_value.as_ref().and_then(|v| v.as_str()), Some("old"));
        assert_eq!(event.new_value.as_ref().and_then(|v| v.as_str()), Some("new"));
        assert!(event.version > 0);
    }

    #[tokio::test]
    async fn remote_changes_flow_through_the_same_port() {
        let stream = InMemoryChangeStream::new();
        let mut rx = stream.subscribe().await.unwrap();

        stream
            .publish_remote(
                "etcd",
                "db.url",
                None,
                Some(ConfigValue::string("postgres://new")),
            )
            .await
            .unwrap();

        let event = timeout(Duration::from_millis(200), rx.next())
            .await
            .expect("timed out")
            .expect("stream ended");
        assert_eq!(event.source, ChangeSource::Remote("etcd".into()));
        assert_eq!(event.key, "db.url");
    }

    #[tokio::test]
    async fn versions_are_monotonic_and_pending_shrinks_on_ack() {
        let stream = InMemoryChangeStream::new();
        let mut rx = stream.subscribe().await.unwrap();

        for i in 0..3 {
            stream
                .publish(ChangeEvent::new(format!("k{i}"), None, None, ChangeSource::Bus))
                .await
                .unwrap();
        }

        let mut versions = Vec::new();
        for _ in 0..3 {
            let ev = timeout(Duration::from_millis(200), rx.next())
                .await
                .expect("timed out")
                .expect("stream ended");
            versions.push(ev.version);
            stream.ack(ev.version).await.unwrap();
        }
        let mut sorted = versions.clone();
        sorted.sort_unstable();
        assert_eq!(versions, sorted, "versions must be delivered monotonically");
        assert_eq!(stream.pending_count(), 0, "acked events release payloads");
    }

    #[tokio::test]
    async fn unacked_events_stay_pending_until_ack() {
        let stream = InMemoryChangeStream::new();
        let mut rx = stream.subscribe().await.unwrap();
        stream
            .publish(ChangeEvent::new("k", None, None, ChangeSource::File))
            .await
            .unwrap();
        let ev = timeout(Duration::from_millis(200), rx.next())
            .await
            .expect("timed out")
            .expect("stream ended");
        assert_eq!(stream.pending_count(), 1);
        stream.ack(ev.version).await.unwrap();
        assert_eq!(stream.pending_count(), 0);

        // The event is delivered exactly once — after ack nothing is
        // redelivered within a grace window.
        let redelivered = timeout(Duration::from_millis(80), rx.next()).await;
        assert!(redelivered.is_err(), "acked event must not be redelivered");
    }
}
