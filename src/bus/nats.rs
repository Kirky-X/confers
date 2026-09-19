// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! NATS-based ConfigBus implementation.
//!
//! Every `subscribe()` call creates its own ephemeral JetStream consumer so
//! that all bus instances receive every published event (broadcast semantics,
//! see the module-level "Multi-instance configuration change broadcast"
//! goal). Ephemeral consumers are removed by the NATS server once pull
//! requests stop arriving, so dropped subscriptions leave no server state.

use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use async_nats::jetstream::{self, consumer::DeliverPolicy};
use async_trait::async_trait;
use futures_util::{Stream, StreamExt};

use super::{ConfigBus, ConfigChangeEvent};
use crate::error::{ConfigConfigError, ConfigError, ConfigResult};
use crate::lifecycle::Lifecycle;

/// How long the NATS server may leave a pull consumer idle before removing
/// it. Must stay comfortably above the 30s pull-request expiry used by
/// `consumer.messages()` so a slowly-polled subscription is not reaped early,
/// while still cleaning up consumers of dropped subscriptions.
const CONSUMER_INACTIVE_THRESHOLD: std::time::Duration = std::time::Duration::from_secs(300);

/// Upper bound on how long the events stream retains messages. Subscribers
/// start at `DeliverPolicy::New`, so old events are never replayed; without a
/// retention limit the stream would grow without bound over the lifetime of
/// the deployment.
const STREAM_MAX_AGE: std::time::Duration = std::time::Duration::from_secs(24 * 60 * 60);

pub struct NatsConfigBus {
    client: async_nats::Client,
    subject: String,
    stream_name: String,
}

impl NatsConfigBus {
    pub async fn connect(url: &str, subject: impl Into<String>) -> ConfigResult<Self> {
        let client =
            async_nats::connect(url)
                .await
                .map_err(|e| ConfigError::RemoteUnavailable {
                    error_type: format!("nats_connect: {}", e),
                    retryable: true,
                })?;

        Ok(Self {
            client,
            subject: subject.into(),
            stream_name: "CONFIG_EVENTS".to_string(),
        })
    }

    pub async fn connect_with_options(
        options: async_nats::ConnectOptions,
        url: &str,
        subject: impl Into<String>,
    ) -> ConfigResult<Self> {
        let client = options
            .connect(url)
            .await
            .map_err(|e| ConfigError::RemoteUnavailable {
                error_type: format!("nats_connect: {}", e),
                retryable: true,
            })?;

        Ok(Self {
            client,
            subject: subject.into(),
            stream_name: "CONFIG_EVENTS".to_string(),
        })
    }

    pub fn with_stream_name(mut self, name: impl Into<String>) -> Self {
        self.stream_name = name.into();
        self
    }

    async fn ensure_stream(&self) -> ConfigResult<jetstream::stream::Stream> {
        let jetstream = jetstream::new(self.client.clone());

        let stream = jetstream
            .get_or_create_stream(jetstream::stream::Config {
                name: self.stream_name.clone(),
                subjects: vec![self.subject.clone()],
                // Bound message retention: subscribers start at
                // `DeliverPolicy::New` and never replay history, so keeping
                // events longer than `STREAM_MAX_AGE` serves no purpose.
                max_age: STREAM_MAX_AGE,
                ..Default::default()
            })
            .await
            .map_err(|e| ConfigError::RemoteUnavailable {
                error_type: format!("nats_stream: {}", e),
                retryable: true,
            })?;

        Ok(stream)
    }

    /// Build a NATS-safe consumer name unique to a single `subscribe()` call.
    ///
    /// A JetStream consumer delivers each message to exactly one consumer, so
    /// a consumer shared across instances would distribute configuration
    /// change events instead of broadcasting them. The name embeds the
    /// subject, the process id, a millisecond timestamp and a per-process
    /// sequence number, keeping it collision-free across processes and across
    /// repeated subscriptions.
    fn subscriber_consumer_name(subject: &str) -> String {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        // Consumer names only allow alphanumeric characters plus `-`/`_`;
        // map every other character (e.g. `.` in subjects) to `_` and cap the
        // subject-derived portion to keep names compact.
        const MAX_SUBJECT_PART: usize = 32;
        let subject_part: String = subject
            .chars()
            .take(MAX_SUBJECT_PART)
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
            .collect();
        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or_default();
        format!(
            "confers-{subject_part}-pid{}-t{timestamp_ms}-s{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        )
    }
}

#[async_trait]
impl Lifecycle for NatsConfigBus {
    async fn start(&self) -> Result<(), ConfigConfigError> {
        Ok(())
    }

