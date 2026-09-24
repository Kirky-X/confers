// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! Redis-based ConfigBus implementation.

use std::pin::Pin;
use std::time::Duration;

use async_trait::async_trait;
use futures_util::{Stream, StreamExt};
use redis::AsyncCommands;

use super::{ConfigBus, ConfigChangeEvent};
use crate::error::{ConfigConfigError, ConfigError, ConfigResult};
use crate::i18n::{tr, tr_args};
use crate::lifecycle::Lifecycle;

/// Default retry wait time when no message is available (100ms).
const DEFAULT_RETRY_WAIT_MS: u64 = 100;

/// Default error retry wait time (1 second).
const DEFAULT_ERROR_RETRY_WAIT_SECS: u64 = 1;

/// Redis default port.
const DEFAULT_REDIS_PORT: u16 = 6379;

/// Initial reconnect backoff after the pubsub connection drops.
pub const REDIS_RECONNECT_INITIAL_DELAY: Duration = Duration::from_secs(1);

/// Cap of the exponential reconnect backoff 1s → 2s → 4s → … → 30s.
pub const REDIS_RECONNECT_MAX_DELAY: Duration = Duration::from_secs(30);

pub struct RedisConfigBus {
    client: redis::Client,
    channel: String,
    /// Retry wait time in milliseconds when no message is available.
    ///
    /// Historical field retained for builder API compatibility. The new
    /// `subscribe()` implementation uses a dedicated PubSub connection with
    /// `on_message()` push delivery, so this value is no longer read.
    #[allow(dead_code)]
    retry_wait_ms: u64,
    /// Error retry wait time in seconds.
    ///
    /// Historical field retained for builder API compatibility. See
    /// `retry_wait_ms` for context.
    #[allow(dead_code)]
    error_retry_wait_secs: u64,
    /// Initial reconnect backoff for the subscription loop.
    reconnect_initial: Duration,
    /// Reconnect backoff cap for the subscription loop.
    reconnect_max: Duration,
}

impl RedisConfigBus {
    fn sanitize_url(url: &str) -> String {
        if let Ok(parsed) = url::Url::parse(url) {
            let host = parsed.host_str().unwrap_or("unknown");
            let port = parsed.port().unwrap_or(DEFAULT_REDIS_PORT);
            format!("{}:{}", host, port)
        } else {
            "invalid_url".to_string()
        }
    }

    pub async fn connect(url: &str, channel: impl Into<String>) -> ConfigResult<Self> {
        Self::connect_with_config(
            url,
            channel,
            DEFAULT_RETRY_WAIT_MS,
            DEFAULT_ERROR_RETRY_WAIT_SECS,
        )
        .await
    }

    /// Connect with custom retry wait times.
    pub async fn connect_with_config(
        url: &str,
        channel: impl Into<String>,
        retry_wait_ms: u64,
        error_retry_wait_secs: u64,
    ) -> ConfigResult<Self> {
        let safe_host = Self::sanitize_url(url);

        let client = redis::Client::open(url).map_err(|e| ConfigError::RemoteUnavailable {
            error_type: format!("redis_connection_failed: host={}, error={}", safe_host, e),
            retryable: true,
        })?;

        Ok(Self {
            client,
            channel: channel.into(),
            retry_wait_ms,
            error_retry_wait_secs,
            reconnect_initial: REDIS_RECONNECT_INITIAL_DELAY,
            reconnect_max: REDIS_RECONNECT_MAX_DELAY,
        })
    }

    /// Establish a dedicated PubSub connection and subscribe to the channel.
    ///
    /// Used by the reconnecting `subscribe()` loop: each iteration opens a
    /// fresh connection, because a connection lost is not reusable.
    async fn connect_pubsub(
        client: &redis::Client,
        channel: &str,
    ) -> ConfigResult<redis::aio::PubSub> {
        // Use a dedicated PubSub connection (not multiplexed). Redis' SUBSCRIBE
        // command transitions the connection into subscription mode, which is
        // incompatible with multiplexed-connection query semantics. The previous
        // implementation used `redis::cmd("SUBSCRIBE").query_async()` on a
        // multiplexed connection, which never delivers real published messages.
        // R3-L5: connect 包超时 —— 对黑洞地址(SYN 丢弃)无超时会让重连
        // 循环无限悬挂、退避停摆。
        let mut pubsub = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            client.get_async_pubsub(),
        )
        .await
        .map_err(|_| ConfigError::RemoteUnavailable {
            error_type: tr("error-redis-pubsub-connect-timeout"),
            retryable: true,
        })?
        .map_err(|e| ConfigError::RemoteUnavailable {
            error_type: tr_args(
                "error-redis-pubsub-connect-failed",
                &[("message", e.to_string())],
            ),
            retryable: true,
        })?;

