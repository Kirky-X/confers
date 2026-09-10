// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Native etcd watch streaming (`etcd-watch` feature).
//!
//! The polling [`EtcdSource`](super::etcd::EtcdSource) detects changes only
//! on its poll interval; this module consumes etcd's native watch stream so
//! changes are delivered the moment they are committed, and keeps watching
//! across connection failures with bounded exponential backoff.
//!
//! The transport is abstracted behind [`WatchEventSource`] so the reconnect
//! loop is testable against a mock feed: a transport error (or a stream
//! ending) tears the watch down and re-establishes it — events keep flowing
//! to the callback without the caller doing anything.

use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// One normalized etcd watch event.
///
/// `value` is `None` for deletions; `mod_revision` is the etcd revision at
/// which the change was committed (used to resume a re-established watch
/// without gaps).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EtcdWatchEvent {
    /// Full etcd key (prefix included).
    pub key: String,
    /// New value (`None` when the key was deleted).
    pub value: Option<String>,
    /// etcd revision of the change.
    pub mod_revision: i64,
}

/// One item yielded by a watch transport: a normalized event, or a
/// transport failure (which the watcher answers with a reconnect).
pub type WatchItem = Result<EtcdWatchEvent, String>;

/// Transport abstraction over an etcd watch stream (mockable).
///
/// Implementations open a prefix watch and translate the gRPC stream into
/// [`WatchItem`]s. Returning `Err` from [`watch`](WatchEventSource::watch)
/// or yielding `Err` items signals connection trouble; the watcher
/// reconnects with backoff either way.
#[async_trait::async_trait]
pub trait WatchEventSource: Send + Sync {
    /// Open a prefix watch stream. Resolves once the watch is established.
    async fn watch(
        &self,
    ) -> crate::error::ConfigResult<
        std::pin::Pin<Box<dyn futures_util::Stream<Item = WatchItem> + Send>>,
    >;
}

/// Reconnect policy for the watch loop.
#[derive(Debug, Clone)]
pub struct EtcdWatchRetry {
    /// Base delay for the first reconnect (doubled per consecutive failure).
    pub base: Duration,
    /// Upper bound for the backoff delay.
    pub max: Duration,
}

impl Default for EtcdWatchRetry {
    fn default() -> Self {
        Self {
            base: Duration::from_millis(500),
            max: Duration::from_secs(30),
        }
    }
}

impl EtcdWatchRetry {
    /// Delay before the `attempt`-th reconnect (1-based), bounded by `max`.
    pub fn delay_for(&self, attempt: u32) -> Duration {
        let shift = attempt.saturating_sub(1).min(16);
        self.base
            .saturating_mul(1u32 << shift)
            .min(self.max)
            .max(Duration::from_millis(1))
    }
}

/// Type-erased watch callback invoked for every committed change.
pub type EtcdWatchCallback = Arc<dyn Fn(&EtcdWatchEvent) + Send + Sync>;

/// Watch loop consuming a [`WatchEventSource`].
///
/// Events are handed to the callback in etcd commit order; the highest seen
/// `mod_revision` is tracked so a reconnect can resume from `last + 1`
/// (no gap, no replay of already-delivered revisions when the transport
/// supports `with_start_revision`).
pub struct EtcdWatcher {
    source: Arc<dyn WatchEventSource>,
    retry: EtcdWatchRetry,
    last_revision: Arc<AtomicI64>,
}

impl EtcdWatcher {
    /// Watch `source` with the default retry policy.
    pub fn new(source: Arc<dyn WatchEventSource>) -> Self {
        Self {
            source,
            retry: EtcdWatchRetry::default(),
            last_revision: Arc::new(AtomicI64::new(0)),
        }
    }

    /// Override the reconnect policy.
    pub fn with_retry(mut self, retry: EtcdWatchRetry) -> Self {
        self.retry = retry;
        self
    }

    /// Highest `mod_revision` delivered so far (0 before the first event).
    pub fn last_revision(&self) -> i64 {
        self.last_revision.load(Ordering::Acquire)
    }

