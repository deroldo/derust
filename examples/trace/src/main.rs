use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::get;
use axum::Router;
use rustboot::envx::Environment;
use rustboot::httpx::json::JsonResponse;
use rustboot::httpx::{start, AppContext, HttpError, HttpTags};
use rustboot::tracex;
use serde_json::json;

#[derive(Clone)]
pub struct AppState {
    pub bar: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // automatic log subscribe and add b3 traceparent
    let _guard = tracex::init();

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
    // automatic add tags into log
) -> Result<JsonResponse<FooResponse>, HttpError> {
    let customer_id = "1";

    // tags to be added into log
    let tags = HttpTags::from([("customer_id", customer_id)]);

    // getting your application state from context
    let bar = context.state().bar.clone();

    // automatic add tags into log (you can simplify this import)
    rustboot::tracex::log::info("Request handler", &tags);

    // you can also log with tracing::{info, warn, error, etc}
    // tracing::info!("Request handler for customer_id={customer_id}");
    // but we recommend using rustboot logging to ensure a standard way to add tags

    Ok(JsonResponse::new(
        StatusCode::OK,
        FooResponse { foo: bar },
        tags,
    ))
}
