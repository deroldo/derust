//! Opt-in idempotency for SQS consumers.
//!
//! This module holds the store contract, the configuration, the idempotency key
//! resolution and an in-memory store. It has no dependency on a database.

mod config;
mod key;
mod memory;
#[cfg(feature = "postgres")]
mod postgres;
mod store;
#[cfg(feature = "postgres")]
mod table;
#[cfg(all(test, feature = "postgres"))]
mod test_support;

pub use config::{CleanupConfig, IdempotencyConfig, IdempotencyConfigError};
#[allow(unused_imports)]
pub(crate) use key::{resolve_key, KeySource, Resolved};
pub use memory::InMemoryIdempotencyStore;
#[cfg(feature = "postgres")]
pub use postgres::{PostgresIdempotencyStore, DEFAULT_TABLE_NAME};
pub use store::{Acquire, BoxFuture, IdempotencyStore, StoreError};
