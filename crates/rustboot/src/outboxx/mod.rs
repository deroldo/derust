mod http;
mod runner;
mod sns;
mod sqs;

pub use http::*;
pub use runner::*;
pub use outbox_pattern_processor::outbox_resources::OutboxProcessorResources;

#[cfg(feature = "aws")]
pub use sqs::*;

#[cfg(feature = "aws")]
pub use sns::*;

use axum::http::StatusCode;
use outbox_pattern_processor::outbox::Outbox;
use outbox_pattern_processor::outbox_repository::OutboxRepository;
use sqlx::PgConnection;
use crate::httpx::{HttpError, Tags};

pub async fn insert_outbox(
    db_conn: &mut PgConnection,
    outbox: Outbox,
    tags: Tags,
) -> Result<Outbox, HttpError> {
    OutboxRepository::insert(db_conn, outbox)
        .await
        .map_err(|error| {
            HttpError::without_body(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to insert outbox: {error}"),
                tags,
            )
        })
}
