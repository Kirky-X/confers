// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! Bus event version arbitration (`config-bus` feature).
//!
//! Bus events previously carried no monotonic sequence: subscribers could not
//! tell duplicates or stale replays from fresh changes, and multi-replica
//! polling produced out-of-order merges. [`VersionArbitratedBus`] wraps any
//! [`ConfigBus`] and
//!
//! - stamps every published event with a **monotonic per-instance version**
//!   (carried in `ConfigChangeEvent.checksum` as a decimal string — the same
//!   field the change stream already parses as a number) plus a **process
//!   epoch** (`ConfigChangeEvent.publisher_epoch`), and
//! - filters each subscribed stream so every source delivers versions in
//!   strictly increasing order *per `(publisher_id, epoch)` track*: stale
//!   replays and duplicates are **dropped** (and counted), never delivered
//!   twice, while a restarting publisher (fresh epoch, sequence restarting
//!   at 1) opens a new track and is accepted.

use std::collections::HashMap;
use std::hash::{BuildHasher, Hasher};
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use futures_util::{Stream, StreamExt};

use super::{ConfigBus, ConfigChangeEvent};
use crate::error::ConfigResult;

/// Generate a process-level random publisher epoch.
///
/// Without pulling in an RNG dependency, the per-process randomized keys of
/// `RandomState` (seeded by the OS once per process) provide the entropy;
/// the wall-clock nanos are mixed in as a second source so two processes
/// starting in the same nanosecond window still differ with overwhelming
/// probability.
pub(crate) fn generate_epoch() -> u64 {
    let hasher = std::collections::hash_map::RandomState::new().build_hasher();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    hasher.finish() ^ nanos.rotate_left(17)
}

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

/// Per-subscriber ordering state: the highest delivered version per
/// `(publisher_id, epoch)` track.
#[derive(Default)]
pub struct OrderedEventFilter {
    last_seen: HashMap<(String, u64), u64>,
    dropped: u64,
}

