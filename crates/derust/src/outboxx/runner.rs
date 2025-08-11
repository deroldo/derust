use crate::httpx::{AppContext, HttpError, HttpTags};
use crate::metricx::{current_gauge, MetricTags, start_stopwatch};
use crate::shutdown_signal;
use axum::http::StatusCode;
use outbox_pattern_processor::error::OutboxPatternProcessorError;
use outbox_pattern_processor::outbox_processor::OutboxProcessor;
use outbox_pattern_processor::outbox_resources::OutboxProcessorResources;
use std::time::Duration;
use tracing::{error, info};
use wg::WaitGroup;

pub async fn run<T>(
    wg: WaitGroup,
    context: AppContext<T>,
    outbox_processor_resources: OutboxProcessorResources,
    #[cfg(any(feature = "statsd", feature = "prometheus"))] metrics_monitor_enabled: bool,
    #[cfg(any(feature = "statsd", feature = "prometheus"))] outbox_metrics_monitor_interval_in_secs: Option<u64>,
) where
    T: Clone + Send + Sync + 'static,
{
    if outbox_processor_resources.scheduled_clear_locked_partition.unwrap_or(false) {
        tokio::spawn(run_clear_locked_partition(wg.add(1), outbox_processor_resources.clone()));
    }

    #[cfg(any(feature = "statsd", feature = "prometheus"))]
    if metrics_monitor_enabled {
        tokio::spawn(metrics_monitor(wg.add(1), context.clone(), outbox_processor_resources.clone(), outbox_metrics_monitor_interval_in_secs));
    }

    info!("Started embedded outbox-pattern-processor");

    let _ = OutboxProcessor::new(outbox_processor_resources.clone())
        .with_graceful_shutdown(shutdown_signal())
        .init_process()
        .await;

    wg.done();

    info!("Embedded outbox-pattern-processor stopped");
}

async fn run_clear_locked_partition(
    wg: WaitGroup,
    outbox_processor_resources: OutboxProcessorResources,
) {
    info!("Started embedded outbox-pattern-processor clear locked partition");

    let _ = OutboxProcessor::new(outbox_processor_resources)
        .with_graceful_shutdown(shutdown_signal())
        .init_processed_locked_cleaner()
        .await;

    wg.done();

    info!("Embedded outbox-pattern-processor clear locked partition stopped");
}

#[cfg(any(feature = "statsd", feature = "prometheus"))]
async fn metrics_monitor<T>(
    wg: WaitGroup,
    context: AppContext<T>,
    outbox_processor_resources: OutboxProcessorResources,
    outbox_metrics_monitor_interval_in_secs: Option<u64>,
) where
    T: Clone + Send + Sync + 'static,
{
    info!("Started embedded outbox-pattern-processor-monitor");

    let mut shutdown_signal = Box::into_pin(Box::new(shutdown_signal()));

    loop {
        tokio::select! {
            _ = one_shot_metrics_monitor(&context) => {
                tokio::time::sleep(Duration::from_secs(outbox_metrics_monitor_interval_in_secs.unwrap_or(5))).await; // TODO ajuste na duration
            }
            _ = &mut shutdown_signal => {
                break;
            }
        }
    }

    wg.done();

    info!("Embedded outbox-pattern-processor-monitor stopped");
}

#[cfg(any(feature = "statsd", feature = "prometheus"))]
async fn one_shot_metrics_monitor<T>(context: &AppContext<T>) -> Result<(), HttpError>
where
    T: Clone + Send + Sync + 'static,
{
    let tags = HttpTags::default();

    let mut metric_tags = MetricTags::from(tags.clone()).push("operation".to_string(), "monitor".to_string());
    let stopwatch = start_stopwatch(&context, "outbox_pattern_processor", metric_tags.clone());

    let query = sqlx::query_scalar::<_, i32>(
        r#"
        select coalesce(floor(extract(epoch from (now() - min(process_after)))), 0)::int as delay
        from outbox
        where processed_at is null
        "#,
    );

    let mut conn = context.database().get_connection(true, &tags).await?;

    let result = query.fetch_one(&mut *conn).await.map_err(|error| {
        HttpError::without_body(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to one_shot_metrics_monitor: {error}"),
            tags.clone(),
        )
    });

    match result {
        Ok(delay) => {
            current_gauge(&context, "outbox_pattern_processor_delay", metric_tags.clone(), delay as f64);
        },
        Err(error) => {
            error!("Failed when try to get outbox_pattern_processor_delay!");
        }
    }

    stopwatch.record(metric_tags.clone());

    Ok(())
}
