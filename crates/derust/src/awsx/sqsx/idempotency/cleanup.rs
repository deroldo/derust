//! Background cleanup of expired idempotency records (opt-in via `with_cleanup`).
//!
//! [`run_cleanup_loop`] is generic over the [`IdempotencyStore`] contract, so it serves the
//! in-memory store, the Postgres store and any custom backend. [`cleanup_plan`] is the pure
//! decision of which consumers need a cleanup task.

use super::{CleanupConfig, IdempotencyStore};
use crate::awsx::sqsx::consumer::SqsQueueConsumer;
use std::future::Future;
use std::sync::Arc;
use tracing::{error, info};

/// Where the records of a consumer live.
pub(crate) enum CleanupTarget {
    /// Non-transactional consumer: the store given to `with_idempotency`.
    Store(Arc<dyn IdempotencyStore>),
    /// Transactional consumer: a `PostgresIdempotencyStore` must be built on this table
    /// from the application database.
    #[cfg(feature = "postgres")]
    Table(String),
}

/// One cleanup task to spawn.
pub(crate) struct CleanupPlan {
    pub queue_url: String,
    pub config: CleanupConfig,
    pub target: CleanupTarget,
}

/// Cleanup tasks required by `consumers`: one per consumer with cleanup configured,
/// empty when none has it.
pub(crate) fn cleanup_plan<T>(consumers: &[SqsQueueConsumer<T>]) -> Vec<CleanupPlan>
where
    T: Clone + Send + Sync + 'static,
{
    consumers
        .iter()
        .filter_map(|consumer| {
            let runtime = consumer.idempotency.as_ref()?;
            let config = runtime.cleanup_config()?.clone();
            Some(CleanupPlan {
                queue_url: consumer.queue_url.clone(),
                config,
                target: runtime.cleanup_target(),
            })
        })
        .collect()
}

/// Removes expired records every `cfg.interval` until `shutdown` completes.
///
/// Each cycle calls `cleanup_expired(batch_size)` repeatedly while it returns a full batch
/// (drain), logs the total at `info` (only when greater than zero) and logs store failures
/// at `error` without stopping the loop. `label` identifies the queue in the logs.
pub(crate) async fn run_cleanup_loop(
    store: Arc<dyn IdempotencyStore>,
    cfg: CleanupConfig,
    label: String,
    shutdown: impl Future<Output = ()>,
) {
    tokio::pin!(shutdown);
    loop {
        tokio::select! {
            _ = &mut shutdown => break,
            _ = tokio::time::sleep(cfg.interval) => {}
        }
        tokio::select! {
            _ = &mut shutdown => break,
            _ = cleanup_cycle(store.as_ref(), cfg.batch_size, &label) => {}
        }
    }
    info!("Idempotency cleanup stopped for queue {label}");
}