impl OrderedEventFilter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Admit an event version on the `(source, epoch)` track: `true` when it
    /// is newer than everything delivered on that track so far, `false` for
    /// stale replays/duplicates.
    ///
    /// - A version of `0` cannot be ordered (un-versioned/legacy producers)
    ///   and is kept (fail-open).
    /// - An epoch never seen before starts a fresh track at 0, so a
    ///   restarting publisher whose sequence restarts at 1 is accepted even
    ///   though the previous track already delivered higher versions.
    pub fn admit(&mut self, source: &str, epoch: u64, version: u64) -> bool {
        if version == 0 {
            // Un-versioned events cannot be ordered; keep them (fail-open for
            // producers not using the sequencer).
            return true;
        }
        let key = (source.to_string(), epoch);
        let last = self.last_seen.get(&key).copied().unwrap_or(0);
        if version > last {
            self.last_seen.insert(key, version);
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
    /// Process-level epoch stamped on every published event.
    publisher_epoch: u64,
    dropped_total: Arc<AtomicU64>,
}

impl<B> VersionArbitratedBus<B> {
    /// Wrap `inner`. A fresh random publisher epoch is drawn for this
    /// process instance.
    pub fn new(inner: B) -> Self {
        Self {
            inner,
            sequencer: MonotonicSequencer::new(),
            publisher_epoch: generate_epoch(),
            dropped_total: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Access the wrapped bus (e.g. to inject raw events in tests).
    pub fn inner(&self) -> &B {
        &self.inner
    }

    /// The publisher epoch stamped onto every event this bus publishes.
    pub fn publisher_epoch(&self) -> u64 {
        self.publisher_epoch
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
        // Stamp the monotonic version into the checksum field (decimal) and
        // this process's epoch alongside it.
        let version = self.sequencer.next(&event.instance_id);
        event.checksum = version.to_string();
        event.publisher_epoch = self.publisher_epoch;
        self.inner.publish(event).await
    }

    async fn subscribe(
        &self,
    ) -> ConfigResult<Pin<Box<dyn Stream<Item = ConfigChangeEvent> + Send>>> {
        let inner_stream = self.inner.subscribe().await?;
        let dropped_total = Arc::clone(&self.dropped_total);
        // Per-subscriber ordering state, carried through the unfold. Events
        // are arbitrated per (publisher_id, epoch) track.
        let state = (inner_stream, OrderedEventFilter::new(), dropped_total);
        let stream = futures_util::stream::unfold(
            state,
            |(mut inner, mut filter, dropped_total)| async move {
                loop {
                    match inner.next().await {
                        Some(event) => {
                            let version = event.checksum.parse::<u64>().unwrap_or(0);
                            if filter.admit(&event.instance_id, event.publisher_epoch, version) {
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

    /// Same as [`event`], but stamped with `epoch` — used to inject events
    /// "from the wire" as if produced by a specific publisher epoch.
    fn event_with_epoch(instance: &str, checksum: &str, epoch: u64) -> ConfigChangeEvent {
        let mut ev = event(instance, checksum);
        ev.publisher_epoch = epoch;
        ev
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
        assert_eq!(
            first.publisher_epoch, bus.publisher_epoch,
            "published events carry the process epoch"
        );
        assert_ne!(
            bus.publisher_epoch, 0,
            "the generated epoch must not collide with the unstamped sentinel"
        );
    }

    #[tokio::test]
    async fn out_of_order_and_duplicate_versions_are_dropped() {
        let bus = VersionArbitratedBus::new(InMemoryBus::new());
        let epoch = bus.publisher_epoch();
        let mut rx = bus.subscribe().await.expect("subscribe");

        // Fresh event through the wrapper (version 1 stamped).
        bus.publish(event("replica-1", "x")).await.unwrap();
        let delivered = next_event(&mut rx).await;
        assert_eq!(delivered.checksum, "1");

        // A duplicate replay of version 1 from the SAME publisher epoch,
        // straight on the wrapped bus: dropped, nothing delivered.
        bus.inner()
            .publish(event_with_epoch("replica-1", "1", epoch))
            .await
            .unwrap();

        // A genuinely fresh event still flows (wrapper stamps version 2).
        bus.publish(event("replica-1", "x")).await.unwrap();
        let fresh = next_event(&mut rx).await;
        assert_eq!(fresh.checksum, "2");

        // Stale replays of 1 and a duplicate of 2 (same epoch): both dropped.
        bus.inner()
            .publish(event_with_epoch("replica-1", "1", epoch))
            .await
            .unwrap();
        bus.inner()
            .publish(event_with_epoch("replica-1", "2", epoch))
            .await
            .unwrap();

        // The stream stays healthy: the next wrapper event (version 3)
        // arrives despite the interleaved drops.
        bus.publish(event("replica-1", "x")).await.unwrap();
        let third = next_event(&mut rx).await;
        assert_eq!(third.checksum, "3");

        assert_eq!(bus.dropped_total(), 3, "stale replays must be counted");
    }

    /// acceptance: a restarting publisher (fresh epoch, sequence
    /// restarting at 1) is accepted even though the old track already
    /// delivered higher versions.
    #[tokio::test]
    async fn publisher_restart_with_new_epoch_is_accepted() {
        let bus = VersionArbitratedBus::new(InMemoryBus::new());
        let old_epoch = bus.publisher_epoch();
        let mut rx = bus.subscribe().await.expect("subscribe");

        // Old publisher instance: versions 1..=5 delivered.
        for v in 1..=5u64 {
            bus.inner()
                .publish(event_with_epoch("replica-1", &v.to_string(), old_epoch))
                .await
                .unwrap();
            let delivered = next_event(&mut rx).await;
            assert_eq!(delivered.checksum, v.to_string());
        }

        // The process restarts: a NEW epoch appears and its sequence starts
        // at 1 again — previously dropped as stale, now a fresh track.
        let new_epoch = old_epoch ^ 0xDEAD_BEEF;
        bus.inner()
            .publish(event_with_epoch("replica-1", "1", new_epoch))
            .await
            .unwrap();
        let restarted = next_event(&mut rx).await;
        assert_eq!(
            restarted.checksum, "1",
            "seq=1 on a new epoch must be delivered"
        );
        assert_eq!(restarted.publisher_epoch, new_epoch);
        assert_eq!(bus.dropped_total(), 0, "nothing may be dropped");

        // The old track is still arbitrated independently: replaying old
        // version 4 is dropped, while the new track advances to 2.
        bus.inner()
            .publish(event_with_epoch("replica-1", "4", old_epoch))
            .await
            .unwrap();
        bus.inner()
            .publish(event_with_epoch("replica-1", "2", new_epoch))
            .await
            .unwrap();
        let after = next_event(&mut rx).await;
        assert_eq!(after.checksum, "2", "new track keeps advancing");
        assert_eq!(bus.dropped_total(), 1, "old-track replay dropped");
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
    fn filter_admits_only_newer_versions_per_epoch_track_and_counts_drops() {
        let mut filter = OrderedEventFilter::new();
        assert!(filter.admit("src", 7, 5));
        assert!(!filter.admit("src", 7, 5), "duplicate dropped");
        assert!(!filter.admit("src", 7, 4), "stale dropped");
        assert!(filter.admit("src", 7, 6));
        assert!(filter.admit("other", 7, 1), "publishers are independent");
        // a fresh epoch of the same publisher is a new track.
        assert!(
            filter.admit("src", 9, 1),
            "new epoch seq=1 must be accepted after high versions on the old epoch"
        );
        assert!(filter.admit("src", 0, 0), "unversioned fails open");
        assert_eq!(filter.dropped(), 2);
    }

    /// epochs are process-random — distinct calls produce distinct
    /// values (equality has ~2^-64 probability per pair) and never 0.
    #[test]
    fn generated_epochs_are_random_and_nonzero() {
        let epochs: Vec<u64> = (0..8).map(|_| generate_epoch()).collect();
        assert!(
            epochs.iter().any(|e| *e != epochs[0]),
            "epochs must be randomized"
        );
        assert!(
            epochs.iter().all(|e| *e != 0),
            "generated epochs must never be the unstamped sentinel"
        );
    }
}
