use crate::httpx::{HttpError, HttpTags};
use crate::outboxx::insert_outbox;
use outbox_pattern_processor::outbox::Outbox;
use serde_json::Value;
use sqlx::PgConnection;
use std::collections::HashMap;
use uuid::Uuid;

pub async fn send_to_http(
    db_conn: &mut PgConnection,
    partition_key: Uuid,
    url: &str,
    headers: Option<HashMap<String, String>>,
    payload: &Value,
    tags: HttpTags,
) -> Result<Outbox, HttpError> {
    let outbox = Outbox::http_post_json(partition_key, url, headers, payload);
    insert_outbox(db_conn, outbox, tags).await
}
