use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::get;
use axum::Router;
use serde::Deserialize;
use rustboot::envx::{load_app_config, Environment};
use rustboot::httpx::json::JsonResponse;
use rustboot::httpx::{start, AppContext, HttpError, HttpTags};

#[derive(Clone)]
pub struct AppState {
    pub app_config: AppConfig,
}

// required serde::Deserialize
// sub-structs must be separated by "__" in your environment property
#[derive(Clone, Deserialize)]
pub struct AppConfig {
    pub port: u16,
    pub foo: AppFooConfig,
    pub jwt: AppJwtConfig,
}

// required serde::Deserialize
#[derive(Clone, Deserialize)]
pub struct AppJwtConfig {
    pub private_key: String,
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
    // required when feature env_from_secrets_manager is enabled
    // should be the AWS secrets-manager id/name
    let secrets_manager_ids = vec!["localstack"];
    // app config loader
    let app_config: AppConfig = load_app_config(env, Some(prefix), secrets_manager_ids).await?;

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
    pub pk: String,
}

async fn handler(
    State(context): State<AppContext<AppState>>,
) -> Result<JsonResponse<FooResponse>, HttpError> {
    // getting your application state from context
    let config = &context.state().app_config;
    let bar = config.foo.bar.clone();
    let pk = config.jwt.private_key.clone();

    Ok(JsonResponse::new(
        StatusCode::OK,
        FooResponse {
            foo: bar,
            pk,
        },
        HttpTags::default(),
    ))
}
