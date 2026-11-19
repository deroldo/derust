use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use super::store::{Acquire, BoxFuture, IdempotencyStore, StoreError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Status {
    InProgress,
    Completed,
}

#[derive(Debug, Clone, Copy)]
struct Entry {
    status: Status,
    expires_at: Instant,
}

type Clock = Arc<dyn Fn() -> Instant + Send + Sync>;

/// In-process [`IdempotencyStore`], useful for tests and single-instance services.
///
/// State is lost on restart and is not shared between instances, so it does not
/// deduplicate across processes. Decisions are atomic (guarded by a `Mutex`).
pub struct InMemoryIdempotencyStore {
    entries: Mutex<HashMap<(String, String), Entry>>,
    clock: Clock,
}

impl Default for InMemoryIdempotencyStore {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryIdempotencyStore {
    pub fn new() -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
            clock: Arc::new(Instant::now),
        }
    }

    /// Store with an injected clock, to test TTLs without sleeping.
    #[cfg(test)]
    pub(crate) fn with_clock(clock: impl Fn() -> Instant + Send + Sync + 'static) -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
            clock: Arc::new(clock),
        }
    }

    fn now(&self) -> Instant {
        (self.clock)()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<(String, String), Entry>> {
        self.entries.lock().unwrap_or_else(|e| e.into_inner())
    }
}

