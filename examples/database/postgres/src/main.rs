use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::get;
use axum::Router;
use sqlx::query_scalar;
use derust::databasex::{DatabaseConfig, PostgresDatabase, Repository};
use derust::envx::Environment;
use derust::httpx::json::JsonResponse;
use derust::httpx::{start, AppContext, HttpError, HttpTags};

#[derive(Clone)]
pub struct AppState {
    pub bar: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let env = Environment::detect().ok().unwrap_or(Environment::Local);

    // any cloneable struct
    let app_state = AppState {
        bar: "bar".to_string(),
    };

    let application_name = "sample";

    // you can also use DatabaseConfig struct in your app config to deserialize from envs
    let database_config = DatabaseConfig {
        host_rw: "localhost".to_string(),
        host_ro: None,
        name: "local".to_string(),
        user: "local".to_string(),
        pass: "local".to_string(),
        app_name: application_name.to_string(),
        port: 5432,
        min_pool_size: 1,
        max_pool_size: 10,
    };

    // create postgres database pool
    let database = PostgresDatabase::create_from_config(&database_config).await?;

    // easy way to get application context things, like your application state struct and database
    let context = AppContext::new(application_name, env, database, app_state)?;

    let port = 3000;
    let router = Router::new().nest("/foo", Router::new().route("/", get(handler)));

    // automatic health-check route
    // automatic route response status code log
    start(port, context, router).await
}

#[derive(serde::Serialize)]
pub struct FooResponse {
    pub foo: u64,
}

async fn handler(
    State(context): State<AppContext<AppState>>,
) -> Result<JsonResponse<FooResponse>, HttpError> {
    let tags = HttpTags::default();

    let query = query_scalar("select count(1) from foo");

    // get connection
    let mut _conn = context.database().get_connection(false, &tags).await?;

    // begin transaction
    let mut trx = context.database().begin_transaction(&context, &tags).await?;

    // available queries for PoolConnection or PostgresTransaction:
    // - fetch_one
    // - fetch_optional
    // - fetch_all
    // - count
    // - exists
    // fetch functions can be used to insert or update commands

    let foo_count = trx.count(&context, "foo_count", query, &tags).await?;

    // commit transaction
    trx.commit_transaction(&tags).await?;

    Ok(JsonResponse::new(
        StatusCode::OK,
        FooResponse { foo: foo_count },
        tags,
    ))
}
