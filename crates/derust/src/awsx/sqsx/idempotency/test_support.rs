//! Test harness for tests that need a real Postgres (`DERUST_TEST_DATABASE_URL`).
//! Without the variable, [`TestTable::create`] returns `None` and tests return early.

use super::PostgresIdempotencyStore;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{AssertSqlSafe, Pool, Postgres};
use std::str::FromStr;

pub(crate) struct TestTable {
    pub pool: Pool<Postgres>,
    pub name: String,
    pub store: PostgresIdempotencyStore,
}

impl TestTable {
    /// Creates a uniquely named table with the official migration SQL.
    pub async fn create() -> Option<Self> {
        let url = match std::env::var("DERUST_TEST_DATABASE_URL") {
            Ok(url) if !url.trim().is_empty() => url,
            _ => {
                eprintln!("DERUST_TEST_DATABASE_URL not set: skipping Postgres test");
                return None;
            }
        };
        // The unique table name doubles as application_name, so tests can tell their own
        // connections apart in pg_stat_activity.
        let name = format!("sqs_idem_{}", uuid::Uuid::new_v4().simple());
        let options = PgConnectOptions::from_str(&url)
            .expect("parse DERUST_TEST_DATABASE_URL")
            .application_name(&name);
        let pool = PgPoolOptions::new()
            .max_connections(30)
            .connect_with(options)
            .await
            .expect("connect to DERUST_TEST_DATABASE_URL");
        let sql = PostgresIdempotencyStore::migration_sql(&name).unwrap();
        // The official SQL has two statements: run them one by one.
        for statement in sql.split(';').map(str::trim).filter(|s| !s.is_empty()) {
            sqlx::query(AssertSqlSafe(statement.to_string()))
                .execute(&pool)
                .await
                .expect("apply migration");
        }
        let store = PostgresIdempotencyStore::from_pool(pool.clone(), &name).unwrap();
        Some(Self { pool, name, store })
    }

    /// Forces a record to be expired (deterministic, no sleeping).
    pub async fn expire(&self, scope: &str, key: &str) {
        let sql = format!(
            "update {} set expires_at = now() - interval '1 second' \
             where queue_url = $1 and idempotency_key = $2",
            self.name
        );
        sqlx::query(AssertSqlSafe(sql))
            .bind(scope)
            .bind(key)
            .execute(&self.pool)
            .await
            .unwrap();
    }

    pub async fn count(&self) -> i64 {
        let sql = format!("select count(*) from {}", self.name);
        sqlx::query_scalar(AssertSqlSafe(sql))
            .fetch_one(&self.pool)
            .await
            .unwrap()
    }

    pub async fn drop_table(self) {
        let sql = format!("drop table if exists {}", self.name);
        sqlx::query(AssertSqlSafe(sql))
            .execute(&self.pool)
            .await
            .unwrap();
    }
}
