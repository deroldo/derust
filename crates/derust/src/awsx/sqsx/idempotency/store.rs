use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

/// Boxed, `Send` future returned by [`IdempotencyStore`] methods.
///
/// A plain alias is used instead of `async-trait`, which is not part of the `sqs` feature.
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Result of [`IdempotencyStore::try_acquire`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Acquire {
    /// The key was free (or expired) and is now reserved as "in progress" by the caller.
    Acquired,
    /// The key was already processed successfully and has not expired yet.
    AlreadyCompleted,
    /// Another delivery currently holds the key and its in-progress TTL has not expired.
    InProgress,
}

/// Error returned by an [`IdempotencyStore`] backend.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreError(pub String);

impl StoreError {
    pub fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "idempotency store error: {}", self.0)
    }
}

impl std::error::Error for StoreError {}

/// Storage contract used to deduplicate SQS messages.
///
/// The full key is the pair `(scope, key)`; the consumer uses the queue URL as `scope`.
/// Implementations must make `try_acquire` atomic: for concurrent calls with the same
/// `(scope, key)` exactly one may return [`Acquire::Acquired`].
///
/// Lifecycle: `try_acquire` reserves the key as in progress (expires after
/// `in_progress_ttl`, so a crashed consumer does not block the key forever); `complete`
/// marks it processed for `completed_ttl`; `release` frees an in-progress key after a
/// failure so the next delivery can retry.
pub trait IdempotencyStore: Send + Sync + 'static {
    /// Atomically reserves `(scope, key)` as in progress for `in_progress_ttl`.
    ///
    /// An expired entry (in progress or completed) is treated as absent.
    fn try_acquire<'a>(
        &'a self,
        scope: &'a str,
        key: &'a str,
        in_progress_ttl: Duration,
    ) -> BoxFuture<'a, Result<Acquire, StoreError>>;

    /// Marks `(scope, key)` as completed; the record is kept for `completed_ttl`.
    fn complete<'a>(
        &'a self,
        scope: &'a str,
        key: &'a str,
        completed_ttl: Duration,
    ) -> BoxFuture<'a, Result<(), StoreError>>;

    /// Frees an in-progress `(scope, key)` so another delivery can acquire it.
    /// Completed records are left untouched.
    fn release<'a>(&'a self, scope: &'a str, key: &'a str)
        -> BoxFuture<'a, Result<(), StoreError>>;

    /// Removes up to `batch_size` expired records and returns how many were removed.
    /// The default implementation does nothing.
    fn cleanup_expired(&self, _batch_size: u32) -> BoxFuture<'_, Result<u64, StoreError>> {
        Box::pin(async { Ok(0) })
    }
}
