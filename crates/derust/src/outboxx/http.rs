use crate::commonx::{DerustError, DerustTags};
use crate::httpx::AppContext;
use crate::outboxx::insert_outbox;
use chrono::{DateTime, Utc};
use outbox_pattern_processor::outbox::Outbox;
use serde_json::Value;
use sqlx::PgConnection;
use std::collections::HashMap;
use uuid::Uuid;

pub async fn send_to_http<S>(
    context: &AppContext<S>,
    db_conn: &mut PgConnection,
    partition_key: Uuid,
    url: &str,
    headers: Option<HashMap<String, String>>,
    payload: &Value,
    tags: &DerustTags,
) -> Result<Outbox, DerustError>
where
    S: Clone,
{
    let outbox = Outbox::http_post_json(partition_key, url, headers, payload);
    insert_outbox(context, db_conn, outbox, tags).await
}

pub async fn send_to_http_with_delay<S>(
    context: &AppContext<S>,
    db_conn: &mut PgConnection,
    partition_key: Uuid,
    url: &str,
    headers: Option<HashMap<String, String>>,
    payload: &Value,
    process_after: Option<DateTime<Utc>>,
    tags: &DerustTags,
) -> Result<Outbox, DerustError>
where
    S: Clone,
{
    let mut outbox = Outbox::http_post_json(partition_key, url, headers, payload);
    outbox.process_after = process_after;
    insert_outbox(context, db_conn, outbox, tags).await
}
