use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::get;
use axum::Router;
use axum_tracing_opentelemetry::tracing_opentelemetry_instrumentation_sdk::find_current_trace_id;
use rustboot::envx::Environment;
use rustboot::httpx::json::JsonResponse;
use rustboot::httpx::{start, HttpError, HttpResponse, MiddlewaresGenericExtension, Tags};
use rustboot::{http_clientx, tracex};
use serde_json::{json, Value};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _guard = tracex::init();

    let env = Environment::detect().ok().unwrap_or(Environment::Local);

    let app_state = AppState {};

    let router = Router::new()
        .nest("/foo", Router::new().route("/", get(foo_handler)))
        .nest("/bar", Router::new().route("/", get(bar_handler)).route("/xpto", get(xpto_handler)))
        .using_httpx(app_state, env);

    start(9095, router).await
}

#[derive(Clone)]
pub struct AppState {}

async fn foo_handler(State(_app_state): State<AppState>) -> Box<dyn HttpResponse> {
    let tags = Tags::from([("a", "b")]);

    Box::new(HttpError::with_json(
        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        "Generic error".to_string(),
        json!({
            "code": 123,
        }),
        None,
        tags,
    ))
}

async fn bar_handler(State(_app_state): State<AppState>) -> Result<String, HttpError> {
    let client = http_clientx::new("my-example", 1000, 200).await.unwrap();

    http_clientx::get::<Value, ()>(
        &client,
        "http://deroldo.free.beeceptor.com",
        Some(vec![("p1", "batata")]),
        Some(vec![("h1", "frita")]),
        Tags::from([("t1", "chips")]),
    ).await?;

    Ok("Hello World".to_string())
}


async fn xpto_handler(State(_app_state): State<AppState>) -> Result<JsonResponse, HttpError> {
    let batata = find_current_trace_id().unwrap_or("none".to_string());
    let tags = Tags::from([("customer_id", "987654323456"), ("batata", &batata)]);
    Ok(JsonResponse::new(StatusCode::OK, json!({
        "code": 456,
    }),None, tags))
}
