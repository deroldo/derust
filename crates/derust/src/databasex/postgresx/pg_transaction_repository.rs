use crate::databasex::repository::Repository;
use crate::databasex::PostgresTransaction;
use crate::httpx::{AppContext, HttpError, HttpTags};
use axum::http::StatusCode;
use sqlx::query::{QueryAs, QueryScalar};
use sqlx::{Database, FromRow, Postgres};

#[async_trait::async_trait]
#[cfg(not(any(feature = "statsd", feature = "prometheus")))]
impl Repository<Postgres> for PostgresTransaction<'_> {
    async fn fetch_one<'a, S, T>(
        &mut self,
        context: &'a AppContext<S>,
        query_name: &'a str,
        query: QueryAs<'a, Postgres, T, <Postgres as Database>::Arguments<'a>>,
        tags: &HttpTags,
    ) -> Result<T, HttpError>
    where
        T: for<'r> FromRow<'r, <Postgres as Database>::Row> + Send + Unpin,
        S: Clone + Send + Sync,
    {
        let result = query
            .fetch_one(&mut *self.transaction)
            .await
            .map_err(|error| {
                HttpError::without_body(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to execute fetch one for {query_name} with error: {error}"),
                    tags.clone(),
                )
            });

        result
    }

    async fn fetch_optional<'a, S, T>(
        &mut self,
        context: &'a AppContext<S>,
        query_name: &'a str,
        query: QueryAs<'a, Postgres, T, <Postgres as Database>::Arguments<'a>>,
        tags: &HttpTags,
    ) -> Result<Option<T>, HttpError>
    where
        T: for<'r> FromRow<'r, <Postgres as Database>::Row> + Send + Unpin,
        S: Clone + Send + Sync,
    {
        let result = query
            .fetch_optional(&mut *self.transaction)
            .await
            .map_err(|error| {
                HttpError::without_body(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!(
                        "Failed to execute fetch optional for {query_name} with error: {error}"
                    ),
                    tags.clone(),
                )
            });

        result
    }

    async fn fetch_all<'a, S, T>(
        &mut self,
        context: &'a AppContext<S>,
        query_name: &'a str,
        query: QueryAs<'a, Postgres, T, <Postgres as Database>::Arguments<'a>>,
        tags: &HttpTags,
    ) -> Result<Vec<T>, HttpError>
    where
        T: for<'r> FromRow<'r, <Postgres as Database>::Row> + Send + Unpin,
        S: Clone + Send + Sync,
    {
        let result = query
            .fetch_all(&mut *self.transaction)
            .await
            .map_err(|error| {
                HttpError::without_body(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to execute fetch all for {query_name} with error: {error}"),
                    tags.clone(),
                )
            });

        result
    }

    async fn count<'a, S>(
        &mut self,
        context: &'a AppContext<S>,
        query_name: &'a str,
        query: QueryScalar<'a, Postgres, i64, <Postgres as Database>::Arguments<'a>>,
        tags: &HttpTags,
    ) -> Result<u64, HttpError>
    where
        S: Clone + Send + Sync,
    {
        let result = query
            .fetch_one(&mut *self.transaction)
            .await
            .map_err(|error| {
                HttpError::without_body(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to execute count for {query_name} with error: {error}"),
                    tags.clone(),
                )
            });

        Ok(result? as u64)
    }

    async fn exists<'a, S>(
        &mut self,
        context: &'a AppContext<S>,
        query_name: &'a str,
        query: QueryScalar<'a, Postgres, bool, <Postgres as Database>::Arguments<'a>>,
        tags: &HttpTags,
    ) -> Result<bool, HttpError>
    where
        S: Clone + Send + Sync,
    {
        let result = query
            .fetch_one(&mut *self.transaction)
            .await
            .map_err(|error| {
                HttpError::without_body(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to execute exists for {query_name} with error: {error}"),
                    tags.clone(),
                )
            });

        result
    }
}