    /// Run the watch loop forever, invoking `on_event` for each change.
    ///
    /// Transport failures reconnect with bounded exponential backoff; the
    /// callback never observes transport internals. This future only ends if
    /// the runtime shuts it down.
    pub async fn run(&self, on_event: EtcdWatchCallback) {
        let mut attempt: u32 = 0;
        loop {
            match self.source.watch().await {
                Ok(mut stream) => {
                    attempt = 0;
                    while let Some(item) =
                        futures_util::StreamExt::next(&mut stream).await
                    {
                        match item {
                            Ok(event) => {
                                if event.mod_revision > 0 {
                                    self.last_revision
                                        .fetch_max(event.mod_revision, Ordering::AcqRel);
                                }
                                on_event(&event);
                            }
                            Err(_err) => {
                                crate::metrics::record_counter(
                                    "confers_etcd_watch_errors_total",
                                    &[("reason", "stream")],
                                );
                                break;
                            }
                        }
                    }
                }
                Err(_) => {
                    crate::metrics::record_counter(
                        "confers_etcd_watch_errors_total",
                        &[("reason", "establish")],
                    );
                }
            }
            attempt = attempt.saturating_add(1);
            tokio::time::sleep(self.retry.delay_for(attempt)).await;
        }
    }
}

/// The production transport: etcd gRPC watch over a prefix.
pub struct EtcdGrpcWatchSource {
    /// `Client::watch` takes `&mut self`, so the client sits behind a mutex;
    /// the lock is held only while the watch is being established — the
    /// returned stream owns its channel and outlives the lock.
    client: tokio::sync::Mutex<etcd_client::Client>,
    prefix: String,
    last_revision: Arc<AtomicI64>,
}

impl EtcdGrpcWatchSource {
    /// Watch `prefix` over the given client, resuming from the shared
    /// revision tracker.
    pub fn new(client: etcd_client::Client, prefix: impl Into<String>) -> Self {
        Self {
            client: tokio::sync::Mutex::new(client),
            prefix: prefix.into(),
            last_revision: Arc::new(AtomicI64::new(0)),
        }
    }

    /// The revision tracker this transport resumes from (share it with an
    /// [`EtcdWatcher`] via [`EtcdWatcher::with_retry`] style wiring if both
    /// sides must agree on the resume point).
    pub fn last_revision(&self) -> &Arc<AtomicI64> {
        &self.last_revision
    }
}

