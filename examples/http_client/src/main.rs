use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::get;
use axum::Router;
use serde::{Deserialize, Serialize};
use derust::envx::Environment;
use derust::httpx::json::JsonResponse;
use derust::httpx::{start, AppContext, HttpError, HttpTags};
use derust::http_clientx::HttpClient;

#[derive(Clone)]
pub struct AppState {
    pub gateway: HttpClient,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let env = Environment::detect().ok().unwrap_or(Environment::Local);

    let gateway = HttpClient::new("derust-http-client", "https://any-base-path.com", 1000, 100).await?;

    // any cloneable struct
    let app_state = AppState {
        gateway,
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

#[derive(Deserialize)]
pub struct GatewayResponseDto {
    value: String,
}

#[derive(Serialize)]
pub struct FooResponseDto {
    foo: String,
}

async fn handler(
    State(context): State<AppContext<AppState>>,
) -> Result<JsonResponse<FooResponseDto>, HttpError> {
    let tags = HttpTags::default();

    // getting your application state from context
    let bar: GatewayResponseDto = context.state().gateway.get(&context, "/bar", None, None, &tags).await?;

    Ok(JsonResponse::new(
        StatusCode::OK,
        FooResponseDto { foo: bar.value },
        tags,
    ))
}