        pubsub
            .subscribe(channel)
            .await
            .map_err(|e| ConfigError::RemoteUnavailable {
                error_type: tr_args(
                    "error-redis-subscribe-failed",
                    &[("message", e.to_string())],
                ),
                retryable: true,
            })?;

        Ok(pubsub)
    }
}

#[async_trait]
impl Lifecycle for RedisConfigBus {
    async fn start(&self) -> Result<(), ConfigConfigError> {
        Ok(())
    }

    async fn stop(&self) -> ConfigResult<()> {
        Ok(())
    }
}

#[async_trait]
impl ConfigBus for RedisConfigBus {
    async fn publish(&self, event: ConfigChangeEvent) -> ConfigResult<()> {
        let mut conn = self
            .client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| ConfigError::RemoteUnavailable {
                error_type: format!("redis_connection: {}", e),
                retryable: true,
            })?;

        let payload = serde_json::to_vec(&event).map_err(|e| ConfigError::SourceChainError {
            message: format!("serialize event: {}", e),
            source_index: 0,
        })?;

        conn.publish::<_, _, ()>(&self.channel, payload)
            .await
            .map_err(|e| ConfigError::RemoteUnavailable {
                error_type: format!("redis_publish: {}", e),
                retryable: true,
            })?;

        Ok(())
    }

    async fn subscribe(
        &self,
    ) -> ConfigResult<Pin<Box<dyn Stream<Item = ConfigChangeEvent> + Send>>> {
        // The first connection is established eagerly so a dead endpoint
        // still surfaces as an immediate `subscribe()` error (the historical
        // contract, pinned by tests). Every subsequent connection is
        // managed by the reconnecting loop below — the pubsub stream ends
        // whenever the connection drops (network blip, server restart, idle
        // timeout); previously that terminated the returned stream silently,
        // so subscribers stopped receiving events forever. Now the loop
        // reconnects with exponential backoff (`reconnect_initial` →
        // `reconnect_max`, doubling), logging a warning for every drop so
        // the outage is observable. Payload decode errors and JSON
        // deserialization errors are skipped (Rule 12: errors that cannot be
        // meaningfully surfaced to the stream consumer are skipped via
        // continue, not silently swallowed).
        let client = self.client.clone();
        let channel = self.channel.clone();
        let reconnect_initial = self.reconnect_initial;
        let reconnect_max = self.reconnect_max;
        let mut pubsub = Self::connect_pubsub(&client, &channel).await?;

        let stream = async_stream::stream! {
            let mut backoff = reconnect_initial;
            loop {
                {
                    let mut msg_stream = pubsub.on_message();
                    while let Some(msg) = msg_stream.next().await {
                        let payload: Vec<u8> = match msg.get_payload() {
                            Ok(p) => p,
                            Err(_) => continue,
                        };
                        match serde_json::from_slice::<ConfigChangeEvent>(&payload) {
                            Ok(event) => yield event,
                            Err(_) => continue,
                        }
                    }
                }
                log::warn!(
                    "{}",
                    tr_args(
                        "log-redis-pubsub-connection-lost",
                        &[
                            ("channel", channel.clone()),
                            ("backoff", format!("{backoff:?}")),
                        ]
                    )
                );
                // Reconnect with exponential backoff until the subscription
                // is re-established, then reset the backoff.
                loop {
                    tokio::time::sleep(backoff).await;
                    backoff = backoff.saturating_mul(2).min(reconnect_max);
                    match Self::connect_pubsub(&client, &channel).await {
                        Ok(new_pubsub) => {
                            backoff = reconnect_initial;
                            pubsub = new_pubsub;
                            break;
                        }
                        Err(e) => {
                            log::warn!(
                                "{}",
                                tr_args(
                                    "log-redis-pubsub-reconnect-failed",
                                    &[
                                        ("channel", channel.clone()),
                                        ("message", e.to_string()),
                                        ("backoff", format!("{backoff:?}")),
                                    ]
                                )
                            );
                        }
                    }
                }
            }
        };

        Ok(Box::pin(stream))
    }
}

