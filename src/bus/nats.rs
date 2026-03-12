//! NATS-based ConfigBus implementation.

use std::pin::Pin;

use async_nats::jetstream::{self, consumer::DeliverPolicy};
use async_trait::async_trait;
use futures_util::{Stream, StreamExt};

use super::{ConfigBus, ConfigChangeEvent};
use crate::error::{ConfigError, ConfigResult};

pub struct NatsConfigBus {
    client: async_nats::Client,
    subject: String,
    stream_name: String,
}

impl NatsConfigBus {
    pub async fn connect(url: &str, subject: impl Into<String>) -> ConfigResult<Self> {
        let client = async_nats::connect(url)
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
                Err(e) => {
                    tracing::warn!("NATS message error: {}", e);
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
