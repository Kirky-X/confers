//! Redis-based ConfigBus implementation.

use std::pin::Pin;

use async_trait::async_trait;
use futures_util::Stream;
use redis::AsyncCommands;

use super::{ConfigBus, ConfigChangeEvent};
use crate::error::{ConfigError, ConfigResult};

pub struct RedisConfigBus {
    client: redis::Client,
    channel: String,
}

impl RedisConfigBus {
    fn sanitize_url(url: &str) -> String {
        if let Ok(parsed) = url::Url::parse(url) {
            let host = parsed.host_str().unwrap_or("unknown");
            let port = parsed.port().unwrap_or(6379);
            format!("{}:{}", host, port)
        } else {
            "invalid_url".to_string()
        }
    }

    pub async fn connect(url: &str, channel: impl Into<String>) -> ConfigResult<Self> {
        let safe_host = Self::sanitize_url(url);

        let client = redis::Client::open(url).map_err(|e| ConfigError::RemoteUnavailable {
            error_type: format!("redis_connection_failed: host={}, error={}", safe_host, e),
            retryable: true,
        })?;

        Ok(Self {
            client,
            channel: channel.into(),
        })
    }

    pub async fn connect_with_pool(
        url: &str,
        channel: impl Into<String>,
        _pool_size: usize,
    ) -> ConfigResult<Self> {
        Self::connect(url, channel).await
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
        let channel = self.channel.clone();
        let client = self.client.clone();

        let stream = async_stream::stream! {
            loop {
                match get_message(&client, &channel).await {
                    Ok(Some(event)) => yield event,
                    Ok(None) => {
                        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    }
                    Err(_e) => {
                        // Log error silently and continue retrying
                        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    }
                }
            }
        };

        Ok(Box::pin(stream))
    }
}

async fn get_message(
    client: &redis::Client,
    channel: &str,
) -> ConfigResult<Option<ConfigChangeEvent>> {
    let mut conn = client
        .get_multiplexed_async_connection()
        .await
        .map_err(|e| ConfigError::RemoteUnavailable {
            error_type: format!("redis_connection: {}", e),
            retryable: true,
        })?;

    let result: Option<Vec<u8>> = redis::cmd("SUBSCRIBE")
        .arg(channel)
        .query_async(&mut conn)
        .await
        .map_err(|e| ConfigError::RemoteUnavailable {
            error_type: format!("redis_subscribe: {}", e),
            retryable: true,
        })?;

    match result {
        Some(payload) => {
            let event: ConfigChangeEvent =
                serde_json::from_slice(&payload).map_err(|e| ConfigError::SourceChainError {
                    message: format!("deserialize event: {}", e),
                    source_index: 0,
                })?;
            Ok(Some(event))
        }
        None => Ok(None),
    }
}

pub struct RedisBusBuilder {
    url: Option<String>,
    channel: Option<String>,
    pool_size: Option<usize>,
}

impl RedisBusBuilder {
    pub fn new() -> Self {
        Self {
            url: None,
            channel: None,
            pool_size: None,
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

    pub async fn build(self) -> ConfigResult<RedisConfigBus> {
        let url = self.url.ok_or(ConfigError::InvalidValue {
            key: "redis_url".to_string(),
            expected_type: "string".to_string(),
            message: "Redis URL is required".to_string(),
        })?;

        let channel = self.channel.unwrap_or_else(|| "config:events".to_string());

        if let Some(size) = self.pool_size {
            RedisConfigBus::connect_with_pool(&url, channel, size).await
        } else {
            RedisConfigBus::connect(&url, channel).await
        }
    }
}

impl Default for RedisBusBuilder {
    fn default() -> Self {
        Self::new()
    }
}