    async fn stop(&self) -> ConfigResult<()> {
        self.client
            .flush()
            .await
            .map_err(|e| ConfigError::RemoteUnavailable {
                error_type: format!("nats_flush: {}", e),
                retryable: false,
            })
    }
}

#[async_trait]
impl ConfigBus for NatsConfigBus {
    async fn publish(&self, event: ConfigChangeEvent) -> ConfigResult<()> {
        // Ensure the stream exists before publishing
        let _stream = self.ensure_stream().await?;

        let jetstream = jetstream::new(self.client.clone());
        let payload = serde_json::to_vec(&event).map_err(|e| ConfigError::SourceChainError {
            message: format!("serialize event: {}", e),
            source_index: 0,
        })?;

        jetstream
            .publish(self.subject.clone(), payload.into())
            .await
            .map_err(|e| ConfigError::RemoteUnavailable {
                error_type: format!("nats_publish: {}", e),
                retryable: true,
            })?;

        Ok(())
    }

    async fn subscribe(
        &self,
    ) -> ConfigResult<Pin<Box<dyn Stream<Item = ConfigChangeEvent> + Send>>> {
        // Retry stream + consumer creation to handle transient JetStream
        // errors (e.g. "stream not found") that occur under heavy concurrent
        // load when many streams are created simultaneously.
        // Each subscription gets its own ephemeral consumer (unique name,
        // `durable_name: None`): JetStream delivers every message to exactly
        // one consumer, so a shared consumer would distribute events across
        // instances instead of broadcasting them. With no durability and a
        // bounded `inactive_threshold`, the server removes the consumer
        // automatically once the subscription is dropped.
        //
        // `DeliverPolicy::New` matches `InMemoryBus` semantics: subscribers
        // receive only events published *after* they subscribe. Replaying the
        // full stream history (`DeliverPolicy::All`) would make a late or
        // restarting instance re-process every historical change as if it
        // were current — current configuration state comes from the config
        // sources, not from replayed events.
        let consumer_name = Self::subscriber_consumer_name(&self.subject);
        let max_retries = 3u32;
        let (consumer, last_err) = {
            let mut last_err = None;
            let mut consumer = None;
            for attempt in 0..=max_retries {
                match self.ensure_stream().await {
                    Ok(stream) => {
                        match stream
                            .create_consumer(jetstream::consumer::pull::Config {
                                deliver_policy: DeliverPolicy::New,
                                durable_name: None,
                                name: Some(consumer_name.clone()),
                                inactive_threshold: CONSUMER_INACTIVE_THRESHOLD,
                                ..Default::default()
                            })
                            .await
                        {
                            Ok(c) => {
                                consumer = Some(c);
                                break;
                            }
                            Err(e) => {
                                last_err = Some(e);
                                if attempt < max_retries {
                                    tokio::time::sleep(std::time::Duration::from_millis(
                                        100 * (attempt as u64 + 1),
                                    ))
                                    .await;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        // ensure_stream errors are already ConfigError;
                        // propagate immediately (not transient).
                        return Err(e);
                    }
                }
            }
            (consumer, last_err)
        };

        let consumer = consumer.ok_or_else(|| ConfigError::RemoteUnavailable {
            error_type: format!(
                "nats_consumer: {} (after {} retries)",
                last_err
                    .as_ref()
                    .map(|e| e.to_string())
                    .unwrap_or_else(|| "unknown error".into()),
                max_retries
            ),
            retryable: true,
        })?;

        let messages = consumer
            .messages()
            .await
            .map_err(|e| ConfigError::RemoteUnavailable {
                error_type: format!("nats_messages: {}", e),
                retryable: true,
            })?;

        let stream = messages.filter_map(|msg| async move {
            match msg {
                Ok(msg) => {
                    let payload = msg.message.payload.clone();
                    let event: Result<ConfigChangeEvent, _> = serde_json::from_slice(&payload);
                    match event {
                        Ok(event) => {
                            let _ = msg.ack().await;
                            Some(event)
                        }
                        Err(_) => {
                            // Deserialization failed — nak so the server can redeliver
                            // or move to dead-letter. Ignore nak errors (best-effort).
                            let _ = msg
                                .ack_with(async_nats::jetstream::AckKind::Nak(None))
                                .await;
                            None
                        }
                    }
                }
                Err(e) => {
                    // Transport error while polling the consumer (the bus may
                    // be offline). The async-nats client reconnects
                    // automatically; log so the outage is visible instead of
                    // silently dropping the error.
                    log::warn!(
                        "NATS config-bus consumer error (bus may be offline; the client will reconnect automatically): {e}"
                    );
                    None
                }
            }
        });

        Ok(Box::pin(stream))
    }
}

pub struct NatsBusBuilder {
    url: Option<String>,
    subject: Option<String>,
    stream_name: Option<String>,
    options: async_nats::ConnectOptions,
}

impl NatsBusBuilder {
    pub fn new() -> Self {
        Self {
            url: None,
            subject: None,
            stream_name: None,
            options: async_nats::ConnectOptions::new(),
        }
    }

    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }

    pub fn subject(mut self, subject: impl Into<String>) -> Self {
        self.subject = Some(subject.into());
        self
    }

    pub fn stream_name(mut self, name: impl Into<String>) -> Self {
        self.stream_name = Some(name.into());
        self
    }

    pub fn options(mut self, options: async_nats::ConnectOptions) -> Self {
        self.options = options;
        self
    }

    pub async fn build(self) -> ConfigResult<NatsConfigBus> {
        let url = self.url.ok_or(ConfigError::InvalidValue {
            key: "nats_url".to_string(),
            expected_type: "string".to_string(),
            message: "NATS URL is required".to_string(),
        })?;

        let subject = self.subject.unwrap_or_else(|| "config.events".to_string());

        let mut bus = NatsConfigBus::connect_with_options(self.options, &url, subject).await?;

        if let Some(name) = self.stream_name {
            bus.stream_name = name;
        }

        Ok(bus)
    }
}

impl Default for NatsBusBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::StreamExt;
    use tokio::time::{Duration, timeout};

    /// Probe whether the local NATS test service is accepting connections.
    fn nats_ready() -> bool {
        std::net::TcpStream::connect(("127.0.0.1", 4222)).is_ok()
    }

    /// Process-unique, NATS-safe (alphanumeric-only) name for subjects and
    /// stream names to keep parallel tests isolated.
    fn unique(suffix: &str) -> String {
        use std::sync::atomic::{AtomicU64, Ordering};
        static N: AtomicU64 = AtomicU64::new(0);
        let clean: String = suffix
            .chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .collect();
        format!("confersunit{}{}", N.fetch_add(1, Ordering::SeqCst), clean)
    }

    fn event(checksum: &str) -> ConfigChangeEvent {
        ConfigChangeEvent::new(
            "test-instance",
            "unit-test",
            vec!["key.alpha".to_string()],
            checksum,
        )
    }

    // ==================== NatsBusBuilder ====================

    #[test]
    fn test_builder_new_defaults() {
        let b = NatsBusBuilder::new();
        assert!(b.url.is_none());
        assert!(b.subject.is_none());
        assert!(b.stream_name.is_none());
    }

    #[test]
    fn test_builder_default_impl_matches_new() {
        let d = NatsBusBuilder::default();
        assert!(d.url.is_none());
        assert!(d.subject.is_none());
        assert!(d.stream_name.is_none());
    }

    #[test]
    fn test_builder_setters_chain_and_store() {
        let b = NatsBusBuilder::new()
            .url("nats://127.0.0.1:4222")
            .subject("unitsubject")
            .stream_name("UNITSTREAM")
            .options(async_nats::ConnectOptions::new());
        assert_eq!(b.url.as_deref(), Some("nats://127.0.0.1:4222"));
        assert_eq!(b.subject.as_deref(), Some("unitsubject"));
        assert_eq!(b.stream_name.as_deref(), Some("UNITSTREAM"));
    }

    #[tokio::test]
    async fn test_build_without_url_returns_invalid_value() {
        let err = NatsBusBuilder::new()
            .subject("s")
            .build()
            .await
            .err()
            .expect("build should error");
        match err {
            ConfigError::InvalidValue {
                key,
                expected_type,
                message,
            } => {
                assert_eq!(key, "nats_url");
                assert_eq!(expected_type, "string");
                assert!(message.contains("required"), "message={}", message);
            }
            other => panic!("expected InvalidValue, got {:?}", other),
        }
    }

    // ==================== connect error paths (no service needed) ====================

    #[tokio::test]
    async fn test_connect_invalid_url_is_retryable() {
        let err = NatsConfigBus::connect("not-a-valid-url", "subj")
            .await
            .err()
            .expect("connect should error");
        match err {
            ConfigError::RemoteUnavailable {
                retryable,
                error_type,
            } => {
                assert!(retryable);
                assert!(
                    error_type.contains("nats_connect"),
                    "error_type={}",
                    error_type
                );
            }
            other => panic!("expected RemoteUnavailable, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_connect_with_options_invalid_url_is_retryable() {
        let err = NatsConfigBus::connect_with_options(
            async_nats::ConnectOptions::new(),
            "not-a-valid-url",
            "subj",
        )
        .await
        .err()
        .expect("connect_with_options should error");
        assert!(matches!(
            err,
            ConfigError::RemoteUnavailable {
                retryable: true,
                ..
            }
        ));
    }

    // ==================== consumer naming (no service needed) ====================

    #[test]
    fn test_subscriber_consumer_name_unique_per_subscription() {
        let a = NatsConfigBus::subscriber_consumer_name("config.events");
        let b = NatsConfigBus::subscriber_consumer_name("config.events");
        assert_ne!(
            a, b,
            "every subscribe() call must own its own consumer name"
        );
    }

    #[test]
    fn test_subscriber_consumer_name_is_nats_safe_and_subject_derived() {
        let name = NatsConfigBus::subscriber_consumer_name("cfg.events.*.>");
        assert!(
            name.chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'),
            "consumer names must only contain alphanumerics, '-' and '_': {name}"
        );
        assert!(
            name.starts_with("confers-cfg_events_"),
            "subject must be embedded (sanitized) for debuggability: {name}"
        );
    }

    // ==================== with_stream_name (not exercised by builder path) ====================

    #[tokio::test]
    async fn test_with_stream_name_sets_field() {
        assert!(
            nats_ready(),
            "NATS service required at 127.0.0.1:4222 for this test"
        );
        let bus = NatsConfigBus::connect("nats://127.0.0.1:4222", unique("subj"))
            .await
            .expect("connect");
        let bus = bus.with_stream_name("CUSTOMSTREAM");
        assert_eq!(bus.stream_name, "CUSTOMSTREAM");
    }

    // ==================== Service-required tests ====================

    #[tokio::test]
    async fn test_build_success_uses_defaults() {
        assert!(
            nats_ready(),
            "NATS service required at 127.0.0.1:4222 for this test"
        );
        let bus = NatsBusBuilder::new()
            .url("nats://127.0.0.1:4222")
            .build()
            .await
            .expect("build should succeed");
        assert_eq!(bus.subject, "config.events");
        assert_eq!(bus.stream_name, "CONFIG_EVENTS");
    }

    #[tokio::test]
    async fn test_build_with_custom_stream_name() {
        assert!(
            nats_ready(),
            "NATS service required at 127.0.0.1:4222 for this test"
        );
        let name = unique("STREAM");
        let bus = NatsBusBuilder::new()
            .url("nats://127.0.0.1:4222")
            .subject(unique("subj"))
            .stream_name(name.clone())
            .build()
            .await
            .expect("build");
        assert_eq!(bus.stream_name, name);
    }

    #[tokio::test]
    async fn test_lifecycle_start_stop() {
        assert!(
            nats_ready(),
            "NATS service required at 127.0.0.1:4222 for this test"
        );
        let bus = NatsBusBuilder::new()
            .url("nats://127.0.0.1:4222")
            .build()
            .await
            .expect("build");
        bus.start().await.expect("start ok");
        bus.stop().await.expect("stop (flush) ok");
    }

    #[tokio::test]
    async fn test_publish_subscribe_roundtrip() {
        assert!(
            nats_ready(),
            "NATS service required at 127.0.0.1:4222 for this test"
        );
        let subject = unique("roundtrip");
        let bus = NatsBusBuilder::new()
            .url("nats://127.0.0.1:4222")
            .subject(subject.clone())
            .stream_name(unique("RTSTREAM"))
            .build()
            .await
            .expect("build");

        let mut rx = bus.subscribe().await.expect("subscribe");
        let ev = event("rt-ck");
        bus.publish(ev.clone()).await.expect("publish");

        let received = timeout(Duration::from_secs(5), rx.next())
            .await
            .expect("timed out waiting for message")
            .expect("stream ended");

        assert_eq!(received.instance_id, ev.instance_id);
        assert_eq!(received.source, ev.source);
        assert_eq!(received.changed_keys, ev.changed_keys);
        assert_eq!(received.checksum, ev.checksum);
    }

    #[tokio::test]
    async fn test_subscribe_broadcasts_to_all_subscribers() {
        // JetStream delivers each message to exactly one consumer, so this
        // only passes when every subscribe() call creates its own consumer
        // (issue #11: the previous shared durable consumer distributed
        // events across instances instead of broadcasting them).
        assert!(
            nats_ready(),
            "NATS service required at 127.0.0.1:4222 for this test"
        );
        let bus = NatsBusBuilder::new()
            .url("nats://127.0.0.1:4222")
            .subject(unique("bcast"))
            .stream_name(unique("BCAST"))
            .build()
            .await
            .expect("build");

        let mut rx1 = bus.subscribe().await.expect("subscribe 1");
        let mut rx2 = bus.subscribe().await.expect("subscribe 2");

        let ev = event("bcast-ck");
        bus.publish(ev.clone()).await.expect("publish");

        for rx in [&mut rx1, &mut rx2] {
            let received = timeout(Duration::from_secs(5), rx.next())
                .await
                .expect("timed out waiting for message")
                .expect("stream ended");
            assert_eq!(received.checksum, "bcast-ck");
        }
    }

    #[tokio::test]
    async fn test_subscribe_invalid_stream_name_returns_error() {
        // NATS stream names cannot contain '.'. ensure_stream's
        // get_or_create_stream must fail, exercising the error path.
        assert!(
            nats_ready(),
            "NATS service required at 127.0.0.1:4222 for this test"
        );
        let bus = NatsBusBuilder::new()
            .url("nats://127.0.0.1:4222")
            .subject(unique("badstream"))
            .stream_name("invalid.stream.name")
            .build()
            .await
            .expect("build");
        let err = bus.subscribe().await.err().expect("subscribe should error");
        assert!(matches!(
            err,
            ConfigError::RemoteUnavailable {
                retryable: true,
                ..
            }
        ));
    }

    #[tokio::test]
    async fn test_subscribe_skips_invalid_payload() {
        // Publish raw non-JSON bytes via a separate jetstream client to the
        // same subject. The subscribe stream's decode-skip branch must skip
        // them and still deliver a subsequently published valid event.
        assert!(
            nats_ready(),
            "NATS service required at 127.0.0.1:4222 for this test"
        );
        let subject = unique("badsubj");
        let stream_name = unique("BADSTREAM");
        let bus = NatsBusBuilder::new()
            .url("nats://127.0.0.1:4222")
            .subject(subject.clone())
            .stream_name(stream_name.clone())
            .build()
            .await
            .expect("build");

        let mut rx = bus.subscribe().await.expect("subscribe");

        // Inject invalid bytes via a standalone jetstream client on the same
        // stream/subject so the consumer receives them.
        let js_client = async_nats::connect("nats://127.0.0.1:4222")
            .await
            .expect("raw client connect");
        let js = async_nats::jetstream::new(js_client);
        js.publish(subject.clone(), b"not-json".to_vec().into())
            .await
            .expect("raw publish");
        // Allow the server to persist the raw message before publishing the
        // valid event so the consumer observes them in order.
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Now publish a valid event through the bus.
        let ev = event("good-ck");
        bus.publish(ev.clone()).await.expect("publish");

        let received = timeout(Duration::from_secs(5), rx.next())
            .await
            .expect("timed out waiting for valid message")
            .expect("stream ended");

        assert_eq!(received.checksum, "good-ck");
        assert_eq!(received.instance_id, ev.instance_id);
    }
}
