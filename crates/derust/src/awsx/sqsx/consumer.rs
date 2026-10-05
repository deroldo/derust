use aws_sdk_sqs::types::builders::MessageBuilder as SdkMessageBuilder;
use aws_sdk_sqs::types::Message as SdkMessage;
use aws_sdk_sqs::types::MessageAttributeValue;
use aws_sdk_sqs::types::MessageSystemAttributeName;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

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

pub struct SqsQueueConsumer<S>
where
    S: Clone + Send + Sync + 'static,
{
    pub(crate) queue_url: String,
    pub(crate) handler: HandlerFn<S>,
    pub(crate) retry_policy: Option<RetryPolicy>,
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
        }
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
    fn with_retry_policy_keeps_queue_url_and_handler() {
        let c = consumer().with_retry_policy(RetryPolicy::default());
        assert_eq!(c.queue_url, "q");
    }
}
