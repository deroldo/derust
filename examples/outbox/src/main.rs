use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::get;
use axum::Router;
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;
use rustboot::databasex::{DatabaseConfig, PostgresDatabase};
use rustboot::envx::{load_app_config, Environment};
use rustboot::httpx::json::JsonResponse;
use rustboot::httpx::{start, AppContext, HttpError, HttpTags};
use rustboot::outboxx;
use rustboot::outboxx::OutboxProcessorResources;

#[derive(Clone)]
pub struct AppState {
    pub bar: String,
    pub app_config: AppConfig,
}

#[derive(Clone, Deserialize)]
pub struct AppConfig {
    pub database: DatabaseConfig,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let env = Environment::detect().ok().unwrap_or(Environment::Local);

    let app_config = load_app_config(env, None).await?;

    // any cloneable struct
    let app_state = AppState {
        bar: "bar".to_string(),
        app_config,
    };

    let database = PostgresDatabase::create_from_config(&app_state.app_config.database).await?;

    let application_name = "sample";

    // easy way to get application context things, like your application state struct
    let context = AppContext::new(application_name, env, database.clone(), app_state)?;

    let port = 3000;
    let router = Router::new().nest("/foo", Router::new().route("/", get(handler)));

    // configuring outbox resources:
    // postgres pool is required
    // sns and sqs clients are optional
    let outbox_resources = OutboxProcessorResources::new(database.read_write.clone(), None, None);

    // automatic health-check route
    // automatic route response status code log
    // automatic start outbox-pattern-processor
    start(port, context, router, outbox_resources).await
}

#[derive(serde::Serialize)]
pub struct FooResponse {
    pub foo: String,
}

async fn handler(
    State(context): State<AppContext<AppState>>,
) -> Result<JsonResponse<FooResponse>, HttpError> {
    let tags = HttpTags::default();

    // getting your application state from context
    let bar = context.state().bar.clone();

    let mut trx = context.database().begin_transaction(&context, &tags).await?;

    // sending outbox event
    // you can also use `send_to_sqs` or `send_to_sns` functions
    outboxx::send_to_http(
        &context,
        &mut trx.transaction,
        Uuid::now_v7(),
        "https://any-base-path.com/foo",
        None,
        &json!({
            "outbox": true,
        }),
        &tags,
    )
    .await?;

    trx.commit_transaction(&tags).await?;

    Ok(JsonResponse::new(
        StatusCode::OK,
        FooResponse { foo: bar },
        tags,
    ))
}
