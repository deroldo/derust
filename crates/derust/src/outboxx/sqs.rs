use crate::commonx::{DerustError, DerustTags};
use crate::httpx::AppContext;
use crate::outboxx::insert_outbox;
use crate::outboxx::trace::{resolve_trace_id, with_trace_header_limited, SQS_SNS_MAX_ATTRIBUTES};
use chrono::{DateTime, Utc};
use outbox_pattern_processor::outbox::Outbox;
use serde_json::Value;
use sqlx::PgConnection;
use std::collections::HashMap;
use uuid::Uuid;

pub async fn send_to_sqs<S>(
    context: &AppContext<S>,
    db_conn: &mut PgConnection,
    partition_key: Uuid,
    queue_url: &str,
    headers: Option<HashMap<String, String>>,
    payload: &Value,
    tags: &DerustTags,
) -> Result<Outbox, DerustError>
where
    S: Clone,
{
    let trace_id = resolve_trace_id(tags);
    let headers =
        with_trace_header_limited(headers, trace_id.as_deref(), 1, SQS_SNS_MAX_ATTRIBUTES);
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
    tags: &DerustTags,
) -> Result<Outbox, DerustError>
where
    S: Clone,
{
    let trace_id = resolve_trace_id(tags);
    let headers =
        with_trace_header_limited(headers, trace_id.as_deref(), 1, SQS_SNS_MAX_ATTRIBUTES);
    let mut outbox = Outbox::sqs(partition_key, queue_url, headers, &payload.to_string());
    outbox.process_after = process_after;
    insert_outbox(context, db_conn, outbox, tags).await
}
