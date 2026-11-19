//! Opt-in idempotency for SQS consumers.
//!
//! This module holds the store contract, the configuration, the idempotency key
//! resolution and an in-memory store. It has no dependency on a database.

mod config;
mod key;
mod memory;
mod store;

pub use config::{CleanupConfig, IdempotencyConfig, IdempotencyConfigError};
#[allow(unused_imports)]
pub(crate) use key::{resolve_key, KeySource, Resolved};
pub use memory::InMemoryIdempotencyStore;
pub use store::{Acquire, BoxFuture, IdempotencyStore, StoreError};
