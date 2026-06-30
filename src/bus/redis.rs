//! Redis-based ConfigBus implementation.

use std::pin::Pin;

use async_trait::async_trait;
use futures_util::{Stream, StreamExt};
use redis::AsyncCommands;

use super::{ConfigBus, ConfigChangeEvent};
use crate::error::{ConfigConfigError, ConfigError, ConfigResult};
use crate::lifecycle::Lifecycle;

/// Default retry wait time when no message is available (100ms).
const DEFAULT_RETRY_WAIT_MS: u64 = 100;

/// Default error retry wait time (1 second).
const DEFAULT_ERROR_RETRY_WAIT_SECS: u64 = 1;

/// Redis default port.
const DEFAULT_REDIS_PORT: u16 = 6379;

pub struct RedisConfigBus {
    client: redis::Client,
    channel: String,
    /// Retry wait time in milliseconds when no message available.
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

    pub async fn connect_with_pool(
        url: &str,
        channel: impl Into<String>,
        _pool_size: usize,
    ) -> ConfigResult<Self> {
        Self::connect(url, channel).await
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
        })
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
        // Use a dedicated PubSub connection (not multiplexed). Redis' SUBSCRIBE
        // command transitions the connection into subscription mode, which is
        // incompatible with multiplexed-connection query semantics. The previous
        // implementation used `redis::cmd("SUBSCRIBE").query_async()` on a
        // multiplexed connection, which never delivers real published messages.
        let mut pubsub =
            self.client
                .get_async_pubsub()
                .await
                .map_err(|e| ConfigError::RemoteUnavailable {
                    error_type: format!("redis_pubsub: {}", e),
                    retryable: true,
                })?;

        pubsub
            .subscribe(&self.channel)
            .await
            .map_err(|e| ConfigError::RemoteUnavailable {
                error_type: format!("redis_subscribe: {}", e),
                retryable: true,
            })?;

        // on_message(&mut self) borrows pubsub and returns a Stream<Item = Msg>.
        // Wrap the polling loop in async_stream::stream! so pubsub is owned by
        // the stream future itself (avoids E0515: cannot return reference to
        // local). Payload decode errors and JSON deserialization errors are
        // skipped (Rule 12: errors that cannot be meaningfully surfaced to the
        // stream consumer are skipped via continue, not silently swallowed).
        let stream = async_stream::stream! {
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
        };

        Ok(Box::pin(stream))
    }
}

pub struct RedisBusBuilder {
    url: Option<String>,
    channel: Option<String>,
    pool_size: Option<usize>,
    /// Retry wait time in milliseconds when no message available.
    retry_wait_ms: u64,
    /// Error retry wait time in seconds.
    error_retry_wait_secs: u64,
}

impl RedisBusBuilder {
    pub fn new() -> Self {
        Self {
            url: None,
            channel: None,
            pool_size: None,
            retry_wait_ms: DEFAULT_RETRY_WAIT_MS,
            error_retry_wait_secs: DEFAULT_ERROR_RETRY_WAIT_SECS,
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

    pub fn pool_size(mut self, size: usize) -> Self {
        self.pool_size = Some(size);
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

    pub async fn build(self) -> ConfigResult<RedisConfigBus> {
        let url = self.url.ok_or(ConfigError::InvalidValue {
            key: "redis_url".to_string(),
            expected_type: "string".to_string(),
            message: "Redis URL is required".to_string(),
        })?;

        let channel = self.channel.unwrap_or_else(|| "config:events".to_string());

        RedisConfigBus::connect_with_config(
            &url,
            channel,
            self.retry_wait_ms,
            self.error_retry_wait_secs,
        )
        .await
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
    use tokio::time::{timeout, Duration};

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
        assert!(b.pool_size.is_none());
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
            .pool_size(8)
            .retry_wait_ms(42)
            .error_retry_wait_secs(3);
        assert_eq!(b.url.as_deref(), Some("redis://127.0.0.1:16379"));
        assert_eq!(b.channel.as_deref(), Some("unit-chan"));
        assert_eq!(b.pool_size, Some(8));
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
    async fn test_connect_with_pool_invalid_url_is_retryable() {
        let err = RedisConfigBus::connect_with_pool("not-a-valid-url", "chan", 4)
            .await
            .err()
            .expect("connect_with_pool should error");
        assert!(matches!(
            err,
            ConfigError::RemoteUnavailable {
                retryable: true,
                ..
            }
        ));
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
    async fn test_connect_with_pool_live_service() {
        if !redis_ready() {
            eprintln!("Skipping test: Redis not available");
            return;
        }
        let bus = RedisConfigBus::connect_with_pool("redis://127.0.0.1:16379", "p", 4)
            .await
            .expect("connect_with_pool should succeed");
        assert_eq!(bus.channel, "p");
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
}
