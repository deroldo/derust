use crate::httpx::{AppContext, HttpError, HttpTags};
use crate::outboxx::insert_outbox;
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
    tags: HttpTags,
) -> Result<Outbox, HttpError>
where
    S: Clone,
{
    let outbox = Outbox::sqs(partition_key, queue_url, headers, &payload.to_string());
    insert_outbox(context, db_conn, outbox, tags).await
}
