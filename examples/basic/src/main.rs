use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::get;
use axum::Router;
use rustboot::envx::Environment;
use rustboot::httpx::json::JsonResponse;
use rustboot::httpx::{start, AppContext, HttpError, HttpTags};

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

    // easy way to get application context things, like your application state struct
    let context = AppContext::new(application_name, env, app_state)?;

    let port = 3000;
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
    let bar = context.state().bar.clone();

    Ok(JsonResponse::new(
        StatusCode::OK,
        FooResponse { foo: bar },
        HttpTags::default(),
    ))
}
