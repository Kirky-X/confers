// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Bus event version arbitration (`config-bus` feature).
//!
//! Bus events previously carried no monotonic sequence: subscribers could not
//! tell duplicates or stale replays from fresh changes, and multi-replica
//! polling produced out-of-order merges. [`VersionArbitratedBus`] wraps any
//! [`ConfigBus`] and
//!
//! - stamps every published event with a **monotonic per-instance version**
//!   (carried in `ConfigChangeEvent.checksum` as a decimal string — the same
//!   field the change stream already parses as a number), and
//! - filters each subscribed stream so every source delivers versions in
//!   strictly increasing order: stale replays and duplicates are **dropped**
//!   (and counted), never delivered twice.

use std::collections::HashMap;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use futures_util::{Stream, StreamExt};

use super::{ConfigBus, ConfigChangeEvent};
use crate::error::ConfigResult;

/// Per-source monotonic sequence generator.
#[derive(Default)]
pub struct MonotonicSequencer {
    next: Mutex<HashMap<String, u64>>,
}

impl MonotonicSequencer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Next version for `source` (starts at 1, strictly increasing).
    pub fn next(&self, source: &str) -> u64 {
        let mut seqs = self.next.lock().expect("sequencer lock");
        let entry = seqs.entry(source.to_string()).or_insert(0);
        *entry += 1;
        *entry
    }
}

/// Per-subscriber ordering state: the highest delivered version per source.
#[derive(Default)]
pub struct OrderedEventFilter {
    last_seen: HashMap<String, u64>,
    dropped: u64,
}

impl OrderedEventFilter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Admit an event version for `source`: `true` when it is newer than
    /// everything delivered so far, `false` for stale replays/duplicates.
    pub fn admit(&mut self, source: &str, version: u64) -> bool {
        if version == 0 {
            // Un-versioned events cannot be ordered; keep them (fail-open for
            // producers not using the sequencer).
            return true;
        }
        let last = self.last_seen.get(source).copied().unwrap_or(0);
        if version > last {
            self.last_seen.insert(source.to_string(), version);
            true
        } else {
            self.dropped += 1;
            false
        }
    }

    /// Number of events this subscriber dropped as stale or duplicated.
    pub fn dropped(&self) -> u64 {
        self.dropped
    }
}

/// A [`ConfigBus`] wrapper adding monotonic versioning and out-of-order
/// arbitration.
pub struct VersionArbitratedBus<B> {
    inner: B,
    sequencer: MonotonicSequencer,
    dropped_total: Arc<AtomicU64>,
}

