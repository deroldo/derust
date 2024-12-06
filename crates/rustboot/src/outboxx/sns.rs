use crate::httpx::{HttpError, Tags};
use crate::outboxx::insert_outbox;
use outbox_pattern_processor::outbox::Outbox;
use serde_json::Value;
use sqlx::PgConnection;
use std::collections::HashMap;
use uuid::Uuid;

pub async fn send_to_sns(
    db_conn: &mut PgConnection,
    partition_key: Uuid,
    topic_arn: &str,
    headers: Option<HashMap<String, String>>,
    payload: &Value,
    tags: Tags,
) -> Result<Outbox, HttpError> {
    let outbox = Outbox::sns(partition_key, topic_arn, headers, &payload.to_string());
    insert_outbox(db_conn, outbox, tags).await
}
