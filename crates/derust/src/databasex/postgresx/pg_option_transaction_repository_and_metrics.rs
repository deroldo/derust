use crate::databasex::repository::Repository;
use crate::databasex::PostgresTransaction;
use crate::httpx::{AppContext, HttpError, HttpTags};
use sqlx::query::{QueryAs, QueryScalar};
use sqlx::{Database, FromRow, Postgres};

#[async_trait::async_trait]
#[cfg(any(feature = "statsd", feature = "prometheus"))]
impl<SS: Clone + Send + Sync> Repository<Postgres> for Option<&mut PostgresTransaction<'_, SS>> {
    async fn fetch_one<'a, S, T>(
        &mut self,
        context: &'a AppContext<S>,
        query_name: &'a str,
        query: QueryAs<'a, Postgres, T, <Postgres as Database>::Arguments<'a>>,
        tags: &HttpTags,
    ) -> Result<T, HttpError>
    where
        T: for<'r> FromRow<'r, <Postgres as Database>::Row> + Send + Unpin,
        S: Clone + Send + Sync {
        match self {
            Some(trx) => trx.fetch_one(context, query_name, query, tags).await,
            None => {
                context
                    .database()
                    .get_connection(true, tags)
                    .await?
                    .fetch_one(context, query_name, query, tags)
                    .await
            }
        }
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
        S: Clone + Send + Sync {
        match self {
            Some(trx) => trx.fetch_optional(context, query_name, query, tags).await,
            None => {
                context
                    .database()
                    .get_connection(true, tags)
                    .await?
                    .fetch_optional(context, query_name, query, tags)
                    .await
            }
        }
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
        S: Clone + Send + Sync {
        match self {
            Some(trx) => trx.fetch_all(context, query_name, query, tags).await,
            None => {
                context
                    .database()
                    .get_connection(true, tags)
                    .await?
                    .fetch_all(context, query_name, query, tags)
                    .await
            }
        }
    }

    async fn count<'a, S>(
        &mut self,
        context: &'a AppContext<S>,
        query_name: &'a str,
        query: QueryScalar<'a, Postgres, i64, <Postgres as Database>::Arguments<'a>>,
        tags: &HttpTags,
    ) -> Result<u64, HttpError>
    where
        S: Clone + Send + Sync {
        match self {
            Some(trx) => trx.count(context, query_name, query, tags).await,
            None => {
                context
                    .database()
                    .get_connection(true, tags)
                    .await?
                    .count(context, query_name, query, tags)
                    .await
            }
        }
    }

    async fn exists<'a, S>(
        &mut self,
        context: &'a AppContext<S>,
        query_name: &'a str,
        query: QueryScalar<'a, Postgres, bool, <Postgres as Database>::Arguments<'a>>,
        tags: &HttpTags,
    ) -> Result<bool, HttpError>
    where
        S: Clone + Send + Sync {
        match self {
            Some(trx) => trx.exists(context, query_name, query, tags).await,
            None => {
                context
                    .database()
                    .get_connection(true, tags)
                    .await?
                    .exists(context, query_name, query, tags)
                    .await
            }
        }
    }
}