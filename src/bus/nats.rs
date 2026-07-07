// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! NATS-based ConfigBus implementation.

use std::pin::Pin;

use async_nats::jetstream::{self, consumer::DeliverPolicy};
use async_trait::async_trait;
use futures_util::{Stream, StreamExt};

use super::{ConfigBus, ConfigChangeEvent};
use crate::error::{ConfigConfigError, ConfigError, ConfigResult};
use crate::lifecycle::Lifecycle;

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
                ..Default::default()
            })
            .await
            .map_err(|e| ConfigError::RemoteUnavailable {
                error_type: format!("nats_stream: {}", e),
                retryable: true,
            })?;

        Ok(stream)
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
        let stream = self.ensure_stream().await?;

        let consumer = stream
            .get_or_create_consumer(
                "config-consumer",
                jetstream::consumer::pull::Config {
                    deliver_policy: DeliverPolicy::All,
                    ..Default::default()
                },
            )
            .await
            .map_err(|e| ConfigError::RemoteUnavailable {
                error_type: format!("nats_consumer: {}", e),
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
                    if let Ok(event) = event {
                        let _ = msg.ack().await;
                        return Some(event);
                    }
                }
                Err(_e) => {
                    // Silently ignore message errors and continue
                }
            }
            None
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
    use tokio::time::{timeout, Duration};

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