async fn cleanup_cycle(store: &dyn IdempotencyStore, batch_size: u32, label: &str) {
    let mut total: u64 = 0;
    loop {
        match store.cleanup_expired(batch_size).await {
            Ok(removed) => {
                total += removed;
                if removed < u64::from(batch_size) {
                    break;
                }
            }
            Err(err) => {
                error!("Idempotency cleanup failed for queue {label}: {err}");
                break;
            }
        }
    }
    if total > 0 {
        info!("Idempotency cleanup removed {total} expired records for queue {label}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::awsx::sqsx::idempotency::{
        Acquire, BoxFuture, IdempotencyConfig, InMemoryIdempotencyStore, StoreError,
    };
    use std::collections::VecDeque;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;
    use std::time::{Duration, Instant};

    /// Store whose `cleanup_expired` replays a script (then returns `Ok(0)`), counting calls.
    struct Scripted {
        calls: Arc<AtomicUsize>,
        script: Mutex<VecDeque<Result<u64, StoreError>>>,
    }

    impl Scripted {
        fn new(script: Vec<Result<u64, StoreError>>) -> (Arc<Self>, Arc<AtomicUsize>) {
            let calls = Arc::new(AtomicUsize::new(0));
            let s = Arc::new(Self {
                calls: calls.clone(),
                script: Mutex::new(script.into()),
            });
            (s, calls)
        }
    }

    impl IdempotencyStore for Scripted {
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
        fn release<'a>(&'a self, _: &'a str, _: &'a str) -> BoxFuture<'a, Result<(), StoreError>> {
            Box::pin(async { Ok(()) })
        }
        fn cleanup_expired(&self, _: u32) -> BoxFuture<'_, Result<u64, StoreError>> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            let next = self.script.lock().unwrap().pop_front().unwrap_or(Ok(0));
            Box::pin(async move { next })
        }
    }

    fn cfg(interval_ms: u64, batch_size: u32) -> CleanupConfig {
        CleanupConfig {
            interval: Duration::from_millis(interval_ms),
            batch_size,
        }
    }

    async fn wait_until(what: &str, cond: impl Fn() -> bool) {
        let deadline = Instant::now() + Duration::from_secs(10);
        while !cond() {
            assert!(Instant::now() < deadline, "timed out waiting for {what}");
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    }

    fn spawn_loop(
        store: Arc<dyn IdempotencyStore>,
        cfg: CleanupConfig,
    ) -> (
        tokio::sync::oneshot::Sender<()>,
        tokio::task::JoinHandle<()>,
    ) {
        let (tx, rx) = tokio::sync::oneshot::channel::<()>();
        let handle = tokio::spawn(run_cleanup_loop(store, cfg, "q".into(), async move {
            let _ = rx.await;
        }));
        (tx, handle)
    }

    #[tokio::test]
    async fn runs_every_interval() {
        let (store, calls) = Scripted::new(vec![]);
        let (tx, handle) = spawn_loop(store, cfg(10, 10));
        wait_until("3 cycles", || calls.load(Ordering::SeqCst) >= 3).await;
        tx.send(()).unwrap();
        handle.await.unwrap();
    }

    #[tokio::test]
    async fn does_not_run_before_first_interval() {
        let (store, calls) = Scripted::new(vec![]);
        let (tx, handle) = spawn_loop(store, cfg(60_000, 10));
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        tx.send(()).unwrap();
        handle.await.unwrap();
    }

    #[tokio::test]
    async fn drains_while_batch_is_full() {
        let (store, calls) = Scripted::new(vec![Ok(5), Ok(5), Ok(2)]);
        // Long interval relative to the drain: the three calls belong to the first cycle.
        let (tx, handle) = spawn_loop(store, cfg(400, 5));
        wait_until("drain", || calls.load(Ordering::SeqCst) >= 3).await;
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert_eq!(calls.load(Ordering::SeqCst), 3);
        tx.send(()).unwrap();
        handle.await.unwrap();
    }

    #[tokio::test]
    async fn continues_after_store_error() {
        let (store, calls) = Scripted::new(vec![Err(StoreError::new("boom")), Ok(0)]);
        let (tx, handle) = spawn_loop(store, cfg(10, 10));
        wait_until("cycle after error", || calls.load(Ordering::SeqCst) >= 2).await;
        tx.send(()).unwrap();
        handle.await.unwrap();
    }

    #[tokio::test]
    async fn stops_on_shutdown() {
        let (store, calls) = Scripted::new(vec![]);
        let (tx, handle) = spawn_loop(store, cfg(10, 10));
        wait_until("first cycle", || calls.load(Ordering::SeqCst) >= 1).await;
        tx.send(()).unwrap();
        tokio::time::timeout(Duration::from_secs(5), handle)
            .await
            .expect("loop stops")
            .unwrap();
        let after = calls.load(Ordering::SeqCst);
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert_eq!(calls.load(Ordering::SeqCst), after);
    }

    #[tokio::test]
    async fn stops_on_shutdown_during_long_interval() {
        let (store, _) = Scripted::new(vec![]);
        let (tx, handle) = spawn_loop(store, cfg(3_600_000, 10));
        tx.send(()).unwrap();
        tokio::time::timeout(Duration::from_secs(5), handle)
            .await
            .expect("loop stops")
            .unwrap();
    }

    #[tokio::test]
    async fn removes_expired_from_in_memory_store() {
        let store = Arc::new(InMemoryIdempotencyStore::new());
        store
            .try_acquire("q", "k", Duration::from_millis(1))
            .await
            .unwrap();
        tokio::time::sleep(Duration::from_millis(10)).await;
        let (tx, handle) = spawn_loop(store.clone(), cfg(10, 10));
        tokio::time::sleep(Duration::from_millis(100)).await;
        tx.send(()).unwrap();
        handle.await.unwrap();
        // The expired record is gone and the key is free again.
        assert_eq!(
            store.try_acquire("q", "k", Duration::from_secs(5)).await,
            Ok(Acquire::Acquired)
        );
    }

    fn consumer() -> SqsQueueConsumer<()> {
        SqsQueueConsumer::new("q", |_, _, _| async { Ok(()) })
    }

    #[test]
    fn plan_is_empty_without_cleanup() {
        let store: Arc<dyn IdempotencyStore> = Arc::new(InMemoryIdempotencyStore::new());
        let plain = consumer();
        let idem = consumer()
            .with_idempotency(store, IdempotencyConfig::default())
            .unwrap();
        assert!(cleanup_plan::<()>(&[]).is_empty());
        assert!(cleanup_plan(&[plain, idem]).is_empty());
    }

    #[test]
    fn plan_has_one_entry_per_consumer_with_cleanup() {
        let store: Arc<dyn IdempotencyStore> = Arc::new(InMemoryIdempotencyStore::new());
        let with = consumer()
            .with_idempotency(
                store.clone(),
                IdempotencyConfig::default().with_cleanup(cfg(7, 3)),
            )
            .unwrap();
        let without = consumer()
            .with_idempotency(store, IdempotencyConfig::default())
            .unwrap();
        let plan = cleanup_plan(&[without, with]);
        assert_eq!(plan.len(), 1);
        assert_eq!(plan[0].config, cfg(7, 3));
        assert!(matches!(plan[0].target, CleanupTarget::Store(_)));
    }

    #[cfg(feature = "postgres")]
    mod postgres {
        use super::*;
        use crate::awsx::sqsx::idempotency::test_support::TestTable;

        #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
        async fn concurrent_loops_remove_only_expired_rows() {
            let Some(t) = TestTable::create().await else {
                return;
            };
            let ttl = Duration::from_secs(300);
            for i in 0..60 {
                t.store
                    .try_acquire("q", &format!("expired-{i}"), ttl)
                    .await
                    .unwrap();
                t.expire("q", &format!("expired-{i}")).await;
            }
            for i in 0..5 {
                t.store
                    .try_acquire("q", &format!("valid-{i}"), ttl)
                    .await
                    .unwrap();
            }
            assert_eq!(t.count().await, 65);

            let store: Arc<dyn IdempotencyStore> = Arc::new(t.store.clone());
            // Small batches force several drain iterations in both loops at once.
            let (tx1, h1) = spawn_loop(store.clone(), cfg(10, 7));
            let (tx2, h2) = spawn_loop(store, cfg(10, 7));
            let deadline = Instant::now() + Duration::from_secs(10);
            while t.count().await > 5 {
                assert!(Instant::now() < deadline, "rows not cleaned in time");
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            tx1.send(()).unwrap();
            tx2.send(()).unwrap();
            h1.await.unwrap();
            h2.await.unwrap();

            assert_eq!(t.count().await, 5);
            for i in 0..5 {
                assert_eq!(
                    t.store.try_acquire("q", &format!("valid-{i}"), ttl).await,
                    Ok(Acquire::InProgress)
                );
            }
            t.drop_table().await;
        }
    }
}