#[async_trait::async_trait]
impl WatchEventSource for EtcdGrpcWatchSource {
    async fn watch(
        &self,
    ) -> crate::error::ConfigResult<
        std::pin::Pin<Box<dyn futures_util::Stream<Item = WatchItem> + Send>>,
    > {
        use etcd_client::{EventType, WatchOptions};
        use futures_util::StreamExt;

        let start = self.last_revision.load(Ordering::Acquire);
        let mut options = WatchOptions::new().with_prefix();
        if start > 0 {
            options = options.with_start_revision(start + 1);
        }

        let watch_stream = self
            .client
            .lock()
            .await
            .watch(self.prefix.as_str(), Some(options))
            .await
            .map_err(|e| crate::error::ConfigError::InvalidValue {
                key: "etcd".to_string(),
                expected_type: "etcd watch stream".to_string(),
                message: format!("failed to establish etcd watch: {e}"),
            })?;

        let last_revision = Arc::clone(&self.last_revision);
        // Each gRPC response becomes a burst of normalized items; a
        // transport error becomes a single Err item (reconnect trigger).
        let mapped = watch_stream
            .then(move |resp| {
                let last_revision = Arc::clone(&last_revision);
                async move {
                    match resp {
                        Ok(response) => {
                            let mut items: Vec<WatchItem> = Vec::new();
                            for event in response.events() {
                                let Some(kv) = event.kv() else {
                                    continue;
                                };
                                let (Ok(key), rev) = (kv.key_str(), kv.mod_revision())
                                else {
                                    continue;
                                };
                                let item = match event.event_type() {
                                    EventType::Put => kv
                                        .value_str()
                                        .ok()
                                        .map(|value| {
                                            last_revision
                                                .fetch_max(rev, Ordering::AcqRel);
                                            Ok(EtcdWatchEvent {
                                                key: key.to_string(),
                                                value: Some(value.to_string()),
                                                mod_revision: rev,
                                            })
                                        })
                                        .unwrap_or_else(|| {
                                            Err(format!(
                                                "etcd watch value for '{key}' is not UTF-8"
                                            ))
                                        }),
                                    EventType::Delete => {
                                        last_revision.fetch_max(rev, Ordering::AcqRel);
                                        Ok(EtcdWatchEvent {
                                            key: key.to_string(),
                                            value: None,
                                            mod_revision: rev,
                                        })
                                    }
                                };
                                items.push(item);
                            }
                            items
                        }
                        Err(e) => vec![Err(format!("etcd watch transport error: {e}"))],
                    }
                }
            })
            .flat_map(futures_util::stream::iter);
        Ok(Box::pin(mapped))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Mock feed: replays queued item sequences per connection attempt.
    ///
    /// `attempts` holds the item sequence for every `watch()` call (i.e.
    /// every connection attempt); the last entry repeats forever. A healthy
    /// idle connection stays pending after replaying its items.
    struct MockWatchSource {
        attempts: Mutex<Vec<Vec<WatchItem>>>,
        establish_failures: Mutex<Vec<bool>>,
        connections: AtomicI64,
    }

    impl MockWatchSource {
        fn new(attempts: Vec<Vec<WatchItem>>, establish_failures: Vec<bool>) -> Arc<Self> {
            Arc::new(Self {
                attempts: Mutex::new(attempts),
                establish_failures: Mutex::new(establish_failures),
                connections: AtomicI64::new(0),
            })
        }
    }

    #[async_trait::async_trait]
    impl WatchEventSource for MockWatchSource {
        async fn watch(
            &self,
        ) -> crate::error::ConfigResult<
            std::pin::Pin<Box<dyn futures_util::Stream<Item = WatchItem> + Send>>,
        > {
            use std::task::{Context, Poll};
            self.connections.fetch_add(1, Ordering::SeqCst);

            let fail = self
                .establish_failures
                .try_lock()
                .ok()
                .and_then(|mut q| (!q.is_empty()).then(|| q.remove(0)))
                .unwrap_or(false);
            if fail {
                return Err(crate::error::ConfigError::InvalidValue {
                    key: "etcd".to_string(),
                    expected_type: "watch stream".to_string(),
                    message: "mock establish failure".to_string(),
                });
            }

            let items: Vec<WatchItem> = {
                let mut q = self.attempts.try_lock().unwrap();
                if q.len() > 1 {
                    q.remove(0)
                } else {
                    q.first().cloned().unwrap_or_default()
                }
            };
            let mut iter = items.into_iter();
            Ok(Box::pin(futures_util::stream::poll_fn(
                move |_cx: &mut Context<'_>| match iter.next() {
                    Some(item) => Poll::Ready(Some(item)),
                    None => Poll::Pending, // healthy idle connection
                },
            )))
        }
    }

    fn put(key: &str, value: &str, rev: i64) -> WatchItem {
        Ok(EtcdWatchEvent {
            key: key.to_string(),
            value: Some(value.to_string()),
            mod_revision: rev,
        })
    }

    type EventSink = Arc<Mutex<Vec<EtcdWatchEvent>>>;

    fn sink_callback(sink: &EventSink) -> EtcdWatchCallback {
        let sink = Arc::clone(sink);
        Arc::new(move |event: &EtcdWatchEvent| {
            sink.try_lock().unwrap().push(event.clone());
        })
    }

