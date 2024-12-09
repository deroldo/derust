use crate::httpx::{AppContext, HttpError, HttpTags};
use axum::http::StatusCode;
use sqlx::pool::PoolConnection;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{Error, Pool, Postgres, Transaction};
use std::env;

#[cfg(any(feature = "statsd", feature = "prometheus"))]
use crate::metricx::{timer, MetricTags, Stopwatch};

#[derive(Clone)]
pub struct PostgresDatabase {
    pub read_write: Pool<Postgres>,
    pub read_only: Option<Pool<Postgres>>,
}

impl PostgresDatabase {
    pub async fn create_from_envs() -> Result<PostgresDatabase, Error> {
        let database = DatabaseAttr::from_env();
        create_database(&database).await
    }

    pub async fn create(
        host_rw: &str,
        host_ro: Option<&str>,
        name: &str,
        user: &str,
        pass: &str,
        app_name: &str,
        port: u16,
        min_pool_size: u32,
        max_pool_size: u32,
    ) -> Result<PostgresDatabase, Error> {
        let database = DatabaseAttr {
            host_rw: host_rw.to_string(),
            host_ro: host_ro.map(|it| it.to_string()),
            name: name.to_string(),
            user: user.to_string(),
            pass: pass.to_string(),
            app_name: app_name.to_string(),
            port,
            min_pool_size,
            max_pool_size,
        };

        create_database(&database).await
    }

    pub async fn get_connection(
        &self,
        read_only: bool,
        tags: HttpTags,
    ) -> Result<PoolConnection<Postgres>, HttpError> {
        let pool = if read_only {
            if let Some(ro) = self.read_only.clone() {
                ro
            } else {
                return Err(HttpError::without_body(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Read-only database not found".to_string(),
                    tags,
                ));
            }
        } else {
            self.read_write.clone()
        };

        pool.acquire().await.map_err(|error| {
            HttpError::without_body(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to acquire connection: {error}"),
                tags,
            )
        })
    }

    #[cfg(any(feature = "statsd", feature = "prometheus"))]
    pub async fn begin_transaction<S>(
        &self,
        context: &AppContext<S>,
        tags: HttpTags,
    ) -> Result<PostgresTransaction<S>, HttpError>
    where
        S: Clone,
    {
        let transaction = self.read_write.begin().await.map_err(|error| {
            HttpError::without_body(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to begin transaction: {error}"),
                tags.clone(),
            )
        })?;

        Ok(PostgresTransaction {
            transaction,
            #[cfg(any(feature = "statsd", feature = "prometheus"))]
            stopwatch: timer::start_stopwatch(
                context,
                "repository_transaction_seconds",
                MetricTags::from(tags),
            ),
        })
    }

    #[cfg(not(any(feature = "statsd", feature = "prometheus")))]
    pub async fn begin_transaction<S>(
        &self,
        context: &AppContext<S>,
        tags: HttpTags,
    ) -> Result<PostgresTransaction, HttpError>
    where
        S: Clone,
    {
        let transaction = self.read_write.begin().await.map_err(|error| {
            HttpError::without_body(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to begin transaction: {error}"),
                tags.clone(),
            )
        })?;

        Ok(PostgresTransaction {
            transaction,
            #[cfg(any(feature = "statsd", feature = "prometheus"))]
            stopwatch: timer::start_stopwatch(
                context,
                "repository_transaction_seconds",
                MetricTags::from(tags),
            ),
        })
    }
}

#[cfg(any(feature = "statsd", feature = "prometheus"))]
pub struct PostgresTransaction<'a, S>
where
    S: Clone,
{
    pub transaction: Transaction<'a, Postgres>,
    #[cfg(any(feature = "statsd", feature = "prometheus"))]
    stopwatch: Stopwatch<S>,
}

#[cfg(not(any(feature = "statsd", feature = "prometheus")))]
pub struct PostgresTransaction<'a> {
    pub transaction: Transaction<'a, Postgres>,
}

