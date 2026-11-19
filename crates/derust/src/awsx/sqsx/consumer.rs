use aws_sdk_sqs::types::builders::MessageBuilder as SdkMessageBuilder;
use aws_sdk_sqs::types::Message as SdkMessage;
use aws_sdk_sqs::types::MessageAttributeValue;
use aws_sdk_sqs::types::MessageSystemAttributeName;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

#[cfg(feature = "postgres")]
use super::idempotency::{reserve_sql, IdempotentTx, PostgresIdempotencyConfig};
use super::idempotency::{
    CleanupConfig, CleanupTarget, IdempotencyConfig, IdempotencyConfigError, IdempotencyStore,
};
use super::retry::RetryPolicy;
use crate::commonx::{DerustError, DerustTags};
use crate::httpx::AppContext;

pub struct Message(SdkMessage);

impl Message {
    pub(crate) fn new(msg: SdkMessage) -> Self {
        Self(msg)
    }

    pub fn builder() -> MessageBuilder {
        MessageBuilder(SdkMessage::builder())
    }

    pub fn body(&self) -> Option<&str> {
        self.0.body()
    }

    pub fn message_id(&self) -> Option<&str> {
        self.0.message_id()
    }

    /// Current attempt number (1 = first delivery). Reads the system attribute
    /// `ApproximateReceiveCount`; missing, non-numeric or 0 means 1.
    pub fn receive_count(&self) -> u32 {
        self.0
            .attributes()
            .and_then(|a| a.get(&MessageSystemAttributeName::ApproximateReceiveCount))
            .and_then(|v| v.parse::<u32>().ok())
            .filter(|n| *n >= 1)
            .unwrap_or(1)
    }

    /// String value of a message attribute, if present.
    pub fn message_attribute(&self, name: &str) -> Option<&str> {
        self.0
            .message_attributes()
            .and_then(|a| a.get(name))
            .and_then(|v| v.string_value())
    }

    #[cfg(test)]
    pub(crate) fn into_sdk(self) -> SdkMessage {
        self.0
    }

    pub(crate) fn receipt_handle(&self) -> Option<&str> {
        self.0.receipt_handle()
    }
}

pub struct MessageBuilder(SdkMessageBuilder);

impl MessageBuilder {
    pub fn body(self, body: impl Into<String>) -> Self {
        Self(self.0.body(body))
    }

    pub fn message_id(self, id: impl Into<String>) -> Self {
        Self(self.0.message_id(id))
    }

    pub fn receipt_handle(self, rh: impl Into<String>) -> Self {
        Self(self.0.receipt_handle(rh))
    }

    /// Sets `ApproximateReceiveCount`; useful to test handlers.
    pub fn receive_count(self, count: u32) -> Self {
        Self(self.0.attributes(
            MessageSystemAttributeName::ApproximateReceiveCount,
            count.to_string(),
        ))
    }

    /// Adds a String message attribute; useful to test handlers.
    pub fn message_attribute(self, name: impl Into<String>, value: impl Into<String>) -> Self {
        match MessageAttributeValue::builder()
            .data_type("String")
            .string_value(value)
            .build()
        {
            Ok(v) => Self(self.0.message_attributes(name, v)),
            Err(_) => self,
        }
    }

    pub fn build(self) -> Message {
        Message::new(self.0.build())
    }
}

type HandlerFn<S> = Arc<
    dyn Fn(
            AppContext<S>,
            Message,
            DerustTags,
        ) -> Pin<Box<dyn Future<Output = Result<(), DerustError>> + Send>>
        + Send
        + Sync,
>;

/// Handler of a transactional consumer: receives the transaction by value and must return it
/// in `Ok` so the consumer can commit it.
#[cfg(feature = "postgres")]
pub(crate) type TxHandlerFn<S> = Arc<
    dyn Fn(
            AppContext<S>,
            Message,
            DerustTags,
            IdempotentTx<S>,
        ) -> Pin<Box<dyn Future<Output = Result<IdempotentTx<S>, DerustError>> + Send>>
        + Send
        + Sync,
>;

