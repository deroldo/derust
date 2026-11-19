//! Postgres-backed [`IdempotencyStore`] (non transactional operations).
//!
//! The only interpolated value is the table name, validated by `table::validate_table_name`
//! (hence `AssertSqlSafe`); every value is bound.

use super::table::{index_suffix, validate_table_name};
use super::{Acquire, BoxFuture, IdempotencyConfigError, IdempotencyStore, StoreError};
use crate::databasex::PostgresDatabase;
use sqlx::{AssertSqlSafe, Pool, Postgres};
use std::time::Duration;

/// Default table name used by [`PostgresIdempotencyStore::with_default_table`].
pub const DEFAULT_TABLE_NAME: &str = "sqs_idempotency";

const MIGRATION_TEMPLATE: &str = include_str!("postgres_migration.sql");

/// [`IdempotencyStore`] backed by a Postgres table. The table is never created at runtime:
/// apply [`PostgresIdempotencyStore::migration_sql`] with your migration tool.
///
/// All time comparisons use the database clock (`now()`).
#[derive(Clone)]
pub struct PostgresIdempotencyStore {
    pool: Pool<Postgres>,
    acquire_sql: String,
    status_sql: String,
    complete_sql: String,
    release_sql: String,
    cleanup_sql: String,
}

impl PostgresIdempotencyStore {
    /// Creates a store on the `read_write` pool of `database`. `table` may be
    /// `schema.table`; it is validated because it is interpolated into the SQL.
    pub fn new(database: &PostgresDatabase, table: &str) -> Result<Self, IdempotencyConfigError> {
        Self::from_pool(database.read_write.clone(), table)
    }

    /// Same as [`Self::new`] with [`DEFAULT_TABLE_NAME`].
    pub fn with_default_table(database: &PostgresDatabase) -> Result<Self, IdempotencyConfigError> {
        Self::new(database, DEFAULT_TABLE_NAME)
    }

    pub(crate) fn from_pool(
        pool: Pool<Postgres>,
        table: &str,
    ) -> Result<Self, IdempotencyConfigError> {
        let t = validate_table_name(table)?;
        Ok(Self {
            pool,
            acquire_sql: format!(
                "insert into {t} as idem (queue_url, idempotency_key, status, expires_at) \
                 values ($1, $2, 'in_progress', now() + make_interval(secs => $3::float8)) \
                 on conflict (queue_url, idempotency_key) do update \
                 set status = 'in_progress', expires_at = excluded.expires_at, created_at = now() \
                 where idem.expires_at <= now() \
                 returning 1"
            ),
            status_sql: format!(
                "select status from {t} where queue_url = $1 and idempotency_key = $2"
            ),
            complete_sql: format!(
                "update {t} set status = 'completed', \
                 expires_at = now() + make_interval(secs => $3::float8) \
                 where queue_url = $1 and idempotency_key = $2"
            ),
            release_sql: format!(
                "delete from {t} where queue_url = $1 and idempotency_key = $2 \
                 and status = 'in_progress'"
            ),
            cleanup_sql: format!(
                "delete from {t} where ctid in (select ctid from {t} \
                 where expires_at < now() limit $1 for update skip locked)"
            ),
        })
    }

    /// Official migration SQL for `table` (placeholders replaced after validating the name).
    pub fn migration_sql(table: &str) -> Result<String, IdempotencyConfigError> {
        let t = validate_table_name(table)?;
        Ok(MIGRATION_TEMPLATE
            .replace("{index}", &index_suffix(t))
            .replace("{table}", t))
    }
}

fn store_error(error: sqlx::Error) -> StoreError {
    StoreError::new(error.to_string())
}

