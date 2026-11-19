use aws_sdk_sqs::types::{Message as SdkMessage, MessageSystemAttributeName};
use std::sync::Arc;
use std::time::Duration;
use tracing::log::{log_enabled, Level};
use tracing::{error, info, warn};
use wg::WaitGroup;

use super::consumer::{IdempotencyRuntime, Message, SqsQueueConsumer};
#[cfg(feature = "postgres")]
use super::idempotency::PostgresIdempotencyStore;
use super::idempotency::{cleanup_plan, run_cleanup_loop, CleanupTarget};
#[cfg(feature = "postgres")]
use super::idempotency::{reserve, Reservation, ReserveError};
use super::idempotency::{resolve_key, Acquire, IdempotencyConfig, IdempotencyStore, KeySource};
#[cfg(feature = "postgres")]
use super::idempotency::{IdempotentTx, PostgresIdempotencyConfig};
use super::retry::{to_visibility_timeout_secs, RetryPolicy};
use super::trace::{self, TRACE_ID_ATTRIBUTE};
use crate::awsx::{load_aws_config, sqs_client, SqsClient};
use crate::commonx::{DerustError, DerustTags};
use crate::httpx::AppContext;
use crate::shutdown_signal;

#[cfg(any(feature = "statsd", feature = "prometheus"))]
use crate::metricx::{start_stopwatch, MetricTags};

pub async fn run<T>(wg: WaitGroup, context: AppContext<T>, consumers: Vec<SqsQueueConsumer<T>>)
where
    T: Clone + Send + Sync + 'static,
{
    let aws_config = load_aws_config(*context.env()).await;
    let sqs = sqs_client(&aws_config).await;

    let mut handles = Vec::new();

    // One cleanup task per consumer with cleanup configured (none otherwise).
    for plan in cleanup_plan(&consumers) {
        let Some(store) = cleanup_store(&context, plan.target) else {
            error!(
                "Idempotency cleanup skipped for queue {}: could not build the store",
                plan.queue_url
            );
            continue;
        };
        info!("Started idempotency cleanup for queue {}", plan.queue_url);
        handles.push(tokio::spawn(run_cleanup_loop(
            store,
            plan.config,
            plan.queue_url,
            shutdown_signal(),
        )));
    }

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

/// Store used by the cleanup task. Transactional consumers get a `PostgresIdempotencyStore` on
/// the application database and the table of their config (used only for `cleanup_expired`).
fn cleanup_store<T>(
    context: &AppContext<T>,
    target: CleanupTarget,
) -> Option<Arc<dyn IdempotencyStore>>
where
    T: Clone + Send + Sync + 'static,
{
    #[cfg(not(feature = "postgres"))]
    let _ = context;
    match target {
        CleanupTarget::Store(store) => Some(store),
        #[cfg(feature = "postgres")]
        CleanupTarget::Table(table) => {
            match PostgresIdempotencyStore::new(context.database(), &table) {
                Ok(store) => Some(Arc::new(store)),
                Err(err) => {
                    error!("Invalid idempotency table {table}: {err}");
                    None
                }
            }
        }
    }
}

async fn poll_queue<T>(context: AppContext<T>, sqs: SqsClient, consumer: SqsQueueConsumer<T>)
where
    T: Clone + Send + Sync + 'static,
{
    let mut shutdown = Box::into_pin(Box::new(shutdown_signal()));

    loop {
        let recv = sqs
            .client
            .receive_message()
            .queue_url(&consumer.queue_url)
            .max_number_of_messages(10)
            .message_system_attribute_names(MessageSystemAttributeName::ApproximateReceiveCount)
            .set_message_attribute_names(Some(message_attribute_names(&consumer)))
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

/// Message attributes requested from SQS. The idempotency key attribute is only requested
/// when idempotency is configured (otherwise the request is the same as before).
fn message_attribute_names<T>(consumer: &SqsQueueConsumer<T>) -> Vec<String>
where
    T: Clone + Send + Sync + 'static,
{
    let mut names = vec![TRACE_ID_ATTRIBUTE.to_string()];
    if let Some(rt) = &consumer.idempotency {
        let key_attribute = rt.key_attribute();
        if key_attribute != TRACE_ID_ATTRIBUTE {
            names.push(key_attribute.to_string());
        }
    }
    names
}

/// Result of processing a message with idempotency enabled (M10).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IdempotencyOutcome {
    Processed,
    DuplicateDiscarded,
    DuplicateDeferred,
    StoreError,
    HandlerFailed,
}

impl IdempotencyOutcome {
    #[cfg_attr(
        not(any(feature = "statsd", feature = "prometheus", test)),
        allow(dead_code)
    )]
    fn is_success(self) -> bool {
        !matches!(self, Self::StoreError | Self::HandlerFailed)
    }
}

/// Extra metric tags added to `sqs_consumer_seconds` only when idempotency is active.
#[cfg_attr(
    not(any(feature = "statsd", feature = "prometheus", test)),
    allow(dead_code)
)]
fn idempotency_metric_tags(outcome: IdempotencyOutcome) -> Vec<(String, String)> {
    let tag = |k: &str, v: &str| vec![(k.to_string(), v.to_string())];
    match outcome {
        IdempotencyOutcome::DuplicateDiscarded => tag("duplicate", "true"),
        IdempotencyOutcome::DuplicateDeferred => tag("duplicate", "in_progress"),
        IdempotencyOutcome::StoreError => tag("idempotency", "store_error"),
        IdempotencyOutcome::Processed | IdempotencyOutcome::HandlerFailed => Vec::new(),
    }
}

fn key_source_str(source: Option<KeySource>) -> &'static str {
    match source {
        Some(KeySource::Attribute) => "attribute",
        Some(KeySource::MessageIdFallback) => "message_id_fallback",
        None => "none",
    }
}

/// Per-delivery data shared by the idempotent flow.
struct Delivery<'a> {
    queue_url: &'a str,
    message_id: &'a str,
    body: &'a str,
    receipt_handle: &'a str,
    receive_count: u32,
    trace_id: &'a str,
}

/// Converts a failed handler task (panic or cancellation) into a handler error.
fn handler_join_error(join_error: tokio::task::JoinError) -> DerustError {
    let what = if join_error.is_panic() {
        "panicked"
    } else {
        "was cancelled"
    };
    DerustError::without_body(
        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        format!("SQS handler {what}"),
        DerustTags::default(),
    )
}

/// Resolves the idempotency key, logging the `message_id` fallback and the missing-key
/// failure. `None` means fail closed.
fn resolve_key_logged(
    msg: &Message,
    key_attribute: &str,
    d: &Delivery<'_>,
) -> Option<(String, &'static str, KeySource)> {
    let Some(resolved) = resolve_key(msg, key_attribute) else {
        error!(
            "SQS idempotency key missing, message not processed :: queue={} :: message_id={} :: trace_id={} :: idempotency_key=none :: key_source=none :: outcome=store_error",
            d.queue_url, d.message_id, d.trace_id
        );
        return None;
    };
    let source = key_source_str(Some(resolved.source));
    if resolved.source == KeySource::MessageIdFallback {
        info!(
            "idempotency key fallback to message_id :: queue={} :: message_id={} :: trace_id={} :: idempotency_key={} :: key_source={}",
            d.queue_url, d.message_id, d.trace_id, resolved.key, source
        );
    }
    Some((resolved.key, source, resolved.source))
}

