use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::get;
use axum::Router;
use serde::Deserialize;
use derust::envx::{load_app_config, Environment};
use derust::httpx::json::JsonResponse;
use derust::httpx::{start, AppContext, HttpError, HttpTags};

#[derive(Clone)]
pub struct AppState {
    pub app_config: AppConfig,
}

// required serde::Deserialize
#[derive(Clone, Deserialize)]
pub struct AppConfig {
    pub port: u16,
    // sub-structs must be separated by "__" in your environment property
    pub foo: AppFooConfig,
}

// required serde::Deserialize
#[derive(Clone, Deserialize)]
pub struct AppFooConfig {
    pub bar: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let env = Environment::detect().ok().unwrap_or(Environment::Local);

    // optional prefix
    let prefix = "APP";
    // app config loader
    let app_config: AppConfig = load_app_config(env, Some(prefix)).await?;

    let port = app_config.port;

    // any cloneable struct
    let app_state = AppState {
        app_config,
    };

    let application_name = "sample";

    // easy way to get application context things, like your application state struct
    let context = AppContext::new(application_name, env, app_state)?;

    let router = Router::new().nest("/foo", Router::new().route("/", get(handler)));

    // automatic health-check route
    // automatic route response status code log
    start(port, context, router).await
}

#[derive(serde::Serialize)]
pub struct FooResponse {
    pub foo: String,
}

async fn handler(
    State(context): State<AppContext<AppState>>,
) -> Result<JsonResponse<FooResponse>, HttpError> {
    // getting your application state from context
    let bar = context.state().app_config.foo.bar.clone();

    Ok(JsonResponse::new(
        StatusCode::OK,
        FooResponse { foo: bar },
        HttpTags::default(),
    ))
}
