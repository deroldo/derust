//! Transactional idempotent mode (Postgres): the idempotency key is written in the same
//! transaction as the handler's effects, so both commit or roll back together.
//!
//! The only interpolated value is the table name, validated by
//! `table::validate_table_name` (hence `AssertSqlSafe`); every value is bound.

use super::config::{DEFAULT_COMPLETED_TTL, DEFAULT_KEY_ATTRIBUTE};
use super::postgres::DEFAULT_TABLE_NAME;
use super::table::validate_table_name;
use super::{CleanupConfig, IdempotencyConfigError};
use crate::databasex::PostgresTransaction;
use sqlx::{AssertSqlSafe, PgConnection};
use std::time::Duration;

const DEFAULT_LOCK_TIMEOUT: Duration = Duration::from_secs(10);
/// PostgreSQL maximum for `lock_timeout` (`INT_MAX` milliseconds).
const MAX_LOCK_TIMEOUT_MILLIS: u128 = i32::MAX as u128;
/// SQLSTATE `lock_not_available`, raised when `lock_timeout` expires.
const LOCK_NOT_AVAILABLE: &str = "55P03";

/// Maps a state type to the transaction type handed to transactional handlers. It exists so
/// that [`IdempotentTx`] has the same spelling with and without `statsd`/`prometheus`
/// (`PostgresTransaction<'static, S>` with them, `PostgresTransaction<'static>` without).
#[doc(hidden)]
pub trait IdempotentTxOf {
    type Tx;
}

#[cfg(any(feature = "statsd", feature = "prometheus"))]
impl<S: Clone> IdempotentTxOf for S {
    type Tx = PostgresTransaction<'static, S>;
}

#[cfg(not(any(feature = "statsd", feature = "prometheus")))]
impl<S: Clone> IdempotentTxOf for S {
    type Tx = PostgresTransaction<'static>;
}

/// The database transaction owned by a transactional handler (see
/// `SqsQueueConsumer::new_transactional`): a `PostgresTransaction<'static, S>` (with
/// `statsd`/`prometheus`) or `PostgresTransaction<'static>` (without them). Use
/// `tx.transaction` as the executor of your queries and return the transaction in `Ok`.
pub type IdempotentTx<S> = <S as IdempotentTxOf>::Tx;

/// Configuration of the transactional idempotent mode. The `with_*` setters never fail; the
/// consumer calls [`PostgresIdempotencyConfig::validate`] when it is created.
///
/// The idempotency table is the one created by
/// [`PostgresIdempotencyStore::migration_sql`](super::PostgresIdempotencyStore::migration_sql).
/// The transaction and the store always use the `read_write` pool and the database should run
/// in `READ COMMITTED` (the PostgreSQL default).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostgresIdempotencyConfig {
    /// Table name (optionally `schema.table`). Default: `sqs_idempotency`.
    pub table: String,
    /// Message attribute holding the idempotency key. Default: `x-idempotent-key`;
    /// when absent or blank the SQS `message_id` is used.
    pub key_attribute: String,
    /// How long a successfully processed key is remembered. Default: 7 days. It should cover
    /// the redelivery/republishing window (SQS retains messages up to 14 days).
    pub completed_ttl: Duration,
    /// How long a duplicate waits for the transaction that holds the same key before the
    /// delivery is deferred (left in the queue, no backoff). Default: 10 seconds, less than a
    /// typical 30 s visibility timeout. Must be at least 1 ms.
    pub lock_timeout: Duration,
    /// Background cleanup of expired records. Default: `None` (disabled).
    pub cleanup: Option<CleanupConfig>,
}

impl Default for PostgresIdempotencyConfig {
    fn default() -> Self {
        Self {
            table: DEFAULT_TABLE_NAME.to_string(),
            key_attribute: DEFAULT_KEY_ATTRIBUTE.to_string(),
            completed_ttl: DEFAULT_COMPLETED_TTL,
            lock_timeout: DEFAULT_LOCK_TIMEOUT,
            cleanup: None,
        }
    }
}

impl PostgresIdempotencyConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_table(mut self, table: impl Into<String>) -> Self {
        self.table = table.into();
        self
    }

    pub fn with_key_attribute(mut self, key_attribute: impl Into<String>) -> Self {
        self.key_attribute = key_attribute.into();
        self
    }

    pub fn with_completed_ttl(mut self, ttl: Duration) -> Self {
        self.completed_ttl = ttl;
        self
    }

    pub fn with_lock_timeout(mut self, lock_timeout: Duration) -> Self {
        self.lock_timeout = lock_timeout;
        self
    }

    pub fn with_cleanup(mut self, cleanup: CleanupConfig) -> Self {
        self.cleanup = Some(cleanup);
        self
    }

    /// Checks the table name, that the key attribute is not blank, that `completed_ttl` is
    /// greater than zero, that `lock_timeout` is between 1 ms and PostgreSQL's maximum, and the
    /// cleanup settings when set.
    pub fn validate(&self) -> Result<(), IdempotencyConfigError> {
        validate_table_name(&self.table)?;
        if self.completed_ttl.is_zero() {
            return Err(IdempotencyConfigError::InvalidTtl);
        }
        if !(1..=MAX_LOCK_TIMEOUT_MILLIS).contains(&self.lock_timeout.as_millis()) {
            return Err(IdempotencyConfigError::InvalidTtl);
        }
        if self.key_attribute.trim().is_empty() {
            return Err(IdempotencyConfigError::EmptyKeyAttribute);
        }
        if let Some(cleanup) = &self.cleanup {
            cleanup.validate()?;
        }
        Ok(())
    }

    /// `lock_timeout` in whole milliseconds (already validated, so it always fits).
    pub(crate) fn lock_timeout_millis(&self) -> u32 {
        u32::try_from(self.lock_timeout.as_millis()).unwrap_or(i32::MAX as u32)
    }
}