fn body_log_field<T>(context: &AppContext<T>, d: &Delivery<'_>) -> String
where
    T: Clone + Send + Sync + 'static,
{
    if context.env().is_local() || log_enabled!(Level::Debug) {
        format!(" :: body={}", d.body)
    } else {
        String::new()
    }
}

/// Runs the handler in its own task so that a panic is reported as a failure.
async fn run_handler_guarded<T>(
    consumer: &SqsQueueConsumer<T>,
    context: &AppContext<T>,
    msg: Message,
    tags: DerustTags,
) -> Result<(), DerustError>
where
    T: Clone + Send + Sync + 'static,
{
    let handler = consumer.handler.clone();
    let ctx = context.clone();
    match tokio::spawn(async move { handler(ctx, msg, tags).await }).await {
        Ok(result) => result,
        Err(join_error) => Err(handler_join_error(join_error)),
    }
}

async fn delete_message_best_effort(sqs: &SqsClient, queue_url: &str, receipt_handle: &str) {
    let _ = sqs
        .client
        .delete_message()
        .queue_url(queue_url)
        .receipt_handle(receipt_handle)
        .send()
        .await;
}

#[allow(clippy::too_many_arguments)]
async fn process_idempotent<T>(
    context: &AppContext<T>,
    sqs: &SqsClient,
    consumer: &SqsQueueConsumer<T>,
    store: &Arc<dyn IdempotencyStore>,
    config: &IdempotencyConfig,
    msg: Message,
    tags: DerustTags,
    d: &Delivery<'_>,
) -> IdempotencyOutcome
where
    T: Clone + Send + Sync + 'static,
{
    let body_field = body_log_field(context, d);

    let Some((key, source, _)) = resolve_key_logged(&msg, &config.key_attribute, d) else {
        return IdempotencyOutcome::StoreError;
    };
    let key = key.as_str();

    match store
        .try_acquire(d.queue_url, key, config.in_progress_ttl)
        .await
    {
        Err(e) => {
            error!(
                "SQS idempotency store failed, message not processed :: queue={} :: message_id={} :: trace_id={} :: idempotency_key={} :: key_source={} :: outcome=store_error :: error={}",
                d.queue_url, d.message_id, d.trace_id, key, source, e
            );
            IdempotencyOutcome::StoreError
        }
        Ok(Acquire::AlreadyCompleted) => {
            info!(
                "SQS duplicate discarded :: queue={} :: message_id={} :: trace_id={} :: idempotency_key={} :: key_source={} :: outcome=duplicate_discarded",
                d.queue_url, d.message_id, d.trace_id, key, source
            );
            delete_message_best_effort(sqs, d.queue_url, d.receipt_handle).await;
            IdempotencyOutcome::DuplicateDiscarded
        }
        Ok(Acquire::InProgress) => {
            warn!(
                "SQS duplicate deferred :: queue={} :: message_id={} :: trace_id={} :: idempotency_key={} :: key_source={} :: outcome=duplicate_deferred",
                d.queue_url, d.message_id, d.trace_id, key, source
            );
            IdempotencyOutcome::DuplicateDeferred
        }
        Ok(Acquire::Acquired) => match run_handler_guarded(consumer, context, msg, tags).await {
            Ok(()) => {
                if let Err(e) = store.complete(d.queue_url, key, config.completed_ttl).await {
                    error!(
                            "Failed to mark idempotency key as completed :: queue={} :: message_id={} :: trace_id={} :: idempotency_key={} :: key_source={} :: error={}",
                            d.queue_url, d.message_id, d.trace_id, key, source, e
                        );
                }
                info!(
                        "SQS message consumed :: queue={} :: message_id={} :: trace_id={} :: idempotency_key={} :: key_source={} :: outcome=processed{}",
                        d.queue_url, d.message_id, d.trace_id, key, source, body_field
                    );
                delete_message_best_effort(sqs, d.queue_url, d.receipt_handle).await;
                IdempotencyOutcome::Processed
            }
            Err(err) => {
                if let Err(e) = store.release(d.queue_url, key).await {
                    error!(
                            "Failed to release idempotency key :: queue={} :: message_id={} :: trace_id={} :: idempotency_key={} :: key_source={} :: error={}",
                            d.queue_url, d.message_id, d.trace_id, key, source, e
                        );
                }
                error!(
                        "SQS message failed :: queue={} :: message_id={} :: trace_id={} :: idempotency_key={} :: key_source={} :: outcome=handler_failed{} :: error={:?}",
                        d.queue_url, d.message_id, d.trace_id, key, source, body_field, err
                    );
                if let Some(delay_secs) =
                    retry_delay_secs(consumer.retry_policy.as_ref(), d.receive_count)
                {
                    apply_retry_backoff(
                        sqs,
                        d.queue_url,
                        d.receipt_handle,
                        d.message_id,
                        d.receive_count,
                        delay_secs,
                        d.trace_id,
                    )
                    .await;
                }
                IdempotencyOutcome::HandlerFailed
            }
        },
    }
}

/// Transactional flow (M7): the key is reserved inside the handler's transaction, which the
/// consumer commits. Every path other than the commit ends in a rollback, and the SQS calls
/// (delete / change visibility) only happen after the transaction is closed.
#[cfg(feature = "postgres")]
#[allow(clippy::too_many_arguments)]
async fn process_transactional<T>(
    context: &AppContext<T>,
    sqs: &SqsClient,
    consumer: &SqsQueueConsumer<T>,
    config: &PostgresIdempotencyConfig,
    reserve_sql: &str,
    msg: Message,
    tags: DerustTags,
    d: &Delivery<'_>,
) -> IdempotencyOutcome
where
    T: Clone + Send + Sync + 'static,
{
    let Some(tx_handler) = consumer.tx_handler.clone() else {
        error!(
            "SQS transactional consumer without handler :: queue={} :: message_id={} :: trace_id={} :: outcome=store_error",
            d.queue_url, d.message_id, d.trace_id
        );
        return IdempotencyOutcome::StoreError;
    };
    let body_field = body_log_field(context, d);

    let Some((key, source, _)) = resolve_key_logged(&msg, &config.key_attribute, d) else {
        return IdempotencyOutcome::StoreError;
    };
    let key = key.as_str();

    #[cfg(any(feature = "statsd", feature = "prometheus"))]
    let begun = context
        .database()
        .begin_transaction(context, "sqs_idempotent_consumer", &tags)
        .await;
    #[cfg(not(any(feature = "statsd", feature = "prometheus")))]
    let begun = context.database().begin_transaction(context, &tags).await;

    let mut tx: IdempotentTx<T> = match begun {
        Ok(tx) => tx,
        Err(e) => {
            error!(
                "SQS idempotency transaction failed to begin, message not processed :: queue={} :: message_id={} :: trace_id={} :: idempotency_key={} :: key_source={} :: outcome=store_error :: error={:?}",
                d.queue_url, d.message_id, d.trace_id, key, source, e
            );
            return IdempotencyOutcome::StoreError;
        }
    };

    let reservation = reserve(
        &mut tx.transaction,
        reserve_sql,
        d.queue_url,
        key,
        config.completed_ttl,
        config.lock_timeout_millis(),
    )
    .await;

    match reservation {
        Ok(Reservation::Fresh) => {}
        Ok(Reservation::Duplicate) => {
            rollback_logged::<T>(tx, d, key).await;
            info!(
                "SQS duplicate discarded :: queue={} :: message_id={} :: trace_id={} :: idempotency_key={} :: key_source={} :: outcome=duplicate_discarded",
                d.queue_url, d.message_id, d.trace_id, key, source
            );
            delete_message_best_effort(sqs, d.queue_url, d.receipt_handle).await;
            return IdempotencyOutcome::DuplicateDiscarded;
        }
        Err(ReserveError::LockNotAvailable) => {
            rollback_logged::<T>(tx, d, key).await;
            warn!(
                "SQS duplicate deferred :: queue={} :: message_id={} :: trace_id={} :: idempotency_key={} :: key_source={} :: outcome=duplicate_deferred",
                d.queue_url, d.message_id, d.trace_id, key, source
            );
            return IdempotencyOutcome::DuplicateDeferred;
        }
        Err(ReserveError::Store(e)) => {
            rollback_logged::<T>(tx, d, key).await;
            error!(
                "SQS idempotency store failed, message not processed :: queue={} :: message_id={} :: trace_id={} :: idempotency_key={} :: key_source={} :: outcome=store_error :: error={}",
                d.queue_url, d.message_id, d.trace_id, key, source, e
            );
            return IdempotencyOutcome::StoreError;
        }
    }

    // The handler owns the transaction inside its own task: on `Err` or panic the transaction
    // is dropped there (rollback) and the connection goes back to the pool.
    let ctx = context.clone();
    let handler_tags = tags.clone();
    let handled = tokio::spawn(async move { tx_handler(ctx, msg, handler_tags, tx).await }).await;
    let result = match handled {
        Ok(result) => result,
        Err(join_error) => Err(handler_join_error(join_error)),
    };

    match result {
        Ok(tx) => {
            if let Err(e) = tx.commit_transaction(&tags).await {
                // Fail-closed: the effects and the key were not committed, so the message must
                // be redelivered.
                error!(
                    "SQS idempotency commit failed, message not deleted :: queue={} :: message_id={} :: trace_id={} :: idempotency_key={} :: key_source={} :: outcome=store_error :: error={:?}",
                    d.queue_url, d.message_id, d.trace_id, key, source, e
                );
                return IdempotencyOutcome::StoreError;
            }
            info!(
                "SQS message consumed :: queue={} :: message_id={} :: trace_id={} :: idempotency_key={} :: key_source={} :: outcome=processed{}",
                d.queue_url, d.message_id, d.trace_id, key, source, body_field
            );
            delete_message_best_effort(sqs, d.queue_url, d.receipt_handle).await;
            IdempotencyOutcome::Processed
        }
        Err(err) => {
            error!(
                "SQS message failed :: queue={} :: message_id={} :: trace_id={} :: idempotency_key={} :: key_source={} :: outcome=handler_failed{} :: error={:?}",
                d.queue_url, d.message_id, d.trace_id, key, source, body_field, err
            );
            if let Some(delay_secs) =
                retry_delay_secs(consumer.retry_policy.as_ref(), d.receive_count)
            {
                apply_retry_backoff(
                    sqs,
                    d.queue_url,
                    d.receipt_handle,
                    d.message_id,
                    d.receive_count,
                    delay_secs,
                    d.trace_id,
                )
                .await;
            }
            IdempotencyOutcome::HandlerFailed
        }
    }
}

/// Explicit rollback so the connection is clean before the pool gets it back.
#[cfg(feature = "postgres")]
async fn rollback_logged<T>(tx: IdempotentTx<T>, d: &Delivery<'_>, key: &str)
where
    T: Clone + Send + Sync + 'static,
{
    if let Err(e) = tx.transaction.rollback().await {
        warn!(
            "SQS idempotency rollback failed :: queue={} :: message_id={} :: trace_id={} :: idempotency_key={} :: error={}",
            d.queue_url, d.message_id, d.trace_id, key, e
        );
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
    let receive_count = msg.receive_count();
    let (tags, trace_id) = trace::tags_for(&msg);

    #[cfg(any(feature = "statsd", feature = "prometheus"))]
    let metric_tags =
        MetricTags::default().push("queue_url".to_string(), consumer.queue_url.clone());

    #[cfg(any(feature = "statsd", feature = "prometheus"))]
    let stopwatch = start_stopwatch(context, "sqs_consumer_seconds", metric_tags);

    if let Some(runtime) = &consumer.idempotency {
        let delivery = Delivery {
            queue_url: &consumer.queue_url,
            message_id: &message_id,
            body: &body,
            receipt_handle: &receipt_handle,
            receive_count,
            trace_id: &trace_id,
        };
        let outcome = match runtime {
            IdempotencyRuntime::NonTransactional { store, config } => {
                process_idempotent(context, sqs, consumer, store, config, msg, tags, &delivery)
                    .await
            }
            #[cfg(feature = "postgres")]
            IdempotencyRuntime::Transactional {
                config,
                reserve_sql,
            } => {
                process_transactional(
                    context,
                    sqs,
                    consumer,
                    config,
                    reserve_sql,
                    msg,
                    tags,
                    &delivery,
                )
                .await
            }
        };

        #[cfg(any(feature = "statsd", feature = "prometheus"))]
        {
            let success = if outcome.is_success() {
                "true"
            } else {
                "false"
            };
            let mut extra = MetricTags::default().push("success".to_string(), success.to_string());
            for (k, v) in idempotency_metric_tags(outcome) {
                extra = extra.push(k, v);
            }
            stopwatch.record(extra);
        }
        #[cfg(not(any(feature = "statsd", feature = "prometheus")))]
        let _ = outcome;
        return;
    }

    let result = (consumer.handler)(context.clone(), msg, tags).await;

    match &result {
        Ok(_) => {
            if context.env().is_local() || log_enabled!(Level::Debug) {
                info!(
                    "SQS message consumed :: queue={} :: message_id={} :: trace_id={} :: body={}",
                    consumer.queue_url, message_id, trace_id, body
                );
            } else {
                info!(
                    "SQS message consumed :: queue={} :: message_id={} :: trace_id={}",
                    consumer.queue_url, message_id, trace_id
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
                    "SQS message failed :: queue={} :: message_id={} :: trace_id={} :: body={} :: error={:?}",
                    consumer.queue_url, message_id, trace_id, body, err
                );
            } else {
                error!(
                    "SQS message failed :: queue={} :: message_id={} :: trace_id={} :: error={:?}",
                    consumer.queue_url, message_id, trace_id, err
                );
            }

            // Runs before the sqs_consumer_seconds metric is recorded, so the
            // ChangeMessageVisibility call time is included in it.
            if let Some(delay_secs) =
                retry_delay_secs(consumer.retry_policy.as_ref(), receive_count)
            {
                apply_retry_backoff(
                    sqs,
                    &consumer.queue_url,
                    &receipt_handle,
                    &message_id,
                    receive_count,
                    delay_secs,
                    &trace_id,
                )
                .await;
            }
        }
    }

    #[cfg(any(feature = "statsd", feature = "prometheus"))]
    {
        let success = if result.is_ok() { "true" } else { "false" };
        stopwatch.record(MetricTags::default().push("success".to_string(), success.to_string()));
    }
}

/// `None` when there is no policy (current behavior preserved).
fn retry_delay_secs(policy: Option<&RetryPolicy>, receive_count: u32) -> Option<i32> {
    policy.map(|p| to_visibility_timeout_secs(p.delay_for(receive_count)))
}

/// Reschedules a failed message. Failures are only logged: the message then
/// returns to the queue after the queue's default visibility timeout.
async fn apply_retry_backoff(
    sqs: &SqsClient,
    queue_url: &str,
    receipt_handle: &str,
    message_id: &str,
    receive_count: u32,
    delay_secs: i32,
    trace_id: &str,
) {
    match sqs
        .client
        .change_message_visibility()
        .queue_url(queue_url)
        .receipt_handle(receipt_handle)
        .visibility_timeout(delay_secs)
        .send()
        .await
    {
        Ok(_) => info!(
            "SQS message retry scheduled :: queue={} :: message_id={} :: receive_count={} :: retry_delay_secs={} :: trace_id={}",
            queue_url, message_id, receive_count, delay_secs, trace_id
        ),
        Err(e) => error!(
            "Failed to apply retry delay :: queue={} :: message_id={} :: receive_count={} :: retry_delay_secs={} :: trace_id={} :: error={:?}",
            queue_url, message_id, receive_count, delay_secs, trace_id, e
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aws_sdk_sqs::config::{retry::RetryConfig, BehaviorVersion, Credentials, Region};
    use aws_smithy_http_client::test_util::infallible_client_fn;
    use std::sync::{Arc, Mutex};

    type Calls = Arc<Mutex<Vec<(String, serde_json::Value)>>>;

    /// SQS client whose HTTP layer records (x-amz-target action, JSON body) and
    /// answers 200, or 400 for ChangeMessageVisibility when `fail_visibility`.
    fn fake_sqs(fail_visibility: bool) -> (SqsClient, Calls) {
        let calls: Calls = Arc::new(Mutex::new(Vec::new()));
        let recorded = calls.clone();
        let http_client = infallible_client_fn(move |req| {
            let target = req
                .headers()
                .get("x-amz-target")
                .and_then(|v| v.to_str().ok())
                .unwrap_or_default()
                .rsplit('.')
                .next()
                .unwrap_or_default()
                .to_string();
            let body = req
                .body()
                .bytes()
                .and_then(|b| serde_json::from_slice(b).ok())
                .unwrap_or(serde_json::Value::Null);
            recorded.lock().unwrap().push((target.clone(), body));
            if fail_visibility && target == "ChangeMessageVisibility" {
                http::Response::builder()
                    .status(400)
                    .header("content-type", "application/x-amz-json-1.0")
                    .body(r#"{"__type":"com.amazonaws.sqs#ReceiptHandleIsInvalid","message":"invalid"}"#)
                    .unwrap()
            } else {
                http::Response::builder()
                    .status(200)
                    .header("content-type", "application/x-amz-json-1.0")
                    .body("{}")
                    .unwrap()
            }
        });
        let config = aws_sdk_sqs::Config::builder()
            .behavior_version(BehaviorVersion::latest())
            .region(Region::new("us-east-1"))
            .credentials_provider(Credentials::new("t", "t", None, None, "test"))
            .retry_config(RetryConfig::disabled())
            .http_client(http_client)
            .build();
        (
            SqsClient {
                client: aws_sdk_sqs::Client::from_conf(config),
            },
            calls,
        )
    }

    fn actions(calls: &Calls) -> Vec<String> {
        calls
            .lock()
            .unwrap()
            .iter()
            .map(|(t, _)| t.clone())
            .collect()
    }

    #[test]
    fn message_attribute_names_only_adds_key_with_idempotency() {
        use super::super::idempotency::InMemoryIdempotencyStore;
        let plain = SqsQueueConsumer::<()>::new("q", |_c, _m, _t| async { Ok(()) });
        assert_eq!(message_attribute_names(&plain), vec![TRACE_ID_ATTRIBUTE]);
        let store: Arc<dyn IdempotencyStore> = Arc::new(InMemoryIdempotencyStore::new());
        let idem = plain
            .with_idempotency(
                store,
                IdempotencyConfig::default().with_key_attribute("my-key"),
            )
            .unwrap();
        assert_eq!(
            message_attribute_names(&idem),
            vec![TRACE_ID_ATTRIBUTE.to_string(), "my-key".to_string()]
        );
    }

    #[test]
    fn idempotency_metric_tags_per_outcome() {
        let t = |o| idempotency_metric_tags(o);
        assert_eq!(
            t(IdempotencyOutcome::DuplicateDiscarded),
            vec![("duplicate".to_string(), "true".to_string())]
        );
        assert_eq!(
            t(IdempotencyOutcome::DuplicateDeferred),
            vec![("duplicate".to_string(), "in_progress".to_string())]
        );
        assert_eq!(
            t(IdempotencyOutcome::StoreError),
            vec![("idempotency".to_string(), "store_error".to_string())]
        );
        assert!(t(IdempotencyOutcome::Processed).is_empty());
        assert!(t(IdempotencyOutcome::HandlerFailed).is_empty());
        assert!(IdempotencyOutcome::Processed.is_success());
        assert!(IdempotencyOutcome::DuplicateDeferred.is_success());
        assert!(!IdempotencyOutcome::StoreError.is_success());
        assert!(!IdempotencyOutcome::HandlerFailed.is_success());
    }

    #[test]
    fn retry_delay_secs_is_none_without_policy() {
        assert_eq!(retry_delay_secs(None, 5), None);
    }

    #[test]
    fn retry_delay_secs_is_within_ceiling() {
        let p = RetryPolicy::default();
        for _ in 0..500 {
            let s = retry_delay_secs(Some(&p), 3).unwrap();
            assert!((0..=20).contains(&s), "got {s}");
        }
    }

    #[test]
    fn retry_delay_secs_never_exceeds_12h() {
        let p = RetryPolicy::default().with_max_delay(Duration::from_secs(100 * 3600));
        for _ in 0..500 {
            let s = retry_delay_secs(Some(&p), u32::MAX).unwrap();
            assert!((0..=43_200).contains(&s), "got {s}");
        }
    }

    #[tokio::test]
    async fn apply_retry_backoff_sends_visibility_timeout() {
        let (sqs, calls) = fake_sqs(false);
        apply_retry_backoff(&sqs, "http://q/queue", "rh-1", "m-1", 3, 17, "t").await;
        let calls = calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "ChangeMessageVisibility");
        assert_eq!(calls[0].1["QueueUrl"], "http://q/queue");
        assert_eq!(calls[0].1["ReceiptHandle"], "rh-1");
        assert_eq!(calls[0].1["VisibilityTimeout"], 17);
    }

    #[tokio::test]
    async fn apply_retry_backoff_swallows_service_error() {
        let (sqs, calls) = fake_sqs(true);
        apply_retry_backoff(&sqs, "http://q/queue", "rh-1", "m-1", 3, 17, "t").await;
        apply_retry_backoff(&sqs, "http://q/queue", "rh-2", "m-2", 1, 0, "t").await;
        assert_eq!(actions(&calls).len(), 2);
    }

    // process_message needs an AppContext, whose constructor changes with metric/db features.
    #[cfg(not(any(
        feature = "statsd",
        feature = "prometheus",
        feature = "postgres",
        feature = "outbox",
        feature = "growthbook"
    )))]
    mod process {
        use super::*;
        use crate::commonx::{DerustError, DerustTags};
        use crate::envx::Environment;
        use axum::http::StatusCode;

        fn ctx() -> AppContext<()> {
            AppContext::new("test", Environment::Test, ()).unwrap()
        }

        fn sdk_msg_traced(id: &str, trace: Option<&str>) -> SdkMessage {
            let mut b = Message::builder()
                .message_id(id)
                .body("b")
                .receipt_handle(format!("rh-{id}"));
            if let Some(t) = trace {
                b = b.message_attribute("x-trace-id", t);
            }
            b.build().into_sdk()
        }

        type SeenTraces = Arc<Mutex<Vec<Option<String>>>>;

        fn recording() -> (SqsQueueConsumer<()>, SeenTraces) {
            let seen = Arc::new(Mutex::new(Vec::new()));
            let s = seen.clone();
            let consumer = SqsQueueConsumer::<()>::new("http://q/queue", move |_c, _m, t| {
                let s = s.clone();
                async move {
                    s.lock()
                        .unwrap()
                        .push(t.values().get("x-trace-id").cloned());
                    Ok(())
                }
            });
            (consumer, seen)
        }

        fn sdk_msg(id: &str, count: Option<u32>) -> SdkMessage {
            let mut b = Message::builder()
                .message_id(id)
                .body("b")
                .receipt_handle(format!("rh-{id}"));
            if let Some(c) = count {
                b = b.receive_count(c);
            }
            b.build().into_sdk()
        }

        fn failing() -> SqsQueueConsumer<()> {
            SqsQueueConsumer::<()>::new("http://q/queue", |_c, _m, _t| async {
                Err(DerustError::without_body(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "boom".to_string(),
                    DerustTags::default(),
                ))
            })
        }

        fn succeeding() -> SqsQueueConsumer<()> {
            SqsQueueConsumer::<()>::new("http://q/queue", |_c, _m, _t| async { Ok(()) })
        }

        #[tokio::test]
        async fn failing_handler_with_policy_changes_visibility() {
            let (sqs, calls) = fake_sqs(false);
            let consumer = failing().with_retry_policy(RetryPolicy::default());
            process_message(&ctx(), &sqs, &consumer, sdk_msg("1", Some(3))).await;
            assert_eq!(actions(&calls), vec!["ChangeMessageVisibility"]);
            let t = calls.lock().unwrap()[0].1["VisibilityTimeout"]
                .as_i64()
                .unwrap();
            assert!((0..=20).contains(&t), "got {t}");
        }

        #[tokio::test]
        async fn failing_handler_without_policy_makes_no_sqs_calls() {
            let (sqs, calls) = fake_sqs(false);
            process_message(&ctx(), &sqs, &failing(), sdk_msg("1", Some(3))).await;
            assert!(actions(&calls).is_empty());
        }

        #[tokio::test]
        async fn successful_handler_with_policy_deletes_and_does_not_change_visibility() {
            let (sqs, calls) = fake_sqs(false);
            let consumer = succeeding().with_retry_policy(RetryPolicy::default());
            process_message(&ctx(), &sqs, &consumer, sdk_msg("1", Some(2))).await;
            assert_eq!(actions(&calls), vec!["DeleteMessage"]);
        }

        #[tokio::test]
        async fn visibility_failure_does_not_stop_processing() {
            let (sqs, calls) = fake_sqs(true);
            let consumer = failing().with_retry_policy(RetryPolicy::default());
            process_message(&ctx(), &sqs, &consumer, sdk_msg("1", Some(1))).await;
            process_message(&ctx(), &sqs, &consumer, sdk_msg("2", Some(2))).await;
            assert_eq!(
                actions(&calls),
                vec!["ChangeMessageVisibility", "ChangeMessageVisibility"]
            );
            // a following successful message is still processed normally
            process_message(&ctx(), &sqs, &succeeding(), sdk_msg("3", None)).await;
            assert_eq!(actions(&calls).last().unwrap(), "DeleteMessage");
        }

        #[tokio::test]
        async fn handler_sees_receive_count() {
            let seen = Arc::new(Mutex::new(Vec::new()));
            let s = seen.clone();
            let consumer = SqsQueueConsumer::<()>::new("http://q/queue", move |_c, m, _t| {
                let s = s.clone();
                async move {
                    s.lock().unwrap().push(m.receive_count());
                    Ok(())
                }
            });
            let (sqs, _calls) = fake_sqs(false);
            process_message(&ctx(), &sqs, &consumer, sdk_msg("1", Some(4))).await;
            process_message(&ctx(), &sqs, &consumer, sdk_msg("2", None)).await;
            assert_eq!(*seen.lock().unwrap(), vec![4, 1]);
        }

        #[tokio::test]
        async fn handler_receives_trace_from_attribute() {
            let (consumer, seen) = recording();
            let (sqs, _c) = fake_sqs(false);
            process_message(&ctx(), &sqs, &consumer, sdk_msg_traced("1", Some("abc"))).await;
            assert_eq!(*seen.lock().unwrap(), vec![Some("abc".to_string())]);
        }

        #[tokio::test]
        async fn missing_or_invalid_trace_generates_new_and_still_consumes() {
            let (consumer, seen) = recording();
            let (sqs, calls) = fake_sqs(false);
            process_message(&ctx(), &sqs, &consumer, sdk_msg_traced("1", None)).await;
            process_message(&ctx(), &sqs, &consumer, sdk_msg_traced("2", Some(""))).await;
            let seen = seen.lock().unwrap();
            for t in seen.iter() {
                let t = t.as_ref().unwrap();
                assert!(t.len() == 32 && t.chars().all(|c| c.is_ascii_hexdigit()));
            }
            assert_eq!(actions(&calls), vec!["DeleteMessage", "DeleteMessage"]);
        }

        #[tokio::test]
        async fn batch_does_not_leak_trace_between_messages() {
            let (consumer, seen) = recording();
            let (sqs, _c) = fake_sqs(false);
            process_message(&ctx(), &sqs, &consumer, sdk_msg_traced("a", Some("aaa"))).await;
            process_message(&ctx(), &sqs, &consumer, sdk_msg_traced("b", None)).await;
            process_message(&ctx(), &sqs, &consumer, sdk_msg_traced("c", Some("ccc"))).await;
            let seen = seen.lock().unwrap();
            assert_eq!(seen[0].as_deref(), Some("aaa"));
            assert_ne!(seen[1].as_deref(), Some("aaa"));
            assert_eq!(seen[2].as_deref(), Some("ccc"));
        }

        #[tokio::test]
        async fn redelivery_keeps_same_trace() {
            let (consumer, seen) = recording();
            let (sqs, _c) = fake_sqs(false);
            process_message(&ctx(), &sqs, &consumer, sdk_msg_traced("1", Some("same"))).await;
            process_message(&ctx(), &sqs, &consumer, sdk_msg_traced("1", Some("same"))).await;
            let seen = seen.lock().unwrap();
            assert_eq!(seen[0], seen[1]);
        }

        // ---- idempotency ----
        use crate::awsx::sqsx::{BoxFuture, InMemoryIdempotencyStore, StoreError};
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::time::Duration as D;

        const Q: &str = "http://q/queue";

        /// Store that can fail `try_acquire` and/or `complete`, delegating to memory.
        struct FlakyStore {
            inner: InMemoryIdempotencyStore,
            fail_acquire: bool,
            fail_complete: bool,
        }

        impl IdempotencyStore for FlakyStore {
            fn try_acquire<'a>(
                &'a self,
                scope: &'a str,
                key: &'a str,
                ttl: D,
            ) -> BoxFuture<'a, Result<Acquire, StoreError>> {
                if self.fail_acquire {
                    return Box::pin(async { Err(StoreError::new("down")) });
                }
                self.inner.try_acquire(scope, key, ttl)
            }
            fn complete<'a>(
                &'a self,
                scope: &'a str,
                key: &'a str,
                ttl: D,
            ) -> BoxFuture<'a, Result<(), StoreError>> {
                if self.fail_complete {
                    return Box::pin(async { Err(StoreError::new("down")) });
                }
                self.inner.complete(scope, key, ttl)
            }
            fn release<'a>(
                &'a self,
                scope: &'a str,
                key: &'a str,
            ) -> BoxFuture<'a, Result<(), StoreError>> {
                self.inner.release(scope, key)
            }
        }

        fn counting(queue: &str, fail_first: usize) -> (SqsQueueConsumer<()>, Arc<AtomicUsize>) {
            let n = Arc::new(AtomicUsize::new(0));
            let c = n.clone();
            let consumer = SqsQueueConsumer::<()>::new(queue, move |_c, _m, _t| {
                let c = c.clone();
                async move {
                    if c.fetch_add(1, Ordering::SeqCst) < fail_first {
                        Err(DerustError::without_body(
                            StatusCode::INTERNAL_SERVER_ERROR,
                            "boom".to_string(),
                            DerustTags::default(),
                        ))
                    } else {
                        Ok(())
                    }
                }
            });
            (consumer, n)
        }

        fn idem_msg(id: &str, key: Option<&str>) -> SdkMessage {
            let mut b = Message::builder()
                .message_id(id)
                .body("b")
                .receipt_handle(format!("rh-{id}"));
            if let Some(k) = key {
                b = b.message_attribute("x-idempotent-key", k);
            }
            b.build().into_sdk()
        }

        fn with_store(
            consumer: SqsQueueConsumer<()>,
            store: Arc<dyn IdempotencyStore>,
        ) -> SqsQueueConsumer<()> {
            consumer
                .with_idempotency(store, IdempotencyConfig::default())
                .unwrap()
        }

        async fn acquire(store: &InMemoryIdempotencyStore, queue: &str, key: &str) -> Acquire {
            store
                .try_acquire(queue, key, D::from_secs(60))
                .await
                .unwrap()
        }

        #[tokio::test]
        async fn without_idempotency_nothing_new_is_called() {
            let (sqs, calls) = fake_sqs(false);
            let (consumer, n) = counting(Q, 0);
            process_message(&ctx(), &sqs, &consumer, idem_msg("1", Some("k"))).await;
            assert_eq!(n.load(Ordering::SeqCst), 1);
            assert_eq!(actions(&calls), vec!["DeleteMessage"]);
        }

        #[tokio::test]
        async fn new_key_success_runs_once_deletes_and_completes() {
            let store = Arc::new(InMemoryIdempotencyStore::new());
            let (sqs, calls) = fake_sqs(false);
            let (c, n) = counting(Q, 0);
            let consumer = with_store(c, store.clone());
            process_message(&ctx(), &sqs, &consumer, idem_msg("1", Some("k"))).await;
            assert_eq!(n.load(Ordering::SeqCst), 1);
            assert_eq!(actions(&calls), vec!["DeleteMessage"]);
            assert_eq!(acquire(&store, Q, "k").await, Acquire::AlreadyCompleted);
        }

        #[tokio::test]
        async fn redelivery_after_success_is_discarded_and_deleted() {
            let store = Arc::new(InMemoryIdempotencyStore::new());
            let (sqs, calls) = fake_sqs(false);
            let (c, n) = counting(Q, 0);
            let consumer = with_store(c, store);
            process_message(&ctx(), &sqs, &consumer, idem_msg("1", Some("k"))).await;
            process_message(&ctx(), &sqs, &consumer, idem_msg("2", Some("k"))).await;
            assert_eq!(n.load(Ordering::SeqCst), 1);
            assert_eq!(actions(&calls), vec!["DeleteMessage", "DeleteMessage"]);
        }

        #[tokio::test]
        async fn handler_error_releases_key_and_retries_with_backoff() {
            let store = Arc::new(InMemoryIdempotencyStore::new());
            let (sqs, calls) = fake_sqs(false);
            let (c, n) = counting(Q, 1);
            let consumer = with_store(c, store.clone()).with_retry_policy(RetryPolicy::default());
            process_message(&ctx(), &sqs, &consumer, idem_msg("1", Some("k"))).await;
            assert_eq!(actions(&calls), vec!["ChangeMessageVisibility"]);
            process_message(&ctx(), &sqs, &consumer, idem_msg("1", Some("k"))).await;
            assert_eq!(n.load(Ordering::SeqCst), 2);
            assert_eq!(
                actions(&calls),
                vec!["ChangeMessageVisibility", "DeleteMessage"]
            );
            assert_eq!(acquire(&store, Q, "k").await, Acquire::AlreadyCompleted);
        }

        #[tokio::test]
        async fn handler_panic_is_a_failure_and_releases_key() {
            let store = Arc::new(InMemoryIdempotencyStore::new());
            let (sqs, calls) = fake_sqs(false);
            fn boom() -> Result<(), DerustError> {
                panic!("boom")
            }
            let c = SqsQueueConsumer::<()>::new(Q, |_c, _m, _t| async { boom() });
            let consumer = with_store(c, store.clone());
            process_message(&ctx(), &sqs, &consumer, idem_msg("1", Some("k"))).await;
            assert!(actions(&calls).is_empty());
            assert_eq!(acquire(&store, Q, "k").await, Acquire::Acquired);
        }

        #[tokio::test]
        async fn in_progress_key_skips_handler_and_makes_no_sqs_call() {
            let store = Arc::new(InMemoryIdempotencyStore::new());
            assert_eq!(acquire(&store, Q, "k").await, Acquire::Acquired);
            let (sqs, calls) = fake_sqs(false);
            let (c, n) = counting(Q, 0);
            let consumer = with_store(c, store).with_retry_policy(RetryPolicy::default());
            process_message(&ctx(), &sqs, &consumer, idem_msg("1", Some("k"))).await;
            assert_eq!(n.load(Ordering::SeqCst), 0);
            assert!(actions(&calls).is_empty());
        }

        #[tokio::test]
        async fn store_error_is_fail_closed_without_backoff() {
            let store = Arc::new(FlakyStore {
                inner: InMemoryIdempotencyStore::new(),
                fail_acquire: true,
                fail_complete: false,
            });
            let (sqs, calls) = fake_sqs(false);
            let (c, n) = counting(Q, 0);
            let consumer = with_store(c, store).with_retry_policy(RetryPolicy::default());
            process_message(&ctx(), &sqs, &consumer, idem_msg("1", Some("k"))).await;
            assert_eq!(n.load(Ordering::SeqCst), 0);
            assert!(actions(&calls).is_empty());
        }

        #[tokio::test]
        async fn complete_failure_still_deletes_message() {
            let store = Arc::new(FlakyStore {
                inner: InMemoryIdempotencyStore::new(),
                fail_acquire: false,
                fail_complete: true,
            });
            let (sqs, calls) = fake_sqs(false);
            let (c, n) = counting(Q, 0);
            let consumer = with_store(c, store);
            process_message(&ctx(), &sqs, &consumer, idem_msg("1", Some("k"))).await;
            assert_eq!(n.load(Ordering::SeqCst), 1);
            assert_eq!(actions(&calls), vec!["DeleteMessage"]);
        }

        #[tokio::test]
        async fn key_comes_from_attribute_or_falls_back_to_message_id() {
            let store = Arc::new(InMemoryIdempotencyStore::new());
            let (sqs, _calls) = fake_sqs(false);
            let (c, n) = counting(Q, 0);
            let consumer = with_store(c, store.clone());
            // missing and empty attribute -> message_id
            process_message(&ctx(), &sqs, &consumer, idem_msg("m1", None)).await;
            process_message(&ctx(), &sqs, &consumer, idem_msg("m2", Some(""))).await;
            assert_eq!(acquire(&store, Q, "m1").await, Acquire::AlreadyCompleted);
            assert_eq!(acquire(&store, Q, "m2").await, Acquire::AlreadyCompleted);
            // same attribute, different message ids -> deduplicated
            process_message(&ctx(), &sqs, &consumer, idem_msg("a", Some("same"))).await;
            process_message(&ctx(), &sqs, &consumer, idem_msg("b", Some("same"))).await;
            assert_eq!(n.load(Ordering::SeqCst), 3);
            assert_eq!(acquire(&store, Q, "same").await, Acquire::AlreadyCompleted);
            assert_eq!(acquire(&store, Q, "a").await, Acquire::Acquired);
        }

        #[tokio::test]
        async fn scope_is_per_queue() {
            let store = Arc::new(InMemoryIdempotencyStore::new());
            let (sqs, _calls) = fake_sqs(false);
            let (c1, n1) = counting("http://q/one", 0);
            let (c2, n2) = counting("http://q/two", 0);
            let c1 = with_store(c1, store.clone());
            let c2 = with_store(c2, store);
            process_message(&ctx(), &sqs, &c1, idem_msg("1", Some("k"))).await;
            process_message(&ctx(), &sqs, &c2, idem_msg("1", Some("k"))).await;
            assert_eq!(n1.load(Ordering::SeqCst), 1);
            assert_eq!(n2.load(Ordering::SeqCst), 1);
        }

        #[test]
        fn invalid_config_is_rejected() {
            let store: Arc<dyn IdempotencyStore> = Arc::new(InMemoryIdempotencyStore::new());
            let (c, _n) = counting(Q, 0);
            let bad = IdempotencyConfig::default().with_key_attribute(" ");
            assert!(c.with_idempotency(store, bad).is_err());
        }
    }

    // Transactional mode needs a real Postgres (DERUST_TEST_DATABASE_URL; self-skipping without
    // it) and an AppContext with a database. The context constructor changes with the features:
    // growthbook needs a client, and statsd + prometheus together cannot both install the global
    // metrics recorder, so those combinations are not covered. With `prometheus` the global
    // recorder can be installed once per process: run under nextest (one process per test).
    #[cfg(all(
        feature = "postgres",
        not(feature = "growthbook"),
        not(all(feature = "statsd", feature = "prometheus"))
    ))]
    mod process_tx {
        use super::*;
        use crate::awsx::sqsx::idempotency::test_support::TestTable;
        use crate::awsx::sqsx::PostgresIdempotencyConfig;
        use crate::commonx::{DerustError, DerustTags};
        use crate::databasex::PostgresDatabase;
        use crate::envx::Environment;
        use axum::http::StatusCode;
        use sqlx::{AssertSqlSafe, Pool, Postgres};
        use std::sync::atomic::{AtomicUsize, Ordering};

        const Q: &str = "http://q/queue";

        #[cfg(not(any(feature = "statsd", feature = "prometheus")))]
        fn ctx_db(pool: Pool<Postgres>) -> AppContext<()> {
            let database = PostgresDatabase {
                read_write: pool,
                read_only: None,
            };
            AppContext::new("test", Environment::Test, database, ()).unwrap()
        }

        #[cfg(feature = "statsd")]
        fn ctx_db(pool: Pool<Postgres>) -> AppContext<()> {
            let database = PostgresDatabase {
                read_write: pool,
                read_only: None,
            };
            let statsd = crate::metricx::StatsdConfig {
                agent_host: "127.0.0.1".to_string(),
                agent_port: None,
                queue_size: None,
                buffer_size: None,
                default_tags: MetricTags::default(),
                denied_metric_tags: vec![],
                denied_metric_tags_by_regex: vec![],
            };
            AppContext::new("test", Environment::Test, database, statsd, ()).unwrap()
        }

        #[cfg(all(feature = "prometheus", not(feature = "statsd")))]
        fn ctx_db(pool: Pool<Postgres>) -> AppContext<()> {
            let database = PostgresDatabase {
                read_write: pool,
                read_only: None,
            };
            let prometheus = crate::metricx::PrometheusConfig {
                denied_metric_tags: vec![],
                denied_metric_tags_by_regex: vec![],
            };
            AppContext::new("test", Environment::Test, database, prometheus, ()).unwrap()
        }

        /// Idempotency table + an "effects" table, plus an AppContext on the same pool.
        struct Fx {
            t: TestTable,
            fx: String,
            ctx: AppContext<()>,
        }

        impl Fx {
            async fn new() -> Option<Self> {
                let t = TestTable::create().await?;
                let fx = format!("{}_fx", t.name);
                sqlx::query(AssertSqlSafe(format!(
                    "create table {fx} (id text not null)"
                )))
                .execute(&t.pool)
                .await
                .unwrap();
                let ctx = ctx_db(t.pool.clone());
                Some(Self { t, fx, ctx })
            }

            fn cfg(&self) -> PostgresIdempotencyConfig {
                PostgresIdempotencyConfig::default().with_table(self.t.name.clone())
            }

            async fn effects(&self) -> i64 {
                sqlx::query_scalar(AssertSqlSafe(format!("select count(*) from {}", self.fx)))
                    .fetch_one(&self.t.pool)
                    .await
                    .unwrap()
            }

            async fn keys(&self) -> Vec<String> {
                sqlx::query_scalar(AssertSqlSafe(format!(
                    "select idempotency_key from {} order by 1",
                    self.t.name
                )))
                .fetch_all(&self.t.pool)
                .await
                .unwrap()
            }

            /// No connection may stay checked out (stuck in an open transaction).
            async fn assert_pool_healthy(&self) {
                let pool = &self.t.pool;
                for _ in 0..60 {
                    if pool.size() as usize == pool.num_idle() {
                        break;
                    }
                    tokio::time::sleep(Duration::from_millis(50)).await;
                }
                assert_eq!(pool.size() as usize, pool.num_idle(), "connection leaked");
                let one: i32 = sqlx::query_scalar("select 1")
                    .fetch_one(pool)
                    .await
                    .unwrap();
                assert_eq!(one, 1);
                let open: i64 = sqlx::query_scalar(
                    "select count(*) from pg_stat_activity \
                     where application_name = $1 and pid <> pg_backend_pid() \
                     and state like 'idle in transaction%'",
                )
                .bind(&self.t.name)
                .fetch_one(pool)
                .await
                .unwrap();
                assert_eq!(open, 0, "transaction left open");
            }

            async fn finish(self) {
                sqlx::query(AssertSqlSafe(format!("drop table if exists {}", self.fx)))
                    .execute(&self.t.pool)
                    .await
                    .unwrap();
                self.t.drop_table().await;
            }
        }

        #[derive(Clone, Copy)]
        enum Plan {
            Commit,
            Fail,
            Panic,
            KillConnection,
        }

        fn boom() -> DerustError {
            DerustError::without_body(
                StatusCode::INTERNAL_SERVER_ERROR,
                "boom".to_string(),
                DerustTags::default(),
            )
        }

        /// Handler: counts the call, sleeps, inserts one effect row through the received
        /// transaction and then follows `plan[call_index]` (default: commit).
        fn tx_consumer(
            queue: &str,
            cfg: PostgresIdempotencyConfig,
            fx: &str,
            plan: Vec<Plan>,
            sleep_ms: u64,
        ) -> (SqsQueueConsumer<()>, Arc<AtomicUsize>) {
            let n = Arc::new(AtomicUsize::new(0));
            let counter = n.clone();
            let plan = Arc::new(plan);
            let fx = fx.to_string();
            let consumer = SqsQueueConsumer::<()>::new_transactional(
                queue,
                cfg,
                move |_ctx, msg, _tags, mut tx| {
                    let counter = counter.clone();
                    let plan = plan.clone();
                    let fx = fx.clone();
                    async move {
                        let i = counter.fetch_add(1, Ordering::SeqCst);
                        if sleep_ms > 0 {
                            tokio::time::sleep(Duration::from_millis(sleep_ms)).await;
                        }
                        sqlx::query(AssertSqlSafe(format!("insert into {fx} (id) values ($1)")))
                            .bind(msg.message_id().unwrap_or("?").to_string())
                            .execute(&mut *tx.transaction)
                            .await
                            .map_err(|_| boom())?;
                        match plan.get(i).copied().unwrap_or(Plan::Commit) {
                            Plan::Commit => Ok(tx),
                            Plan::Fail => Err(boom()),
                            Plan::Panic => panic!("boom"),
                            Plan::KillConnection => {
                                let _ =
                                    sqlx::query("select pg_terminate_backend(pg_backend_pid())")
                                        .execute(&mut *tx.transaction)
                                        .await;
                                Ok(tx)
                            }
                        }
                    }
                },
            )
            .unwrap();
            (consumer, n)
        }

        fn tx_msg(id: &str, key: Option<&str>) -> SdkMessage {
            let mut b = Message::builder()
                .message_id(id)
                .body("b")
                .receipt_handle(format!("rh-{id}"));
            if let Some(k) = key {
                b = b.message_attribute("x-idempotent-key", k);
            }
            b.build().into_sdk()
        }

        #[tokio::test]
        async fn new_key_success_commits_effect_and_key_then_deletes() {
            let Some(fx) = Fx::new().await else { return };
            let (sqs, calls) = fake_sqs(false);
            let (c, n) = tx_consumer(Q, fx.cfg(), &fx.fx, vec![], 0);
            process_message(&fx.ctx, &sqs, &c, tx_msg("1", Some("k"))).await;
            assert_eq!(n.load(Ordering::SeqCst), 1);
            assert_eq!(actions(&calls), vec!["DeleteMessage"]);
            assert_eq!(fx.effects().await, 1);
            assert_eq!(fx.keys().await, vec!["k"]);
            let status: String =
                sqlx::query_scalar(AssertSqlSafe(format!("select status from {}", fx.t.name)))
                    .fetch_one(&fx.t.pool)
                    .await
                    .unwrap();
            assert_eq!(status, "completed");
            fx.assert_pool_healthy().await;
            fx.finish().await;
        }

        #[tokio::test]
        async fn handler_error_rolls_back_effect_and_key_and_retries() {
            let Some(fx) = Fx::new().await else { return };
            let (sqs, calls) = fake_sqs(false);
            let (c, n) = tx_consumer(Q, fx.cfg(), &fx.fx, vec![Plan::Fail], 0);
            let c = c.with_retry_policy(RetryPolicy::default());
            process_message(&fx.ctx, &sqs, &c, tx_msg("1", Some("k"))).await;
            assert_eq!(actions(&calls), vec!["ChangeMessageVisibility"]);
            assert_eq!(fx.effects().await, 0, "effect must be rolled back");
            assert!(fx.keys().await.is_empty(), "key must be rolled back");
            fx.assert_pool_healthy().await;
            // next delivery runs the handler again and commits
            process_message(&fx.ctx, &sqs, &c, tx_msg("1", Some("k"))).await;
            assert_eq!(n.load(Ordering::SeqCst), 2);
            assert_eq!(
                actions(&calls),
                vec!["ChangeMessageVisibility", "DeleteMessage"]
            );
            assert_eq!(fx.effects().await, 1);
            assert_eq!(fx.keys().await, vec!["k"]);
            fx.finish().await;
        }

        #[tokio::test]
        async fn handler_panic_rolls_back_and_does_not_propagate() {
            let Some(fx) = Fx::new().await else { return };
            let (sqs, calls) = fake_sqs(false);
            let (c, n) = tx_consumer(Q, fx.cfg(), &fx.fx, vec![Plan::Panic], 0);
            process_message(&fx.ctx, &sqs, &c, tx_msg("1", Some("k"))).await;
            assert_eq!(n.load(Ordering::SeqCst), 1);
            assert!(actions(&calls).is_empty());
            assert_eq!(fx.effects().await, 0);
            assert!(fx.keys().await.is_empty());
            fx.assert_pool_healthy().await;
            process_message(&fx.ctx, &sqs, &c, tx_msg("1", Some("k"))).await;
            assert_eq!(n.load(Ordering::SeqCst), 2);
            assert_eq!(actions(&calls), vec!["DeleteMessage"]);
            fx.finish().await;
        }

        #[tokio::test]
        async fn completed_duplicate_is_discarded_and_deleted() {
            let Some(fx) = Fx::new().await else { return };
            let (sqs, calls) = fake_sqs(false);
            let (c, n) = tx_consumer(Q, fx.cfg(), &fx.fx, vec![], 0);
            process_message(&fx.ctx, &sqs, &c, tx_msg("1", Some("k"))).await;
            process_message(&fx.ctx, &sqs, &c, tx_msg("2", Some("k"))).await;
            assert_eq!(n.load(Ordering::SeqCst), 1);
            assert_eq!(actions(&calls), vec!["DeleteMessage", "DeleteMessage"]);
            assert_eq!(fx.effects().await, 1);
            fx.assert_pool_healthy().await;
            fx.finish().await;
        }

        #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
        async fn concurrent_duplicate_waits_for_lock_then_is_discarded() {
            let Some(fx) = Fx::new().await else { return };
            let (sqs, calls) = fake_sqs(false);
            let (c, n) = tx_consumer(Q, fx.cfg(), &fx.fx, vec![], 300);
            let started = std::time::Instant::now();
            tokio::join!(
                process_message(&fx.ctx, &sqs, &c, tx_msg("1", Some("k"))),
                process_message(&fx.ctx, &sqs, &c, tx_msg("2", Some("k"))),
            );
            assert!(started.elapsed() >= Duration::from_millis(300));
            assert_eq!(n.load(Ordering::SeqCst), 1, "handler must run once");
            assert_eq!(actions(&calls), vec!["DeleteMessage", "DeleteMessage"]);
            assert_eq!(fx.effects().await, 1);
            assert_eq!(fx.keys().await, vec!["k"]);
            fx.assert_pool_healthy().await;
            fx.finish().await;
        }

        #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
        async fn concurrent_duplicate_runs_after_first_rolls_back() {
            let Some(fx) = Fx::new().await else { return };
            let (sqs, calls) = fake_sqs(false);
            let (c, n) = tx_consumer(Q, fx.cfg(), &fx.fx, vec![Plan::Fail], 300);
            let c = c.with_retry_policy(RetryPolicy::default());
            tokio::join!(
                process_message(&fx.ctx, &sqs, &c, tx_msg("1", Some("k"))),
                process_message(&fx.ctx, &sqs, &c, tx_msg("2", Some("k"))),
            );
            assert_eq!(
                n.load(Ordering::SeqCst),
                2,
                "second runs after the rollback"
            );
            let mut got = actions(&calls);
            got.sort();
            assert_eq!(got, vec!["ChangeMessageVisibility", "DeleteMessage"]);
            assert_eq!(fx.effects().await, 1);
            assert_eq!(fx.keys().await, vec!["k"]);
            fx.assert_pool_healthy().await;
            fx.finish().await;
        }

        #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
        async fn short_lock_timeout_defers_without_delete_or_backoff() {
            let Some(fx) = Fx::new().await else { return };
            let (sqs1, calls1) = fake_sqs(false);
            let (sqs2, calls2) = fake_sqs(false);
            let cfg = fx.cfg().with_lock_timeout(Duration::from_millis(200));
            let (c, n) = tx_consumer(Q, cfg, &fx.fx, vec![], 700);
            let c = c.with_retry_policy(RetryPolicy::default());
            tokio::join!(
                process_message(&fx.ctx, &sqs1, &c, tx_msg("1", Some("k"))),
                async {
                    tokio::time::sleep(Duration::from_millis(150)).await;
                    process_message(&fx.ctx, &sqs2, &c, tx_msg("2", Some("k"))).await;
                },
            );
            assert_eq!(n.load(Ordering::SeqCst), 1);
            assert_eq!(actions(&calls1), vec!["DeleteMessage"]);
            assert!(
                actions(&calls2).is_empty(),
                "deferred delivery: no delete, no backoff"
            );
            assert_eq!(fx.effects().await, 1);
            fx.assert_pool_healthy().await;
            fx.finish().await;
        }

        #[tokio::test]
        async fn expired_key_is_reused() {
            let Some(fx) = Fx::new().await else { return };
            let (sqs, calls) = fake_sqs(false);
            let (c, n) = tx_consumer(Q, fx.cfg(), &fx.fx, vec![], 0);
            process_message(&fx.ctx, &sqs, &c, tx_msg("1", Some("k"))).await;
            fx.t.expire(Q, "k").await;
            process_message(&fx.ctx, &sqs, &c, tx_msg("2", Some("k"))).await;
            assert_eq!(n.load(Ordering::SeqCst), 2);
            assert_eq!(actions(&calls), vec!["DeleteMessage", "DeleteMessage"]);
            assert_eq!(fx.effects().await, 2);
            let live: i64 = sqlx::query_scalar(AssertSqlSafe(format!(
                "select count(*) from {} where expires_at > now()",
                fx.t.name
            )))
            .fetch_one(&fx.t.pool)
            .await
            .unwrap();
            assert_eq!(live, 1);
            fx.finish().await;
        }

        #[tokio::test]
        async fn key_comes_from_attribute_or_falls_back_to_message_id() {
            let Some(fx) = Fx::new().await else { return };
            let (sqs, _calls) = fake_sqs(false);
            let (c, n) = tx_consumer(Q, fx.cfg(), &fx.fx, vec![], 0);
            process_message(&fx.ctx, &sqs, &c, tx_msg("m1", None)).await;
            process_message(&fx.ctx, &sqs, &c, tx_msg("m2", Some(" "))).await;
            process_message(&fx.ctx, &sqs, &c, tx_msg("a", Some("same"))).await;
            process_message(&fx.ctx, &sqs, &c, tx_msg("b", Some("same"))).await;
            assert_eq!(n.load(Ordering::SeqCst), 3);
            assert_eq!(fx.keys().await, vec!["m1", "m2", "same"]);
            fx.finish().await;
        }

        #[tokio::test]
        async fn scope_is_per_queue() {
            let Some(fx) = Fx::new().await else { return };
            let (sqs, _calls) = fake_sqs(false);
            let (c1, n1) = tx_consumer("http://q/one", fx.cfg(), &fx.fx, vec![], 0);
            let (c2, n2) = tx_consumer("http://q/two", fx.cfg(), &fx.fx, vec![], 0);
            process_message(&fx.ctx, &sqs, &c1, tx_msg("1", Some("k"))).await;
            process_message(&fx.ctx, &sqs, &c2, tx_msg("1", Some("k"))).await;
            assert_eq!(n1.load(Ordering::SeqCst), 1);
            assert_eq!(n2.load(Ordering::SeqCst), 1);
            fx.finish().await;
        }

        #[tokio::test]
        async fn commit_failure_is_fail_closed_and_frees_the_pool() {
            let Some(fx) = Fx::new().await else { return };
            let (sqs, calls) = fake_sqs(false);
            let (c, n) = tx_consumer(Q, fx.cfg(), &fx.fx, vec![Plan::KillConnection], 0);
            let c = c.with_retry_policy(RetryPolicy::default());
            process_message(&fx.ctx, &sqs, &c, tx_msg("1", Some("k"))).await;
            assert_eq!(n.load(Ordering::SeqCst), 1);
            assert!(actions(&calls).is_empty(), "no delete and no backoff");
            assert_eq!(fx.effects().await, 0);
            assert!(fx.keys().await.is_empty());
            fx.assert_pool_healthy().await;
            // redelivery processes normally
            process_message(&fx.ctx, &sqs, &c, tx_msg("1", Some("k"))).await;
            assert_eq!(actions(&calls), vec!["DeleteMessage"]);
            fx.finish().await;
        }

        #[tokio::test]
        async fn unavailable_database_is_fail_closed() {
            let Some(fx) = Fx::new().await else { return };
            let (sqs, calls) = fake_sqs(false);
            let (c, n) = tx_consumer(Q, fx.cfg(), &fx.fx, vec![], 0);
            let c = c.with_retry_policy(RetryPolicy::default());
            // The context shares this pool: closing it makes `begin_transaction` fail.
            fx.t.pool.close().await;
            process_message(&fx.ctx, &sqs, &c, tx_msg("1", Some("k"))).await;
            assert_eq!(n.load(Ordering::SeqCst), 0);
            assert!(actions(&calls).is_empty());
            // clean up through a fresh pool
            let pool = sqlx::PgPool::connect(&std::env::var("DERUST_TEST_DATABASE_URL").unwrap())
                .await
                .unwrap();
            for table in [&fx.fx, &fx.t.name] {
                sqlx::query(AssertSqlSafe(format!("drop table if exists {table}")))
                    .execute(&pool)
                    .await
                    .unwrap();
            }
        }

        #[tokio::test]
        async fn missing_table_is_a_store_error_and_frees_the_connection() {
            let Some(fx) = Fx::new().await else { return };
            let (sqs, calls) = fake_sqs(false);
            let cfg = PostgresIdempotencyConfig::default().with_table("no_such_table_xyz");
            let (c, n) = tx_consumer(Q, cfg, &fx.fx, vec![], 0);
            process_message(&fx.ctx, &sqs, &c, tx_msg("1", Some("k"))).await;
            assert_eq!(n.load(Ordering::SeqCst), 0);
            assert!(actions(&calls).is_empty());
            fx.assert_pool_healthy().await;
            fx.finish().await;
        }
    }
}