impl<B> VersionArbitratedBus<B> {
    /// Wrap `inner`.
    pub fn new(inner: B) -> Self {
        Self {
            inner,
            sequencer: MonotonicSequencer::new(),
            dropped_total: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Access the wrapped bus (e.g. to inject raw events in tests).
    pub fn inner(&self) -> &B {
        &self.inner
    }

    /// Events dropped as stale/duplicated across all subscribers so far.
    pub fn dropped_total(&self) -> u64 {
        self.dropped_total.load(Ordering::Acquire)
    }
}

#[async_trait]
impl<B> crate::lifecycle::Lifecycle for VersionArbitratedBus<B>
where
    B: ConfigBus + Send + Sync,
{
    async fn start(&self) -> Result<(), crate::error::ConfigConfigError> {
        self.inner.start().await
    }

    async fn stop(&self) -> ConfigResult<()> {
        self.inner.stop().await
    }
}

#[async_trait]
impl<B> ConfigBus for VersionArbitratedBus<B>
where
    B: ConfigBus + Send + Sync,
{
    async fn publish(&self, mut event: ConfigChangeEvent) -> ConfigResult<()> {
        // Stamp the monotonic version into the checksum field (decimal).
        let version = self.sequencer.next(&event.instance_id);
        event.checksum = version.to_string();
        self.inner.publish(event).await
    }

    async fn subscribe(
        &self,
    ) -> ConfigResult<Pin<Box<dyn Stream<Item = ConfigChangeEvent> + Send>>> {
        let inner_stream = self.inner.subscribe().await?;
        let dropped_total = Arc::clone(&self.dropped_total);
        // Per-subscriber ordering state, carried through the unfold.
        let state = (inner_stream, OrderedEventFilter::new(), dropped_total);
        let stream = futures_util::stream::unfold(
            state,
            |(mut inner, mut filter, dropped_total)| async move {
                loop {
                    match inner.next().await {
                        Some(event) => {
                            let version = event.checksum.parse::<u64>().unwrap_or(0);
                            if filter.admit(&event.instance_id, version) {
                                return Some((event, (inner, filter, dropped_total)));
                            }
                            dropped_total.fetch_add(1, Ordering::AcqRel);
                            // Stale/duplicate: keep waiting for a fresh one.
                        }
                        None => return None,
                    }
                }
            },
        );
        Ok(Box::pin(stream))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bus::InMemoryBus;
    use futures_util::StreamExt;
    use std::time::Duration;
    use tokio::time::timeout;

    fn event(instance: &str, checksum: &str) -> ConfigChangeEvent {
        ConfigChangeEvent::new(instance, "test", vec!["k".to_string()], checksum)
    }

    async fn next_event<S: Stream<Item = ConfigChangeEvent> + Unpin>(
        stream: &mut S,
    ) -> ConfigChangeEvent {
        timeout(Duration::from_millis(300), stream.next())
            .await
            .expect("timed out waiting for event")
            .expect("stream ended")
    }

    #[tokio::test]
    async fn published_events_carry_monotonic_versions() {
        let bus = VersionArbitratedBus::new(InMemoryBus::new());
        let mut rx = bus.subscribe().await.expect("subscribe");

        bus.publish(event("instance-a", "ignored")).await.unwrap();
        bus.publish(event("instance-a", "ignored")).await.unwrap();

        let first = next_event(&mut rx).await;
        let second = next_event(&mut rx).await;
        assert_eq!(first.checksum, "1", "versions start at 1");
        assert_eq!(second.checksum, "2", "versions increase monotonically");
    }

    #[tokio::test]
    async fn out_of_order_and_duplicate_versions_are_dropped() {
        let bus = VersionArbitratedBus::new(InMemoryBus::with_capacity(64));
        let mut rx = bus.subscribe().await.expect("subscribe");

        // Fresh event through the wrapper (version 1 stamped).
        bus.publish(event("replica-1", "x")).await.unwrap();
        let delivered = next_event(&mut rx).await;
        assert_eq!(delivered.checksum, "1");

        // A duplicate replay of version 1, straight on the wrapped bus
        // (simulating a replaying producer): dropped, nothing delivered.
        bus.inner().publish(event("replica-1", "1")).await.unwrap();

        // A genuinely fresh event still flows (wrapper stamps version 2).
        bus.publish(event("replica-1", "x")).await.unwrap();
        let fresh = next_event(&mut rx).await;
        assert_eq!(fresh.checksum, "2");

        // Stale replays of 1 and a duplicate of 2: both dropped.
        bus.inner().publish(event("replica-1", "1")).await.unwrap();
        bus.inner().publish(event("replica-1", "2")).await.unwrap();

        // The stream stays healthy: the next wrapper event (version 3)
        // arrives despite the interleaved drops.
        bus.publish(event("replica-1", "x")).await.unwrap();
        let third = next_event(&mut rx).await;
        assert_eq!(third.checksum, "3");

        assert_eq!(bus.dropped_total(), 3, "stale replays must be counted");
    }

    #[tokio::test]
    async fn unversioned_events_fail_open() {
        let bus = VersionArbitratedBus::new(InMemoryBus::new());
        let mut rx = bus.subscribe().await.expect("subscribe");

        // checksum not parseable as a number: cannot be ordered, delivered.
        bus.inner()
            .publish(event("legacy", "not-a-number"))
            .await
            .unwrap();
        let delivered = next_event(&mut rx).await;
        assert_eq!(delivered.checksum, "not-a-number");
        assert_eq!(bus.dropped_total(), 0);
    }

    #[test]
    fn sequencer_is_strictly_monotonic_per_source() {
        let seq = MonotonicSequencer::new();
        assert_eq!(seq.next("a"), 1);
        assert_eq!(seq.next("a"), 2);
        assert_eq!(seq.next("b"), 1, "independent per source");
        assert_eq!(seq.next("a"), 3);
    }

    #[test]
    fn filter_admits_only_newer_versions_and_counts_drops() {
        let mut filter = OrderedEventFilter::new();
        assert!(filter.admit("src", 5));
        assert!(!filter.admit("src", 5), "duplicate dropped");
        assert!(!filter.admit("src", 4), "stale dropped");
        assert!(filter.admit("src", 6));
        assert!(filter.admit("other", 1), "sources are independent");
        assert!(filter.admit("src", 0), "unversioned fails open");
        assert_eq!(filter.dropped(), 2);
    }
}
