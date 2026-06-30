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