impl IdempotencyStore for InMemoryIdempotencyStore {
    fn try_acquire<'a>(
        &'a self,
        scope: &'a str,
        key: &'a str,
        in_progress_ttl: Duration,
    ) -> BoxFuture<'a, Result<Acquire, StoreError>> {
        Box::pin(async move {
            let now = self.now();
            let mut map = self.lock();
            let k = (scope.to_string(), key.to_string());
            match map.get(&k) {
                Some(entry) if entry.expires_at > now => Ok(match entry.status {
                    Status::Completed => Acquire::AlreadyCompleted,
                    Status::InProgress => Acquire::InProgress,
                }),
                _ => {
                    map.insert(
                        k,
                        Entry {
                            status: Status::InProgress,
                            expires_at: now + in_progress_ttl,
                        },
                    );
                    Ok(Acquire::Acquired)
                }
            }
        })
    }

    fn complete<'a>(
        &'a self,
        scope: &'a str,
        key: &'a str,
        completed_ttl: Duration,
    ) -> BoxFuture<'a, Result<(), StoreError>> {
        Box::pin(async move {
            let now = self.now();
            self.lock().insert(
                (scope.to_string(), key.to_string()),
                Entry {
                    status: Status::Completed,
                    expires_at: now + completed_ttl,
                },
            );
            Ok(())
        })
    }

    fn release<'a>(
        &'a self,
        scope: &'a str,
        key: &'a str,
    ) -> BoxFuture<'a, Result<(), StoreError>> {
        Box::pin(async move {
            let mut map = self.lock();
            let k = (scope.to_string(), key.to_string());
            if map.get(&k).is_some_and(|e| e.status == Status::InProgress) {
                map.remove(&k);
            }
            Ok(())
        })
    }

    fn cleanup_expired(&self, batch_size: u32) -> BoxFuture<'_, Result<u64, StoreError>> {
        Box::pin(async move {
            let now = self.now();
            let mut map = self.lock();
            let expired: Vec<_> = map
                .iter()
                .filter(|(_, e)| e.expires_at <= now)
                .map(|(k, _)| k.clone())
                .take(batch_size as usize)
                .collect();
            for k in &expired {
                map.remove(k);
            }
            Ok(expired.len() as u64)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const Q: &str = "queue-a";
    const TTL: Duration = Duration::from_secs(10);

    /// Store plus a handle to advance its fake clock.
    fn store() -> (InMemoryIdempotencyStore, Arc<Mutex<Duration>>) {
        let base = Instant::now();
        let offset = Arc::new(Mutex::new(Duration::ZERO));
        let o = offset.clone();
        let s = InMemoryIdempotencyStore::with_clock(move || base + *o.lock().unwrap());
        (s, offset)
    }

    fn advance(offset: &Arc<Mutex<Duration>>, d: Duration) {
        *offset.lock().unwrap() += d;
    }

    #[tokio::test]
    async fn lifecycle() {
        let (s, _) = store();
        assert_eq!(s.try_acquire(Q, "k", TTL).await, Ok(Acquire::Acquired));
        assert_eq!(s.try_acquire(Q, "k", TTL).await, Ok(Acquire::InProgress));
        s.complete(Q, "k", TTL).await.unwrap();
        assert_eq!(
            s.try_acquire(Q, "k", TTL).await,
            Ok(Acquire::AlreadyCompleted)
        );
    }

    #[tokio::test]
    async fn release_frees_in_progress_only() {
        let (s, _) = store();
        s.try_acquire(Q, "k", TTL).await.unwrap();
        s.release(Q, "k").await.unwrap();
        assert_eq!(s.try_acquire(Q, "k", TTL).await, Ok(Acquire::Acquired));
        s.complete(Q, "k", TTL).await.unwrap();
        s.release(Q, "k").await.unwrap();
        assert_eq!(
            s.try_acquire(Q, "k", TTL).await,
            Ok(Acquire::AlreadyCompleted)
        );
    }

    #[tokio::test]
    async fn expired_in_progress_is_reacquired() {
        let (s, off) = store();
        s.try_acquire(Q, "k", TTL).await.unwrap();
        advance(&off, TTL - Duration::from_millis(1));
        assert_eq!(s.try_acquire(Q, "k", TTL).await, Ok(Acquire::InProgress));
        advance(&off, Duration::from_millis(1));
        assert_eq!(s.try_acquire(Q, "k", TTL).await, Ok(Acquire::Acquired));
    }

    #[tokio::test]
    async fn expired_completed_is_reacquired() {
        let (s, off) = store();
        s.try_acquire(Q, "k", TTL).await.unwrap();
        s.complete(Q, "k", TTL).await.unwrap();
        advance(&off, TTL);
        assert_eq!(s.try_acquire(Q, "k", TTL).await, Ok(Acquire::Acquired));
    }

    #[tokio::test]
    async fn scopes_do_not_collide() {
        let (s, _) = store();
        assert_eq!(s.try_acquire("a", "k", TTL).await, Ok(Acquire::Acquired));
        assert_eq!(s.try_acquire("b", "k", TTL).await, Ok(Acquire::Acquired));
        assert_eq!(s.try_acquire("a", "k", TTL).await, Ok(Acquire::InProgress));
    }

    #[tokio::test]
    async fn cleanup_removes_only_expired() {
        let (s, off) = store();
        s.try_acquire(Q, "short1", Duration::from_secs(1))
            .await
            .unwrap();
        s.try_acquire(Q, "short2", Duration::from_secs(1))
            .await
            .unwrap();
        s.try_acquire(Q, "long", Duration::from_secs(100))
            .await
            .unwrap();
        advance(&off, Duration::from_secs(5));
        assert_eq!(s.cleanup_expired(1).await, Ok(1));
        assert_eq!(s.cleanup_expired(10).await, Ok(1));
        assert_eq!(s.cleanup_expired(10).await, Ok(0));
        assert_eq!(s.try_acquire(Q, "long", TTL).await, Ok(Acquire::InProgress));
        assert_eq!(s.try_acquire(Q, "short1", TTL).await, Ok(Acquire::Acquired));
    }

    #[tokio::test]
    async fn default_cleanup_is_noop() {
        struct Noop;
        impl IdempotencyStore for Noop {
            fn try_acquire<'a>(
                &'a self,
                _: &'a str,
                _: &'a str,
                _: Duration,
            ) -> BoxFuture<'a, Result<Acquire, StoreError>> {
                Box::pin(async { Ok(Acquire::Acquired) })
            }
            fn complete<'a>(
                &'a self,
                _: &'a str,
                _: &'a str,
                _: Duration,
            ) -> BoxFuture<'a, Result<(), StoreError>> {
                Box::pin(async { Ok(()) })
            }
            fn release<'a>(
                &'a self,
                _: &'a str,
                _: &'a str,
            ) -> BoxFuture<'a, Result<(), StoreError>> {
                Box::pin(async { Ok(()) })
            }
        }
        assert_eq!(Noop.cleanup_expired(10).await, Ok(0));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn concurrent_acquire_has_exactly_one_winner() {
        let s = Arc::new(InMemoryIdempotencyStore::new());
        let handles: Vec<_> = (0..50)
            .map(|_| {
                let s = s.clone();
                tokio::spawn(async move { s.try_acquire(Q, "same", TTL).await.unwrap() })
            })
            .collect();
        let mut acquired = 0;
        for h in handles {
            if h.await.unwrap() == Acquire::Acquired {
                acquired += 1;
            }
        }
        assert_eq!(acquired, 1);
    }
}