/// SQL that reserves the key as `completed` inside the handler's transaction. A returned row
/// means the key is new (or expired and reused); no row means a concurrent/previous
/// transaction already committed it. Uses the primary key `(queue_url, idempotency_key)`.
pub(crate) fn reserve_sql(table: &str) -> Result<String, IdempotencyConfigError> {
    let t = validate_table_name(table)?;
    Ok(format!(
        "insert into {t} as idem (queue_url, idempotency_key, status, expires_at) \
         values ($1, $2, 'completed', now() + make_interval(secs => $3::float8)) \
         on conflict (queue_url, idempotency_key) do update \
         set status = 'completed', expires_at = excluded.expires_at, created_at = now() \
         where idem.expires_at <= now() \
         returning 1"
    ))
}

/// Outcome of the reservation inside the transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Reservation {
    /// The key is ours: run the handler.
    Fresh,
    /// The key was already committed by another delivery.
    Duplicate,
}

/// Failure of the reservation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ReserveError {
    /// `lock_timeout` expired waiting for a transaction holding the same key (SQLSTATE 55P03).
    LockNotAvailable,
    /// Any other SQL or pool error (fail-closed).
    Store(String),
}

/// Sets `lock_timeout` for this transaction (bound value, `set_config(.., true)` = local) and
/// reserves the key.
pub(crate) async fn reserve(
    conn: &mut PgConnection,
    sql: &str,
    scope: &str,
    key: &str,
    completed_ttl: Duration,
    lock_timeout_millis: u32,
) -> Result<Reservation, ReserveError> {
    sqlx::query("select set_config('lock_timeout', $1, true)")
        .bind(format!("{lock_timeout_millis}ms"))
        .execute(&mut *conn)
        .await
        .map_err(|e| ReserveError::Store(e.to_string()))?;

    let row = sqlx::query(AssertSqlSafe(sql))
        .bind(scope)
        .bind(key)
        .bind(completed_ttl.as_secs_f64())
        .fetch_optional(&mut *conn)
        .await
        .map_err(|e| {
            let lock_not_available = e
                .as_database_error()
                .and_then(|db| db.code())
                .is_some_and(|code| code == LOCK_NOT_AVAILABLE);
            if lock_not_available {
                ReserveError::LockNotAvailable
            } else {
                ReserveError::Store(e.to_string())
            }
        })?;
    Ok(if row.is_some() {
        Reservation::Fresh
    } else {
        Reservation::Duplicate
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults() {
        let c = PostgresIdempotencyConfig::default();
        assert_eq!(c.table, "sqs_idempotency");
        assert_eq!(c.key_attribute, "x-idempotent-key");
        assert_eq!(c.completed_ttl, Duration::from_secs(7 * 86_400));
        assert_eq!(c.lock_timeout, Duration::from_secs(10));
        assert!(c.cleanup.is_none());
        assert_eq!(c.validate(), Ok(()));
        assert_eq!(c.lock_timeout_millis(), 10_000);
    }

    #[test]
    fn setters() {
        let c = PostgresIdempotencyConfig::new()
            .with_table("app.t")
            .with_key_attribute("k")
            .with_completed_ttl(Duration::from_secs(1))
            .with_lock_timeout(Duration::from_millis(250))
            .with_cleanup(CleanupConfig::default());
        assert_eq!(c.table, "app.t");
        assert_eq!(c.key_attribute, "k");
        assert_eq!(c.completed_ttl, Duration::from_secs(1));
        assert_eq!(c.lock_timeout_millis(), 250);
        assert!(c.cleanup.is_some());
        assert_eq!(c.validate(), Ok(()));
    }

    #[test]
    fn rejects_invalid() {
        let base = PostgresIdempotencyConfig::new;
        assert_eq!(
            base().with_table("x; drop table y").validate(),
            Err(IdempotencyConfigError::InvalidTableName)
        );
        assert_eq!(
            base().with_completed_ttl(Duration::ZERO).validate(),
            Err(IdempotencyConfigError::InvalidTtl)
        );
        for bad in [
            Duration::ZERO,
            Duration::from_micros(500),
            Duration::from_millis(i32::MAX as u64 + 1),
        ] {
            assert_eq!(
                base().with_lock_timeout(bad).validate(),
                Err(IdempotencyConfigError::InvalidTtl),
                "{bad:?}"
            );
        }
        assert_eq!(
            base()
                .with_lock_timeout(Duration::from_millis(i32::MAX as u64))
                .validate(),
            Ok(())
        );
        assert_eq!(
            base().with_key_attribute("  ").validate(),
            Err(IdempotencyConfigError::EmptyKeyAttribute)
        );
        assert_eq!(
            base()
                .with_cleanup(CleanupConfig {
                    interval: Duration::from_secs(1),
                    batch_size: 0
                })
                .validate(),
            Err(IdempotencyConfigError::InvalidBatchSize)
        );
    }

    #[test]
    fn reserve_sql_uses_validated_table_and_binds_values() {
        let sql = reserve_sql("app.sqs_idempotency").unwrap();
        assert!(sql.contains("insert into app.sqs_idempotency as idem"));
        assert!(sql.contains("on conflict (queue_url, idempotency_key)"));
        assert!(sql.contains("$1") && sql.contains("$2") && sql.contains("$3"));
        assert_eq!(
            reserve_sql("x; drop table y"),
            Err(IdempotencyConfigError::InvalidTableName)
        );
    }
}
