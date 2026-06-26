use aws_sdk_sqs::types::Message as SdkMessage;
use std::time::Duration;
use tracing::log::{log_enabled, Level};
use tracing::{error, info};
use wg::WaitGroup;

use super::consumer::{Message, SqsQueueConsumer};
use crate::awsx::{load_aws_config, sqs_client, SqsClient};
use crate::httpx::AppContext;
use crate::shutdown_signal;

#[cfg(any(feature = "statsd", feature = "prometheus"))]
use crate::metricx::{start_stopwatch, MetricTags};

pub async fn run<T>(
    wg: WaitGroup,
    context: AppContext<T>,
    consumers: Vec<SqsQueueConsumer<T>>,
) where
    T: Clone + Send + Sync + 'static,
{
    let aws_config = load_aws_config(*context.env()).await;
    let sqs = sqs_client(&aws_config).await;

    let mut handles = Vec::new();
    for consumer in consumers {
        info!("Started SQS consumer for queue {}", consumer.queue_url);
        let ctx = context.clone();
        let sqs_clone = sqs.clone();
        handles.push(tokio::spawn(poll_queue(ctx, sqs_clone, consumer)));
    }

    for handle in handles {
        let _ = handle.await;
    }

    wg.done();

    info!("SQS consumers stopped");
}

async fn poll_queue<T>(
    context: AppContext<T>,
    sqs: SqsClient,
    consumer: SqsQueueConsumer<T>,
) where
    T: Clone + Send + Sync + 'static,
{
    let mut shutdown = Box::into_pin(Box::new(shutdown_signal()));

    loop {
        let recv = sqs
            .client
            .receive_message()
            .queue_url(&consumer.queue_url)
            .max_number_of_messages(10)
            .wait_time_seconds(20)
            .send();

        tokio::select! {
            result = recv => {
                match result {
                    Ok(output) => {
                        for sdk_msg in output.messages.unwrap_or_default() {
                            process_message(&context, &sqs, &consumer, sdk_msg).await;
                        }
                    }
                    Err(e) => {
                        error!("Failed to receive messages from {}: {:?}", consumer.queue_url, e);
                        tokio::time::sleep(Duration::from_secs(5)).await;
                    }
                }
            }
            _ = &mut shutdown => {
                info!("SQS consumer for queue {} stopped", consumer.queue_url);
                break;
            }
        }
    }
}

async fn process_message<T>(
    context: &AppContext<T>,
    sqs: &SqsClient,
    consumer: &SqsQueueConsumer<T>,
    sdk_msg: SdkMessage,
) where
    T: Clone + Send + Sync + 'static,
{
    let msg = Message::new(sdk_msg);
    let message_id = msg.message_id().unwrap_or("unknown").to_string();
    let body = msg.body().unwrap_or("").to_string();
    let receipt_handle = msg.receipt_handle().unwrap_or_default().to_string();

    #[cfg(any(feature = "statsd", feature = "prometheus"))]
    let metric_tags = MetricTags::default().push("queue_url".to_string(), consumer.queue_url.clone());

    #[cfg(any(feature = "statsd", feature = "prometheus"))]
    let stopwatch = start_stopwatch(context, "sqs_consumer_seconds", metric_tags);

    let result = (consumer.handler)(context.clone(), msg).await;

    match &result {
        Ok(_) => {
            if context.env().is_local() || log_enabled!(Level::Debug) {
                info!(
                    "SQS message consumed :: queue={} :: message_id={} :: body={}",
                    consumer.queue_url, message_id, body
                );
            } else {
                info!(
                    "SQS message consumed :: queue={} :: message_id={}",
                    consumer.queue_url, message_id
                );
            }

            let _ = sqs
                .client
                .delete_message()
                .queue_url(&consumer.queue_url)
                .receipt_handle(&receipt_handle)
                .send()
                .await;
        }
        Err(err) => {
            if context.env().is_local() || log_enabled!(Level::Debug) {
                error!(
                    "SQS message failed :: queue={} :: message_id={} :: body={} :: error={:?}",
                    consumer.queue_url, message_id, body, err
                );
            } else {
                error!(
                    "SQS message failed :: queue={} :: message_id={} :: error={:?}",
                    consumer.queue_url, message_id, err
                );
            }
        }
    }

    #[cfg(any(feature = "statsd", feature = "prometheus"))]
    {
        let success = if result.is_ok() { "true" } else { "false" };
        stopwatch.record(MetricTags::default().push("success".to_string(), success.to_string()));
    }
}
