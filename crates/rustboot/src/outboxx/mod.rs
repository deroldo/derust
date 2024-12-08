mod http;
mod runner;
mod sns;
mod sqs;

pub use http::*;
pub use outbox_pattern_processor::outbox_resources::OutboxProcessorResources;
pub use runner::*;

use crate::httpx::{AppContext, HttpError, HttpTags};
use axum::http::StatusCode;
use outbox_pattern_processor::outbox::Outbox;
use outbox_pattern_processor::outbox_repository::OutboxRepository;
use sqlx::PgConnection;

#[cfg(feature = "aws")]
pub use sqs::*;

#[cfg(feature = "aws")]
pub use sns::*;

#[cfg(any(feature = "statsd", feature = "prometheus"))]
use crate::metricx::{timer, MetricTags};

pub async fn insert_outbox<S>(
    context: &AppContext<S>,
    db_conn: &mut PgConnection,
    outbox: Outbox,
    tags: HttpTags,
) -> Result<Outbox, HttpError>
where
    S: Clone,
{
    #[cfg(any(feature = "statsd", feature = "prometheus"))]
    let stopwatch = timer::start_stopwatch(
        context,
        "repository_outbox_insert_seconds",
        MetricTags::from(tags.clone()),
    );

    let result = OutboxRepository::insert(db_conn, outbox)
        .await
        .map_err(|error| {
            HttpError::without_body(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to insert outbox: {error}"),
                tags.clone(),
            )
        });

    #[cfg(any(feature = "statsd", feature = "prometheus"))]
    {
        let success = match result {
            Ok(_) => "true",
            Err(_) => "false",
        };

        let mut result_metric_tags = MetricTags::from(tags);
        result_metric_tags = result_metric_tags.push("success".to_string(), success.to_string());
        stopwatch.record(result_metric_tags);
    }

    result
}
