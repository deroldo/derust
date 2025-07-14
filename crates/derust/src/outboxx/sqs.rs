use crate::httpx::{AppContext, HttpError, HttpTags};
use crate::outboxx::insert_outbox;
use outbox_pattern_processor::outbox::Outbox;
use serde_json::Value;
use sqlx::PgConnection;
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use uuid::Uuid;

pub async fn send_to_sqs<S>(
    context: &AppContext<S>,
    db_conn: &mut PgConnection,
    partition_key: Uuid,
    queue_url: &str,
    headers: Option<HashMap<String, String>>,
    payload: &Value,
    tags: &HttpTags,
) -> Result<Outbox, HttpError>
where
    S: Clone,
{
    let outbox = Outbox::sqs(partition_key, queue_url, headers, &payload.to_string());
    insert_outbox(context, db_conn, outbox, tags).await
}

pub async fn send_to_sqs_with_delay<S>(
    context: &AppContext<S>,
    db_conn: &mut PgConnection,
    partition_key: Uuid,
    queue_url: &str,
    headers: Option<HashMap<String, String>>,
    payload: &Value,
    process_after: Option<DateTime<Utc>>,
    tags: &HttpTags,
) -> Result<Outbox, HttpError>
where
    S: Clone,
{
    let mut outbox = Outbox::sqs(partition_key, queue_url, headers, &payload.to_string());
    outbox.process_after = process_after;
    insert_outbox(context, db_conn, outbox, tags).await
}
