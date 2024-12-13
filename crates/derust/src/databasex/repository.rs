use crate::httpx::{AppContext, HttpError, HttpTags};
use sqlx::query::{QueryAs, QueryScalar};
use sqlx::{Database, FromRow};

#[async_trait::async_trait]
pub trait Repository<DB: Database> {
    async fn fetch_one<'a, S, T>(
        &mut self,
        context: &'a AppContext<S>,
        query_name: &'a str,
        query: QueryAs<'a, DB, T, <DB as Database>::Arguments<'a>>,
        tags: &HttpTags,
    ) -> Result<T, HttpError>
    where
        T: for<'r> FromRow<'r, <DB as Database>::Row> + Send + Unpin,
        S: Clone + Send + Sync;

    async fn fetch_optional<'a, S, T>(
        &mut self,
        context: &'a AppContext<S>,
        query_name: &'a str,
        query: QueryAs<'a, DB, T, <DB as Database>::Arguments<'a>>,
        tags: &HttpTags,
    ) -> Result<Option<T>, HttpError>
    where
        T: for<'r> FromRow<'r, <DB as Database>::Row> + Send + Unpin,
        S: Clone + Send + Sync;

    async fn fetch_all<'a, S, T>(
        &mut self,
        context: &'a AppContext<S>,
        query_name: &'a str,
        query: QueryAs<'a, DB, T, <DB as Database>::Arguments<'a>>,
        tags: &HttpTags,
    ) -> Result<Vec<T>, HttpError>
    where
        T: for<'r> FromRow<'r, <DB as Database>::Row> + Send + Unpin,
        S: Clone + Send + Sync;

    async fn count<'a, S>(
        &mut self,
        context: &'a AppContext<S>,
        query_name: &'a str,
        query: QueryScalar<'a, DB, i64, <DB as Database>::Arguments<'a>>,
        tags: &HttpTags,
    ) -> Result<u64, HttpError>
    where
        S: Clone + Send + Sync;

    async fn exists<'a, S>(
        &mut self,
        context: &'a AppContext<S>,
        query_name: &'a str,
        query: QueryScalar<'a, DB, bool, <DB as Database>::Arguments<'a>>,
        tags: &HttpTags,
    ) -> Result<bool, HttpError>
    where
        S: Clone + Send + Sync;
}