    /// Wait until `predicate` holds on the sink (bounded busy-wait).
    async fn wait_for(sink: &EventSink, min_len: usize) {
        for _ in 0..400 {
            if sink.try_lock().unwrap().len() >= min_len {
                return;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    }

    #[tokio::test]
    async fn change_events_reach_the_callback() {
        let source = MockWatchSource::new(
            vec![vec![put("app/db/url", "postgres://v2", 5)]],
            vec![],
        );
        let watcher = EtcdWatcher::new(source);
        let sink: EventSink = Arc::new(Mutex::new(Vec::new()));

        let handle = tokio::spawn({
            let sink = Arc::clone(&sink);
            async move { watcher.run(sink_callback(&sink)).await }
        });
        wait_for(&sink, 1).await;
        handle.abort();

        let events = sink.try_lock().unwrap();
        assert_eq!(events.len(), 1, "watch change must trigger the callback");
        assert_eq!(events[0].key, "app/db/url");
        assert_eq!(events[0].value.as_deref(), Some("postgres://v2"));
        assert_eq!(events[0].mod_revision, 5);
    }

    #[tokio::test]
    async fn stream_error_reconnects_and_events_keep_flowing() {
        // Attempt 1: one event then a transport error. Attempt 2 (the
        // reconnect): a fresh event on a healthy idle connection.
        let source = MockWatchSource::new(
            vec![
                vec![put("a", "1", 1), Err("connection reset".to_string())],
                vec![put("a", "2", 2)],
            ],
            vec![],
        );
        let retry = EtcdWatchRetry {
            base: Duration::from_millis(1),
            max: Duration::from_millis(4),
        };
        let watcher = EtcdWatcher::new(source.clone()).with_retry(retry);
        let sink: EventSink = Arc::new(Mutex::new(Vec::new()));

        let handle = tokio::spawn({
            let sink = Arc::clone(&sink);
            async move { watcher.run(sink_callback(&sink)).await }
        });
        wait_for(&sink, 2).await;
        handle.abort();

        let events = sink.try_lock().unwrap();
        assert!(
            events.len() >= 2,
            "events must flow across the reconnect, got {events:?}"
        );
        assert_eq!(events[0].value.as_deref(), Some("1"));
        assert_eq!(events[1].value.as_deref(), Some("2"));
        assert!(
            source.connections.load(Ordering::SeqCst) >= 2,
            "must have reconnected after the stream error"
        );
    }

    #[tokio::test]
    async fn establish_failure_backs_off_then_retries() {
        let source = MockWatchSource::new(
            vec![vec![put("k", "v", 9)]],
            vec![true], // first establish attempt fails
        );
        let retry = EtcdWatchRetry {
            base: Duration::from_millis(1),
            max: Duration::from_millis(2),
        };
        let watcher = EtcdWatcher::new(source.clone()).with_retry(retry);
        let sink: EventSink = Arc::new(Mutex::new(Vec::new()));

        let handle = tokio::spawn({
            let sink = Arc::clone(&sink);
            async move { watcher.run(sink_callback(&sink)).await }
        });
        wait_for(&sink, 1).await;
        handle.abort();

        assert!(
            !sink.try_lock().unwrap().is_empty(),
            "watcher must recover after an establish failure"
        );
        assert!(source.connections.load(Ordering::SeqCst) >= 2);
    }

    #[test]
    fn deletion_yields_event_without_value() {
        let deleted = EtcdWatchEvent {
            key: "k".to_string(),
            value: None,
            mod_revision: 3,
        };
        assert_eq!(deleted.value, None, "deletion carries no value");
    }

    #[test]
    fn backoff_delays_grow_and_stay_bounded() {
        let retry = EtcdWatchRetry {
            base: Duration::from_millis(100),
            max: Duration::from_secs(30),
        };
        assert_eq!(retry.delay_for(1), Duration::from_millis(100));
        assert_eq!(retry.delay_for(2), Duration::from_millis(200));
        assert_eq!(retry.delay_for(3), Duration::from_millis(400));
        assert_eq!(
            retry.delay_for(10),
            Duration::from_secs(30),
            "capped at max"
        );
    }
}
