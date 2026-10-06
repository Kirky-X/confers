// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

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

    /// Synthetic re-synchronization event for a subscriber that fell behind:
    /// versions below `from` were evicted from the retention store before
    /// this subscriber could observe them. `missed` is the version whose
    /// lookup triggered the resync (R-watch-006).
    pub fn resync(from: u64, missed: u64) -> Self {
        Self {
            version: missed,
            key: RESYNC_KEY.to_string(),
            old_value: None,
            new_value: Some(ConfigValue::uint(from)),
            source: ChangeSource::Other("resync".to_string()),
        }
    }

    /// True for the synthetic re-synchronization events emitted when a
    /// subscriber fell behind the retention window (see [`Self::resync`]).
    pub fn is_resync(&self) -> bool {
        self.key == RESYNC_KEY
    }
}

/// `key` carried by the synthetic re-synchronization events a subscriber
/// receives after its missed versions were already evicted.
pub const RESYNC_KEY: &str = "__resync__";

/// Error returned when resolving a retained change event fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChangeStreamError {
    /// The requested version was already evicted: the consumer fell behind
    /// the retention window. `from` is the oldest version still retained,
    /// so the consumer knows where to re-sync from.
    Lagged {
        /// Oldest retained version; every version below it is gone.
        from: u64,
    },
    /// The version was never published on this stream.
    NotFound {
        /// The version that could not be resolved.
        version: u64,
    },
}

impl std::fmt::Display for ChangeStreamError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Lagged { from } => {
                write!(f, "subscriber lagged: versions below {from} were evicted")
            }
            Self::NotFound { version } => {
                write!(f, "version {version} was not published on this stream")
            }
        }
    }
}

impl std::error::Error for ChangeStreamError {}

