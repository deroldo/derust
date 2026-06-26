use aws_sdk_sqs::types::Message as SdkMessage;
use aws_sdk_sqs::types::builders::MessageBuilder as SdkMessageBuilder;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::httpx::{AppContext, HttpError};

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

    pub fn build(self) -> Message {
        Message::new(self.0.build())
    }
}

type HandlerFn<S> = Arc<
    dyn Fn(AppContext<S>, Message) -> Pin<Box<dyn Future<Output = Result<(), HttpError>> + Send>>
        + Send
        + Sync,
>;

pub struct SqsQueueConsumer<S>
where
    S: Clone + Send + Sync + 'static,
{
    pub(crate) queue_url: String,
    pub(crate) handler: HandlerFn<S>,
}

impl<S> SqsQueueConsumer<S>
where
    S: Clone + Send + Sync + 'static,
{
    pub fn new<H, Fut>(queue_url: &str, handler: H) -> Self
    where
        H: Fn(AppContext<S>, Message) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), HttpError>> + Send + 'static,
    {
        Self {
            queue_url: queue_url.to_string(),
            handler: Arc::new(move |ctx, msg| Box::pin(handler(ctx, msg))),
        }
    }
}