pub struct RedisBusBuilder {
    url: Option<String>,
    channel: Option<String>,
    /// Retry wait time in milliseconds when no message is available.
    retry_wait_ms: u64,
    /// Error retry wait time in seconds.
    error_retry_wait_secs: u64,
    /// Initial reconnect backoff for the subscription loop.
    reconnect_initial: Duration,
    /// Reconnect backoff cap for the subscription loop.
    reconnect_max: Duration,
}

impl RedisBusBuilder {
    pub fn new() -> Self {
        Self {
            url: None,
            channel: None,
            retry_wait_ms: DEFAULT_RETRY_WAIT_MS,
            error_retry_wait_secs: DEFAULT_ERROR_RETRY_WAIT_SECS,
            reconnect_initial: REDIS_RECONNECT_INITIAL_DELAY,
            reconnect_max: REDIS_RECONNECT_MAX_DELAY,
        }
    }

    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }

    pub fn channel(mut self, channel: impl Into<String>) -> Self {
        self.channel = Some(channel.into());
        self
    }

    /// Set retry wait time in milliseconds when no message is available.
    ///
    /// Default: 100ms.
    pub fn retry_wait_ms(mut self, ms: u64) -> Self {
        self.retry_wait_ms = ms;
        self
    }

    /// Set error retry wait time in seconds.
    ///
    /// Default: 1 second.
    pub fn error_retry_wait_secs(mut self, secs: u64) -> Self {
        self.error_retry_wait_secs = secs;
        self
    }

    /// Set the initial reconnect backoff of the subscription loop.
    ///
    /// Default: 1 second. The backoff doubles up to `reconnect_delay_max`
    /// while the connection keeps failing and resets on a successful
    /// (re)subscription.
    pub fn reconnect_delay_initial(mut self, delay: Duration) -> Self {
        self.reconnect_initial = delay;
        self
    }

    /// Set the reconnect backoff cap of the subscription loop.
    ///
    /// Default: 30 seconds.
    pub fn reconnect_delay_max(mut self, delay: Duration) -> Self {
        self.reconnect_max = delay;
        self
    }

    pub async fn build(self) -> ConfigResult<RedisConfigBus> {
        let url = self.url.ok_or(ConfigError::InvalidValue {
            key: "redis_url".to_string(),
            expected_type: "string".to_string(),
            message: "Redis URL is required".to_string(),
        })?;

        let channel = self.channel.unwrap_or_else(|| "config:events".to_string());

        let mut bus = RedisConfigBus::connect_with_config(
            &url,
            channel,
            self.retry_wait_ms,
            self.error_retry_wait_secs,
        )
        .await?;
        bus.reconnect_initial = self.reconnect_initial;
        bus.reconnect_max = self.reconnect_max;
        Ok(bus)
    }
}