// Dual-track i18n contract (see `src/i18n/error_ext.rs`): Display stays the
// English canonical form; localization flows through `LocalizedMsg` and the
// `error-stream-*` keys in `locales/{en,zh}/errors.ftl`.
impl crate::i18n::LocalizedMsg for ChangeStreamError {
    fn message_key(&self) -> &'static str {
        match self {
            Self::Lagged { .. } => "error-stream-lagged",
            Self::NotFound { .. } => "error-stream-version-not-found",
        }
    }

    fn message_args(&self) -> Vec<(&str, String)> {
        match self {
            Self::Lagged { from } => vec![("from", from.to_string())],
            Self::NotFound { version } => vec![("version", version.to_string())],
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
    ///
    /// Semantics note (R3-L2): ack is a GLOBAL release, not per-subscriber —
    /// once one subscriber acks `version`, the in-memory implementation may
    /// evict that payload for everyone, and a slower subscriber that has not
    /// seen it yet receives the explicit `resync` event instead of silent
    /// data loss.
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
    /// Watermark: versions below this value were FIFO-evicted.
    fn min_retained(&self) -> u64;
}

#[derive(Default)]
struct BoundedPendingStore {
    map: std::collections::HashMap<u64, ChangeEvent>,
    order: std::collections::VecDeque<u64>,
    capacity: usize,
    /// Oldest version still retained; everything below was evicted. Used to
    /// distinguish "lagged" from "never published" (R-watch-006).
    min_retained: u64,
}

impl PendingStore for BoundedPendingStore {
    fn insert(&mut self, version: u64, event: ChangeEvent) {
        if self.map.len() >= self.capacity
            && let Some(evict) = self.order.pop_front()
        {
            self.map.remove(&evict);
            // Advance the watermark past the evicted version: lookups for it
            // must be reported as Lagged instead of silently missing.
            self.min_retained = self.min_retained.max(evict + 1);
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

    fn min_retained(&self) -> u64 {
        self.min_retained
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
                min_retained: 0,
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

    /// Resolve the retained payload for `version` (R-watch-006).
    ///
    /// Returns the stored envelope while it is retained (until ack or FIFO
    /// eviction). A version inside the already-evicted range yields
    /// [`ChangeStreamError::Lagged`] with the oldest still-retained version
    /// in `from` — the explicit signal that this consumer fell behind and
    /// missed events; consumers should re-sync instead of silently
    /// continuing. A version that was never published yields
    /// [`ChangeStreamError::NotFound`].
    pub fn get(&self, version: u64) -> Result<ChangeEvent, ChangeStreamError> {
        let store = self.payloads.lock().unwrap_or_else(|p| p.into_inner());
        if let Some(event) = store.get(version) {
            return Ok(event.clone());
        }
        if version > 0 && version < store.min_retained() {
            return Err(ChangeStreamError::Lagged {
                from: store.min_retained(),
            });
        }
        Err(ChangeStreamError::NotFound { version })
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
        // reused ConfigBus transport. The store must keep up with the
        // broadcast: a poisoned lock is recovered (matching this crate's
        // other lock call sites) so subscribers can still resolve the
        // version they are about to be notified of.
        //
        // Scoped block, not `drop()`: the guard must not be considered live
        // across the await below when the future's Send-ness is computed.
        {
            let mut store = self.payloads.lock().unwrap_or_else(|p| p.into_inner());
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
                // is delivered here rather than a destructive remove. The
                // lock is recovered if poisoned.
                let store = payloads.lock().unwrap_or_else(|p| p.into_inner());
                if let Some(event) = store.get(version) {
                    return Some(event.clone());
                }
                // The payload is gone: when the version fell inside the
                // evicted range the subscriber lagged, and the event must
                // surface as an explicit resync signal instead of being
                // swallowed by this filter_map (R-watch-006).
                // Version 0 means the checksum could not be parsed at all —
                // nothing to re-sync from, skip it.
                if version > 0 && version < store.min_retained() {
                    return Some(ChangeEvent::resync(store.min_retained(), version));
                }
                None
            }
        });
        Ok(Box::pin(mapped))
    }

    async fn ack(&self, version: u64) -> ConfigResult<()> {
        let mut store = self.payloads.lock().unwrap_or_else(|p| p.into_inner());
        store.remove(version);
        Ok(())
    }

    fn pending_count(&self) -> usize {
        self.payloads
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .len()
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
        assert_eq!(
            event.old_value.as_ref().and_then(|v| v.as_str()),
            Some("old")
        );
        assert_eq!(
            event.new_value.as_ref().and_then(|v| v.as_str()),
            Some("new")
        );
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
                .publish(ChangeEvent::new(
                    format!("k{i}"),
                    None,
                    None,
                    ChangeSource::Bus,
                ))
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

    /// R-watch-006: versions evicted by the FIFO retention are
    /// reported explicitly as `Lagged { from }` — never as a silent miss.
    #[tokio::test]
    async fn get_reports_lagged_for_evicted_versions() {
        let stream = InMemoryChangeStream::with_capacity(2);
        for i in 0..4 {
            stream
                .publish(ChangeEvent::new(
                    format!("k{i}"),
                    None,
                    None,
                    ChangeSource::Bus,
                ))
                .await
                .unwrap();
        }

        // Versions 1 and 2 were FIFO-evicted; retention starts at 3.
        assert_eq!(
            stream.get(1),
            Err(ChangeStreamError::Lagged { from: 3 }),
            "evicted versions must be reported as Lagged with the retention start"
        );
        assert_eq!(stream.get(2), Err(ChangeStreamError::Lagged { from: 3 }));
        // Retained versions still resolve.
        assert_eq!(stream.get(3).unwrap().key, "k2");
        assert_eq!(stream.get(4).unwrap().key, "k3");
        // A version that was never published is NotFound, not Lagged.
        assert_eq!(
            stream.get(999),
            Err(ChangeStreamError::NotFound { version: 999 })
        );
    }

    /// R-watch-006: a subscriber whose events were evicted before it
    /// could observe them receives explicit resync events — the filter_map
    /// must not swallow the evicted versions silently.
    #[tokio::test]
    async fn evicted_events_surface_as_explicit_resync() {
        let stream = InMemoryChangeStream::with_capacity(2);
        let mut rx = stream.subscribe().await.unwrap();

        for i in 0..4 {
            stream
                .publish(ChangeEvent::new(
                    format!("k{i}"),
                    None,
                    None,
                    ChangeSource::File,
                ))
                .await
                .unwrap();
        }

        // Drain the four summaries: the two evicted versions must arrive as
        // resync events, the two retained ones as normal events.
        let mut seen = Vec::new();
        for _ in 0..4 {
            let event = timeout(Duration::from_millis(200), rx.next())
                .await
                .expect("timed out waiting for stream event")
                .expect("stream ended");
            seen.push(event);
        }

        let resyncs: Vec<&ChangeEvent> = seen.iter().filter(|e| e.is_resync()).collect();
        assert_eq!(
            resyncs.len(),
            2,
            "the two evicted versions must surface as resync events, got {seen:?}"
        );
        for resync in &resyncs {
            assert_eq!(
                resync.new_value.as_ref().and_then(|v| v.as_u64()),
                Some(3),
                "resync carries the first retained version"
            );
            assert_eq!(resync.source, ChangeSource::Other("resync".into()));
        }

        let normal: Vec<&ChangeEvent> = seen.iter().filter(|e| !e.is_resync()).collect();
        assert_eq!(normal.len(), 2, "retained events are delivered normally");
        assert_eq!(normal[0].key, "k2");
        assert_eq!(normal[1].key, "k3");
        assert_eq!(normal[0].version, 3);
        assert_eq!(normal[1].version, 4);
    }

    #[test]
    fn change_source_display_and_as_str_cover_all_variants() {
        assert_eq!(ChangeSource::File.as_str(), "file");
        assert_eq!(ChangeSource::Bus.as_str(), "bus");
        assert_eq!(ChangeSource::Canary.as_str(), "canary");
        assert_eq!(ChangeSource::Remote("etcd".into()).as_str(), "remote");
        assert_eq!(ChangeSource::Other("custom".into()).as_str(), "custom");

        assert_eq!(ChangeSource::File.to_string(), "file");
        assert_eq!(ChangeSource::Bus.to_string(), "bus");
        assert_eq!(ChangeSource::Canary.to_string(), "canary");
        assert_eq!(
            ChangeSource::Remote("nacos".into()).to_string(),
            "remote:nacos"
        );
        assert_eq!(ChangeSource::Other("webhook".into()).to_string(), "webhook");
    }

    #[test]
    fn change_stream_error_display_mentions_recovery_hint() {
        assert_eq!(
            ChangeStreamError::Lagged { from: 7 }.to_string(),
            "subscriber lagged: versions below 7 were evicted"
        );
        assert_eq!(
            ChangeStreamError::NotFound { version: 42 }.to_string(),
            "version 42 was not published on this stream"
        );
    }

    /// Dual-track i18n guard (same style as `src/i18n/error_ext.rs`): the en
    /// FTL template mirrors the canonical Display string verbatim and the
    /// localized output always comes from the catalog.
    #[test]
    fn change_stream_error_localization_is_catalog_backed() {
        use crate::i18n::{I18nExt, LocalizedMsg};

        let lagged = ChangeStreamError::Lagged { from: 7 };
        assert_eq!(lagged.message_key(), "error-stream-lagged");
        assert_eq!(
            lagged.message_args(),
            vec![("from", "7".to_string())],
            "en/zh templates interpolate the oldest retained version"
        );
        assert_eq!(lagged.to_string(), lagged.message_en());
        assert!(
            matches!(
                lagged.to_localized_string().as_str(),
                "subscriber lagged: versions below 7 were evicted"
                    | "订阅者已落后: 7 之前的版本已被逐出"
            ),
            "localized output must come from the catalog"
        );

        let not_found = ChangeStreamError::NotFound { version: 42 };
        assert_eq!(not_found.message_key(), "error-stream-version-not-found");
        assert_eq!(
            not_found.message_args(),
            vec![("version", "42".to_string())]
        );
        assert_eq!(not_found.to_string(), not_found.message_en());
        assert!(
            matches!(
                not_found.to_localized_string().as_str(),
                "version 42 was not published on this stream" | "版本 42 未在此流上发布过"
            ),
            "localized output must come from the catalog"
        );
    }

    /// The `ChangeStream::pending_count` default (0) must surface for
    /// implementors that do not override it.
    #[tokio::test]
    async fn pending_count_defaults_to_zero_for_custom_implementors() {
        struct NoPendingStream(InMemoryChangeStream);

        #[async_trait]
        impl ChangeStream for NoPendingStream {
            async fn publish(&self, event: ChangeEvent) -> ConfigResult<()> {
                self.0.publish(event).await
            }
            async fn subscribe(
                &self,
            ) -> ConfigResult<Pin<Box<dyn futures_util::Stream<Item = ChangeEvent> + Send>>>
            {
                self.0.subscribe().await
            }
            async fn ack(&self, version: u64) -> ConfigResult<()> {
                self.0.ack(version).await
            }
        }

        async fn read_pending(stream: &dyn ChangeStream) -> usize {
            stream.pending_count()
        }

        let stream = NoPendingStream(InMemoryChangeStream::new());
        assert_eq!(read_pending(&stream).await, 0);
    }

    #[tokio::test]
    async fn lifecycle_start_stop_are_noop_successes() {
        use crate::lifecycle::Lifecycle;

        let stream = InMemoryChangeStream::new();
        stream.start().await.expect("start is a no-op");
        stream.stop().await.expect("stop is a no-op");
    }

    /// The fs-watcher bridge publishes mapped events into the stream and
    /// ends when the watcher closes its channel; `None` mappings are skipped.
    #[tokio::test]
    #[serial_test::serial]
    async fn bridge_publishes_mapped_events_and_skips_none() {
        let stream = InMemoryChangeStream::new();
        let mut rx = stream.subscribe().await.unwrap();

        let dir = tempfile::tempdir().expect("tempdir");
        let mut watcher = crate::watcher::FsWatcher::with_recv_timeout(dir.path(), 30, 50)
            .await
            .expect("watchable temp dir");

        let bridge_fut = bridge_fs_watcher(&stream, &mut watcher, |path| {
            let name = path.file_name()?.to_string_lossy().into_owned();
            // Only `*.publish` files become events; others are skipped.
            if name.ends_with(".publish") {
                Some(ChangeEvent::new(
                    name,
                    None,
                    Some(ConfigValue::string("changed")),
                    ChangeSource::File,
                ))
            } else {
                None
            }
        });
        tokio::pin!(bridge_fut);

        // Let the debouncer establish the watch before writing (same pattern
        // as the fs_watcher tests).
        tokio::time::sleep(Duration::from_millis(400)).await;
        std::fs::write(dir.path().join("ignored.txt"), b"skip me").expect("write");
        std::fs::write(dir.path().join("app.publish"), b"publish me").expect("write");

        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        let mut seen_app = false;
        while !seen_app && std::time::Instant::now() < deadline {
            tokio::select! {
                _ = &mut bridge_fut => break, // bridge ended early
                maybe = rx.next() => match maybe {
                    Some(event) if event.key == "app.publish" => seen_app = true,
                    Some(_) => continue, // unrelated summary event
                    None => break,
                },
            }
        }
        assert!(seen_app, "the mapped .publish event must reach the stream");
    }

    #[tokio::test]
    async fn ack_and_pending_count_track_payload_retention() {
        // default 必须与 new 等价。
        let stream = InMemoryChangeStream::default();
        let mut rx = stream.subscribe().await.unwrap();

        stream
            .publish_watch("k", None, Some(ConfigValue::string("v1")))
            .await
            .unwrap();
        stream
            .publish_watch(
                "k",
                Some(ConfigValue::string("v1")),
                Some(ConfigValue::string("v2")),
            )
            .await
            .unwrap();
        for _ in 0..2 {
            timeout(Duration::from_millis(200), rx.next())
                .await
                .expect("event in time")
                .expect("stream alive");
        }

        // Payload 在 ack 前一直保留:两条事件 → pending 2,ack 一条 → 1。
        assert_eq!(stream.pending_count(), 2);
        stream.ack(1).await.unwrap();
        assert_eq!(stream.pending_count(), 1, "acked payload must be dropped");
    }

    #[tokio::test]
    async fn lagging_subscriber_gets_resync_signal() {
        // 订阅者落后于保留窗口时必须收到显式 resync 信号,
        // 而不是被静默吞掉(R-watch-006)。容量 2 的存储:发布 3 条后
        // version 1 被淘汰,min_retained 前移到 2。
        let stream = InMemoryChangeStream::with_capacity(2);
        let mut rx = stream.subscribe().await.unwrap();
        for v in ["v1", "v2", "v3"] {
            stream
                .publish_watch("k", None, Some(ConfigValue::string(v)))
                .await
                .unwrap();
        }

        // 滞留的 version 1 已被容量淘汰,重放必须转成 resync 信号。
        let first = timeout(Duration::from_millis(200), rx.next())
            .await
            .expect("first lagged event")
            .expect("stream alive");
        assert!(
            first.is_resync(),
            "lagged version inside the evicted window must resync: {first:?}"
        );
    }
}
