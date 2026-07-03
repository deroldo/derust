use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::httpx::{AppContext, HttpError};

/// Configuration for a Kafka cluster connection.
///
/// Use `extra_config` for arbitrary rdkafka settings (SASL, SSL, timeouts, etc.).
/// For handlers that take longer than 5 minutes, set `max.poll.interval.ms` via `extra_config`.
#[derive(Clone)]
pub struct KafkaClusterConfig {
    pub brokers: String,
    pub group_id: String,
    pub extra_config: Vec<(String, String)>,
}

impl KafkaClusterConfig {
    pub fn new(brokers: impl Into<String>, group_id: impl Into<String>) -> Self {
        Self {
            brokers: brokers.into(),
            group_id: group_id.into(),
            extra_config: Vec::new(),
        }
    }

    pub fn with_extra_config(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.extra_config.push((key.into(), value.into()));
        self
    }
}

pub struct Message {
    payload: Option<Vec<u8>>,
    key: Option<Vec<u8>>,
    topic: String,
    partition: i32,
    offset: i64,
}

impl Message {
    pub(crate) fn new(
        payload: Option<Vec<u8>>,
        key: Option<Vec<u8>>,
        topic: String,
        partition: i32,
        offset: i64,
    ) -> Self {
        Self { payload, key, topic, partition, offset }
    }

    pub fn builder() -> MessageBuilder {
        MessageBuilder::default()
    }

    pub fn payload(&self) -> Option<&[u8]> {
        self.payload.as_deref()
    }

    pub fn payload_as_str(&self) -> Option<&str> {
        self.payload.as_deref().and_then(|b| std::str::from_utf8(b).ok())
    }

    pub fn key(&self) -> Option<&[u8]> {
        self.key.as_deref()
    }

    pub fn key_as_str(&self) -> Option<&str> {
        self.key.as_deref().and_then(|b| std::str::from_utf8(b).ok())
    }

    pub fn topic(&self) -> &str {
        &self.topic
    }

    pub fn partition(&self) -> i32 {
        self.partition
    }

    pub fn offset(&self) -> i64 {
        self.offset
    }
}

#[derive(Default)]
pub struct MessageBuilder {
    payload: Option<Vec<u8>>,
    key: Option<Vec<u8>>,
    topic: String,
    partition: i32,
    offset: i64,
}

impl MessageBuilder {
    pub fn payload(mut self, payload: impl Into<Vec<u8>>) -> Self {
        self.payload = Some(payload.into());
        self
    }

    pub fn payload_str(mut self, payload: impl Into<String>) -> Self {
        self.payload = Some(payload.into().into_bytes());
        self
    }

    pub fn key(mut self, key: impl Into<Vec<u8>>) -> Self {
        self.key = Some(key.into());
        self
    }

    pub fn topic(mut self, topic: impl Into<String>) -> Self {
        self.topic = topic.into();
        self
    }

    pub fn partition(mut self, partition: i32) -> Self {
        self.partition = partition;
        self
    }

    pub fn offset(mut self, offset: i64) -> Self {
        self.offset = offset;
        self
    }

    pub fn build(self) -> Message {
        Message::new(self.payload, self.key, self.topic, self.partition, self.offset)
    }
}

type HandlerFn<S> = Arc<
    dyn Fn(AppContext<S>, Message) -> Pin<Box<dyn Future<Output = Result<(), HttpError>> + Send>>
        + Send
        + Sync,
>;

pub struct KafkaTopicConsumer<S>
where
    S: Clone + Send + Sync + 'static,
{
    pub(crate) cluster_config: KafkaClusterConfig,
    pub(crate) topic: String,
    pub(crate) concurrency: usize,
    pub(crate) handler: HandlerFn<S>,
}

impl<S> Clone for KafkaTopicConsumer<S>
where
    S: Clone + Send + Sync + 'static,
{
    fn clone(&self) -> Self {
        Self {
            cluster_config: self.cluster_config.clone(),
            topic: self.topic.clone(),
            concurrency: self.concurrency,
            handler: Arc::clone(&self.handler),
        }
    }
}

impl<S> KafkaTopicConsumer<S>
where
    S: Clone + Send + Sync + 'static,
{
    pub fn new<H, Fut>(cluster_config: KafkaClusterConfig, topic: &str, handler: H) -> Self
    where
        H: Fn(AppContext<S>, Message) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), HttpError>> + Send + 'static,
    {
        Self {
            cluster_config,
            topic: topic.to_string(),
            concurrency: 1,
            handler: Arc::new(move |ctx, msg| Box::pin(handler(ctx, msg))),
        }
    }

    /// Spawns `concurrency` independent StreamConsumer tasks for this topic.
    /// Kafka distributes partitions among them automatically within the same group_id.
    /// Maximum useful value equals the number of partitions on the topic.
    pub fn with_concurrency(mut self, concurrency: usize) -> Self {
        self.concurrency = concurrency;
        self
    }
}