impl IdempotencyStore for PostgresIdempotencyStore {
    fn try_acquire<'a>(
        &'a self,
        scope: &'a str,
        key: &'a str,
        in_progress_ttl: Duration,
    ) -> BoxFuture<'a, Result<Acquire, StoreError>> {
        Box::pin(async move {
            let acquired = sqlx::query(AssertSqlSafe(self.acquire_sql.as_str()))
                .bind(scope)
                .bind(key)
                .bind(in_progress_ttl.as_secs_f64())
                .fetch_optional(&self.pool)
                .await
                .map_err(store_error)?;
            if acquired.is_some() {
                return Ok(Acquire::Acquired);
            }
            let status: Option<(String,)> = sqlx::query_as(AssertSqlSafe(self.status_sql.as_str()))
                .bind(scope)
                .bind(key)
                .fetch_optional(&self.pool)
                .await
                .map_err(store_error)?;
            // A vanished row (race with release) is reported as in progress: SQS redelivers.
            Ok(match status {
                Some((status,)) if status == "completed" => Acquire::AlreadyCompleted,
                _ => Acquire::InProgress,
            })
        })
    }

    fn complete<'a>(
        &'a self,
        scope: &'a str,
        key: &'a str,
        completed_ttl: Duration,
    ) -> BoxFuture<'a, Result<(), StoreError>> {
        Box::pin(async move {
            sqlx::query(AssertSqlSafe(self.complete_sql.as_str()))
                .bind(scope)
                .bind(key)
                .bind(completed_ttl.as_secs_f64())
                .execute(&self.pool)
                .await
                .map_err(store_error)?;
            Ok(())
        })
    }

    fn release<'a>(
        &'a self,
        scope: &'a str,
        key: &'a str,
    ) -> BoxFuture<'a, Result<(), StoreError>> {
        Box::pin(async move {
            sqlx::query(AssertSqlSafe(self.release_sql.as_str()))
                .bind(scope)
                .bind(key)
                .execute(&self.pool)
                .await
                .map_err(store_error)?;
            Ok(())
        })
    }

    fn cleanup_expired(&self, batch_size: u32) -> BoxFuture<'_, Result<u64, StoreError>> {
        Box::pin(async move {
            let result = sqlx::query(AssertSqlSafe(self.cleanup_sql.as_str()))
                .bind(i64::from(batch_size))
                .execute(&self.pool)
                .await
                .map_err(store_error)?;
            Ok(result.rows_affected())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::awsx::sqsx::idempotency::test_support::TestTable;
    use std::sync::Arc;
    use tokio::sync::Barrier;

    const TTL: Duration = Duration::from_secs(60);

    #[test]
    fn migration_sql_replaces_placeholders() {
        let sql = PostgresIdempotencyStore::migration_sql("sqs_idempotency").unwrap();
        assert!(sql.contains("create table sqs_idempotency"));
        assert!(sql.contains("primary key (queue_url, idempotency_key)"));
        assert!(sql.contains("idx_sqs_idempotency_by_expires_at on sqs_idempotency"));
        assert!(!sql.contains('{'));
        let sql = PostgresIdempotencyStore::migration_sql("app.sqs_idempotency").unwrap();
        assert!(sql.contains("create table app.sqs_idempotency"));
        assert!(sql.contains("idx_app_sqs_idempotency_by_expires_at on app.sqs_idempotency"));
    }

    #[test]
    fn migration_sql_rejects_invalid_name() {
        assert_eq!(
            PostgresIdempotencyStore::migration_sql("x; drop table y"),
            Err(IdempotencyConfigError::InvalidTableName)
        );
    }

    #[tokio::test]
    async fn new_rejects_invalid_table_without_connecting() {
        let pool = sqlx::postgres::PgPoolOptions::new()
            .connect_lazy("postgres://u:p@127.0.0.1:1/d")
            .unwrap();
        let database = PostgresDatabase {
            read_write: pool,
            read_only: None,
        };
        assert!(PostgresIdempotencyStore::new(&database, "a;b").is_err());
        assert!(PostgresIdempotencyStore::with_default_table(&database).is_ok());
    }

    #[tokio::test]
    async fn acquire_complete_release_lifecycle() {
        let Some(t) = TestTable::create().await else {
            return;
        };
        let s = &t.store;
        assert_eq!(
            s.try_acquire("q", "k", TTL).await.unwrap(),
            Acquire::Acquired
        );
        assert_eq!(
            s.try_acquire("q", "k", TTL).await.unwrap(),
            Acquire::InProgress
        );
        s.release("q", "k").await.unwrap();
        assert_eq!(
            s.try_acquire("q", "k", TTL).await.unwrap(),
            Acquire::Acquired
        );
        s.complete("q", "k", TTL).await.unwrap();
        assert_eq!(
            s.try_acquire("q", "k", TTL).await.unwrap(),
            Acquire::AlreadyCompleted
        );
        // release does not touch completed records
        s.release("q", "k").await.unwrap();
        assert_eq!(
            s.try_acquire("q", "k", TTL).await.unwrap(),
            Acquire::AlreadyCompleted
        );
        t.drop_table().await;
    }

    #[tokio::test]
    async fn scopes_are_independent() {
        let Some(t) = TestTable::create().await else {
            return;
        };
        let s = &t.store;
        assert_eq!(
            s.try_acquire("q1", "k", TTL).await.unwrap(),
            Acquire::Acquired
        );
        assert_eq!(
            s.try_acquire("q2", "k", TTL).await.unwrap(),
            Acquire::Acquired
        );
        t.drop_table().await;
    }

    #[tokio::test]
    async fn expired_records_are_reacquired() {
        let Some(t) = TestTable::create().await else {
            return;
        };
        let s = &t.store;
        // expired in progress
        s.try_acquire("q", "a", TTL).await.unwrap();
        t.expire("q", "a").await;
        assert_eq!(
            s.try_acquire("q", "a", TTL).await.unwrap(),
            Acquire::Acquired
        );
        // expired completed
        s.try_acquire("q", "b", TTL).await.unwrap();
        s.complete("q", "b", TTL).await.unwrap();
        t.expire("q", "b").await;
        assert_eq!(
            s.try_acquire("q", "b", TTL).await.unwrap(),
            Acquire::Acquired
        );
        // reacquired entry is in progress again, not completed
        assert_eq!(
            s.try_acquire("q", "b", TTL).await.unwrap(),
            Acquire::InProgress
        );
        t.drop_table().await;
    }

    #[tokio::test]
    async fn cleanup_removes_only_expired_and_respects_batch() {
        let Some(t) = TestTable::create().await else {
            return;
        };
        let s = &t.store;
        for i in 0..5 {
            let key = format!("old{i}");
            s.try_acquire("q", &key, TTL).await.unwrap();
            t.expire("q", &key).await;
        }
        s.try_acquire("q", "live", TTL).await.unwrap();
        assert_eq!(s.cleanup_expired(2).await.unwrap(), 2);
        assert_eq!(t.count().await, 4);
        assert_eq!(s.cleanup_expired(100).await.unwrap(), 3);
        assert_eq!(t.count().await, 1);
        assert_eq!(s.cleanup_expired(100).await.unwrap(), 0);
        assert_eq!(
            s.try_acquire("q", "live", TTL).await.unwrap(),
            Acquire::InProgress
        );
        t.drop_table().await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn concurrent_acquire_has_single_winner() {
        let Some(t) = TestTable::create().await else {
            return;
        };
        let store = Arc::new(t.store.clone());
        let barrier = Arc::new(Barrier::new(20));
        let mut handles = Vec::new();
        for _ in 0..20 {
            let (store, barrier) = (store.clone(), barrier.clone());
            handles.push(tokio::spawn(async move {
                barrier.wait().await;
                store.try_acquire("q", "same", TTL).await.unwrap()
            }));
        }
        let mut acquired = 0;
        let mut in_progress = 0;
        for h in handles {
            match h.await.unwrap() {
                Acquire::Acquired => acquired += 1,
                Acquire::InProgress => in_progress += 1,
                Acquire::AlreadyCompleted => panic!("unexpected AlreadyCompleted"),
            }
        }
        assert_eq!((acquired, in_progress), (1, 19));
        t.drop_table().await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn concurrent_cleanup_does_not_double_delete() {
        let Some(t) = TestTable::create().await else {
            return;
        };
        for i in 0..50 {
            let key = format!("old{i}");
            t.store.try_acquire("q", &key, TTL).await.unwrap();
            t.expire("q", &key).await;
        }
        let store = Arc::new(t.store.clone());
        let barrier = Arc::new(Barrier::new(2));
        let mut handles = Vec::new();
        for _ in 0..2 {
            let (store, barrier) = (store.clone(), barrier.clone());
            handles.push(tokio::spawn(async move {
                barrier.wait().await;
                let mut total = 0;
                loop {
                    let n = store.cleanup_expired(7).await.unwrap();
                    total += n;
                    if n == 0 {
                        break;
                    }
                }
                total
            }));
        }
        let mut sum = 0;
        for h in handles {
            sum += h.await.unwrap();
        }
        assert_eq!(sum, 50);
        assert_eq!(t.count().await, 0);
        t.drop_table().await;
    }
}