/// Idempotency mode of a consumer.
pub(crate) enum IdempotencyRuntime {
    NonTransactional {
        store: Arc<dyn IdempotencyStore>,
        config: IdempotencyConfig,
    },
    /// The matching handler lives in `SqsQueueConsumer::tx_handler`.
    #[cfg(feature = "postgres")]
    Transactional {
        config: PostgresIdempotencyConfig,
        reserve_sql: String,
    },
}

impl IdempotencyRuntime {
    pub(crate) fn key_attribute(&self) -> &str {
        match self {
            Self::NonTransactional { config, .. } => &config.key_attribute,
            #[cfg(feature = "postgres")]
            Self::Transactional { config, .. } => &config.key_attribute,
        }
    }

    /// Background cleanup settings, when enabled.
    pub(crate) fn cleanup_config(&self) -> Option<&CleanupConfig> {
        match self {
            Self::NonTransactional { config, .. } => config.cleanup.as_ref(),
            #[cfg(feature = "postgres")]
            Self::Transactional { config, .. } => config.cleanup.as_ref(),
        }
    }

    /// Where the records to clean up live.
    pub(crate) fn cleanup_target(&self) -> CleanupTarget {
        match self {
            Self::NonTransactional { store, .. } => CleanupTarget::Store(store.clone()),
            #[cfg(feature = "postgres")]
            Self::Transactional { config, .. } => CleanupTarget::Table(config.table.clone()),
        }
    }

    fn is_transactional(&self) -> bool {
        match self {
            Self::NonTransactional { .. } => false,
            #[cfg(feature = "postgres")]
            Self::Transactional { .. } => true,
        }
    }
}

pub struct SqsQueueConsumer<S>
where
    S: Clone + Send + Sync + 'static,
{
    pub(crate) queue_url: String,
    pub(crate) handler: HandlerFn<S>,
    pub(crate) retry_policy: Option<RetryPolicy>,
    pub(crate) idempotency: Option<IdempotencyRuntime>,
    /// Set only together with `IdempotencyRuntime::Transactional`.
    #[cfg(feature = "postgres")]
    pub(crate) tx_handler: Option<TxHandlerFn<S>>,
}