#[cfg(any(feature = "statsd", feature = "prometheus"))]
impl<'a, S> PostgresTransaction<'a, S>
where
    S: Clone,
{
    pub async fn commit_transaction(self, tags: HttpTags) -> Result<(), HttpError> {
        let result = self.transaction.commit().await.map_err(|error| {
            HttpError::without_body(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to commit transaction: {error}"),
                tags.clone(),
            )
        });

        #[cfg(any(feature = "statsd", feature = "prometheus"))]
        {
            let success = match result {
                Ok(_) => "true",
                Err(_) => "false",
            };

            let mut result_metric_tags = MetricTags::from(tags);
            result_metric_tags =
                result_metric_tags.push("success".to_string(), success.to_string());
            self.stopwatch.record(result_metric_tags);
        }

        result
    }
}

#[cfg(not(any(feature = "statsd", feature = "prometheus")))]
impl<'a> PostgresTransaction<'a> {
    pub async fn commit_transaction(self, tags: HttpTags) -> Result<(), HttpError> {
        self.transaction.commit().await.map_err(|error| {
            HttpError::without_body(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to commit transaction: {error}"),
                tags.clone(),
            )
        })
    }
}

async fn create_database(database: &DatabaseAttr) -> Result<PostgresDatabase, Error> {
    let read_write = PgPoolOptions::new()
        .min_connections(database.min_pool_size)
        .max_connections(database.max_pool_size)
        .test_before_acquire(true)
        .connect_with(database.db_connection_options(false))
        .await?;

    let read_only = if database.host_ro.is_some() {
        Some(
            PgPoolOptions::new()
                .min_connections(database.min_pool_size)
                .max_connections(database.max_pool_size)
                .test_before_acquire(true)
                .connect_with(database.db_connection_options(true))
                .await?,
        )
    } else {
        None
    };

    Ok(PostgresDatabase {
        read_write,
        read_only,
    })
}

struct DatabaseAttr {
    host_rw: String,
    host_ro: Option<String>,
    name: String,
    user: String,
    pass: String,
    app_name: String,
    port: u16,
    min_pool_size: u32,
    max_pool_size: u32,
}

impl DatabaseAttr {
    fn from_env() -> Self {
        let host_rw = env::var("DB_HOST_RW").expect("DB_HOST_RW not found");
        let host_ro = env::var("DB_HOST_RO").ok();
        let name = env::var("DB_NAME").expect("DB_NAME not found");
        let user = env::var("DB_USER").expect("DB_USER not found");
        let pass = env::var("DB_PASS").expect("DB_PASS not found");
        let port = env::var("DB_PORT")
            .expect("DB_PORT not found")
            .parse()
            .expect("DB_PORT is not a number");
        let app_name = env::var("DB_APP_NAME").expect("DB_APP_NAME not found");
        let min_pool_size = env::var("DB_MIN_POOL_SIZE")
            .expect("DB_MIN_POOL_SIZE not found")
            .parse()
            .expect("DB_MIN_POOL_SIZE is not a number");
        let max_pool_size = env::var("DB_MAX_POOL_SIZE")
            .expect("DB_MAX_POOL_SIZE not found")
            .parse()
            .expect("DB_MAX_POOL_SIZE is not a number");

        Self {
            host_rw,
            host_ro,
            name,
            user,
            pass,
            app_name,
            port,
            min_pool_size,
            max_pool_size,
        }
    }

    fn db_connection_options(&self, read_only: bool) -> PgConnectOptions {
        let host = if read_only {
            self.host_ro
                .clone()
                .expect("Read-only database host not found")
        } else {
            self.host_rw.clone()
        };

        PgConnectOptions::new()
            .host(&host)
            .database(&self.name)
            .username(&self.user)
            .password(&self.pass)
            .port(self.port)
            .application_name(&self.app_name)
    }
}
