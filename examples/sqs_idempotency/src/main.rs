use derust::awsx::sqsx::{
    self, CleanupConfig, IdempotencyConfig, IdempotentTx, PostgresIdempotencyConfig,
    PostgresIdempotencyStore, SqsQueueConsumer,
};
use derust::databasex::{DatabaseConfig, PostgresDatabase};
use derust::envx::{load_app_config, Environment};
use derust::httpx::AppContext;
use serde::Deserialize;
use std::sync::Arc;
use std::time::Duration;
use wg::WaitGroup;

#[derive(Clone)]
pub struct AppState {}

#[derive(Clone, Deserialize)]
pub struct AppConfig {
    pub database: DatabaseConfig,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let env = Environment::detect().ok().unwrap_or(Environment::Local);
    let app_config: AppConfig = load_app_config(env, None).await?;
    let database = PostgresDatabase::create_from_config(&app_config.database).await?;
    let context = AppContext::new("sample", env, database.clone(), AppState {})?;

    let queue_url = "http://localhost:4566/000000000000/orders";

    // 1) Non transactional mode: the handler runs at most once with success per key.
    //    Works with any `IdempotencyStore` (Postgres here; `InMemoryIdempotencyStore` for tests).
    //    Producers should send the `x-idempotent-key` message attribute (fallback: message id).
    let store = Arc::new(PostgresIdempotencyStore::with_default_table(&database)?);
    let simple = SqsQueueConsumer::new(queue_url, |_ctx, msg, _tags| async move {
        tracing_free_log(msg.body().unwrap_or(""));
        Ok(())
    })
    .with_idempotency(
        store,
        IdempotencyConfig::default()
            .with_completed_ttl(Duration::from_secs(7 * 24 * 3600))
            .with_in_progress_ttl(Duration::from_secs(300))
            // opt-in, off by default: remove expired records every hour
            .with_cleanup(CleanupConfig::default()),
    )?;

    // 2) Transactional mode: the key and the handler's database effects are committed
    //    atomically. The handler receives the transaction and must return it; the consumer
    //    commits (never commit inside the handler). Do HTTP calls before using the transaction.
    let transactional = SqsQueueConsumer::<AppState>::new_transactional(
        "http://localhost:4566/000000000000/payments",
        PostgresIdempotencyConfig::default().with_lock_timeout(Duration::from_secs(10)),
        |_ctx, msg, _tags, mut tx: IdempotentTx<AppState>| async move {
            sqlx::query("select $1")
                .bind(msg.body().unwrap_or("").to_string())
                .execute(&mut *tx.transaction)
                .await
                .map_err(|e| {
                    derust::commonx::DerustError::without_body(
                        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                        e.to_string(),
                        derust::commonx::DerustTags::default(),
                    )
                })?;
            Ok(tx)
        },
    )?;

    let wg = WaitGroup::new();
    wg.add(1);
    sqsx::run(wg.clone(), context, vec![simple, transactional]).await;
    wg.wait();
    Ok(())
}

fn tracing_free_log(body: &str) {
    println!("processing {body}");
}