impl<S> SqsQueueConsumer<S>
where
    S: Clone + Send + Sync + 'static,
{
    pub fn new<H, Fut>(queue_url: &str, handler: H) -> Self
    where
        H: Fn(AppContext<S>, Message, DerustTags) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), DerustError>> + Send + 'static,
    {
        Self {
            queue_url: queue_url.to_string(),
            handler: Arc::new(move |ctx, msg, tags| Box::pin(handler(ctx, msg, tags))),
            retry_policy: None,
            idempotency: None,
            #[cfg(feature = "postgres")]
            tx_handler: None,
        }
    }

    /// Transactional idempotent consumer (Postgres): the idempotency key is written in the same
    /// database transaction as the effects of the handler, so for the same key the effects
    /// happen at most once even with duplicates or concurrent deliveries.
    ///
    /// The consumer opens the transaction (always on the `read_write` pool of
    /// `context.database()`, with operation name `sqs_idempotent_consumer` in metrics),
    /// reserves the key `(queue_url, key)` in `config.table` and then calls the handler,
    /// which receives the transaction **by value**:
    ///
    /// ```ignore
    /// SqsQueueConsumer::new_transactional(url, PostgresIdempotencyConfig::default(),
    ///     |ctx, msg, tags, mut tx| async move {
    ///         sqlx::query("insert into payments (id) values ($1)")
    ///             .bind(msg.body().unwrap_or_default())
    ///             .execute(&mut *tx.transaction)
    ///             .await
    ///             .map_err(|e| DerustError::without_body(
    ///                 StatusCode::INTERNAL_SERVER_ERROR, e.to_string(), tags))?;
    ///         Ok(tx) // hand the transaction back: the consumer commits it
    ///     })?
    /// ```
    ///
    /// Rules for the handler:
    /// - **Never commit** the transaction. Return `Ok(tx)` and the consumer commits the key and
    ///   the effects together, then deletes the SQS message. Returning `Err` (or panicking)
    ///   rolls everything back, the message is not deleted and the usual failure behavior
    ///   (including the retry backoff, when configured) applies.
    /// - The transaction stays open, holding a pool connection, for the whole handler. Do any
    ///   HTTP call (or other slow I/O) **before** using the transaction and keep the handler
    ///   short; the consumer itself makes no SQS call while the transaction is open.
    /// - Run the database in `READ COMMITTED` (the PostgreSQL default). A concurrent delivery of
    ///   the same key waits for the first transaction (up to `config.lock_timeout`): if it
    ///   commits, the second one is discarded as a duplicate; if it rolls back, the second one
    ///   runs the handler. When the wait times out the delivery is deferred (message kept, no
    ///   backoff) and SQS redelivers it.
    /// - If the commit fails the message is not deleted (fail-closed) and SQS redelivers it.
    ///
    /// Fails when the config is invalid. The table is not created at runtime: apply
    /// `PostgresIdempotencyStore::migration_sql`.
    #[cfg(feature = "postgres")]
    pub fn new_transactional<H, Fut>(
        queue_url: &str,
        config: PostgresIdempotencyConfig,
        handler: H,
    ) -> Result<Self, IdempotencyConfigError>
    where
        H: Fn(AppContext<S>, Message, DerustTags, IdempotentTx<S>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<IdempotentTx<S>, DerustError>> + Send + 'static,
    {
        config.validate()?;
        let reserve_sql = reserve_sql(&config.table)?;
        // Never called: a transactional consumer runs `tx_handler`.
        let unused: HandlerFn<S> = Arc::new(|_ctx, _msg, tags| {
            Box::pin(async move {
                Err(DerustError::without_body(
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    "transactional consumer has no plain handler".to_string(),
                    tags,
                ))
            })
        });
        Ok(Self {
            queue_url: queue_url.to_string(),
            handler: unused,
            retry_policy: None,
            idempotency: Some(IdempotencyRuntime::Transactional {
                config,
                reserve_sql,
            }),
            tx_handler: Some(Arc::new(move |ctx, msg, tags, tx| {
                Box::pin(handler(ctx, msg, tags, tx))
            })),
        })
    }

    /// Opt-in idempotency (non-transactional, any store backend): for the same key the
    /// handler succeeds at most once, even with duplicates or concurrent deliveries.
    /// Fails when the config is invalid or the consumer is already transactional.
    pub fn with_idempotency(
        mut self,
        store: Arc<dyn IdempotencyStore>,
        config: IdempotencyConfig,
    ) -> Result<Self, IdempotencyConfigError> {
        if self
            .idempotency
            .as_ref()
            .is_some_and(IdempotencyRuntime::is_transactional)
        {
            return Err(IdempotencyConfigError::AlreadyTransactional);
        }
        config.validate()?;
        self.idempotency = Some(IdempotencyRuntime::NonTransactional { store, config });
        Ok(self)
    }

    /// Opt-in: when the handler fails, the message is made visible again after an
    /// exponential backoff with full jitter instead of the queue's fixed visibility timeout.
    pub fn with_retry_policy(mut self, policy: RetryPolicy) -> Self {
        self.retry_policy = Some(policy);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_count(v: &str) -> Message {
        Message::new(
            SdkMessage::builder()
                .attributes(MessageSystemAttributeName::ApproximateReceiveCount, v)
                .build(),
        )
    }

    #[test]
    fn receive_count_defaults_to_one_when_attribute_missing() {
        assert_eq!(Message::builder().body("x").build().receive_count(), 1);
    }

    #[test]
    fn receive_count_reads_builder_value() {
        assert_eq!(
            Message::builder().receive_count(4).build().receive_count(),
            4
        );
    }

    #[test]
    fn receive_count_invalid_or_zero_is_one() {
        for v in ["abc", "0", "", "-3"] {
            assert_eq!(with_count(v).receive_count(), 1, "value {v:?}");
        }
    }

    #[test]
    fn receive_count_large_value() {
        assert_eq!(with_count("4294967295").receive_count(), u32::MAX);
        assert_eq!(with_count("99999999999").receive_count(), 1);
    }

    #[test]
    fn message_attribute_reads_values() {
        let m = Message::builder().message_attribute("a", "1").build();
        assert_eq!(m.message_attribute("a"), Some("1"));
        assert_eq!(m.message_attribute("b"), None);
        assert_eq!(Message::builder().build().message_attribute("a"), None);
        let no_string = SdkMessage::builder()
            .message_attributes(
                "a",
                MessageAttributeValue::builder()
                    .data_type("Binary")
                    .build()
                    .unwrap(),
            )
            .build();
        assert_eq!(Message::new(no_string).message_attribute("a"), None);
    }

    fn consumer() -> SqsQueueConsumer<()> {
        SqsQueueConsumer::<()>::new("q", |_c, _m, _t| async { Ok(()) })
    }

    #[test]
    fn new_has_no_retry_policy() {
        assert!(consumer().retry_policy.is_none());
    }

    #[test]
    fn with_retry_policy_sets_policy() {
        let c = consumer().with_retry_policy(RetryPolicy::default());
        assert_eq!(c.retry_policy, Some(RetryPolicy::default()));
    }

    #[test]
    fn new_has_no_idempotency() {
        assert!(consumer().idempotency.is_none());
    }

    #[test]
    fn with_idempotency_validates_config() {
        use super::super::idempotency::InMemoryIdempotencyStore;
        let store: Arc<dyn IdempotencyStore> = Arc::new(InMemoryIdempotencyStore::new());
        let bad = IdempotencyConfig::default().with_completed_ttl(std::time::Duration::ZERO);
        assert!(matches!(
            consumer().with_idempotency(store.clone(), bad),
            Err(IdempotencyConfigError::InvalidTtl)
        ));
        let ok = consumer()
            .with_idempotency(store, IdempotencyConfig::default())
            .unwrap();
        assert!(ok.idempotency.is_some());
    }

    #[cfg(feature = "postgres")]
    fn transactional(
        config: PostgresIdempotencyConfig,
    ) -> Result<SqsQueueConsumer<()>, IdempotencyConfigError> {
        SqsQueueConsumer::<()>::new_transactional(
            "q",
            config,
            |_c, _m, _t, tx| async move { Ok(tx) },
        )
    }

    #[cfg(feature = "postgres")]
    #[test]
    fn new_transactional_accepts_async_move_handler_and_validates_config() {
        let c = transactional(PostgresIdempotencyConfig::default()).unwrap();
        assert_eq!(c.queue_url, "q");
        assert!(c.tx_handler.is_some());
        assert!(c
            .idempotency
            .as_ref()
            .is_some_and(IdempotencyRuntime::is_transactional));
        assert_eq!(
            c.idempotency.as_ref().unwrap().key_attribute(),
            "x-idempotent-key"
        );
        assert!(matches!(
            transactional(PostgresIdempotencyConfig::default().with_table("x; drop table y")),
            Err(IdempotencyConfigError::InvalidTableName)
        ));
        assert!(matches!(
            transactional(
                PostgresIdempotencyConfig::default().with_lock_timeout(std::time::Duration::ZERO)
            ),
            Err(IdempotencyConfigError::InvalidTtl)
        ));
    }

    #[cfg(feature = "postgres")]
    #[test]
    fn with_idempotency_on_transactional_consumer_is_rejected() {
        use super::super::idempotency::InMemoryIdempotencyStore;
        let store: Arc<dyn IdempotencyStore> = Arc::new(InMemoryIdempotencyStore::new());
        let c = transactional(PostgresIdempotencyConfig::default()).unwrap();
        assert!(matches!(
            c.with_idempotency(store, IdempotencyConfig::default()),
            Err(IdempotencyConfigError::AlreadyTransactional)
        ));
    }

    #[test]
    fn with_retry_policy_keeps_queue_url_and_handler() {
        let c = consumer().with_retry_policy(RetryPolicy::default());
        assert_eq!(c.queue_url, "q");
    }
}
