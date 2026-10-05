use crate::commonx::{DerustError, DerustTags};
use crate::httpx::AppContext;
use sqlx::query::{Query, QueryAs, QueryScalar};
use sqlx::{Database, FromRow, Postgres};

#[async_trait::async_trait]
pub trait Repository<DB: Database> {
    async fn fetch_one<'a, S, T>(
        &mut self,
        context: &'a AppContext<S>,
        query_name: &'a str,
        query: QueryAs<'a, DB, T, <DB as Database>::Arguments>,
        tags: &DerustTags,
    ) -> Result<T, DerustError>
    where
        T: for<'r> FromRow<'r, <DB as Database>::Row> + Send + Unpin,
        S: Clone + Send + Sync;

    async fn fetch_optional<'a, S, T>(
        &mut self,
        context: &'a AppContext<S>,
        query_name: &'a str,
        query: QueryAs<'a, DB, T, <DB as Database>::Arguments>,
        tags: &DerustTags,
    ) -> Result<Option<T>, DerustError>
    where
        T: for<'r> FromRow<'r, <DB as Database>::Row> + Send + Unpin,
        S: Clone + Send + Sync;

    async fn fetch_all<'a, S, T>(
        &mut self,
        context: &'a AppContext<S>,
        query_name: &'a str,
        query: QueryAs<'a, DB, T, <DB as Database>::Arguments>,
        tags: &DerustTags,
    ) -> Result<Vec<T>, DerustError>
    where
        T: for<'r> FromRow<'r, <DB as Database>::Row> + Send + Unpin,
        S: Clone + Send + Sync;

    async fn count<'a, S>(
        &mut self,
        context: &'a AppContext<S>,
        query_name: &'a str,
        query: QueryScalar<'a, DB, i64, <DB as Database>::Arguments>,
        tags: &DerustTags,
    ) -> Result<u64, DerustError>
    where
        S: Clone + Send + Sync;

    async fn exists<'a, S>(
        &mut self,
        context: &'a AppContext<S>,
        query_name: &'a str,
        query: QueryScalar<'a, DB, bool, <DB as Database>::Arguments>,
        tags: &DerustTags,
    ) -> Result<bool, DerustError>
    where
        S: Clone + Send + Sync;

    async fn execute<'a, S>(
        &mut self,
        context: &'a AppContext<S>,
        query_name: &'a str,
        query: Query<'a, Postgres, <Postgres as Database>::Arguments>,
        tags: &DerustTags,
    ) -> Result<(), DerustError>
    where
        S: Clone + Send + Sync;
}
