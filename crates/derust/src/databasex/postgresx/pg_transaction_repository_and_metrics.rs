use crate::databasex::repository::Repository;
use crate::databasex::PostgresTransaction;
use crate::httpx::{AppContext, HttpError, HttpTags};
use axum::http::StatusCode;
use sqlx::query::{QueryAs, QueryScalar};
use sqlx::{Database, FromRow, Postgres};

#[cfg(any(feature = "statsd", feature = "prometheus"))]
use crate::metricx::{timer, MetricTags, Stopwatch};

#[async_trait::async_trait]
#[cfg(any(feature = "statsd", feature = "prometheus"))]
impl<SS: Clone + Send + Sync> Repository<Postgres> for PostgresTransaction<'_, SS> {
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
        #[cfg(any(feature = "statsd", feature = "prometheus"))]
        let stopwatch = stopwatch_start(context, query_name, tags);

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

        #[cfg(any(feature = "statsd", feature = "prometheus"))]
        stopwatch_record(tags, stopwatch, result.is_ok());

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
        #[cfg(any(feature = "statsd", feature = "prometheus"))]
        let stopwatch = stopwatch_start(context, query_name, tags);

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

        #[cfg(any(feature = "statsd", feature = "prometheus"))]
        stopwatch_record(tags, stopwatch, result.is_ok());

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
        #[cfg(any(feature = "statsd", feature = "prometheus"))]
        let stopwatch = stopwatch_start(context, query_name, tags);

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

        #[cfg(any(feature = "statsd", feature = "prometheus"))]
        stopwatch_record(tags, stopwatch, result.is_ok());

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
        #[cfg(any(feature = "statsd", feature = "prometheus"))]
        let stopwatch = stopwatch_start(context, query_name, tags);

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

        #[cfg(any(feature = "statsd", feature = "prometheus"))]
        stopwatch_record(tags, stopwatch, result.is_ok());

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
        #[cfg(any(feature = "statsd", feature = "prometheus"))]
        let stopwatch = stopwatch_start(context, query_name, tags);

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

        #[cfg(any(feature = "statsd", feature = "prometheus"))]
        stopwatch_record(tags, stopwatch, result.is_ok());

        result
    }
}

#[cfg(any(feature = "statsd", feature = "prometheus"))]
fn stopwatch_start<S>(context: &AppContext<S>, query_name: &str, tags: &HttpTags) -> Stopwatch<S>
where
    S: Clone + Send + Sync,
{
    let mut metric_tags = MetricTags::from(tags.clone());
    metric_tags = metric_tags.push("query".to_string(), query_name.to_string());
    timer::start_stopwatch(context, "repository_query_seconds", metric_tags)
}

#[cfg(any(feature = "statsd", feature = "prometheus"))]
fn stopwatch_record<S>(tags: &HttpTags, stopwatch: Stopwatch<S>, success: bool)
where
    S: Clone,
{
    let mut result_metric_tags = MetricTags::from(tags.clone());
    result_metric_tags = result_metric_tags.push("success".to_string(), success.to_string());
    stopwatch.record(result_metric_tags);
}