impl Default for RedisBusBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::StreamExt;
    use tokio::time::{Duration, timeout};

    /// Probe whether the local Redis test service is accepting connections.
    fn redis_ready() -> bool {
        std::net::TcpStream::connect(("127.0.0.1", 16379)).is_ok()
    }

    /// Process-unique channel name to keep parallel tests isolated.
    fn unique(suffix: &str) -> String {
        use std::sync::atomic::{AtomicU64, Ordering};
        static N: AtomicU64 = AtomicU64::new(0);
        format!(
            "confers.unit.{}.{}",
            N.fetch_add(1, Ordering::SeqCst),
            suffix
        )
    }

    /// Obtain a port that is (almost certainly) closed to trigger connection
    /// errors without relying on a hard-coded port number.
    fn closed_port() -> u16 {
        let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        port
    }

    fn event(checksum: &str) -> ConfigChangeEvent {
        ConfigChangeEvent::new(
            "test-instance",
            "unit-test",
            vec!["key.alpha".to_string()],
            checksum,
        )
    }

    // ==================== sanitize_url (private helper) ====================

    #[test]
    fn test_sanitize_url_valid_host_and_port() {
        assert_eq!(
            RedisConfigBus::sanitize_url("redis://1.2.3.4:7000"),
            "1.2.3.4:7000"
        );
    }

    #[test]
    fn test_sanitize_url_uses_default_port_when_absent() {
        assert_eq!(
            RedisConfigBus::sanitize_url("redis://1.2.3.4"),
            "1.2.3.4:6379"
        );
    }

    #[test]
    fn test_sanitize_url_invalid_returns_sentinel() {
        assert_eq!(RedisConfigBus::sanitize_url("not-a-url"), "invalid_url");
    }

    // ==================== RedisBusBuilder ====================

    #[test]
    fn test_builder_new_defaults() {
        let b = RedisBusBuilder::new();
        assert!(b.url.is_none());
        assert!(b.channel.is_none());
        assert_eq!(b.retry_wait_ms, DEFAULT_RETRY_WAIT_MS);
        assert_eq!(b.error_retry_wait_secs, DEFAULT_ERROR_RETRY_WAIT_SECS);
    }

    #[test]
    fn test_builder_default_impl_matches_new() {
        let d = RedisBusBuilder::default();
        assert!(d.url.is_none());
        assert!(d.channel.is_none());
        assert_eq!(d.retry_wait_ms, DEFAULT_RETRY_WAIT_MS);
        assert_eq!(d.error_retry_wait_secs, DEFAULT_ERROR_RETRY_WAIT_SECS);
    }

    #[test]
    fn test_builder_setters_chain_and_store() {
        let b = RedisBusBuilder::new()
            .url("redis://127.0.0.1:16379")
            .channel("unit-chan")
            .retry_wait_ms(42)
            .error_retry_wait_secs(3);
        assert_eq!(b.url.as_deref(), Some("redis://127.0.0.1:16379"));
        assert_eq!(b.channel.as_deref(), Some("unit-chan"));
        assert_eq!(b.retry_wait_ms, 42);
        assert_eq!(b.error_retry_wait_secs, 3);
    }

    #[tokio::test]
    async fn test_build_without_url_returns_invalid_value() {
        let err = RedisBusBuilder::new()
            .channel("c")
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
                assert_eq!(key, "redis_url");
                assert_eq!(expected_type, "string");
                assert!(message.contains("required"), "message={}", message);
            }
            other => panic!("expected InvalidValue, got {:?}", other),
        }
    }

    // ==================== connect error paths (no service needed) ====================

    #[tokio::test]
    async fn test_connect_invalid_url_is_retryable() {
        let err = RedisConfigBus::connect("not-a-valid-url", "chan")
            .await
            .err()
            .expect("connect should error");
        match err {
            ConfigError::RemoteUnavailable {
                retryable,
                error_type,
            } => {
                assert!(retryable, "should be retryable");
                // sanitize_url returns "invalid_url" for unparseable input, and
                // the error_type embeds the sanitized host.
                assert!(
                    error_type.contains("invalid_url")
                        || error_type.contains("redis_connection_failed"),
                    "error_type={}",
                    error_type
                );
            }
            other => panic!("expected RemoteUnavailable, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_connect_with_config_dead_port_opens_client() {
        // Client::open succeeds for a syntactically valid URL pointing at a
        // closed port; failure surfaces later at publish/subscribe time.
        let port = closed_port();
        let bus = RedisConfigBus::connect_with_config(
            &format!("redis://127.0.0.1:{}", port),
            "chan",
            10,
            1,
        )
        .await
        .expect("client open should succeed for valid URL");
        assert_eq!(bus.channel, "chan");
        assert_eq!(bus.retry_wait_ms, 10);
        assert_eq!(bus.error_retry_wait_secs, 1);
    }

    #[tokio::test]
    async fn test_publish_dead_port_returns_retryable_error() {
        let port = closed_port();
        let bus = RedisConfigBus::connect(&format!("redis://127.0.0.1:{}", port), "chan")
            .await
            .expect("client open succeeds");
        let result = timeout(Duration::from_secs(3), bus.publish(event("dead"))).await;
        let err = result.expect("publish should not time out").unwrap_err();
        assert!(matches!(
            err,
            ConfigError::RemoteUnavailable {
                retryable: true,
                ..
            }
        ));
    }

    #[tokio::test]
    async fn test_subscribe_dead_port_returns_retryable_error() {
        let port = closed_port();
        let bus = RedisConfigBus::connect(&format!("redis://127.0.0.1:{}", port), "chan")
            .await
            .expect("client open succeeds");
        let result = timeout(Duration::from_secs(3), bus.subscribe()).await;
        let err = result
            .expect("subscribe should not time out")
            .err()
            .expect("subscribe should have errored");
        assert!(matches!(
            err,
            ConfigError::RemoteUnavailable {
                retryable: true,
                ..
            }
        ));
    }

    // ==================== Lifecycle (start/stop are no-ops for Redis) ====================

    #[tokio::test]
    async fn test_lifecycle_start_stop_are_noops() {
        // start/stop do not require a live connection for Redis.
        let port = closed_port();
        let bus = RedisConfigBus::connect(&format!("redis://127.0.0.1:{}", port), "chan")
            .await
            .expect("client open succeeds");
        bus.start().await.expect("start is always Ok");
        bus.stop().await.expect("stop is always Ok");
    }

    // ==================== Service-required tests ====================

    #[tokio::test]
    async fn test_build_success_uses_default_channel() {
        if !redis_ready() {
            eprintln!("Skipping test: Redis not available");
            return;
        }
        let bus = RedisBusBuilder::new()
            .url("redis://127.0.0.1:16379")
            .build()
            .await
            .expect("build should succeed with live Redis");
        assert_eq!(bus.channel, "config:events");
    }

    #[tokio::test]
    async fn test_publish_subscribe_roundtrip() {
        if !redis_ready() {
            eprintln!("Skipping test: Redis not available");
            return;
        }
        let channel = unique("roundtrip");
        let bus = RedisConfigBus::connect("redis://127.0.0.1:16379", &channel)
            .await
            .expect("connect");
        let mut rx = bus.subscribe().await.expect("subscribe");

        // Give Redis time to register the pubsub subscription before publishing
        // (at-most-once delivery race).
        tokio::time::sleep(Duration::from_millis(150)).await;

        let ev = event("rt-ck");
        bus.publish(ev.clone()).await.expect("publish");

        let received = timeout(Duration::from_secs(2), rx.next())
            .await
            .expect("timed out waiting for message")
            .expect("stream ended");

        assert_eq!(received.instance_id, ev.instance_id);
        assert_eq!(received.source, ev.source);
        assert_eq!(received.changed_keys, ev.changed_keys);
        assert_eq!(received.checksum, ev.checksum);
    }

    #[tokio::test]
    async fn test_subscribe_skips_invalid_payload() {
        // Publish raw non-JSON bytes directly to the channel via a separate
        // client. The subscribe stream's decode-error continue branch must
        // skip them and still deliver a subsequently published valid event.
        if !redis_ready() {
            eprintln!("Skipping test: Redis not available");
            return;
        }
        let channel = unique("badpayload");
        let bus = RedisConfigBus::connect("redis://127.0.0.1:16379", &channel)
            .await
            .expect("connect");
        let mut rx = bus.subscribe().await.expect("subscribe");
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Inject an invalid payload directly via a standalone client.
        let pub_client = redis::Client::open("redis://127.0.0.1:16379").unwrap();
        let mut conn = pub_client
            .get_multiplexed_async_connection()
            .await
            .expect("pub connection");
        conn.publish::<_, _, ()>(&channel, b"not-json".to_vec())
            .await
            .expect("raw publish");

        // Now publish a valid event through the bus.
        let ev = event("good-ck");
        bus.publish(ev.clone()).await.expect("publish");

        let received = timeout(Duration::from_secs(2), rx.next())
            .await
            .expect("timed out waiting for valid message")
            .expect("stream ended");

        // The invalid payload must have been skipped, leaving the valid event.
        assert_eq!(received.checksum, "good-ck");
        assert_eq!(received.instance_id, ev.instance_id);
    }

    // ==================== pubsub reconnect loop (mock) ====================

    /// RESP helpers for a hand-rolled Redis pubsub mock (same convention as
    /// the hand-written TCP mocks in remote/k8s.rs and remote/nacos.rs).
    mod resp_mock {
        /// Bulk string: `$<len>\r\n<data>\r\n`.
        fn bulk(data: &[u8]) -> Vec<u8> {
            let mut out = format!("${}\r\n", data.len()).into_bytes();
            out.extend_from_slice(data);
            out.extend_from_slice(b"\r\n");
            out
        }

        /// Server confirmation for `SUBSCRIBE <channel>`:
        /// `*3\r\n$9\r\nsubscribe\r\n<channel>\r\n:1\r\n`.
        pub fn subscribe_confirmation(channel: &str) -> Vec<u8> {
            let mut out = b"*3\r\n".to_vec();
            out.extend_from_slice(&bulk(b"subscribe"));
            out.extend_from_slice(&bulk(channel.as_bytes()));
            out.extend_from_slice(b":1\r\n");
            out
        }

        /// Pushed message: `*3\r\n$7\r\nmessage\r\n<channel>\r\n<payload>\r\n`.
        pub fn message(channel: &str, payload: &[u8]) -> Vec<u8> {
            let mut out = b"*3\r\n".to_vec();
            out.extend_from_slice(&bulk(b"message"));
            out.extend_from_slice(&bulk(channel.as_bytes()));
            out.extend_from_slice(&bulk(payload));
            out
        }

        /// Reply `+OK\r\n` once per command array found in `buf` (the client
        /// pipelines setup commands like `CLIENT SETINFO` before SUBSCRIBE).
        pub fn ok_replies(buf: &[u8]) -> Vec<u8> {
            // Each RESP array command begins with `*<n>\r\n`.
            let commands = buf
                .split(|&b| b == b'*')
                .skip(1)
                .filter(|f| !f.is_empty())
                .count();
            b"+OK\r\n".repeat(commands)
        }
    }

    /// acceptance: after the pubsub connection drops, the subscription
    /// loop reconnects (short backoff configured via the builder) and a
    /// subsequently published event is delivered again — the stream no
    /// longer terminates silently.
    #[tokio::test]
    async fn test_subscribe_reconnects_after_connection_drop() {
        use std::sync::Arc as StdArc;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind ephemeral port");
        let addr = listener.local_addr().expect("local addr");
        let channel = format!("confers-reconnect-test-{}", std::process::id());

        let conns = StdArc::new(AtomicUsize::new(0));
        let conns_server = conns.clone();
        let channel_server = channel.clone();
        let server = tokio::spawn(async move {
            loop {
                let Ok((mut stream, _)) = listener.accept().await else {
                    break;
                };
                let conns = conns_server.clone();
                let channel = channel_server.clone();
                tokio::spawn(async move {
                    let n = conns.fetch_add(1, Ordering::SeqCst) + 1;
                    // Read the client's pipelined commands (setup + SUBSCRIBE).
                    let mut buf = [0u8; 8192];
                    let read = stream.read(&mut buf).await.expect("read");
                    // Reply +OK to every non-subscribe command.
                    let _ = stream.write_all(&resp_mock::ok_replies(&buf[..read])).await;
                    // Confirm the subscription, then push one valid event.
                    // The payload is serialized from the real event type —
                    // exactly what `publish()` puts on the wire.
                    let event = ConfigChangeEvent::new(
                        "reconnect-test",
                        "mock",
                        vec!["key.alpha".to_string()],
                        format!("drop-{n}"),
                    );
                    let _ = stream
                        .write_all(&resp_mock::subscribe_confirmation(&channel))
                        .await;
                    let _ = stream
                        .write_all(&resp_mock::message(
                            &channel,
                            serde_json::to_vec(&event)
                                .expect("serialize event")
                                .as_slice(),
                        ))
                        .await;
                    let _ = stream.flush().await;
                    // Drop the connection: the pubsub stream must end and the
                    // loop must reconnect.
                });
            }
        });

        let bus = RedisBusBuilder::new()
            .url(format!("redis://{addr}"))
            .channel(channel.clone())
            // Fast backoffs so the test stays quick; production defaults are
            // 1s → 30s.
            .reconnect_delay_initial(Duration::from_millis(50))
            .reconnect_delay_max(Duration::from_millis(200))
            .build()
            .await
            .expect("build against mock");

        let mut rx = bus.subscribe().await.expect("subscribe");

        // Event 1 arrives on the first connection.
        let first = timeout(Duration::from_secs(2), rx.next())
            .await
            .expect("timed out waiting for first event")
            .expect("stream ended");
        assert_eq!(first.checksum, "drop-1");

        // The server already dropped connection 1; the loop must reconnect
        // and deliver event 2 — previously the stream ended silently here.
        let second = timeout(Duration::from_secs(3), rx.next())
            .await
            .expect("timed out waiting for post-reconnect event (T029 reconnect failed)")
            .expect("stream ended");
        assert_eq!(second.checksum, "drop-2");
        assert!(
            conns.load(Ordering::SeqCst) >= 2,
            "a second connection must have been accepted"
        );

        server.abort();
    }

    /// the reconnect backoff constants and their builder overrides.
    #[test]
    fn test_reconnect_backoff_configuration() {
        assert_eq!(REDIS_RECONNECT_INITIAL_DELAY, Duration::from_secs(1));
        assert_eq!(REDIS_RECONNECT_MAX_DELAY, Duration::from_secs(30));

        let builder = RedisBusBuilder::new()
            .reconnect_delay_initial(Duration::from_millis(250))
            .reconnect_delay_max(Duration::from_secs(5));
        assert_eq!(builder.reconnect_initial, Duration::from_millis(250));
        assert_eq!(builder.reconnect_max, Duration::from_secs(5));
    }
}
